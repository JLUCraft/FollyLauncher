import { config } from "../federation/mock-ipc";

const mode = new URL(location.href).searchParams.get("test-color-mode");
if (mode === "light" || mode === "dark")
  config.appearance.theme.colorMode = mode;
