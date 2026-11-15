import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { transitionResource } from "@/lib/repository";
export async function POST(
  request: Request,
  context: { params: Promise<{ id: string }> },
) {
  return api(async () => {
    assertSameOrigin(request);
    const { id } = await context.params;
    return transitionResource(
      await currentActor(request),
      id,
      await jsonBody(request),
    );
  });
}
