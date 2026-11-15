import "server-only";
import { randomUUID } from "node:crypto";
import { mkdtemp, mkdir, writeFile, readFile, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { ZipFile } from "yazl";
import { z } from "zod";
import { database } from "./db";
import { DomainError, type Actor } from "./contracts";
import { put, read, current } from "./organization-epub";
import {
  publisherReviewReply,
  downloadReviewPackage,
} from "./publisher-review";
import { exportIntake, intake, validateSource } from "./intake-service";
import { getResource } from "./repository";
import { hash, importPrepared } from "./intake-import";
import { readArchive, xml, xmlText, array } from "./intake-archive";
import {
  sourceSubmissionSchema,
  sourceReviewPageSchema,
  sourceDecisionSchema,
  sourceReviewItemSchema,
  sourceReviewDetailSchema,
  packagePreviewSchema,
  type SourceReviewItem,
} from "./source-review-contracts";
function local() {
  if (process.env.NODE_ENV !== "development")
    throw new DomainError(
      503,
      "review_storage_unavailable",
      "Source review and handoff currently use private local-development storage.",
    );
}
function fail(code: string, message: string, status = 409): never {
  throw new DomainError(status, code, message);
}
function resultError(result: { error?: string }) {
  if (result.error)
    fail(
      result.error,
      (
        {
          already_submitted:
            "This exact publisher revision is already submitted. Open its existing review.",
          independent_reviewer_required:
            "An independent Fontes maintainer must make this decision.",
          version_conflict:
            "This resource version already has a handoff with different source bytes. Submit a new version.",
          review_inactive:
            "This submission was withdrawn or superseded. Submit the newly approved package.",
        } as Record<string, string>
      )[result.error] ??
        "The review changed or your access ended. Reload before retrying.",
      result.error === "not_found"
        ? 404
        : ["forbidden", "independent_reviewer_required"].includes(result.error)
          ? 403
          : 409,
    );
}
async function accessible(actor: Actor, id: string) {
  local();
  z.uuid().parse(id);
  const sql = database();
  const rows = await sql`SELECT s.*,
 EXISTS(SELECT 1 FROM fontes_cloud.resources r WHERE r.id=s.personal_resource_id AND r.owner_id=${actor.id}) OR EXISTS(SELECT 1 FROM fontes_cloud.organization_members m WHERE m.organization_id=s.organization_id AND m.account_id=${actor.id} AND m.role IN ('owner','admin','publisher')) AS publishing
 FROM fontes_cloud.source_reviews s WHERE s.id=${id}::uuid AND (${actor.role === "reviewer"} OR EXISTS(SELECT 1 FROM fontes_cloud.resources r WHERE r.id=s.personal_resource_id AND r.owner_id=${actor.id}) OR EXISTS(SELECT 1 FROM fontes_cloud.organization_members m WHERE m.organization_id=s.organization_id AND m.account_id=${actor.id} AND m.role IN ('owner','admin','publisher')))`;
  if (!rows[0])
    fail(
      "not_found",
      "This source review is not available to your account.",
      404,
    );
  return rows[0];
}
function wire(row: Record<string, any>, actor: Actor): SourceReviewItem {
  return sourceReviewItemSchema.parse({
    ...row,
    created_at: new Date(row.created_at).toISOString(),
    updated_at: new Date(row.updated_at).toISOString(),
    independent:
      actor.role === "reviewer" &&
      ![row.submitted_by, row.publisher_actor, row.creator_id].includes(
        actor.id,
      ),
    canWithdraw: row.publishing === true,
  });
}
export async function listSourceReviews(actor: Actor, history = false) {
  local();
  const sql = database();
  const rows =
    await sql`SELECT s.*,EXISTS(SELECT 1 FROM fontes_cloud.resources r WHERE r.id=s.personal_resource_id AND r.owner_id=${actor.id}) OR EXISTS(SELECT 1 FROM fontes_cloud.organization_members m WHERE m.organization_id=s.organization_id AND m.account_id=${actor.id} AND m.role IN ('owner','admin','publisher')) AS publishing FROM fontes_cloud.source_reviews s
 WHERE (${actor.role === "reviewer"} OR EXISTS(SELECT 1 FROM fontes_cloud.resources r WHERE r.id=s.personal_resource_id AND r.owner_id=${actor.id}) OR EXISTS(SELECT 1 FROM fontes_cloud.organization_members m WHERE m.organization_id=s.organization_id AND m.account_id=${actor.id} AND m.role IN ('owner','admin','publisher')))
 AND ((s.status IN ('withdrawn','superseded'))=${history}) ORDER BY s.updated_at DESC LIMIT 100`;
  return { items: rows.map((r) => wire(r, actor)), limit: 100, history };
}
export async function sourceReviewDetail(actor: Actor, id: string) {
  const row = await accessible(actor, id),
    sql = database();
  const preview = packagePreviewSchema.parse(
    JSON.parse((await read(row.snapshot_sha256)).toString()),
  );
  const events =
    await sql`SELECT id,actor_id,action,note,created_at,details FROM fontes_cloud.source_review_events WHERE review_id=${id}::uuid ORDER BY created_at DESC,id DESC LIMIT 100`;
  return sourceReviewDetailSchema.parse({
    item: wire(row, actor),
    preview,
    events: events.map((e) => ({
      ...e,
      created_at: new Date(e.created_at).toISOString(),
    })),
  });
}
async function normalize(buffer: Buffer) {
  const temp = await mkdtemp(path.join(os.tmpdir(), "fontes-source-review-"));
  try {
    const raw = await readArchive(buffer),
      m = xml(raw.get("metadata.xml")?.toString() ?? "").resource;
    const filename = `${m["@_id"]}--${m["@_version"]}.fontes-source.zip`;
    const input = path.join(temp, "input.zip");
    await writeFile(input, buffer, { mode: 0o600 });
    await mkdir(path.join(temp, "first"));
    await mkdir(path.join(temp, "second"));
    const output = path.join(temp, "first", filename),
      again = path.join(temp, "second", filename);
    const cli =
      process.env.FONTES_RESOURCE_CLI ??
      path.resolve(process.cwd(), "../../target/debug/fontes-resource");
    const report = await validateSource(input, true);
    for (const destination of [output, again])
      await promisify(execFile)(
        cli,
        [
          "pack",
          input,
          "--output",
          destination,
          "--deny-warnings",
          ...(process.env.FONTES_RESOURCE_TAXONOMY
            ? ["--taxonomy", process.env.FONTES_RESOURCE_TAXONOMY]
            : []),
        ],
        {
          timeout: 30000,
          maxBuffer: 1024 * 1024,
          env: {
            NODE_ENV: "development",
            PATH: process.env.PATH,
            LANG: "en_US.UTF-8",
          },
        },
      );
    const canonical = await readFile(output);
    if (hash(canonical) !== hash(await readFile(again)))
      fail(
        "nonreproducible_source",
        "The trusted source packer produced different canonical ZIP bytes.",
        422,
      );
    return { buffer: canonical, report };
  } finally {
    await rm(temp, { recursive: true, force: true });
  }
}
export async function submitSourceReview(actor: Actor, input: unknown) {
  local();
  const v = sourceSubmissionSchema.parse(input),
    s = v.source;
  let output: { buffer: Buffer; filename: string },
    publisherApproval: { actor: string; at: string; digest: string };
  if (s.kind === "organization") {
    const reply = await publisherReviewReply(
      actor,
      s.organizationId,
      s.resourceId,
    );
    if (
      reply.resourceRevision !== v.resourceRevision ||
      reply.organizationRevision !== s.organizationRevision ||
      reply.reviewRevision !== s.reviewRevision
    )
      fail(
        "revision_conflict",
        "Reload the publisher review before submitting.",
      );
    if (!reply.approved || !reply.draft?.package || !reply.draft.approval)
      fail(
        "publisher_approval_required",
        "Approve and generate the current package before submitting it.",
      );
    output = await downloadReviewPackage(actor, s.organizationId, s.resourceId);
    publisherApproval = reply.draft.approval;
  } else {
    const resource = await getResource(actor, s.resourceId);
    if (resource.owner_id !== actor.id)
      fail("not_found", "Only the publisher can submit this source.", 404);
    if (resource.revision !== v.resourceRevision)
      fail(
        "revision_conflict",
        "Reload the publisher draft before submitting.",
      );
    const prep = await intake(actor, s.resourceId);
    if (!prep.approval)
      fail(
        "publisher_approval_required",
        "Approve the source preparation first.",
      );
    output = await exportIntake(actor, s.resourceId);
    publisherApproval = prep.approval;
  }
  const prepared = await normalize(output.buffer),
    files = await readArchive(prepared.buffer),
    m = xml(files.get("metadata.xml")!.toString()).resource;
  // Preview the declared rights paths, including valid packages using non-default filenames.
  const aliases = new Map(files);
  aliases.set("rights/LICENSE.txt", files.get(m.rights["@_license-file"])!);
  const state = importPrepared(aliases);
  if (
    state.fields.permissions["offline-display"] !== "allow" ||
    state.fields.permissions["keyword-search"] !== "allow"
  )
    fail(
      "public_rights_required",
      "Public-free source review requires declared offline display and keyword search permission.",
      422,
    );
  if (
    state.documents.some((d) => d.blocks.some((b) => b.text.length > 64000)) ||
    state.documents.length > 256 ||
    state.documents.reduce((n, d) => n + d.blocks.length, 0) > 25000 ||
    state.documents.reduce(
      (n, d) => n + d.blocks.reduce((a, b) => a + b.text.length, 0),
      0,
    ) > 8000000
  )
    fail(
      "review_limit",
      "This package exceeds the local web review limits. Use repository intake.",
      413,
    );
  // Add inline references/images as readable assertions rather than hiding them in flattened prose.
  function assertions(nodes: any[]): string[] {
    return nodes.flatMap((n) =>
      Object.entries(n).flatMap(([k, value]) =>
        k === ":@"
          ? []
          : [
              ...([
                "ref",
                "image",
                "strongs",
                "term",
                "scripture",
                "concept",
              ].includes(k)
                ? [
                    `${k}: ${Object.entries(n[":@"] ?? {})
                      .map(([a, v]) => `${a.slice(2)}=${v}`)
                      .join(", ")}`,
                  ]
                : []),
              ...(Array.isArray(value) ? assertions(value) : []),
            ],
      ),
    );
  }
  const documents = [];
  for (const doc of state.documents) {
    const source = xml(files.get(doc.sourcePath)!.toString(), true).find(
      (n: any) => n.document,
    );
    const nodes = source.document.filter((n: any) => n.block);
    doc.blocks.forEach((b, i) => {
      b.sourceAssertions = [
        ...(b.sourceAssertions ?? []),
        ...assertions(nodes[i]?.block ?? []),
      ];
    });
    const bytes = Buffer.from(
      JSON.stringify({
        ...doc,
        sourceAssertions: assertions(
          source.document.filter((n: any) => n.annotations),
        ),
      }),
    );
    if (bytes.length > 8 * 1024 * 1024)
      fail("review_limit", "A preview section exceeds 8 MiB.", 413);
    documents.push({
      id: doc.id,
      title: doc.title,
      sha256: await put(bytes),
      blocks: doc.blocks.length,
    });
  }
  const sourceHash = await put(prepared.buffer),
    upstreamHash = hash(output.buffer);
  const preview = packagePreviewSchema.parse({
    title: xmlText(m.title),
    description: xmlText(m.description),
    kind: m["@_kind"],
    language: m["@_language"],
    fields: {
      ...state.fields,
      coverageRanges: array<any>(m.coverage.range).map(
        (r) => `${r["@_start"]}${r["@_end"] ? `–${r["@_end"]}` : ""}`,
      ),
      contributors: array<any>(m.contributors?.contributor).map(
        (c) => `${c["@_role"] ?? "contributor"}: ${xmlText(c)}`,
      ),
    },
    releaseNotes: v.releaseNotes,
    documents,
    files: [],
    validation: prepared.report,
    publisherApproval,
  });
  for (const [name, bytes] of files)
    preview.files.push({
      path: name,
      sha256: await put(bytes),
      bytes: bytes.length,
    });
  const snapshot = await put(Buffer.from(JSON.stringify(preview))),
    id = randomUUID(),
    sql = database();
  let rows;
  try {
    rows =
      await sql`SELECT fontes_cloud.submit_source_review(${actor.id},${id}::uuid,${s.kind === "personal" ? s.resourceId : null},${s.kind === "organization" ? s.organizationId : null}::uuid,${s.kind === "organization" ? s.resourceId : null}::uuid,${v.resourceRevision},${s.kind === "organization" ? s.organizationRevision : 0},${s.kind === "organization" ? s.reviewRevision : 0},${publisherApproval.actor},${m["@_id"]},${m.publisher["@_id"]},${m["@_version"]},${preview.title},${sourceHash},${upstreamHash},${snapshot}) AS result`;
  } catch (error) {
    if ((error as { code?: string }).code === "23514")
      fail("queue_capacity", "The local active source-review queue is full.");
    throw error;
  }
  resultError(rows[0].result);
  return sourceReviewDetail(actor, id);
}
async function active(actor: Actor, id: string) {
  const row = await accessible(actor, id);
  if (["withdrawn", "superseded"].includes(row.status))
    fail(
      "review_inactive",
      "This review is withdrawn or superseded. Open the newly submitted package.",
    );
  return row;
}
export async function reviewDocument(
  actor: Actor,
  id: string,
  q: URLSearchParams,
) {
  const row = await active(actor, id),
    snapshot = packagePreviewSchema.parse(
      JSON.parse((await read(row.snapshot_sha256)).toString()),
    );
  const item = snapshot.documents.find((d) => d.id === q.get("document"));
  if (!item) fail("not_found", "Section not found.", 404);
  const doc = JSON.parse((await read(item.sha256)).toString()),
    offset = z.coerce
      .number()
      .int()
      .min(0)
      .max(25000)
      .parse(q.get("offset") ?? 0);
  if (offset >= doc.blocks.length) fail("not_found", "Page not found.", 404);
  return sourceReviewPageSchema.parse({
    ...doc,
    blocks: doc.blocks.slice(offset, offset + 40),
    offset,
    total: doc.blocks.length,
    nextOffset: offset + 40 < doc.blocks.length ? offset + 40 : null,
    previousOffset: offset ? Math.max(0, offset - 40) : null,
  });
}
export async function reviewFile(actor: Actor, id: string, q: URLSearchParams) {
  const row = await active(actor, id),
    snapshot = packagePreviewSchema.parse(
      JSON.parse((await read(row.snapshot_sha256)).toString()),
    );
  const name = q.get("path"),
    f = snapshot.files.find((f) => f.path === name);
  if (!f) fail("not_found", "File not found in this reviewed package.", 404);
  const buffer = await read(f.sha256);
  return {
    buffer,
    filename: path.basename(f.path),
    mediaType: /\.png$/.test(f.path)
      ? "image/png"
      : /\.jpe?g$/.test(f.path)
        ? "image/jpeg"
        : "application/octet-stream",
  };
}
export async function sourceDownload(
  actor: Actor,
  id: string,
  handoff = false,
) {
  const row = await active(actor, id);
  if (handoff && (!row.handoff || row.status !== "handoff_ready"))
    fail("handoff_required", "Prepare the publication handoff first.");
  const buffer = await read(handoff ? row.handoff.sha256 : row.source_sha256);
  const latest = await active(actor, id);
  if (latest.revision !== row.revision)
    fail("revision_conflict", "The review changed before download.");
  return {
    buffer,
    filename: handoff
      ? row.handoff.filename
      : `${row.resource_id}--${row.version}.fontes-source.zip`,
  };
}
export async function zipFiles(files: Record<string, Buffer>) {
  const zip = new ZipFile();
  const output = new Promise<Buffer>((resolve, reject) => {
    const chunks: Buffer[] = [];
    zip.outputStream.on("data", (b) => chunks.push(b));
    zip.outputStream.on("end", () => resolve(Buffer.concat(chunks)));
    zip.outputStream.on("error", reject);
  });
  for (const [name, bytes] of Object.entries(files).sort(([a], [b]) =>
    a.localeCompare(b),
  ))
    zip.addBuffer(bytes, name, {
      mtime: new Date("1980-01-01T00:00:00Z"),
      mode: 0o100644,
    });
  zip.end();
  return output;
}
async function handoffFiles(row: Record<string, any>) {
  const snapshot = packagePreviewSchema.parse(
      JSON.parse((await read(row.snapshot_sha256)).toString()),
    ),
    a = row.approval;
  const source = await read(row.source_sha256),
    review = await normalize(source);
  if (hash(review.buffer) !== row.source_sha256)
    fail(
      "source_drift",
      "The trusted packer changed the reviewed canonical bytes. Submit a new package.",
    );
  const approval = {
    approval_version: 1,
    resource_id: row.resource_id,
    version: row.version,
    source_sha256: row.source_sha256,
    distribution: "public_free",
    rights_review_id: a.rightsReviewId,
    publisher_review_id: a.publisherReviewId,
  };
  const submission = {
    submission_version: 1,
    resource_id: row.resource_id,
    publisher_id: row.publisher_id,
    version: row.version,
    build_revision: 1,
    source: {
      type: "path",
      path: `submissions/${row.resource_id}/source.fontes-source.zip`,
    },
    release_notes: snapshot.releaseNotes,
  };
  const json = (v: unknown) => Buffer.from(JSON.stringify(v, null, 2) + "\n");
  return zipFiles({
    [`submissions/${row.resource_id}/source.fontes-source.zip`]: source,
    [`submissions/${row.resource_id}/submission.json`]: json(submission),
    [`approvals/${row.resource_id}--${row.version}.json`]: json(approval),
    "review-receipt.json": json({
      reviewId: row.id,
      sourceSha256: row.source_sha256,
      upstreamSha256: row.upstream_sha256,
      publisherApproval: snapshot.publisherApproval,
      maintainerApproval: a,
      validation: snapshot.validation,
    }),
    "README.txt": Buffer.from(
      `Fontes source publication handoff — not published or signed.\n\nCopy submissions/ and approvals/ into the separately trusted library repository. Keep private rights evidence outside Git; the review IDs must be non-secret references.\nVerify namespace authority and add ${row.publisher_id} to the real reviewed publisher policy if authorized. This bundle contains no policy change or signing authority.\nRun trusted fontes-resource submissions check using the repository's existing policy/taxonomy/trust, review the changes, and merge through the normal protected default-branch source-release workflow. Never give untrusted validation work signing credentials.\nExisting source releases are immutable. Check destination versions/builds before selecting a build revision; the descriptor starts at build_revision 1.\nThis bundle contains no compiled app package or catalog listing.\n`,
    ),
  });
}
export async function decideSourceReview(
  actor: Actor,
  id: string,
  input: unknown,
) {
  const v = sourceDecisionSchema.parse(input),
    row = await active(actor, id),
    c = v.command;
  if (row.revision !== v.revision || row.source_sha256 !== v.sourceSha256)
    fail("revision_conflict", "Reload this review before deciding.");
  if (
    !["withdraw", "comment"].includes(c.action) &&
    (actor.role !== "reviewer" ||
      [row.submitted_by, row.publisher_actor, row.creator_id].includes(
        actor.id,
      ))
  )
    fail(
      "independent_reviewer_required",
      "An independent Fontes maintainer must make this decision.",
      403,
    );
  if (c.action === "withdraw" && !row.publishing)
    fail(
      "forbidden",
      "Only the current publishing team can withdraw a submission.",
      403,
    );
  let approval = null,
    handoff = null;
  if (c.action === "approve") {
    if (row.status !== "submitted")
      fail("invalid_transition", "Only a submitted package can be approved.");
    await read(row.source_sha256);
    approval = {
      actor: actor.id,
      at: new Date().toISOString(),
      sourceSha256: row.source_sha256,
      rightsReviewId: c.rightsReviewId,
      publisherReviewId: c.publisherReviewId,
      checks: c.checks,
      note: c.note,
    };
  }
  if (c.action === "handoff") {
    if (row.status === "handoff_ready") return sourceReviewDetail(actor, id);
    if (row.status !== "approved" || !row.approval)
      fail(
        "approval_required",
        "Record maintainer approval before preparing a publication handoff.",
      );
    const buffer = await handoffFiles(row);
    handoff = {
      sha256: await put(buffer),
      filename: `${row.resource_id}--${row.version}.publication-handoff.zip`,
    };
  }
  const sql = database();
  const rows =
    await sql`SELECT fontes_cloud.source_review_command(${actor.id},${id}::uuid,${v.revision},${v.sourceSha256},${c.action},${"note" in c ? c.note : "Prepared an unsigned source publication handoff for the trusted release workflow."},${approval ? JSON.stringify(approval) : null}::jsonb,${handoff ? JSON.stringify(handoff) : null}::jsonb) AS result`;
  resultError(rows[0].result);
  return sourceReviewDetail(actor, id);
}
