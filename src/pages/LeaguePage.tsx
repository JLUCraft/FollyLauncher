import { For, Show, createSignal, createResource } from "solid-js";
import { createQuery, createMutation, useQueryClient } from "@tanstack/solid-query";
import {
    listTournaments,
    listMatches,
    listTeams,
    listDisputes,
    registerForTournament,
    getOnboardingStatus,
    subscribeTournamentEvents,
    unsubscribeTournamentEvents,
    type Tournament,
    type Match,
    type DisputeMatch,
} from "../services";
import { DisputeDialog } from "../components/DisputeDialog";

const STATUS_LABEL: Record<string, string> = {
    draft: "草稿", registration: "报名中", ongoing: "进行中", paused: "已暂停",
    cancelled: "已取消", completed: "已结束",
};

const STATUS_CLASS: Record<string, string> = {
    draft: "bg-stone-100 text-stone-600",
    registration: "bg-blue-100 text-blue-800",
    ongoing: "bg-teal-100 text-teal-800",
    paused: "bg-amber-100 text-amber-800",
    cancelled: "bg-red-100 text-red-800",
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

const DISPUTE_STATUS_LABEL: Record<string, string> = {
    open: "待处理",
    under_review: "审核中",
    resolved: "已解决",
    closed: "已关闭",
};

const DISPUTE_STATUS_CLASS: Record<string, string> = {
    open: "bg-red-100 text-red-800",
    under_review: "bg-amber-100 text-amber-800",
    resolved: "bg-green-100 text-green-800",
    closed: "bg-stone-100 text-stone-500",
};

export function LeaguePage() {
    const qc = useQueryClient();
    const [selected, setSelected] = createSignal<Tournament | null>(null);
    const [matchesOpen, setMatchesOpen] = createSignal(false);
    const [disputesOpen, setDisputesOpen] = createSignal(false);
    const [disputeTarget, setDisputeTarget] = createSignal<{
        tournamentId: string;
        matchId: string;
        round: number;
    } | null>(null);

    // Resolve dispute removed — FollyLauncher is a player-side launcher
    // without admin TEE / canonical command signing capability. The
    // "处理" button in the dispute list is also removed; dispute
    // resolution must go through union-manager's resolveDisputeViaProposal.

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

    const disputes = createQuery(() => ({
        queryKey: ["disputes", selected()?.id],
        queryFn: () => {
            const id = selected()?.id;
            const empty: DisputeMatch[] = [];
            if (!id) return Promise.resolve(empty);
            return listDisputes(id);
        },
        enabled: () => !!selected(),
    }));

    const teams = createQuery(() => ({
        queryKey: ["teams"],
        queryFn: listTeams,
        refetchInterval: 60000,
    }));

    // Personal onboarding status for score / role context
    const onboarding = createQuery(() => ({
        queryKey: ["onboarding-league"],
        queryFn: getOnboardingStatus,
        staleTime: 30_000,
    }));

    // Tournament event subscription when viewing a tournament
    const [eventSubscribed, setEventSubscribed] = createSignal(false);
    const [prevSubscribedId, setPrevSubscribedId] = createSignal<string | null>(null);
    createResource(
        () => selected()?.id,
        async (id) => {
            // Unsubscribe previous if any
            const prevId = prevSubscribedId();
            if (prevId) {
                try { await unsubscribeTournamentEvents(prevId); } catch { /* ok */ }
                setPrevSubscribedId(null);
                setEventSubscribed(false);
            }
            if (id) {
                try {
                    await subscribeTournamentEvents(id);
                    setEventSubscribed(true);
                    setPrevSubscribedId(id);
                } catch { /* topic subscriptions are best-effort */ }
            }
            return id;
        },
    );

    const myTeam = () => {
        const all = teams.data ?? [];
        // Use onboarding data to find the user's team
        const club = onboarding.data?.club;
        if (!club) return null;
        return all.find((t) => t.name === club) ?? null;
    };

    const registerMutation = createMutation(() => ({
        mutationFn: (id: string) => registerForTournament(id),
        onSuccess: () => qc.invalidateQueries({ queryKey: ["tournaments"] }),
    }));

    function handleDisputeSubmitted() {
        setDisputeTarget(null);
        qc.invalidateQueries({ queryKey: ["disputes"] });
        qc.invalidateQueries({ queryKey: ["matches"] });
    }

    function disputeForMatch(matchId: string): DisputeMatch | undefined {
        return (disputes.data ?? []).find((d) => d.match_id === matchId && d.status !== "closed");
    }

    return (
        <div class="h-full overflow-y-auto">
            <Show when={!selected()}>
                {/* Tournament list */}
                <div class="px-8 py-5">
                    <h2 class="text-2xl font-black text-stone-950">联赛</h2>

                    {/* Personal score / eligibility card */}
                    <Show when={onboarding.data}>
                        {(status) => (
                            <div class="mt-4 rounded-xl border border-teal-200 bg-teal-50/60 p-4">
                                <div class="flex items-center justify-between">
                                    <div>
                                        <p class="text-sm font-semibold text-teal-900">
                                            个人联赛面板
                                        </p>
                                        <p class="text-xs text-teal-700 mt-1">
                                            身份: {status().mode_label}
                                            <Show when={myTeam()}>
                                                {(team) => (
                                                    <span>
                                                        {" · 队伍: "}{team().name}
                                                        {" · 积分: "}{team().total_score}
                                                    </span>
                                                )}
                                            </Show>
                                        </p>
                                    </div>
                                    <div class="text-right">
                                        <Show when={myTeam()}>
                                            {(team) => (
                                                <div class="flex items-center gap-2">
                                                    <span class="text-2xl font-black text-teal-800">
                                                        {team().total_score}
                                                    </span>
                                                    <span class="text-xs text-teal-600">积分</span>
                                                </div>
                                            )}
                                        </Show>
                                    </div>
                                </div>
                                <Show when={!status().is_member && !status().is_guest}>
                                    <p class="mt-2 text-xs text-amber-700 bg-amber-50 rounded px-2 py-1">
                                        ⚠ 需要社团成员 VC 才可在联赛中获取积分和报名参赛。
                                    </p>
                                </Show>
                                <Show when={status().is_guest}>
                                    <p class="mt-2 text-xs text-amber-700 bg-amber-50 rounded px-2 py-1">
                                        ⚠ MUA 访客无法报名联赛；请先获取社团 VC。
                                    </p>
                                </Show>
                            </div>
                        )}
                    </Show>

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
                                    onClick={() => { setSelected(t); setMatchesOpen(false); setDisputesOpen(false); }}
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
                            <button
                                type="button"
                                class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                                onClick={() => setDisputesOpen(!disputesOpen())}
                            >
                                {disputesOpen() ? "收起争议" : "查看争议"}
                            </button>
                        </div>

                        {/* Event subscription status */}
                        <div class="mt-3 flex items-center gap-2 text-xs text-stone-500">
                            <span
                                class={`inline-block h-2 w-2 rounded-full ${
                                    eventSubscribed() ? "bg-teal-500" : "bg-stone-300"
                                }`}
                            />
                            <span>
                                {eventSubscribed()
                                    ? "已订阅联赛事件通知"
                                    : "联赛事件通知不可用"}
                            </span>
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
                                        {(m) => {
                                            const dispute = disputeForMatch(m.id);
                                            return (
                                                <div class="flex items-center justify-between rounded-lg border border-stone-200 bg-white px-4 py-3">
                                                    <div class="min-w-0 flex-1">
                                                        <div class="flex items-center gap-2">
                                                            <span class="text-sm font-semibold text-stone-800">第 {m.round} 轮</span>
                                                            <span class={`rounded-full px-2 py-0.5 text-xs font-medium ${MATCH_STATUS_CLASS[m.status] ?? MATCH_STATUS_CLASS.scheduled}`}>
                                                                {MATCH_STATUS_LABEL[m.status] ?? m.status}
                                                            </span>
                                                            {dispute && (
                                                                <span class={`rounded-full px-2 py-0.5 text-xs font-medium ${DISPUTE_STATUS_CLASS[dispute.status] ?? DISPUTE_STATUS_CLASS.open}`}>
                                                                    争议: {DISPUTE_STATUS_LABEL[dispute.status] ?? dispute.status}
                                                                </span>
                                                            )}
                                                        </div>
                                                        <span class="mt-1 block text-xs text-stone-400">
                                                            {new Date(m.scheduled_at).toLocaleString()}
                                                        </span>
                                                    </div>
                                                    <Show when={m.status === "finished"}>
                                                        <button
                                                            type="button"
                                                            class="ml-3 shrink-0 rounded-lg border border-red-200 bg-red-50 px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-100"
                                                            onClick={() =>
                                                                setDisputeTarget({
                                                                    tournamentId: t().id,
                                                                    matchId: m.id,
                                                                    round: m.round,
                                                                })
                                                            }
                                                        >
                                                            提交争议
                                                        </button>
                                                    </Show>
                                                </div>
                                            );
                                        }}
                                    </For>
                                </div>
                            </div>
                        </Show>

                        {/* Dispute list */}
                        <Show when={disputesOpen()}>
                            <div class="mt-6">
                                <h3 class="font-bold text-stone-800">争议记录</h3>
                                <Show when={disputes.isLoading}>
                                    <p class="mt-3 text-sm text-stone-400">加载中…</p>
                                </Show>
                                <Show when={!disputes.isLoading && (disputes.data?.length ?? 0) === 0}>
                                    <p class="mt-3 text-sm text-stone-400">暂无争议记录</p>
                                </Show>
                                <div class="mt-3 grid gap-2">
                                    <For each={disputes.data ?? []}>
                                        {(d) => (
                                            <div class="rounded-lg border border-stone-200 bg-white px-4 py-3">
                                                <div class="flex items-center justify-between">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-sm font-semibold text-stone-800">
                                                            比赛 {d.match_id.slice(0, 8)}…
                                                        </span>
                                                        <span class={`rounded-full px-2 py-0.5 text-xs font-medium ${DISPUTE_STATUS_CLASS[d.status] ?? DISPUTE_STATUS_CLASS.open}`}>
                                                            {DISPUTE_STATUS_LABEL[d.status] ?? d.status}
                                                        </span>
                                                    </div>
                                                    <Show when={d.submitted_by}>
                                                        <span class="text-xs text-stone-400">
                                                            {d.submitted_by}
                                                        </span>
                                                    </Show>
                                                </div>
                                                <p class="mt-2 text-sm text-stone-700">{d.reason}</p>
                                                <Show when={d.evidence_urls.length > 0}>
                                                    <div class="mt-2 flex flex-wrap gap-1.5">
                                                        <For each={d.evidence_urls}>
                                                            {(url) => (
                                                                <a
                                                                    href={url}
                                                                    target="_blank"
                                                                    rel="noopener noreferrer"
                                                                    class="inline-block max-w-[200px] truncate rounded bg-stone-100 px-2 py-0.5 text-xs text-teal-700 hover:underline"
                                                                >
                                                                    {url}
                                                                </a>
                                                            )}
                                                        </For>
                                                    </div>
                                                </Show>
                                                <Show when={d.resolution}>
                                                    <p class="mt-2 rounded bg-stone-50 px-3 py-2 text-xs text-stone-600">
                                                        处理结果: {d.resolution}
                                                    </p>
                                                </Show>
                                                {/* Resolve button removed — dispute resolution
                                                    requires admin TEE / canonical command
                                                    signing, which the launcher does not have.
                                                    Use union-manager resolveDisputeViaProposal. */}
                                            </div>
                                        )}
                                    </For>
                                </div>
                            </div>
                        </Show>
                    </div>
                )}
            </Show>

            {/* Dispute dialog overlay */}
            <Show when={disputeTarget()}>
                {(dt) => (
                    <DisputeDialog
                        tournamentId={dt().tournamentId}
                        matchId={dt().matchId}
                        matchRound={dt().round}
                        onClose={(dispute) => {
                            if (dispute) {
                                handleDisputeSubmitted();
                            } else {
                                setDisputeTarget(null);
                            }
                        }}
                    />
                )}
            </Show>

            {/* Resolve dispute dialog removed — dispute resolution
                requires admin TEE / canonical command signing capability
                that FollyLauncher does not possess. Resolution must use
                union-manager's resolveDisputeViaProposal path. */}
        </div>
    );
}
