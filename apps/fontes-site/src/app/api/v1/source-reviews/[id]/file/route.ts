import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { reviewFile } from "@/lib/source-review";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  let output: Awaited<ReturnType<typeof reviewFile>> | undefined;
  const response = await api(async () => {
    output = await reviewFile(
      await currentActor(request),
      (await ctx.params).id,
      new URL(request.url).searchParams,
    );
    return null;
  });
  if (!output) return response;
  return new Response(new Uint8Array(output.buffer), {
    headers: {
      "Content-Type": output.mediaType,
      "Content-Disposition": `attachment; filename="${output.filename}"`,
      "Cache-Control": "private, no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
