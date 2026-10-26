import {
  Badge,
  Button,
  Grid,
  GridItem,
  HStack,
  Icon,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useRouter } from "next/router";
import React from "react";
import { useTranslation } from "react-i18next";
import { IconType } from "react-icons";
import {
  LuCircleHelp,
  LuCloudDownload,
  LuCoffee,
  LuGamepad2,
  LuGrid2X2Plus,
  LuInfo,
  LuNetwork,
  LuPalette,
  LuSettings,
  LuShieldCheck,
  LuSparkles,
} from "react-icons/lu";
import NavMenu from "@/components/common/nav-menu";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import {
  FEDERATION_MANAGEMENT_SETTINGS_PATH,
  FEDERATION_ROOMS_PATH,
  FEDERATION_SETTINGS_PATH,
  federationReturnPath,
} from "@/utils/federation/navigation";

interface SettingsLayoutProps {
  children: React.ReactNode;
}

const SettingsLayout: React.FC<SettingsLayoutProps> = ({ children }) => {
  const router = useRouter();
  const { t } = useTranslation();
  const { config, newerVersion } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const { isAdmin } = useFederation();
  const returnTo = federationReturnPath(router.query.returnTo);
  const isAllianceSetting = [
    FEDERATION_SETTINGS_PATH,
    FEDERATION_MANAGEMENT_SETTINGS_PATH,
  ].includes(router.pathname);

  const settingsDomainList: { key: string; icon: IconType }[][] = [
    [
      { key: "global-game", icon: LuGamepad2 },
      { key: "java", icon: LuCoffee },
    ],
    [
      { key: "federation", icon: LuNetwork },
      ...(isAdmin
        ? [{ key: "federation-management", icon: LuShieldCheck }]
        : []),
    ],
    [
      { key: "general", icon: LuSettings },
      { key: "appearance", icon: LuPalette },
      { key: "download", icon: LuCloudDownload },
      { key: "intelligence", icon: LuSparkles },
      { key: "extension", icon: LuGrid2X2Plus },
      { key: "help", icon: LuCircleHelp },
      { key: "about", icon: LuInfo },
    ],
  ];

  const settingsPaths = settingsDomainList
    .flat()
    .map((item) => `/settings/${item.key}`);
  const selectedKey =
    settingsPaths.find(
      (path) =>
        router.pathname === path || router.pathname.startsWith(`${path}/`)
    ) || "";

  return (
    <Grid templateColumns="1fr 3fr" gap={4} h="100%">
      <GridItem className="content-full-y">
        <VStack align="stretch" spacing={4}>
          {settingsDomainList.map((group) => (
            <NavMenu
              key={group[0].key}
              selectedKeys={[selectedKey]}
              onClick={(value) => {
                const alliancePath = [
                  FEDERATION_SETTINGS_PATH,
                  FEDERATION_MANAGEMENT_SETTINGS_PATH,
                ].includes(value);
                void router.push(
                  alliancePath
                    ? { pathname: value, query: { returnTo } }
                    : value
                );
              }}
              items={group.map((item) => ({
                label: (
                  <HStack spacing={2} overflow="hidden" w="100%">
                    <Icon as={item.icon} />
                    <Text fontSize="sm" className="ellipsis-text">
                      {t(`SettingsLayout.settingsDomainList.${item.key}`)}
                    </Text>
                    {item.key === "about" && newerVersion.version && (
                      <Badge colorScheme={primaryColor} ml="auto">
                        {t("General.new")}
                      </Badge>
                    )}
                  </HStack>
                ),
                value: `/settings/${item.key}`,
                tooltip: t(`SettingsLayout.settingsDomainList.${item.key}`),
              }))}
            />
          ))}
        </VStack>
      </GridItem>
      <GridItem className="content-full-y" key={router.asPath}>
        <VStack align="stretch" spacing={4}>
          {isAllianceSetting && (
            <Button
              as={Link}
              href={returnTo}
              alignSelf="flex-start"
              variant="outline"
            >
              {t(
                returnTo.split(/[?#]/)[0] === FEDERATION_ROOMS_PATH
                  ? "ConnectionFlow.returnRooms"
                  : "ConnectionFlow.returnActivities"
              )}
            </Button>
          )}
          {children}
        </VStack>
      </GridItem>
    </Grid>
  );
};

export default SettingsLayout;
