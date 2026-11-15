import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { reviewDocument } from "@/lib/source-review";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  return api(async () =>
    reviewDocument(
      await currentActor(request),
      (await ctx.params).id,
      new URL(request.url).searchParams,
    ),
  );
}
