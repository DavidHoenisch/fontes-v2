import { NextResponse } from "next/server";
import { z } from "zod";
import { resolveAuthMode } from "@/lib/auth-policy";
import { DomainError } from "@/lib/contracts";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
export async function POST(request: Request) {
  let persona: string | undefined;
  const response = await api(async () => {
    assertSameOrigin(request);
    if (
      resolveAuthMode(process.env, new URL(request.url).hostname) !==
      "development"
    )
      throw new DomainError(404, "not_found", "Not found.");
    persona = z
      .object({ persona: z.enum(["publisher", "reviewer"]) })
      .strict()
      .parse(await jsonBody(request)).persona;
    return { persona };
  });
  if (response.ok && persona)
    response.cookies.set("fontes_dev_persona", persona, {
      httpOnly: true,
      sameSite: "strict",
      path: "/",
      maxAge: 86_400,
    });
  return response as NextResponse;
}
