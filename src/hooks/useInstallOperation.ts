import { createSignal } from "solid-js";


 *
 *
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
