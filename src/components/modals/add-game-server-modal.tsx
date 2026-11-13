import {
  Button,
  FormControl,
  FormErrorMessage,
  FormLabel,
  Input,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  VStack,
} from "@chakra-ui/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLauncherConfig } from "@/contexts/config";
import { useToast } from "@/contexts/toast";
import { useKeyedState } from "@/hooks/keyed-state";
import { InstanceService } from "@/services/instance";

interface AddGameServerModalProps extends Omit<ModalProps, "children"> {
  presetAddr?: string;
  instanceId: string;
}

const AddGameServerModal: React.FC<AddGameServerModalProps> = ({
  presetAddr = "",
  instanceId,
  ...modalProps
}) => {
  const { t } = useTranslation();
  const toast = useToast();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { isOpen, onClose } = modalProps;
  const initialRef = useRef(null);
  const hasAutoPresetRef = useRef(false);

  const [serverAddr, setServerAddr] = useKeyedState<string>(
    presetAddr,
    [isOpen, presetAddr],
    isOpen
  );
  const [serverName, setServerName] = useKeyedState<string>(
    "",
    [isOpen, presetAddr],
    isOpen
  );
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const [isServerAddrTouched, setIsServerAddrTouched] = useKeyedState(
    false,
    [isOpen, presetAddr],
    isOpen
  );
  const trimmedServerAddr = serverAddr.trim();
  const isServerAddrInvalid = isServerAddrTouched && !trimmedServerAddr;
  const serverNamePlaceholder = t("AddGameServerModal.placeholder.serverName");

  useEffect(() => {
    if (isOpen) {
      hasAutoPresetRef.current = false;
    }
  }, [isOpen, presetAddr]);

  const handleAddGameServer = useCallback(
    (instanceId: string) => {
      const finalServerName = serverName.trim() || serverNamePlaceholder;
      setIsLoading(true);
      InstanceService.addGameServer(
        instanceId,
        trimmedServerAddr,
        finalServerName
      )
        .then((response) => {
          if (response.status === "success") {
            toast({
              title: response.message,
              status: "success",
            });
            onClose?.();
          } else {
            toast({
              title: response.message,
              description: response.details,
              status: "error",
            });
          }
        })
        .finally(() => {
          setIsLoading(false);
        })
        .catch((error: unknown) => console.error(error));
    },
    [trimmedServerAddr, serverName, serverNamePlaceholder, toast, onClose]
  );

  return (
    <Modal
      size={{ base: "md", lg: "lg", xl: "xl" }}
      initialFocusRef={initialRef}
      returnFocusOnClose={false}
      {...modalProps}
    >
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>{t("AddGameServerModal.header.title")}</ModalHeader>
        <ModalCloseButton />
        <ModalBody>
          <VStack spacing={4} align="stretch">
            <FormControl isInvalid={isServerAddrInvalid} isRequired>
              <FormLabel htmlFor="serverAddr">
                {t("AddGameServerModal.label.serverAddr")}
              </FormLabel>
              <Input
                id="serverAddr"
                type="url"
                placeholder={t("AddGameServerModal.placeholder.serverAddr")}
                value={serverAddr}
                onChange={(e) => setServerAddr(e.target.value)}
                onBlur={() => setIsServerAddrTouched(true)}
                ref={initialRef}
                focusBorderColor={`${primaryColor}.500`}
              />
              {isServerAddrInvalid && (
                <FormErrorMessage>
                  {t("AddGameServerModal.serverAddrRequired")}
                </FormErrorMessage>
              )}
            </FormControl>

            <FormControl>
              <FormLabel htmlFor="serverName">
                {t("AddGameServerModal.label.serverName")}
              </FormLabel>
              <Input
                id="serverName"
                type="text"
                placeholder={serverNamePlaceholder}
                value={serverName}
                onChange={(e) => setServerName(e.target.value)}
                focusBorderColor={`${primaryColor}.500`}
              />
            </FormControl>
          </VStack>
        </ModalBody>

        <ModalFooter>
          <Button variant="ghost" onClick={onClose}>
            {t("General.cancel")}
          </Button>
          <Button
            colorScheme={primaryColor}
            onClick={() => {
              if (instanceId) {
                handleAddGameServer(instanceId);
              }
            }}
            isLoading={isLoading}
            isDisabled={!trimmedServerAddr}
          >
            {t("General.confirm")}
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
};

export default AddGameServerModal;
