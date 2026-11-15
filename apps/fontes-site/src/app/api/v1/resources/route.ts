import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { createResource, workspace } from "@/lib/repository";
export async function GET(request: Request) {
  return api(async () => ({
    items: (await workspace(await currentActor(request))).resources,
    limit: 100,
  }));
}
export async function POST(request: Request) {
  return api(async () => {
    assertSameOrigin(request);
    return createResource(await currentActor(request), await jsonBody(request));
  }, 201);
}
