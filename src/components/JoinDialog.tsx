import { createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { Instance } from "../types";
import { getResourceSyncStatus, syncResources } from "../api/tauri";

interface Props {
  instance: Instance;
  onClose: () => void;
}

export function JoinDialog(props: Props) {
  const [step, setStep] = createSignal<"confirm" | "syncing" | "launching" | "done" | "error">("confirm");
  const [errorMsg, setErrorMsg] = createSignal("");
  const [syncInfo, setSyncInfo] = createSignal("");

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
      const bridgePort = await invoke<number>("launch_instance", {
        instanceId: props.instance.id,
        version: props.instance.version,
      });
      console.log("bridge port:", bridgePort);
      setStep("done");
    } catch (e) {
      setErrorMsg(String(e));
      setStep("error");
    }
  }

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
                onClick={launch}
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
