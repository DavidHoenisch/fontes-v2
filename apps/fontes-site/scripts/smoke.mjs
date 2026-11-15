import assert from "node:assert/strict";
import { neon } from "@neondatabase/serverless";
const base = "http://127.0.0.1:3000";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error(
    "Smoke tests require the dedicated development configuration.",
  );
let created;
let checks = 0;
async function call(
  path,
  {
    persona = "publisher",
    method = "GET",
    body,
    expected = 200,
    origin = base,
    raw,
  } = {},
) {
  const r = await fetch(base + path, {
    method,
    headers: {
      "Content-Type": "application/json",
      Cookie: `fontes_dev_persona=${persona}`,
      ...(origin ? { Origin: origin } : {}),
    },
    ...(body !== undefined
      ? { body: JSON.stringify(body) }
      : raw
        ? { body: raw }
        : {}),
  });
  const data = await r.json();
  assert.equal(
    r.status,
    expected,
    `${method} ${path}: ${JSON.stringify(data)}`,
  );
  checks++;
  return data.data;
}
try {
  const initial = await call("/api/v1/workspace");
  assert.equal(initial.actor.mode, "development");
  const input = {
    title: "HTTP smoke test draft",
    description: "Synthetic metadata created by the repeatable API smoke test.",
    kind: "study-guide",
    language: "en",
    version: "1.0.0",
  };
  await call("/api/v1/resources", {
    method: "POST",
    body: input,
    origin: "https://example.invalid",
    expected: 403,
  });
  await call("/api/v1/resources", {
    method: "POST",
    body: { ...input, owner_id: "dev-reviewer" },
    expected: 400,
  });
  await call("/api/v1/resources", {
    method: "POST",
    raw: "{invalid",
    expected: 400,
  });
  await call("/api/v1/resources", {
    method: "POST",
    raw: "x".repeat(16_385),
    expected: 413,
  });
  created = await call("/api/v1/resources", {
    method: "POST",
    body: input,
    expected: 201,
  });
  const path = `/api/v1/resources/${encodeURIComponent(created.id)}`;
  await call(path, { persona: "reviewer", expected: 404 });
  await call(path, {
    method: "PATCH",
    body: { ...input, revision: 99 },
    expected: 409,
  });
  let resource = await call(path, {
    method: "PATCH",
    body: {
      ...input,
      title: "Refined HTTP smoke test draft",
      revision: created.revision,
    },
  });
  resource = await call(path + "/transitions", {
    method: "POST",
    body: { action: "submit", revision: resource.revision },
  });
  await call(path + "/transitions", {
    method: "POST",
    body: { action: "approve_metadata", revision: resource.revision },
    expected: 403,
  });
  await call(path + "/transitions", {
    persona: "reviewer",
    method: "POST",
    body: { action: "approve_metadata", revision: 1 },
    expected: 409,
  });
  resource = await call(path + "/transitions", {
    persona: "reviewer",
    method: "POST",
    body: {
      action: "request_changes",
      revision: resource.revision,
      note: "Please make the coverage explicit.",
    },
  });
  assert.equal(resource.status, "changes_requested");
  resource = await call(path, {
    method: "PATCH",
    body: { ...input, revision: resource.revision },
  });
  resource = await call(path + "/transitions", {
    method: "POST",
    body: { action: "submit", revision: resource.revision },
  });
  const competing = await Promise.all(
    ["First editorial decision", "Concurrent editorial decision"].map((note) =>
      fetch(base + path + "/transitions", {
        method: "POST",
        headers: {
          Origin: base,
          Cookie: "fontes_dev_persona=reviewer",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          action: "approve_metadata",
          revision: resource.revision,
          note,
        }),
      }),
    ),
  );
  assert.deepEqual(competing.map((r) => r.status).sort(), [200, 409]);
  checks++;
  const persisted = await call(path);
  assert.equal(persisted.status, "metadata_approved");
  await call(path, {
    method: "PATCH",
    body: { ...input, revision: persisted.revision },
    expected: 409,
  });
  await call(path + "/transitions", {
    persona: "reviewer",
    method: "POST",
    body: { action: "publish", revision: persisted.revision },
    expected: 400,
  });
  const final = await call("/api/v1/workspace");
  const events = final.activity.filter((e) => e.resource_id === created.id);
  assert.equal(events.filter((e) => e.action === "approve_metadata").length, 1);
  assert.equal(events.length, 7);
  const auth = await fetch(base + "/api/auth/get-session");
  assert.equal(auth.status, 200);
  assert.equal(await auth.json(), null);
  checks++;
  const spec = await fetch(base + "/api/v1/openapi").then((r) => r.json());
  assert.equal(spec.openapi, "3.1.0");
  assert.ok(spec.components.schemas.Workspace);
  checks++;
  console.log(
    `${checks} HTTP/database checks passed: scope, validation, Origin, revisions, concurrent review, persistence and Neon session proxy.`,
  );
} finally {
  if (created) {
    const sql = neon(process.env.DATABASE_URL);
    // Only the exact synthetic record created by this invocation is removed.
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.events WHERE resource_id = ${created.id}`,
      sql`DELETE FROM fontes_cloud.resources WHERE id = ${created.id} AND is_demo = true AND owner_id = 'dev-publisher'`,
    ]);
    console.log(
      "Removed this invocation’s smoke-test record; all other records were preserved.",
    );
  }
}
