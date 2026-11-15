import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { intake, saveIntake } from "@/lib/intake-service";
type Context = { params: Promise<{ id: string }> };
export async function GET(request: Request, context: Context) {
  return api(async () =>
    intake(await currentActor(request), (await context.params).id),
  );
}
export async function PUT(request: Request, context: Context) {
  return api(async () => {
    assertSameOrigin(request);
    return saveIntake(
      await currentActor(request),
      (await context.params).id,
      await jsonBody(request, 2 * 1024 * 1024),
    );
  });
}
