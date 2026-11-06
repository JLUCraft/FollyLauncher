import {
  Alert,
  AlertIcon,
  Button,
  Checkbox,
  HStack,
  Image,
  Link,
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  Text,
} from "@chakra-ui/react";
import { exit } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { LuLanguages } from "react-icons/lu";
import LanguageMenu from "@/components/language-menu";
import { useGuidedTour } from "@/components/special/guided-tour-provider";
import { useLauncherConfig } from "@/contexts/config";
import { BuildType } from "@/enums/misc";
import { UnionService } from "@/services/union";

const WelcomeAndTermsModal: React.FC<Omit<ModalProps, "children">> = ({
  ...props
}) => {
  const { t } = useTranslation();
  const { config, update } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { startGuidedTour } = useGuidedTour();
  const [relayEnabled, setRelayEnabled] = useState(
    config.federation.relayEnabled
  );
  const [submitting, setSubmitting] = useState(false);
  const [relayFailed, setRelayFailed] = useState(false);

  const handleAgree = async () => {
    setSubmitting(true);
    setRelayFailed(false);
    try {
      await UnionService.setRelayEnabled(relayEnabled);
      update("runCount", config.runCount + 1);
      props.onClose();
      startGuidedTour();
    } catch {
      setRelayFailed(true);
      setSubmitting(false);
    }
  };

  // For non-release builds, use the build type as the version label for the warning alert.
  // release builds get no warning; the rest reuse General.version.<label>.
  const matchedUnstableVersionLabel =
    config.basicInfo.buildType !== BuildType.Release
      ? config.basicInfo.buildType
      : config.basicInfo.launcherVersion.startsWith("0.") ||
          config.basicInfo.launcherVersion.includes("beta")
        ? BuildType.Beta
        : undefined;

  const isWinArm64 =
    config.basicInfo.platform === "windows" &&
    config.basicInfo.arch === "aarch64";

  return (
    <Modal
      autoFocus={false}
      closeOnEsc={false}
      closeOnOverlayClick={false}
      size={{ base: "sm", lg: "md" }}
      returnFocusOnClose={false}
      {...props}
    >
      <ModalOverlay />
      <ModalContent borderRadius="md" overflow="hidden">
        <Image alt="banner" src="/images/banner.png" />
        <ModalHeader>
          🎉&nbsp;&nbsp;{t("WelcomeAndTermsModal.header.title")}
        </ModalHeader>
        <ModalBody mt={-1}>
          <Text color="gray.500">
            <Trans
              i18nKey="WelcomeAndTermsModal.body.text"
              components={{
                terms: (
                  <Link
                    color={`${primaryColor}.500`}
                    onClick={() => {
                      openUrl(
                        t(
                          "AboutSettingsPage.legalInfo.settings.userAgreement.url"
                        )
                      );
                    }}
                  />
                ),
              }}
            />
          </Text>
          <Checkbox
            mt={4}
            colorScheme={primaryColor}
            isChecked={relayEnabled}
            isDisabled={submitting}
            onChange={(event) => setRelayEnabled(event.target.checked)}
          >
            {t("FederationPage.settings.relay.title")}
          </Checkbox>
          <Text mt={1} fontSize="sm" className="secondary-text">
            {t("FederationPage.settings.relay.description")}
          </Text>
          {relayFailed && (
            <Alert status="error" mt={3} fontSize="xs-sm" borderRadius="md">
              <AlertIcon />
              {t("FederationPage.settings.relay.updateFailed")}
            </Alert>
          )}
          {matchedUnstableVersionLabel && (
            <Alert status="warning" mt={3} fontSize="xs-sm" borderRadius="md">
              <AlertIcon />
              {t("WelcomeAndTermsModal.warning.unstableVersion", {
                versionLabel: t(
                  `General.version.${matchedUnstableVersionLabel}`
                ),
              })}
            </Alert>
          )}
          {isWinArm64 && (
            <Alert status="info" mt={3} fontSize="xs-sm" borderRadius="md">
              <AlertIcon />
              <Text>
                <Trans
                  i18nKey="WelcomeAndTermsModal.warning.winArm64Notice"
                  components={{
                    b: <b />,
                    glpack: (
                      <Link
                        color={`${primaryColor}.500`}
                        onClick={() => {
                          openUrl(
                            "ms-windows-store://pdp/?productid=9NQPSL29BFFF"
                          );
                        }}
                      />
                    ),
                  }}
                />
              </Text>
            </Alert>
          )}
        </ModalBody>
        <ModalFooter w="100%">
          <HStack spacing={2}>
            <LuLanguages />
            <LanguageMenu placement="top" />
          </HStack>
          <HStack spacing={3} ml="auto">
            <Button
              variant="ghost"
              isDisabled={submitting}
              onClick={() => exit(0)}
            >
              {t("General.exit")}
            </Button>
            <Button
              colorScheme={primaryColor}
              isLoading={submitting}
              onClick={handleAgree}
            >
              {t("WelcomeAndTermsModal.button.agree")}
            </Button>
          </HStack>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
};

export default WelcomeAndTermsModal;
