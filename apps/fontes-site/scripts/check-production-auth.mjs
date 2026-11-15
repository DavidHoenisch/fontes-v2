import { spawn } from "node:child_process";
import assert from "node:assert/strict";
const origin = "http://127.0.0.1:3001";
for (const [mode, expected, code] of [
  ["neon", 401, "unauthenticated"],
  ["development", 503, "development_auth_forbidden"],
]) {
  const child = spawn(
    process.execPath,
    [
      "node_modules/next/dist/bin/next",
      "start",
      "--hostname",
      "127.0.0.1",
      "--port",
      "3001",
    ],
    {
      env: {
        ...process.env,
        NODE_ENV: "production",
        FONTES_AUTH_MODE: mode,
        FONTES_APP_ORIGIN: origin,
      },
      stdio: "ignore",
    },
  );
  try {
    let ready = false;
    for (let i = 0; i < 100; i++) {
      try {
        const r = await fetch(origin);
        if (r.ok) {
          ready = true;
          break;
        }
      } catch {}
      if (child.exitCode !== null)
        throw new Error(
          "Production server exited before readiness. Check that port 3001 is free.",
        );
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    assert.ok(ready, "Production server readiness");
    const probes = [
      ["/api/v1/source-reviews", "GET"],
      ["/api/v1/source-reviews", "POST"],
      ["/api/v1/source-reviews/00000000-0000-4000-8000-000000000001", "GET"],
      ["/api/v1/source-reviews/00000000-0000-4000-8000-000000000001", "PATCH"],
      ...[
        "document?document=fixture",
        "file?path=metadata.xml",
        "package",
        "handoff",
      ].map((path) => [
        "/api/v1/source-reviews/00000000-0000-4000-8000-000000000001/" + path,
        "GET",
      ]),
      ["/api/v1/workspace", "GET"],
      ["/api/v1/organizations", "GET"],
      ["/api/v1/organizations/00000000-0000-4000-8000-000000000001", "GET"],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation",
        "GET",
      ],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation/document?document=document-000000000000000000000000",
        "GET",
      ],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation/assets/" +
          "0".repeat(64),
        "GET",
      ],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation",
        "PATCH",
      ],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation/cover",
        "POST",
      ],
      [
        "/api/v1/organizations/00000000-0000-4000-8000-000000000001/resources/00000000-0000-4000-8000-000000000002/preparation/package",
        "GET",
      ],
      ["/api/v1/invitations/preview", "POST"],
      ["/api/v1/invitations/accept", "POST"],
    ];
    for (const [path, method] of probes) {
      const result = await fetch(origin + path, {
        method,
        headers: {
          Cookie: "fontes_dev_persona=reviewer",
          Origin: origin,
          "Content-Type": "application/json",
        },
        ...(["POST", "PATCH"].includes(method)
          ? { body: JSON.stringify({ token: "a".repeat(43) }) }
          : {}),
      });
      const body = await result.json();
      assert.equal(result.status, expected, path);
      assert.equal(body.error.code, code, path);
    }
    console.log(
      `Production ${mode} mode: ${expected} ${code}; developer cookie does not grant workspace, organization or invitation access.`,
    );
  } finally {
    if (child.exitCode === null) {
      child.kill("SIGTERM");
      await new Promise((resolve) => child.once("exit", resolve));
    }
  }
}
