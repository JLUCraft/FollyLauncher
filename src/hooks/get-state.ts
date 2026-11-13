import { useCallback } from "react";
import { useKeyedState } from "@/hooks/keyed-state";

export enum GetStateFlag {
  Cancelled = "%CANCELLED%",
}

export function useGetState<T>(
  state: T | undefined,
  retrieveHandler: () => void
): (sync?: boolean) => T | undefined {
  const getState = useCallback(
    (sync = false) => {
      if (sync || state === undefined) retrieveHandler();
      return state;
    },
    [state, retrieveHandler]
  );

  return getState;
}

export function usePromisedGetState<T>(
  state: T | undefined,
  version: string | undefined,
  retrieveHandler: () => Promise<NoInfer<T> | GetStateFlag | undefined>
): [(sync?: boolean) => Promise<T | GetStateFlag | undefined>, boolean] {
  const [status, setStatus] = useKeyedState<{
    completedVersion: string | undefined;
    pending: number;
  }>({ completedVersion: undefined, pending: 0 }, [version]);
  const getState = useCallback(
    async (sync = false) => {
      if (!sync && state !== undefined && status.completedVersion === version)
        return state;
      setStatus((previous) => ({ ...previous, pending: previous.pending + 1 }));
      try {
        return await retrieveHandler();
      } catch {
        return undefined;
      } finally {
        setStatus((previous) => ({
          ...previous,
          pending: previous.pending - 1,
          completedVersion: version,
        }));
      }
    },
    [state, version, status.completedVersion, retrieveHandler, setStatus]
  );
  return [getState, status.pending > 0 || status.completedVersion !== version];
}
