import { Grid, GridItem, HStack, Icon, Text, VStack } from "@chakra-ui/react";
import { useRouter } from "next/router";
import { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  LuCalendar,
  LuClipboardCheck,
  LuFolder,
  LuGamepad2,
  LuHouse,
  LuSettings,
  LuTrophy,
  LuUsers,
} from "react-icons/lu";
import NavMenu from "@/components/common/nav-menu";
import { useFederation } from "@/contexts/federation";

function Navigation({ children }: { children: ReactNode }) {
  const router = useRouter();
  const { t } = useTranslation();
  const { isAdmin } = useFederation();
  const items = [
    { key: "overview", icon: LuHouse },
    { key: "league", icon: LuTrophy },
    { key: "rooms", icon: LuGamepad2 },
    { key: "identity", icon: LuUsers },
    { key: "events", icon: LuCalendar },
    { key: "approvals", icon: LuClipboardCheck },
    { key: "files", icon: LuFolder },
    ...(isAdmin ? [{ key: "manage", icon: LuSettings }] : []),
  ];
  return (
    <Grid templateColumns="1fr 3fr" gap={4} h="100%">
      <GridItem className="content-full-y">
        <NavMenu
          selectedKeys={[router.pathname]}
          onClick={(value) => void router.push(value)}
          items={items.map((item) => ({
            value: `/federation/${item.key}`,
            label: (
              <HStack spacing={2}>
                <Icon as={item.icon} />
                <Text fontSize="sm">{t(`FederationNav.${item.key}`)}</Text>
              </HStack>
            ),
            tooltip: t(`FederationNav.${item.key}`),
          }))}
        />
      </GridItem>
      <GridItem className="content-full-y" minW={0}>
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
