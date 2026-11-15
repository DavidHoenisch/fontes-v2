import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { getReviewDocument } from "@/lib/organization-epub";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string; resourceId: string }> },
) {
  return api(async () => {
    const p = await ctx.params;
    return getReviewDocument(
      await currentActor(request),
      p.id,
      p.resourceId,
      new URL(request.url).searchParams,
    );
  });
}
