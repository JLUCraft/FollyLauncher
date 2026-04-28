import { For, Show, createSignal } from "solid-js";
import { createQuery, createMutation, useQueryClient } from "@tanstack/solid-query";
import {
    listTournaments,
    listMatches,
    listTeams,
    registerForTournament,
    type Tournament,
    type Match,
} from "../api/tauri";

const STATUS_LABEL: Record<string, string> = {
    draft: "草稿", registration: "报名中", ongoing: "进行中", completed: "已结束",
};

const STATUS_CLASS: Record<string, string> = {
    draft: "bg-stone-100 text-stone-600",
    registration: "bg-blue-100 text-blue-800",
    ongoing: "bg-teal-100 text-teal-800",
    completed: "bg-stone-200 text-stone-500",
};

const MATCH_STATUS_LABEL: Record<string, string> = {
    scheduled: "待开始", live: "进行中", finished: "已结束", disputed: "争议中",
};

const MATCH_STATUS_CLASS: Record<string, string> = {
    scheduled: "bg-amber-100 text-amber-800",
    live: "bg-teal-100 text-teal-800",
    finished: "bg-stone-100 text-stone-500",
    disputed: "bg-red-100 text-red-800",
};

export function LeaguePage() {
    const qc = useQueryClient();
    const [selected, setSelected] = createSignal<Tournament | null>(null);
    const [matchesOpen, setMatchesOpen] = createSignal(false);

    const tournaments = createQuery(() => ({
        queryKey: ["tournaments"],
        queryFn: listTournaments,
        refetchInterval: 30000,
    }));

    const matches = createQuery(() => ({
        queryKey: ["matches", selected()?.id],
        queryFn: () => {
            const id = selected()?.id;
            const empty: Match[] = [];
            if (!id) return Promise.resolve(empty);
            return listMatches(id);
        },
        enabled: () => !!selected(),
    }));

    const teams = createQuery(() => ({
        queryKey: ["teams"],
        queryFn: listTeams,
        refetchInterval: 60000,
    }));

    const registerMutation = createMutation(() => ({
        mutationFn: (id: string) => registerForTournament(id),
        onSuccess: () => qc.invalidateQueries({ queryKey: ["tournaments"] }),
    }));

    return (
        <div class="h-full overflow-y-auto">
            <Show when={!selected()}>
                {/* Tournament list */}
                <div class="px-8 py-5">
                    <h2 class="text-2xl font-black text-stone-950">联赛</h2>

                    <Show when={tournaments.isLoading}>
                        <div class="mt-8 flex justify-center">
                            <span class="loading loading-spinner loading-md text-teal-700" />
                        </div>
                    </Show>

                    <Show when={tournaments.isError}>
                        <p class="mt-6 text-sm text-red-600">加载失败: {String(tournaments.error)}</p>
                    </Show>

                    <Show when={!tournaments.isLoading && tournaments.data?.length === 0}>
                        <p class="mt-8 text-center text-stone-400">暂无赛事</p>
                    </Show>

                    <div class="mt-6 grid gap-3">
                        <For each={tournaments.data ?? []}>
                            {(t) => (
                                <button
                                    type="button"
                                    class="w-full cursor-pointer rounded-xl border border-stone-200 bg-white p-5 text-left transition-all hover:border-teal-300 hover:shadow-sm"
                                    onClick={() => { setSelected(t); setMatchesOpen(false); }}
                                >
                                    <div class="flex items-start justify-between gap-4">
                                        <div class="min-w-0">
                                            <p class="truncate font-bold text-stone-900">{t.name}</p>
                                            <p class="mt-1 text-sm text-stone-500">{t.game_type} · {t.mode}</p>
                                        </div>
                                        <span class={`shrink-0 rounded-full px-3 py-1 text-xs font-medium ${STATUS_CLASS[t.status] ?? STATUS_CLASS.draft}`}>
                                            {STATUS_LABEL[t.status] ?? t.status}
                                        </span>
                                    </div>
                                    <div class="mt-3 flex items-center gap-4 text-xs text-stone-400">
                                        <span>参与 {t.participant_count}/{t.max_participants}</span>
                                        <span>{new Date(t.created_at).toLocaleDateString()}</span>
                                    </div>
                                </button>
                            )}
                        </For>
                    </div>

                    {/* Teams section */}
                    <Show when={(teams.data?.length ?? 0) > 0}>
                        <h3 class="mt-10 text-lg font-bold text-stone-800">队伍</h3>
                        <div class="mt-4 grid gap-3">
                            <For each={teams.data ?? []}>
                                {(team) => (
                                    <div class="rounded-xl border border-stone-200 bg-white p-4">
                                        <div class="flex items-center justify-between">
                                            <p class="font-semibold text-stone-900">{team.name}</p>
                                            <span class="text-sm text-stone-500">积分 {team.total_score}</span>
                                        </div>
                                        <div class="mt-2 flex flex-wrap gap-1.5">
                                            <For each={team.members?.slice(0, 6) ?? []}>
                                                {(m) => (
                                                    <span class="rounded-full bg-stone-100 px-2 py-0.5 text-xs text-stone-600">
                                                        {m.slice(0, 12)}…
                                                    </span>
                                                )}
                                            </For>
                                            <Show when={(team.members?.length ?? 0) > 6}>
                                                <span class="rounded-full bg-stone-100 px-2 py-0.5 text-xs text-stone-500">
                                                    +{team.members.length - 6}
                                                </span>
                                            </Show>
                                        </div>
                                    </div>
                                )}
                            </For>
                        </div>
                    </Show>
                </div>
            </Show>

            <Show when={selected()}>
                {(t) => (
                    <div class="px-8 py-5">
                        {/* Back + header */}
                        <div class="flex items-center gap-3">
                            <button
                                type="button"
                                class="btn btn-sm rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                                onClick={() => setSelected(null)}
                            >
                                ← 返回列表
                            </button>
                        </div>

                        <div class="mt-5 flex items-start justify-between gap-4">
                            <div>
                                <h2 class="text-2xl font-black text-stone-950">{t().name}</h2>
                                <p class="mt-1 text-sm text-stone-500">{t().game_type} · {t().mode}</p>
                            </div>
                            <span class={`rounded-full px-3 py-1 text-xs font-medium ${STATUS_CLASS[t().status] ?? STATUS_CLASS.draft}`}>
                                {STATUS_LABEL[t().status] ?? t().status}
                            </span>
                        </div>

                        {/* Meta */}
                        <div class="mt-6 divide-y divide-stone-100 rounded-xl border border-stone-200 bg-white">
                            {[
                                ["参与人数", `${t().participant_count} / ${t().max_participants}`],
                                ["创建时间", new Date(t().created_at).toLocaleString()],
                            ].map(([label, value]) => (
                                <div class="flex items-center justify-between px-5 py-3 text-sm">
                                    <span class="text-stone-500">{label}</span>
                                    <span class="font-medium text-stone-900">{value}</span>
                                </div>
                            ))}
                        </div>

                        {/* Actions */}
                        <div class="mt-5 flex gap-3">
                            <Show when={t().status === "registration"}>
                                <button
                                    class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                    disabled={registerMutation.isPending}
                                    onClick={() => registerMutation.mutate(t().id)}
                                >
                                    {registerMutation.isPending ? "报名中…" : "报名参赛"}
                                </button>
                            </Show>
                            <button
                                type="button"
                                class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                                onClick={() => setMatchesOpen(!matchesOpen())}
                            >
                                {matchesOpen() ? "收起对阵" : "查看对阵"}
                            </button>
                        </div>

                        {/* Match list */}
                        <Show when={matchesOpen()}>
                            <div class="mt-6">
                                <h3 class="font-bold text-stone-800">比赛对阵</h3>
                                <Show when={matches.isLoading}>
                                    <p class="mt-3 text-sm text-stone-400">加载中…</p>
                                </Show>
                                <Show when={!matches.isLoading && (matches.data?.length ?? 0) === 0}>
                                    <p class="mt-3 text-sm text-stone-400">暂无对阵信息</p>
                                </Show>
                                <div class="mt-3 grid gap-2">
                                    <For each={matches.data ?? []}>
                                        {(m) => (
                                            <div class="flex items-center justify-between rounded-lg border border-stone-200 bg-white px-4 py-3">
                                                <div>
                                                    <span class="text-sm font-semibold text-stone-800">第 {m.round} 轮</span>
                                                    <span class="ml-3 text-xs text-stone-400">
                                                        {new Date(m.scheduled_at).toLocaleString()}
                                                    </span>
                                                </div>
                                                <span class={`rounded-full px-3 py-1 text-xs font-medium ${MATCH_STATUS_CLASS[m.status] ?? MATCH_STATUS_CLASS.scheduled}`}>
                                                    {MATCH_STATUS_LABEL[m.status] ?? m.status}
                                                </span>
                                            </div>
                                        )}
                                    </For>
                                </div>
                            </div>
                        </Show>
                    </div>
                )}
            </Show>
        </div>
    );
}
