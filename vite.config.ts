import type { UserConfig } from "vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";

const host =
  (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env
    ?.TAURI_DEV_HOST;

export default defineConfig((): UserConfig => {
  const config: UserConfig = {
    plugins: [solid(), tailwindcss()],

    clearScreen: false,

    server: {
      port: 1420,
      strictPort: true,
      host: host ?? false,
      watch: {

        ignored: ["**/src-tauri/**"],
      },
    },
  };

  if (host && config.server) {
    config.server.hmr = {
      protocol: "ws",
      host,
      port: 1421,
    };
  }

  return config;
});