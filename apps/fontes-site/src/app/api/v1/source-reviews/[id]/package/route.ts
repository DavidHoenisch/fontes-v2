import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { sourceDownload } from "@/lib/source-review";
export async function GET(
  request: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  let output: Awaited<ReturnType<typeof sourceDownload>> | undefined;
  const response = await api(async () => {
    output = await sourceDownload(
      await currentActor(request),
      (await ctx.params).id,
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
