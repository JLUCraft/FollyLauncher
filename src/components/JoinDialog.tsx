import { createSignal, createResource, Show } from "solid-js";
import type { Instance } from "../types";
import {
  getResourceSyncStatus,
  syncResources,
  getMuaStatus,
  launchInstance,
} from "../services";

interface Props {
  instance: Instance;
  onClose: () => void;
}

export function JoinDialog(props: Props) {
  const [step, setStep] = createSignal<"confirm" | "syncing" | "launching" | "done" | "error">("confirm");
  const [errorMsg, setErrorMsg] = createSignal("");
  const [syncInfo, setSyncInfo] = createSignal("");
  const [launchResult, setLaunchResult] = createSignal<{
    bridge_port: number;
    pid: number | null;
    target_peer_id: string;
  } | null>(null);

  const [muaStatus] = createResource(getMuaStatus);
  const loggedIn = () => muaStatus()?.logged_in === true;
  const peerBound = () => muaStatus()?.peer_bound === true;

  async function checkAndSync() {
    setStep("syncing");
    try {
      const status = await getResourceSyncStatus(props.instance.id);
      if (status.manifest_loaded && status.missing_files > 0) {
        setSyncInfo(`同步资源: ${status.cached_files}/${status.total_files} 已缓存，${status.missing_files} 待下载`);
        const result = await syncResources(props.instance.id);
        setSyncInfo(`下载完成: ${result.downloaded} 成功, ${result.failed} 失败, ${result.skipped} 跳过`);
        if (result.failed > 0) {
          throw new Error(`资源同步失败 ${result.failed} 个文件`);
        }
      } else if (status.manifest_loaded) {
        setSyncInfo("所有资源已缓存");
      } else {
        throw new Error("资源清单未加载");
      }
      await launch();
    } catch (e) {
      setErrorMsg(String(e));
      setStep("error");
    }
  }

  async function launch() {
    setStep("launching");
    try {
      const result = await launchInstance(props.instance.id, props.instance.version);
      setLaunchResult({
        bridge_port: result.bridge_port,
        pid: result.pid,
        target_peer_id: result.target_peer_id,
      });
      setStep("done");
    } catch (e) {
      setErrorMsg(String(e));
      setStep("error");
    }
  }

  const peerIdSummary = (peerId: string) => {
    if (peerId.length <= 12) return peerId;
    return peerId.slice(0, 6) + "…" + peerId.slice(-6);
  };

  return (
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <div class="w-full max-w-md rounded-xl border border-stone-300 bg-white p-6 shadow-2xl">
        <h2 class="text-xl font-bold text-stone-900">
          加入 {props.instance.name}
        </h2>

        <div class="mt-4 space-y-3 text-sm text-stone-600">
          <div class="flex justify-between">
            <span>模式</span>
            <span class="font-medium text-stone-900">{props.instance.mode}</span>
          </div>
          <div class="flex justify-between">
            <span>社团</span>
            <span class="font-medium text-stone-900">{props.instance.club}</span>
          </div>
          <div class="flex justify-between">
            <span>版本</span>
            <span class="font-medium text-stone-900">{props.instance.version}</span>
          </div>
          <div class="flex justify-between">
            <span>当前人数</span>
            <span class="font-medium text-stone-900">{props.instance.players}</span>
          </div>
        </div>

        {/* MUA status check */}
        <Show when={!muaStatus.loading && !loggedIn()}>
          <div class="mt-4 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            需要 MUA 登录以提供 Minecraft Yggdrasil 身份。请前往「我的」页面登录后重试。
          </div>
        </Show>

        {/* Peer not bound warning */}
        <Show when={loggedIn() && !peerBound()}>
          <div class="mt-4 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            PeerID 尚未与服务端绑定，部分实例可能拒绝连接。
          </div>
        </Show>

        {step() === "confirm" && (
          <div class="mt-6 flex gap-3">
            <button
              class="btn flex-1 rounded-md border-stone-300 bg-transparent text-stone-900 hover:bg-stone-100"
              onClick={props.onClose}
            >
              取消
            </button>
            <button
              class="btn flex-1 rounded-md bg-teal-800 text-white hover:bg-teal-900"
              disabled={!loggedIn()}
              onClick={checkAndSync}
            >
              启动游戏
            </button>
          </div>
        )}

        {step() === "syncing" && (
          <div class="mt-6 flex items-center justify-center gap-3 py-2">
            <span class="loading loading-spinner loading-sm text-teal-800" />
            <span class="text-sm text-stone-600">{syncInfo() || "检查资源状态..."}</span>
          </div>
        )}

        {step() === "launching" && (
          <div class="mt-6 flex items-center justify-center gap-3 py-2">
            <span class="loading loading-spinner loading-sm text-teal-800" />
            <span class="text-sm text-stone-600">正在启动 Minecraft...</span>
          </div>
        )}

        {step() === "done" && (
          <div class="mt-6 text-center py-2">
            <p class="text-sm font-medium text-teal-800">游戏已启动</p>
            <Show when={launchResult()}>
              {(r) => (
                <dl class="mt-3 divide-y divide-stone-100 text-left text-sm">
                  <div class="flex justify-between py-1.5">
                    <span class="text-stone-500">代理端口</span>
                    <span class="font-mono text-stone-800">{r().bridge_port}</span>
                  </div>
                  <div class="flex justify-between py-1.5">
                    <span class="text-stone-500">进程 PID</span>
                    <span class="font-mono text-stone-800">{r().pid ?? "未知"}</span>
                  </div>
                  <div class="flex justify-between py-1.5">
                    <span class="text-stone-500">目标 PeerID</span>
                    <span class="font-mono text-xs text-stone-700" title={r().target_peer_id}>
                      {peerIdSummary(r().target_peer_id)}
                    </span>
                  </div>
                </dl>
              )}
            </Show>
            <button
              class="btn mt-3 rounded-md bg-stone-950 text-white hover:bg-stone-800"
              onClick={props.onClose}
            >
              关闭
            </button>
          </div>
        )}

        {step() === "error" && (
          <div class="mt-6 py-2">
            <p class="text-sm text-red-700">启动失败: {errorMsg()}</p>
            <div class="mt-3 flex gap-3">
              <button
                class="btn flex-1 rounded-md border-stone-300 bg-transparent text-stone-900 hover:bg-stone-100"
                onClick={props.onClose}
              >
                关闭
              </button>
              <button
                class="btn flex-1 rounded-md bg-teal-800 text-white hover:bg-teal-900"
                onClick={checkAndSync}
              >
                重试
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
