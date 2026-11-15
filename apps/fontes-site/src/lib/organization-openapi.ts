import {
  reviewReplySchema,
  reviewPageSchema,
  reviewMutationSchema,
} from "./epub-review-contracts";
import { z } from "zod";
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
const envelope = (schema: object) => ({
  description: "Private success envelope; no-store.",
  content: {
    "application/json": {
      schema: {
        type: "object",
        required: ["data", "request_id"],
        properties: { data: schema, request_id: { type: "string" } },
      },
    },
  },
});
const ref = (name: string) => ({ $ref: `#/components/schemas/${name}` });
const body = (name: string) => ({
  required: true,
  content: { "application/json": { schema: ref(name) } },
});
const parameter = (name: string) => ({
  name,
  in: "path",
  required: true,
  schema: { type: "string" },
});
const errors = {
  "400": { description: "Invalid request" },
  "403": { description: "Origin or role rejected" },
  "404": { description: "Not available to this account" },
  "409": {
    description:
      "Stale revision, inactive invite, last owner or capacity conflict",
  },
  "429": { description: "Creation/invitation limit" },
};
const operation = (
  id: string,
  input?: string,
  output = "OrganizationDetail",
  parameters: object[] = [],
) => ({
  operationId: id,
  parameters,
  ...(input ? { requestBody: body(input) } : {}),
  responses: { "200": envelope(ref(output)), ...errors },
});
const org = [parameter("id")];
export const organizationPaths = {
  "/api/v1/organizations/{id}/resources/{resourceId}/preparation": {
    get: operation("getOrganizationEpubReview", undefined, "EpubReview", [
      parameter("id"),
      parameter("resourceId"),
    ]),
    patch: {
      ...operation(
        "editApproveOrGenerateOrganizationReview",
        "PublisherReviewMutation",
        "EpubReview",
        [parameter("id"), parameter("resourceId")],
      ),
      description:
        "Local publisher corrections, fields, content approval and subsequent source generation. JSON limit 2 MiB. Requires resourceRevision, organizationRevision, reviewRevision and a strict command. Edits invalidate approval; package requires current digest approval and strict Rust validation. No signing/publication.",
    },
    post: {
      operationId: "importOrganizationEpub",
      description:
        "Local-development import; retains original extraction and resets publisher corrections/approval. Source/prose stay in private local files. Publishing role and current resource/organization revisions required; rechecked under lock. No member release or publication.",
      parameters: [
        parameter("id"),
        parameter("resourceId"),
        {
          name: "X-Fontes-Revision",
          in: "header",
          required: true,
          schema: { type: "integer", minimum: 1 },
        },
        {
          name: "X-Fontes-Organization-Revision",
          in: "header",
          required: true,
          schema: { type: "integer", minimum: 1 },
        },
      ],
      requestBody: {
        required: true,
        content: {
          "application/epub+zip": {
            schema: { type: "string", format: "binary", maxLength: 20971520 },
          },
        },
      },
      responses: {
        "200": envelope(ref("EpubReview")),
        "413": { description: "Upload/evaluation limit" },
        "422": { description: "Unsupported content or failed fidelity check" },
        "503": { description: "Local storage unavailable" },
        ...errors,
      },
    },
  },
  "/api/v1/organizations/{id}/resources/{resourceId}/preparation/cover": {
    post: {
      operationId: "uploadOrganizationReviewCover",
      parameters: [
        parameter("id"),
        parameter("resourceId"),
        ...[
          "X-Fontes-Revision",
          "X-Fontes-Organization-Revision",
          "X-Fontes-Review-Revision",
        ].map((name) => ({
          name,
          in: "header",
          required: true,
          schema: { type: "integer", minimum: 0 },
        })),
      ],
      requestBody: {
        required: true,
        content: {
          "image/png": {
            schema: { type: "string", format: "binary", maxLength: 4194304 },
          },
          "image/jpeg": {
            schema: { type: "string", format: "binary", maxLength: 4194304 },
          },
        },
      },
      responses: {
        "200": envelope(ref("EpubReview")),
        "422": {
          description: "Cover must decode upright, single-frame 1200 × 1800",
        },
        ...errors,
      },
    },
  },
  "/api/v1/organizations/{id}/resources/{resourceId}/preparation/package": {
    get: {
      operationId: "downloadOrganizationSourcePackage",
      parameters: [parameter("id"), parameter("resourceId")],
      description:
        "Local-only, current publishing-team access and current publisher approval required. Returns stored hash-verified Rust-packed source ZIP. No compiled app delivery or signing.",
      responses: {
        "200": {
          description: "Approved generated source bytes",
          content: {
            "application/zip": { schema: { type: "string", format: "binary" } },
          },
        },
        ...errors,
      },
    },
  },
  "/api/v1/organizations/{id}/resources/{resourceId}/preparation/document": {
    get: operation("getOrganizationEpubDocument", undefined, "EpubReviewPage", [
      parameter("id"),
      parameter("resourceId"),
      {
        name: "document",
        in: "query",
        required: true,
        schema: { type: "string" },
      },
      {
        name: "offset",
        in: "query",
        schema: { type: "integer", minimum: 0, maximum: 25000 },
      },
      { name: "block", in: "query", schema: { type: "string" } },
      { name: "pending", in: "query", schema: { enum: ["next"] } },
      { name: "after", in: "query", schema: { type: "string" } },
    ]),
  },
  "/api/v1/organizations/{id}/resources/{resourceId}/preparation/assets/{digest}":
    {
      get: {
        operationId: "getOrganizationEpubImage",
        description:
          "Private verified PNG/JPEG bytes, only if listed in the current preparation. Local runtime and publishing role required.",
        parameters: [
          parameter("id"),
          parameter("resourceId"),
          parameter("digest"),
        ],
        responses: {
          "200": {
            description: "Source image bytes",
            content: {
              "image/png": { schema: { type: "string", format: "binary" } },
              "image/jpeg": { schema: { type: "string", format: "binary" } },
            },
          },
          ...errors,
        },
      },
    },
  "/api/v1/organizations": {
    get: operation("listOrganizations", undefined, "OrganizationList"),
    post: {
      ...operation("createOrganization", "CreateOrganization"),
      responses: { "201": envelope(ref("OrganizationDetail")), ...errors },
    },
  },
  "/api/v1/organizations/{id}": {
    get: operation("getOrganization", undefined, "OrganizationDetail", org),
    patch: operation(
      "updateOrganization",
      "UpdateOrganization",
      "OrganizationDetail",
      org,
    ),
  },
  "/api/v1/organizations/{id}/members/{accountId}": {
    patch: operation(
      "changeOrganizationRole",
      "ChangeMember",
      "OrganizationMutation",
      [...org, parameter("accountId")],
    ),
    delete: operation(
      "removeOrganizationMember",
      "OrganizationRevision",
      "OrganizationMutation",
      [...org, parameter("accountId")],
    ),
  },
  "/api/v1/organizations/{id}/leave": {
    post: operation(
      "leaveOrganization",
      "OrganizationRevision",
      "OrganizationMutation",
      org,
    ),
  },
  "/api/v1/organizations/{id}/invitations": {
    post: {
      ...operation(
        "inviteOrganizationMember",
        "InviteMember",
        "InvitationLink",
        org,
      ),
      responses: { "201": envelope(ref("InvitationLink")), ...errors },
    },
  },
  "/api/v1/organizations/{id}/invitations/{invitationId}": {
    post: {
      ...operation(
        "renewOrganizationInvitation",
        "OrganizationRevision",
        "InvitationLink",
        [...org, parameter("invitationId")],
      ),
      description:
        "Rotate the single-use link; old token stops working. No email is sent.",
    },
    delete: operation(
      "revokeOrganizationInvitation",
      "OrganizationRevision",
      "OrganizationMutation",
      [...org, parameter("invitationId")],
    ),
  },
  "/api/v1/invitations/preview": {
    post: {
      ...operation("previewInvitation", "InvitationToken", "InvitationPreview"),
      description:
        "Token in JSON only; matching verified email required. Never put the token in a query/path.",
    },
  },
  "/api/v1/invitations/accept": {
    post: operation("acceptInvitation", "InvitationToken"),
  },
  "/api/v1/organizations/{id}/resources": {
    post: {
      ...operation(
        "createOrganizationDraft",
        "CreateOrganizationResource",
        "OrganizationDetail",
        org,
      ),
      responses: { "201": envelope(ref("OrganizationDetail")), ...errors },
    },
  },
  "/api/v1/organizations/{id}/resources/{resourceId}": {
    patch: operation(
      "editOrganizationDraft",
      "UpdateOrganizationResource",
      "OrganizationDetail",
      [...org, parameter("resourceId")],
    ),
  },
};
export const organizationSchemas = {
  PublisherReviewMutation: z.toJSONSchema(reviewMutationSchema),
  EpubReview: z.toJSONSchema(reviewReplySchema),
  EpubReviewPage: z.toJSONSchema(reviewPageSchema),
  CreateOrganization: z.toJSONSchema(organizationInput),
  UpdateOrganization: z.toJSONSchema(organizationUpdate),
  Organization: z.toJSONSchema(organizationSchema),
  OrganizationDetail: z.toJSONSchema(organizationDetailSchema),
  OrganizationList: z.toJSONSchema(
    z.object({
      items: z.array(organizationSchema).max(20),
      limit: z.literal(20),
    }),
  ),
  InviteMember: z.toJSONSchema(invitationInput),
  ChangeMember: z.toJSONSchema(memberInput),
  OrganizationRevision: z.toJSONSchema(revisionInput),
  InvitationToken: z.toJSONSchema(inviteTokenInput),
  InvitationPreview: z.toJSONSchema(invitationPreviewSchema),
  CreateOrganizationResource: z.toJSONSchema(organizationResourceInput),
  UpdateOrganizationResource: z.toJSONSchema(organizationResourceUpdate),
  OrganizationMutation: {
    type: "object",
    required: ["id"],
    properties: { id: { type: "string", format: "uuid" } },
  },
  InvitationLink: {
    type: "object",
    required: [
      "organization_id",
      "invitation_id",
      "token",
      "expires_in_seconds",
      "delivery",
    ],
    properties: {
      organization_id: { type: "string", format: "uuid" },
      invitation_id: { type: "string", format: "uuid" },
      token: { type: "string", pattern: "^[A-Za-z0-9_-]{43}$" },
      expires_in_seconds: { const: 604800 },
      delivery: { const: "copy_link" },
    },
  },
};
