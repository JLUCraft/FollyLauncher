import { For, Show, createSignal, createMemo, createEffect } from "solid-js";
import { createQuery } from "@tanstack/solid-query";
import {
    listPeers,
    listInstances,
    listInstancesHttp,
    resolveInstance,
    getClusterMessages,
    measureLatency,
    type HttpInstance,
} from "../api/tauri";
import { InstanceCard } from "../components/InstanceCard";
import { JoinDialog } from "../components/JoinDialog";
import { CreateRoomDialog } from "../components/CreateRoomDialog";
import type { Instance } from "../types";

const TYPE_TABS = [
    { label: "全部", value: "all" },
    { label: "服务", value: "service" },
    { label: "房间", value: "room" },
] as const;

function toInstanceType(kind: string): "service" | "room" {
    return kind === "service" ? "service" : "room";
}

function mapHttpToInstance(info: HttpInstance): Instance {
    return {
        id: info.id,
        name: info.name,
        mode: info.kind === "service" ? "服务" : "房间",
        club: info.club,
        type: toInstanceType(info.kind),
        players: "--",
        latency: null,
        state: mapStatus(info.status),
        version: "未提供",
        peer_id: null,
    };
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
    const [joinInstance, setJoinInstance] = createSignal<Instance | null>(null);
    const [createRoomOpen, setCreateRoomOpen] = createSignal(false);
    const [latencies, setLatencies] = createSignal<Record<string, number | null>>({});

    const instancesQuery = createQuery(() => ({
        queryKey: ["instances"],
        queryFn: listInstances,
        refetchInterval: 10000,
    }));

    const instancesHttpQuery = createQuery(() => ({
        queryKey: ["instances-http"],
        queryFn: listInstancesHttp,
        refetchInterval: 15000,
    }));

    const messages = createQuery(() => ({
        queryKey: ["cluster-messages"],
        queryFn: getClusterMessages,
        refetchInterval: 15000,
    }));

    const peers = createQuery(() => ({
        queryKey: ["peers"],
        queryFn: listPeers,
        refetchInterval: 60000,
    }));

    const instanceList = createMemo((): Instance[] => {
        const p2p = instancesQuery.data ?? [];
        const http = instancesHttpQuery.data ?? [];
        const lat = latencies();
        const map = new Map<string, Instance>();
        for (const info of p2p) {
            map.set(info.id, {
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
            });
        }
        for (const info of http) {
            if (!map.has(info.id)) map.set(info.id, mapHttpToInstance(info));
        }
        return Array.from(map.values());
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
        try { await resolveInstance(id); } catch { /* non-critical */ }
    }

    const isLoading = createMemo(() => instancesQuery.isLoading && instancesHttpQuery.isLoading);

    const lastMessagePreview = createMemo(() => {
        const data = messages.data;
        if (!data || data.length === 0) return "";
        const last = data[data.length - 1];
        if (!last) return "";
        const payload = typeof last.payload === "object"
            ? JSON.stringify(last.payload).slice(0, 80)
            : String(last.payload).slice(0, 80);
        return `${last.topic} — ${payload}`;
    });

    return (
        <div class="flex h-full flex-col">
            {/* Page header */}
            <header class="border-b border-stone-200 px-8 py-5">
                <div class="flex items-center justify-between">
                    <h2 class="text-2xl font-black text-stone-950">实例列表</h2>
                    <div class="flex items-center gap-3 text-sm text-stone-500">
                        <button
                            class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900"
                            onClick={() => setCreateRoomOpen(true)}
                        >
                            + 创建房间
                        </button>
                        <Show when={isLoading()}>
                            <span class="loading loading-spinner loading-xs" />
                        </Show>
                        <span>{filtered().length} 个实例</span>
                        <span class="text-stone-300">·</span>
                        <span>{peers.data ? `${peers.data.length} 个节点` : "节点数不可用"}</span>
                    </div>
                </div>

                {/* Filter bar */}
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
            </header>

            {/* Instance list */}
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
                            />
                        )}
                    </For>
                </div>
            </div>

            {/* Network messages strip */}
            <Show when={messages.data && messages.data.length > 0}>
                <footer class="border-t border-stone-200 bg-stone-50 px-8 py-2">
                    <div class="flex items-center gap-3 overflow-hidden text-xs text-stone-500">
                        <span class="shrink-0 font-semibold text-teal-700">网络消息</span>
                        <span class="truncate">{lastMessagePreview()}</span>
                    </div>
                </footer>
            </Show>

            {/* Join dialog */}
            <Show when={joinInstance()}>
                {(inst) => (
                    <JoinDialog
                        instance={inst()}
                        onClose={() => setJoinInstance(null)}
                    />
                )}
            </Show>

            {/* Create room dialog */}
            <Show when={createRoomOpen()}>
                <CreateRoomDialog
                    onClose={() => setCreateRoomOpen(false)}
                    onCreated={() => {
                        setCreateRoomOpen(false);
                        instancesQuery.refetch();
                        instancesHttpQuery.refetch();
                    }}
                />
            </Show>
        </div>
    );
}
