import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { useEffect, useRef } from "react";
import { brand } from "@/brand";

const BRAND_LINK_PREFIX = `${brand.deeplinkScheme}://`;
const EMIT_DEEPLINK_EVENT = "deeplink:emit";

type TriggerRule = RegExp | ((subpath: string) => boolean);

interface UseDeepLinkOptions {
  trigger: TriggerRule;
  onCall: (path: string, subpath: string) => void;
}

// Do not use openUrl so this helper can be used during development.
export const emitDeepLink = (urls: string[]) => {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent<string[]>(EMIT_DEEPLINK_EVENT, {
      detail: urls,
    })
  );
};

export const useDeepLink = ({ trigger, onCall }: UseDeepLinkOptions) => {
  const options = useRef({ trigger, onCall });
  options.current = { trigger, onCall };
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const handleUrls = (urls: string[]) => {
      if (!active) return;
      urls.forEach((url) => {
        if (!url.startsWith(BRAND_LINK_PREFIX)) return;
        const subpath = url.slice(BRAND_LINK_PREFIX.length);
        const { trigger, onCall } = options.current;
        if (
          typeof trigger === "function"
            ? trigger(subpath)
            : trigger.test(subpath)
        )
          onCall(url, subpath);
      });
    };
    const handleDevUrls = (event: Event) =>
      handleUrls((event as CustomEvent<string[]>).detail || []);
    window.addEventListener(EMIT_DEEPLINK_EVENT, handleDevUrls);
    void (async () => {
      try {
        const cleanup = await onOpenUrl(handleUrls);
        if (!active) {
          cleanup();
          return;
        }
        unlisten = cleanup;
      } catch (err) {
        logger.error("Failed to listen to deep links:", err);
      }
      try {
        const urls = await getCurrent();
        if (urls) handleUrls(urls);
      } catch (err) {
        logger.error("getCurrent failed:", err);
      }
    })();
    return () => {
      active = false;
      window.removeEventListener(EMIT_DEEPLINK_EVENT, handleDevUrls);
      unlisten?.();
    };
  }, []);
};
export default useDeepLink;
