import { createEffect, createSignal, For, Show } from "solid-js";
import { createQuery } from "@tanstack/solid-query";
import { save } from "@tauri-apps/plugin-dialog";
import { launchListStates, launchCancel, launchExportCrash, type LaunchStateResponse } from "../services";

function deriveStatus(state: LaunchStateResponse): "Running" | "Exited" | "Crashed" | "Pending" | "Unknown" {
  if (state.exit_code != null) {
    return state.exit_ok === true ? "Exited" : "Crashed";
  }
  if (state.game_ready) return "Running";
  if (!state.step || state.step === "pending") return "Pending";
  return "Unknown";
}

function statusLabel(status: ReturnType<typeof deriveStatus>): string {
  const map: Record<string, string> = {
    Running: "运行中",
    Exited: "已退出",
    Crashed: "崩溃",
    Pending: "等待中",
    Unknown: "其他",
  };
  return map[status] ?? status;
}

function statusColor(status: ReturnType<typeof deriveStatus>): string {
  const map: Record<string, string> = {
    Running: "bg-emerald-100 text-emerald-800",
    Exited: "bg-stone-100 text-stone-600",
    Crashed: "bg-red-100 text-red-800",
    Pending: "bg-amber-100 text-amber-800",
    Unknown: "bg-blue-100 text-blue-800",
  };
  return map[status] ?? "bg-stone-100 text-stone-600";
}

function fmtTime(ts: number | null): string {
  if (ts == null) return "—";
  // Assume seconds; handle both seconds and milliseconds
  const ms = ts > 1e12 ? ts : ts * 1000;
  return new Date(ms).toLocaleString();
}

export function LaunchPage() {
  const [selectedId, setSelectedId] = createSignal<number | null>(null);
  const [actionLoading, setActionLoading] = createSignal(false);
  const [actionError, setActionError] = createSignal<string | null>(null);
  const [actionOk, setActionOk] = createSignal<string | null>(null);

  const statesQuery = createQuery(() => ({
    queryKey: ["launch-states"],
    queryFn: launchListStates,
    refetchInterval: 2000,
  }));

  const selected = () => {
    const id = selectedId();
    if (id == null) return null;
    return (statesQuery.data ?? []).find((s) => s.id === id) ?? null;
  };

  // Clear stale selection when the selected item disappears from the list
  createEffect(() => {
    const id = selectedId();
    if (id == null) return;
    // Force reactivity on the data to re-check after each refetch
    const list = statesQuery.data ?? [];
    if (list.length > 0 && !list.some((s) => s.id === id)) {
      setSelectedId(null);
    }
  });

  const handleCancel = async (launchingId: number) => {
    setActionLoading(true);
    setActionError(null);
    setActionOk(null);
    try {
      await launchCancel(launchingId);
      setActionOk("已发送取消指令");
      statesQuery.refetch();
    } catch (e) {
      setActionError(String(e));
    } finally {
      setActionLoading(false);
    }
  };

  const handleExportCrash = async (launchingId: number) => {
    setActionLoading(true);
    setActionError(null);
    setActionOk(null);
    try {
      const path = await save({
        defaultPath: `crash-report-${launchingId}.zip`,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (!path) {
        setActionLoading(false);
        return;
      }
      const result = await launchExportCrash(launchingId, path);
      setActionOk(`崩溃报告已导出至: ${result}`);
    } catch (e) {
      setActionError(String(e));
    } finally {
      setActionLoading(false);
    }
  };

  return (
    <div class="flex h-full flex-col">
      {/* Page header */}
      <header class="border-b border-stone-200 px-8 py-5">
        <div class="flex items-center justify-between">
          <h2 class="text-2xl font-black text-stone-950">启动日志</h2>
          <div class="flex items-center gap-3 text-sm text-stone-500">
            <button
              class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900"
              onClick={() => statesQuery.refetch()}
            >
              刷新
            </button>
            <Show when={statesQuery.isFetching}>
              <span class="loading loading-spinner loading-xs" />
            </Show>
            <span>{(statesQuery.data ?? []).length} 条记录</span>
          </div>
        </div>
      </header>

      {/* Error display */}
      <Show when={statesQuery.error}>
        <div class="mx-8 mt-3 rounded bg-red-50 px-4 py-2 text-sm text-red-700">
          加载失败：{String(statesQuery.error)}
        </div>
      </Show>

      {/* Content area */}
      <div class="flex flex-1 overflow-hidden">
        {/* Launch state list */}
        <div class="w-80 shrink-0 overflow-y-auto border-r border-stone-200">
          <Show
            when={(statesQuery.data ?? []).length > 0}
            fallback={
              <div class="flex h-40 items-center justify-center text-stone-400">
                暂无启动记录
              </div>
            }
          >
            <div class="divide-y divide-stone-100">
              <For each={statesQuery.data}>
                {(state) => {
                  const st = deriveStatus(state);
                  return (
                    <button
                      type="button"
                      class={`w-full px-4 py-3 text-left transition-colors hover:bg-stone-50 ${
                        selectedId() === state.id ? "bg-teal-50 border-l-2 border-teal-600" : ""
                      }`}
                      onClick={() => setSelectedId(state.id)}
                    >
                      <div class="flex items-center justify-between">
                        <span class="truncate text-sm font-semibold text-stone-800">
                          {state.instance_id}
                        </span>
                        <span class={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium ${statusColor(st)}`}>
                          {statusLabel(st)}
                        </span>
                      </div>
                      <div class="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-stone-500">
                        <span>{state.version}</span>
                        <Show when={state.pid !== 0}>
                          <span>PID {state.pid}</span>
                        </Show>
                        <span>{state.step || "—"}</span>
                      </div>
                    </button>
                  );
                }}
              </For>
            </div>
          </Show>
        </div>

        {/* Detail panel */}
        <div class="flex-1 overflow-y-auto bg-stone-50">
          <Show
            when={selected()}
            fallback={
              <div class="flex h-full items-center justify-center text-stone-400">
                选择左侧一条记录查看详情
              </div>
            }
          >
            {(state) => {
              const st = deriveStatus(state());
              return (
                <div class="p-6">
                  {/* Detail card */}
                  <div class="rounded-lg border border-stone-200 bg-white p-5 shadow-sm">
                    <div class="flex items-center justify-between">
                      <h3 class="text-lg font-bold text-stone-900">{state().instance_id}</h3>
                      <span class={`rounded px-2 py-0.5 text-xs font-semibold ${statusColor(st)}`}>
                        {statusLabel(st)}
                      </span>
                    </div>
                    <div class="mt-4 grid grid-cols-2 gap-3 text-sm">
                      <div>
                        <span class="text-stone-400">版本</span>
                        <p class="font-medium text-stone-800">{state().version}</p>
                      </div>
                      <div>
                        <span class="text-stone-400">PID</span>
                        <p class="font-medium text-stone-800">{state().pid !== 0 ? state().pid : "—"}</p>
                      </div>
                      <div>
                        <span class="text-stone-400">步骤</span>
                        <p class="font-medium text-stone-800">{state().step || "—"}</p>
                      </div>
                      <div>
                        <span class="text-stone-400">游戏就绪</span>
                        <p class="font-medium text-stone-800">{state().game_ready ? "是" : "否"}</p>
                      </div>
                      <div>
                        <span class="text-stone-400">退出码</span>
                        <p class="font-medium text-stone-800">
                          {state().exit_code != null ? state().exit_code : "—"}
                        </p>
                      </div>
                      <div>
                        <span class="text-stone-400">正常退出</span>
                        <p class="font-medium text-stone-800">
                          {state().exit_ok != null ? (state().exit_ok ? "是" : "否") : "—"}
                        </p>
                      </div>
                      <div>
                        <span class="text-stone-400">启动时间</span>
                        <p class="font-medium text-stone-800">{fmtTime(state().start_time)}</p>
                      </div>
                      <div>
                        <span class="text-stone-400">结束时间</span>
                        <p class="font-medium text-stone-800">{fmtTime(state().end_time)}</p>
                      </div>
                    </div>
                  </div>

                  {/* Action buttons */}
                  <div class="mt-3 flex flex-wrap items-center gap-2">
                    <Show when={actionOk()}>
                      <div class="rounded bg-emerald-50 px-3 py-1.5 text-xs text-emerald-700">{actionOk()}</div>
                    </Show>
                    <Show when={actionError()}>
                      <div class="rounded bg-red-50 px-3 py-1.5 text-xs text-red-700">{actionError()}</div>
                    </Show>
                    <Show when={st === "Running" || st === "Pending"}>
                      <button
                        class="btn rounded-lg bg-amber-700 px-4 py-2 text-sm text-white hover:bg-amber-800 disabled:opacity-50"
                        disabled={actionLoading()}
                        onClick={() => handleCancel(state().id)}
                      >
                        {actionLoading() ? "处理中..." : "取消启动"}
                      </button>
                    </Show>
                    <Show when={st === "Crashed"}>
                      <button
                        class="btn rounded-lg bg-red-700 px-4 py-2 text-sm text-white hover:bg-red-800 disabled:opacity-50"
                        disabled={actionLoading()}
                        onClick={() => handleExportCrash(state().id)}
                      >
                        {actionLoading() ? "处理中..." : "导出崩溃报告"}
                      </button>
                    </Show>
                  </div>

                  {/* Logs panel */}
                  <div class="mt-4 rounded-lg border border-stone-200 bg-white p-5 shadow-sm">
                    <h4 class="mb-3 text-sm font-semibold text-stone-700">最近日志</h4>
                    <Show
                      when={(state().recent_logs ?? []).length > 0}
                      fallback={
                        <p class="py-8 text-center text-sm text-stone-400">暂无日志</p>
                      }
                    >
                      <pre class="max-h-96 overflow-auto rounded bg-stone-900 p-4 text-xs text-green-300 leading-relaxed">
                        <For each={state().recent_logs}>
                          {(line) => <div>{line}</div>}
                        </For>
                      </pre>
                    </Show>
                  </div>
                </div>
              );
            }}
          </Show>
        </div>
      </div>
    </div>
  );
}
