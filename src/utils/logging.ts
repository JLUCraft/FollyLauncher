import { debug, error, info, trace, warn } from "@tauri-apps/plugin-log";
import { displayValue } from "@/utils/string";

function formatArg(arg: unknown): string {
  if (arg instanceof Error) return `${arg.name}: ${arg.message}\n${arg.stack}`;
  return displayValue(arg);
}

export const logger = {
  info: async (...args: unknown[]) => info(args.map(formatArg).join(" ")),
  warn: async (...args: unknown[]) => warn(args.map(formatArg).join(" ")),
  error: async (...args: unknown[]) => error(args.map(formatArg).join(" ")),
  debug: async (...args: unknown[]) => debug(args.map(formatArg).join(" ")),
  trace: async (...args: unknown[]) => trace(args.map(formatArg).join(" ")),
};

export function setupLogger() {
  if (typeof window !== "undefined" && !window.logger) {
    window.logger = logger;
  }
}
