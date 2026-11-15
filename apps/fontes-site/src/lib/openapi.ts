import { organizationPaths, organizationSchemas } from "./organization-openapi";
import { z } from "zod";
import {
  sourceSubmissionSchema,
  sourceDecisionSchema,
  sourceReviewItemSchema,
  sourceReviewDetailSchema,
  sourceReviewPageSchema,
} from "./source-review-contracts";
import { intakeSaveSchema, intakeReplySchema } from "./intake-contracts";
import {
  createResourceSchema,
  updateResourceSchema,
  transitionSchema,
  resourceSchema,
  workspaceSchema,
} from "./contracts";
const error = {
  description: "Structured error with stable code and request ID.",
  content: {
    "application/json": { schema: { $ref: "#/components/schemas/Error" } },
  },
};
const response = (schema: object) => ({
  description: "Success envelope: data plus request_id.",
  content: {
    "application/json": {
      schema: {
        type: "object",
        required: ["data", "request_id"],
        properties: {
          data: schema,
          request_id: { type: "string", format: "uuid" },
        },
      },
    },
  },
});
const success = response({ $ref: "#/components/schemas/Resource" });
const workspaceResponse = response({ $ref: "#/components/schemas/Workspace" });
const listResponse = response({
  type: "object",
  required: ["items", "limit"],
  properties: {
    items: {
      type: "array",
      maxItems: 100,
      items: { $ref: "#/components/schemas/Resource" },
    },
    limit: { const: 100 },
  },
});
const pathId = {
  name: "id",
  in: "path",
  required: true,
  schema: { type: "string" },
};
const body = (schema: string) => ({
  required: true,
  content: {
    "application/json": { schema: { $ref: `#/components/schemas/${schema}` } },
  },
});
export const openapi = {
  openapi: "3.1.0",
  info: {
    title: "Fontes publishing API",
    version: "0.1.0-draft",
    description:
      "Development publishing workspace with metadata review and local-only source preparation. EPUB/prepared-package uploads, publisher preparation approval and source ZIP export do not publish or authorize source releases. AI enrichment is not connected. Organization roles, single-use invitation links and private metadata drafts are available; private publication, email delivery, offline grants, payment and native token APIs remain unimplemented. Cookie sessions or guarded local personas. Mutations require the configured Origin. Lists are bounded, not complete catalogs.",
  },
  servers: [{ url: "/" }],
  security: [{ SDKSession: [] }],
  paths: {
    ...organizationPaths,
    "/api/v1/source-reviews": {
      get: {
        operationId: "listSourceReviews",
        description:
          "Explicitly submitted public-free source snapshots only. Current publishing team or Fontes maintainer; active queue or latest 100 inactive history records. Local development.",
        parameters: [
          { name: "history", in: "query", schema: { type: "boolean" } },
        ],
        responses: {
          "200": response({
            type: "object",
            properties: {
              items: {
                type: "array",
                items: { $ref: "#/components/schemas/SourceReviewItem" },
              },
              limit: { const: 100 },
              history: { type: "boolean" },
            },
          }),
          "401": error,
        },
      },
      post: {
        operationId: "submitSourcePackage",
        description:
          "Freeze an approved personal/org source package after explicit public-review consent. Requires origin revisions. Two strict canonical builds; pointer/audit commit rechecks authorization and revisions.",
        requestBody: body("SourceSubmission"),
        responses: {
          "201": response({ $ref: "#/components/schemas/SourceReviewDetail" }),
          "400": error,
          "404": error,
          "409": error,
          "422": error,
          "503": error,
        },
      },
    },
    "/api/v1/source-reviews/{id}": {
      get: {
        operationId: "readSourceReview",
        parameters: [pathId],
        responses: {
          "200": response({ $ref: "#/components/schemas/SourceReviewDetail" }),
          "404": error,
        },
      },
      patch: {
        operationId: "decideSourceReview",
        parameters: [pathId],
        requestBody: body("SourceDecision"),
        description:
          "Feedback, changes, independent exact-hash approval, withdrawal or unsigned publication handoff. All mutations require Origin. No live signing or publication.",
        responses: {
          "200": response({ $ref: "#/components/schemas/SourceReviewDetail" }),
          "400": error,
          "403": error,
          "404": error,
          "409": error,
        },
      },
    },
    "/api/v1/source-reviews/{id}/document": {
      get: {
        operationId: "sourceReviewDocument",
        parameters: [
          pathId,
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
        ],
        responses: {
          "200": response({ $ref: "#/components/schemas/SourceReviewPage" }),
          "404": error,
          "409": error,
        },
      },
    },
    ...Object.fromEntries(
      ["file", "package", "handoff"].map((operation) => [
        `/api/v1/source-reviews/{id}/${operation}`,
        {
          get: {
            operationId: `downloadReviewed${operation}`,
            parameters: [
              pathId,
              ...(operation === "file"
                ? [
                    {
                      name: "path",
                      in: "query",
                      required: true,
                      schema: { type: "string" },
                    },
                  ]
                : []),
            ],
            description:
              "Scoped private verified bytes from the frozen inventory. Inactive reviews cannot download; handoff requires handoff_ready. No signing/publication.",
            responses: {
              "200": {
                description: "Private binary bytes; no-store/nosniff",
                content: {
                  [operation === "file"
                    ? "application/octet-stream"
                    : "application/zip"]: {
                    schema: { type: "string", format: "binary" },
                  },
                },
              },
              "404": error,
              "409": error,
            },
          },
        },
      ]),
    ),
    "/api/v1/resources/{id}/intake": {
      get: {
        operationId: "getPreparation",
        parameters: [pathId],
        responses: {
          "200": response({ $ref: "#/components/schemas/Preparation" }),
          "401": error,
          "404": error,
        },
      },
      put: {
        operationId: "savePreparation",
        parameters: [pathId],
        requestBody: body("SavePreparation"),
        responses: {
          "200": response({ $ref: "#/components/schemas/Preparation" }),
          "400": error,
          "403": error,
          "409": error,
          "413": error,
        },
      },
    },
    "/api/v1/resources/{id}/intake/{operation}": {
      post: {
        operationId: "prepareSource",
        description:
          "Local-development binary uploads: epub/prepared up to 20 MiB, cover up to 4 MiB. Required X-Fontes-Revision binds current resource. Approve instead accepts application/json {revision} and validates/packages exact bytes before recording publisher approval. No signing or publication.",
        parameters: [
          pathId,
          {
            name: "operation",
            in: "path",
            required: true,
            schema: { enum: ["epub", "prepared", "cover", "approve"] },
          },
          {
            name: "X-Fontes-Revision",
            in: "header",
            schema: { type: "integer", minimum: 1 },
          },
        ],
        requestBody: {
          required: true,
          content: {
            "application/octet-stream": {
              schema: { type: "string", format: "binary" },
            },
            "application/json": {
              schema: {
                type: "object",
                required: ["revision"],
                additionalProperties: false,
                properties: { revision: { type: "integer", minimum: 1 } },
              },
            },
          },
        },
        responses: {
          "200": response({ $ref: "#/components/schemas/Preparation" }),
          "403": error,
          "404": error,
          "409": error,
          "413": error,
          "422": error,
          "503": error,
        },
      },
      get: {
        operationId: "downloadPreparedAsset",
        parameters: [
          pathId,
          {
            name: "operation",
            in: "path",
            required: true,
            schema: { enum: ["cover", "package"] },
          },
        ],
        responses: {
          "200": {
            description:
              "Private validated cover bytes or approval-bound source ZIP. Prepared ZIP bytes are unchanged; guided ZIP uses strict Rust validation/packing. This is not a signed app delivery.",
            content: {
              "application/zip": {
                schema: { type: "string", format: "binary" },
              },
              "image/png": { schema: { type: "string", format: "binary" } },
              "image/jpeg": { schema: { type: "string", format: "binary" } },
            },
          },
          "403": error,
          "404": error,
          "409": error,
          "422": error,
          "503": error,
        },
      },
    },
    "/api/v1/workspace": {
      get: {
        operationId: "getWorkspace",
        summary: "Current actor, scoped resources, activity and capabilities",
        responses: {
          "200": workspaceResponse,
          "401": error,
          "403": error,
          "503": error,
        },
      },
    },
    "/api/v1/resources": {
      get: {
        operationId: "listResources",
        summary: "List up to 100 accessible metadata records",
        responses: { "200": listResponse, "401": error },
      },
      post: {
        operationId: "createResource",
        summary: "Create an owned metadata draft",
        requestBody: body("CreateResource"),
        responses: {
          "201": success,
          "400": error,
          "401": error,
          "403": error,
          "413": error,
          "415": error,
        },
      },
    },
    "/api/v1/resources/{id}": {
      get: {
        operationId: "getResource",
        parameters: [pathId],
        responses: { "200": success, "401": error, "404": error },
      },
      patch: {
        operationId: "updateResource",
        parameters: [pathId],
        summary: "Edit an owned draft with optimistic concurrency",
        requestBody: body("UpdateResource"),
        responses: {
          "200": success,
          "400": error,
          "403": error,
          "404": error,
          "409": error,
        },
      },
    },
    "/api/v1/resources/{id}/transitions": {
      post: {
        operationId: "transitionResource",
        summary: "Submit metadata or record an independent metadata review",
        parameters: [pathId],
        requestBody: body("TransitionResource"),
        responses: {
          "200": success,
          "400": error,
          "401": error,
          "403": error,
          "404": error,
          "409": error,
        },
      },
    },
  },
  components: {
    securitySchemes: {
      SDKSession: {
        type: "apiKey",
        in: "header",
        name: "Cookie",
        description:
          "SDK-managed web session cookies; no native token contract is established. Development bypass is local-runtime only and not enabled by request headers.",
      },
    },
    schemas: {
      ...organizationSchemas,
      SourceSubmission: z.toJSONSchema(sourceSubmissionSchema),
      SourceDecision: z.toJSONSchema(sourceDecisionSchema),
      SourceReviewItem: z.toJSONSchema(sourceReviewItemSchema),
      SourceReviewDetail: z.toJSONSchema(sourceReviewDetailSchema),
      SourceReviewPage: z.toJSONSchema(sourceReviewPageSchema),
      CreateResource: z.toJSONSchema(createResourceSchema),
      SavePreparation: z.toJSONSchema(intakeSaveSchema),
      Preparation: z.toJSONSchema(intakeReplySchema),
      UpdateResource: z.toJSONSchema(updateResourceSchema),
      TransitionResource: z.toJSONSchema(transitionSchema),
      Resource: z.toJSONSchema(resourceSchema),
      Workspace: z.toJSONSchema(workspaceSchema),
      Error: {
        type: "object",
        required: ["error"],
        properties: {
          error: {
            type: "object",
            required: ["code", "message", "request_id"],
            properties: {
              code: { type: "string" },
              message: { type: "string" },
              request_id: { type: "string" },
              details: {},
            },
          },
        },
      },
    },
  },
};
