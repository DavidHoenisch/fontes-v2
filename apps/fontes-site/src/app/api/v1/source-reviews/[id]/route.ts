import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { sourceReviewDetail, decideSourceReview } from "@/lib/source-review";
type Context = { params: Promise<{ id: string }> };
export async function GET(request: Request, ctx: Context) {
  return api(async () =>
    sourceReviewDetail(await currentActor(request), (await ctx.params).id),
  );
}
export async function PATCH(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    return decideSourceReview(
      await currentActor(request),
      (await ctx.params).id,
      await jsonBody(request),
    );
  });
}
