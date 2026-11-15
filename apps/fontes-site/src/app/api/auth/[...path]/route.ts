import { managedAuth } from "@/lib/neon-auth";
type Context = { params: Promise<{ path: string[] }> };
export async function GET(request: Request, context: Context) {
  return managedAuth().handler().GET(request, context);
}
export async function POST(request: Request, context: Context) {
  return managedAuth().handler().POST(request, context);
}
