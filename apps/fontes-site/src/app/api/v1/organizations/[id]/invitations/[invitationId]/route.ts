import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { inviteMember, revokeInvitation } from "@/lib/organizations";
type Context = { params: Promise<{ id: string; invitationId: string }> };
export async function POST(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return inviteMember(
      await currentActor(request),
      p.id,
      await jsonBody(request),
      p.invitationId,
    );
  });
}
export async function DELETE(request: Request, ctx: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const p = await ctx.params;
    return revokeInvitation(
      await currentActor(request),
      p.id,
      p.invitationId,
      await jsonBody(request),
    );
  });
}
