import React, {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
} from "react";
import {
  sharedModals,
  type OpenSharedModal,
  type SharedModalParams,
} from "@/components/modals/registry";
import { useLauncherConfig } from "@/contexts/config";

interface SharedModalContextType {
  openSharedModal: OpenSharedModal;
  closeSharedModal: (key: string, modalId?: number) => void;
  openGenericConfirmDialog: (
    params: SharedModalParams<"generic-confirm">
  ) => void;
  modalStates: {
    key: string;
    modalId: number;
    element: React.ReactElement;
    onCloseCallback?: () => void;
  }[];
}

export const SharedModalContext = createContext<
  SharedModalContextType | undefined
>(undefined);

export const SharedModalContextProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const [modalStates, setModalStates] = useState<
    SharedModalContextType["modalStates"]
  >([]);
  const nextModalId = useRef(0);
  const { config } = useLauncherConfig();

  const closeSharedModal = useCallback((key: string, modalId?: number) => {
    setModalStates((prev) =>
      prev.filter(
        (modal) =>
          modal.key !== key ||
          (modalId !== undefined && modal.modalId !== modalId)
      )
    );
  }, []);

  const openSharedModal = useCallback<OpenSharedModal>(
    (...args) => {
      const [key, params, options] = args;
      const nextId = nextModalId.current++;
      const { onCloseCallback, ...modalParams } = params ?? {};
      type Props = SharedModalParams<typeof key> & {
        isOpen: boolean;
        onClose: () => void;
      };
      const Modal = sharedModals[key] as React.ComponentType<Props>;
      setModalStates((prev) => {
        const index = prev.map((modal) => modal.key).lastIndexOf(key);
        const replace = !options?.stack && index !== -1;
        const modalId = replace ? prev[index].modalId : nextId;
        const props = {
          ...modalParams,
          isOpen: true,
          onClose: () => {
            closeSharedModal(key, modalId);
            onCloseCallback?.();
          },
        } as Props;
        const modal = {
          key,
          modalId,
          onCloseCallback,
          element: React.createElement(Modal, { ...props, key: modalId }),
        };
        return replace
          ? prev.map((previous, i) => (i === index ? modal : previous))
          : [...prev, modal];
      });
      logger
        .info("Opened shared modal:", key, params)
        .catch((error: unknown) => console.error(error));
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
