import { z } from "zod";

export const statuses = [
  "draft",
  "submitted",
  "changes_requested",
  "metadata_approved",
] as const;
export const kinds = [
  "translation",
  "commentary",
  "lexicon",
  "word-study",
  "study-guide",
  "devotional",
  "book",
] as const;
export type ResourceStatus = (typeof statuses)[number];
export type Persona = "publisher" | "reviewer";
export type Actor = {
  id: string;
  name: string;
  email: string;
  role: Persona;
  namespace: string;
  mode: "development" | "neon";
};
export const createResourceSchema = z
  .object({
    title: z.string().trim().min(2).max(180),
    description: z.string().trim().min(10).max(4000),
    kind: z.enum(kinds),
    language: z.string().trim().min(2).max(40).default("en"),
    version: z
      .string()
      .regex(/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/)
      .max(40)
      .default("1.0.0"),
  })
  .strict();
export const updateResourceSchema = createResourceSchema.extend({
  revision: z.number().int().positive(),
});
export const transitionSchema = z
  .object({
    action: z.enum(["submit", "request_changes", "approve_metadata"]),
    revision: z.number().int().positive(),
    note: z.string().trim().max(2000).default(""),
  })
  .strict()
  .refine((v) => v.action !== "request_changes" || v.note.length >= 5, {
    message: "Explain the requested changes (at least five characters).",
    path: ["note"],
  });
export type Resource = {
  id: string;
  owner_id: string;
  title: string;
  description: string;
  kind: (typeof kinds)[number];
  language: string;
  version: string;
  status: ResourceStatus;
  revision: number;
  created_at: string;
  updated_at: string;
  owner_name: string;
  is_demo: boolean;
  has_authoring: boolean;
};
export type Activity = {
  id: string;
  resource_id: string;
  title: string;
  actor_name: string;
  action: string;
  note: string;
  created_at: string;
};
export type Workspace = {
  actor: Actor;
  resources: Resource[];
  activity: Activity[];
  database: "connected";
  capabilities: { organizations: true; delivery: false; uploads: boolean };
};
export type ApiError = {
  error: {
    code: string;
    message: string;
    request_id: string;
    details?: unknown;
  };
};

export const actorSchema = z.object({
  id: z.string(),
  name: z.string(),
  email: z.string(),
  role: z.enum(["publisher", "reviewer"]),
  namespace: z.string(),
  mode: z.enum(["development", "neon"]),
});
export const resourceSchema = z.object({
  id: z.string(),
  owner_id: z.string(),
  title: z.string(),
  description: z.string(),
  kind: z.enum(kinds),
  language: z.string(),
  version: z.string(),
  status: z.enum(statuses),
  revision: z.number().int().positive(),
  created_at: z.string(),
  updated_at: z.string(),
  owner_name: z.string(),
  is_demo: z.boolean(),
  has_authoring: z.boolean().default(false),
});
export const activitySchema = z.object({
  id: z.string(),
  resource_id: z.string(),
  title: z.string(),
  actor_name: z.string(),
  action: z.string(),
  note: z.string(),
  created_at: z.string(),
});
export const workspaceSchema = z.object({
  actor: actorSchema,
  resources: z.array(resourceSchema).max(100),
  activity: z.array(activitySchema).max(30),
  database: z.literal("connected"),
  capabilities: z.object({
    organizations: z.literal(true),
    delivery: z.literal(false),
    uploads: z.boolean(),
  }),
});

export class DomainError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: unknown,
  ) {
    super(message);
  }
}

export function assertTransition(
  actor: Pick<Actor, "id" | "role">,
  resource: Pick<Resource, "owner_id" | "status" | "revision">,
  input: z.infer<typeof transitionSchema>,
): ResourceStatus {
  if (resource.revision !== input.revision)
    throw new DomainError(
      409,
      "revision_conflict",
      "This draft has changed. Refresh and try again.",
    );
  if (input.action === "submit") {
    if (resource.owner_id !== actor.id)
      throw new DomainError(
        403,
        "forbidden",
        "Only the owner can submit this draft.",
      );
    if (!["draft", "changes_requested"].includes(resource.status))
      throw new DomainError(
        409,
        "invalid_transition",
        "Only drafts or requested revisions can be submitted.",
      );
    return "submitted";
  }
  if (actor.role !== "reviewer" || resource.owner_id === actor.id)
    throw new DomainError(
      403,
      "forbidden",
      "An independent maintainer must review this resource.",
    );
  if (resource.status !== "submitted")
    throw new DomainError(
      409,
      "invalid_transition",
      "This resource is not awaiting review.",
    );
  return input.action === "request_changes"
    ? "changes_requested"
    : "metadata_approved";
}
