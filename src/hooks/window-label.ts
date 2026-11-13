import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSyncExternalStore } from "react";

// A native window keeps its label throughout its lifetime.
const subscribe = () => () => {};
const getSnapshot = () => getCurrentWindow().label;
const getServerSnapshot = () => undefined;

export function useWindowLabel(): string | undefined {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}
