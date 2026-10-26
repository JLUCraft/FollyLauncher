import { type WindowConfig } from "@/models/misc";
import { UtilsService } from "@/services/utils";

/**
 * Height of the custom window titlebar in px, matching the native Windows 11 /
 * macOS 15 titlebar. The titlebar renders at this height and publishes it as
 * the `--window-titlebar-height` CSS variable so floating layers stacked from
 * the window top (toasts) can stay below it.
 */
export const WINDOW_TITLEBAR_HEIGHT = 28;
export const WINDOW_TITLEBAR_HEIGHT_WINDOWS = 32;

export const windowTitlebarHeight = (osType: string): number =>
  osType === "windows"
    ? WINDOW_TITLEBAR_HEIGHT_WINDOWS
    : WINDOW_TITLEBAR_HEIGHT;

export const createWindow = (
  label?: string, // only allow to contain a-zA-Z plus "-", "/", ":", "_" (space will convert to "_")
  route?: string,
  options?: Pick<WindowConfig, "title" | "minWidth" | "minHeight">
) => {
  // use current timestamp as the unique label if none is provided
  let windowLabel = label || `${Date.now()}`;
  windowLabel = windowLabel.replaceAll(" ", "_");

  (async () => {
    const response = await UtilsService.createWindow(
      {
        label: windowLabel,
        url: route || "/",
        title: options?.title,
        height: 550,
        minWidth: options?.minWidth,
        minHeight: options?.minHeight,
      },
      true
    );
    if (response.status === "success") {
      logger.info(`Child window ${windowLabel} successfully created`);
    }
  })();
};

export const parseIdFromWindowLabel = (label: string): number => {
  const match = label.match(/(game_error|game_log)_(\d+)/);
  if (match) {
    return parseInt(match[2], 10);
  }
  return 0; // or throw an error if preferred
};
