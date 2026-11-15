import { z } from "zod";
import { intakeFieldsSchema } from "./intake-contracts";
const count = z.number().int().nonnegative(),
  digest = z.string().regex(/^[a-f0-9]{64}$/),
  documentId = z.string().regex(/^document-[a-f0-9]{24}$/),
  blockId = z.string().regex(/^block-[a-f0-9]{24}$/);
export const reviewSegmentSchema = z.object({
  text: z.string().max(64000),
  lineBreak: z.literal(true).optional(),
  marks: z.array(z.enum(["emphasis", "strong", "superscript"])).max(64),
  target: z.object({ documentId, blockId: blockId.optional() }).optional(),
  unresolved: z.string().max(500).optional(),
});
export const publisherReferenceSchema = z
  .object({
    id: z.string().min(1).max(96),
    blockId,
    evidence: z.string().min(1).max(64000),
    start: z.string().max(40),
    end: z.string().max(40).optional(),
    method: z.enum(["literal-reference-v1", "publisher-correction-v1"]),
    decision: z.enum(["pending", "accepted", "rejected"]),
  })
  .strict();
export const reviewBlockSchema = z.object({
  id: blockId,
  kind: z.enum(["heading", "paragraph", "list-item", "note", "image"]),
  text: z.string().max(64000),
  segments: z.array(reviewSegmentSchema).max(200000),
  sourceElementId: z.string().optional(),
  image: digest.optional(),
  alt: z.string().optional(),
  poetry: z.boolean().optional(),
  references: z.array(publisherReferenceSchema).max(1000),
});
export const reviewManifestSchema = z.object({
  profile: z.literal("epub-review-v1"),
  title: z.string(),
  authors: z.array(z.string()),
  contributors: z.array(z.string()),
  publisher: z.string(),
  language: z.string(),
  declaredRights: z.string(),
  publicationNotice: z.string().max(40000),
  documents: z
    .array(
      z.object({
        id: documentId,
        title: z.string().max(1000),
        sourcePath: z.string().max(240),
        blocks: count,
        characters: count,
        sha256: digest,
      }),
    )
    .min(1)
    .max(256),
  statistics: z.object({
    documents: count,
    blocks: count,
    characters: count,
    footnotes: count,
    footnoteReferences: count,
    sourceLinks: count,
    resolvedLinks: count,
    unresolvedLinks: count,
    scriptureCandidates: count,
    images: count,
    poetryTables: count,
  }),
  fidelity: z.object({
    normalizedTextMatches: z.literal(true),
    sourceTextSha256: digest,
    extractedTextSha256: digest,
  }),
  warnings: z.array(z.string()).max(300),
  rightsReviewRequired: z.literal(true),
  exportable: z.literal(false),
  sourceSha256: digest,
  sourceBytes: count,
  assets: z
    .array(
      z.object({
        sha256: digest,
        mediaType: z.enum(["image/png", "image/jpeg"]),
        width: count,
        height: count,
      }),
    )
    .max(256),
});
export const publisherDraftSchema = z.object({
  documents: reviewManifestSchema.shape.documents,
  fields: intakeFieldsSchema,
  cover: z
    .object({
      sha256: digest,
      mediaType: z.enum(["image/png", "image/jpeg"]),
      bytes: count,
    })
    .optional(),
  pending: count,
  accepted: count,
  rejected: count,
  editedBlocks: z.array(blockId).max(25000),
  approval: z.object({ digest, actor: z.string(), at: z.string() }).nullable(),
  package: z
    .object({ sha256: digest, filename: z.string(), approvalDigest: digest })
    .nullable(),
});
export type PublisherDraft = z.infer<typeof publisherDraftSchema>;
export const reviewVersionSchema = z.object({
  resourceRevision: count,
  organizationRevision: count,
  reviewRevision: count,
});
export const reviewMutationSchema = reviewVersionSchema
  .extend({
    command: z.discriminatedUnion("action", [
      z
        .object({
          action: z.literal("correct"),
          documentId,
          blockId,
          segmentTexts: z.array(z.string().max(64000)).max(200000),
          references: z.array(publisherReferenceSchema).max(1000),
        })
        .strict(),
      z
        .object({ action: z.literal("fields"), fields: intakeFieldsSchema })
        .strict(),
      z.object({ action: z.literal("approve") }).strict(),
      z.object({ action: z.literal("package") }).strict(),
    ]),
  })
  .strict();
export const reviewReplySchema = z.object({
  resourceId: z.uuid(),
  resourceRevision: count,
  organizationRevision: count,
  preparationRevision: count,
  manifest: reviewManifestSchema.nullable(),
  reviewRevision: count,
  draft: publisherDraftSchema.nullable(),
  approved: z.boolean(),
  checks: z.array(z.string()),
});
export const reviewPageSchema = z.object({
  resourceRevision: count,
  preparationRevision: count,
  id: documentId,
  title: z.string(),
  sourcePath: z.string(),
  blocks: z
    .array(
      reviewBlockSchema.extend({
        originalText: z.string().max(64000).optional(),
      }),
    )
    .max(40),
  offset: count,
  total: count,
  nextOffset: count.nullable(),
  previousOffset: count.nullable(),
  focusBlockId: blockId.nullable(),
});
export type ReviewReply = z.infer<typeof reviewReplySchema>;
export type ReviewPage = z.infer<typeof reviewPageSchema>;
