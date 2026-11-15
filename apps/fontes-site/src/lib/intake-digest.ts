import { createHash } from "node:crypto";
import type { Resource } from "./contracts";
import type { IntakeState } from "./intake-contracts";
export function preparationDigest(
  resource: Pick<
    Resource,
    "id" | "title" | "description" | "kind" | "language" | "version"
  >,
  state: IntakeState,
) {
  return createHash("sha256")
    .update(
      JSON.stringify({
        resource: {
          id: resource.id,
          title: resource.title,
          description: resource.description,
          kind: resource.kind,
          language: resource.language,
          version: resource.version,
        },
        state,
      }),
    )
    .digest("hex");
}
