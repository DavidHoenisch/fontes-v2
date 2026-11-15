import "server-only";
import { randomUUID } from "node:crypto";
import { NextResponse } from "next/server";
import { ZodError } from "zod";
import { DomainError } from "./contracts";

export async function api(work: () => Promise<unknown>, status = 200) {
  const id = randomUUID();
  try {
    return NextResponse.json(
      { data: await work(), request_id: id },
      { status, headers: { "Cache-Control": "private, no-store" } },
    );
  } catch (error) {
    const known = error instanceof DomainError;
    const invalid = error instanceof ZodError;
    if (!known && !invalid)
      console.error("Fontes request failed", {
        request_id: id,
        category: error instanceof Error ? error.name : "unknown",
      });
    return NextResponse.json(
      {
        error: {
          code: known
            ? error.code
            : invalid
              ? "invalid_request"
              : "service_unavailable",
          message: known
            ? error.message
            : invalid
              ? "Check the supplied fields."
              : "The service is unavailable. Check server configuration or retry.",
          request_id: id,
          ...(invalid ? { details: error.flatten() } : {}),
          ...(known && error.details ? { details: error.details } : {}),
        },
      },
      {
        status: known ? error.status : invalid ? 400 : 503,
        headers: { "Cache-Control": "private, no-store" },
      },
    );
  }
}
export function assertSameOrigin(request: Request) {
  const origin = request.headers.get("origin");
  const expected = process.env.FONTES_APP_ORIGIN;
  if (!expected || !origin || origin !== expected)
    throw new DomainError(
      403,
      "origin_rejected",
      "A matching application origin is required for this mutation.",
    );
  const site = request.headers.get("sec-fetch-site");
  if (site && !["same-origin", "none"].includes(site))
    throw new DomainError(
      403,
      "origin_rejected",
      "Cross-site mutations are not allowed.",
    );
}
export async function jsonBody(request: Request, maxBytes = 16_384) {
  if (!request.headers.get("content-type")?.includes("application/json"))
    throw new DomainError(
      415,
      "unsupported_media_type",
      "Use application/json.",
    );
  const reader = request.body?.getReader();
  if (!reader)
    throw new DomainError(400, "invalid_json", "A JSON body is required.");
  let size = 0;
  const chunks: Uint8Array[] = [];
  try {
    while (true) {
      const part = await reader.read();
      if (part.done) break;
      size += part.value.length;
      if (size > maxBytes) {
        await reader.cancel();
        throw new DomainError(
          413,
          "body_too_large",
          `JSON requests are limited to ${maxBytes / 1024} KiB.`,
        );
      }
      chunks.push(part.value);
    }
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch (error) {
    if (error instanceof DomainError) throw error;
    throw new DomainError(
      400,
      "invalid_json",
      "The request body is not valid JSON.",
    );
  }
}
