import { currentActor } from "@/lib/auth";
import { api, assertSameOrigin, jsonBody } from "@/lib/http";
import { listSourceReviews, submitSourceReview } from "@/lib/source-review";
export async function GET(request: Request) {
  return api(async () =>
    listSourceReviews(
      await currentActor(request),
      new URL(request.url).searchParams.get("history") === "true",
    ),
  );
}
export async function POST(request: Request) {
  return api(async () => {
    assertSameOrigin(request);
    return submitSourceReview(
      await currentActor(request),
      await jsonBody(request),
    );
  }, 201);
}
