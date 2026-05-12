import { For, Show, createSignal, createMemo, createEffect, createResource, onCleanup } from "solid-js";
import { createQuery } from "@tanstack/solid-query";
import { listen } from "@tauri-apps/api/event";
import {
    listPeers,
    listInstances,
    resolveInstance,
    measureLatency,
    listLocalInstances,
    createLocalInstance,
    deleteLocalInstance,
    launchLocalInstance,
    launchGetState,
    retrieveInstanceWorkspace,
    setModEnabled,
    deleteModFile,
    exportModpackManifest,
    importModpackManifest,
    exportModpackZip,
    importModpackZip,
    type LocalInstance,
    type LocalInstanceKind,
    type LaunchLocalInstanceResult,
    type LaunchStateResponse,
    type InstanceWorkspaceInfo,
    type ExportModpackManifestResult,
    type ImportModpackManifestResult,
    type ModpackManifest,
    type ExportModpackZipResult,
    type ImportModpackZipResult,
} from "../services";
import {
    installClientVersionForInstance,
    installLibrariesForInstance,
    installAssetsForInstance,
    installLoaderForInstance,
    startInstallClientVersionTask,
    startInstallLibrariesTask,
    startInstallAssetsTask,
    startInstallLoaderTask,
    type InstallClientVersionRequest,
    type InstallClientVersionResult,
    type InstallLibrariesRequest,
    type InstallLibrariesResult,
    type InstallAssetsRequest,
    type InstallAssetsResult,
    type InstallLoaderRequest,
    type InstallLoaderResult,
    type InstallLoaderKind,
    type AsyncInstallClientVersionRequest,
    type AsyncInstallTaskStarted,
    type AsyncInstallLibrariesRequest,
    type AsyncInstallLibrariesStarted,
    type AsyncInstallAssetsRequest,
    type AsyncInstallAssetsStarted,
    type AsyncInstallLoaderRequest,
    type AsyncInstallLoaderStarted,
} from "../services/resource";
import { InstanceCard } from "../components/InstanceCard";
import { JoinDialog } from "../components/JoinDialog";
import { CreateRoomDialog } from "../components/CreateRoomDialog";
import { InviteDialog } from "../components/InviteDialog";
import type { Instance } from "../types";
import { getOnboardingStatus, type OnboardingStatus } from "../services/account";
import { canCreateFederatedRoom, roomCreationBlockedMessage } from "../utils/roomGuard";
import { createInstallOperation } from "../hooks/useInstallOperation";

const INSTANCE_KINDS: { label: string; value: LocalInstanceKind }[] = [
    { label: "Vanilla", value: "Vanilla" },
    { label: "Fabric", value: "Fabric" },
    { label: "Forge", value: "Forge" },
    { label: "NeoForge", value: "NeoForge" },
    { label: "Quilt", value: "Quilt" },
    { label: "Custom", value: "Custom" },
];

const TYPE_TABS = [
    { label: "全部", value: "all" },
    { label: "服务", value: "service" },
    { label: "房间", value: "room" },
] as const;

function toInstanceType(kind: string): "service" | "room" {
    return kind === "service" ? "service" : "room";
}

const STATUS_MAP: Record<string, string> = {
    running: "运行中", stopped: "已停止", migrating: "迁移中",
    degraded: "已降级", created: "已创建",
};
function mapStatus(status: string): string {
    return STATUS_MAP[status] ?? status;
}

export function ServersPage() {
    const [search, setSearch] = createSignal("");
    const [typeTab, setTypeTab] = createSignal<"all" | "service" | "room">("all");
    const [viewMode, setViewMode] = createSignal<"federated" | "local">("federated");
    const [joinInstance, setJoinInstance] = createSignal<Instance | null>(null);
    const [inviteInstance, setInviteInstance] = createSignal<Instance | null>(null);
    const [createRoomOpen, setCreateRoomOpen] = createSignal(false);
    const [latencies, setLatencies] = createSignal<Record<string, number | null>>({});
    const [createOpen, setCreateOpen] = createSignal(false);
    const [createName, setCreateName] = createSignal("");
    const [createVersion, setCreateVersion] = createSignal("");
    const [createKind, setCreateKind] = createSignal<LocalInstanceKind>("Vanilla");
    const [createError, setCreateError] = createSignal("");
    const [deleteConfirm, setDeleteConfirm] = createSignal<string | null>(null);
    const [deleteError, setDeleteError] = createSignal("");
    const [launchingId, setLaunchingId] = createSignal<string | null>(null);
    const [launchResult, setLaunchResult] = createSignal<LaunchLocalInstanceResult | null>(null);
    const [launchState, setLaunchState] = createSignal<LaunchStateResponse | null>(null);
    const [launchError, setLaunchError] = createSignal("");
    const [launchErrorFor, setLaunchErrorFor] = createSignal<string | null>(null);
    const [expandedInstance, setExpandedInstance] = createSignal<string | null>(null);
    const [workspaceData, setWorkspaceData] = createSignal<InstanceWorkspaceInfo | null>(null);
    const [workspaceError, setWorkspaceError] = createSignal("");
    const [workspaceErrorFor, setWorkspaceErrorFor] = createSignal<string | null>(null);
    const [workspaceLoading, setWorkspaceLoading] = createSignal(false);
    const [modOpLoading, setModOpLoading] = createSignal<string | null>(null);
    const [modOpError, setModOpError] = createSignal<{ key: string; error: string } | null>(null);
    const [modDeleteConfirm, setModDeleteConfirm] = createSignal<string | null>(null);
    const [actionPanelInstance, setActionPanelInstance] = createSignal<string | null>(null);


    const [exportResult, setExportResult] = createSignal<ExportModpackManifestResult | null>(null);
    const [exportError, setExportError] = createSignal("");
    const [exportLoading, setExportLoading] = createSignal(false);
    const [importText, setImportText] = createSignal("");
    const [importOverwrite, setImportOverwrite] = createSignal(false);
    const [importResult, setImportResult] = createSignal<ImportModpackManifestResult | null>(null);
    const [importError, setImportError] = createSignal("");
    const [importLoading, setImportLoading] = createSignal(false);


    const [zipExportPath, setZipExportPath] = createSignal("");
    const [zipExportResult, setZipExportResult] = createSignal<ExportModpackZipResult | null>(null);
    const [zipExportError, setZipExportError] = createSignal("");
    const [zipExportLoading, setZipExportLoading] = createSignal(false);
    const [zipImportPath, setZipImportPath] = createSignal("");
    const [zipImportOverwrite, setZipImportOverwrite] = createSignal(false);
    const [zipImportResult, setZipImportResult] = createSignal<ImportModpackZipResult | null>(null);
    const [zipImportError, setZipImportError] = createSignal("");
    const [zipImportLoading, setZipImportLoading] = createSignal(false);


    const installClientOp = createInstallOperation<InstallClientVersionResult>();
    const [installOverwrite, setInstallOverwrite] = createSignal<Record<string, boolean>>({});


    const asyncInstallClientOp = createInstallOperation<AsyncInstallTaskStarted>();


    const installLibOp = createInstallOperation<InstallLibrariesResult>();
    const [libInstallOverwrite, setLibInstallOverwrite] = createSignal<Record<string, boolean>>({});


    const asyncInstallLibOp = createInstallOperation<AsyncInstallLibrariesStarted>();


    const installAssetOp = createInstallOperation<InstallAssetsResult>();
    const [assetInstallOverwrite, setAssetInstallOverwrite] = createSignal<Record<string, boolean>>({});


    const asyncInstallAssetOp = createInstallOperation<AsyncInstallAssetsStarted>();


    const installLoaderOp = createInstallOperation<InstallLoaderResult>();
    const [loaderInstallOverwrite, setLoaderInstallOverwrite] = createSignal<Record<string, boolean>>({});


    const asyncInstallLoaderOp = createInstallOperation<AsyncInstallLoaderStarted>();

    const instancesQuery = createQuery(() => ({
        queryKey: ["instances"],
        queryFn: listInstances,
        refetchInterval: 10000,
    }));




    createEffect(() => {
        const unlisten = listen("instances-changed", () => {
            instancesQuery.refetch();
        });
        onCleanup(() => {
            unlisten.then((fn) => fn());
        });
    });

    const peers = createQuery(() => ({
        queryKey: ["peers"],
        queryFn: listPeers,
        refetchInterval: 60000,
    }));

    const onboarding = createQuery<OnboardingStatus>(() => ({
        queryKey: ["onboarding-status"],
        queryFn: getOnboardingStatus,
        staleTime: 30_000,
    }));

    const [localInstances, { refetch: refetchLocal }] = createResource(
        () => viewMode() === "local",
        async (active) => {
            if (!active) return [] as LocalInstance[];
            return listLocalInstances();
        },
    );

    async function handleCreateLocal() {
        setCreateError("");
        const name = createName().trim();
        const version = createVersion().trim();
        if (!name) {
            setCreateError("实例名称不能为空");
            return;
        }
        if (!version) {
            setCreateError("游戏版本不能为空");
            return;
        }
        try {
            await createLocalInstance({
                name,
                game_version: version,
                kind: createKind(),
            });
            setCreateOpen(false);
            setCreateName("");
            setCreateVersion("");
            setCreateKind("Vanilla");
            refetchLocal();
        } catch (e) {
            setCreateError(String(e));
        }
    }

    async function handleDeleteLocal(id: string) {
        setDeleteError("");
        try {
            await deleteLocalInstance(id);
            setDeleteConfirm(null);
            refetchLocal();
        } catch (e) {
            setDeleteError(String(e));
        }
    }

    async function handleLaunchLocal(inst: LocalInstance) {
        setLaunchingId(inst.id);
        setLaunchResult(null);
        setLaunchState(null);
        setLaunchError("");
        setLaunchErrorFor(null);
        try {
            const result = await launchLocalInstance(inst.id);
            setLaunchResult(result);
            try {
                const state = await launchGetState(result.launching_id);
                setLaunchState(state);
            } catch {

            }
        } catch (e) {
            setLaunchError(String(e));
            setLaunchErrorFor(inst.id);
        } finally {
            installClientOp.setLoading(null);
        }
    }

    async function handleAsyncInstallClientVersion(inst: LocalInstance) {
        await asyncInstallClientOp.execute(inst.id, () =>
            startInstallClientVersionTask({
                instanceId: inst.id,
                overwrite: installOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleInstallLibraries(inst: LocalInstance) {
        await installLibOp.execute(inst.id, () =>
            installLibrariesForInstance({
                instanceId: inst.id,
                overwrite: libInstallOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleAsyncInstallLibraries(inst: LocalInstance) {
        await asyncInstallLibOp.execute(inst.id, () =>
            startInstallLibrariesTask({
                instanceId: inst.id,
                overwrite: libInstallOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleInstallAssets(inst: LocalInstance) {
        await installAssetOp.execute(inst.id, () =>
            installAssetsForInstance({
                instanceId: inst.id,
                overwrite: assetInstallOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleAsyncInstallAssets(inst: LocalInstance) {
        await asyncInstallAssetOp.execute(inst.id, () =>
            startInstallAssetsTask({
                instanceId: inst.id,
                overwrite: assetInstallOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleInstallLoader(inst: LocalInstance, kind: InstallLoaderKind) {
        const loadingKey = `${inst.id}|${kind}`;
        await installLoaderOp.execute(loadingKey, () =>
            installLoaderForInstance({
                instanceId: inst.id,
                kind,
                loaderVersion: null,
                overwrite: loaderInstallOverwrite()[inst.id] ?? false,
            })
        );
        refetchLocal();
    }

    async function handleAsyncInstallLoader(inst: LocalInstance, kind: InstallLoaderKind) {
        const loadingKey = `${inst.id}|${kind}`;
        await asyncInstallLoaderOp.execute(loadingKey, () =>
            startInstallLoaderTask({
                instanceId: inst.id,
                kind,
                overwrite: loaderInstallOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function handleInstallClientVersion(inst: LocalInstance) {
        await installClientOp.execute(inst.id, () =>
            installClientVersionForInstance({
                instanceId: inst.id,
                overwrite: installOverwrite()[inst.id] ?? false,
            })
        );
    }

    async function toggleWorkspace(instId: string) {
        if (expandedInstance() === instId) {
            setExpandedInstance(null);
            setWorkspaceData(null);
            setWorkspaceError("");
            setWorkspaceErrorFor(null);
            setExportResult(null);
            setExportError("");
            setImportText("");
            setImportResult(null);
            setImportError("");
            return;
        }
        setExpandedInstance(instId);
        setWorkspaceData(null);
        setWorkspaceError("");
        setWorkspaceErrorFor(null);
        setExportResult(null);
        setExportError("");
        setImportText("");
        setImportResult(null);
        setImportError("");
        setWorkspaceLoading(true);
        try {
            const data = await retrieveInstanceWorkspace(instId);
            setWorkspaceData(data);
        } catch (e) {
            setWorkspaceError(String(e));
            setWorkspaceErrorFor(instId);
        } finally {
            setWorkspaceLoading(false);
        }
    }

    async function handleModToggle(instId: string, fileName: string, currentlyEnabled: boolean) {
        const key = `${instId}|${fileName}`;
        setModOpLoading(key);
        setModOpError(null);
        setModDeleteConfirm(null);
        try {
            await setModEnabled(instId, fileName, !currentlyEnabled);
            const data = await retrieveInstanceWorkspace(instId);
            setWorkspaceData(data);
        } catch (e) {
            setModOpError({ key, error: String(e) });
        } finally {
            setModOpLoading(null);
        }
    }

    async function handleModDelete(instId: string, fileName: string) {
        const key = `${instId}|${fileName}`;
        setModOpLoading(key);
        setModOpError(null);
        setModDeleteConfirm(null);
        try {
            await deleteModFile(instId, fileName);
            const data = await retrieveInstanceWorkspace(instId);
            setWorkspaceData(data);
        } catch (e) {
            setModOpError({ key, error: String(e) });
        } finally {
            setModOpLoading(null);
        }
    }


    async function handleExportManifest(instId: string) {
        setExportLoading(true);
        setExportResult(null);
        setExportError("");
        try {
            const result = await exportModpackManifest(instId);
            setExportResult(result);
        } catch (e) {
            setExportError(String(e));
        } finally {
            setExportLoading(false);
        }
    }

    async function handleImportManifest(instId: string) {
        const text = importText().trim();
        if (!text) {
            setImportError("请输入清单 JSON");
            return;
        }
        let manifest: ModpackManifest;
        try {
            manifest = JSON.parse(text) as ModpackManifest;
        } catch {
            setImportError("JSON 解析失败");
            return;
        }
        setImportLoading(true);
        setImportResult(null);
        setImportError("");
        try {
            const result = await importModpackManifest({
                target_instance_id: instId,
                manifest,
                overwrite: importOverwrite(),
            });
            setImportResult(result);

            const data = await retrieveInstanceWorkspace(instId);
            setWorkspaceData(data);
        } catch (e) {
            setImportError(String(e));
        } finally {
            setImportLoading(false);
        }
    }


    async function handleExportZip(instId: string) {
        const path = zipExportPath().trim();
        if (!path) {
            setZipExportError("请输入输出路径");
            return;
        }
        setZipExportLoading(true);
        setZipExportResult(null);
        setZipExportError("");
        try {
            const result = await exportModpackZip({
                instanceId: instId,
                outputPath: path,
            });
            setZipExportResult(result);
        } catch (e) {
            setZipExportError(String(e));
        } finally {
            setZipExportLoading(false);
        }
    }

    async function handleImportZip(instId: string) {
        const path = zipImportPath().trim();
        if (!path) {
            setZipImportError("请输入 ZIP 文件路径");
            return;
        }
        setZipImportLoading(true);
        setZipImportResult(null);
        setZipImportError("");
        try {
            const result = await importModpackZip({
                targetInstanceId: instId,
                zipPath: path,
                overwrite: zipImportOverwrite(),
            });
            setZipImportResult(result);

            const data = await retrieveInstanceWorkspace(instId);
            setWorkspaceData(data);
        } catch (e) {
            setZipImportError(String(e));
        } finally {
            setZipImportLoading(false);
        }
    }

    const instanceList = createMemo((): Instance[] => {
        const p2p = instancesQuery.data ?? [];
        const lat = latencies();
        return p2p.map((info) => ({
            id: info.id,
            name: info.name,
            mode: info.mode,
            club: info.club,
            type: toInstanceType(info.kind),
            players: `${info.players}/${info.max_players}`,
            latency: lat[info.id] ?? null,
            state: mapStatus(info.status),
            version: info.version,
            peer_id: info.peer_id,
        }));
    });

    async function measureAllLatencies() {
        const p2p = instancesQuery.data ?? [];
        const targets = p2p.filter((i) => i.peer_id);
        const limit = 8;
        for (let i = 0; i < targets.length; i += limit) {
            const chunk = targets.slice(i, i + limit);
            const results = await Promise.all(
                chunk.map(async (info) => {
                    try {
                        const ms = await measureLatency(info.peer_id);
                        const result: { id: string; ms: number | null } = { id: info.id, ms };
                        return result;
                    } catch {
                        const result: { id: string; ms: number | null } = { id: info.id, ms: null };
                        return result;
                    }
                })
            );
            setLatencies((prev) => {
                const next = { ...prev };
                for (const r of results) {
                    next[r.id] = r.ms;
                }
                return next;
            });
        }
    }

    createEffect((prevIds: string | undefined) => {
        const ids = instancesQuery.data?.map((i) => i.id).sort().join(",");
        if (ids && ids !== prevIds) {
            measureAllLatencies();
        }
        return ids;
    });

    const filtered = createMemo(() => {
        let list = instanceList();
        const q = search().trim().toLowerCase();
        if (q) {
            list = list.filter((i) =>
                i.name.toLowerCase().includes(q) ||
                i.mode.toLowerCase().includes(q) ||
                i.club.toLowerCase().includes(q)
            );
        }
        const tab = typeTab();
        if (tab !== "all") list = list.filter((i) => i.type === tab);
        return list;
    });

    async function probeInstance(id: string) {
        try { await resolveInstance(id); } catch {  }
    }

    const isLoading = createMemo(() => instancesQuery.isLoading);

    return (
        <div class="flex h-full flex-col">
            <header class="border-b border-stone-200 px-8 py-5">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <h2 class="text-2xl font-black text-stone-950">实例列表</h2>
                        <div class="flex overflow-hidden rounded-lg border border-stone-300 bg-white">
                            <button
                                type="button"
                                class={`h-8 px-4 text-sm font-medium transition-colors ${viewMode() === "federated"
                                        ? "bg-teal-800 text-white"
                                        : "text-stone-600 hover:bg-stone-100"
                                    }`}
                                onClick={() => setViewMode("federated")}
                            >
                                联邦实例
                            </button>
                            <button
                                type="button"
                                class={`h-8 px-4 text-sm font-medium transition-colors ${viewMode() === "local"
                                        ? "bg-teal-800 text-white"
                                        : "text-stone-600 hover:bg-stone-100"
                                    }`}
                                onClick={() => setViewMode("local")}
                            >
                                本地实例
                            </button>
                        </div>
                    </div>
                    <Show when={viewMode() === "federated"}>
                        <div class="flex items-center gap-3 text-sm text-stone-500">
                            <Show
                                when={onboarding.data && canCreateFederatedRoom(onboarding.data)}
                                fallback={
                                    <button
                                        class="btn rounded-lg px-4 py-2 text-sm opacity-60 cursor-not-allowed"
                                        style="background-color: #d1d5db; color: #6b7280;"
                                        disabled
                                        title={
                                            onboarding.data
                                                ? (roomCreationBlockedMessage(onboarding.data) ?? "")
                                                : "正在检查身份…"
                                        }
                                    >
                                        + 创建房间（需社团 VC）
                                    </button>
                                }
                            >
                                <button
                                    class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900"
                                    onClick={() => setCreateRoomOpen(true)}
                                >
                                    + 创建房间
                                </button>
                            </Show>
                            <Show when={onboarding.data && !canCreateFederatedRoom(onboarding.data)}>
                                <span class="text-amber-700 bg-amber-50 px-2 py-1 rounded text-xs">
                                    {roomCreationBlockedMessage(onboarding.data!)}
                                </span>
                            </Show>
                            <Show when={isLoading()}>
                                <span class="loading loading-spinner loading-xs" />
                            </Show>
                            <span>{filtered().length} 个实例</span>
                            <span class="text-stone-300">·</span>
                            <span>{peers.data ? `${peers.data.length} 个节点` : "节点数不可用"}</span>
                        </div>
                    </Show>
                    <Show when={viewMode() === "local"}>
                        <div class="flex items-center gap-3 text-sm text-stone-500">
                            <button
                                class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900"
                                onClick={() => setCreateOpen(true)}
                            >
                                + 创建本地实例
                            </button>
                            <span>{(localInstances() ?? []).length} 个本地实例</span>
                        </div>
                    </Show>
                </div>

                <Show when={viewMode() === "federated"}>
                    <div class="mt-4 flex gap-3">
                        <input
                            class="input input-bordered h-9 w-72 rounded-lg border-stone-300 bg-white text-sm"
                            placeholder="搜索名称、模式或社团…"
                            value={search()}
                            onInput={(e) => setSearch(e.currentTarget.value)}
                        />
                        <div class="flex overflow-hidden rounded-lg border border-stone-300 bg-white">
                            <For each={TYPE_TABS}>
                                {(tab) => (
                                    <button
                                        type="button"
                                        class={`h-9 px-4 text-sm font-medium transition-colors ${typeTab() === tab.value
                                                ? "bg-teal-800 text-white"
                                                : "text-stone-600 hover:bg-stone-100"
                                            }`}
                                        onClick={() => setTypeTab(tab.value)}
                                    >
                                        {tab.label}
                                    </button>
                                )}
                            </For>
                        </div>
                    </div>
                </Show>
            </header>

            <Show when={viewMode() === "federated"}>
                <div class="flex-1 overflow-y-auto">
                    <Show when={!isLoading() && filtered().length === 0}>
                        <div class="flex h-40 items-center justify-center text-stone-400">
                            暂无匹配的实例
                        </div>
                    </Show>

                    <div class="divide-y divide-stone-100">
                        <For each={filtered()}>
                            {(instance) => (
                                <InstanceCard
                                    instance={instance}
                                    onJoin={() => {
                                        setJoinInstance(instance);
                                        probeInstance(instance.id);
                                    }}
                                    onInvite={() => {
                                        setInviteInstance(instance);
                                    }}
                                />
                            )}
                        </For>
                    </div>
                </div>
            </Show>

            <Show when={viewMode() === "local"}>
                <div class="flex-1 overflow-y-auto">
                    <Show when={(localInstances() ?? []).length === 0}>
                        <div class="flex h-40 items-center justify-center text-stone-400">
                            暂无本地实例，点击上方按钮创建
                        </div>
                    </Show>

                    <div class="divide-y divide-stone-100">
                        <For each={localInstances() ?? []}>
                            {(inst) => (
                                <>
                                    <article class="grid grid-cols-[1fr_100px_100px_260px] items-center border-b border-stone-200 px-5 py-4 last:border-b-0">
                                        <div>
                                            <div class="flex items-center gap-3">
                                                <h3 class="text-lg font-bold">{inst.name}</h3>
                                                <span class="badge rounded badge-outline text-xs">
                                                    {inst.kind}
                                                </span>
                                                <span class="text-xs text-stone-400">{inst.game_version}</span>
                                            </div>
                                            <p class="mt-1 text-xs text-stone-500 truncate max-w-md" title={inst.game_dir}>
                                                {inst.game_dir}
                                            </p>
                                            <p class="mt-0.5 text-xs text-stone-400">
                                                {inst.last_played_at
                                                    ? `上次游玩: ${new Date(inst.last_played_at).toLocaleString()}`
                                                    : "未曾游玩"}
                                            </p>
                                            <Show when={launchResult()?.instance_id === inst.id}>
                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                    <span class="font-semibold">启动成功</span>
                                                    {" — PID: "}{launchResult()?.pid}
                                                    {" | "}{launchResult()?.username}
                                                    {" | Java: "}{launchResult()?.java_path}
                                                </div>
                                            </Show>
                                            <Show when={launchState() && launchResult()?.instance_id === inst.id}>
                                                <div class="mt-1 rounded bg-blue-50 px-2 py-1 text-xs text-blue-700">
                                                    <span class="font-semibold">进程状态</span>
                                                    {" — 步骤: "}{launchState()?.step}
                                                    {" | 就绪: "}{launchState()?.game_ready ? "是" : "否"}
                                                    {launchState()?.exit_code != null ? ` | 退出码: ${launchState()?.exit_code}` : ""}
                                                </div>
                                            </Show>
                                            <Show when={launchErrorFor() === inst.id && launchError()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {launchError()}
                                                </div>
                                            </Show>
                                            <Show when={installClientOp.result() && installClientOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                    <span class="font-semibold">版本安装完成</span>
                                                    {" — JSON: "}{installClientOp.result()!.jsonBytesWritten > 1024
                                                        ? `${(installClientOp.result()!.jsonBytesWritten / 1024).toFixed(1)} KB`
                                                        : `${installClientOp.result()!.jsonBytesWritten} B`}
                                                    {" | JAR: "}{installClientOp.result()!.jarBytesWritten > 1024
                                                        ? `${(installClientOp.result()!.jarBytesWritten / 1024).toFixed(1)} KB`
                                                        : `${installClientOp.result()!.jarBytesWritten} B`}
                                                    {" | SHA1: "}{installClientOp.result()!.jarSha1Verified ? "通过" : "未校验"}
                                                    <Show when={installClientOp.result()!.replacedExisting}>
                                                        <span class="text-amber-600"> (已覆盖)</span>
                                                    </Show>
                                                </div>
                                            </Show>
                                            <Show when={installClientOp.errorFor() === inst.id && installClientOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {installClientOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallClientOp.result() && asyncInstallClientOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-blue-50 px-2 py-1 text-xs text-blue-700">
                                                    <span class="font-semibold">已在后台启动安装</span>
                                                    {" — 任务组: "}{asyncInstallClientOp.result()?.groupId}
                                                    {"，可前往「任务」页面查看进度"}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallClientOp.errorFor() === inst.id && asyncInstallClientOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {asyncInstallClientOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={installLibOp.result() && installLibOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                    <span class="font-semibold">库文件安装完成</span>
                                                    {" — 版本: "}{installLibOp.result()!.gameVersion}
                                                    {" | 扫描: "}{installLibOp.result()!.scanned}
                                                    {" | 下载: "}{installLibOp.result()!.downloaded}
                                                    {" | 跳过: "}{installLibOp.result()!.skipped}
                                                    <Show when={installLibOp.result()!.failed > 0}>
                                                        <span class="text-red-600">{" | 失败: "}{installLibOp.result()!.failed}</span>
                                                    </Show>
                                                    {" | 写入: "}{installLibOp.result()!.bytesWritten > 1024
                                                        ? `${(installLibOp.result()!.bytesWritten / 1024).toFixed(1)} KB`
                                                        : `${installLibOp.result()!.bytesWritten} B`}
                                                </div>
                                            </Show>
                                            <Show when={installLibOp.errorFor() === inst.id && installLibOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {installLibOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallLibOp.result() && asyncInstallLibOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-blue-50 px-2 py-1 text-xs text-blue-700">
                                                    <span class="font-semibold">已在后台启动运行库安装</span>
                                                    {" — 任务组: "}{asyncInstallLibOp.result()?.groupId}
                                                    {"，可前往「任务」页面查看进度"}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallLibOp.errorFor() === inst.id && asyncInstallLibOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {asyncInstallLibOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={installAssetOp.result() && installAssetOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                    <span class="font-semibold">资源文件安装完成</span>
                                                    {" — 版本: "}{installAssetOp.result()!.gameVersion}
                                                    {" | Index: "}{installAssetOp.result()!.assetIndexId}
                                                    {" | 扫描: "}{installAssetOp.result()!.scanned}
                                                    {" | 下载: "}{installAssetOp.result()!.downloaded}
                                                    {" | 跳过: "}{installAssetOp.result()!.skipped}
                                                    <Show when={installAssetOp.result()!.failed > 0}>
                                                        <span class="text-red-600">{" | 失败: "}{installAssetOp.result()!.failed}</span>
                                                    </Show>
                                                    {" | 写入: "}{installAssetOp.result()!.bytesWritten > 1024
                                                        ? `${(installAssetOp.result()!.bytesWritten / 1024).toFixed(1)} KB`
                                                        : `${installAssetOp.result()!.bytesWritten} B`}
                                                </div>
                                            </Show>
                                            <Show when={installAssetOp.errorFor() === inst.id && installAssetOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {installAssetOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallAssetOp.result() && asyncInstallAssetOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-blue-50 px-2 py-1 text-xs text-blue-700">
                                                    <span class="font-semibold">已在后台启动资源文件安装</span>
                                                    {" — 任务组: "}{asyncInstallAssetOp.result()?.groupId}
                                                    {"，可前往「任务」页面查看进度"}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallAssetOp.errorFor() === inst.id && asyncInstallAssetOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {asyncInstallAssetOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={installLoaderOp.result() && installLoaderOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                    <span class="font-semibold">Loader 安装完成</span>
                                                    {" — 原版本: "}{installLoaderOp.result()!.previousGameVersion}
                                                    {" → 新版本: "}{installLoaderOp.result()!.newGameVersion}
                                                    {" | Loader: "}{installLoaderOp.result()!.loaderVersion}
                                                    {" | 写入: "}{installLoaderOp.result()!.bytesWritten > 1024
                                                        ? `${(installLoaderOp.result()!.bytesWritten / 1024).toFixed(1)} KB`
                                                        : `${installLoaderOp.result()!.bytesWritten} B`}
                                                    <Show when={installLoaderOp.result()!.replacedExisting}>
                                                        <span class="text-amber-600"> (已覆盖)</span>
                                                    </Show>
                                                </div>
                                            </Show>
                                            <Show when={installLoaderOp.errorFor()?.startsWith(`${inst.id}|`) && installLoaderOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {installLoaderOp.error()}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallLoaderOp.result() && asyncInstallLoaderOp.result()!.instanceId === inst.id}>
                                                <div class="mt-1 rounded bg-blue-50 px-2 py-1 text-xs text-blue-700">
                                                    <span class="font-semibold">已在后台启动 Loader 安装</span>
                                                    {" — 任务组: "}{asyncInstallLoaderOp.result()?.groupId}
                                                    {"，可前往「任务」页面查看进度"}
                                                </div>
                                            </Show>
                                            <Show when={asyncInstallLoaderOp.errorFor()?.startsWith(`${inst.id}|`) && asyncInstallLoaderOp.error()}>
                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">
                                                    {asyncInstallLoaderOp.error()}
                                                </div>
                                            </Show>
                                        </div>
                                        <p class="text-sm text-stone-500">
                                            {new Date(inst.created_at).toLocaleDateString()}
                                        </p>
                                        <p class="text-sm text-stone-500">
                                            {new Date(inst.updated_at).toLocaleDateString()}
                                        </p>
                                        <div class="flex items-start gap-1 flex-wrap">
                                            <button
                                                class="btn btn-xs rounded bg-teal-800 text-white hover:bg-teal-900 disabled:opacity-50"
                                                disabled={launchingId() === inst.id}
                                                onClick={() => handleLaunchLocal(inst)}
                                            >
                                                {launchingId() === inst.id ? "启动中…" : "启动"}
                                            </button>
                                            <button
                                                class="btn btn-xs rounded border border-stone-300 bg-white text-stone-600 hover:bg-stone-100"
                                                onClick={() => toggleWorkspace(inst.id)}
                                            >
                                                {expandedInstance() === inst.id ? "收起" : "详情"}
                                            </button>
                                            <button
                                                class="btn btn-xs rounded border border-stone-300 bg-white text-stone-600 hover:bg-stone-100"
                                                onClick={() => setActionPanelInstance(actionPanelInstance() === inst.id ? null : inst.id)}
                                            >
                                                {actionPanelInstance() === inst.id ? "收起操作" : "更多操作"}
                                            </button>
                                            <Show when={deleteConfirm() === inst.id}>
                                                <div class="flex flex-col gap-1">
                                                    <span class="text-xs text-amber-600">仅删除记录，不删除文件</span>
                                                    <Show when={deleteError()}>
                                                        <span class="text-xs text-red-600">{deleteError()}</span>
                                                    </Show>
                                                    <div class="flex gap-1">
                                                        <button
                                                            class="btn btn-xs rounded bg-red-600 text-white hover:bg-red-700"
                                                            onClick={() => handleDeleteLocal(inst.id)}
                                                        >
                                                            确认
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded border border-stone-300 bg-white text-stone-600"
                                                            onClick={() => { setDeleteConfirm(null); setDeleteError(""); }}
                                                        >
                                                            取消
                                                        </button>
                                                    </div>
                                                </div>
                                            </Show>
                                            <Show when={deleteConfirm() !== inst.id}>
                                                <button
                                                    class="btn btn-xs rounded border border-red-300 bg-white text-red-600 hover:bg-red-50"
                                                    onClick={() => setDeleteConfirm(inst.id)}
                                                >
                                                    删除
                                                </button>
                                            </Show>
                                        </div>
                                    </article>
                                    <Show when={actionPanelInstance() === inst.id}>
                                        <div class="border-b border-stone-200 bg-stone-50 px-5 py-4">
                                            <div class="space-y-3">
                                                <div>
                                                    <h4 class="mb-1.5 text-xs font-semibold text-stone-500">版本文件</h4>
                                                    <div class="flex items-center gap-2 flex-wrap">
                                                        <button
                                                            class="btn btn-xs rounded bg-amber-700 text-white hover:bg-amber-800 disabled:opacity-50"
                                                            disabled={installClientOp.loading() === inst.id}
                                                            onClick={() => handleInstallClientVersion(inst)}
                                                        >
                                                            {installClientOp.loading() === inst.id ? "安装中…" : "安装版本"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-amber-600 text-white hover:bg-amber-700 disabled:opacity-50"
                                                            disabled={asyncInstallClientOp.loading() === inst.id}
                                                            onClick={() => handleAsyncInstallClientVersion(inst)}
                                                        >
                                                            {asyncInstallClientOp.loading() === inst.id ? "启动中…" : "后台安装版本"}
                                                        </button>
                                                        <label class="flex items-center gap-1 text-[11px] text-stone-400 cursor-pointer select-none">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                checked={installOverwrite()[inst.id] ?? false}
                                                                onChange={(e) => setInstallOverwrite({ ...installOverwrite(), [inst.id]: e.currentTarget.checked })}
                                                            />
                                                            覆盖安装
                                                        </label>
                                                    </div>
                                                </div>
                                                <div>
                                                    <h4 class="mb-1.5 text-xs font-semibold text-stone-500">运行依赖</h4>
                                                    <div class="flex items-center gap-2 flex-wrap">
                                                        <button
                                                            class="btn btn-xs rounded bg-indigo-700 text-white hover:bg-indigo-800 disabled:opacity-50"
                                                            disabled={installLibOp.loading() === inst.id}
                                                            onClick={() => handleInstallLibraries(inst)}
                                                        >
                                                            {installLibOp.loading() === inst.id ? "修复中…" : "修复库文件"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-indigo-500 text-white hover:bg-indigo-600 disabled:opacity-50"
                                                            disabled={asyncInstallLibOp.loading() === inst.id}
                                                            onClick={() => handleAsyncInstallLibraries(inst)}
                                                        >
                                                            {asyncInstallLibOp.loading() === inst.id ? "启动中…" : "后台修复库文件"}
                                                        </button>
                                                        <label class="flex items-center gap-1 text-[11px] text-stone-400 cursor-pointer select-none">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                checked={libInstallOverwrite()[inst.id] ?? false}
                                                                onChange={(e) => setLibInstallOverwrite({ ...libInstallOverwrite(), [inst.id]: e.currentTarget.checked })}
                                                            />
                                                            覆盖库文件
                                                        </label>
                                                        <button
                                                            class="btn btn-xs rounded bg-purple-700 text-white hover:bg-purple-800 disabled:opacity-50"
                                                            disabled={installAssetOp.loading() === inst.id}
                                                            onClick={() => handleInstallAssets(inst)}
                                                        >
                                                            {installAssetOp.loading() === inst.id ? "修复中…" : "修复资源文件"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-purple-500 text-white hover:bg-purple-600 disabled:opacity-50"
                                                            disabled={asyncInstallAssetOp.loading() === inst.id}
                                                            onClick={() => handleAsyncInstallAssets(inst)}
                                                        >
                                                            {asyncInstallAssetOp.loading() === inst.id ? "启动中…" : "后台修复资源文件"}
                                                        </button>
                                                        <label class="flex items-center gap-1 text-[11px] text-stone-400 cursor-pointer select-none">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                checked={assetInstallOverwrite()[inst.id] ?? false}
                                                                onChange={(e) => setAssetInstallOverwrite({ ...assetInstallOverwrite(), [inst.id]: e.currentTarget.checked })}
                                                            />
                                                            覆盖资源文件
                                                        </label>
                                                    </div>
                                                </div>
                                                <div>
                                                    <h4 class="mb-1.5 text-xs font-semibold text-stone-500">Loader</h4>
                                                    <div class="flex items-center gap-2 flex-wrap">
                                                        <button
                                                            class="btn btn-xs rounded bg-cyan-700 text-white hover:bg-cyan-800 disabled:opacity-50"
                                                            disabled={installLoaderOp.loading() === `${inst.id}|Fabric`}
                                                            onClick={() => handleInstallLoader(inst, "Fabric")}
                                                        >
                                                            {installLoaderOp.loading() === `${inst.id}|Fabric` ? "安装中…" : "安装 Fabric"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-cyan-600 text-white hover:bg-cyan-700 disabled:opacity-50"
                                                            disabled={asyncInstallLoaderOp.loading() === `${inst.id}|Fabric`}
                                                            onClick={() => handleAsyncInstallLoader(inst, "Fabric")}
                                                        >
                                                            {asyncInstallLoaderOp.loading() === `${inst.id}|Fabric` ? "启动中…" : "后台安装 Fabric"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-sky-700 text-white hover:bg-sky-800 disabled:opacity-50"
                                                            disabled={installLoaderOp.loading() === `${inst.id}|Quilt`}
                                                            onClick={() => handleInstallLoader(inst, "Quilt")}
                                                        >
                                                            {installLoaderOp.loading() === `${inst.id}|Quilt` ? "安装中…" : "安装 Quilt"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-sky-600 text-white hover:bg-sky-700 disabled:opacity-50"
                                                            disabled={asyncInstallLoaderOp.loading() === `${inst.id}|Quilt`}
                                                            onClick={() => handleAsyncInstallLoader(inst, "Quilt")}
                                                        >
                                                            {asyncInstallLoaderOp.loading() === `${inst.id}|Quilt` ? "启动中…" : "后台安装 Quilt"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-orange-700 text-white hover:bg-orange-800 disabled:opacity-50"
                                                            disabled={installLoaderOp.loading() === `${inst.id}|Forge`}
                                                            onClick={() => handleInstallLoader(inst, "Forge")}
                                                        >
                                                            {installLoaderOp.loading() === `${inst.id}|Forge` ? "安装中…" : "安装 Forge"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-orange-600 text-white hover:bg-orange-700 disabled:opacity-50"
                                                            disabled={asyncInstallLoaderOp.loading() === `${inst.id}|Forge`}
                                                            onClick={() => handleAsyncInstallLoader(inst, "Forge")}
                                                        >
                                                            {asyncInstallLoaderOp.loading() === `${inst.id}|Forge` ? "启动中…" : "后台安装 Forge"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-emerald-700 text-white hover:bg-emerald-800 disabled:opacity-50"
                                                            disabled={installLoaderOp.loading() === `${inst.id}|NeoForge`}
                                                            onClick={() => handleInstallLoader(inst, "NeoForge")}
                                                        >
                                                            {installLoaderOp.loading() === `${inst.id}|NeoForge` ? "安装中…" : "安装 NeoForge"}
                                                        </button>
                                                        <button
                                                            class="btn btn-xs rounded bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-50"
                                                            disabled={asyncInstallLoaderOp.loading() === `${inst.id}|NeoForge`}
                                                            onClick={() => handleAsyncInstallLoader(inst, "NeoForge")}
                                                        >
                                                            {asyncInstallLoaderOp.loading() === `${inst.id}|NeoForge` ? "启动中…" : "后台安装 NeoForge"}
                                                        </button>
                                                        <label class="flex items-center gap-1 text-[11px] text-stone-400 cursor-pointer select-none">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                checked={loaderInstallOverwrite()[inst.id] ?? false}
                                                                onChange={(e) => setLoaderInstallOverwrite({ ...loaderInstallOverwrite(), [inst.id]: e.currentTarget.checked })}
                                                            />
                                                            覆盖 loader
                                                        </label>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </Show>
                                    <Show when={expandedInstance() === inst.id}>
                                        <div class="border-b border-stone-200 bg-stone-50 px-5 py-4">
                                            <Show when={workspaceLoading()}>
                                                <div class="flex items-center gap-2 text-sm text-stone-500">
                                                    <span class="loading loading-spinner loading-xs" />
                                                    正在加载实例资源…
                                                </div>
                                            </Show>
                                            <Show when={workspaceErrorFor() === inst.id && workspaceError()}>
                                                <div class="rounded bg-red-50 px-3 py-2 text-sm text-red-600">
                                                    {workspaceError()}
                                                </div>
                                            </Show>
                                            <Show when={workspaceData() && !workspaceLoading() && workspaceErrorFor() !== inst.id}>
                                                <div class="space-y-3">
                                                    <div>
                                                        <span class="text-xs font-semibold text-stone-500">游戏目录：</span>
                                                        <span class="text-xs text-stone-700 break-all">{workspaceData()!.game_dir}</span>
                                                    </div>
                                                    <div class="flex flex-wrap gap-4 text-xs text-stone-600">
                                                        <span>
                                                            <span class="font-semibold">Mod：</span>
                                                            {workspaceData()!.mods.length}
                                                        </span>
                                                        <span>
                                                            <span class="font-semibold">资源包：</span>
                                                            {workspaceData()!.resource_packs.length}
                                                        </span>
                                                        <span>
                                                            <span class="font-semibold">光影包：</span>
                                                            {workspaceData()!.shader_packs.length}
                                                        </span>
                                                        <span>
                                                            <span class="font-semibold">世界：</span>
                                                            {workspaceData()!.worlds.length}
                                                        </span>
                                                        <span>
                                                            <span class="font-semibold">截图：</span>
                                                            {workspaceData()!.screenshots.length}
                                                        </span>
                                                        <span>
                                                            <span class="font-semibold">服务器：</span>
                                                            {workspaceData()!.servers.length}
                                                        </span>
                                                    </div>
                                                    <Show when={workspaceData()!.mods.length > 0}>
                                                        <div>
                                                            <h4 class="mb-1 text-xs font-semibold text-stone-500">
                                                                Mod 列表（前 10 个）
                                                            </h4>
                                                            <div class="space-y-1">
                                                                <For each={workspaceData()!.mods.slice(0, 10)}>
                                                                    {(mod) => {
                                                                        const opKey = `${inst.id}|${mod.file_name}`;
                                                                        const isLoading = modOpLoading() === opKey;
                                                                        const opErr = modOpError();
                                                                        const showErr = opErr?.key === opKey;
                                                                        return (
                                                                            <div class="flex items-center gap-2 text-xs text-stone-700">
                                                                                <span
                                                                                    class={`inline-block h-1.5 w-1.5 rounded-full shrink-0 ${mod.enabled ? "bg-green-500" : "bg-stone-400"}`}
                                                                                />
                                                                                <span class="font-medium truncate max-w-[180px]">{mod.name}</span>
                                                                                <span class="text-stone-400 shrink-0">
                                                                                    ({mod.enabled ? "启用" : "禁用"})
                                                                                </span>
                                                                                <span class="text-stone-400 shrink-0">
                                                                                    {mod.size > 1024
                                                                                        ? `${(mod.size / 1024).toFixed(1)} KB`
                                                                                        : `${mod.size} B`}
                                                                                </span>
                                                                                <Show when={!isLoading}>
                                                                                    <button
                                                                                        type="button"
                                                                                        class="ml-auto shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium border border-stone-300 bg-white text-stone-600 hover:bg-stone-100"
                                                                                        onClick={() => handleModToggle(inst.id, mod.file_name, mod.enabled)}
                                                                                    >
                                                                                        {mod.enabled ? "禁用" : "启用"}
                                                                                    </button>
                                                                                </Show>
                                                                                <Show when={isLoading}>
                                                                                    <span class="ml-auto loading loading-spinner loading-xs shrink-0" />
                                                                                </Show>
                                                                                <Show when={modDeleteConfirm() === opKey}>
                                                                                    <div class="flex items-center gap-1 shrink-0">
                                                                                        <span class="text-[10px] text-amber-600">确认删除?</span>
                                                                                        <button
                                                                                            type="button"
                                                                                            class="rounded px-1 py-0.5 text-[10px] font-medium bg-red-600 text-white hover:bg-red-700"
                                                                                            onClick={() => handleModDelete(inst.id, mod.file_name)}
                                                                                        >
                                                                                            确认
                                                                                        </button>
                                                                                        <button
                                                                                            type="button"
                                                                                            class="rounded px-1 py-0.5 text-[10px] border border-stone-300 bg-white text-stone-600"
                                                                                            onClick={() => setModDeleteConfirm(null)}
                                                                                        >
                                                                                            取消
                                                                                        </button>
                                                                                    </div>
                                                                                </Show>
                                                                                <Show when={modDeleteConfirm() !== opKey && !isLoading}>
                                                                                    <button
                                                                                        type="button"
                                                                                        class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium border border-red-300 bg-white text-red-600 hover:bg-red-50"
                                                                                        onClick={() => setModDeleteConfirm(opKey)}
                                                                                    >
                                                                                        删除
                                                                                    </button>
                                                                                </Show>
                                                                                <Show when={showErr}>
                                                                                    <span class="text-[10px] text-red-600 shrink-0">{opErr!.error}</span>
                                                                                </Show>
                                                                            </div>
                                                                        );
                                                                    }}
                                                                </For>
                                                            </div>
                                                        </div>
                                                    </Show>
                                                    <div class="border-t border-stone-200 pt-3 mt-3">
                                                        <h4 class="mb-2 text-xs font-semibold text-stone-500">Modpack 清单</h4>
                                                        <div class="flex items-center gap-2 mb-2">
                                                            <button
                                                                type="button"
                                                                class="rounded bg-teal-800 px-3 py-1 text-xs text-white hover:bg-teal-900 disabled:opacity-50"
                                                                disabled={exportLoading()}
                                                                onClick={() => handleExportManifest(inst.id)}
                                                            >
                                                                {exportLoading() ? "导出中…" : "导出清单"}
                                                            </button>
                                                        </div>
                                                        <Show when={exportError()}>
                                                            <div class="mb-2 rounded bg-red-50 px-2 py-1 text-xs text-red-600">{exportError()}</div>
                                                        </Show>
                                                        <Show when={exportResult()}>
                                                            <div class="mb-2 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                                已导出 {exportResult()!.file_count} 个文件，共 {exportResult()!.total_bytes > 1024
                                                                    ? `${(exportResult()!.total_bytes / 1024).toFixed(1)} KB`
                                                                    : `${exportResult()!.total_bytes} B`}
                                                            </div>
                                                            <textarea
                                                                class="mb-2 w-full rounded border border-stone-300 bg-white p-2 text-xs font-mono text-stone-700"
                                                                rows={8}
                                                                readonly
                                                                value={JSON.stringify(exportResult()!.manifest, null, 2)}
                                                            />
                                                        </Show>
                                                        <div class="border-t border-stone-100 pt-2">
                                                            <label class="mb-1 block text-xs font-medium text-stone-600">导入清单 JSON</label>
                                                            <textarea
                                                                class="mb-2 w-full rounded border border-stone-300 bg-white p-2 text-xs font-mono text-stone-700"
                                                                rows={4}
                                                                placeholder='粘贴导出的 ModpackManifest JSON…'
                                                                value={importText()}
                                                                onInput={(e) => setImportText(e.currentTarget.value)}
                                                            />
                                                            <div class="flex items-center gap-3 mb-2">
                                                                <label class="flex items-center gap-1 text-xs text-stone-600">
                                                                    <input
                                                                        type="checkbox"
                                                                        class="checkbox checkbox-xs"
                                                                        checked={importOverwrite()}
                                                                        onChange={(e) => setImportOverwrite(e.currentTarget.checked)}
                                                                    />
                                                                    覆盖已有文件
                                                                </label>
                                                                <button
                                                                    type="button"
                                                                    class="rounded bg-teal-800 px-3 py-1 text-xs text-white hover:bg-teal-900 disabled:opacity-50"
                                                                    disabled={importLoading()}
                                                                    onClick={() => handleImportManifest(inst.id)}
                                                                >
                                                                    {importLoading() ? "导入中…" : "导入清单"}
                                                                </button>
                                                            </div>
                                                            <Show when={importError()}>
                                                                <div class="mb-2 rounded bg-red-50 px-2 py-1 text-xs text-red-600">{importError()}</div>
                                                            </Show>
                                                            <Show when={importResult()}>
                                                                <div class="mb-2 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                                    导入完成：成功 {importResult()!.imported}，跳过 {importResult()!.skipped}，失败 {importResult()!.failed}，写入 {importResult()!.bytes_written > 1024
                                                                        ? `${(importResult()!.bytes_written / 1024).toFixed(1)} KB`
                                                                        : `${importResult()!.bytes_written} B`}
                                                                </div>
                                                            </Show>
                                                        </div>
                                                    </div>
                                                    <div class="border-t border-stone-200 pt-3 mt-3">
                                                        <h4 class="mb-2 text-xs font-semibold text-stone-500">Modpack ZIP</h4>
                                                        <div class="mb-3">
                                                            <label class="mb-1 block text-xs font-medium text-stone-600">导出 ZIP 输出路径</label>
                                                            <div class="flex items-center gap-2">
                                                                <input
                                                                    class="input input-bordered h-8 flex-1 rounded border-stone-300 bg-white text-xs"
                                                                    placeholder="e.g. C:\Users\me\Desktop\mypack.zip"
                                                                    value={zipExportPath()}
                                                                    onInput={(e) => setZipExportPath(e.currentTarget.value)}
                                                                />
                                                                <button
                                                                    type="button"
                                                                    class="shrink-0 rounded bg-teal-800 px-3 py-1 text-xs text-white hover:bg-teal-900 disabled:opacity-50"
                                                                    disabled={zipExportLoading()}
                                                                    onClick={() => handleExportZip(inst.id)}
                                                                >
                                                                    {zipExportLoading() ? "导出中…" : "导出 ZIP"}
                                                                </button>
                                                            </div>
                                                            <Show when={zipExportError()}>
                                                                <div class="mt-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">{zipExportError()}</div>
                                                            </Show>
                                                            <Show when={zipExportResult()}>
                                                                <div class="mt-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                                    导出完成：{zipExportResult()!.fileCount} 个文件，共 {zipExportResult()!.totalBytes > 1024
                                                                        ? `${(zipExportResult()!.totalBytes / 1024).toFixed(1)} KB`
                                                                        : `${zipExportResult()!.totalBytes} B`}
                                                                    {"，清单 "}{zipExportResult()!.manifestBytes > 1024
                                                                        ? `${(zipExportResult()!.manifestBytes / 1024).toFixed(1)} KB`
                                                                        : `${zipExportResult()!.manifestBytes} B`}
                                                                </div>
                                                            </Show>
                                                        </div>
                                                        <div>
                                                            <label class="mb-1 block text-xs font-medium text-stone-600">导入 ZIP 文件路径</label>
                                                            <div class="flex items-center gap-2">
                                                                <input
                                                                    class="input input-bordered h-8 flex-1 rounded border-stone-300 bg-white text-xs"
                                                                    placeholder="e.g. C:\Users\me\Desktop\mypack.zip"
                                                                    value={zipImportPath()}
                                                                    onInput={(e) => setZipImportPath(e.currentTarget.value)}
                                                                />
                                                            </div>
                                                            <div class="mt-1 flex items-center gap-3 mb-2">
                                                                <label class="flex items-center gap-1 text-xs text-stone-600">
                                                                    <input
                                                                        type="checkbox"
                                                                        class="checkbox checkbox-xs"
                                                                        checked={zipImportOverwrite()}
                                                                        onChange={(e) => setZipImportOverwrite(e.currentTarget.checked)}
                                                                    />
                                                                    覆盖已有文件
                                                                </label>
                                                                <button
                                                                    type="button"
                                                                    class="shrink-0 rounded bg-teal-800 px-3 py-1 text-xs text-white hover:bg-teal-900 disabled:opacity-50"
                                                                    disabled={zipImportLoading()}
                                                                    onClick={() => handleImportZip(inst.id)}
                                                                >
                                                                    {zipImportLoading() ? "导入中…" : "导入 ZIP"}
                                                                </button>
                                                            </div>
                                                            <Show when={zipImportError()}>
                                                                <div class="mb-1 rounded bg-red-50 px-2 py-1 text-xs text-red-600">{zipImportError()}</div>
                                                            </Show>
                                                            <Show when={zipImportResult()}>
                                                                <div class="mb-1 rounded bg-green-50 px-2 py-1 text-xs text-green-700">
                                                                    导入完成：成功 {zipImportResult()!.imported}，跳过 {zipImportResult()!.skipped}，失败 {zipImportResult()!.failed}，写入 {zipImportResult()!.bytesWritten > 1024
                                                                        ? `${(zipImportResult()!.bytesWritten / 1024).toFixed(1)} KB`
                                                                        : `${zipImportResult()!.bytesWritten} B`}
                                                                </div>
                                                            </Show>
                                                        </div>
                                                    </div>
                                                </div>
                                            </Show>
                                        </div>
                                    </Show>
                                </>
                            )}
                        </For>
                    </div>
                </div>
            </Show>

            <Show when={joinInstance()}>
                {(inst) => (
                    <JoinDialog
                        instance={inst()}
                        onClose={() => setJoinInstance(null)}
                    />
                )}
            </Show>

            <Show when={inviteInstance()}>
                {(inst) => (
                    <InviteDialog
                        instanceId={inst().id}
                        instanceName={inst().name}
                        onClose={() => setInviteInstance(null)}
                    />
                )}
            </Show>

            <Show when={createRoomOpen()}>
                <CreateRoomDialog
                    onClose={() => setCreateRoomOpen(false)}
                    onCreated={() => {
                        setCreateRoomOpen(false);
                        instancesQuery.refetch();
                    }}
                    memberClub={onboarding.data?.club ?? null}
                />
            </Show>

            <Show when={createOpen()}>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setCreateOpen(false)}>
                    <div class="w-96 rounded-xl bg-white p-6 shadow-xl" onClick={(e) => e.stopPropagation()}>
                        <h3 class="mb-4 text-lg font-bold">创建本地实例</h3>
                        <div class="flex flex-col gap-3">
                            <div>
                                <label class="mb-1 block text-sm font-medium text-stone-600">实例名称</label>
                                <input
                                    class="input input-bordered h-9 w-full rounded-lg border-stone-300 bg-white text-sm"
                                    placeholder="我的世界"
                                    value={createName()}
                                    onInput={(e) => setCreateName(e.currentTarget.value)}
                                />
                            </div>
                            <div>
                                <label class="mb-1 block text-sm font-medium text-stone-600">游戏版本</label>
                                <input
                                    class="input input-bordered h-9 w-full rounded-lg border-stone-300 bg-white text-sm"
                                    placeholder="1.21.4"
                                    value={createVersion()}
                                    onInput={(e) => setCreateVersion(e.currentTarget.value)}
                                />
                            </div>
                            <div>
                                <label class="mb-1 block text-sm font-medium text-stone-600">实例类型</label>
                                <select
                                    class="select select-bordered h-9 w-full rounded-lg border-stone-300 bg-white text-sm"
                                    value={createKind()}
                                    onChange={(e) => setCreateKind(e.currentTarget.value as LocalInstanceKind)}
                                >
                                    <For each={INSTANCE_KINDS}>
                                        {(k) => <option value={k.value}>{k.label}</option>}
                                    </For>
                                </select>
                            </div>
                            <Show when={createError()}>
                                <p class="text-sm text-red-600">{createError()}</p>
                            </Show>
                            <div class="flex justify-end gap-2 pt-2">
                                <button
                                    class="btn rounded-lg border border-stone-300 bg-white px-4 py-2 text-sm text-stone-600 hover:bg-stone-50"
                                    onClick={() => { setCreateOpen(false); setCreateError(""); }}
                                >
                                    取消
                                </button>
                                <button
                                    class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900"
                                    onClick={handleCreateLocal}
                                >
                                    创建
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
}
