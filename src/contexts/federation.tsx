import {
  ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { brand } from "@/brand";
import {
  IdentityFacts,
  UnionService,
  UnionState,
  isIdentityNotConfigured,
} from "@/services/union";

interface FederationTarget {
  address: string;
  peer: string;
  genesis: string;
}

const CONNECTION_STORAGE_KEY = "federation-connections";
export function validTarget(value: unknown): value is FederationTarget {
  if (!value || typeof value !== "object") return false;
  const target = value as FederationTarget;
  return (
    typeof target.address === "string" &&
    !!target.address &&
    typeof target.peer === "string" &&
    !!target.peer &&
    typeof target.genesis === "string" &&
    /^[a-f0-9]{64}$/.test(target.genesis)
  );
}
function useFederationState() {
  const [address, setAddress] = useState(brand.federation.defaultAddress);
  const [peer, setPeer] = useState(brand.federation.defaultPeer);
  const [genesisHash, setGenesisHash] = useState("");
  const saved = useRef<{ league?: FederationTarget }>({});
  const remember = useCallback((domain: "league", target: FederationTarget) => {
    saved.current = { ...saved.current, [domain]: target };
    try {
      localStorage.setItem(
        CONNECTION_STORAGE_KEY,
        JSON.stringify(saved.current)
      );
    } catch {
      /* Session settings remain usable without browser storage. */
    }
  }, []);
  const [identityFacts, setIdentityFacts] = useState<IdentityFacts | null>(
    null
  );
  const [identity, setIdentity] = useState("");
  const [state, setState] = useState<UnionState>();
  const stateRef = useRef<UnionState>();
  const [activeTarget, setActiveTarget] = useState<FederationTarget>();
  const targetRef = useRef<FederationTarget>();
  const [connecting, setConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState(false);
  const [identityRequired, setIdentityRequired] = useState(false);
  const [identityError, setIdentityError] = useState(false);
  const generation = useRef(0);
  const connectTarget = useCallback(
    async (target: FederationTarget) => {
      const current = ++generation.current;
      if (JSON.stringify(targetRef.current) !== JSON.stringify(target)) {
        stateRef.current = undefined;
        setState(undefined);
      }
      targetRef.current = target;
      setActiveTarget(target);
      setConnecting(true);
      setConnectionError(false);
      setIdentityRequired(false);
      remember("league", target);
      try {
        const next = await UnionService.request(
          target.address,
          target.peer,
          target.genesis
        );
        if (current === generation.current) {
          stateRef.current = next;
          setState(next);
        }
      } catch (error) {
        if (current === generation.current) {
          if (isIdentityNotConfigured(error)) setIdentityRequired(true);
          else setConnectionError(true);
        }
      } finally {
        if (current === generation.current) setConnecting(false);
      }
    },
    [remember]
  );
  useEffect(() => {
    let active = true;
    void UnionService.identity()
      .then((id) => {
        if (active) setIdentity(id);
      })
      .catch(() => {
        if (active) setIdentityError(true);
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    let config: { league?: FederationTarget } = {};
    try {
      const parsed = JSON.parse(
        localStorage.getItem(CONNECTION_STORAGE_KEY) || "null"
      );
      if (validTarget(parsed?.league)) config.league = parsed.league;
    } catch {
      /* No valid saved configuration. */
    }
    saved.current = config;
    const target = config.league;
    if (target) {
      setAddress(target.address);
      setPeer(target.peer);
    }
    setGenesisHash(config.league?.genesis || "");
    if (config.league) void connectTarget(config.league);
  }, [connectTarget]);
  const request = useCallback(async (action?: Record<string, unknown>) => {
    const target = targetRef.current;
    const current = generation.current;
    if (!target || !stateRef.current) throw new Error("NOT_CONNECTED");
    const next = await UnionService.request(
      target.address,
      target.peer,
      target.genesis,
      stateRef.current.revision,
      action
    );
    if (
      current === generation.current &&
      (!stateRef.current || next.revision >= stateRef.current.revision)
    ) {
      stateRef.current = next;
      setState(next);
    }
  }, []);
  const member = Object.entries(state?.members || {}).find(
    ([, m]) => m.device === identity || m.additional_devices?.includes(identity)
  )?.[0];
  return {
    address,
    setAddress,
    peer,
    setPeer,
    genesisHash,
    setGenesisHash,
    identityFacts,
    setIdentityFacts,
    identity,
    state,
    activeTarget,
    connecting,
    connectionError,
    identityRequired,
    identityError,
    connect: () => connectTarget({ address, peer, genesis: genesisHash }),
    request,
    member,
    isAdmin: !!identity && !!state?.admins?.includes(identity),
  };
}
const FederationContext = createContext<ReturnType<
  typeof useFederationState
> | null>(null);
export function FederationProvider({ children }: { children: ReactNode }) {
  const value = useFederationState();
  return (
    <FederationContext.Provider value={value}>
      {children}
    </FederationContext.Provider>
  );
}
export function useFederation() {
  const value = useContext(FederationContext);
  if (!value) throw new Error("FederationProvider missing");
  return value;
}
// Request feedback belongs to the component initiating the operation, not every domain page.
export function useLeagueRequest() {
  const { request } = useFederation();
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const handleRequest = async (action?: Record<string, unknown>) => {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError("");
    try {
      await request(action);
    } catch {
      setError(t("FederationPage.error"));
    } finally {
      pending.current = false;
      setBusy(false);
    }
  };
  return { busy, error, handleRequest };
}
