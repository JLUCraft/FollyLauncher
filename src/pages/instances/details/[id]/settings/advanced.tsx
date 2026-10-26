import { Badge, HStack, Text } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import GameAdvancedSettingsGroups from "@/components/game-advanced-settings-group";
import { useInstanceSharedData } from "@/contexts/instance";

const InstanceGameAdvancedSettingsPage = () => {
  const { t } = useTranslation();
  const {
    summary,
    gameConfig: instanceGameConfig,
    handleUpdateInstanceConfig,
  } = useInstanceSharedData();
  const useSpecGameConfig = summary?.useSpecGameConfig || false;

  return (
    <>
      {/* Scope banner: these advanced options apply to one instance only, and
          only while per-instance game settings are enabled. */}
      {summary && (
        <AdvancedCard level="front" p={3}>
          <HStack spacing={2} flexWrap="wrap">
            <Badge colorScheme={useSpecGameConfig ? "green" : "gray"}>
              {t(
                useSpecGameConfig
                  ? "InstanceSettingsPage.advancedScope.enabled"
                  : "InstanceSettingsPage.advancedScope.disabled"
              )}
            </Badge>
            <Text fontSize="sm" className="secondary-text">
              {t("InstanceSettingsPage.advancedScope.instance", {
                name: summary.name,
              })}
            </Text>
          </HStack>
        </AdvancedCard>
      )}
      {instanceGameConfig && (
        <GameAdvancedSettingsGroups
          gameConfig={instanceGameConfig}
          updateGameConfig={(key: string, value: any) => {
            handleUpdateInstanceConfig(`specGameConfig.${key}`, value);
          }}
        />
      )}
    </>
  );
};

export default InstanceGameAdvancedSettingsPage;
