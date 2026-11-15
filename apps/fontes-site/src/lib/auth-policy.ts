import { DomainError } from "./contracts";

export function resolveAuthMode(
  env: Record<string, string | undefined>,
  hostname?: string,
): "development" | "neon" {
  const mode = env.FONTES_AUTH_MODE ?? "neon";
  if (mode !== "development" && mode !== "neon")
    throw new DomainError(
      503,
      "auth_configuration",
      "Unknown authentication mode.",
    );
  if (mode === "development") {
    const localRuntime =
      env.NODE_ENV === "development" || env.NODE_ENV === "test";
    const deployed = !!(
      env.VERCEL ||
      env.CF_PAGES ||
      env.AWS_LAMBDA_FUNCTION_NAME ||
      env.RENDER ||
      env.FLY_APP_NAME ||
      env.RAILWAY_ENVIRONMENT
    );
    const loopback =
      hostname === undefined ||
      ["localhost", "127.0.0.1", "[::1]"].includes(hostname);
    if (!localRuntime || deployed || !loopback)
      throw new DomainError(
        503,
        "development_auth_forbidden",
        "Development authentication is available only in a local development runtime.",
      );
  }
  return mode;
}
