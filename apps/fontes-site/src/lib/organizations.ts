import "server-only";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import { z } from "zod";
import { database } from "./db";
import { DomainError, type Actor } from "./contracts";
import {
  organizationInput,
  organizationUpdate,
  organizationSchema,
  organizationDetailSchema,
  invitationInput,
  memberInput,
  revisionInput,
  inviteTokenInput,
  invitationPreviewSchema,
  organizationResourceInput,
  organizationResourceUpdate,
} from "./organization-contracts";
const uuid = z.uuid();
const hash = (token: string) =>
  createHash("sha256").update(token).digest("hex");
const errors: Record<string, [number, string]> = {
  not_found: [
    404,
    "This organization or invitation is not available to your account.",
  ],
  forbidden: [403, "Your organization role does not allow this action."],
  revision_conflict: [
    409,
    "This organization changed. Refresh before trying again.",
  ],
  last_owner: [
    409,
    "Assign another owner before the last owner leaves or changes role.",
  ],
  invite_inactive: [
    409,
    "This invitation has expired, been revoked, or already been accepted.",
  ],
  invite_exists: [
    409,
    "A pending invitation exists for this email. Renew or revoke its link.",
  ],
  already_member: [
    409,
    "This account is already a member of the organization.",
  ],
  rate_limited: [
    429,
    "Too many organization or invitation requests. Try again later.",
  ],
};
async function command(
  actor: Actor,
  id: string,
  action: string,
  input: object,
) {
  const sql = database();
  const rows =
    await sql`SELECT fontes_cloud.organization_command(${actor.id},${uuid.parse(id)}::uuid,${action},${JSON.stringify(input)}::jsonb) AS result`;
  const result = rows[0].result;
  if (result.error) {
    const [status, message] = errors[result.error] ?? [
      409,
      "This prototype's organization capacity has been reached.",
    ];
    throw new DomainError(status, result.error, message);
  }
  return result as { id: string; target_id?: string };
}
export async function listOrganizations(actor: Actor) {
  const sql = database();
  const rows =
    await sql`SELECT o.*, m.role FROM fontes_cloud.organizations o JOIN fontes_cloud.organization_members m ON m.organization_id=o.id
    WHERE m.account_id=${actor.id} ORDER BY o.created_at DESC, o.id LIMIT 20`;
  return {
    items: rows.map((o) =>
      organizationSchema.parse({
        ...o,
        created_at: o.created_at.toISOString(),
        updated_at: o.updated_at.toISOString(),
      }),
    ),
    limit: 20,
  };
}
export async function getOrganization(actor: Actor, id: string) {
  const sql = database();
  // One statement/snapshot keeps roster, invitations, drafts and audit scoped to
  // current membership, without an authorization/read time-of-check gap.
  const rows = await sql`SELECT jsonb_build_object(
    'organization',to_jsonb(o)||jsonb_build_object('role',m.role),
    'members',(SELECT coalesce(jsonb_agg(x ORDER BY x.joined_at,x.account_id),'[]'::jsonb) FROM (
      SELECT a.id AS account_id,a.name,CASE WHEN m.role IN ('owner','admin') THEN a.email ELSE NULL END AS email,mm.role,mm.joined_at
      FROM fontes_cloud.organization_members mm JOIN fontes_cloud.accounts a ON a.id=mm.account_id WHERE mm.organization_id=o.id) x),
    'invitations',(SELECT coalesce(jsonb_agg(x ORDER BY x.created_at DESC,x.id),'[]'::jsonb) FROM (
      SELECT i.id,i.email,i.role,CASE WHEN i.status='pending' AND i.expires_at<=now() THEN 'expired' ELSE i.status END AS status,i.expires_at,i.created_at
      FROM fontes_cloud.organization_invitations i WHERE i.organization_id=o.id AND m.role IN ('owner','admin')) x),
    'resources',(SELECT coalesce(jsonb_agg(x ORDER BY x.updated_at DESC,x.id),'[]'::jsonb) FROM (
      SELECT r.*, EXISTS(SELECT 1 FROM fontes_cloud.organization_epub_preparations p WHERE p.resource_id=r.id) AS has_preparation FROM fontes_cloud.organization_resources r WHERE r.organization_id=o.id AND m.role IN ('owner','admin','publisher')) x),
    'activity',(SELECT coalesce(jsonb_agg(x ORDER BY x.created_at DESC,x.id),'[]'::jsonb) FROM (
      SELECT e.id,e.action,a.name AS actor_name,e.created_at FROM fontes_cloud.organization_events e JOIN fontes_cloud.accounts a ON a.id=e.actor_id
      WHERE e.organization_id=o.id ORDER BY e.created_at DESC,e.id LIMIT 30) x),
    'capabilities',jsonb_build_object('manage',m.role IN ('owner','admin'),'prepare_metadata',m.role IN ('owner','admin','publisher'),'private_publication',false,'email_delivery',false,'offline_grants',false)
  ) AS detail FROM fontes_cloud.organizations o JOIN fontes_cloud.organization_members m ON m.organization_id=o.id
  WHERE o.id=${uuid.parse(id)}::uuid AND m.account_id=${actor.id}`;
  if (!rows[0]) throw new DomainError(404, "not_found", errors.not_found[1]);
  return organizationDetailSchema.parse(rows[0].detail);
}
export async function createOrganization(actor: Actor, raw: unknown) {
  const result = await command(
    actor,
    randomUUID(),
    "create",
    organizationInput.parse(raw),
  );
  return getOrganization(actor, result.id);
}
export async function updateOrganization(
  actor: Actor,
  id: string,
  raw: unknown,
) {
  await command(actor, id, "settings", organizationUpdate.parse(raw));
  return getOrganization(actor, id);
}
export async function changeMember(
  actor: Actor,
  id: string,
  target: string,
  action: string,
  raw: unknown,
) {
  const input = (action === "member" ? memberInput : revisionInput).parse(raw);
  z.string().min(1).max(200).parse(target);
  await command(actor, id, action, { ...input, target_id: target });
  // Self-demotion/removal may remove management or all read access immediately.
  return { id };
}
export async function inviteMember(
  actor: Actor,
  id: string,
  raw: unknown,
  target?: string,
) {
  const input = target ? revisionInput.parse(raw) : invitationInput.parse(raw);
  if (target) uuid.parse(target);
  const token = randomBytes(32).toString("base64url");
  const result = await command(actor, id, target ? "renew" : "invite", {
    ...input,
    id: randomUUID(),
    target_id: target,
    token_hash: hash(token),
  });
  return {
    organization_id: id,
    invitation_id: result.target_id!,
    token,
    expires_in_seconds: 604800,
    delivery: "copy_link" as const,
  };
}
export async function revokeInvitation(
  actor: Actor,
  id: string,
  target: string,
  raw: unknown,
) {
  await command(actor, id, "revoke", {
    ...revisionInput.parse(raw),
    target_id: uuid.parse(target),
  });
  return { id };
}
export async function previewInvitation(actor: Actor, raw: unknown) {
  const { token } = inviteTokenInput.parse(raw);
  const sql = database();
  const rows =
    await sql`SELECT o.name AS organization_name,a.name AS inviter_name,i.role,i.expires_at,i.status
    FROM fontes_cloud.organization_invitations i JOIN fontes_cloud.organizations o ON o.id=i.organization_id
    JOIN fontes_cloud.accounts a ON a.id=i.invited_by
    WHERE i.token_hash=${hash(token)} AND i.email=${actor.email.toLowerCase()}`;
  if (!rows[0])
    throw new DomainError(
      404,
      "not_found",
      "This invitation is unavailable. Sign in with the verified email it was sent to.",
    );
  if (rows[0].status !== "pending" || rows[0].expires_at <= new Date())
    throw new DomainError(409, "invite_inactive", errors.invite_inactive[1]);
  return invitationPreviewSchema.parse({
    ...rows[0],
    expires_at: rows[0].expires_at.toISOString(),
  });
}
export async function acceptInvitation(actor: Actor, raw: unknown) {
  const { token } = inviteTokenInput.parse(raw);
  const result = await command(actor, randomUUID(), "accept", {
    token_hash: hash(token),
  });
  return getOrganization(actor, result.id);
}
export async function saveOrganizationResource(
  actor: Actor,
  id: string,
  raw: unknown,
  target?: string,
) {
  const input = (
    target ? organizationResourceUpdate : organizationResourceInput
  ).parse(raw);
  await command(actor, id, target ? "resource_update" : "resource_create", {
    ...input,
    id: randomUUID(),
    target_id: target ? uuid.parse(target) : undefined,
  });
  return getOrganization(actor, id);
}
