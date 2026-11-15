import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { inviteMember } from "@/lib/organizations";
export async function POST(
  request: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  return api(async () => {
    assertSameOrigin(request);
    return inviteMember(
      await currentActor(request),
      (await ctx.params).id,
      await jsonBody(request),
    );
  }, 201);
}
