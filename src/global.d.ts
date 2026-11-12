export {};

declare global {
  interface Window {
    logger: typeof import("@/utils/logging").logger;
  }

  const logger: Window["logger"];
}
