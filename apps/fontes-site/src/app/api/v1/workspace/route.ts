import { currentActor } from "@/lib/auth";
import { api } from "@/lib/http";
import { workspace } from "@/lib/repository";
export const dynamic = "force-dynamic";
export async function GET(request: Request) {
  return api(async () => workspace(await currentActor(request)));
}
