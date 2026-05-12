import { createSignal, For, Show, createEffect, createMemo, onCleanup } from "solid-js";
import {
    listTaskGroups,
    cancelTaskGroup,
    removeTaskGroup,
    exportTaskSnapshot,
    importTaskSnapshot,
    type TaskGroup,
    type TaskProgress,
    type ImportTaskSnapshotResult,
} from "../services";

type StatusFilter = "all" | "active" | "completed" | "failed" | "cancelled";

const FILTER_OPTIONS: { value: StatusFilter; label: string }[] = [
    { value: "all", label: "全部" },
    { value: "active", label: "活跃" },
    { value: "completed", label: "已完成" },
    { value: "failed", label: "失败" },
    { value: "cancelled", label: "已取消" },
];

function statusLabel(status: string): string {
    const map: Record<string, string> = {
        Pending: "等待中",
        Running: "进行中",
        Paused: "已暂停",
        Completed: "已完成",
        Failed: "失败",
        Cancelled: "已取消",
    };
    return map[status] ?? status;
}

function statusColor(status: string): string {
    const map: Record<string, string> = {
        Pending: "text-stone-500",
        Running: "text-blue-600",
        Paused: "text-amber-500",
        Completed: "text-emerald-600",
        Failed: "text-red-600",
        Cancelled: "text-stone-400",
    };
    return map[status] ?? "text-stone-600";
}

function progressBarColor(status: string): string {
    const map: Record<string, string> = {
        Pending: "bg-stone-300",
        Running: "bg-blue-500",
        Paused: "bg-amber-400",
        Completed: "bg-emerald-500",
        Failed: "bg-red-500",
        Cancelled: "bg-stone-300",
    };
    return map[status] ?? "bg-stone-400";
}

function isActiveStatus(s: string): boolean {
    return s === "Running" || s === "Pending" || s === "Paused";
}

function statusMatchesFilter(status: string, filter: StatusFilter): boolean {
    if (filter === "all") return true;
    if (filter === "active") return isActiveStatus(status);
    if (filter === "completed") return status === "Completed";
    if (filter === "failed") return status === "Failed";
    if (filter === "cancelled") return status === "Cancelled";
    return false;
}

function groupMatchesKeyword(group: TaskGroup, keyword: string): boolean {
    const trimmed = keyword.trim();
    if (!trimmed) return true;
    const lower = trimmed.toLowerCase();
    if (group.groupId.toLowerCase().includes(lower)) return true;
    for (const t of group.tasks) {
        if (t.name.toLowerCase().includes(lower)) return true;
        if (t.message && t.message.toLowerCase().includes(lower)) return true;
    }
    return false;
}

function TaskProgressRow(props: { task: TaskProgress }) {
    const t = props.task;
    const taskPct =
        t.status === "Completed"
            ? 100
            : t.total > 0
                ? Math.round((t.current / t.total) * 100)
                : 0;

    return (
        <div class="flex items-center gap-3 rounded border border-stone-200 bg-stone-50 px-3 py-2">
            <div class="flex-1 min-w-0">
                <p class="truncate text-sm font-medium text-stone-800">{t.name}</p>
                <Show when={t.message}>
                    <p class="truncate text-xs text-stone-500 mt-0.5">{t.message}</p>
                </Show>
            </div>
            <div class="w-24 shrink-0">
                <div class="mb-0.5 flex justify-between text-[10px] text-stone-400">
                    <span>{t.current}/{t.total}</span>
                    <span>{taskPct}%</span>
                </div>
                <div class="h-1.5 w-full overflow-hidden rounded-full bg-stone-200">
                    <div
                        class={`h-full rounded-full transition-all ${progressBarColor(t.status)}`}
                        style={{ width: `${taskPct}%` }}
                    />
                </div>
            </div>
            <span class={`w-14 text-right text-xs font-semibold ${statusColor(t.status)}`}>
                {statusLabel(t.status)}
            </span>
        </div>
    );
}

function TaskGroupCard(props: {
    group: TaskGroup;
    onCancel: (id: string) => void;
    onRemove: (id: string) => void;
    cancelLoading: string | null;
    removeLoading: string | null;
}) {
    const [expanded, setExpanded] = createSignal(false);
    const g = props.group;
    const isActive =
        g.overallStatus === "Running" ||
        g.overallStatus === "Pending" ||
        g.overallStatus === "Paused";

    return (
        <div class="rounded-xl border border-stone-200 bg-white shadow-sm">
            {}
            <div
                class="flex w-full cursor-pointer items-center gap-4 px-5 py-4 text-left transition-colors hover:bg-stone-50/80"
                onClick={() => setExpanded((v) => !v)}
                onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setExpanded((v) => !v);
                    }
                }}
                role="button"
                tabindex={0}
            >
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                        <p class="truncate text-sm font-bold text-stone-900">{g.groupId}</p>
                        <span class={`text-xs font-semibold ${statusColor(g.overallStatus)}`}>
                            {statusLabel(g.overallStatus)}
                        </span>
                    </div>
                    <p class="mt-1 text-[11px] text-stone-400">
                        {g.completedTasks}/{g.totalTasks} 完成 · {g.progressPercent}% · 更新于{" "}
                        {new Date(g.updatedAt).toLocaleTimeString()}
                    </p>
                </div>
                <div class="w-28 shrink-0">
                    <div class="mb-0.5 flex justify-end text-[10px] text-stone-400">
                        {g.progressPercent}%
                    </div>
                    <div class="h-2 w-full overflow-hidden rounded-full bg-stone-200">
                        <div
                            class={`h-full rounded-full transition-all ${progressBarColor(g.overallStatus)}`}
                        style={{ width: `${g.progressPercent}%` }}
                        />
                    </div>
                </div>
                {}
                <div class="flex items-center gap-1.5" onClick={(e) => e.stopPropagation()}>
                    <Show when={isActive}>
                        <button
                            class="rounded-md border border-amber-300 bg-amber-50 px-2.5 py-1 text-xs font-medium text-amber-700 hover:bg-amber-100 disabled:opacity-50"
                            disabled={props.cancelLoading === g.groupId}
                            onClick={() => props.onCancel(g.groupId)}
                        >
                            {props.cancelLoading === g.groupId ? "标记中…" : "标记取消"}
                        </button>
                    </Show>
                    <button
                        class="rounded-md border border-red-200 bg-red-50 px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-100 disabled:opacity-50"
                        disabled={props.removeLoading === g.groupId}
                        onClick={() => props.onRemove(g.groupId)}
                    >
                        {props.removeLoading === g.groupId ? "…" : "移除"}
                    </button>
                </div>
                {}
                <span class="ml-1 text-xs text-stone-300">
                    {expanded() ? "▾" : "▸"}
                </span>
            </div>

            {}
            <Show when={expanded()}>
                <div class="border-t border-stone-100 px-5 pb-4 pt-3 space-y-2">
                    <For each={g.tasks}>
                        {(task) => <TaskProgressRow task={task} />}
                    </For>
                </div>
            </Show>
        </div>
    );
}

export function TasksPage() {
    const [groups, setGroups] = createSignal<TaskGroup[]>([]);
    const [error, setError] = createSignal<string | null>(null);
    const [cancelLoading, setCancelLoading] = createSignal<string | null>(null);
    const [removeLoading, setRemoveLoading] = createSignal<string | null>(null);
    const [actionError, setActionError] = createSignal<string | null>(null);


    const [snapshotJson, setSnapshotJson] = createSignal("");
    const [replaceExisting, setReplaceExisting] = createSignal(false);
    const [dropActive, setDropActive] = createSignal(true);
    const [importResult, setImportResult] = createSignal<ImportTaskSnapshotResult | null>(null);
    const [snapshotLoading, setSnapshotLoading] = createSignal(false);
    const [snapshotError, setSnapshotError] = createSignal<string | null>(null);
    const [showSnapshotPanel, setShowSnapshotPanel] = createSignal(false);

    const [statusFilter, setStatusFilter] = createSignal<StatusFilter>("all");
    const [keyword, setKeyword] = createSignal("");
    const [bulkRemoving, setBulkRemoving] = createSignal(false);

    let intervalId: ReturnType<typeof setInterval> | undefined;

    const filteredGroups = createMemo(() => {
        const gs = groups();
        return gs.filter(
            (g) =>
                statusMatchesFilter(g.overallStatus, statusFilter()) &&
                groupMatchesKeyword(g, keyword()),
        );
    });

    const counts = createMemo(() => {
        const gs = groups();
        let all = 0;
        let active = 0;
        let completed = 0;
        let failed = 0;
        let cancelled = 0;
        for (const g of gs) {
            all++;
            if (isActiveStatus(g.overallStatus)) active++;
            else if (g.overallStatus === "Completed") completed++;
            else if (g.overallStatus === "Failed") failed++;
            else if (g.overallStatus === "Cancelled") cancelled++;
        }
        return { all, active, completed, failed, cancelled };
    });

    async function refresh() {
        try {
            const gs = await listTaskGroups();
            setGroups(gs);
            setError(null);
        } catch (e) {
            setError(String(e));
        }
    }

    createEffect(() => {
        refresh();
        intervalId = setInterval(() => {
            if (!bulkRemoving()) {
                refresh();
            }
        }, 2000);
        onCleanup(() => {
            if (intervalId) clearInterval(intervalId);
        });
    });

    async function handleCancel(groupId: string) {
        if (
            !window.confirm(
                "您确定要标记取消该任务组吗？此操作不会强制停止后台下载，已开始的下载或安装可能仍会继续至自然结束。",
            )
        ) {
            return;
        }
        setCancelLoading(groupId);
        setActionError(null);
        try {
            await cancelTaskGroup(groupId);
            await refresh();
        } catch (e) {
            setActionError(String(e));
        } finally {
            setCancelLoading(null);
        }
    }

    async function handleRemove(groupId: string) {
        setRemoveLoading(groupId);
        setActionError(null);
        try {
            await removeTaskGroup(groupId);
            await refresh();
        } catch (e) {
            setActionError(String(e));
        } finally {
            setRemoveLoading(null);
        }
    }

    async function handleBulkRemove(
        label: string,
        matchFn: (g: TaskGroup) => boolean,
    ) {
        if (!window.confirm(`确定要批量${label}吗？此操作不可撤销。`)) return;
        setBulkRemoving(true);
        setActionError(null);
        let failedCount = 0;
        const targets = groups().filter(matchFn);
        for (const g of targets) {
            try {
                await removeTaskGroup(g.groupId);
            } catch {
                failedCount++;
            }
        }
        if (failedCount > 0) {
            setActionError(
                `批量${label}完成，${failedCount} 个任务组移除失败`,
            );
        }
        setBulkRemoving(false);
        await refresh();
    }

    async function handleExportSnapshot() {
        setSnapshotLoading(true);
        setSnapshotError(null);
        try {
            const json = await exportTaskSnapshot();
            setSnapshotJson(json);
            setImportResult(null);
        } catch (e) {
            setSnapshotError(String(e));
        } finally {
            setSnapshotLoading(false);
        }
    }

    async function handleImportSnapshot() {
        const json = snapshotJson().trim();
        if (!json) {
            setSnapshotError("请粘贴快照 JSON");
            return;
        }
        setSnapshotLoading(true);
        setSnapshotError(null);
        setImportResult(null);
        try {
            const result = await importTaskSnapshot({
                bundleJson: json,
                replaceExisting: replaceExisting(),
                dropActive: dropActive(),
            });
            setImportResult(result);
            await refresh();
        } catch (e) {
            setSnapshotError(String(e));
        } finally {
            setSnapshotLoading(false);
        }
    }

    const isBulkActionDisabled = () =>
        bulkRemoving() || groups().length === 0;

    return (
        <div class="flex h-full flex-col overflow-hidden">
            {}
            <div class="shrink-0 border-b border-stone-200 bg-stone-50/60 px-8 py-5">
                <h2 class="text-lg font-bold text-stone-900">任务</h2>
                <p class="mt-1 text-sm text-stone-500">
                    下载、安装、同步等任务进度
                </p>
            </div>

            {}
            <div class="shrink-0 border-b border-stone-100 bg-white px-8 py-4 space-y-3">
                {}
                <div class="flex flex-wrap items-center gap-4 text-xs text-stone-500">
                    <span>
                        全部{" "}
                        <span class="font-semibold text-stone-800">
                            {counts().all}
                        </span>
                    </span>
                    <span>
                        活跃{" "}
                        <span class="font-semibold text-blue-600">
                            {counts().active}
                        </span>
                    </span>
                    <span>
                        完成{" "}
                        <span class="font-semibold text-emerald-600">
                            {counts().completed}
                        </span>
                    </span>
                    <span>
                        失败{" "}
                        <span class="font-semibold text-red-600">
                            {counts().failed}
                        </span>
                    </span>
                    <span>
                        取消{" "}
                        <span class="font-semibold text-stone-400">
                            {counts().cancelled}
                        </span>
                    </span>
                </div>

                {}
                <div class="rounded-md border border-stone-200 bg-stone-50 px-3 py-2 text-xs text-stone-500">
                    标记取消只会终止任务中心进度显示，不保证中断已经开始的下载或安装。
                </div>

                {}
                <div class="flex flex-wrap items-center gap-2">
                    <For each={FILTER_OPTIONS}>
                        {(opt) => (
                            <button
                                class={`rounded-md px-3 py-1 text-xs font-medium transition-colors ${
                                    statusFilter() === opt.value
                                        ? "bg-stone-800 text-white"
                                        : "border border-stone-200 bg-white text-stone-600 hover:bg-stone-100"
                                }`}
                                onClick={() => setStatusFilter(opt.value)}
                            >
                                {opt.label}
                            </button>
                        )}
                    </For>
                </div>

                {}
                <div class="flex items-center gap-2">
                    <input
                        type="text"
                        class="w-full max-w-xs rounded-md border border-stone-200 bg-stone-50 px-3 py-1.5 text-sm text-stone-800 placeholder-stone-400 focus:border-stone-400 focus:outline-none"
                        placeholder="搜索任务组 / 任务名 / 消息"
                        value={keyword()}
                        onInput={(e) => setKeyword(e.currentTarget.value)}
                    />
                </div>

                {}
                <div class="flex flex-wrap items-center gap-2">
                    <button
                        class="rounded-md border border-emerald-300 bg-emerald-50 px-3 py-1 text-xs font-medium text-emerald-700 hover:bg-emerald-100 disabled:opacity-50"
                        disabled={isBulkActionDisabled()}
                        onClick={() =>
                            handleBulkRemove(
                                "清理已完成",
                                (g) => g.overallStatus === "Completed",
                            )
                        }
                    >
                        {bulkRemoving() ? "清理中…" : "清理已完成"}
                    </button>
                    <button
                        class="rounded-md border border-red-300 bg-red-50 px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-100 disabled:opacity-50"
                        disabled={isBulkActionDisabled()}
                        onClick={() =>
                            handleBulkRemove(
                                "清理失败/取消",
                                (g) =>
                                    g.overallStatus === "Failed" ||
                                    g.overallStatus === "Cancelled",
                            )
                        }
                    >
                        {bulkRemoving() ? "清理中…" : "清理失败/取消"}
                    </button>
                </div>

                {}
                <button
                    class="text-xs text-stone-500 hover:text-stone-700 underline"
                    onClick={() => setShowSnapshotPanel((v) => !v)}
                >
                    {showSnapshotPanel() ? "收起快照导入导出 ▴" : "快照导入导出 ▾"}
                </button>
            </div>

            {}
            <Show when={showSnapshotPanel()}>
                <div class="shrink-0 border-b border-stone-100 bg-amber-50/50 px-8 py-4 space-y-3">
                    <p class="text-xs text-stone-500">
                        快照不会恢复正在下载的后台任务；活跃任务会被跳过或标记为取消。
                    </p>

                    {}
                    <div class="space-y-2">
                        <div class="flex items-center gap-3">
                            <button
                                class="rounded-md border border-stone-300 bg-white px-3 py-1.5 text-xs font-medium text-stone-700 hover:bg-stone-100 disabled:opacity-50"
                                disabled={snapshotLoading()}
                                onClick={handleExportSnapshot}
                            >
                                {snapshotLoading() ? "导出中…" : "导出快照"}
                            </button>
                        </div>
                    </div>

                    {}
                    <div class="space-y-2">
                        <textarea
                            class="w-full rounded-md border border-stone-200 bg-white px-3 py-2 text-xs text-stone-800 font-mono placeholder-stone-400 focus:border-stone-400 focus:outline-none"
                            rows={6}
                            placeholder="粘贴快照 JSON…"
                            value={snapshotJson()}
                            onInput={(e) => {
                                setSnapshotJson(e.currentTarget.value);
                                setImportResult(null);
                                setSnapshotError(null);
                            }}
                        />
                        <div class="flex flex-wrap items-center gap-4">
                            <label class="flex items-center gap-1.5 text-xs text-stone-600">
                                <input
                                    type="checkbox"
                                    checked={replaceExisting()}
                                    onChange={(e) => setReplaceExisting(e.currentTarget.checked)}
                                    class="h-3.5 w-3.5 rounded border-stone-300"
                                />
                                覆盖已存在
                            </label>
                            <label class="flex items-center gap-1.5 text-xs text-stone-600">
                                <input
                                    type="checkbox"
                                    checked={dropActive()}
                                    onChange={(e) => setDropActive(e.currentTarget.checked)}
                                    class="h-3.5 w-3.5 rounded border-stone-300"
                                />
                                跳过活跃任务
                            </label>
                            <button
                                class="rounded-md border border-blue-300 bg-blue-50 px-3 py-1.5 text-xs font-medium text-blue-700 hover:bg-blue-100 disabled:opacity-50"
                                disabled={snapshotLoading() || !snapshotJson().trim()}
                                onClick={handleImportSnapshot}
                            >
                                {snapshotLoading() ? "导入中…" : "导入快照"}
                            </button>
                        </div>
                    </div>

                    {}
                    <Show when={snapshotError()}>
                        <div class="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
                            {snapshotError()}
                        </div>
                    </Show>

                    {}
                    <Show when={importResult()}>
                        <div class="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-800">
                            导入完成：已导入 {importResult()!.imported}，跳过{" "}
                            {importResult()!.skipped}，失败{" "}
                            {importResult()!.failed}，总计{" "}
                            {importResult()!.total}
                        </div>
                    </Show>
                </div>
            </Show>

            {}
            <Show when={error() || actionError()}>
                <div class="mx-8 mt-4 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
                    {error() && <p>{error()}</p>}
                    {actionError() && <p>{actionError()}</p>}
                </div>
            </Show>

            {}
            <div class="flex-1 overflow-auto px-8 py-5">
                <Show
                    when={groups().length > 0}
                    fallback={
                        <div class="flex h-40 items-center justify-center text-stone-400 text-sm">
                            暂无任务
                        </div>
                    }
                >
                    <Show
                        when={filteredGroups().length > 0}
                        fallback={
                            <div class="flex h-40 items-center justify-center text-stone-400 text-sm">
                                没有符合条件的任务
                            </div>
                        }
                    >
                        <div class="space-y-3">
                            <For each={filteredGroups()}>
                                {(group) => (
                                    <TaskGroupCard
                                        group={group}
                                        onCancel={handleCancel}
                                        onRemove={handleRemove}
                                        cancelLoading={cancelLoading()}
                                        removeLoading={removeLoading()}
                                    />
                                )}
                            </For>
                        </div>
                    </Show>
                </Show>
            </div>
        </div>
    );
}
