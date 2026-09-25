import {
  ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { useFederation, validTarget } from "@/contexts/federation";
import { OfficeService, OfficeSnapshot, OfficeTarget } from "@/services/office";

function useOfficeRequestState(
  execute: ReturnType<typeof useOffice>["execute"]
) {
  const [pendingKey, setPendingKey] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const pending = useRef(false);
  const run = async (
    operation?: (target: OfficeTarget) => Promise<unknown>,
    key = ""
  ) => {
    if (pending.current) return false;
    pending.current = true;
    setPendingKey(key);
    setError(false);
    setUncertain(false);
    try {
      await execute(operation);
      return true;
    } catch {
      setError(true);
      setUncertain(!!operation);
      return false;
    } finally {
      pending.current = false;
      setPendingKey(null);
    }
  };
  return {
    busy: pendingKey !== null,
    pendingKey,
    error,
    uncertain,
    run,
    act: (action: Record<string, unknown>, key?: string) =>
      run((target) => OfficeService.action(target, action), key),
  };
}
export const OFFICE_REFRESH_KEY = "office:refresh";
function useOfficeState() {
  const {
    address,
    peer,
    identity,
    officeGenesis: genesis,
    setOfficeGenesis: setGenesis,
    restored,
    remember,
  } = useFederation();
  const [data, setData] = useState<OfficeSnapshot>();
  const [target, setTarget] = useState<OfficeTarget>();
  const [connecting, setConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState(false);
  const generation = useRef(0);
  const targetRef = useRef<OfficeTarget>();
  const connectTarget = useCallback(
    async (nextTarget: OfficeTarget) => {
      const current = ++generation.current;
      if (JSON.stringify(targetRef.current) !== JSON.stringify(nextTarget))
        setData(undefined);
      targetRef.current = nextTarget;
      setTarget(nextTarget);
      setConnecting(true);
      setConnectionError(false);
      remember("office", nextTarget);
      try {
        const next = await OfficeService.snapshot(nextTarget);
        if (current === generation.current) setData(next);
      } catch {
        if (current === generation.current) setConnectionError(true);
      } finally {
        if (current === generation.current) setConnecting(false);
      }
    },
    [remember]
  );
  useEffect(() => {
    if (restored?.office) void connectTarget(restored.office);
  }, [restored, connectTarget]);
  const execute = useCallback(
    async (operation?: (target: OfficeTarget) => Promise<unknown>) => {
      const active = targetRef.current;
      if (!active) throw new Error("NOT_CONNECTED");
      const current = generation.current;
      if (operation) await operation(active);
      const next = await OfficeService.snapshot(active);
      if (current === generation.current)
        setData((old) =>
          !old || next.state.revision >= old.state.revision ? next : old
        );
    },
    []
  );
  const member = Object.entries(data?.identity.members || {}).find(
    ([, m]) => m.device === identity || m.additional_devices?.includes(identity)
  )?.[0];
  const request = useOfficeRequestState(execute);
  return {
    genesis,
    setGenesis,
    target,
    data,
    member,
    isAdmin: !!data?.identity.admins.includes(identity),
    ready: !!target,
    canConnect: validTarget({ address, peer, genesis }),
    connecting,
    connectionError,
    connect: () => connectTarget({ address, peer, genesis }),
    execute,
    request,
  };
}
const OfficeContext = createContext<ReturnType<typeof useOfficeState> | null>(
  null
);
export function OfficeProvider({ children }: { children: ReactNode }) {
  const value = useOfficeState();
  return (
    <OfficeContext.Provider value={value}>{children}</OfficeContext.Provider>
  );
}
export function useOffice() {
  const value = useContext(OfficeContext);
  if (!value) throw new Error("OfficeProvider missing");
  return value;
}
export function useOfficeRequest() {
  return useOffice().request;
}
