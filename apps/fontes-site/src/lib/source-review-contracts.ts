import { z } from "zod";
const digest = z.string().regex(/^[a-f0-9]{64}$/),
  revision = z.number().int().positive();
export const sourceSubmissionSchema = z
  .object({
    source: z.discriminatedUnion("kind", [
      z
        .object({
          kind: z.literal("personal"),
          resourceId: z.string().max(160),
        })
        .strict(),
      z
        .object({
          kind: z.literal("organization"),
          organizationId: z.uuid(),
          resourceId: z.uuid(),
          organizationRevision: revision,
          reviewRevision: z.number().int().nonnegative(),
        })
        .strict(),
    ]),
    resourceRevision: revision,
    consent: z.literal(true),
    releaseNotes: z.string().trim().min(10).max(4000),
  })
  .strict();
export const sourceDecisionSchema = z
  .object({
    revision,
    sourceSha256: digest,
    command: z.discriminatedUnion("action", [
      z
        .object({
          action: z.enum(["comment", "request_changes", "withdraw"]),
          note: z.string().trim().min(5).max(4000),
        })
        .strict(),
      z
        .object({
          action: z.literal("approve"),
          note: z.string().trim().min(5).max(4000),
          rightsReviewId: z.string().trim().min(3).max(240),
          publisherReviewId: z.string().trim().min(3).max(240),
          checks: z
            .object({
              publisher: z.literal(true),
              textRights: z.literal(true),
              artworkRights: z.literal(true),
              content: z.literal(true),
              references: z.literal(true),
            })
            .strict(),
        })
        .strict(),
      z.object({ action: z.literal("handoff") }).strict(),
    ]),
  })
  .strict();
export const sourceReviewItemSchema = z.object({
  id: z.uuid(),
  title: z.string(),
  resource_id: z.string(),
  publisher_id: z.string(),
  version: z.string(),
  source_sha256: digest,
  upstream_sha256: digest,
  status: z.enum([
    "submitted",
    "changes_requested",
    "approved",
    "handoff_ready",
    "withdrawn",
    "superseded",
  ]),
  revision,
  submitted_by: z.string(),
  publisher_actor: z.string(),
  creator_id: z.string(),
  organization_id: z.uuid().nullable(),
  personal_resource_id: z.string().nullable(),
  organization_resource_id: z.uuid().nullable(),
  origin_revision: revision,
  created_at: z.string(),
  updated_at: z.string(),
  independent: z.boolean(),
  canWithdraw: z.boolean(),
  approval: z
    .object({
      actor: z.string(),
      at: z.string(),
      sourceSha256: digest,
      rightsReviewId: z.string(),
      publisherReviewId: z.string(),
      note: z.string(),
      checks: z.record(z.string(), z.boolean()),
    })
    .nullable(),
  handoff: z.object({ sha256: digest, filename: z.string() }).nullable(),
});
export const packagePreviewSchema = z.object({
  title: z.string(),
  description: z.string(),
  kind: z.string(),
  language: z.string(),
  releaseNotes: z.string(),
  fields: z.record(z.string(), z.unknown()),
  documents: z
    .array(
      z.object({
        id: z.string(),
        title: z.string(),
        sha256: digest,
        blocks: z.number().int().nonnegative(),
      }),
    )
    .max(256),
  files: z
    .array(
      z.object({
        path: z.string(),
        sha256: digest,
        bytes: z.number().int().nonnegative(),
      }),
    )
    .max(2000),
  validation: z.unknown(),
  publisherApproval: z.object({ actor: z.string(), at: z.string(), digest }),
});
export const sourceReviewDetailSchema = z.object({
  item: sourceReviewItemSchema,
  preview: packagePreviewSchema,
  events: z
    .array(
      z.object({
        id: z.uuid(),
        actor_id: z.string().nullable(),
        details: z.record(z.string(), z.unknown()),
        action: z.string(),
        note: z.string(),
        created_at: z.string(),
      }),
    )
    .max(100),
});
export type SourceReviewItem = z.infer<typeof sourceReviewItemSchema>;
export type SourceReviewDetail = z.infer<typeof sourceReviewDetailSchema>;

export const sourceReviewPageSchema = z.object({
  id: z.string(),
  title: z.string(),
  sourcePath: z.string(),
  sourceAssertions: z.array(z.string()).optional(),
  blocks: z
    .array(
      z.object({
        id: z.string(),
        kind: z.enum([
          "heading",
          "paragraph",
          "list-item",
          "verse",
          "entry",
          "note",
        ]),
        text: z.string().max(64000),
        sourceAssertions: z.array(z.string()).optional(),
      }),
    )
    .max(40),
  offset: z.number().int().nonnegative(),
  total: z.number().int().nonnegative(),
  nextOffset: z.number().int().nonnegative().nullable(),
  previousOffset: z.number().int().nonnegative().nullable(),
});
