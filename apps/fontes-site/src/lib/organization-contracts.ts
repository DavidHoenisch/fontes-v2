import { z } from "zod";
import { createResourceSchema, kinds } from "./contracts";
export const organizationRoles = [
  "owner",
  "admin",
  "publisher",
  "member",
] as const;
export const offlinePreferences = [
  "retain_downloads",
  "renewable_lease",
] as const;
export const organizationInput = z
  .object({
    name: z.string().trim().min(2).max(100),
    description: z.string().trim().max(2000).default(""),
    offline_policy_preference: z
      .enum(offlinePreferences)
      .default("retain_downloads"),
  })
  .strict();
const revision = z.number().int().positive();
export const organizationUpdate = organizationInput.extend({ revision });
export const invitationInput = z
  .object({
    revision,
    email: z.string().trim().toLowerCase().email().max(254),
    role: z.enum(["admin", "publisher", "member"]),
  })
  .strict();
export const memberInput = z
  .object({ revision, role: z.enum(organizationRoles) })
  .strict();
export const revisionInput = z.object({ revision }).strict();
export const inviteTokenInput = z
  .object({ token: z.string().regex(/^[A-Za-z0-9_-]{43}$/) })
  .strict();
export const organizationResourceInput = createResourceSchema.extend({
  revision,
});
export const organizationResourceUpdate = organizationResourceInput.extend({
  resource_revision: revision,
});
export const organizationSchema = z.object({
  id: z.uuid(),
  name: z.string(),
  description: z.string(),
  namespace: z.string(),
  role: z.enum(organizationRoles),
  revision,
  policy_revision: revision,
  offline_policy_preference: z.enum(offlinePreferences),
  created_at: z.string(),
  updated_at: z.string(),
  is_demo: z.boolean(),
});
export const organizationResourceSchema = z.object({
  has_preparation: z.boolean().default(false),
  id: z.uuid(),
  organization_id: z.uuid(),
  title: z.string(),
  description: z.string(),
  kind: z.enum(kinds),
  language: z.string(),
  version: z.string(),
  revision,
  status: z.literal("draft"),
  created_at: z.string(),
  updated_at: z.string(),
});
export const organizationDetailSchema = z.object({
  organization: organizationSchema,
  members: z
    .array(
      z.object({
        account_id: z.string(),
        name: z.string(),
        email: z.string().nullable(),
        role: z.enum(organizationRoles),
        joined_at: z.string(),
      }),
    )
    .max(100),
  invitations: z
    .array(
      z.object({
        id: z.uuid(),
        email: z.string(),
        role: z.enum(["admin", "publisher", "member"]),
        status: z.enum(["pending", "accepted", "revoked", "expired"]),
        expires_at: z.string(),
        created_at: z.string(),
      }),
    )
    .max(100),
  resources: z.array(organizationResourceSchema).max(100),
  activity: z
    .array(
      z.object({
        id: z.string(),
        action: z.string(),
        actor_name: z.string(),
        created_at: z.string(),
      }),
    )
    .max(30),
  capabilities: z.object({
    manage: z.boolean(),
    prepare_metadata: z.boolean(),
    private_publication: z.literal(false),
    email_delivery: z.literal(false),
    offline_grants: z.literal(false),
  }),
});
export const invitationPreviewSchema = z.object({
  organization_name: z.string(),
  inviter_name: z.string(),
  role: z.enum(["admin", "publisher", "member"]),
  expires_at: z.string(),
});
export type Organization = z.infer<typeof organizationSchema>;
export type OrganizationDetail = z.infer<typeof organizationDetailSchema>;
export type InvitationPreview = z.infer<typeof invitationPreviewSchema>;
export const canManageOrganization = (role: string) =>
  ["owner", "admin"].includes(role);
export const canPrepareOrganization = (role: string) =>
  ["owner", "admin", "publisher"].includes(role);
