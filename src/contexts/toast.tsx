import {
  Center,
  ToastId,
  UseToastOptions,
  useToast as chakraUseToast,
  useColorModeValue,
} from "@chakra-ui/react";
import React, {
  ReactNode,
  createContext,
  useCallback,
  useContext,
} from "react";
import { BeatLoader } from "react-spinners";

interface ToastContextProviderProps {
  children: ReactNode;
}

interface ToastContextType {
  (options: UseToastOptions): ToastId;
}

const ToastContext = createContext<ToastContextType | null>(null);

// Same-kind errors (e.g. one failed request retried by several components)
// collapse into a single toast instead of stacking up the same message.
const getToastDedupeKey = (options: UseToastOptions): string | undefined => {
  if (options.status !== "error") return undefined;
  return `toast-error:${String(options.title ?? "")}:${String(
    options.description ?? ""
  )}`;
};

export const ToastContextProvider: React.FC<ToastContextProviderProps> = ({
  children,
}) => {
  const chakraToast = chakraUseToast();
  const toastVariant = useColorModeValue("left-accent", "solid");

  const customToast: ToastContextType = useCallback(
    (options) => {
      const dedupeKey = getToastDedupeKey(options);
      const toastOptions: UseToastOptions = {
        position: "top-right",
        duration: options.status === "loading" ? null : 3000,
        icon:
          options.status === "loading" ? (
            <Center h="100%" mt={0.5}>
              <BeatLoader size={4} />
            </Center>
          ) : null,
        variant: toastVariant,
        isClosable: true,
        // Stack below the custom window titlebar: a toast pinned to the window
        // top sits underneath the draggable titlebar layer. Chakra applies
        // `containerStyle` to the static toast inner box and hardcodes the list
        // container's `top`, so the offset has to be a margin (a `top` here is
        // ignored), and the +1px clears the window's 1px chrome edge.
        containerStyle: {
          marginTop: "calc(var(--window-titlebar-height, 28px) + 1px)",
          minWidth: "2xs",
          userSelect: "none",
        },
        ...options,
      };
      // Chakra appends every notify call to the container even when an id is
      // passed, so refresh the live toast instead of stacking a duplicate.
      if (dedupeKey && chakraToast.isActive(dedupeKey)) {
        chakraToast.update(dedupeKey, toastOptions);
        return dedupeKey;
      }
      return chakraToast({ ...toastOptions, id: dedupeKey });
    },
    [chakraToast, toastVariant]
  );

  return (
    <ToastContext.Provider value={customToast}>
      {children}
    </ToastContext.Provider>
  );
};

export const useToast = (): ToastContextType => {
  const context = useContext(ToastContext);
  if (!context)
    throw new Error("useToast must be used within a ToastContextProvider");
  return context;
};
