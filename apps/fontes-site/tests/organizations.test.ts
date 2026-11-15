import { describe, it, expect } from "vitest";
import {
  invitationInput,
  inviteTokenInput,
  memberInput,
  organizationInput,
  organizationUpdate,
  organizationResourceUpdate,
  canManageOrganization,
  canPrepareOrganization,
} from "../src/lib/organization-contracts";
import { splitSql } from "../scripts/split-sql.mjs";
import { openapi } from "../src/lib/openapi";
import { readFileSync } from "node:fs";
describe("organization request boundaries", () => {
  it("does not accept a forged owner or namespace", () => {
    expect(
      organizationInput.safeParse({ name: "Team", owner_id: "someone" })
        .success,
    ).toBe(false);
    expect(
      organizationInput.safeParse({ name: "Team", namespace: "public" })
        .success,
    ).toBe(false);
  });
  it("never invites an owner directly", () => {
    expect(
      invitationInput.safeParse({
        email: "a@example.org",
        revision: 1,
        role: "owner",
      }).success,
    ).toBe(false);
    expect(
      invitationInput.parse({
        email: "Case@Example.org",
        revision: 1,
        role: "member",
      }).email,
    ).toBe("case@example.org");
  });
  it("requires bounded unpredictable token shape", () => {
    expect(inviteTokenInput.safeParse({ token: "abc" }).success).toBe(false);
    expect(inviteTokenInput.safeParse({ token: "a".repeat(44) }).success).toBe(
      false,
    );
    expect(
      inviteTokenInput.parse({ token: "a".repeat(43) }).token,
    ).toHaveLength(43);
  });
  it("rejects invented offline policy and unsigned lease durations", () => {
    expect(
      organizationUpdate.safeParse({
        name: "Team",
        revision: 1,
        offline_policy_preference: "immediate_recall",
      }).success,
    ).toBe(false);
    expect(
      organizationInput.safeParse({ name: "Team", lease_days: 30 }).success,
    ).toBe(false);
  });
  it("bounds stale, unknown role and resource status inputs", () => {
    expect(memberInput.safeParse({ revision: 0, role: "owner" }).success).toBe(
      false,
    );
    expect(
      memberInput.safeParse({ revision: 1, role: "platform_admin" }).success,
    ).toBe(false);
    expect(
      organizationResourceUpdate.safeParse({
        revision: 1,
        resource_revision: 1,
        title: "Draft",
        description: "Description of a resource",
        kind: "book",
        status: "published",
      }).success,
    ).toBe(false);
  });
  it("separates role capabilities from platform review", () => {
    expect(canManageOrganization("reviewer")).toBe(false);
    expect(canManageOrganization("publisher")).toBe(false);
    expect(canPrepareOrganization("publisher")).toBe(true);
    expect(canPrepareOrganization("member")).toBe(false);
  });
  it("exports organization API schemas without raw token in detail", () => {
    expect(openapi.paths["/api/v1/invitations/accept"]).toBeTruthy();
    expect(
      JSON.stringify(openapi.components.schemas.OrganizationDetail),
    ).not.toContain('"token"');
    expect(() => JSON.stringify(openapi)).not.toThrow();
  });
});
describe("transactional SQL migrations", () => {
  it("keeps dollar-quoted procedural statements intact", () => {
    const statements = splitSql(
      "-- skip;\nCREATE FUNCTION f() RETURNS void AS $fn$ BEGIN RAISE NOTICE 'x;y'; END; $fn$ LANGUAGE plpgsql; SELECT 'it''s; valid'; /* nested /* ; */ comment */ SELECT 2;",
    );
    expect(statements).toHaveLength(3);
    expect(statements[0]).toContain("END;");
    expect(statements[1]).toBe("SELECT 'it''s; valid'");
  });
  it("rejects truncated function bodies", () => {
    expect(() => splitSql("SELECT $$unterminated;")).toThrow();
  });
  it("preserves the organization command as a single migration statement", () => {
    const statements = splitSql(
      readFileSync(
        new URL("../db/004-organizations.sql", import.meta.url),
        "utf8",
      ),
    );
    expect(
      statements.filter((s: string) => s.startsWith("CREATE FUNCTION")),
    ).toHaveLength(1);
    expect(statements.at(-1)).toContain("RETURN jsonb_build_object('id',p_org");
  });
});
