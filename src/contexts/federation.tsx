import { useRouter } from "next/router";
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
import { useDeepLink } from "@/hooks/deep-link";
import {
  IdentityFacts,
  UnionService,
  UnionState,
  isIdentityNotConfigured,
} from "@/services/union";
import {
  CONNECTION_LINK_TRIGGER,
  FederationTarget,
  GENESIS_PATTERN,
  parseConnectionLink,
} from "@/utils/federation/connection-link";
import {
  FederationErrorKind,
  classifyFederationError,
  federationErrorKey,
} from "@/utils/federation/errors";
import {
  FEDERATION_ROOMS_PATH,
  FEDERATION_SETTINGS_PATH,
  federationSettingsLink,
} from "@/utils/federation/navigation";

const CONNECTION_STORAGE_KEY = "federation-connection";
export function validTarget(value: unknown): value is FederationTarget {
  if (!value || typeof value !== "object") return false;
  const target = value as FederationTarget;
  return (
    typeof target.address === "string" &&
    !!target.address &&
    typeof target.peer === "string" &&
    !!target.peer &&
    typeof target.genesis === "string" &&
    GENESIS_PATTERN.test(target.genesis)
  );
}
function useFederationState() {
  const [address, setAddress] = useState(brand.federation.defaultAddress);
  const [peer, setPeer] = useState(brand.federation.defaultPeer);
  const [genesisHash, setGenesisHash] = useState("");
  const [connectionLink, setConnectionLink] = useState("");
  const remember = useCallback((target: FederationTarget) => {
    try {
      localStorage.setItem(CONNECTION_STORAGE_KEY, JSON.stringify(target));
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
  const [connectionError, setConnectionError] =
    useState<FederationErrorKind | null>(null);
  const [identityConfigured, setIdentityConfigured] = useState<boolean | null>(
    null
  );
  const [identityRequired, setIdentityRequired] = useState(false);
  const [identityError, setIdentityError] = useState(false);
  const [discovering, setDiscovering] = useState(false);
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
      setDiscovering(false);
      setConnectionError(null);
      setIdentityRequired(false);
      remember(target);
      try {
        const next = await UnionService.request(
          target.address,
          target.peer,
          target.genesis
        );
        if (current === generation.current) {
          stateRef.current = next;
          setState(next);
          setConnectionLink((link) => {
            const pending = parseConnectionLink(link, brand.deeplinkScheme);
            return pending?.address === target.address &&
              pending.peer === target.peer &&
              pending.genesis === target.genesis
              ? ""
              : link;
          });
        }
      } catch (error) {
        if (current === generation.current) {
          if (isIdentityNotConfigured(error)) setIdentityRequired(true);
          else setConnectionError(classifyFederationError(error));
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
    let active = true;
    void UnionService.identityConfiguration()
      .then((value) => {
        if (active) setIdentityConfigured(!!value);
      })
      .catch(() => {
        if (active) setIdentityConfigured(false);
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    let target: FederationTarget | undefined;
    try {
      const parsed: unknown = JSON.parse(
        localStorage.getItem(CONNECTION_STORAGE_KEY) || "null"
      );
      if (validTarget(parsed)) target = parsed;
    } catch {
      /* No valid saved configuration. */
    }
    if (target) {
      restoredTarget.current = true;
      setAddress(target.address);
      setPeer(target.peer);
      setGenesisHash(target.genesis);
      void connectTarget(target);
    }
  }, [connectTarget]);
  const disconnect = useCallback(() => {
    // Invalidate any in-flight connect so it cannot resurrect state.
    generation.current += 1;
    stateRef.current = undefined;
    targetRef.current = undefined;
    setState(undefined);
    setActiveTarget(undefined);
    setConnecting(false);
    setDiscovering(false);
    setConnectionError(null);
    setIdentityRequired(false);
    try {
      localStorage.removeItem(CONNECTION_STORAGE_KEY);
    } catch {
      /* Session settings remain usable without browser storage. */
    }
  }, []);
  const connectWithLink = useCallback(
    (link: string) => {
      const target = parseConnectionLink(link, brand.deeplinkScheme);
      if (!target) return false;
      setAddress(target.address);
      setPeer(target.peer);
      setGenesisHash(target.genesis);
      void connectTarget(target);
      return true;
    },
    [connectTarget]
  );
  // LAN auto-connect: find the union pinned into this build, adopt its identity
  // configuration when missing, and connect without any manual input.
  const [discoveryRan, setDiscoveryRan] = useState(false);
  const discoveryGeneration = useRef(0);
  const restoredTarget = useRef(false);
  const discoverAndConnect = useCallback(async () => {
    const pin = brand.federation.genesisHash;
    if (!pin) return;
    const current = generation.current;
    const discovery = ++discoveryGeneration.current;
    const isCurrent = () =>
      current === generation.current &&
      discovery === discoveryGeneration.current;
    setDiscovering(true);
    setDiscoveryRan(true);
    try {
      const servers = await UnionService.discover(pin);
      if (!isCurrent()) return;
      const server = servers.find((entry) => entry.application);
      if (!server) return;
      if (!identityConfigured) {
        if (!isCurrent()) return;
        // Gate dispatch and continuation; an already dispatched command may have saved.
        await UnionService.identityConfiguration({
          genesis: server.genesis,
          genesis_hash: server.genesis_hash,
          address: server.address,
          peer: server.peer,
        });
        if (!isCurrent()) return;
        setIdentityConfigured(true);
      }
      // The announced address is freshly probed; prefer it over any remembered one.
      const target =
        server.application && GENESIS_PATTERN.test(server.application)
          ? {
              address: server.address,
              peer: server.peer,
              genesis: server.application,
            }
          : restoredTarget.current &&
              address &&
              peer &&
              GENESIS_PATTERN.test(genesisHash)
            ? { address, peer, genesis: genesisHash }
            : null;
      if (!target || !isCurrent()) return;
      setAddress(target.address);
      setPeer(target.peer);
      setGenesisHash(target.genesis);
      void connectTarget(target);
    } catch {
      /* Discovery failure falls back to the manual link on the page. */
    } finally {
      if (discovery === discoveryGeneration.current) setDiscovering(false);
    }
  }, [identityConfigured, connectTarget, address, peer, genesisHash]);
  useEffect(() => {
    if (identityConfigured === null || discoveryRan) return;
    if (state || connecting) return;
    // A remembered target gets one chance first; discovery repairs a lost
    // identity configuration or a target that stopped answering.
    if (restoredTarget.current && !identityRequired && !connectionError) return;
    void discoverAndConnect();
  }, [
    identityConfigured,
    discoveryRan,
    state,
    connecting,
    identityRequired,
    connectionError,
    discoverAndConnect,
  ]);
  const router = useRouter();
  useDeepLink({
    trigger: CONNECTION_LINK_TRIGGER,
    onCall: (url) => {
      const target = parseConnectionLink(url, brand.deeplinkScheme);
      if (!target) return;
      setAddress(target.address);
      setPeer(target.peer);
      setGenesisHash(target.genesis);
      setConnectionLink(url);
      void router.push(
        federationSettingsLink(FEDERATION_SETTINGS_PATH, FEDERATION_ROOMS_PATH)
      );
    },
  });
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
      return true;
    }
    return false;
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
    connectionLink,
    setConnectionLink,
    identityFacts,
    setIdentityFacts,
    identity,
    state,
    activeTarget,
    connecting,
    connectionError,
    identityConfigured,
    setIdentityConfigured,
    identityRequired,
    identityError,
    connect: () => connectTarget({ address, peer, genesis: genesisHash }),
    connectWithLink,
    disconnect,
    discovering,
    discoveryRan,
    discoverAndConnect,
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
export function useApplicationRequest() {
  const { request } = useFederation();
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const handleRequest = async (action?: Record<string, unknown>) => {
    if (pending.current) return false;
    pending.current = true;
    setBusy(true);
    setError("");
    try {
      return await request(action);
    } catch (error) {
      setError(t(federationErrorKey(classifyFederationError(error))));
      return false;
    } finally {
      pending.current = false;
      setBusy(false);
    }
  };
  return { busy, error, handleRequest };
}
