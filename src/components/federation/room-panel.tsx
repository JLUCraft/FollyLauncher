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
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import { Section } from "@/components/common/section";
import { CurrentGameConnection } from "@/components/federation/current-game-connection";
import { useLauncherConfig } from "@/contexts/config";
import { useRooms } from "@/contexts/rooms";

export function RoomPanel() {
  const { t } = useTranslation();
  const { config } = useLauncherConfig();
  const {
    seeds,
    setSeeds,
    resetSeeds,
    query,
    setQuery,
    school,
    setSchool,
    relays,
    setRelays,
    configPath,
    identities,
    rooms,
    seedsChanged,
    selected,
    setSelected,
    selectedRoom,
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
    handleIdentities,
  } = useRooms();
  return (
    <Section title={t("Rooms.title")} description={t("Rooms.description")}>
      <VStack align="stretch" spacing={3}>
        <Section
          title={t("FederationNav.advanced")}
          isAccordion
          initialIsOpen={false}
        >
          <Text fontSize="sm">{t("Rooms.manualHint")}</Text>
          <Button isDisabled={busy} onClick={() => void handleIdentities()}>
            {t("Rooms.showIdentities")}
          </Button>
          {identities && (
            <>
              <FormControl>
                <FormLabel htmlFor="room-host-identity">
                  {t("Rooms.hostIdentity")}
                </FormLabel>
                <Input
                  id="room-host-identity"
                  value={identities.host}
                  isReadOnly
                />
              </FormControl>
              <FormControl>
                <FormLabel htmlFor="room-player-identity">
                  {t("Rooms.playerIdentity")}
                </FormLabel>
                <Input
                  id="room-player-identity"
                  value={identities.player}
                  isReadOnly
                />
              </FormControl>
            </>
          )}
          <FormControl>
            <FormLabel htmlFor="room-seeds">{t("Rooms.seeds")}</FormLabel>
            <Textarea
              id="room-seeds"
              value={seeds}
              onChange={(e) => setSeeds(e.target.value)}
              isDisabled={busy}
            />
          </FormControl>
          <Button onClick={resetSeeds} isDisabled={busy}>
            {t("ConnectionFlow.defaultSeed")}
          </Button>
        </Section>
        <Input
          aria-label={t("RoomSearch.search")}
          placeholder={t("RoomSearch.search")}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelected("");
          }}
        />
        <Select
          aria-label={t("RoomSearch.school")}
          placeholder={t("RoomSearch.all")}
          value={school}
          onChange={(e) => {
            setSchool(e.target.value);
            setSelected("");
          }}
        >
          {Array.from(new Set(rooms.map((r) => r.metadata.school)))
            .sort()
            .map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
        </Select>
        <Button onClick={() => void handleRefresh()} isLoading={busy}>
          {t("Rooms.refresh")}
        </Button>
        {partial && <Text role="status">{t("Rooms.partial")}</Text>}
        {seedsChanged && <Text role="status">{t("Rooms.seedsChanged")}</Text>}
        {!busy && rooms.length === 0 && (
          <Text>{t(loaded ? "Rooms.empty" : "Rooms.initial")}</Text>
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
            <AdvancedCard key={`${room.peer}/${room.service}`}>
              <VStack align="stretch">
                <Text fontWeight="semibold">{room.name}</Text>
                <Text>
                  {t("Rooms.option", {
                    name: room.name,
                    school: room.metadata.school,
                    version: room.metadata.version,
                    max: room.metadata.players_max,
                  })}
                </Text>
                <Button
                  aria-pressed={selected === `${room.peer}/${room.service}`}
                  isDisabled={busy || !!address}
                  onClick={() => setSelected(`${room.peer}/${room.service}`)}
                >
                  {t(
                    selected === `${room.peer}/${room.service}`
                      ? "RoomSearch.selected"
                      : "Rooms.choose"
                  )}
                </Button>
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
          ) && <Text>{t("RoomSearch.noMatches")}</Text>}
        <HStack>
          <Button
            colorScheme={config.appearance.theme.primaryColor}
            isDisabled={busy || !selectedRoom || !!address}
            onClick={() => void handleJoin()}
          >
            {t("Rooms.join")}
          </Button>
        </HStack>
        <CurrentGameConnection />
        <Section title={t("Rooms.host")} isAccordion initialIsOpen={false}>
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
            <Text role="status">{t("Rooms.hosting", { name: host.name })}</Text>
          )}
        </Section>
        {error && (
          <Text role="alert" color="red.500">
            {t("Rooms.error")}
          </Text>
        )}
      </VStack>
    </Section>
  );
}
