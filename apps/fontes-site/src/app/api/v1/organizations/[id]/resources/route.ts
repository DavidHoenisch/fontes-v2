import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { saveOrganizationResource } from "@/lib/organizations";
export async function POST(
  request: Request,
  ctx: { params: Promise<{ id: string; resourceId?: string }> },
) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return saveOrganizationResource(
      await currentActor(request),
      p.id,
      await jsonBody(request),
      p.resourceId,
    );
  }, 201);
}
