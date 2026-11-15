import { z } from "zod";
import { createResourceSchema } from "./contracts";

export const permissionKeys = [
  "offline-display",
  "keyword-search",
  "export",
  "print",
  "embeddings",
  "ai-context",
] as const;
const field = z.string().max(4000).default("");
export const intakeFieldsSchema = z
  .object({
    author: field,
    publisher: field,
    edition: field,
    publicationDate: field,
    sourceVersification: field,
    coverageStart: field,
    coverageEnd: field,
    licenseId: field,
    rightsHolder: field,
    copyright: field,
    attribution: field,
    licenseText: z.string().max(100_000).default(""),
    coverAlt: field,
    coverLicenseId: field,
    coverRightsHolder: field,
    coverAttribution: field,
    permissions: z
      .object(
        Object.fromEntries(
          permissionKeys.map((k) => [
            k,
            z.enum(["allow", "deny"]).default("deny"),
          ]),
        ) as Record<
          (typeof permissionKeys)[number],
          z.ZodDefault<z.ZodEnum<{ allow: "allow"; deny: "deny" }>>
        >,
      )
      .default({
        "offline-display": "deny",
        "keyword-search": "deny",
        export: "deny",
        print: "deny",
        embeddings: "deny",
        "ai-context": "deny",
      }),
    rightsAttested: z.boolean().default(false),
    conversionAcknowledged: z.boolean().default(false),
  })
  .strict();
export type IntakeFields = z.infer<typeof intakeFieldsSchema>;
export type IntakeBlock = {
  id: string;
  kind: "heading" | "paragraph" | "list-item" | "verse" | "entry" | "note";
  text: string;
  sourcePath: string;
  sourceElementId?: string;
  sourceAssertions?: string[];
};
export type IntakeDocument = {
  id: string;
  title: string;
  sourcePath: string;
  blocks: IntakeBlock[];
};
export type Proposal = {
  id: string;
  blockId: string;
  evidence: string;
  start: string;
  end?: string;
  method: "literal-reference-v1";
  decision: "pending" | "accepted" | "rejected";
};
export type IntakeState = {
  method: "guided" | "prepared";
  fields: IntakeFields;
  documents: IntakeDocument[];
  proposals: Proposal[];
  warnings: string[];
  source?: {
    sha256: string;
    name: string;
    bytes: number;
    format: "epub" | "fontes-source";
    resourceId?: string;
    publisherId?: string;
    report?: unknown;
  };
  cover?: {
    sha256: string;
    mediaType: "image/png" | "image/jpeg";
    bytes: number;
  };
};
export type IntakeReply = {
  resourceId: string;
  revision: number;
  state: IntakeState;
  approval: {
    digest: string;
    package_sha256: string;
    actor: string;
    at: string;
  } | null;
  checks: string[];
  ai: { configured: false };
};
export const intakeSaveSchema = z
  .object({
    revision: z.number().int().positive(),
    method: z.enum(["guided", "prepared"]),
    fields: intakeFieldsSchema,
    metadata: createResourceSchema.optional(),
    decisions: z
      .array(
        z
          .object({
            id: z.string().max(96),
            decision: z.enum(["pending", "accepted", "rejected"]),
          })
          .strict(),
      )
      .max(5000)
      .default([]),
  })
  .strict();
export function emptyIntake(
  method: IntakeState["method"] = "guided",
): IntakeState {
  return {
    method,
    fields: intakeFieldsSchema.parse({ sourceVersification: "KJV" }),
    documents: [],
    proposals: [],
    warnings: [],
  };
}
const blockSchema = z.object({
  id: z.string(),
  kind: z.enum(["heading", "paragraph", "list-item", "verse", "entry", "note"]),
  text: z.string(),
  sourcePath: z.string(),
  sourceElementId: z.string().optional(),
  sourceAssertions: z.array(z.string()).optional(),
});
export const intakeReplySchema = z.object({
  resourceId: z.string(),
  revision: z.number().int().positive(),
  state: z.object({
    method: z.enum(["guided", "prepared"]),
    fields: intakeFieldsSchema,
    documents: z.array(
      z.object({
        id: z.string(),
        title: z.string(),
        sourcePath: z.string(),
        blocks: z.array(blockSchema),
      }),
    ),
    proposals: z.array(
      z.object({
        id: z.string(),
        blockId: z.string(),
        evidence: z.string(),
        start: z.string(),
        end: z.string().optional(),
        method: z.literal("literal-reference-v1"),
        decision: z.enum(["pending", "accepted", "rejected"]),
      }),
    ),
    warnings: z.array(z.string()),
    source: z
      .object({
        sha256: z.string(),
        name: z.string(),
        bytes: z.number(),
        format: z.enum(["epub", "fontes-source"]),
        resourceId: z.string().optional(),
        publisherId: z.string().optional(),
        report: z.unknown().optional(),
      })
      .optional(),
    cover: z
      .object({
        sha256: z.string(),
        mediaType: z.enum(["image/png", "image/jpeg"]),
        bytes: z.number(),
      })
      .optional(),
  }),
  approval: z
    .object({
      digest: z.string(),
      package_sha256: z.string(),
      actor: z.string(),
      at: z.string(),
    })
    .nullable(),
  checks: z.array(z.string()),
  ai: z.object({ configured: z.literal(false) }),
});
