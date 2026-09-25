import { useRouter } from "next/router";
import {
  ReactNode,
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { useDeepLink } from "@/hooks/deep-link";
import { GameConnection, UnionService } from "@/services/union";

const STATUS_INTERVAL_MS = 3000;
function useGameState() {
  const router = useRouter();
  const [gameLink, setGameLink] = useState("");
  const [connection, setConnection] = useState<GameConnection | null>(null);
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const generation = useRef(0);
  const knownConnection = useRef<GameConnection | null>(null);
  useDeepLink({
    trigger: (path) => path === "connect" || path.startsWith("connect?"),
    onCall: (url) => {
      setGameLink(url);
      void router.push("/federation/league");
    },
  });
  const connected = !!connection;
  useEffect(() => {
    let active = true;
    const refresh = async () => {
      const revision = generation.current;
      try {
        const next = await UnionService.gameConnection();
        if (active && revision === generation.current && !lock.current) {
          if (
            knownConnection.current?.peer !== next?.peer ||
            knownConnection.current?.service !== next?.service
          )
            setLabel("");
          knownConnection.current = next;
          setConnection(next);
        }
      } catch {
        /* A failed status query does not disconnect a live proxy. */
      }
    };
    void refresh();
    const timer = connected
      ? setInterval(() => void refresh(), STATUS_INTERVAL_MS)
      : undefined;
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [connected]);
  const change = async (
    operation: () => Promise<GameConnection | null>,
    name = ""
  ) => {
    if (lock.current) throw new Error("GAME_CONNECTION_BUSY");
    lock.current = true;
    generation.current++;
    setBusy(true);
    try {
      const next = await operation();
      knownConnection.current = next;
      setConnection(next);
      setLabel(name);
    } catch (error) {
      // Another window may already own the native proxy. Show its actual target.
      try {
        const actual = await UnionService.gameConnection();
        knownConnection.current = actual;
        setConnection(actual);
        setLabel("");
      } catch {
        /* Preserve the operation's original error. */
      }
      throw error;
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  return {
    gameLink,
    setGameLink,
    connection,
    label,
    busy,
    connect: (operation: () => Promise<GameConnection>, name: string) =>
      change(operation, name),
    disconnect: () =>
      change(async () => {
        await UnionService.disconnectGame();
        return null;
      }),
  };
}
const GameContext = createContext<ReturnType<typeof useGameState> | null>(null);
export function GameConnectionProvider({ children }: { children: ReactNode }) {
  const value = useGameState();
  return <GameContext.Provider value={value}>{children}</GameContext.Provider>;
}
export function useGameConnection() {
  const value = useContext(GameContext);
  if (!value) throw new Error("GameConnectionProvider missing");
  return value;
}
