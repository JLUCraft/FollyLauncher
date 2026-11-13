import { useCallback, useState } from "react";
import type { DependencyList, Dispatch, SetStateAction } from "react";

/** Resets local state when its inputs change and discards updates from older inputs. */
export function useKeyedState<Value>(
  initialValue: Value | (() => Value),
  inputs: DependencyList,
  reset = true
): [Value, Dispatch<SetStateAction<Value>>] {
  const initialize = () =>
    typeof initialValue === "function"
      ? (initialValue as () => Value)()
      : initialValue;
  const [snapshot, setSnapshot] = useState(() => ({
    inputs,
    value: initialize(),
  }));
  let current = snapshot;
  if (
    inputs.length !== snapshot.inputs.length ||
    inputs.some((value, index) => !Object.is(value, snapshot.inputs[index]))
  ) {
    current = { inputs, value: reset ? initialize() : snapshot.value };
    setSnapshot(current);
  }
  const generation = current.inputs;
  const setValue = useCallback(
    (update: SetStateAction<Value>) => {
      setSnapshot((previous) => {
        if (previous.inputs !== generation) return previous;
        const value =
          typeof update === "function"
            ? (update as (previous: Value) => Value)(previous.value)
            : update;
        return Object.is(value, previous.value)
          ? previous
          : { ...previous, value };
      });
    },
    [generation]
  );
  return [current.value, setValue];
}
