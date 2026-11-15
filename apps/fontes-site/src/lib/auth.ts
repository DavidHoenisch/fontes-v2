import "server-only";
import { createHash } from "node:crypto";
import { cookies, headers } from "next/headers";
import { resolveAuthMode } from "./auth-policy";
import { DomainError, type Actor } from "./contracts";
import { managedAuth } from "./neon-auth";
import { database } from "./db";

export async function currentActor(request?: Request): Promise<Actor> {
  const host = request
    ? new URL(request.url).hostname
    : (await headers()).get("host")?.split(":")[0];
  const mode = resolveAuthMode(process.env, host);
  if (mode === "development") {
    const persona =
      (await cookies()).get("fontes_dev_persona")?.value === "reviewer"
        ? "reviewer"
        : "publisher";
    return persona === "reviewer"
      ? {
          id: "dev-reviewer",
          name: "Morgan Ellis",
          email: "reviewer@example.invalid",
          role: "reviewer",
          namespace: "dev.fontes.review",
          mode,
        }
      : {
          id: "dev-publisher",
          name: "Alex Morgan",
          email: "publisher@example.invalid",
          role: "publisher",
          namespace: "dev.fontes.alex",
          mode,
        };
  }
  const { data: session } = await managedAuth().getSession();
  if (!session?.user)
    throw new DomainError(
      401,
      "unauthenticated",
      "Sign in to open your publishing workspace.",
    );
  if (!session.user.emailVerified)
    throw new DomainError(
      403,
      "email_unverified",
      "Verify your email before using the publishing workspace.",
    );
  const user = session.user;
  const id = `neon:${user.id}`;
  const sql = database();
  // Stable owner IDs do not depend on a rename or a provider's organization UI.
  const namespace = `user.fontes.u${createHash("sha256").update(id).digest("hex").slice(0, 20)}`;
  const rows =
    await sql`INSERT INTO fontes_cloud.accounts (id, name, email, namespace, role, is_demo)
    VALUES (${id}, ${user.name}, ${user.email}, ${namespace}, 'publisher', false)
    ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, email = EXCLUDED.email
    RETURNING id, name, email, namespace, role`;
  return { ...rows[0], mode } as Actor;
}
