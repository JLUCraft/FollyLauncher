import {
  Avatar,
  Button,
  FormControl,
  FormLabel,
  HStack,
  Icon,
  Input,
  InputGroup,
  InputLeftElement,
  Select,
  Tag,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useRouter } from "next/router";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { LuSchool, LuSearch, LuUsers } from "react-icons/lu";
import {
  COMPACT_ICON_BUTTON_HEIGHT,
  CommonIconButton,
} from "@/components/common/common-icon-button";
import Empty from "@/components/common/empty";
import { OptionItem, OptionItemGroup } from "@/components/common/option-item";
import { Section } from "@/components/common/section";
import { CurrentGameConnection } from "@/components/federation/current-game-connection";
import { RoomNetworkSettings } from "@/components/federation/room-network-settings";
import { useLauncherConfig } from "@/contexts/config";
import { useFederation } from "@/contexts/federation";
import { useRooms } from "@/contexts/rooms";
import { FEDERATION_SETTINGS_PATH } from "@/utils/federation/navigation";

export function RoomPanel() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const router = useRouter();
  const { identityConfigured, identityRequired } = useFederation();
  const needsSetup = identityConfigured === false || identityRequired;
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
  const roomAnchor = (room: (typeof rooms)[number]) =>
    `room-${encodeURIComponent(`${room.peer}-${room.service}`)}`;
  const visibleRooms = rooms.filter(
    (room) =>
      (!school || room.metadata.school === school) &&
      [room.name, room.metadata.school, room.metadata.version].some((v) =>
        v.toLocaleLowerCase().includes(query.toLocaleLowerCase())
      )
  );
  return (
    <Section
      title={t("Rooms.title")}
      description={t("Rooms.description")}
      headExtra={
        <CommonIconButton
          icon="refresh"
          label={t("Rooms.refresh")}
          size="xs"
          fontSize="sm"
          h={COMPACT_ICON_BUTTON_HEIGHT}
          isLoading={busy}
          onClick={() => void handleRefresh()}
        />
      }
    >
      <VStack align="stretch" spacing={4}>
        {needsSetup && (
          <HStack flexWrap="wrap" spacing={3}>
            <Text fontSize="sm">{t("Rooms.setupHint")}</Text>
            <Button as={Link} href={FEDERATION_SETTINGS_PATH} size="sm">
              {t("FederationPage.openIdentity")}
            </Button>
          </HStack>
        )}
        <HStack spacing={2}>
          <InputGroup size="sm" flex="1">
            <InputLeftElement pointerEvents="none">
              <Icon as={LuSearch} className="secondary-text" />
            </InputLeftElement>
            <Input
              aria-label={t("RoomSearch.search")}
              placeholder={t("RoomSearch.search")}
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
              }}
              borderRadius="md"
              focusBorderColor={`${primaryColor}.500`}
            />
          </InputGroup>
          <Select
            aria-label={t("RoomSearch.school")}
            placeholder={t("RoomSearch.all")}
            value={school}
            onChange={(e) => {
              setSchool(e.target.value);
            }}
            size="sm"
            borderRadius="md"
            w="auto"
            flexShrink={0}
            focusBorderColor={`${primaryColor}.500`}
          >
            {Array.from(new Set(rooms.map((r) => r.metadata.school)))
              .sort()
              .map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
          </Select>
        </HStack>
        {partial && (
          <Text role="status" fontSize="sm" className="secondary-text">
            {t("Rooms.partial")}
          </Text>
        )}
        {seedsChanged && (
          <Text role="status" fontSize="sm" className="secondary-text">
            {t("Rooms.seedsChanged")}
          </Text>
        )}
        {!busy && rooms.length === 0 && (
          <Empty
            size="sm"
            withIcon={false}
            description={t(loaded ? "Rooms.empty" : "Rooms.initial")}
          />
        )}
        {visibleRooms.length > 0 && (
          <OptionItemGroup
            items={visibleRooms.map((room) => (
              <OptionItem
                key={roomAnchor(room)}
                id={roomAnchor(room)}
                prefixElement={
                  <Avatar
                    name={room.name}
                    boxSize="32px"
                    borderRadius="md"
                    fontSize="xs"
                  />
                }
                title={room.name}
                titleExtra={
                  <Tag colorScheme={primaryColor} className="tag-xs">
                    {room.metadata.version}
                  </Tag>
                }
                description={
                  <HStack
                    spacing={3}
                    fontSize="xs"
                    className="secondary-text"
                    flexWrap="wrap"
                  >
                    <HStack spacing={1}>
                      <Icon as={LuSchool} />
                      <Text>{room.metadata.school}</Text>
                    </HStack>
                    <HStack spacing={1}>
                      <Icon as={LuUsers} />
                      <Text>
                        {t("Rooms.maxPlayers", {
                          max: room.metadata.players_max,
                        })}
                      </Text>
                    </HStack>
                  </HStack>
                }
              >
                {needsSetup ? (
                  <Button
                    as={Link}
                    href={FEDERATION_SETTINGS_PATH}
                    size="sm"
                    variant="outline"
                    colorScheme={primaryColor}
                    flexShrink={0}
                    isDisabled={busy || !!address}
                  >
                    {t("Rooms.setupToJoin")}
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    colorScheme={primaryColor}
                    flexShrink={0}
                    isDisabled={busy || !!address}
                    onClick={() => void handleJoin(room)}
                  >
                    {t("Rooms.join")}
                  </Button>
                )}
              </OptionItem>
            ))}
          />
        )}
        {rooms.length > 0 && visibleRooms.length === 0 && (
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
            <Button as={Link} href={FEDERATION_SETTINGS_PATH}>
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
