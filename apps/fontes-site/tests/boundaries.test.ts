import { describe, expect, it } from "vitest";
import { resolveAuthMode } from "../src/lib/auth-policy";
import {
  assertTransition,
  createResourceSchema,
  transitionSchema,
} from "../src/lib/contracts";

describe("local auth bypass boundary", () => {
  const dev = { NODE_ENV: "development", FONTES_AUTH_MODE: "development" };
  it("defaults to managed auth when bypass was not selected", () =>
    expect(resolveAuthMode({ NODE_ENV: "development" }, "localhost")).toBe(
      "neon",
    ));
  it.each(["localhost", "127.0.0.1", "[::1]"])(
    "permits explicit local bypass on %s",
    (host) => expect(resolveAuthMode(dev, host)).toBe("development"),
  );
  it.each(["production", undefined, "staging"])(
    "rejects bypass outside a local runtime: %s",
    (node) =>
      expect(() =>
        resolveAuthMode({ ...dev, NODE_ENV: node }, "localhost"),
      ).toThrow(),
  );
  it.each([
    "VERCEL",
    "CF_PAGES",
    "AWS_LAMBDA_FUNCTION_NAME",
    "RENDER",
    "FLY_APP_NAME",
    "RAILWAY_ENVIRONMENT",
  ])("rejects deployed development runtimes: %s", (name) =>
    expect(() =>
      resolveAuthMode({ ...dev, [name]: "1" }, "localhost"),
    ).toThrow(),
  );
  it("rejects non-loopback bypass requests", () =>
    expect(() => resolveAuthMode(dev, "example.com")).toThrow());
  it("rejects misspelled modes rather than silently bypassing", () =>
    expect(() =>
      resolveAuthMode({ ...dev, FONTES_AUTH_MODE: "dev" }, "localhost"),
    ).toThrow());
});

describe("metadata review state machine", () => {
  const owner = { id: "author", role: "publisher" as const };
  const reviewer = { id: "editor", role: "reviewer" as const };
  const resource = {
    owner_id: "author",
    status: "submitted" as const,
    revision: 2,
  };
  it("permits independent metadata review", () =>
    expect(
      assertTransition(reviewer, resource, {
        action: "approve_metadata",
        revision: 2,
        note: "",
      }),
    ).toBe("metadata_approved"));
  it("rejects self-review even for a maintainer", () =>
    expect(() =>
      assertTransition({ ...reviewer, id: "author" }, resource, {
        action: "approve_metadata",
        revision: 2,
        note: "",
      }),
    ).toThrow("independent"));
  it("rejects publisher approval", () =>
    expect(() =>
      assertTransition(owner, resource, {
        action: "approve_metadata",
        revision: 2,
        note: "",
      }),
    ).toThrow());
  it("rejects a stale approval", () =>
    expect(() =>
      assertTransition(reviewer, resource, {
        action: "approve_metadata",
        revision: 1,
        note: "",
      }),
    ).toThrow("changed"));
  it("rejects approval of an editable draft", () =>
    expect(() =>
      assertTransition(
        reviewer,
        { ...resource, status: "draft" },
        { action: "approve_metadata", revision: 2, note: "" },
      ),
    ).toThrow("not awaiting"));
  it("permits an owner to resubmit requested changes", () =>
    expect(
      assertTransition(
        owner,
        { ...resource, status: "changes_requested" },
        { action: "submit", revision: 2, note: "" },
      ),
    ).toBe("submitted"));
  it("prevents someone else from submitting an owned draft", () =>
    expect(() =>
      assertTransition(
        reviewer,
        { ...resource, status: "draft" },
        { action: "submit", revision: 2, note: "" },
      ),
    ).toThrow());
  it("requires actionable feedback for changes", () =>
    expect(
      transitionSchema.safeParse({
        action: "request_changes",
        revision: 2,
        note: "  ",
      }).success,
    ).toBe(false));
  it("does not accept source approval or publishing as a metadata action", () =>
    expect(
      transitionSchema.safeParse({ action: "publish", revision: 2 }).success,
    ).toBe(false));
  it("does not accept a client-supplied role/owner", () =>
    expect(
      createResourceSchema.safeParse({
        title: "A draft",
        description: "A useful resource description.",
        kind: "book",
        owner_id: "someone-else",
        role: "reviewer",
      }).success,
    ).toBe(false));
});
