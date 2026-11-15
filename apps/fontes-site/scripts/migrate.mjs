import { splitSql } from "./split-sql.mjs";
import { neon } from "@neondatabase/serverless";
import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
if (!process.env.DATABASE_URL)
  throw new Error("Set DATABASE_URL in .env.local.");
const sql = neon(process.env.DATABASE_URL);
await sql`CREATE SCHEMA IF NOT EXISTS fontes_cloud`;
await sql`CREATE TABLE IF NOT EXISTS fontes_cloud.migrations (name text PRIMARY KEY, sha256 text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now())`;
for (const name of (await readdir(new URL("../db/", import.meta.url)))
  .filter((name) => /^\d+.*\.sql$/.test(name))
  .sort()) {
  const content = await readFile(
    new URL(`../db/${name}`, import.meta.url),
    "utf8",
  );
  const digest = createHash("sha256").update(content).digest("hex");
  const existing =
    await sql`SELECT sha256 FROM fontes_cloud.migrations WHERE name = ${name}`;
  if (existing[0]) {
    if (existing[0].sha256 !== digest)
      throw new Error(
        "An applied migration was modified. Create a new migration.",
      );
    console.log("Workspace schema is current.");
  } else {
    await sql.transaction([
      ...splitSql(content).map((s) => sql.query(s)),
      sql`INSERT INTO fontes_cloud.migrations (name, sha256) VALUES (${name}, ${digest})`,
    ]);
    console.log(
      `Applied ${name} to fontes_cloud; Neon auth schema is unchanged.`,
    );
  }
}
