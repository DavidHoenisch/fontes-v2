import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin } from "@/lib/http";
import { getEpubReview, importOrganizationEpub } from "@/lib/organization-epub";
type Context = { params: Promise<{ id: string; resourceId: string }> };
export async function GET(request: Request, ctx: Context) {
  return api(async () => {
    const p = await ctx.params;
    return getEpubReview(await currentActor(request), p.id, p.resourceId);
  });
}
export async function POST(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return importOrganizationEpub(
      await currentActor(request),
      p.id,
      p.resourceId,
      request,
    );
  });
}
export async function PATCH(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    const { mutatePublisherReview } = await import("@/lib/publisher-review");
    const { jsonBody } = await import("@/lib/http");
    return mutatePublisherReview(
      await currentActor(request),
      p.id,
      p.resourceId,
      await jsonBody(request, 2 * 1024 * 1024),
    );
  });
}
