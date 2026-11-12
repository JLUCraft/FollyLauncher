import React, { createContext, useCallback, useContext, useState } from "react";
import {
  sharedModals,
  type OpenSharedModal,
  type SharedModalParams,
} from "@/components/modals/registry";
import { useLauncherConfig } from "@/contexts/config";

interface SharedModalContextType {
  openSharedModal: OpenSharedModal;
  closeSharedModal: (key: string) => void;
  openGenericConfirmDialog: (
    params: SharedModalParams<"generic-confirm">
  ) => void;
  modalStates: Record<string, React.ReactElement>;
}

export const SharedModalContext = createContext<
  SharedModalContextType | undefined
>(undefined);

export const SharedModalContextProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const [modalStates, setModalStates] = useState<
    Record<string, React.ReactElement>
  >({});
  const { config } = useLauncherConfig();

  const closeSharedModal = useCallback((key: string) => {
    setModalStates((prev) => {
      const { [key]: _, ...newStates } = prev;
      return newStates;
    });
  }, []);

  const openSharedModal = useCallback<OpenSharedModal>(
    (...args) => {
      const [key, params] = args;
      type Props = SharedModalParams<typeof key> & {
        isOpen: boolean;
        onClose: () => void;
      };
      const Modal = sharedModals[key] as React.ComponentType<Props>;
      const props = {
        ...params,
        isOpen: true,
        onClose: () => closeSharedModal(key),
      } as Props;
      setModalStates((prev) => ({
        ...prev,
        [key]: React.createElement(Modal, { ...props, key }),
      }));
      logger.info("Opened shared modal:", key, params);
    },
    [closeSharedModal]
  );

  const openGenericConfirmDialog = useCallback(
    (params: SharedModalParams<"generic-confirm">) => {
      // If the user has previously selected "Don't show again", skip the dialog and call the OK callback directly
      if (
        params.suppressKey &&
        config.suppressedDialogs?.includes(params.suppressKey)
      ) {
        params?.onOKCallback?.();
        return;
      }
      openSharedModal("generic-confirm", {
        ...params,
      });
    },
    [config.suppressedDialogs, openSharedModal]
  );

  return (
    <SharedModalContext.Provider
      value={{
        openSharedModal,
        closeSharedModal,
        openGenericConfirmDialog,
        modalStates,
      }}
    >
      {children}
    </SharedModalContext.Provider>
  );
};

export const useSharedModals = (): SharedModalContextType => {
  const context = useContext(SharedModalContext);
  if (!context) {
    throw new Error(
      "useSharedModals must be used within a SharedModalContextProvider"
    );
  }
  return context;
};
