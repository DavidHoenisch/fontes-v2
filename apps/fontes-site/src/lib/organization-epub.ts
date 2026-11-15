import "server-only";
import {
  reviewManifestSchema,
  reviewReplySchema,
  reviewPageSchema,
} from "./epub-review-contracts";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { z } from "zod";
import { database } from "./db";
import { DomainError, type Actor } from "./contracts";
import { readArchive, MAX_UPLOAD } from "./intake-archive";
import { hash } from "./intake-import";
import {
  reviewEpub,
  type ReviewDocument,
  type ReviewManifest,
} from "./epub-review";
const root = path.resolve(process.cwd(), ".local/org-epub");
function local() {
  if (process.env.NODE_ENV !== "development")
    throw new DomainError(
      503,
      "review_storage_unavailable",
      "EPUB evaluation currently uses private local-development storage.",
    );
}
const artifactPath = (digest: string) =>
  path.join(
    root,
    z
      .string()
      .regex(/^[a-f0-9]{64}$/)
      .parse(digest),
  );
export async function put(buffer: Buffer) {
  const digest = hash(buffer);
  await mkdir(root, { recursive: true, mode: 0o700 });
  try {
    await writeFile(artifactPath(digest), buffer, { mode: 0o600, flag: "wx" });
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
    if (hash(await readFile(artifactPath(digest))) !== digest)
      throw new DomainError(
        503,
        "artifact_integrity",
        "A retained local artifact failed verification.",
      );
  }
  return digest;
}
export async function read(digest: string) {
  const b = await readFile(artifactPath(digest));
  if (hash(b) !== digest)
    throw new DomainError(
      503,
      "artifact_integrity",
      "A retained local artifact failed verification.",
    );
  return b;
}
export async function current(actor: Actor, org: string, id: string) {
  local();
  z.uuid().parse(org);
  z.uuid().parse(id);
  const sql = database();
  const rows =
    await sql`SELECT r.*,o.revision AS organization_revision,p.manifest_sha256,p.revision AS preparation_revision,p.review_sha256,p.review_revision,o.namespace
    FROM fontes_cloud.organization_resources r JOIN fontes_cloud.organizations o ON o.id=r.organization_id
    JOIN fontes_cloud.organization_members m ON m.organization_id=o.id
    LEFT JOIN fontes_cloud.organization_epub_preparations p ON p.resource_id=r.id
    WHERE o.id=${org}::uuid AND r.id=${id}::uuid AND m.account_id=${actor.id} AND m.role IN ('owner','admin','publisher')`;
  if (!rows[0])
    throw new DomainError(
      404,
      "not_found",
      "This preparation is not available to your account.",
    );
  return rows[0];
}
export async function retained(actor: Actor, org: string, id: string) {
  const row = await current(actor, org, id);
  if (!row.manifest_sha256)
    throw new DomainError(
      404,
      "preparation_missing",
      "Upload an EPUB to start evaluation.",
    );
  return {
    row,
    manifest: reviewManifestSchema.parse(
      JSON.parse((await read(row.manifest_sha256)).toString()),
    ),
  };
}
export async function getEpubReview(actor: Actor, org: string, id: string) {
  const { publisherReviewReply } = await import("./publisher-review");
  return publisherReviewReply(actor, org, id);
}
export async function importOrganizationEpub(
  actor: Actor,
  org: string,
  id: string,
  request: Request,
) {
  const row = await current(actor, org, id);
  const resourceRevision = Number(request.headers.get("x-fontes-revision")),
    orgRevision = Number(request.headers.get("x-fontes-organization-revision"));
  if (
    row.revision !== resourceRevision ||
    row.organization_revision !== orgRevision
  )
    throw new DomainError(
      409,
      "revision_conflict",
      "Refresh the organization before importing.",
    );
  const reader = request.body?.getReader();
  if (!reader) throw new DomainError(400, "upload_required", "Choose an EPUB.");
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const part = await reader.read();
    if (part.done) break;
    total += part.value.length;
    if (total > MAX_UPLOAD) {
      await reader.cancel();
      throw new DomainError(
        413,
        "upload_too_large",
        "EPUB uploads are limited to 20 MiB.",
      );
    }
    chunks.push(part.value);
  }
  if (!total)
    throw new DomainError(400, "upload_required", "Choose a nonempty EPUB.");
  const source = Buffer.concat(chunks),
    result = await reviewEpub(await readArchive(source));
  // Book prose and source bytes stay on disk; only inventory/report metadata go to Postgres.
  for (const doc of result.documents) {
    const buffer = Buffer.from(JSON.stringify(doc));
    if (buffer.length > 8 * 1024 * 1024)
      throw new DomainError(
        413,
        "review_document_limit",
        "A derived document exceeds 8 MiB.",
      );
    await put(buffer);
  }
  const assets = [];
  for (const [sha256, asset] of result.assets) {
    await put(asset.buffer);
    assets.push({
      sha256,
      mediaType: asset.mediaType,
      width: asset.width,
      height: asset.height,
    });
  }
  const sourceSha256 = await put(source);
  const manifest = reviewManifestSchema.parse({
    ...result.summary,
    sourceSha256,
    sourceBytes: source.length,
    assets,
  });
  const manifestBytes = Buffer.from(JSON.stringify(manifest));
  if (manifestBytes.length > 262144)
    throw new DomainError(
      413,
      "review_inventory_limit",
      "The evaluation inventory exceeds 256 KiB.",
    );
  const manifestSha256 = await put(manifestBytes);
  const sql = database();
  const rows =
    await sql`SELECT fontes_cloud.organization_import_epub(${actor.id},${org}::uuid,${id}::uuid,${orgRevision},${resourceRevision},${manifestSha256},${sourceSha256},${JSON.stringify(result.summary)}::jsonb) AS result`;
  if (rows[0].result.error) {
    const code = rows[0].result.error;
    throw new DomainError(
      code === "not_found" ? 404 : code === "forbidden" ? 403 : 409,
      code,
      "The organization changed or your publishing access ended. Refresh before retrying.",
    );
  }
  return getEpubReview(actor, org, id);
}
export async function getReviewDocument(
  actor: Actor,
  org: string,
  id: string,
  query: URLSearchParams,
) {
  const { row, manifest } = await retained(actor, org, id);
  let documentId = z
    .string()
    .regex(/^document-[a-f0-9]{24}$/)
    .parse(query.get("document"));
  const { loadDraft } = await import("./publisher-review");
  const draft = await loadDraft(row, manifest);
  if (query.get("pending") === "next") {
    query.delete("block");
    const start = draft.documents.findIndex((d) => d.id === documentId);
    if (start < 0)
      throw new DomainError(
        404,
        "not_found",
        "The section is not in this preparation.",
      );
    const after = query.get("after");
    for (let step = 0; step <= draft.documents.length; step++) {
      const candidate =
        draft.documents[(Math.max(0, start) + step) % draft.documents.length];
      const document = JSON.parse(
        (await read(candidate.sha256)).toString(),
      ) as ReviewDocument;
      const afterIndex =
        step === 0 && after
          ? document.blocks.findIndex((b) => b.id === after)
          : -1;
      const pending = document.blocks.find(
        (b, i) =>
          i > afterIndex && b.references.some((r) => r.decision === "pending"),
      );
      if (pending) {
        documentId = candidate.id;
        query.set("block", pending.id);
        break;
      }
    }
    if (!query.get("block"))
      throw new DomainError(
        404,
        "no_pending_references",
        "All Scripture reference candidates have a decision.",
      );
  }
  const item = draft.documents.find((d) => d.id === documentId);
  if (!item)
    throw new DomainError(
      404,
      "not_found",
      "The section is not in this preparation.",
    );
  const doc = JSON.parse(
    (await read(item.sha256)).toString(),
  ) as ReviewDocument;
  const requested = z.coerce
    .number()
    .int()
    .min(0)
    .max(25000)
    .parse(query.get("offset") ?? 0);
  const focus = query.get("block");
  let offset = requested;
  if (focus) {
    z.string()
      .regex(/^block-[a-f0-9]{24}$/)
      .parse(focus);
    const index = doc.blocks.findIndex((b) => b.id === focus);
    if (index < 0)
      throw new DomainError(
        404,
        "not_found",
        "The target block is not in this section.",
      );
    offset = Math.floor(index / 40) * 40;
  }
  if (offset >= doc.blocks.length)
    throw new DomainError(404, "not_found", "This section has no such page.");
  const originalItem = manifest.documents.find((d) => d.id === documentId)!;
  const original = JSON.parse(
    (await read(originalItem.sha256)).toString(),
  ) as ReviewDocument;
  return reviewPageSchema.parse({
    resourceRevision: row.revision,
    preparationRevision: row.preparation_revision,
    id: doc.id,
    title: doc.title,
    sourcePath: doc.sourcePath,
    blocks: doc.blocks.slice(offset, offset + 40).map((b) => ({
      ...b,
      originalText: original.blocks.find((o) => o.id === b.id)?.text,
    })),
    offset,
    total: doc.blocks.length,
    nextOffset: offset + 40 < doc.blocks.length ? offset + 40 : null,
    previousOffset: offset ? Math.max(0, offset - 40) : null,
    focusBlockId: focus,
  });
}
export async function getReviewAsset(
  actor: Actor,
  org: string,
  id: string,
  digest: string,
) {
  const { manifest } = await retained(actor, org, id);
  const asset = manifest.assets.find((a) => a.sha256 === digest);
  if (!asset)
    throw new DomainError(
      404,
      "not_found",
      "This image is not in the preparation.",
    );
  return { buffer: await read(asset.sha256), mediaType: asset.mediaType };
}
