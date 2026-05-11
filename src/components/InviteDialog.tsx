import { createSignal } from "solid-js";
import { invitePlayers } from "../services";

interface Props {
  instanceId: string;
  instanceName: string;
  onClose: () => void;
}

export function InviteDialog(props: Props) {
  const [playerInput, setPlayerInput] = createSignal("");
  const [players, setPlayers] = createSignal<string[]>([]);
  const [sending, setSending] = createSignal(false);
  const [result, setResult] = createSignal<{
    invited_count: number;
    missing_recipients: string[];
  } | null>(null);
  const [error, setError] = createSignal("");

  function addPlayer() {
    const name = playerInput().trim();
    if (!name) return;
    if (players().includes(name)) {
      setError("该玩家已在列表中");
      return;
    }
    setPlayers([...players(), name]);
    setPlayerInput("");
    setError("");
  }

  function removePlayer(name: string) {
    setPlayers(players().filter((p) => p !== name));
  }

  async function handleInvite() {
    if (players().length === 0) {
      setError("请至少添加一位玩家");
      return;
    }
    setSending(true);
    setError("");
    setResult(null);
    try {
      const res = await invitePlayers(props.instanceId, players());
      setResult({
        invited_count: res.invited_count,
        missing_recipients: res.missing_recipients,
      });
      setPlayers([]);
      setPlayerInput("");
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
          邀请玩家加入 {props.instanceName}
        </h2>
        <p class="mt-1 text-sm text-stone-500">
          输入 MUA 用户名邀请其他玩家加入此实例。
          {players().length > 0 && (
            <span class="ml-1 font-medium text-teal-700">
              已添加 {players().length} 位
            </span>
          )}
        </p>

        {/* Input */}
        <div class="mt-4 flex gap-2">
          <input
            class="input input-bordered flex-1 rounded-lg border-stone-300 bg-white px-3 py-2 text-sm"
            placeholder="玩家 MUA 用户名"
            value={playerInput()}
            onInput={(e) => setPlayerInput(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addPlayer();
              }
            }}
            disabled={sending()}
          />
          <button
            type="button"
            class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900 disabled:opacity-50"
            onClick={addPlayer}
            disabled={sending() || !playerInput().trim()}
          >
            添加
          </button>
        </div>

        {/* Player list */}
        {players().length > 0 && (
          <div class="mt-3 max-h-40 overflow-y-auto rounded-lg border border-stone-200 bg-stone-50 p-2">
            <ul class="space-y-1">
              {players().map((name) => (
                <li class="flex items-center justify-between rounded px-2 py-1 text-sm">
                  <span class="font-medium text-stone-800">{name}</span>
                  <button
                    type="button"
                    class="text-xs text-red-500 hover:text-red-700"
                    onClick={() => removePlayer(name)}
                    disabled={sending()}
                  >
                    移除
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Error */}
        {error() && (
          <p class="mt-3 text-sm text-red-600">{error()}</p>
        )}

        {/* Result */}
        {result() && (
          <div class="mt-3 rounded-lg bg-green-50 border border-green-200 p-3">
            <p class="text-sm font-medium text-green-800">
              已邀请 {result()?.invited_count} 位玩家
            </p>
            {result()!.missing_recipients.length > 0 && (
              <p class="mt-1 text-xs text-amber-700">
                未送达: {result()!.missing_recipients.join("、")}
              </p>
            )}
          </div>
        )}

        {/* Actions */}
        <div class="mt-5 flex gap-3">
          <button
            type="button"
            class="btn flex-1 rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
            onClick={props.onClose}
            disabled={sending()}
          >
            关闭
          </button>
          <button
            type="button"
            class="btn flex-1 rounded-lg bg-teal-800 text-white hover:bg-teal-900 disabled:opacity-50"
            onClick={handleInvite}
            disabled={sending() || players().length === 0}
          >
            {sending() ? "发送中…" : `邀请 (${players().length})`}
          </button>
        </div>
      </div>
    </div>
  );
}
