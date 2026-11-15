import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { neon } from "@neondatabase/serverless";
import sharp from "sharp";
import { zip, epubEntries } from "../tests/intake-fixtures.ts";
const origin = "http://127.0.0.1:3000";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error("Use the dedicated development database/server.");
let checks = 0;
const created = [];
const digest = (b) => createHash("sha256").update(b).digest("hex");
async function call(
  path,
  {
    method = "GET",
    body,
    buffer,
    revision,
    persona = "publisher",
    expected = 200,
    binary = false,
  } = {},
) {
  const response = await fetch(origin + path, {
    method,
    headers: {
      Origin: origin,
      Cookie: `fontes_dev_persona=${persona}`,
      "Content-Type": buffer ? "application/octet-stream" : "application/json",
      ...(revision ? { "X-Fontes-Revision": String(revision) } : {}),
    },
    ...(buffer ? { body: buffer } : body ? { body: JSON.stringify(body) } : {}),
  });
  if (response.status !== expected) {
    console.log(await response.text());
    throw new Error(
      `${method} ${path}: expected ${expected}, received ${response.status}`,
    );
  }
  checks++;
  return binary
    ? Buffer.from(await response.arrayBuffer())
    : (await response.json()).data;
}
try {
  const metadata = {
    title: "Original fixture & study",
    description:
      "An original synthetic work for source preparation and approval tests.",
    kind: "book",
    language: "en",
    version: "1.0.0",
  };
  const resource = await call("/api/v1/resources", {
    method: "POST",
    body: metadata,
    expected: 201,
  });
  created.push(resource.id);
  const url = `/api/v1/resources/${resource.id}/intake`;
  let draft = await call(url);
  draft.state.fields = {
    ...draft.state.fields,
    author: "Fixture Author",
    publisher: "Fixture Publisher",
    sourceVersification: "KJV",
    coverageStart: "John.3.16",
    licenseId: "CC0-1.0",
    rightsHolder: "Fixture Author",
    copyright: "Original test fixture dedicated to the public domain.",
    attribution: "Original synthetic test fixture.",
    licenseText:
      "The author dedicates this original test fixture to the public domain under CC0 1.0.",
    coverAlt: "Plain original test cover",
    coverLicenseId: "CC0-1.0",
    coverRightsHolder: "Fixture Author",
    coverAttribution: "Original test cover.",
    permissions: {
      "offline-display": "allow",
      "keyword-search": "allow",
      export: "deny",
      print: "deny",
      embeddings: "deny",
      "ai-context": "deny",
    },
  };
  const save = async (current, fields = current.state.fields) =>
    call(url, {
      method: "PUT",
      body: {
        revision: current.revision,
        method: "guided",
        fields,
        decisions: current.state.proposals.map((p) => ({
          id: p.id,
          decision: p.decision,
        })),
      },
    });
  draft = await save(draft);
  await call(url, { persona: "reviewer", expected: 404 });
  await call(url + "/approve", {
    method: "POST",
    body: { revision: draft.revision },
    expected: 422,
  });
  const cover = await sharp({
    create: { width: 1200, height: 1800, channels: 3, background: "#ececec" },
  })
    .png()
    .toBuffer();
  draft = await call(url + "/cover", {
    method: "POST",
    buffer: cover,
    revision: draft.revision,
  });
  draft = await call(url + "/epub", {
    method: "POST",
    buffer: await zip(epubEntries()),
    revision: draft.revision,
  });
  assert.equal(draft.state.documents.length, 2);
  assert.equal(draft.state.proposals.length, 2);
  await call(url + "/package", { expected: 409 });
  draft.state.proposals.forEach((p) => (p.decision = "accepted"));
  draft = await save(draft, {
    ...draft.state.fields,
    rightsAttested: true,
    conversionAcknowledged: true,
  });
  assert.deepEqual(draft.checks, []);
  const beforeApproval = draft.revision;
  draft = await call(url + "/approve", {
    method: "POST",
    body: { revision: draft.revision },
  });
  assert.ok(draft.approval.digest);
  assert.equal(draft.ai.configured, false);
  await call(url + "/approve", {
    method: "POST",
    body: { revision: beforeApproval },
    expected: 409,
  });
  const first = await call(url + "/package", { binary: true });
  const second = await call(url + "/package", { binary: true });
  assert.equal(digest(first), digest(second));
  const prepared = await call("/api/v1/resources", {
    method: "POST",
    body: { ...metadata, title: "A prepared upload" },
    expected: 201,
  });
  created.push(prepared.id);
  const preparedUrl = `/api/v1/resources/${prepared.id}/intake`;
  let other = await call(preparedUrl);
  other = await call(preparedUrl, {
    method: "PUT",
    body: {
      revision: other.revision,
      method: "prepared",
      fields: other.state.fields,
    },
  });
  other = await call(preparedUrl + "/prepared", {
    method: "POST",
    buffer: first,
    revision: other.revision,
  });
  assert.equal(other.state.source.resourceId, resource.id);
  assert.equal(other.state.documents.length, 2);
  assert.ok(
    other.state.documents
      .flatMap((d) => d.blocks)
      .some((b) => b.sourceAssertions?.length),
  );
  other = await call(preparedUrl, {
    method: "PUT",
    body: {
      revision: other.revision,
      method: "prepared",
      fields: {
        ...other.state.fields,
        rightsAttested: true,
        conversionAcknowledged: true,
      },
    },
  });
  other = await call(preparedUrl + "/approve", {
    method: "POST",
    body: { revision: other.revision },
  });
  const preserved = await call(preparedUrl + "/package", { binary: true });
  assert.equal(digest(preserved), digest(first));
  const edited = await save(draft, {
    ...draft.state.fields,
    author: "Revised Fixture Author",
  });
  assert.equal(edited.approval, null);
  await call(url + "/package", { expected: 409 });
  const sql = neon(process.env.DATABASE_URL);
  const history =
    await sql`SELECT snapshot FROM fontes_cloud.authoring_history WHERE resource_id=${resource.id} ORDER BY revision`;
  assert.ok(history.some((r) => r.snapshot.authoring_approval?.digest));
  assert.ok(
    history.some((r) =>
      r.snapshot.authoring.proposals.some(
        (p) => p.decision === "accepted" && p.method === "literal-reference-v1",
      ),
    ),
  );
  console.log(
    `${checks} intake HTTP checks passed; EPUB extraction, canonical proposal review, strict Rust validation, reproducible generated ZIP, unchanged prepared ZIP, approval invalidation, isolation and history verified.`,
  );
} finally {
  const sql = neon(process.env.DATABASE_URL);
  for (const id of created)
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.authoring_history WHERE resource_id=${id}`,
      sql`DELETE FROM fontes_cloud.events WHERE resource_id=${id}`,
      sql`DELETE FROM fontes_cloud.resources WHERE id=${id} AND owner_id='dev-publisher' AND is_demo=true`,
    ]);
  console.log(
    "Removed only this test's synthetic database records; private uploaded fixtures remain in ignored local storage.",
  );
}
