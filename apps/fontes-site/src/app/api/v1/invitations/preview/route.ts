import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { previewInvitation } from "@/lib/organizations";
export async function POST(request: Request) {
  return api(async () => {
    assertSameOrigin(request);
    return previewInvitation(
      await currentActor(request),
      await jsonBody(request),
    );
  });
}
