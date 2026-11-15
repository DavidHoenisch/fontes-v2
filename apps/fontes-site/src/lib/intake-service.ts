import "server-only";
import { preparationDigest } from "./intake-digest";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdir, readFile, writeFile, mkdtemp, rm } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import sharp from "sharp";
import { database } from "./db";
import { getResource } from "./repository";
import {
  DomainError,
  createResourceSchema,
  type Actor,
  type Resource,
} from "./contracts";
import {
  emptyIntake,
  intakeSaveSchema,
  intakeReplySchema,
  type IntakeReply,
  type IntakeState,
} from "./intake-contracts";
import { readArchive, xml, xmlText, MAX_UPLOAD } from "./intake-archive";
import { hash, importEpub, importPrepared, validRange } from "./intake-import";

const exec = promisify(execFile);
const store = path.resolve(process.cwd(), ".local/intake");
const assetPath = (id: string, digest: string) =>
  path.join(store, hash(id), digest);
function localOnly() {
  if (process.env.NODE_ENV !== "development")
    throw new DomainError(
      503,
      "intake_storage_unavailable",
      "File preparation currently runs only in local development. Hosted isolated workers/storage are not connected.",
    );
}
async function row(actor: Actor, id: string, edit = false) {
  const resource = await getResource(actor, id);
  if (
    edit &&
    (resource.owner_id !== actor.id ||
      !["draft", "changes_requested"].includes(resource.status))
  )
    throw new DomainError(
      403,
      "intake_not_editable",
      "Only the owner of an editable draft can prepare its source.",
    );
  const sql = database();
  const records =
    await sql`SELECT authoring, authoring_approval FROM fontes_cloud.resources WHERE id=${id}`;
  return {
    resource,
    state: (records[0].authoring ?? emptyIntake()) as IntakeState,
    approval: records[0].authoring_approval,
  };
}
export function preparationChecks(state: IntakeState): string[] {
  const f = state.fields,
    checks: string[] = [];
  if (!state.source || !state.documents.length)
    checks.push("Add a source package or EPUB and review its reading order.");
  if (state.method === "guided") {
    for (const [key, label] of [
      ["author", "Author"],
      ["publisher", "Publisher"],
      ["licenseId", "Text license"],
      ["rightsHolder", "Text rights holder"],
      ["copyright", "Copyright statement"],
      ["attribution", "Text attribution"],
      ["licenseText", "License text"],
      ["coverAlt", "Cover description"],
      ["coverLicenseId", "Cover license"],
      ["coverRightsHolder", "Cover rights holder"],
      ["coverAttribution", "Cover attribution"],
    ] as const)
      if (!f[key].trim()) checks.push(`${label} is required.`);
    if (!validRange(f.coverageStart, f.coverageEnd || undefined))
      checks.push(
        "Choose an existing canonical coverage start/end, in order (for example Matt.5.1 through Matt.7.29).",
      );
    if (!f.sourceVersification.trim())
      checks.push("Name the source versification.");
    if (!state.cover) checks.push("Upload a 1200 × 1800 PNG/JPEG cover.");
    if (
      f.permissions["offline-display"] !== "allow" ||
      f.permissions["keyword-search"] !== "allow"
    )
      checks.push(
        "Public-free preparation requires explicit offline-display and keyword-search permission.",
      );
    if (state.proposals.some((p) => p.decision === "pending"))
      checks.push("Accept or reject every detected Scripture reference.");
  }
  if (!f.rightsAttested)
    checks.push(
      "Confirm you are authorized to submit the source text and artwork.",
    );
  if (!f.conversionAcknowledged)
    checks.push(
      "Confirm you reviewed the rendered content and conversion report.",
    );
  return checks;
}
export async function intake(actor: Actor, id: string): Promise<IntakeReply> {
  const { resource, state, approval } = await row(actor, id);
  return intakeReplySchema.parse({
    resourceId: id,
    revision: resource.revision,
    state,
    approval,
    checks: preparationChecks(state),
    ai: { configured: false },
  });
}
async function replace(
  actor: Actor,
  resource: Resource,
  state: IntakeState,
  metadata?: ReturnType<typeof createResourceSchema.parse>,
) {
  if (Buffer.byteLength(JSON.stringify(state)) > 3_000_000)
    throw new DomainError(
      413,
      "draft_too_large",
      "The extracted draft exceeds 3 MB. Use offline preparation for this work.",
    );
  const sql = database();
  const m = metadata ?? resource;
  const updated =
    await sql`WITH updated AS (UPDATE fontes_cloud.resources SET title=${m.title},description=${m.description},kind=${m.kind},language=${m.language},version=${m.version},authoring=${JSON.stringify(state)}::jsonb,authoring_approval=NULL,revision=revision+1,updated_at=now()
    WHERE id=${resource.id} AND owner_id=${actor.id} AND revision=${resource.revision} AND status IN ('draft','changes_requested') RETURNING *),
    history AS (INSERT INTO fontes_cloud.authoring_history(resource_id,revision,actor_id,snapshot) SELECT id,revision,${actor.id},to_jsonb(updated) FROM updated)
    SELECT revision FROM updated`;
  if (!updated[0])
    throw new DomainError(
      409,
      "revision_conflict",
      "The draft changed while you were preparing it. Reload the current draft.",
    );
  return intake(actor, resource.id);
}
export async function saveIntake(actor: Actor, id: string, input: unknown) {
  const body = intakeSaveSchema.parse(input);
  const current = await row(actor, id, true);
  if (current.resource.revision !== body.revision)
    throw new DomainError(
      409,
      "revision_conflict",
      "Reload before saving a changed draft.",
    );
  if (current.state.source && body.method !== current.state.method)
    throw new DomainError(
      409,
      "intake_method",
      "Start another draft to change preparation paths after import.",
    );
  if (
    new Set(body.decisions.map((p) => p.id)).size !== body.decisions.length ||
    body.decisions.some(
      (p) => !current.state.proposals.some((old) => old.id === p.id),
    )
  )
    throw new DomainError(
      400,
      "unknown_proposal",
      "A reference decision does not belong to this draft.",
    );
  const state = {
    ...current.state,
    method: body.method,
    fields: body.fields,
    proposals: current.state.proposals.map((p) => ({
      ...p,
      decision:
        body.decisions.find((d) => d.id === p.id)?.decision ?? p.decision,
    })),
  };
  if (state.method === "prepared")
    state.fields = {
      ...current.state.fields,
      rightsAttested: body.fields.rightsAttested,
      conversionAcknowledged: body.fields.conversionAcknowledged,
    };
  if (
    state.method === "guided" &&
    !["book", "commentary", "study-guide", "devotional"].includes(
      body.metadata?.kind ?? current.resource.kind,
    )
  )
    throw new DomainError(
      422,
      "guided_profile",
      "Use a prepared source package for this advanced resource profile.",
    );
  return replace(
    actor,
    current.resource,
    state,
    state.method === "guided" ? body.metadata : undefined,
  );
}
async function bytes(request: Request, max = MAX_UPLOAD) {
  const reader = request.body?.getReader();
  if (!reader) throw new DomainError(400, "upload_required", "Choose a file.");
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const part = await reader.read();
    if (part.done) break;
    total += part.value.length;
    if (total > max) {
      await reader.cancel();
      throw new DomainError(
        413,
        "upload_too_large",
        `Upload is limited to ${max / 1024 / 1024} MiB.`,
      );
    }
    chunks.push(part.value);
  }
  if (!total)
    throw new DomainError(
      400,
      "upload_required",
      "The selected file is empty.",
    );
  return Buffer.concat(chunks);
}
export async function validateSource(sourcePath: string, denyWarnings = false) {
  const cli =
    process.env.FONTES_RESOURCE_CLI ??
    path.resolve(process.cwd(), "../../target/debug/fontes-resource");
  const args = [
    "validate",
    sourcePath,
    "--format",
    "json",
    ...(denyWarnings ? ["--deny-warnings"] : []),
    ...(process.env.FONTES_RESOURCE_TAXONOMY
      ? ["--taxonomy", process.env.FONTES_RESOURCE_TAXONOMY]
      : []),
  ];
  let stdout = "";
  try {
    stdout = (
      await exec(cli, args, {
        timeout: 30_000,
        maxBuffer: 4 * 1024 * 1024,
        env: {
          NODE_ENV: "development",
          PATH: process.env.PATH,
          LANG: "en_US.UTF-8",
        },
      })
    ).stdout;
  } catch (error: any) {
    if (error.stdout) stdout = error.stdout;
    else
      throw new DomainError(
        503,
        "validator_unavailable",
        "Build the trusted fontes-resource CLI and install xmllint before source validation.",
      );
  }
  const report = JSON.parse(stdout);
  if (!report.valid)
    throw new DomainError(
      422,
      "source_validation_failed",
      "The package needs changes before it can be imported.",
      { diagnostics: report.diagnostics.slice(0, 100) },
    );
  return report;
}
export async function uploadIntake(
  actor: Actor,
  id: string,
  request: Request,
  kind: "epub" | "prepared" | "cover",
) {
  localOnly();
  const current = await row(actor, id, true);
  const revision = Number(request.headers.get("x-fontes-revision"));
  if (revision !== current.resource.revision)
    throw new DomainError(
      409,
      "revision_conflict",
      "Reload the current draft before uploading.",
    );
  const buffer = await bytes(
      request,
      kind === "cover" ? 4 * 1024 * 1024 : MAX_UPLOAD,
    ),
    digest = hash(buffer);
  let state = current.state;
  let importedMetadata:
    | ReturnType<typeof createResourceSchema.parse>
    | undefined;
  if (kind === "cover") {
    if (state.method === "prepared")
      throw new DomainError(
        409,
        "prepared_immutable",
        "Replace artwork in the offline source, then upload a revised package.",
      );
    const metadata = await sharp(buffer, { limitInputPixels: 2_160_000 })
      .metadata()
      .catch(() => null);
    if (
      !metadata ||
      !["png", "jpeg"].includes(metadata.format ?? "") ||
      metadata.width !== 1200 ||
      metadata.height !== 1800 ||
      (metadata.pages ?? 1) !== 1 ||
      (metadata.orientation ?? 1) !== 1
    )
      throw new DomainError(
        422,
        "cover_dimensions",
        "Use a single-frame 1200 × 1800 PNG/JPEG, upright and at most 4 MiB.",
      );
    await sharp(buffer, { limitInputPixels: 2_160_000 }).raw().toBuffer();
    state = {
      ...state,
      cover: {
        sha256: digest,
        bytes: buffer.length,
        mediaType: metadata.format === "png" ? "image/png" : "image/jpeg",
      },
    };
  } else {
    if ((kind === "prepared") !== (state.method === "prepared"))
      throw new DomainError(
        409,
        "intake_method",
        "The file does not match this draft's preparation path.",
      );
    const files = await readArchive(buffer);
    if (kind === "prepared") {
      const temp = await mkdtemp(
        path.join(os.tmpdir(), "fontes-source-check-"),
      );
      try {
        const archive = path.join(temp, "upload.fontes-source.zip");
        await writeFile(archive, buffer, { mode: 0o600 });
        const report = await validateSource(archive);
        state = importPrepared(files);
        const metadata = xml(files.get("metadata.xml")!.toString()).resource;
        state.source = {
          sha256: digest,
          name: "Prepared source package",
          bytes: buffer.length,
          format: "fontes-source",
          resourceId: metadata["@_id"],
          publisherId: metadata.publisher["@_id"],
          report,
        };
        importedMetadata = createResourceSchema.parse({
          title: xmlText(metadata.title),
          description: xmlText(metadata.description),
          kind: metadata["@_kind"],
          version: metadata["@_version"],
          language: metadata["@_language"],
        });
        const coverBytes = files.get(metadata.cover["@_href"])!,
          coverDigest = hash(coverBytes);
        state.cover = {
          sha256: coverDigest,
          bytes: coverBytes.length,
          mediaType: metadata.cover["@_media-type"],
        };
        await mkdir(path.dirname(assetPath(id, coverDigest)), {
          recursive: true,
          mode: 0o700,
        });
        await writeFile(assetPath(id, coverDigest), coverBytes, {
          mode: 0o600,
        });
        state.warnings.push(
          "The package namespace is a submitted claim. Uploading a valid archive does not verify publisher authority or grant publication permission.",
        );
      } finally {
        await rm(temp, { recursive: true, force: true });
      }
    } else {
      const extracted = importEpub(files);
      state = {
        ...extracted,
        fields: {
          ...current.state.fields,
          rightsAttested: false,
          conversionAcknowledged: false,
        },
        cover: current.state.cover,
        source: {
          sha256: digest,
          name: "Imported EPUB",
          bytes: buffer.length,
          format: "epub",
        },
      };
    }
  }
  await mkdir(path.dirname(assetPath(id, digest)), {
    recursive: true,
    mode: 0o700,
  });
  await writeFile(assetPath(id, digest), buffer, { mode: 0o600 });
  return replace(actor, current.resource, state, importedMetadata);
}
export async function approveIntake(
  actor: Actor,
  id: string,
  revision: number,
) {
  const { resource, state } = await row(actor, id, true);
  if (resource.revision !== revision)
    throw new DomainError(
      409,
      "revision_conflict",
      "Reload the draft before approving it.",
    );
  const checks = preparationChecks(state);
  if (checks.length)
    throw new DomainError(422, "preparation_incomplete", checks[0], { checks });
  localOnly();
  const original = await intakeAsset(actor, id, "source");
  const prepared =
    state.method === "prepared"
      ? original
      : await (
          await import("./intake-package")
        ).buildSource(resource, state, await intakeAsset(actor, id, "cover"));
  const packageDigest = hash(prepared.buffer);
  const digest = preparationDigest(resource, state);
  const sql = database();
  const result =
    await sql`WITH updated AS (UPDATE fontes_cloud.resources SET authoring_approval=${JSON.stringify({ digest, package_sha256: packageDigest, actor: actor.id, at: new Date().toISOString() })}::jsonb,revision=revision+1,updated_at=now() WHERE id=${id} AND owner_id=${actor.id} AND revision=${revision} AND status IN ('draft','changes_requested') RETURNING *),
    history AS (INSERT INTO fontes_cloud.authoring_history(resource_id,revision,actor_id,snapshot) SELECT id,revision,${actor.id},to_jsonb(updated) FROM updated) SELECT id FROM updated`;
  if (!result[0])
    throw new DomainError(
      409,
      "revision_conflict",
      "Another edit changed this draft.",
    );
  return intake(actor, id);
}
export async function intakeAsset(
  actor: Actor,
  id: string,
  kind: "cover" | "source",
) {
  localOnly();
  const { state } = await row(actor, id);
  const asset = kind === "cover" ? state.cover : state.source;
  if (!asset)
    throw new DomainError(
      404,
      "asset_not_found",
      "This draft has no such asset.",
    );
  const buffer = await readFile(assetPath(id, asset.sha256));
  if (hash(buffer) !== asset.sha256)
    throw new DomainError(
      503,
      "asset_integrity",
      "The local asset failed its integrity check.",
    );
  return {
    buffer,
    mediaType: kind === "cover" ? state.cover!.mediaType : "application/zip",
  };
}
export async function exportIntake(actor: Actor, id: string) {
  localOnly();
  const current = await row(actor, id);
  if (current.resource.owner_id !== actor.id)
    throw new DomainError(
      403,
      "forbidden",
      "Only the owner can export this preparation draft.",
    );
  if (
    !current.approval ||
    current.approval.digest !==
      preparationDigest(current.resource, current.state)
  )
    throw new DomainError(
      409,
      "publisher_approval_required",
      "Review and approve this exact preparation draft before downloading a package.",
    );
  if (current.state.method === "prepared") {
    const asset = await intakeAsset(actor, id, "source");
    if (hash(asset.buffer) !== current.approval.package_sha256)
      throw new DomainError(
        409,
        "package_changed",
        "Package bytes changed. Review and approve the preparation again.",
      );
    return {
      buffer: asset.buffer,
      filename: `${current.state.source!.resourceId}--${current.resource.version}.fontes-source.zip`,
    };
  }
  const { buildSource } = await import("./intake-package");
  const output = await buildSource(
    current.resource,
    current.state,
    await intakeAsset(actor, id, "cover"),
  );
  if (hash(output.buffer) !== current.approval.package_sha256)
    throw new DomainError(
      409,
      "package_changed",
      "Package bytes changed. Review and approve the preparation again.",
    );
  return output;
}
