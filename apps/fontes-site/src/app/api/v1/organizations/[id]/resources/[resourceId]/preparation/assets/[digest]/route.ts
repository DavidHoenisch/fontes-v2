import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { getReviewAsset } from "@/lib/organization-epub";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string; resourceId: string; digest: string }> },
) {
  let asset: { buffer: Buffer; mediaType: string } | undefined;
  const response = await api(async () => {
    const p = await ctx.params;
    asset = await getReviewAsset(
      await currentActor(request),
      p.id,
      p.resourceId,
      p.digest,
    );
    return null;
  });
  if (!response.ok || !asset) return response;
  return new Response(new Uint8Array(asset.buffer), {
    headers: {
      "Content-Type": asset.mediaType,
      "Cache-Control": "private, no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
