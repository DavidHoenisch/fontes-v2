import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { neon } from "@neondatabase/serverless";
if (process.env.FONTES_AUTH_MODE !== "development")
  throw new Error(
    "Organization smoke checks require the dedicated development configuration.",
  );
const sql = neon(process.env.DATABASE_URL),
  base = "http://127.0.0.1:3000",
  owned = [];
let checks = 0;
async function call(
  path,
  {
    persona = "publisher",
    method = "GET",
    body,
    expected = 200,
    origin = base,
  } = {},
) {
  const r = await fetch(`${base}/api/v1/${path}`, {
    method,
    headers: {
      Cookie: `fontes_dev_persona=${persona}`,
      Origin: origin,
      "Content-Type": "application/json",
    },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const result = await r.json();
  // Do not echo response bodies: invitation tokens are one-time secrets.
  assert.equal(
    r.status,
    expected,
    `${method} ${path}: ${result.error?.code ?? "unexpected status"}`,
  );
  checks++;
  return result.data;
}
let id;
try {
  await call("organizations", {
    method: "POST",
    origin: "https://untrusted.invalid",
    body: { name: "Origin test" },
    expected: 403,
  });
  await call("organizations", {
    method: "POST",
    body: { name: "Forged org", owner_id: "dev-reviewer" },
    expected: 400,
  });
  let d = await call("organizations", {
    method: "POST",
    body: {
      name: "Synthetic organization API checks",
      description:
        "Disposable workflow fixture for organization authorization.",
    },
    expected: 201,
  });
  id = d.organization.id;
  owned.push(id);
  const path = `organizations/${id}`;
  assert.equal(d.organization.role, "owner");
  assert.equal(d.members.length, 1);
  assert.equal(d.capabilities.private_publication, false);
  const get = (persona = "publisher") => call(path, { persona });
  await get("publisher");
  await call(path, { persona: "reviewer", expected: 404 });
  await call(`${path}/members/dev-publisher`, {
    method: "PATCH",
    body: { revision: d.organization.revision, role: "member" },
    expected: 409,
  });
  await call(`${path}/leave`, {
    method: "POST",
    body: { revision: d.organization.revision },
    expected: 409,
  });
  await call(path, {
    method: "PATCH",
    body: { name: "Stale", revision: 999 },
    expected: 409,
  });
  d = await call(path, {
    method: "PATCH",
    body: {
      name: d.organization.name,
      description: d.organization.description,
      revision: d.organization.revision,
      offline_policy_preference: "renewable_lease",
    },
  });
  assert.equal(d.organization.policy_revision, 2);
  const policies =
    await sql`SELECT revision,preference FROM fontes_cloud.organization_policy_revisions WHERE organization_id=${id}`;
  assert.equal(policies.length, 2);
  checks++;
  let invitation = await call(`${path}/invitations`, {
    method: "POST",
    body: {
      email: "Reviewer@Example.Invalid",
      role: "member",
      revision: d.organization.revision,
    },
    expected: 201,
  });
  const stored =
    await sql`SELECT token_hash FROM fontes_cloud.organization_invitations WHERE id=${invitation.invitation_id}`;
  assert.equal(
    stored[0].token_hash,
    createHash("sha256").update(invitation.token).digest("hex"),
  );
  checks++;
  await call("invitations/preview", {
    method: "POST",
    body: { token: invitation.token },
    expected: 404,
  });
  const preview = await call("invitations/preview", {
    persona: "reviewer",
    method: "POST",
    body: { token: invitation.token },
  });
  assert.equal(preview.role, "member");
  d = await get();
  assert.equal(JSON.stringify(d).includes(invitation.token), false);
  await call(`${path}/invitations`, {
    method: "POST",
    body: {
      email: "reviewer@example.invalid",
      role: "publisher",
      revision: d.organization.revision,
    },
    expected: 409,
  });
  const old = invitation.token;
  invitation = await call(`${path}/invitations/${invitation.invitation_id}`, {
    method: "POST",
    body: { revision: d.organization.revision },
  });
  await call("invitations/accept", {
    persona: "reviewer",
    method: "POST",
    body: { token: old },
    expected: 404,
  });
  const accepted = await Promise.all(
    [0, 1].map(() =>
      fetch(`${base}/api/v1/invitations/accept`, {
        method: "POST",
        headers: {
          Origin: base,
          Cookie: "fontes_dev_persona=reviewer",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ token: invitation.token }),
      }),
    ),
  );
  assert.deepEqual(accepted.map((r) => r.status).sort(), [200, 409]);
  checks++;
  const count =
    await sql`SELECT count(*)::int AS n FROM fontes_cloud.organization_members WHERE organization_id=${id} AND account_id='dev-reviewer'`;
  assert.equal(count[0].n, 1);
  checks++;
  await call("invitations/preview", {
    persona: "reviewer",
    method: "POST",
    body: { token: invitation.token },
    expected: 409,
  });
  d = await get("reviewer");
  assert.equal(d.organization.role, "member");
  assert.deepEqual(d.invitations, []);
  assert.ok(d.members.every((m) => m.email === null));
  await call(`${path}/invitations`, {
    persona: "reviewer",
    method: "POST",
    body: {
      email: "other@example.invalid",
      role: "member",
      revision: d.organization.revision,
    },
    expected: 403,
  });
  const draft = {
    title: "Private draft for workflow checks",
    description:
      "Synthetic private metadata that must never enter public review.",
    kind: "study-guide",
    language: "en",
    version: "1.0.0",
  };
  await call(`${path}/resources`, {
    persona: "reviewer",
    method: "POST",
    body: { ...draft, revision: d.organization.revision },
    expected: 403,
  });
  d = await get();
  d = await call(`${path}/resources`, {
    method: "POST",
    body: { ...draft, revision: d.organization.revision },
    expected: 201,
  });
  const resource = d.resources[0];
  const memberView = await get("reviewer");
  assert.deepEqual(memberView.resources, []);
  assert.equal(JSON.stringify(memberView).includes(draft.title), false);
  const personal = await call("workspace", { persona: "reviewer" });
  assert.equal(
    personal.resources.some((r) => r.id === resource.id),
    false,
  );
  await call(`resources/${resource.id}`, {
    persona: "reviewer",
    expected: 404,
  });
  await call(`resources/${resource.id}/intake`, {
    persona: "reviewer",
    expected: 404,
  });
  await call(`${path}/resources/${resource.id}`, {
    method: "PATCH",
    body: {
      ...draft,
      revision: d.organization.revision,
      resource_revision: 99,
    },
    expected: 409,
  });
  d = await call(`${path}/resources/${resource.id}`, {
    method: "PATCH",
    body: {
      ...draft,
      title: "Refined private organization draft",
      revision: d.organization.revision,
      resource_revision: resource.revision,
    },
  });
  await call(`${path}/members/dev-reviewer`, {
    method: "PATCH",
    body: { role: "publisher", revision: d.organization.revision },
  });
  d = await get("reviewer");
  assert.equal(d.resources.length, 1);
  d = await call(`${path}/resources`, {
    persona: "reviewer",
    method: "POST",
    body: { ...draft, revision: d.organization.revision },
    expected: 201,
  });
  await call(`${path}/members/dev-publisher`, {
    persona: "reviewer",
    method: "PATCH",
    body: { role: "member", revision: d.organization.revision },
    expected: 403,
  });
  await call(`${path}/members/dev-reviewer`, {
    method: "PATCH",
    body: { role: "admin", revision: d.organization.revision },
  });
  d = await get("reviewer");
  await call(`${path}/members/dev-reviewer`, {
    persona: "reviewer",
    method: "PATCH",
    body: { role: "owner", revision: d.organization.revision },
    expected: 403,
  });
  await call(`${path}/members/dev-publisher`, {
    persona: "reviewer",
    method: "DELETE",
    body: { revision: d.organization.revision },
    expected: 403,
  });
  await call(`${path}/invitations`, {
    persona: "reviewer",
    method: "POST",
    body: {
      email: "other@example.invalid",
      role: "admin",
      revision: d.organization.revision,
    },
    expected: 403,
  });
  await call(`${path}/members/dev-reviewer`, {
    method: "PATCH",
    body: { role: "owner", revision: d.organization.revision },
  });
  d = await get();
  const outcomes = await Promise.all(
    [
      { persona: "publisher", target: "dev-publisher" },
      { persona: "reviewer", target: "dev-reviewer" },
    ].map(({ persona, target }) =>
      fetch(`${base}/api/v1/${path}/members/${target}`, {
        method: "PATCH",
        headers: {
          Origin: base,
          Cookie: `fontes_dev_persona=${persona}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          role: "member",
          revision: d.organization.revision,
        }),
      }),
    ),
  );
  assert.deepEqual(outcomes.map((r) => r.status).sort(), [200, 409]);
  checks++;
  const owners =
    await sql`SELECT account_id FROM fontes_cloud.organization_members WHERE organization_id=${id} AND role='owner'`;
  assert.equal(owners.length, 1);
  checks++;
  const ownerPersona =
      owners[0].account_id === "dev-publisher" ? "publisher" : "reviewer",
    otherPersona = ownerPersona === "publisher" ? "reviewer" : "publisher",
    otherId = otherPersona === "publisher" ? "dev-publisher" : "dev-reviewer";
  d = await get(ownerPersona);
  await call(`${path}/leave`, {
    persona: ownerPersona,
    method: "POST",
    body: { revision: d.organization.revision },
    expected: 409,
  });
  // Expiry and revoke/renew checks use the non-owner after removal.
  await call(`${path}/members/${otherId}`, {
    persona: ownerPersona,
    method: "DELETE",
    body: { revision: d.organization.revision },
  });
  await call(path, { persona: otherPersona, expected: 404 });
  const unjoined = await call("organizations", { persona: otherPersona });
  assert.equal(
    unjoined.items.some((o) => o.id === id),
    false,
  );
  d = await get(ownerPersona);
  invitation = await call(`${path}/invitations`, {
    persona: ownerPersona,
    method: "POST",
    body: {
      email:
        otherPersona === "publisher"
          ? "publisher@example.invalid"
          : "reviewer@example.invalid",
      role: "publisher",
      revision: d.organization.revision,
    },
    expected: 201,
  });
  await sql`UPDATE fontes_cloud.organization_invitations SET expires_at=now()-interval '1 minute' WHERE id=${invitation.invitation_id} AND organization_id=${id}`;
  await call("invitations/accept", {
    persona: otherPersona,
    method: "POST",
    body: { token: invitation.token },
    expected: 409,
  });
  d = await get(ownerPersona);
  assert.equal(
    d.invitations.find((i) => i.id === invitation.invitation_id).status,
    "expired",
  );
  invitation = await call(`${path}/invitations/${invitation.invitation_id}`, {
    persona: ownerPersona,
    method: "POST",
    body: { revision: d.organization.revision },
  });
  d = await get(ownerPersona);
  await call(`${path}/invitations/${invitation.invitation_id}`, {
    persona: ownerPersona,
    method: "DELETE",
    body: { revision: d.organization.revision },
  });
  await call("invitations/accept", {
    persona: otherPersona,
    method: "POST",
    body: { token: invitation.token },
    expected: 409,
  });
  // A second tenant has no platform-review override or cross-tenant draft access.
  const second = await call("organizations", {
    persona: otherPersona,
    method: "POST",
    body: { name: "Other synthetic organization" },
    expected: 201,
  });
  owned.push(second.organization.id);
  await call(`${path}/resources`, {
    persona: otherPersona,
    method: "POST",
    body: { ...draft, revision: d.organization.revision },
    expected: 404,
  });
  await call(
    `organizations/${second.organization.id}/resources/${resource.id}`,
    {
      persona: otherPersona,
      method: "PATCH",
      body: {
        ...draft,
        revision: second.organization.revision,
        resource_revision: resource.revision,
      },
      expected: 409,
    },
  );
  // The org row lock also serializes persistent invitation-rate checks.
  await sql`INSERT INTO fontes_cloud.organization_events(organization_id,actor_id,action)
    SELECT ${id}::uuid,${owners[0].account_id},'invited' FROM generate_series(1,20)`;
  d = await get(ownerPersona);
  await call(`${path}/invitations`, {
    persona: ownerPersona,
    method: "POST",
    expected: 429,
    body: {
      email: "rate-check@example.invalid",
      role: "member",
      revision: d.organization.revision,
    },
  });
  const spec = await fetch(`${base}/api/v1/openapi`).then((r) => r.json());
  assert.ok(spec.paths["/api/v1/organizations/{id}/members/{accountId}"]);
  checks++;
  console.log(
    `${checks} organization HTTP/Postgres checks passed: tenant/role isolation, policy history, token hashing/rotation/expiry/replay, concurrent acceptance/last-owner protection, scoped draft CRUD and removal.`,
  );
} finally {
  for (const org of owned) {
    await sql.transaction([
      sql`DELETE FROM fontes_cloud.organization_events WHERE organization_id=${org}`,
      sql`DELETE FROM fontes_cloud.organization_resources WHERE organization_id=${org}`,
      sql`DELETE FROM fontes_cloud.organization_invitations WHERE organization_id=${org}`,
      sql`DELETE FROM fontes_cloud.organization_policy_revisions WHERE organization_id=${org}`,
      sql`DELETE FROM fontes_cloud.organization_members WHERE organization_id=${org}`,
      sql`DELETE FROM fontes_cloud.organizations WHERE id=${org} AND is_demo=true`,
    ]);
  }
  console.log(
    "Removed only this invocation's synthetic organizations; existing workspaces were preserved.",
  );
}
