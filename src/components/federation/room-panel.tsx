import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useRouter } from "next/router";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { CurrentGameConnection } from "@/components/federation/current-game-connection";
import { RoomNetworkSettings } from "@/components/federation/room-network-settings";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useRooms } from "@/contexts/rooms";
import {
  FEDERATION_SETTINGS_PATH,
  federationSettingsLink,
} from "@/utils/federation/navigation";

export function RoomPanel() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const router = useRouter();
  const { identityConfigured, identityRequired } = useFederation();
  const needsSetup = identityConfigured === false || identityRequired;
  const setupLink = (fragment = "") =>
    federationSettingsLink(
      FEDERATION_SETTINGS_PATH,
      `${router.asPath.split("#")[0]}${fragment}`
    );
  const {
    query,
    setQuery,
    school,
    setSchool,
    relays,
    setRelays,
    configPath,
    rooms,
    seedsChanged,
    host,
    address,
    busy,
    error,
    partial,
    loaded,
    handleRefresh,
    handleJoin,
    handleChoose,
    handleHost,
    handleStop,
  } = useRooms();
  const discoverOnEntry = useRef(handleRefresh);
  discoverOnEntry.current = handleRefresh;
  useEffect(() => {
    if (!loaded) void discoverOnEntry.current();
    // Discover once when the page opens; later visits keep the cached list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return (
    <Section title={t("Rooms.title")} description={t("Rooms.description")}>
      <VStack align="stretch" spacing={3}>
        {needsSetup && (
          <HStack flexWrap="wrap" spacing={3}>
            <Text fontSize="sm">{t("Rooms.setupHint")}</Text>
            <Button as={Link} href={setupLink()} size="sm">
              {t("FederationPage.openIdentity")}
            </Button>
          </HStack>
        )}
        {/* Search, school filter and refresh share one toolbar row. */}
        <HStack spacing={2} flexWrap="wrap">
          <Input
            aria-label={t("RoomSearch.search")}
            placeholder={t("RoomSearch.search")}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
            flex="1"
            minW="12rem"
          />
          <Select
            aria-label={t("RoomSearch.school")}
            placeholder={t("RoomSearch.all")}
            value={school}
            onChange={(e) => {
              setSchool(e.target.value);
            }}
            w="auto"
            minW="10rem"
            flexShrink={0}
          >
            {Array.from(new Set(rooms.map((r) => r.metadata.school)))
              .sort()
              .map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
          </Select>
          <Button
            flexShrink={0}
            onClick={() => void handleRefresh()}
            isLoading={busy}
          >
            {t("Rooms.refresh")}
          </Button>
        </HStack>
        {partial && <Text role="status">{t("Rooms.partial")}</Text>}
        {seedsChanged && <Text role="status">{t("Rooms.seedsChanged")}</Text>}
        {!busy && rooms.length === 0 && (
          <Empty
            size="sm"
            withIcon={false}
            description={t(loaded ? "Rooms.empty" : "Rooms.initial")}
          />
        )}
        {rooms
          .filter(
            (room) =>
              (!school || room.metadata.school === school) &&
              [room.name, room.metadata.school, room.metadata.version].some(
                (v) => v.toLocaleLowerCase().includes(query.toLocaleLowerCase())
              )
          )
          .map((room) => (
            <AdvancedCard
              key={`${room.peer}/${room.service}`}
              id={`room-${encodeURIComponent(`${room.peer}-${room.service}`)}`}
            >
              <VStack align="stretch">
                <Text fontWeight="semibold">{room.name}</Text>
                {/* the title above already shows the room name */}
                <Text fontSize="sm" className="secondary-text">
                  {t("Rooms.optionDetails", {
                    school: room.metadata.school,
                    version: room.metadata.version,
                    max: room.metadata.players_max,
                  })}
                </Text>
                {needsSetup ? (
                  <Button
                    as={Link}
                    href={setupLink(
                      `#room-${encodeURIComponent(`${room.peer}-${room.service}`)}`
                    )}
                    alignSelf="flex-start"
                    colorScheme={config.appearance.theme.primaryColor}
                    isDisabled={busy || !!address}
                  >
                    {t("Rooms.setupToJoin")}
                  </Button>
                ) : (
                  <Button
                    alignSelf="flex-start"
                    colorScheme={config.appearance.theme.primaryColor}
                    isDisabled={busy || !!address}
                    onClick={() => void handleJoin(room)}
                  >
                    {t("Rooms.join")}
                  </Button>
                )}
              </VStack>
            </AdvancedCard>
          ))}
        {rooms.length > 0 &&
          !rooms.some(
            (room) =>
              (!school || room.metadata.school === school) &&
              [room.name, room.metadata.school, room.metadata.version].some(
                (v) => v.toLocaleLowerCase().includes(query.toLocaleLowerCase())
              )
          ) && (
            <Empty
              size="sm"
              withIcon={false}
              description={t("RoomSearch.noMatches")}
            />
          )}
        <CurrentGameConnection />
        <Section
          id="room-host"
          title={t("Rooms.host")}
          isAccordion
          initialIsOpen={router.asPath.endsWith("#room-host")}
        >
          {needsSetup && !host ? (
            <Button as={Link} href={setupLink("#room-host")}>
              {t("Rooms.setupToHost")}
            </Button>
          ) : (
            <>
              <FormControl>
                <FormLabel htmlFor="room-config">
                  {t("Rooms.configuration")}
                </FormLabel>
                <Input id="room-config" value={configPath} isReadOnly />
              </FormControl>
              <Button
                isDisabled={busy || !!host}
                onClick={() => void handleChoose()}
              >
                {t("Rooms.pick")}
              </Button>
              <FormControl>
                <FormLabel htmlFor="room-relays">{t("Rooms.relays")}</FormLabel>
                <Textarea
                  id="room-relays"
                  value={relays}
                  onChange={(e) => setRelays(e.target.value)}
                  isDisabled={busy || !!host}
                />
              </FormControl>
              <Text fontSize="sm">{t("Rooms.hostHint")}</Text>
              <HStack>
                <Button
                  isLoading={busy}
                  isDisabled={!!host || !configPath}
                  onClick={() => void handleHost()}
                >
                  {t("Rooms.start")}
                </Button>
                <Button
                  isDisabled={busy || !host}
                  onClick={() => void handleStop()}
                >
                  {t("Rooms.stop")}
                </Button>
              </HStack>
              {host && (
                <Text role="status">
                  {t("Rooms.hosting", { name: host.name })}
                </Text>
              )}
            </>
          )}
        </Section>
        <RoomNetworkSettings />
        {error && (
          <Text role="alert" color="red.600">
            {t("Rooms.error")}
          </Text>
        )}
      </VStack>
    </Section>
  );
}
