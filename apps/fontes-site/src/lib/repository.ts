import "server-only";
import { randomUUID } from "node:crypto";
import { database } from "./db";
import { preparationDigest } from "./intake-digest";
import {
  assertTransition,
  createResourceSchema,
  updateResourceSchema,
  transitionSchema,
  resourceSchema,
  workspaceSchema,
  DomainError,
  type Actor,
  type Resource,
  type Workspace,
} from "./contracts";

function wireResource(row: Record<string, unknown>) {
  return resourceSchema.parse({
    ...row,
    created_at: (row.created_at as Date).toISOString(),
    updated_at: (row.updated_at as Date).toISOString(),
    has_authoring: !!row.authoring,
  });
}

export async function workspace(actor: Actor): Promise<Workspace> {
  const sql = database();
  const reviewer = actor.role === "reviewer";
  const [resources, activity] = await Promise.all([
    sql`SELECT r.*, a.name AS owner_name FROM fontes_cloud.resources r JOIN fontes_cloud.accounts a ON a.id = r.owner_id
      WHERE r.owner_id = ${actor.id} OR (${reviewer} AND r.status <> 'draft') ORDER BY r.updated_at DESC LIMIT 100`,
    sql`SELECT e.*, r.title, a.name AS actor_name FROM fontes_cloud.events e
      JOIN fontes_cloud.resources r ON r.id = e.resource_id JOIN fontes_cloud.accounts a ON a.id = e.actor_id
      WHERE r.owner_id = ${actor.id} OR (${reviewer} AND r.status <> 'draft') ORDER BY e.created_at DESC LIMIT 30`,
  ]);
  return workspaceSchema.parse({
    actor,
    resources: resources.map(wireResource),
    activity: activity.map((e) => ({
      ...e,
      created_at: (e.created_at as Date).toISOString(),
    })),
    database: "connected",
    capabilities: {
      organizations: true,
      delivery: false,
      uploads: process.env.NODE_ENV === "development",
    },
  });
}
export async function createResource(actor: Actor, raw: unknown) {
  const input = createResourceSchema.parse(raw);
  const sql = database();
  const id = `${actor.namespace}.r${randomUUID().replaceAll("-", "")}`;
  const rows = await sql`WITH resource AS (
    INSERT INTO fontes_cloud.resources (id, owner_id, title, description, kind, language, version, is_demo)
    VALUES (${id}, ${actor.id}, ${input.title}, ${input.description}, ${input.kind}, ${input.language}, ${input.version}, ${actor.mode === "development"}) RETURNING *
  ), event AS (
    INSERT INTO fontes_cloud.events (resource_id, actor_id, action, note)
    SELECT id, ${actor.id}, 'created', 'Created a resource draft.' FROM resource
  ) SELECT resource.*, a.name AS owner_name FROM resource JOIN fontes_cloud.accounts a ON a.id = resource.owner_id`;
  return wireResource(rows[0]);
}
export async function getResource(actor: Actor, id: string) {
  const sql = database();
  const rows =
    await sql`SELECT r.*, a.name AS owner_name FROM fontes_cloud.resources r
    JOIN fontes_cloud.accounts a ON a.id = r.owner_id
    WHERE r.id = ${id} AND (r.owner_id = ${actor.id} OR (${actor.role === "reviewer"} AND r.status <> 'draft'))`;
  if (!rows[0])
    throw new DomainError(
      404,
      "not_found",
      "The resource was not found in this workspace.",
    );
  return wireResource(rows[0]);
}
export async function transitionResource(
  actor: Actor,
  id: string,
  raw: unknown,
) {
  const input = transitionSchema.parse(raw);
  const sql = database();
  const rows = await sql`SELECT * FROM fontes_cloud.resources WHERE id = ${id}
    AND (owner_id = ${actor.id} OR (${actor.role === "reviewer"} AND status <> 'draft'))`;
  if (!rows[0])
    throw new DomainError(
      404,
      "not_found",
      "The resource was not found in this workspace.",
    );
  const resource = rows[0] as Resource;
  if (
    input.action === "submit" &&
    rows[0].authoring &&
    (!rows[0].authoring_approval ||
      rows[0].authoring_approval.digest !==
        preparationDigest(resource, rows[0].authoring))
  ) {
    throw new DomainError(
      409,
      "preparation_approval_required",
      "Review and approve the exact preparation draft before sending its source for maintainer review.",
    );
  }
  const status = assertTransition(actor, resource, input);
  const updated = await sql`WITH resource AS (
    UPDATE fontes_cloud.resources SET status = ${status}, revision = revision + 1, updated_at = now()
    WHERE id = ${id} AND revision = ${input.revision} AND status = ${resource.status} RETURNING *
  ), event AS (
    INSERT INTO fontes_cloud.events (resource_id, actor_id, action, note)
    SELECT id, ${actor.id}, ${input.action}, ${input.note || (input.action === "submit" ? "Submitted metadata for review." : "Metadata approved; source validation and trusted publication are still required.")} FROM resource
  ) SELECT resource.*, a.name AS owner_name FROM resource JOIN fontes_cloud.accounts a ON a.id = resource.owner_id`;
  if (!updated[0])
    throw new DomainError(
      409,
      "revision_conflict",
      "Another action changed this resource. Refresh and try again.",
    );
  return wireResource(updated[0]);
}

export async function updateResource(actor: Actor, id: string, raw: unknown) {
  const input = updateResourceSchema.parse(raw);
  const sql = database();
  const rows = await sql`WITH resource AS (
    UPDATE fontes_cloud.resources SET title = ${input.title}, description = ${input.description}, kind = ${input.kind},
      language = ${input.language}, version = ${input.version}, status = 'draft', revision = revision + 1, updated_at = now()
    WHERE id = ${id} AND owner_id = ${actor.id} AND revision = ${input.revision} AND status IN ('draft', 'changes_requested') AND authoring IS NULL RETURNING *
  ), event AS (
    INSERT INTO fontes_cloud.events (resource_id, actor_id, action, note)
    SELECT id, ${actor.id}, 'edited', 'Updated the draft metadata.' FROM resource
  ) SELECT resource.*, a.name AS owner_name FROM resource JOIN fontes_cloud.accounts a ON a.id = resource.owner_id`;
  if (!rows[0]) {
    const visible =
      await sql`SELECT id FROM fontes_cloud.resources WHERE id = ${id} AND owner_id = ${actor.id}`;
    if (!visible[0])
      throw new DomainError(
        404,
        "not_found",
        "The resource was not found in this workspace.",
      );
    throw new DomainError(
      409,
      "revision_conflict",
      "Only the current editable draft can be changed. Refresh and try again.",
    );
  }
  return wireResource(rows[0]);
}
