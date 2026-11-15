import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { getOrganization, updateOrganization } from "@/lib/organizations";
type Context = { params: Promise<{ id: string }> };
export async function GET(request: Request, ctx: Context) {
  return api(async () =>
    getOrganization(await currentActor(request), (await ctx.params).id),
  );
}
export async function PATCH(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    return updateOrganization(
      await currentActor(request),
      (await ctx.params).id,
      await jsonBody(request),
    );
  });
}
