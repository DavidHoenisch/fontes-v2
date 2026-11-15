import type { NextConfig } from "next";
import path from "node:path";
const config: NextConfig = {
  agentRules: false,
  poweredByHeader: false,
  turbopack: { root: path.resolve(process.cwd(), "../..") },
  serverExternalPackages: ["@neondatabase/serverless"],
  async headers() {
    return [
      {
        source: "/api/:path*",
        headers: [
          { key: "Cache-Control", value: "private, no-store" },
          { key: "X-Content-Type-Options", value: "nosniff" },
        ],
      },
    ];
  },
};
export default config;
