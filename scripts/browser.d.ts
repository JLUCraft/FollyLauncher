import type { NextRouter } from "next/router";
import type { LauncherConfig } from "@/models/config";
import type { BrowserFixtureState } from "./federation/mock-ipc";

declare global {
  interface Window extends BrowserFixtureState {
    next: { router: NextRouter };
    testConfig: LauncherConfig;
    savedIdentity?: unknown;
  }
}
