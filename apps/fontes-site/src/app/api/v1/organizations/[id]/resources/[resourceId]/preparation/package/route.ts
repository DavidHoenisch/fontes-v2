import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { downloadReviewPackage } from "@/lib/publisher-review";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string; resourceId: string }> },
) {
  let output: Awaited<ReturnType<typeof downloadReviewPackage>> | undefined;
  const response = await api(async () => {
    const p = await ctx.params;
    output = await downloadReviewPackage(
      await currentActor(request),
      p.id,
      p.resourceId,
    );
    return null;
  });
  if (!output) return response;
  return new Response(new Uint8Array(output.buffer), {
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="${output.filename}"`,
      "Cache-Control": "private, no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
