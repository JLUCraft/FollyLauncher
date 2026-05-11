import { createSignal } from "solid-js";

/**
 * A reusable state machine for install/async-install operations.
 *
 * Each instance holds loading/result/error/errorFor signals and provides
 * an `execute` function that wraps the API call with standard try/catch
 * signal updates.
 *
 * Usage:
 *   const op = createInstallOperation<MyResult>();
 *   await op.execute("key", () => myApiCall(buildRequest()));
 */
export function createInstallOperation<Res>() {
    const [loading, setLoading] = createSignal<string | null>(null);
    const [result, setResult] = createSignal<Res | null>(null);
    const [error, setError] = createSignal("");
    const [errorFor, setErrorFor] = createSignal<string | null>(null);

    const execute = async (key: string, apiCall: () => Promise<Res>) => {
        setLoading(key);
        setResult(null);
        setError("");
        setErrorFor(null);
        try {
            const r = await apiCall();
            setResult(r);
            return r;
        } catch (e) {
            setError(String(e));
            setErrorFor(key);
            throw e;
        } finally {
            setLoading(null);
        }
    };

    return { loading, result, error, errorFor, execute, setLoading };
}
