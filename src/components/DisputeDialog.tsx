import { createSignal, For } from "solid-js";
import { createMatchDispute, type DisputeMatch } from "../services";

interface Props {
  tournamentId: string;
  matchId: string;
  matchRound: number;
  onClose: (dispute?: DisputeMatch) => void;
}

export function DisputeDialog(props: Props) {
  const [reason, setReason] = createSignal("");
  const [urlInput, setUrlInput] = createSignal("");
  const [evidenceUrls, setEvidenceUrls] = createSignal<string[]>([]);
  const [sending, setSending] = createSignal(false);
  const [error, setError] = createSignal("");

  function addUrl() {
    const url = urlInput().trim();
    if (!url) return;
    if (evidenceUrls().includes(url)) {
      setError("该链接已存在");
      return;
    }
    setEvidenceUrls([...evidenceUrls(), url]);
    setUrlInput("");
    setError("");
  }

  function removeUrl(url: string) {
    setEvidenceUrls(evidenceUrls().filter((u) => u !== url));
  }

  async function handleSubmit() {
    const trimmedReason = reason().trim();
    if (!trimmedReason) {
      setError("请填写争议原因");
      return;
    }
    setSending(true);
    setError("");
    try {
      const result = await createMatchDispute(
        props.tournamentId,
        props.matchId,
        trimmedReason,
        evidenceUrls(),
      );
      props.onClose(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  }

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <div class="w-full max-w-md rounded-xl border border-stone-300 bg-white p-6 shadow-2xl">
        <h2 class="text-xl font-bold text-stone-900">
          提交比赛争议 — 第 {props.matchRound} 轮
        </h2>
        <p class="mt-1 text-sm text-stone-500">
          请描述争议原因并提供证据链接（可选）。
        </p>

        {}
        <div class="mt-4">
          <label class="block text-sm font-medium text-stone-700 mb-1">
            争议原因 <span class="text-red-500">*</span>
          </label>
          <textarea
            class="textarea textarea-bordered w-full rounded-lg border-stone-300 bg-white px-3 py-2 text-sm"
            rows={4}
            placeholder="例如：对手使用外挂、比赛结果争议、规则违反等"
            value={reason()}
            onInput={(e) => setReason(e.currentTarget.value)}
            disabled={sending()}
          />
        </div>

        {}
        <div class="mt-4">
          <label class="block text-sm font-medium text-stone-700 mb-1">
            证据链接
          </label>
          <div class="flex gap-2">
            <input
              class="input input-bordered flex-1 rounded-lg border-stone-300 bg-white px-3 py-2 text-sm"
              placeholder="https://..."
              value={urlInput()}
              onInput={(e) => setUrlInput(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addUrl();
                }
              }}
              disabled={sending()}
            />
            <button
              type="button"
              class="btn rounded-lg bg-stone-200 px-4 py-2 text-sm text-stone-700 hover:bg-stone-300 disabled:opacity-50"
              onClick={addUrl}
              disabled={sending() || !urlInput().trim()}
            >
              添加
            </button>
          </div>

          <For each={evidenceUrls()}>
            {(url) => (
              <div class="mt-2 flex items-center justify-between rounded-lg border border-stone-200 bg-stone-50 px-3 py-1.5">
                <span class="truncate text-xs text-stone-600">{url}</span>
                <button
                  type="button"
                  class="ml-2 shrink-0 text-xs text-red-500 hover:text-red-700"
                  onClick={() => removeUrl(url)}
                  disabled={sending()}
                >
                  移除
                </button>
              </div>
            )}
          </For>
        </div>

        {}
        {error() && (
          <p class="mt-3 text-sm text-red-600">{error()}</p>
        )}

        {}
        <div class="mt-5 flex gap-3">
          <button
            type="button"
            class="btn flex-1 rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
            onClick={() => props.onClose()}
            disabled={sending()}
          >
            取消
          </button>
          <button
            type="button"
            class="btn flex-1 rounded-lg bg-red-700 text-white hover:bg-red-800 disabled:opacity-50"
            onClick={handleSubmit}
            disabled={sending() || !reason().trim()}
          >
            {sending() ? "提交中…" : "提交争议"}
          </button>
        </div>
      </div>
    </div>
  );
}
