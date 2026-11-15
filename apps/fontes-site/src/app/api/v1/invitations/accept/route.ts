import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { acceptInvitation } from "@/lib/organizations";
export async function POST(request: Request) {
  return api(async () => {
    assertSameOrigin(request);
    return acceptInvitation(
      await currentActor(request),
      await jsonBody(request),
    );
  });
}
