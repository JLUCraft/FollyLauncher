import { open } from "@tauri-apps/plugin-dialog";
import {
  ReactNode,
  createContext,
  useContext,
  useEffect,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { brand } from "@/brand";
import { useFederation } from "@/contexts/federation";
import { useGameConnection } from "@/contexts/game-connection";
import { Room, RoomService } from "@/services/rooms";
import { UnionService } from "@/services/union";

const routes = (value: string) => value.split(/\s+/).filter(Boolean);
function useRoomsState() {
  const { t } = useTranslation();
  const { address, activeTarget } = useFederation();
  const federationAddress = activeTarget?.address || address;
  const [seedOverride, setSeedOverride] = useState<string | null>(null);
  const seeds =
    seedOverride ?? (federationAddress || brand.federation.defaultAddress);
  const setSeeds = (value: string) => setSeedOverride(value);
  const resetSeeds = () => setSeedOverride(null);
  const { connection, connect, busy: gameBusy } = useGameConnection();
  const gameAddress = connection?.address;
  const [resultSource, setResultSource] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [school, setSchool] = useState("");
  const [relays, setRelays] = useState("");
  const [configPath, setConfigPath] = useState("");
  const [identities, setIdentities] = useState<{
    host: string;
    player: string;
  } | null>(null);
  const [discovered, setRooms] = useState<Room[]>([]);
  const rooms = resultSource === seeds ? discovered : [];
  const seedsChanged =
    resultSource !== null && resultSource !== seeds && discovered.length > 0;
  const [host, setHost] = useState<Room | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [partial, setPartial] = useState(false);
  const loaded = resultSource === seeds;
  useEffect(() => {
    let mounted = true;
    RoomService.status()
      .then((room) => {
        if (mounted) {
          setHost(room);
        }
      })
      .catch(() => {
        if (mounted) setError(true);
      });
    return () => {
      mounted = false;
    };
  }, []);
  const handle = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(false);
    try {
      await action();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  };
  const handleRefresh = () =>
    handle(async () => {
      const result = await RoomService.discover(routes(seeds));
      setRooms(result.rooms);
      setPartial(result.unavailable.length > 0);
      setHost(await RoomService.status());
      setResultSource(seeds);
    });
  const handleJoin = (room: Room) =>
    handle(async () => {
      await connect(() => RoomService.join(room), room.name);
    });
  const handleChoose = () =>
    handle(async () => {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: t("Rooms.configuration"), extensions: ["json"] }],
      });
      if (typeof path === "string") setConfigPath(path);
    });
  const handleHost = () =>
    handle(async () => {
      setHost(
        await RoomService.host(configPath, routes(seeds), routes(relays))
      );
    });
  const handleStop = () =>
    handle(async () => {
      await RoomService.stop();
      setHost(null);
    });
  const handleIdentities = () =>
    handle(async () => {
      const [host, player] = await Promise.all([
        RoomService.hostIdentity(),
        UnionService.identity(),
      ]);
      setIdentities({ host, player });
    });
  return {
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
    host,
    address: gameAddress,
    busy: busy || gameBusy,
    error,
    partial,
    loaded,
    handleRefresh,
    handleJoin,
    handleChoose,
    handleHost,
    handleStop,
    handleIdentities,
  };
}
const RoomsContext = createContext<ReturnType<typeof useRoomsState> | null>(
  null
);
export function RoomsProvider({ children }: { children: ReactNode }) {
  const value = useRoomsState();
  return (
    <RoomsContext.Provider value={value}>{children}</RoomsContext.Provider>
  );
}
export function useRooms() {
  const value = useContext(RoomsContext);
  if (!value) throw new Error("RoomsProvider missing");
  return value;
}
