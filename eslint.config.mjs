import nextCoreWebVitals from "eslint-config-next/core-web-vitals";
import unusedImports from "eslint-plugin-unused-imports";

const config = [
  {
    ignores: ["src-tauri/**", "tools/**"],
  },
  ...nextCoreWebVitals,
  {
    plugins: {
      "unused-imports": unusedImports,
    },
    rules: {
      "unused-imports/no-unused-imports": "warn",
    },
  },
];

export default config;
