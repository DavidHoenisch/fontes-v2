import "server-only";
import sharp from "sharp";
import { z } from "zod";
import { database } from "./db";
import { DomainError, type Actor } from "./contracts";
import { intakeFieldsSchema, type IntakeState } from "./intake-contracts";
import { hash, validRange, detectReferences } from "./intake-import";
import { preparationChecks } from "./intake-service";
import { buildSource, sourceFiles } from "./intake-package";
import { current, retained, read, put } from "./organization-epub";
import {
  publisherDraftSchema,
  reviewReplySchema,
  reviewMutationSchema,
  reviewVersionSchema,
  reviewBlockSchema,
  type PublisherDraft,
} from "./epub-review-contracts";
import type { ReviewManifest, ReviewDocument } from "./epub-review";

type Row = Awaited<ReturnType<typeof current>>;
function fail(code: string, message: string, status = 422): never {
  throw new DomainError(status, code, message);
}
export async function loadDraft(
  row: Row,
  manifest: ReviewManifest,
): Promise<PublisherDraft> {
  if (row.review_sha256)
    return publisherDraftSchema.parse(
      JSON.parse((await read(row.review_sha256)).toString()),
    );
  return publisherDraftSchema.parse({
    documents: manifest.documents,
    fields: intakeFieldsSchema.parse({ sourceVersification: "KJV" }),
    pending: manifest.statistics.scriptureCandidates,
    accepted: 0,
    rejected: 0,
    editedBlocks: [],
    approval: null,
    package: null,
  });
}
const metadata = (row: Row) => ({
  id: row.id,
  title: row.title,
  description: row.description,
  kind: row.kind,
  language: row.language,
  version: row.version,
  namespace: row.namespace,
  metadataRevision: row.metadata_revision,
});
export function reviewDigest(
  row: Row,
  manifest: ReviewManifest,
  draft: PublisherDraft,
) {
  const { approval: _approval, package: _package, ...content } = draft;
  return hash(
    JSON.stringify({
      metadata: metadata(row),
      source: manifest.sourceSha256,
      content,
    }),
  );
}

function checks(row: Row, manifest: ReviewManifest, draft: PublisherDraft) {
  const state: IntakeState = {
    method: "guided",
    fields: draft.fields,
    cover: draft.cover,
    documents: draft.documents.map((d) => ({
      id: d.id,
      title: d.title,
      sourcePath: d.sourcePath,
      blocks: [],
    })),
    proposals: [],
    source: {
      sha256: manifest.sourceSha256,
      name: "EPUB",
      bytes: manifest.sourceBytes,
      format: "epub",
    },
    warnings: [],
  };
  const result = preparationChecks(state).map((s) =>
    s.replace("Public-free preparation", "This source preparation"),
  );
  if (draft.pending)
    result.push(
      "Accept or reject every Scripture reference, including publisher corrections.",
    );
  if (!["book", "commentary", "study-guide", "devotional"].includes(row.kind))
    result.push("Use offline preparation for this resource kind.");
  return result;
}
export async function publisherReviewReply(
  actor: Actor,
  org: string,
  id: string,
) {
  const row = await current(actor, org, id);
  const manifest = row.manifest_sha256
    ? (JSON.parse(
        (await read(row.manifest_sha256)).toString(),
      ) as ReviewManifest)
    : null;
  const draft = manifest ? await loadDraft(row, manifest) : null;
  return reviewReplySchema.parse({
    resourceId: id,
    resourceRevision: row.revision,
    organizationRevision: row.organization_revision,
    preparationRevision: row.preparation_revision ?? 0,
    reviewRevision: row.review_revision ?? 0,
    manifest,
    draft,
    approved: !!(
      manifest &&
      draft?.approval &&
      draft.approval.digest === reviewDigest(row, manifest, draft)
    ),
    checks:
      manifest && draft
        ? checks(row, manifest, draft)
        : ["Upload an EPUB first."],
  });
}
function assertVersion(row: Row, v: z.infer<typeof reviewVersionSchema>) {
  if (
    row.revision !== v.resourceRevision ||
    row.organization_revision !== v.organizationRevision ||
    row.review_revision !== v.reviewRevision
  )
    fail(
      "revision_conflict",
      "This draft changed. Reload before saving; your corrections have not been applied.",
      409,
    );
}
async function commit(
  actor: Actor,
  org: string,
  id: string,
  row: Row,
  draft: PublisherDraft,
  action: string,
) {
  const bytes = Buffer.from(JSON.stringify(publisherDraftSchema.parse(draft)));
  if (bytes.length > 524288)
    fail("review_limit", "The review inventory exceeds 512 KiB.", 413);
  const digest = await put(bytes),
    sql = database();
  const rows =
    await sql`SELECT fontes_cloud.organization_save_review(${actor.id},${org}::uuid,${id}::uuid,${row.organization_revision},${row.revision},${row.review_revision},${row.manifest_sha256},${digest},${action}) AS result`;
  if (rows[0].result.error)
    fail(
      rows[0].result.error,
      "The organization changed or your publishing access ended. Reload before retrying.",
      rows[0].result.error === "not_found"
        ? 404
        : rows[0].result.error === "forbidden"
          ? 403
          : 409,
    );
  return publisherReviewReply(actor, org, id);
}
export async function mutatePublisherReview(
  actor: Actor,
  org: string,
  id: string,
  input: unknown,
) {
  const v = reviewMutationSchema.parse(input),
    { row, manifest } = await retained(actor, org, id);
  assertVersion(row, v);
  const draft = await loadDraft(row, manifest),
    c = v.command;
  if (c.action === "correct") {
    const item = draft.documents.find((d) => d.id === c.documentId);
    if (!item) fail("not_found", "Section not found.", 404);
    const doc = JSON.parse(
      (await read(item.sha256)).toString(),
    ) as ReviewDocument;
    const block = doc.blocks.find((b) => b.id === c.blockId);
    if (!block || block.kind === "image")
      fail("not_found", "Editable text block not found.", 404);
    if (c.segmentTexts.length !== block.segments.length)
      fail("segment_shape", "Keep the existing inline segments and links.");
    const text = c.segmentTexts.join("");
    if (
      !text.trim() ||
      text.length > 64000 ||
      /[\x00-\x08\x0b\x0c\x0e-\x1f]/.test(text)
    )
      fail(
        "block_text",
        "Use nonempty XML-compatible text, at most 64,000 characters.",
      );
    const changed = text !== block.text;
    block.segments = block.segments.map((s, i) => ({
      ...s,
      text: s.lineBreak ? "\n" : c.segmentTexts[i],
    }));
    const actual = block.segments.map((s) => s.text).join("");
    if (actual !== text)
      fail("line_break", "Keep explicit line break segments unchanged.");
    if (changed) {
      block.text = text;
      // Evidence and decisions cannot survive a changed text context. Manual references need re-entry.
      block.references = detectReferences([
        {
          ...doc,
          blocks: [{ ...block, sourcePath: doc.sourcePath, kind: block.kind }],
        },
      ]);
    } else {
      const seen = new Set<string>();
      block.references = c.references.map((r) => {
        if (seen.has(r.id) || r.blockId !== block.id)
          fail(
            "reference_identity",
            "Use unique references belonging to this block.",
          );
        seen.add(r.id);
        if (!block.text.includes(r.evidence))
          fail(
            "reference_evidence",
            "Reference evidence must occur verbatim in this block.",
          );
        if (!validRange(r.start, r.end))
          fail(
            "reference_target",
            "Choose an existing canonical OSIS range in order.",
          );
        const prior = block.references.find((p) => p.id === r.id);
        const same =
          prior &&
          prior.evidence === r.evidence &&
          prior.start === r.start &&
          prior.end === r.end;
        return {
          ...r,
          method: same ? prior.method : ("publisher-correction-v1" as const),
        };
      });
    }
    reviewBlockSchema.parse(block);
    if (!draft.editedBlocks.includes(block.id))
      draft.editedBlocks.push(block.id);
    const bytes = Buffer.from(JSON.stringify(doc));
    if (bytes.length > 8 * 1024 * 1024)
      fail("review_document_limit", "A corrected section exceeds 8 MiB.", 413);
    item.sha256 = await put(bytes);
    item.characters = doc.blocks.reduce((n, b) => n + b.text.length, 0);
    if (draft.documents.reduce((n, d) => n + d.characters, 0) > 8_000_000)
      fail(
        "review_text_limit",
        "The corrected book exceeds 8 million characters.",
        413,
      );
    draft.pending = draft.accepted = draft.rejected = 0;
    for (const d of draft.documents) {
      const document =
        d.id === doc.id
          ? doc
          : (JSON.parse((await read(d.sha256)).toString()) as ReviewDocument);
      for (const b of document.blocks)
        for (const r of b.references) draft[r.decision]++;
    }
  } else if (c.action === "fields") draft.fields = c.fields;
  else if (c.action === "approve") {
    const problems = checks(row, manifest, draft);
    if (problems.length)
      throw new DomainError(422, "review_incomplete", problems.join(" "), {
        checks: problems,
      });
    await read(manifest.sourceSha256);
    for (const d of draft.documents) await read(d.sha256);
    if (draft.cover) await read(draft.cover.sha256);
    draft.approval = {
      digest: reviewDigest(row, manifest, draft),
      actor: actor.id,
      at: new Date().toISOString(),
    };
    draft.package = null;
    return commit(actor, org, id, row, draft, "approve");
  } else if (c.action === "package") {
    if (
      !draft.approval ||
      draft.approval.digest !== reviewDigest(row, manifest, draft)
    )
      fail(
        "approval_required",
        "Approve the current content, references and rights before generating a source package.",
        409,
      );
    const output = await generate(row, manifest, draft);
    const sha256 = await put(output.buffer);
    draft.package = {
      sha256,
      filename: output.filename,
      approvalDigest: draft.approval.digest,
    };
    return commit(actor, org, id, row, draft, "package");
  }
  draft.approval = null;
  draft.package = null;
  return commit(actor, org, id, row, draft, c.action);
}
export async function uploadReviewCover(
  actor: Actor,
  org: string,
  id: string,
  request: Request,
) {
  const { row, manifest } = await retained(actor, org, id);
  assertVersion(
    row,
    reviewVersionSchema.parse({
      resourceRevision: Number(request.headers.get("x-fontes-revision")),
      organizationRevision: Number(
        request.headers.get("x-fontes-organization-revision"),
      ),
      reviewRevision: Number(request.headers.get("x-fontes-review-revision")),
    }),
  );
  const reader = request.body?.getReader();
  if (!reader) fail("cover_required", "Choose a cover.", 400);
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const p = await reader.read();
    if (p.done) break;
    total += p.value.length;
    if (total > 4 * 1024 * 1024) {
      await reader.cancel();
      fail("cover_size", "Cover limit is 4 MiB.", 413);
    }
    chunks.push(p.value);
  }
  const buffer = Buffer.concat(chunks),
    m = await sharp(buffer, { limitInputPixels: 2_160_000 })
      .metadata()
      .catch(() => null);
  if (
    !m ||
    !["png", "jpeg"].includes(m.format ?? "") ||
    m.width !== 1200 ||
    m.height !== 1800 ||
    (m.pages ?? 1) !== 1 ||
    (m.orientation ?? 1) !== 1
  )
    fail(
      "cover_dimensions",
      "Use an upright, single-frame 1200 × 1800 PNG/JPEG.",
    );
  await sharp(buffer, { limitInputPixels: 2_160_000 }).raw().toBuffer();
  const draft = await loadDraft(row, manifest);
  draft.cover = {
    sha256: await put(buffer),
    bytes: buffer.length,
    mediaType: m.format === "png" ? "image/png" : "image/jpeg",
  };
  draft.approval = null;
  draft.package = null;
  return commit(actor, org, id, row, draft, "cover");
}
const escape = (s: string) =>
  s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
async function generate(
  row: Row,
  manifest: ReviewManifest,
  draft: PublisherDraft,
) {
  const documents: ReviewDocument[] = [];
  for (const d of draft.documents)
    documents.push(JSON.parse((await read(d.sha256)).toString()));
  const resource = {
    version: row.version,
    title: row.title,
    description: row.description,
    kind: row.kind,
    language: row.language,
    id: `${row.namespace}.r${row.id.replaceAll("-", "")}`,
  };
  const state: IntakeState = {
    method: "guided",
    fields: draft.fields,
    cover: draft.cover,
    documents: documents.map((d) => ({
      ...d,
      blocks: d.blocks.map((b) => ({
        ...b,
        kind: b.kind === "image" ? "paragraph" : b.kind,
        sourcePath: d.sourcePath,
      })),
    })),
    proposals: [],
    warnings: [],
    source: {
      sha256: manifest.sourceSha256,
      name: "EPUB",
      bytes: manifest.sourceBytes,
      format: "epub",
    },
  };
  const files = sourceFiles(resource, state),
    binaries: Record<string, Buffer> = {};
  files["metadata.xml"] = files["metadata.xml"].replace(
    "Fontes prose EPUB importer v1; publisher-reviewed conversion",
    "Fontes EPUB review v1; publisher-approved corrected conversion",
  );
  for (const doc of documents) {
    files[`content/${doc.id}.xml`] =
      `<?xml version="1.0" encoding="UTF-8"?><document xmlns="urn:fontes:resource:1" id="${doc.id}" resource-id="${resource.id}"><title>${escape(doc.title)}</title>${doc.blocks
        .map((b) => {
          let text = b.segments
            .map((s) =>
              s.marks.includes("emphasis")
                ? `<em>${escape(s.text)}</em>`
                : escape(s.text),
            )
            .join("");
          if (b.kind === "image") {
            const asset = manifest.assets.find((a) => a.sha256 === b.image);
            if (!asset || !b.alt?.trim())
              fail(
                "image_description",
                "Every image needs its retained asset and a nonempty source description. Use offline preparation to correct missing image descriptions.",
              );
            const name = `media/image-${asset.sha256}.${asset.mediaType === "image/png" ? "png" : "jpg"}`;
            text = `<image src="${name}" alt="${escape(b.alt)}"/>`;
          }
          const refs = b.references.filter((r) => r.decision === "accepted");
          return `<block id="${b.id}" kind="${b.kind === "image" ? "paragraph" : b.kind}"><text>${text}</text>${refs.length ? `<annotations>${refs.map((r) => `<scripture start="${escape(r.start)}"${r.end ? ` end="${escape(r.end)}"` : ""} relation="cites" basis="author"/>`).join("")}</annotations>` : ""}</block>`;
        })
        .join("")}</document>`;
  }
  for (const asset of manifest.assets) {
    if (documents.some((d) => d.blocks.some((b) => b.image === asset.sha256)))
      binaries[
        `media/image-${asset.sha256}.${asset.mediaType === "image/png" ? "png" : "jpg"}`
      ] = await read(asset.sha256);
  }
  return buildSource(
    resource,
    state,
    { buffer: await read(draft.cover!.sha256) },
    { files, binaries },
  );
}
export async function downloadReviewPackage(
  actor: Actor,
  org: string,
  id: string,
) {
  const { row, manifest } = await retained(actor, org, id),
    draft = await loadDraft(row, manifest),
    receipt = draft.package;
  if (
    !receipt ||
    !draft.approval ||
    receipt.approvalDigest !== reviewDigest(row, manifest, draft) ||
    draft.approval.digest !== receipt.approvalDigest
  )
    fail(
      "approval_required",
      "Generate a package from the current publisher approval first.",
      409,
    );
  const buffer = await read(receipt.sha256);
  const latest = await current(actor, org, id);
  if (
    latest.revision !== row.revision ||
    latest.review_sha256 !== row.review_sha256
  )
    fail(
      "revision_conflict",
      "The draft changed before download. Reload it.",
      409,
    );
  return { buffer, filename: receipt.filename };
}
