import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { getResource, updateResource } from "@/lib/repository";
type Context = { params: Promise<{ id: string }> };
export async function GET(request: Request, context: Context) {
  return api(async () => {
    const { id } = await context.params;
    return getResource(await currentActor(request), id);
  });
}
export async function PATCH(request: Request, context: Context) {
  return api(async () => {
    assertSameOrigin(request);
    const { id } = await context.params;
    return updateResource(
      await currentActor(request),
      id,
      await jsonBody(request),
    );
  });
}
