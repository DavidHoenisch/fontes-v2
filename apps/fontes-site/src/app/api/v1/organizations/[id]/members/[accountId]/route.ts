import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { changeMember } from "@/lib/organizations";
type Context = { params: Promise<{ id: string; accountId: string }> };
export async function PATCH(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return changeMember(
      await currentActor(request),
      p.id,
      p.accountId,
      "member",
      await jsonBody(request),
    );
  });
}
export async function DELETE(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return changeMember(
      await currentActor(request),
      p.id,
      p.accountId,
      "remove",
      await jsonBody(request),
    );
  });
}
