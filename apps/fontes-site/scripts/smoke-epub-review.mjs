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
    `${method} ${path}: ${result.error?.code ?? "unexpected status"}`,
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
      '<package><metadata><title>Original evaluation fixture</title><creator>Fontes test</creator><rights>Original synthetic text for testing.</rights></metadata><manifest><item id="chapter" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="chapter"/></spine></package>',
    "EPUB/chapter.xhtml":
      '<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>Test chapter</title></head><body><span id="page"/><h1>Original fixture</h1>' +
      Array.from(
        { length: 90 },
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
try {
  let d = await call("organizations", {
    method: "POST",
    body: { name: "Synthetic EPUB review checks" },
    expected: 201,
  });
  org = d.organization.id;
  const root = `organizations/${org}`;
  d = await call(`${root}/resources`, {
    method: "POST",
    body: {
      title: "EPUB review fixture",
      description: "A disposable original source evaluation fixture.",
      kind: "book",
      revision: d.organization.revision,
    },
    expected: 201,
  });
  const resource = d.resources[0],
    path = `${root}/resources/${resource.id}/preparation`;
  let reply = await call(path);
  assert.equal(reply.manifest, null);
  const bytes = await fixture();
  reply = await call(path, {
    method: "POST",
    bytes,
    headers: {
      "X-Fontes-Revision": String(reply.resourceRevision),
      "X-Fontes-Organization-Revision": String(reply.organizationRevision),
    },
  });
  assert.equal(reply.manifest.fidelity.normalizedTextMatches, true);
  assert.equal(reply.manifest.exportable, false);
  assert.equal(reply.manifest.statistics.footnotes, 1);
  await call(path, { persona: "publisher", expected: 404 });
  const doc = reply.manifest.documents[0];
  let page = await call(`${path}/document?document=${doc.id}`);
  assert.equal(page.blocks.length, 40);
  assert.equal(page.nextOffset, 40);
  assert.equal(page.total, 93);
  const next = await call(`${path}/document?document=${doc.id}&offset=40`);
  assert.equal(next.offset, 40);
  assert.equal(next.blocks[0].text, "Original test paragraph 39. John 3:16.");
  const last = await call(`${path}/document?document=${doc.id}&offset=80`);
  const note = last.blocks.find((b) => b.kind === "note");
  assert.ok(note);
  const target = await call(
    `${path}/document?document=${doc.id}&block=${note.id}`,
  );
  assert.equal(target.focusBlockId, note.id);
  await call(`${path}/document?document=${doc.id}&offset=25001`, {
    expected: 400,
  });
  await call(`${path}/document?document=document-${"0".repeat(24)}`, {
    expected: 404,
  });
  await call(`${path}/document?document=${doc.id}`, {
    persona: "publisher",
    expected: 404,
  });
  await call(`${path}/assets/${"0".repeat(64)}`, { expected: 404 });
  await call(path, {
    method: "POST",
    bytes,
    headers: {
      "X-Fontes-Revision": "1",
      "X-Fontes-Organization-Revision": "1",
    },
    expected: 409,
  });
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
  await call(path, { persona: "publisher", expected: 404 });
  d = await call(root);
  await call(`${root}/members/dev-publisher`, {
    method: "PATCH",
    body: { revision: d.organization.revision, role: "publisher" },
  });
  await call(path, { persona: "publisher" });
  await call(`${path}/document?document=${doc.id}`, { persona: "publisher" });
  d = await call(root);
  await call(`${root}/members/dev-publisher`, {
    method: "DELETE",
    body: { revision: d.organization.revision },
  });
  await call(path, { persona: "publisher", expected: 404 });
  await call(`${path}/document?document=${doc.id}`, {
    persona: "publisher",
    expected: 404,
  });
  reply = await call(path);
  const second = await call(path, {
    method: "POST",
    bytes,
    headers: {
      "X-Fontes-Revision": String(reply.resourceRevision),
      "X-Fontes-Organization-Revision": String(reply.organizationRevision),
    },
  });
  assert.equal(second.preparationRevision, 2);
  assert.equal(second.manifest.sourceSha256, reply.manifest.sourceSha256);
  const history =
    await sql`SELECT count(*)::int AS n FROM fontes_cloud.organization_epub_history WHERE resource_id=${resource.id}`;
  assert.equal(history[0].n, 2);
  checks++;
  const row =
    await sql`SELECT summary::text AS report FROM fontes_cloud.organization_epub_preparations WHERE resource_id=${resource.id}`;
  assert.equal(row[0].report.includes("Original test paragraph"), false);
  checks++;
  console.log(
    `${checks} organization EPUB HTTP/database checks passed: bounded pagination, links/footnotes, role isolation, removal, stale import rejection, revision history and local-only prose.`,
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
