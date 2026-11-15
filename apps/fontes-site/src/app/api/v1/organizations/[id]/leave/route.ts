import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { changeMember } from "@/lib/organizations";
export async function POST(
  request: Request,
  ctx: { params: Promise<{ id: string }> },
) {
  return api(async () => {
    assertSameOrigin(request);
    const actor = await currentActor(request);
    return changeMember(
      actor,
      (await ctx.params).id,
      actor.id,
      "leave",
      await jsonBody(request),
    );
  });
}
