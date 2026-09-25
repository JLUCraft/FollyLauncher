const isProd = process.env.NODE_ENV === "production";

const internalHost = process.env.TAURI_DEV_HOST || "localhost";

/** @type {import('next').NextConfig} */
const nextConfig = {
  // Ensure Next.js uses SSG instead of SSR
  // https://nextjs.org/docs/pages/building-your-application/deploying/static-exports
  output: "export",
  // Note: This feature is required to use the Next.js Image component in SSG mode.
  // See https://nextjs.org/docs/messages/export-image-api for different workarounds.
  images: {
    unoptimized: true,
  },
  // Configure assetPrefix or else the server won't properly resolve your assets.
  assetPrefix: isProd ? undefined : `http://${internalHost}:3000`,
  // Keep the developer-only test page out of the production static export
  // shipped with Tauri; it remains available in `next dev`.
  exportPathMap: async (
    defaultPathMap: Record<string, { page: string }>,
    { dev }: { dev: boolean }
  ): Promise<Record<string, { page: string }>> => {
    if (dev) return defaultPathMap;
    return Object.fromEntries(
      Object.entries(defaultPathMap).filter(([path]) => path !== "/dev-test")
    );
  },
  devIndicators: false,
};

// Now can run `ANALYZE=true bun run build` to analyze frontend bundle size
const withBundleAnalyzer = require("@next/bundle-analyzer")({
  enabled: process.env.ANALYZE === "true",
});

export default withBundleAnalyzer(nextConfig);
