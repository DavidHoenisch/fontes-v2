import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { listOrganizations, createOrganization } from "@/lib/organizations";
export async function GET(request: Request) {
  return api(async () => listOrganizations(await currentActor(request)));
}
export async function POST(request: Request) {
  return api(async () => {
    assertSameOrigin(request);
    return createOrganization(
      await currentActor(request),
      await jsonBody(request),
    );
  }, 201);
}
