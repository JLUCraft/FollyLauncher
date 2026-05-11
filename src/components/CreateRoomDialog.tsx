import { createSignal, Show } from "solid-js";
import { createQuickRoom } from "../services";

interface Props {
  onClose: () => void;
  onCreated: () => void;
  /** 来自 VC/onboarding 的社团名，不为 null 时才允许提交 */
  memberClub: string | null;
}

export function CreateRoomDialog(props: Props) {
  const [name, setName] = createSignal("");
  const [version, setVersion] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  async function handleSubmit(e: Event) {
    e.preventDefault();
    setError("");
    if (!name().trim()) {
      setError("房间名称不能为空");
      return;
    }
    if (!props.memberClub) {
      setError("未绑定社团 VC，无法创建联邦房间。请联系社长申请平台身份。");
      return;
    }
    if (!version().trim()) {
      setError("MC 版本不能为空");
      return;
    }
    setLoading(true);
    try {
      await createQuickRoom(name().trim(), version().trim());
      props.onCreated();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <div class="w-full max-w-md rounded-2xl border border-stone-200 bg-white p-6 shadow-2xl">
        <h3 class="text-lg font-bold text-stone-900">创建房间</h3>
        <p class="mt-1 text-sm text-stone-500">快速创建一个新的 Minecraft 房间实例。</p>

        <form onSubmit={handleSubmit} class="mt-5 space-y-4">
          <div>
            <label class="block text-sm font-medium text-stone-700">房间名称</label>
            <input
              class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
              placeholder="例如：JLU 生存服"
              value={name()}
              onInput={(e) => setName(e.currentTarget.value)}
              disabled={loading()}
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-stone-700">社团</label>
            <input
              class="mt-1 w-full rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-sm text-stone-700 cursor-not-allowed"
              value={props.memberClub ?? "未绑定社团"}
              readOnly
              disabled
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-stone-700">MC 版本</label>
            <select
              class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
              value={version()}
              onChange={(e) => setVersion(e.currentTarget.value)}
              disabled={loading()}
            >
              <option value="" disabled>请选择版本</option>
            </select>
          </div>

          <Show when={error()}>
            <p class="text-sm text-red-600">{error()}</p>
          </Show>

          <div class="flex gap-3 pt-2">
            <button
              type="button"
              class="btn flex-1 rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
              onClick={props.onClose}
              disabled={loading()}
            >
              取消
            </button>
            <button
              type="submit"
              class="btn flex-1 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
              disabled={loading() || !name().trim() || !props.memberClub || !version().trim()}
            >
              {loading() ? "创建中…" : "创建"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
