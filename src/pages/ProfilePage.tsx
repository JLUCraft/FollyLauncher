import { createSignal, Show, For, onCleanup } from "solid-js";
import { createQuery } from "@tanstack/solid-query";
import {
    getIdentity,
    getProxyPort,
    listPeers,
    getVcStatus,
    importVc,
    clearVc,
    getMuaStatus,
    startMuaLogin,
    pollMuaLogin,
    logoutMua,
    getSkinTextures,
    listTeams,
    type VcHolderState,
    type Team,
} from "../api/tauri";
import { useQueryClient } from "@tanstack/solid-query";

export function ProfilePage() {
    const qc = useQueryClient();
    const [vcInput, setVcInput] = createSignal("");
    const [muaDialogOpen, setMuaDialogOpen] = createSignal(false);
    const [muaUserCode, setMuaUserCode] = createSignal("");
    const [muaVerificationUri, setMuaVerificationUri] = createSignal("");
    const [muaPolling, setMuaPolling] = createSignal(false);
    const [muaError, setMuaError] = createSignal("");
    const [vcImporting, setVcImporting] = createSignal(false);
    const [vcMessage, setVcMessage] = createSignal("");

    let muaPollTimer: ReturnType<typeof setTimeout> | null = null;
    onCleanup(() => {
        if (muaPollTimer) clearTimeout(muaPollTimer);
    });

    const identity = createQuery(() => ({ queryKey: ["identity"], queryFn: getIdentity }));
    const proxy = createQuery(() => ({ queryKey: ["proxy"], queryFn: getProxyPort }));
    const peers = createQuery(() => ({ queryKey: ["peers"], queryFn: listPeers, refetchInterval: 60000 }));
    const vc = createQuery(() => ({ queryKey: ["vc"], queryFn: getVcStatus }));
    const muaStatus = createQuery(() => ({ queryKey: ["mua-status"], queryFn: getMuaStatus }));
    const skinTextures = createQuery(() => ({
        queryKey: ["skin-textures"],
        queryFn: getSkinTextures,
        enabled: () => !!muaStatus.data?.logged_in,
        staleTime: 60000,
    }));
    const teams = createQuery(() => ({
        queryKey: ["teams"],
        queryFn: listTeams,
        enabled: () => !!muaStatus.data?.is_member,
        staleTime: 60000,
    }));

    async function beginMuaLogin() {
        setMuaError("");
        let mounted = true;
        onCleanup(() => { mounted = false; });
        try {
            const resp = await startMuaLogin();
            if (!mounted) return;
            setMuaUserCode(resp.user_code);
            setMuaVerificationUri(resp.verification_uri);
            setMuaDialogOpen(true);
            setMuaPolling(true);
            try {
                const { openUrl } = await import("@tauri-apps/plugin-opener");
                await openUrl(resp.verification_uri);
            } catch { /* opener may not be available */ }
            muaPollTimer = setTimeout(async () => {
                muaPollTimer = null;
                if (!mounted) return;
                try {
                    await pollMuaLogin();
                    if (!mounted) return;
                    qc.invalidateQueries({ queryKey: ["mua-status"] });
                    setMuaPolling(false);
                    setMuaDialogOpen(false);
                } catch (e) {
                    if (!mounted) return;
                    setMuaError(String(e));
                    setMuaPolling(false);
                }
            }, 8000);
        } catch (e) {
            setMuaError(String(e));
        }
    }

    async function handleImportVc() {
        setVcImporting(true);
        setVcMessage("");
        try {
            const result = await importVc(vcInput());
            qc.invalidateQueries({ queryKey: ["vc"] });
            if (result.state === "Member") {
                setVcMessage("✓ VC 导入并验证成功");
            } else if (result.expired) {
                setVcMessage("⚠ VC 已导入但已过期，需续签");
            } else if (!result.verified) {
                setVcMessage("⚠ VC 已导入但签名验证失败");
            } else {
                setVcMessage("⚠ VC 已导入但状态异常");
            }
            setVcInput("");
        } catch (e) {
            setVcMessage("✗ 导入失败: " + e);
        } finally {
            setVcImporting(false);
        }
    }

    async function handleLogout() {
        await logoutMua();
        qc.invalidateQueries({ queryKey: ["mua-status"] });
    }

    async function handleClearVc() {
        await clearVc();
        qc.invalidateQueries({ queryKey: ["vc"] });
        setVcMessage("");
    }

    return (
        <div class="h-full overflow-y-auto px-8 py-5">
            <h2 class="text-2xl font-black text-stone-950">我的</h2>

            <div class="mt-6 grid gap-5">
                {/* Identity */}
                <section class="rounded-xl border border-stone-200 bg-white p-6">
                    <h3 class="font-bold text-stone-800">节点身份</h3>
                    <Show when={identity.isLoading}>
                        <p class="mt-3 text-sm text-stone-400">加载中…</p>
                    </Show>
                    <Show when={!identity.isLoading}>
                        <dl class="mt-4 divide-y divide-stone-100">
                            <Row label="社团" value={identity.data?.club ?? "未签发"} />
                            <Row label="PeerID" value={identity.data?.peer_id ?? "未加载"} mono />
                            <Row label="代理端口" value={proxy.data ? String(proxy.data.local_port) : "未加载"} />
                            <Row label="已连接节点" value={peers.data ? String(peers.data.length) : "未加载"} />
                        </dl>
                    </Show>
                </section>

                {/* MUA Login */}
                <section class="rounded-xl border border-stone-200 bg-white p-6">
                    <h3 class="font-bold text-stone-800">MUA 皮肤站</h3>
                    <Show
                        when={muaStatus.data?.logged_in}
                        fallback={
                            <div class="mt-4">
                                <p class="text-sm text-stone-500">未登录 MUA 联合皮肤站。登录后可使用 Yggdrasil 身份进入服务器。</p>
                                <Show when={muaError()}>
                                    <p class="mt-2 text-sm text-red-600">{muaError()}</p>
                                </Show>
                                <button
                                    class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                    disabled={muaPolling()}
                                    onClick={beginMuaLogin}
                                >
                                    {muaPolling() ? "登录中…" : "MUA 登录"}
                                </button>
                            </div>
                        }
                    >
                        <div class="mt-4 flex gap-5">
                            <div class="shrink-0">
                                <Show when={skinTextures.data?.skin_url} fallback={
                                    <div class="h-24 w-24 rounded-lg bg-stone-100 flex items-center justify-center text-stone-400 text-xs">
                                        无皮肤
                                    </div>
                                }>
                                    {(url) => (
                                        <img
                                            src={url()}
                                            alt="皮肤预览"
                                            class="h-24 w-24 rounded-lg object-cover border border-stone-200"
                                        />
                                    )}
                                </Show>
                            </div>
                            <dl class="flex-1 divide-y divide-stone-100">
                                <Row label="用户名" value={muaStatus.data?.username ?? "未提供"} />
                                <Row label="UUID" value={muaStatus.data?.uuid ?? "未提供"} mono />
                                <Row label="皮肤站" value={muaStatus.data?.auth_server_url ?? "未提供"} />
                                <Row label="Peer绑定" value={muaStatus.data?.peer_bound ? "已绑定" : "未绑定"} accent={muaStatus.data?.peer_bound ? "success" : "warn"} />
                                <div class="flex items-center justify-between py-2.5 text-sm">
                                    <span class="text-stone-500">身份</span>
                                    <span class={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                                        muaStatus.data?.is_member ? "bg-teal-100 text-teal-800" :
                                        muaStatus.data?.is_guest ? "bg-amber-100 text-amber-800" :
                                        "bg-stone-100 text-stone-500"
                                    }`}>
                                        {muaStatus.data?.is_member ? "成员" :
                                         muaStatus.data?.is_guest ? "访客" : "未登录"}
                                    </span>
                                </div>
                            </dl>
                        </div>
                        <button
                            class="btn mt-4 rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                            onClick={handleLogout}
                        >
                            退出登录
                        </button>
                    </Show>
                </section>

                {/* Points / Teams */}
                <Show when={muaStatus.data?.is_member && teams.data}>
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">战队与积分</h3>
                        <Show when={teams.data && teams.data.length > 0} fallback={
                            <p class="mt-3 text-sm text-stone-500">暂无战队信息</p>
                        }>
                            <div class="mt-4 grid gap-3">
                                <For each={teams.data}>
                                    {(team: Team) => (
                                        <div class="flex items-center justify-between rounded-lg border border-stone-200 px-4 py-3">
                                            <div>
                                                <p class="font-medium text-stone-800">{team.name}</p>
                                                <p class="text-xs text-stone-400">{team.members.length} 名成员</p>
                                            </div>
                                            <div class="text-right">
                                                <p class="text-lg font-bold text-teal-700">{team.total_score}</p>
                                                <p class="text-xs text-stone-400">积分</p>
                                            </div>
                                        </div>
                                    )}
                                </For>
                            </div>
                        </Show>
                    </section>
                </Show>

                {/* VC */}
                <section class="rounded-xl border border-stone-200 bg-white p-6">
                    <h3 class="font-bold text-stone-800">身份凭证 (VC)</h3>
                    <Show
                        when={vc.data && vc.data.state !== "Unverified"}
                        fallback={
                            <div class="mt-4">
                                <p class="text-sm text-stone-500">暂无有效身份凭证。粘贴社长签发的 VC JSON 导入：</p>
                                <textarea
                                    class="mt-3 w-full rounded-lg border border-stone-300 bg-white p-3 font-mono text-xs focus:border-teal-500 focus:outline-none"
                                    rows={4}
                                    placeholder='{"@context": [...], "type": [...], ...}'
                                    value={vcInput()}
                                    onInput={(e) => setVcInput(e.currentTarget.value)}
                                />
                                <Show when={vcMessage()}>
                                    <p class={`mt-2 text-sm ${vcMessage().startsWith("✓") ? "text-teal-700" : vcMessage().startsWith("⚠") ? "text-amber-600" : "text-red-600"}`}>
                                        {vcMessage()}
                                    </p>
                                </Show>
                                <button
                                    class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                    disabled={vcImporting() || !vcInput().trim()}
                                    onClick={handleImportVc}
                                >
                                    {vcImporting() ? "导入中…" : "导入 VC"}
                                </button>
                            </div>
                        }
                    >
                        <dl class="mt-4 divide-y divide-stone-100">
                            <Row label="角色" value={vc.data?.role ?? "未提供"} />
                            <Row label="社团" value={vc.data?.club ?? "未提供"} />
                            <Row label="签发方" value={vc.data?.issuer ?? "未提供"} mono />
                            <Row
                                label="验证状态"
                                value={vcStatusLabel(vc.data?.state)}
                                accent={vcStatusAccent(vc.data?.state)}
                            />
                        </dl>
                        <button
                            class="btn mt-4 rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                            onClick={handleClearVc}
                        >
                            清除 VC
                        </button>
                    </Show>
                </section>
            </div>

            {/* MUA Dialog */}
            <Show when={muaDialogOpen()}>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
                    <div class="w-full max-w-sm rounded-2xl border border-stone-200 bg-white p-6 shadow-2xl">
                        <h3 class="text-lg font-bold text-stone-900">MUA 设备验证</h3>
                        <p class="mt-2 text-sm text-stone-500">在浏览器中输入以下验证码：</p>
                        <div class="mt-4 rounded-xl bg-stone-50 py-5 text-center">
                            <p class="font-mono text-3xl font-bold tracking-[0.3em] text-stone-900">
                                {muaUserCode()}
                            </p>
                        </div>
                        <p class="mt-3 break-all text-xs text-stone-400">{muaVerificationUri()}</p>
                        <Show when={muaError()}>
                            <p class="mt-3 text-sm text-red-600">{muaError()}</p>
                        </Show>
                        <div class="mt-5 flex justify-end">
                            <button
                                class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                                onClick={() => {
                                if (muaPollTimer) { clearTimeout(muaPollTimer); muaPollTimer = null; }
                                setMuaDialogOpen(false);
                                setMuaPolling(false);
                            }}
                            >
                                取消
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
}

function vcStatusLabel(state: VcHolderState | undefined): string {
    switch (state) {
        case "Member": return "已验证 (成员)";
        case "Expired": return "已过期";
        case "Revoked": return "已吊销";
        default: return "未验证";
    }
}

type StatusAccent = "success" | "warn" | "neutral";

function vcStatusAccent(state: VcHolderState | undefined): StatusAccent {
    switch (state) {
        case "Member": return "success";
        case "Expired":
        case "Revoked":
            return "warn";
        default: return "neutral";
    }
}

function Row(props: {
    label: string;
    value: string;
    mono?: boolean;
    accent?: StatusAccent;
}) {
    const valueClass = () => {
        if (props.accent === "success") return "font-medium text-teal-700";
        if (props.accent === "warn") return "font-medium text-amber-600";
        return props.mono ? "font-mono text-xs text-stone-700" : "text-stone-800";
    };
    return (
        <div class="flex items-center justify-between py-2.5 text-sm">
            <span class="text-stone-500">{props.label}</span>
            <span class={valueClass()}>{props.value}</span>
        </div>
    );
}
