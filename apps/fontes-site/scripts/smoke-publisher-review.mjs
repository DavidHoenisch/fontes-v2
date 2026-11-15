import sharp from "sharp";
import yauzl from "yauzl";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { neon } from "@neondatabase/serverless";
import { ZipFile } from "yazl";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error("Use the dedicated development configuration.");
const sql = neon(process.env.DATABASE_URL),
  base = "http://127.0.0.1:3000";
let org,
  checks = 0;
async function call(
  path,
  {
    persona = "reviewer",
    method = "GET",
    body,
    bytes,
    headers = {},
    expected = 200,
  } = {},
) {
  const r = await fetch(`${base}/api/v1/${path}`, {
    method,
    headers: {
      Origin: base,
      Cookie: `fontes_dev_persona=${persona}`,
      "Content-Type": bytes ? "application/epub+zip" : "application/json",
      ...headers,
    },
    ...(bytes
      ? { body: bytes }
      : body !== undefined
        ? { body: JSON.stringify(body) }
        : {}),
  });
  if (!r.headers.get("content-type")?.includes("application/json"))
    throw new Error(`${path} did not return JSON (${r.status}).`);
  const result = await r.json();
  assert.equal(
    r.status,
    expected,
    `${method} ${path}: ${result.error?.code ?? "unexpected status"} ${JSON.stringify(result.error?.details ?? "")}`,
  );
  checks++;
  return result.data;
}
async function fixture() {
  const zip = new ZipFile();
  for (const [name, value] of Object.entries({
    mimetype: "application/epub+zip",
    "META-INF/container.xml":
      '<container><rootfiles><rootfile full-path="EPUB/package.opf"/></rootfiles></container>',
    "EPUB/package.opf":
      '<package><metadata><title>Original evaluation fixture</title><creator>Fontes test</creator><rights>Original synthetic text for testing.</rights></metadata><manifest><item id="picture" href="picture.png" media-type="image/png"/><item id="chapter" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="chapter"/></spine></package>',
    "EPUB/picture.png": await sharp({
      create: { width: 64, height: 64, channels: 3, background: "#235f79" },
    })
      .png()
      .toBuffer(),
    "EPUB/chapter.xhtml":
      '<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>Test chapter</title></head><body><img src="picture.png" alt="Original synthetic blue square"/><span id="page"/><h1>Original fixture</h1>' +
      Array.from(
        { length: 2 },
        (_, i) => `<p id="p${i}">Original test paragraph ${i}. John 3:16.</p>`,
      ).join("") +
      '<p id="ref">A note reference<a href="#n" epub:type="noteref"><sup>1</sup></a>.</p><aside id="n" epub:type="footnote"><a href="#ref">1</a><span>Original footnote text.</span></aside></body></html>',
  })) {
    zip.addBuffer(Buffer.from(value), name);
  }
  const done = new Promise((resolve, reject) => {
    const chunks = [];
    zip.outputStream.on("data", (c) => chunks.push(c));
    zip.outputStream.on("end", () => resolve(Buffer.concat(chunks)));
    zip.outputStream.on("error", reject);
  });
  zip.end();
  return done;
}
async function unzip(bytes) {
  return new Promise((resolve, reject) => {
    yauzl.fromBuffer(bytes, { lazyEntries: true }, (error, zip) => {
      if (error) return reject(error);
      const files = new Map();
      zip.on("error", reject);
      zip.on("end", () => resolve(files));
      zip.on("entry", (entry) =>
        zip.openReadStream(entry, (error, stream) => {
          if (error) return reject(error);
          const chunks = [];
          stream.on("error", reject);
          stream.on("data", (b) => chunks.push(b));
          stream.on("end", () => {
            files.set(entry.fileName, Buffer.concat(chunks));
            zip.readEntry();
          });
        }),
      );
      zip.readEntry();
    });
  });
}
try {
  let d = await call("organizations", {
    method: "POST",
    body: { name: "Synthetic publisher approval checks" },
    expected: 201,
  });
  org = d.organization.id;
  const root = `organizations/${org}`;
  d = await call(`${root}/resources`, {
    method: "POST",
    body: {
      title: "Original publisher review fixture",
      description:
        "Original synthetic content for testing publisher corrections and source packaging.",
      kind: "book",
      revision: d.organization.revision,
    },
    expected: 201,
  });
  const id = d.resources[0].id,
    path = `${root}/resources/${id}/preparation`,
    bytes = await fixture();
  let reply = await call(path);
  const versions = () => ({
    resourceRevision: reply.resourceRevision,
    organizationRevision: reply.organizationRevision,
    reviewRevision: reply.reviewRevision,
  });
  const uploadHeaders = () => ({
    "X-Fontes-Revision": String(reply.resourceRevision),
    "X-Fontes-Organization-Revision": String(reply.organizationRevision),
    "X-Fontes-Review-Revision": String(reply.reviewRevision),
  });
  reply = await call(path, { method: "POST", bytes, headers: uploadHeaders() });
  const doc = reply.manifest.documents[0];
  const firstPending = await call(
    `${path}/document?document=${doc.id}&pending=next`,
  );
  assert.ok(firstPending.focusBlockId);
  checks++;

  const page = () => call(`${path}/document?document=${doc.id}`);
  async function mutate(command, expected = 200, extra = {}) {
    const r = await call(path, {
      method: "PATCH",
      body: { ...versions(), command },
      expected,
      ...extra,
    });
    if (expected === 200) reply = r;
    return r;
  }
  await mutate({ action: "approve" }, 422);
  await mutate({ action: "package" }, 409);
  await call(`${path}/package`, { expected: 409 });
  await mutate({ action: "approve" }, 404, { persona: "publisher" });
  await mutate({ action: "approve" }, 403, { headers: { Origin: "" } });
  d = await call(root);
  const invitation = await call(`${root}/invitations`, {
    method: "POST",
    body: {
      email: "publisher@example.invalid",
      role: "member",
      revision: d.organization.revision,
    },
    expected: 201,
  });
  await call("invitations/accept", {
    persona: "publisher",
    method: "POST",
    body: { token: invitation.token },
  });
  reply = await call(path);
  await mutate({ action: "approve" }, 404, { persona: "publisher" });
  await call(`${path}/package`, { persona: "publisher", expected: 404 });
  await call(`${path}/cover`, {
    persona: "publisher",
    method: "POST",
    bytes: Buffer.from("not a cover"),
    headers: uploadHeaders(),
    expected: 404,
  });
  d = await call(root);
  await call(`${root}/members/dev-publisher`, {
    method: "PATCH",
    body: { revision: d.organization.revision, role: "publisher" },
  });
  await call(path, { persona: "publisher" });
  d = await call(root);
  await call(`${root}/members/dev-publisher`, {
    method: "DELETE",
    body: { revision: d.organization.revision },
  });
  await mutate({ action: "approve" }, 404, { persona: "publisher" });
  reply = await call(path);
  let p = await page(),
    block = p.blocks.find((b) => b.references.length);
  const oldVersions = versions();
  await mutate({
    action: "correct",
    documentId: doc.id,
    blockId: block.id,
    segmentTexts: block.segments.map((s) =>
      s.text.replace("John 3:16", "Romans 8:28"),
    ),
    references: block.references,
  });
  p = await page();
  block = p.blocks.find((b) => b.id === block.id);
  assert.match(block.text, /Romans 8:28/);
  assert.match(block.originalText, /John 3:16/);
  assert.equal(block.references[0].decision, "pending");
  checks++;
  const command = {
    action: "correct",
    documentId: doc.id,
    blockId: block.id,
    segmentTexts: block.segments.map((s) => s.text),
    references: block.references.map((r) => ({ ...r, decision: "accepted" })),
  };
  await call(path, {
    method: "PATCH",
    body: { ...oldVersions, command },
    expected: 409,
  });
  await mutate(
    {
      ...command,
      references: command.references.map((r) => ({ ...r, start: "Rom.8.999" })),
    },
    422,
  );
  await mutate(
    {
      ...command,
      references: command.references.map((r) => ({
        ...r,
        evidence: "not present",
      })),
    },
    422,
  );
  await mutate(
    { ...command, references: [...command.references, command.references[0]] },
    422,
  );
  await mutate(command);
  p = await page();
  const second = p.blocks.find((b) =>
    b.references.some((r) => r.decision === "pending"),
  );
  await mutate({
    action: "correct",
    documentId: doc.id,
    blockId: second.id,
    segmentTexts: second.segments.map((s) => s.text),
    references: second.references.map((r) => ({ ...r, decision: "rejected" })),
  });
  const note = p.blocks.find((b) => b.kind === "note"),
    noteTarget = note.segments[0].target;
  await mutate({
    action: "correct",
    documentId: doc.id,
    blockId: note.id,
    segmentTexts: note.segments.map((s) => s.text),
    references: [
      {
        id: "manual-test",
        blockId: note.id,
        evidence: "Original footnote",
        start: "Ps.23.1",
        method: "publisher-correction-v1",
        decision: "accepted",
      },
    ],
  });
  assert.deepEqual(
    (await page()).blocks.find((b) => b.id === note.id).segments[0].target,
    noteTarget,
  );
  checks++;
  assert.equal(reply.draft.pending, 0);
  assert.equal(reply.draft.accepted, 2);
  assert.equal(reply.draft.rejected, 1);
  await call(`${path}/document?document=${doc.id}&pending=next`, {
    expected: 404,
  });
  const fields = {
    ...reply.draft.fields,
    author: "Fontes test author",
    publisher: "Fontes test publisher",
    coverageStart: "Rom.8.28",
    coverageEnd: "Rom.8.29",
    licenseId: "CC0-1.0",
    rightsHolder: "Fontes test",
    copyright: "Original test fixture",
    attribution: "Fontes original test",
    licenseText:
      "Original synthetic fixture dedicated to the public domain for tests.",
    coverAlt: "Original blue test rectangle",
    coverLicenseId: "CC0-1.0",
    coverRightsHolder: "Fontes test",
    coverAttribution: "Original generated test rectangle",
    rightsAttested: true,
    conversionAcknowledged: true,
    permissions: {
      ...reply.draft.fields.permissions,
      "offline-display": "allow",
      "keyword-search": "allow",
    },
  };
  await mutate({ action: "fields", fields });
  const cover = await sharp({
    create: { width: 1200, height: 1800, channels: 3, background: "#235f79" },
  })
    .png()
    .toBuffer();
  reply = await call(`${path}/cover`, {
    method: "POST",
    bytes: cover,
    headers: uploadHeaders(),
  });
  assert.equal(reply.checks.length, 0);
  checks++;
  await mutate({ action: "approve" });
  assert.equal(reply.approved, true);
  assert.equal(reply.draft.package, null);
  await mutate({ action: "package" });
  const digest = reply.draft.package.sha256;
  const r = await fetch(`${base}/api/v1/${path}/package`, {
    headers: { Cookie: "fontes_dev_persona=reviewer" },
  });
  assert.equal(r.status, 200);
  const output = Buffer.from(await r.arrayBuffer());
  assert.equal(createHash("sha256").update(output).digest("hex"), digest);
  checks++;
  const packaged = await unzip(output);
  const content = packaged.get(`content/${doc.id}.xml`).toString();
  assert.match(content, /Romans 8:28/);
  assert.match(content, /<scripture start="Rom.8.28"/);
  assert.match(content, /<scripture start="Ps.23.1"/);
  assert.doesNotMatch(content, /<scripture start="John.3.16"/);
  assert.match(content, /kind="note"/);
  assert.match(content, /<image src="media\/image-/);
  const asset = reply.manifest.assets[0];
  assert.ok(packaged.get(`media/image-${asset.sha256}.png`));
  checks++;
  await mutate({ action: "package" });
  assert.equal(reply.draft.package.sha256, digest);
  checks++;
  // A reference decision after approval clears both approval and package.
  p = await page();
  block = p.blocks.find((b) => b.id === block.id);
  await mutate({
    action: "correct",
    documentId: doc.id,
    blockId: block.id,
    segmentTexts: block.segments.map((s) => s.text),
    references: block.references.map((r) => ({ ...r, decision: "rejected" })),
  });
  assert.equal(reply.approved, false);
  assert.equal(reply.draft.approval, null);
  assert.equal(reply.draft.package, null);
  checks++;
  await call(`${path}/package`, { expected: 409 });
  await mutate({ action: "approve" });
  // Metadata edits are outside the preparation service and still invalidate its approval digest.
  d = await call(root);
  await call(`${root}/resources/${id}`, {
    method: "PATCH",
    body: {
      revision: d.organization.revision,
      resource_revision: reply.resourceRevision,
      title: "Changed publisher review fixture",
      description:
        "Original synthetic content with updated metadata for approval invalidation.",
      kind: "book",
      version: "1.0.0",
      language: "en",
    },
  });
  reply = await call(path);
  assert.equal(reply.approved, false);
  checks++;
  await call(`${path}/package`, { expected: 409 });
  await mutate({ action: "package" }, 409);
  // Restoring the former metadata must not resurrect the earlier approval.
  d = await call(root);
  await call(`${root}/resources/${id}`, {
    method: "PATCH",
    body: {
      revision: d.organization.revision,
      resource_revision: reply.resourceRevision,
      title: "Original publisher review fixture",
      description:
        "Original synthetic content for testing publisher corrections and source packaging.",
      kind: "book",
      version: "1.0.0",
      language: "en",
    },
  });
  reply = await call(path);
  assert.equal(reply.approved, false);
  checks++;
  await call(`${path}/package`, { expected: 409 });
  await mutate({ action: "approve" });
  const before = reply.reviewRevision;
  reply = await call(path, { method: "POST", bytes, headers: uploadHeaders() });
  assert.equal(reply.approved, false);
  assert.equal(reply.draft.accepted, 0);
  assert.equal(reply.draft.pending, 2);
  assert.ok(reply.reviewRevision > before);
  checks++;
  await call(`${path}/package`, { expected: 409 });
  const rows =
    await sql`SELECT action,snapshot_sha256 FROM fontes_cloud.organization_publisher_reviews WHERE resource_id=${id}`;
  assert.ok(rows.some((r) => r.action === "approve"));
  assert.ok(rows.some((r) => r.action === "package"));
  checks++;
  console.log(
    `${checks} publisher review HTTP/Postgres/Rust checks passed: corrections, evidence/range checks, manual references, preserved links, revision conflicts, approval, deterministic package generation, metadata/reimport invalidation, authorization and history.`,
  );
} finally {
  if (org)
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.organization_publisher_reviews WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${org})`,
      sql`DELETE FROM fontes_cloud.organization_epub_history WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${org})`,
      sql`DELETE FROM fontes_cloud.organization_epub_preparations WHERE resource_id IN (SELECT id FROM fontes_cloud.organization_resources WHERE organization_id=${org})`,
      ...[
        "organization_events",
        "organization_invitations",
        "organization_policy_revisions",
        "organization_members",
        "organization_resources",
      ].map((table) =>
        sql.query(
          `DELETE FROM fontes_cloud.${table} WHERE organization_id=$1`,
          [org],
        ),
      ),
      sql`DELETE FROM fontes_cloud.organizations WHERE id=${org} AND is_demo=true`,
    ]);
  console.log(
    "Removed only this invocation's synthetic organization; local fixture artifacts remain ignored.",
  );
}
