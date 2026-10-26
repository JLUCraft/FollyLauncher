import {
  Button,
  FormControl,
  FormLabel,
  Input,
  Text,
  Textarea,
} from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { useRooms } from "@/contexts/rooms";

export function RoomNetworkSettings() {
  const { t } = useTranslation();
  const {
    seeds,
    setSeeds,
    resetSeeds,
    identities,
    busy,
    error,
    handleIdentities,
  } = useRooms();
  return (
    <Section
      title={t("Rooms.networkSettings")}
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
            <Input id="room-host-identity" value={identities.host} isReadOnly />
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
          onChange={(event) => setSeeds(event.target.value)}
          isDisabled={busy}
        />
      </FormControl>
      <Button onClick={resetSeeds} isDisabled={busy}>
        {t("ConnectionFlow.defaultSeed")}
      </Button>
      {error && <Text role="alert">{t("Rooms.error")}</Text>}
    </Section>
  );
}
