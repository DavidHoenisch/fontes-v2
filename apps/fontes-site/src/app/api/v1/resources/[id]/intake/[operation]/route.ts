import { z } from "zod";
import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import {
  approveIntake,
  exportIntake,
  intakeAsset,
  uploadIntake,
} from "@/lib/intake-service";
import { DomainError } from "@/lib/contracts";
type Context = { params: Promise<{ id: string; operation: string }> };
export async function POST(request: Request, context: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const { id, operation } = await context.params,
      actor = await currentActor(request);
    if (["epub", "prepared", "cover"].includes(operation))
      return uploadIntake(
        actor,
        id,
        request,
        operation as "epub" | "prepared" | "cover",
      );
    if (operation === "approve")
      return approveIntake(
        actor,
        id,
        z
          .object({ revision: z.number().int().positive() })
          .strict()
          .parse(await jsonBody(request)).revision,
      );
    throw new DomainError(404, "not_found", "Unknown preparation operation.");
  });
}
export async function GET(request: Request, context: Context) {
  let file:
    | { buffer: Buffer; mediaType?: string; filename?: string }
    | undefined;
  const response = await api(async () => {
    const { id, operation } = await context.params,
      actor = await currentActor(request);
    if (operation === "package") file = await exportIntake(actor, id);
    else if (operation === "cover")
      file = await intakeAsset(actor, id, "cover");
    else throw new DomainError(404, "not_found", "Unknown preparation asset.");
    return null;
  });
  if (!response.ok || !file) return response;
  return new Response(new Uint8Array(file.buffer), {
    headers: {
      "Content-Type": file.mediaType ?? "application/zip",
      "Content-Disposition": file.filename
        ? `attachment; filename="${file.filename}"`
        : "inline",
      "Cache-Control": "private, no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
