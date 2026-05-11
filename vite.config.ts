import type { UserConfig } from "vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";

const host =
  (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env
    ?.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig((): UserConfig => {
  const config: UserConfig = {
    plugins: [solid(), tailwindcss()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host ?? false,
      watch: {
        // 3. tell Vite to ignore watching `src-tauri`
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
