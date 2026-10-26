import { Grid, GridItem, HStack, Icon, Text, VStack } from "@chakra-ui/react";
import { useRouter } from "next/router";
import { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { LuGamepad2, LuShieldCheck, LuTrophy, LuUsers } from "react-icons/lu";
import NavMenu from "@/components/common/nav-menu";
import { useFederation } from "@/contexts/federation";
import {
  FEDERATION_ACTIVITIES_PATH,
  FEDERATION_MANAGEMENT_SETTINGS_PATH,
  FEDERATION_ROOMS_PATH,
  FEDERATION_SETTINGS_PATH,
} from "@/utils/federation/navigation";

function Navigation({ children }: { children: ReactNode }) {
  const router = useRouter();
  const { t } = useTranslation();
  const { isAdmin } = useFederation();
  const currentPath = router.asPath.split(/[?#]/)[0];
  const items = [
    { key: "rooms", icon: LuGamepad2, path: FEDERATION_ROOMS_PATH },
    { key: "league", icon: LuTrophy, path: FEDERATION_ACTIVITIES_PATH },
    { key: "identity", icon: LuUsers, path: FEDERATION_SETTINGS_PATH },
    ...(isAdmin
      ? [
          {
            key: "manage",
            icon: LuShieldCheck,
            path: FEDERATION_MANAGEMENT_SETTINGS_PATH,
          },
        ]
      : []),
  ];
  return (
    <Grid templateColumns="1fr 3fr" gap={4} h="100%">
      <GridItem className="content-full-y">
        <NavMenu
          selectedKeys={[currentPath]}
          onClick={(value) => void router.push(value)}
          items={items.map((item) => ({
            value: item.path,
            label: (
              <HStack spacing={2} overflow="hidden" w="100%">
                <Icon as={item.icon} />
                <Text fontSize="sm" className="ellipsis-text">
                  {t(`FederationNav.${item.key}`)}
                </Text>
              </HStack>
            ),
            tooltip: t(`FederationNav.${item.key}`),
          }))}
        />
      </GridItem>
      <GridItem key={router.asPath} className="content-full-y" minW={0}>
        <VStack align="stretch" spacing={4}>
          {children}
        </VStack>
      </GridItem>
    </Grid>
  );
}
export default function FederationLayout({
  children,
}: {
  children: ReactNode;
}) {
  return <Navigation>{children}</Navigation>;
}
