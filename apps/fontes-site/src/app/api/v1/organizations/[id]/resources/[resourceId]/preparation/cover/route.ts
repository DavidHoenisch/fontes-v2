import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin } from "@/lib/http";
import { uploadReviewCover } from "@/lib/publisher-review";
export async function POST(
  request: Request,
  ctx: { params: Promise<{ id: string; resourceId: string }> },
) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return uploadReviewCover(
      await currentActor(request),
      p.id,
      p.resourceId,
      request,
    );
  });
}
