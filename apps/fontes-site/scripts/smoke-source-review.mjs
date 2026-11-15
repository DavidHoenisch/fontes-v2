import assert from "node:assert/strict";
import { readFile, writeFile, mkdtemp, rm, mkdir, cp } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { createHash } from "node:crypto";
import { neon } from "@neondatabase/serverless";
import yauzl from "yauzl";
import { zip as fixtureZip, epubEntries } from "../tests/intake-fixtures.ts";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error("Use dedicated development mode.");
const sql = neon(process.env.DATABASE_URL),
  origin = "http://127.0.0.1:3000",
  created = [],
  temp = await mkdtemp(path.join(os.tmpdir(), "fontes-maintainer-test-"));
let checks = 0,
  orgId;
async function call(
  path,
  {
    persona = "publisher",
    method = "GET",
    body,
    bytes,
    headers = {},
    expected = 200,
    binary = false,
  } = {},
) {
  const r = await fetch(`${origin}/api/v1/${path}`, {
    method,
    headers: {
      Origin: origin,
      Cookie: `fontes_dev_persona=${persona}`,
      "Content-Type": bytes ? "application/octet-stream" : "application/json",
      ...headers,
    },
    ...(bytes ? { body: bytes } : body ? { body: JSON.stringify(body) } : {}),
  });
  if (r.status !== expected)
    throw new Error(`${method} ${path}: ${r.status} ${await r.text()}`);
  checks++;
  return binary ? Buffer.from(await r.arrayBuffer()) : (await r.json()).data;
}
async function unzip(bytes) {
  return new Promise((resolve, reject) =>
    yauzl.fromBuffer(bytes, { lazyEntries: true }, (e, zip) => {
      if (e) return reject(e);
      const out = new Map();
      zip.on("error", reject);
      zip.on("end", () => resolve(out));
      zip.on("entry", (entry) =>
        zip.openReadStream(entry, (e, s) => {
          if (e) return reject(e);
          const chunks = [];
          s.on("data", (b) => chunks.push(b));
          s.on("error", reject);
          s.on("end", () => {
            out.set(entry.fileName, Buffer.concat(chunks));
            zip.readEntry();
          });
        }),
      );
      zip.readEntry();
    }),
  );
}
try {
  const resource = await call("resources", {
    method: "POST",
    body: {
      title: "Original source review fixture",
      description:
        "Synthetic original resource for maintainer review and unsigned publication handoff checks.",
      kind: "study-guide",
    },
    expected: 201,
  });
  created.push(resource.id);
  let draft = await call(`resources/${resource.id}/intake`),
    intake = `resources/${resource.id}/intake`;
  const cli = path.resolve("../../target/debug/fontes-resource"),
    source = path.join(temp, "source"),
    input = path.join(temp, `${resource.id}--1.0.0.fontes-source.zip`);
  await cp(
    "../../library/submissions/org.example.romans-study/source",
    source,
    { recursive: true },
  );
  await rm(path.join(source, "taxonomy"), { recursive: true });
  for (const name of [
    "index.xml",
    "metadata.xml",
    "content/rom/001.xml",
    "content/introductions/romans.xml",
  ]) {
    const f = path.join(source, name);
    let text = await readFile(f, "utf8");
    text = text
      .replaceAll("org.example.romans-study", resource.id)
      .replaceAll(
        'publisher id="org.example"',
        'publisher id="dev.fontes.alex"',
      )
      .replace(/<term[^>]*>(.*?)<\/term>/gs, "$1")
      .replace(/<concept[^>]*\/>/g, "")
      .replace(/<annotations>\s*<\/annotations>/g, "");
    await writeFile(f, text);
  }
  await promisify(execFile)(cli, [
    "pack",
    source,
    "--output",
    input,
    "--deny-warnings",
  ]);
  const bytes = await readFile(input);
  const uploadBytes = await fixtureZip(Object.fromEntries(await unzip(bytes)));
  assert.notEqual(
    createHash("sha256").update(uploadBytes).digest("hex"),
    createHash("sha256").update(bytes).digest("hex"),
  );
  draft = await call(intake, {
    method: "PUT",
    body: {
      revision: draft.revision,
      method: "prepared",
      fields: draft.state.fields,
    },
  });
  draft = await call(`${intake}/prepared`, {
    method: "POST",
    bytes: uploadBytes,
    headers: { "X-Fontes-Revision": String(draft.revision) },
  });
  const submit = () => ({
    source: { kind: "personal", resourceId: resource.id },
    resourceRevision: draft.revision,
    consent: true,
    releaseNotes:
      "Original CC0 fixture submitted solely for synthetic maintainer workflow tests.",
  });
  await call("source-reviews", {
    method: "POST",
    body: submit(),
    expected: 409,
  });
  draft = await call(intake, {
    method: "PUT",
    body: {
      revision: draft.revision,
      method: "prepared",
      fields: {
        ...draft.state.fields,
        rightsAttested: true,
        conversionAcknowledged: true,
      },
    },
  });
  draft = await call(`${intake}/approve`, {
    method: "POST",
    body: { revision: draft.revision },
  });
  await call("source-reviews", {
    method: "POST",
    body: { ...submit(), consent: false },
    expected: 400,
  });
  let detail = await call("source-reviews", {
    method: "POST",
    body: submit(),
    expected: 201,
  });
  const id = detail.item.id,
    root = `source-reviews/${id}`;
  assert.equal(
    detail.item.upstream_sha256,
    createHash("sha256").update(uploadBytes).digest("hex"),
  );
  checks++;
  assert.equal(
    detail.item.source_sha256,
    createHash("sha256").update(bytes).digest("hex"),
  );
  checks++;
  await call("source-reviews", {
    method: "POST",
    body: submit(),
    expected: 409,
  });
  async function command(
    c,
    expected = 200,
    persona = "reviewer",
    version = detail.item.revision,
  ) {
    const r = await call(root, {
      persona,
      method: "PATCH",
      body: {
        revision: version,
        sourceSha256: detail.item.source_sha256,
        command: c,
      },
      expected,
    });
    if (expected === 200) detail = r;
    return r;
  }
  await command({ action: "handoff" }, 409);
  const document = await call(
    `${root}/document?document=${detail.preview.documents.find((d) => d.id === "rom-001").id}`,
    { persona: "reviewer" },
  );
  assert.ok(
    document.sourceAssertions?.length ||
      document.blocks.some((b) => b.sourceAssertions?.length),
  );
  checks++;
  await call(`${root}/file?path=../../.env.local`, {
    persona: "reviewer",
    expected: 404,
  });
  const zip = await call(`${root}/package`, {
    persona: "reviewer",
    binary: true,
  });
  assert.deepEqual(zip, bytes);
  checks++;
  await command(
    {
      action: "approve",
      note: "Synthetic fixture review only",
      rightsReviewId: "cc0-original-fixture-only",
      publisherReviewId: "reserved-fictional-example-only",
      checks: {
        publisher: true,
        textRights: true,
        artworkRights: true,
        content: true,
        references: false,
      },
    },
    400,
  );
  const approved = {
    action: "approve",
    note: "Original CC0 prose and artwork reviewed as a synthetic test fixture; no production publisher authority is asserted.",
    rightsReviewId: "cc0-original-fixture-only",
    publisherReviewId: "reserved-fictional-example-only",
    checks: {
      publisher: true,
      textRights: true,
      artworkRights: true,
      content: true,
      references: true,
    },
  };
  await command(approved, 403, "publisher");
  const stale = detail.item.revision;
  await command({
    action: "comment",
    note: "Please clarify the fixture source provenance.",
  });
  await command(approved, 409, "reviewer", stale);
  await command({
    action: "request_changes",
    note: "Clarify the original fixture attribution before release.",
  });
  assert.equal(detail.item.status, "changes_requested");
  checks++;
  await command(approved, 409);
  // A new publisher approval is a new origin revision and invalidates the old review.
  draft = await call(`${intake}/approve`, {
    method: "POST",
    body: { revision: draft.revision },
  });
  const previous = await call(root, { persona: "reviewer" });
  assert.equal(previous.item.status, "superseded");
  checks++;
  await call(`${root}/package`, { persona: "reviewer", expected: 409 });
  detail = await call("source-reviews", {
    method: "POST",
    body: submit(),
    expected: 201,
  });
  const next = `source-reviews/${detail.item.id}`;
  async function decide(c, expected = 200) {
    const r = await call(next, {
      persona: "reviewer",
      method: "PATCH",
      body: {
        revision: detail.item.revision,
        sourceSha256: detail.item.source_sha256,
        command: c,
      },
      expected,
    });
    if (expected === 200) detail = r;
    return r;
  }
  await decide(approved);
  assert.equal(detail.item.status, "approved");
  checks++;
  await decide({ action: "handoff" });
  assert.equal(detail.item.status, "handoff_ready");
  checks++;
  const handoff = await call(`${next}/handoff`, {
      persona: "reviewer",
      binary: true,
    }),
    files = await unzip(handoff);
  const descriptor = JSON.parse(
      files.get(`submissions/${detail.item.resource_id}/submission.json`),
    ),
    approval = JSON.parse(
      files.get(
        `approvals/${detail.item.resource_id}--${detail.item.version}.json`,
      ),
    );
  assert.equal(approval.source_sha256, detail.item.source_sha256);
  assert.equal(approval.distribution, "public_free");
  assert.equal(descriptor.source.type, "path");
  assert.equal(files.has("policy.json"), false);
  checks++;
  await decide({ action: "handoff" });
  assert.deepEqual(
    await call(`${next}/handoff`, { persona: "reviewer", binary: true }),
    handoff,
  );
  checks++;
  // Consume the exported descriptors with the real, separately trusted CLI using isolated synthetic test policy.
  const library = path.join(temp, "library");
  await mkdir(library);
  for (const [name, b] of files)
    if (name.startsWith("submissions/") || name.startsWith("approvals/")) {
      const dest = path.join(library, name);
      await mkdir(path.dirname(dest), { recursive: true });
      await writeFile(dest, b);
    }
  const policy = JSON.parse(
    await readFile("../../library/policy.json", "utf8"),
  );
  policy.publishers.push("dev.fontes.alex");
  await writeFile(path.join(library, "policy.json"), JSON.stringify(policy));
  await writeFile(
    path.join(library, "taxonomy.xml"),
    await readFile("../../library/taxonomy.xml"),
  );
  await mkdir(path.join(library, "trust"));
  await writeFile(
    path.join(library, "trust/source-release-1.json"),
    await readFile("../../library/trust/source-release-1.json"),
  );
  await promisify(execFile)(cli, [
    "submissions",
    "check",
    "--root",
    library,
    "--policy-root",
    library,
    "--output",
    path.join(temp, "validated"),
    "--repository",
    "DavidHoenisch/fontes-v2",
    "--source-repository",
    "DavidHoenisch/fontes-v2",
    "--source-commit",
    "0".repeat(40),
    "--tooling-commit",
    "0".repeat(40),
  ]);
  checks++;
  const trustedReport=JSON.parse(await readFile(path.join(temp,"validated/submission-report.json"),"utf8"));
  assert.equal(trustedReport.valid,true);assert.equal(trustedReport.submissions[0].status,"ready");assert.equal(trustedReport.submissions[0].approved,true);checks++;
  await call(next, {
    method: "PATCH",
    body: {
      revision: detail.item.revision,
      sourceSha256: detail.item.source_sha256,
      command: {
        action: "withdraw",
        note: "Withdraw this synthetic fixture after validation.",
      },
    },
  });
  await call(`${next}/handoff`, { persona: "reviewer", expected: 409 });
  const audit = await call(next, { persona: "reviewer" });
  assert.equal(audit.item.approval, null);
  assert.equal(
    audit.events.find((e) => e.action === "approve").details.approval
      .rightsReviewId,
    approved.rightsReviewId,
  );
  checks++;
  // Changed canonical bytes cannot reuse a resource version already handed off.
  const metadataFile = path.join(source, "metadata.xml");
  await writeFile(
    metadataFile,
    (await readFile(metadataFile, "utf8")).replace(
      "<description>",
      "<description>Revised original fixture. ",
    ),
  );
  const revisedDir = path.join(temp, "revision");
  await mkdir(revisedDir);
  const revisedZip = path.join(
    revisedDir,
    `${resource.id}--1.0.0.fontes-source.zip`,
  );
  await promisify(execFile)(cli, [
    "pack",
    source,
    "--output",
    revisedZip,
    "--deny-warnings",
  ]);
  draft = await call(`${intake}/prepared`, {
    method: "POST",
    bytes: await readFile(revisedZip),
    headers: { "X-Fontes-Revision": String(draft.revision) },
  });
  draft = await call(intake, {
    method: "PUT",
    body: {
      revision: draft.revision,
      method: "prepared",
      fields: {
        ...draft.state.fields,
        rightsAttested: true,
        conversionAcknowledged: true,
      },
    },
  });
  draft = await call(`${intake}/approve`, {
    method: "POST",
    body: { revision: draft.revision },
  });
  const changed = await call("source-reviews", {
    method: "POST",
    body: submit(),
    expected: 201,
  });
  const changedPath = `source-reviews/${changed.item.id}`;
  const changedApproval = await call(changedPath, {
    persona: "reviewer",
    method: "PATCH",
    body: {
      revision: changed.item.revision,
      sourceSha256: changed.item.source_sha256,
      command: approved,
    },
  });
  await call(changedPath, {
    persona: "reviewer",
    method: "PATCH",
    body: {
      revision: changedApproval.item.revision,
      sourceSha256: changedApproval.item.source_sha256,
      command: { action: "handoff" },
    },
    expected: 409,
  });
  const history = await call("source-reviews?history=true", {
    persona: "reviewer",
  });
  assert.ok(history.items.some((r) => r.id === detail.item.id));
  checks++;

  // Organization drafts remain inaccessible until explicit public-intake submission.
  let organization = await call("organizations", {
    method: "POST",
    body: { name: "Synthetic maintainer intake team" },
    expected: 201,
  });
  orgId = organization.organization.id;
  organization = await call(`organizations/${orgId}/resources`, {
    method: "POST",
    body: {
      revision: organization.organization.revision,
      title: "Original org source review fixture",
      description:
        "An original synthetic organization resource used only for explicit intake scope tests.",
      kind: "book",
    },
    expected: 201,
  });
  const orgResource = organization.resources[0],
    prepPath = `organizations/${orgId}/resources/${orgResource.id}/preparation`;
  let prep = await call(prepPath);
  const orgHeaders = () => ({
    "X-Fontes-Revision": String(prep.resourceRevision),
    "X-Fontes-Organization-Revision": String(prep.organizationRevision),
    "X-Fontes-Review-Revision": String(prep.reviewRevision),
  });
  prep = await call(prepPath, {
    method: "POST",
    bytes: await fixtureZip(epubEntries()),
    headers: orgHeaders(),
  });
  async function orgCommand(command) {
    prep = await call(prepPath, {
      method: "PATCH",
      body: {
        resourceRevision: prep.resourceRevision,
        organizationRevision: prep.organizationRevision,
        reviewRevision: prep.reviewRevision,
        command,
      },
    });
  }
  const fields = {
    ...draft.state.fields,
    author: "Original synthetic author",
    publisher: "Original synthetic team",
    licenseText:
      "Original synthetic prose and CC0 format-example placeholder artwork for tests.",
    rightsAttested: true,
    conversionAcknowledged: true,
    coverageStart: "John.3.16",
  };
  await orgCommand({ action: "fields", fields });
  prep = await call(`${prepPath}/cover`, {
    method: "POST",
    bytes: (await unzip(bytes)).get("assets/cover.png"),
    headers: orgHeaders(),
  });
  for (const d of prep.manifest.documents) {
    const page = await call(`${prepPath}/document?document=${d.id}`);
    for (const b of page.blocks.filter((b) => b.references.length))
      await orgCommand({
        action: "correct",
        documentId: d.id,
        blockId: b.id,
        segmentTexts: b.segments.map((s) => s.text),
        references: b.references.map((r) => ({ ...r, decision: "accepted" })),
      });
  }
  await orgCommand({ action: "approve" });
  await orgCommand({ action: "package" });
  await call(prepPath, { persona: "reviewer", expected: 404 });
  let queue = await call("source-reviews", { persona: "reviewer" });
  assert.equal(
    queue.items.some((r) => r.organization_resource_id === orgResource.id),
    false,
  );
  checks++;
  const orgSubmission = {
    source: {
      kind: "organization",
      organizationId: orgId,
      resourceId: orgResource.id,
      organizationRevision: prep.organizationRevision,
      reviewRevision: prep.reviewRevision,
    },
    resourceRevision: prep.resourceRevision,
    consent: true,
    releaseNotes:
      "Original organization fixture expressly shared for synthetic Fontes maintainer review.",
  };
  let shared = await call("source-reviews", {
    method: "POST",
    body: orgSubmission,
    expected: 201,
  });
  const sharedPath = `source-reviews/${shared.item.id}`;
  shared = await call(sharedPath, { persona: "reviewer" });
  assert.equal(shared.item.independent, true);
  checks++;
  await call(prepPath, { persona: "reviewer", expected: 404 });
  await call(`${sharedPath}/package`, { persona: "reviewer", binary: true });
  await call(`organizations/${orgId}/resources/${orgResource.id}`, {
    method: "PATCH",
    body: {
      revision: prep.organizationRevision,
      resource_revision: prep.resourceRevision,
      title: "Revised original org source review fixture",
      description:
        "An original synthetic organization resource after metadata corrections.",
      kind: "book",
      language: "en",
      version: "1.0.0",
    },
  });
  shared = await call(sharedPath, { persona: "reviewer" });
  assert.equal(shared.item.status, "superseded");
  checks++;
  await call(sharedPath, {
    persona: "reviewer",
    method: "PATCH",
    body: {
      revision: shared.item.revision,
      sourceSha256: shared.item.source_sha256,
      command: approved,
    },
    expected: 409,
  });
  await call(`${sharedPath}/package`, { persona: "reviewer", expected: 409 });
  // Reviewer capability does not authorize self-approval of a publisher-origin snapshot.
  const self = await call("resources", {
    persona: "reviewer",
    method: "POST",
    body: {
      title: "Self review fixture",
      description: "Synthetic source for the independent maintainer invariant.",
      kind: "book",
    },
    expected: 201,
  });
  created.push(self.id);
  const selfPath = `resources/${self.id}/intake`;
  let own = await call(selfPath, { persona: "reviewer" });
  own = await call(selfPath, {
    persona: "reviewer",
    method: "PUT",
    body: {
      revision: own.revision,
      method: "prepared",
      fields: own.state.fields,
    },
  });
  own = await call(`${selfPath}/prepared`, {
    persona: "reviewer",
    method: "POST",
    bytes,
    headers: { "X-Fontes-Revision": String(own.revision) },
  });
  own = await call(selfPath, {
    persona: "reviewer",
    method: "PUT",
    body: {
      revision: own.revision,
      method: "prepared",
      fields: {
        ...own.state.fields,
        rightsAttested: true,
        conversionAcknowledged: true,
      },
    },
  });
  own = await call(`${selfPath}/approve`, {
    persona: "reviewer",
    method: "POST",
    body: { revision: own.revision },
  });
  const ownReview = await call("source-reviews", {
    persona: "reviewer",
    method: "POST",
    body: {
      source: { kind: "personal", resourceId: self.id },
      resourceRevision: own.revision,
      consent: true,
      releaseNotes:
        "Original synthetic work for self-review permission checks.",
    },
    expected: 201,
  });
  await call(`source-reviews/${ownReview.item.id}`, {
    persona: "reviewer",
    method: "PATCH",
    body: {
      revision: ownReview.item.revision,
      sourceSha256: ownReview.item.source_sha256,
      command: approved,
    },
    expected: 403,
  });
  await call(`source-reviews/${ownReview.item.id}`, { expected: 404 });
  await call(`source-reviews/${ownReview.item.id}/package`, { expected: 404 });
  console.log(
    `${checks} maintainer source-review HTTP/Postgres/Rust checks passed: frozen bytes, independent review, feedback, changes/resubmission, stale decisions, supersession, canonical approval and unsigned reproducible handoff accepted by trusted tooling.`,
  );
} finally {
  if (orgId)
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.source_handoff_versions WHERE review_id IN (SELECT id FROM fontes_cloud.source_reviews WHERE organization_id=${orgId})`,
      sql`DELETE FROM fontes_cloud.source_review_events WHERE review_id IN (SELECT id FROM fontes_cloud.source_reviews WHERE organization_id=${orgId})`,
      sql`DELETE FROM fontes_cloud.source_reviews WHERE organization_id=${orgId}`,
      sql`DELETE FROM fontes_cloud.organization_publisher_reviews WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${orgId})`,
      sql`DELETE FROM fontes_cloud.organization_epub_history WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${orgId})`,
      sql`DELETE FROM fontes_cloud.organization_epub_preparations WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${orgId})`,
      ...[
        "organization_events",
        "organization_invitations",
        "organization_policy_revisions",
        "organization_members",
        "organization_resources",
      ].map((t) =>
        sql.query(`DELETE FROM fontes_cloud.${t} WHERE organization_id=$1`, [
          orgId,
        ]),
      ),
      sql`DELETE FROM fontes_cloud.organizations WHERE id=${orgId} AND is_demo=true`,
    ]);
  for (const id of created)
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.source_handoff_versions WHERE review_id IN (SELECT id FROM fontes_cloud.source_reviews WHERE personal_resource_id=${id})`,
      sql`DELETE FROM fontes_cloud.source_review_events WHERE review_id IN (SELECT id FROM fontes_cloud.source_reviews WHERE personal_resource_id=${id})`,
      sql`DELETE FROM fontes_cloud.source_reviews WHERE personal_resource_id=${id}`,
      sql`DELETE FROM fontes_cloud.authoring_history WHERE resource_id=${id}`,
      sql`DELETE FROM fontes_cloud.events WHERE resource_id=${id}`,
      sql`DELETE FROM fontes_cloud.resources WHERE id=${id} AND is_demo=true AND owner_id IN ('dev-publisher','dev-reviewer')`,
    ]);
  await rm(temp, { recursive: true, force: true });
}
