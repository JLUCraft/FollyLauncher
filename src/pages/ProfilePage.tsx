import { createSignal, Show, For, onCleanup, onMount, createResource } from "solid-js";
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
    getBootstrapStatus,
    getOnboardingStatus,
    getNetworkDiagnostics,
    listLauncherAccounts,
    addOfflineAccount,
    addThirdPartyAccount,
    selectLauncherAccount,
    deleteLauncherAccount,
    startMicrosoftLogin,
    pollMicrosoftLogin,
    refreshMicrosoftAccount,
    updateAccountAvatar,
    refreshAccountAvatar,
    exportLauncherAccounts,
    importLauncherAccounts,
    importExternalAccounts,
    type VcHolderState,
    type Team,
    type LauncherAccount,
    type LauncherAccountKind,
    type BootstrapStatus,
} from "../services";
import { useQueryClient } from "@tanstack/solid-query";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import jsQR from "jsqr";

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
    const [vcImportTab, setVcImportTab] = createSignal<"paste" | "file" | "scan">("paste");
    const [vcFileName, setVcFileName] = createSignal("");
    const [vcScanOpen, setVcScanOpen] = createSignal(false);
    const [addAccountName, setAddAccountName] = createSignal("");
    const [accountError, setAccountError] = createSignal("");
    // Third-party Yggdrasil login form
    const [tpAuthUrl, setTpAuthUrl] = createSignal("");
    const [tpUsername, setTpUsername] = createSignal("");
    const [tpPassword, setTpPassword] = createSignal("");
    const [tpLoading, setTpLoading] = createSignal(false);
    const [tpError, setTpError] = createSignal("");
    const [tpSuccess, setTpSuccess] = createSignal("");
    // Microsoft OAuth device flow
    const [msDialogOpen, setMsDialogOpen] = createSignal(false);
    const [msDeviceCode, setMsDeviceCode] = createSignal("");
    const [msUserCode, setMsUserCode] = createSignal("");
    const [msVerificationUri, setMsVerificationUri] = createSignal("");
    const [msVerificationUriComplete, setMsVerificationUriComplete] = createSignal<string | null>(null);
    const [msPolling, setMsPolling] = createSignal(false);
    const [msError, setMsError] = createSignal("");
    const [msSuccess, setMsSuccess] = createSignal("");
    // Per-account Microsoft refresh state
    const [msRefreshLoading, setMsRefreshLoading] = createSignal<Record<string, boolean>>({});
    const [msRefreshError, setMsRefreshError] = createSignal<Record<string, string>>({});
    const [msRefreshSuccess, setMsRefreshSuccess] = createSignal<Record<string, string>>({});
    // Per-account avatar management state
    const [avatarUrlInput, setAvatarUrlInput] = createSignal<Record<string, string>>({});
    const [avatarLoading, setAvatarLoading] = createSignal<Record<string, boolean>>({});
    const [avatarError, setAvatarError] = createSignal<Record<string, string>>({});
    const [avatarSuccess, setAvatarSuccess] = createSignal<Record<string, string>>({});
    // Export / Import account state
    const [exportedJson, setExportedJson] = createSignal("");
    const [showExport, setShowExport] = createSignal(false);
    const [importBundleJson, setImportBundleJson] = createSignal("");
    const [dedupeByUuid, setDedupeByUuid] = createSignal(true);
    const [importing, setImporting] = createSignal(false);
    const [importResult, setImportResult] = createSignal("");
    const [importError, setImportError] = createSignal("");
    // External (Prism/MultiMC) import state
    const [externalAccountsJson, setExternalAccountsJson] = createSignal("");
    const [externalDedupe, setExternalDedupe] = createSignal(true);
    const [externalImporting, setExternalImporting] = createSignal(false);
    const [externalImportResult, setExternalImportResult] = createSignal("");
    const [externalImportError, setExternalImportError] = createSignal("");

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
    const bootstrap = createQuery(() => ({
        queryKey: ["bootstrap"],
        queryFn: getBootstrapStatus,
        staleTime: 30000,
    }));
    const onboarding = createQuery(() => ({
        queryKey: ["onboarding"],
        queryFn: getOnboardingStatus,
        staleTime: 10000,
    }));
    const launcherAccounts = createQuery(() => ({
        queryKey: ["launcher-accounts"],
        queryFn: listLauncherAccounts,
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
                    qc.invalidateQueries({ queryKey: ["onboarding"] });
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

    // Shared VC import logic used by paste / file / scan
    async function doImportVc(vcJson: string, sourceLabel?: string): Promise<boolean> {
        setVcImporting(true);
        setVcMessage("");
        try {
            const result = await importVc(vcJson);
            qc.invalidateQueries({ queryKey: ["vc"] });
            qc.invalidateQueries({ queryKey: ["onboarding"] });
            const label = sourceLabel ? ` (${sourceLabel})` : "";
            if (result.state === "Member") {
                setVcMessage("✓ VC 导入并验证成功" + label);
            } else if (result.expired) {
                setVcMessage("⚠ VC 已导入但已过期，需续签" + label);
            } else if (!result.verified) {
                setVcMessage("⚠ VC 已导入但签名验证失败" + label);
            } else {
                setVcMessage("⚠ VC 已导入但状态异常" + label);
            }
            setVcInput("");
            setVcFileName("");
            return true;
        } catch (e) {
            setVcMessage("✗ VC 解析失败: " + e);
            return false;
        } finally {
            setVcImporting(false);
        }
    }

    async function handleImportVc() {
        await doImportVc(vcInput());
        setVcInput("");
    }

    // File import: open system file picker for .json, read & import
    async function handleImportVcFile() {
        setVcFileName("");
        setVcMessage("");
        try {
            const selected = await open({
                filters: [{ name: "JSON 文件", extensions: ["json"] }],
                multiple: false,
            });
            if (!selected) return; // user cancelled
            const path = typeof selected === "string" ? selected : String(selected);
            if (!path) return;
            const fileName = path.split(/[/\\]/).pop() ?? path;
            setVcFileName(fileName);
            const content = await readTextFile(path);
            await doImportVc(content, fileName);
        } catch (e) {
            setVcMessage("✗ 文件读取失败: " + e);
        }
    }

    // QR scan: open camera, decode QR via jsQR, feed into import
    async function handleQrScanned(data: string) {
        setVcScanOpen(false);
        await doImportVc(data, "扫码");
    }

    async function handleLogout() {
        await logoutMua();
        qc.invalidateQueries({ queryKey: ["mua-status"] });
        qc.invalidateQueries({ queryKey: ["onboarding"] });
    }

    async function handleClearVc() {
        await clearVc();
        qc.invalidateQueries({ queryKey: ["vc"] });
        qc.invalidateQueries({ queryKey: ["onboarding"] });
        setVcMessage("");
    }

    async function handleAddOfflineAccount() {
        setAccountError("");
        const name = addAccountName().trim();
        if (!name) {
            setAccountError("请输入玩家名称");
            return;
        }
        try {
            await addOfflineAccount({ username: name });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setAddAccountName("");
        } catch (e) {
            setAccountError(String(e));
        }
    }

    async function handleSelectAccount(id: string) {
        try {
            await selectLauncherAccount(id);
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
        } catch (e) {
            setAccountError(String(e));
        }
    }

    async function handleDeleteAccount(id: string) {
        if (!confirm("确定删除此账户？注意：仅删除启动器记录，不会影响本地文件。")) return;
        try {
            await deleteLauncherAccount(id);
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
        } catch (e) {
            setAccountError(String(e));
        }
    }

    async function handleAddThirdPartyAccount() {
        setTpError("");
        setTpSuccess("");
        const url = tpAuthUrl().trim();
        const username = tpUsername().trim();
        const password = tpPassword();
        if (!url) { setTpError("请输入认证服务器地址"); return; }
        if (!username) { setTpError("请输入用户名或邮箱"); return; }
        if (!password) { setTpError("请输入密码"); return; }
        setTpLoading(true);
        try {
            const result = await addThirdPartyAccount({
                auth_server_url: url,
                username_or_email: username,
                password,
            });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setTpSuccess(`登录成功：${result.account.username}`);
            setTpPassword(""); // clear password after success
        } catch (e) {
            setTpError(String(e));
        } finally {
            setTpLoading(false);
        }
    }

    async function beginMicrosoftLogin() {
        setMsError("");
        setMsSuccess("");
        try {
            const resp = await startMicrosoftLogin();
            setMsDeviceCode(resp.device_code);
            setMsUserCode(resp.user_code);
            setMsVerificationUri(resp.verification_uri);
            setMsVerificationUriComplete(resp.verification_uri_complete ?? null);
            setMsDialogOpen(true);
            try {
                const { openUrl } = await import("@tauri-apps/plugin-opener");
                const uri = resp.verification_uri_complete ?? resp.verification_uri;
                await openUrl(uri);
            } catch { /* opener may not be available */ }
        } catch (e) {
            setMsError(String(e));
        }
    }

    async function handleMicrosoftPoll() {
        setMsError("");
        setMsPolling(true);
        try {
            const result = await pollMicrosoftLogin(msDeviceCode());
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setMsSuccess(`登录成功：${result.account.username}`);
            setMsDialogOpen(false);
        } catch (e) {
            setMsError(String(e));
        } finally {
            setMsPolling(false);
        }
    }

    async function handleRefreshMicrosoftAccount(accountId: string) {
        setMsRefreshLoading((prev) => ({ ...prev, [accountId]: true }));
        setMsRefreshError((prev) => ({ ...prev, [accountId]: "" }));
        setMsRefreshSuccess((prev) => ({ ...prev, [accountId]: "" }));
        try {
            const result = await refreshMicrosoftAccount(accountId);
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setMsRefreshSuccess((prev) => ({
                ...prev,
                [accountId]: `刷新成功：${result.account.username}`,
            }));
        } catch (e) {
            setMsRefreshError((prev) => ({ ...prev, [accountId]: String(e) }));
        } finally {
            setMsRefreshLoading((prev) => ({ ...prev, [accountId]: false }));
        }
    }

    async function handleSetAvatarUrl(accountId: string) {
        const url = (avatarUrlInput()[accountId] ?? "").trim();
        if (!url) {
            setAvatarError((prev) => ({ ...prev, [accountId]: "请输入头像URL" }));
            return;
        }
        setAvatarLoading((prev) => ({ ...prev, [accountId]: true }));
        setAvatarError((prev) => ({ ...prev, [accountId]: "" }));
        setAvatarSuccess((prev) => ({ ...prev, [accountId]: "" }));
        try {
            await updateAccountAvatar({ account_id: accountId, avatar_url: url });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setAvatarSuccess((prev) => ({
                ...prev,
                [accountId]: "头像已设置",
            }));
            setAvatarUrlInput((prev) => ({ ...prev, [accountId]: "" }));
        } catch (e) {
            setAvatarError((prev) => ({ ...prev, [accountId]: String(e) }));
        } finally {
            setAvatarLoading((prev) => ({ ...prev, [accountId]: false }));
        }
    }

    async function handleClearAvatar(accountId: string) {
        setAvatarLoading((prev) => ({ ...prev, [accountId]: true }));
        setAvatarError((prev) => ({ ...prev, [accountId]: "" }));
        setAvatarSuccess((prev) => ({ ...prev, [accountId]: "" }));
        try {
            await updateAccountAvatar({ account_id: accountId, avatar_url: null });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setAvatarSuccess((prev) => ({ ...prev, [accountId]: "头像已清除" }));
        } catch (e) {
            setAvatarError((prev) => ({ ...prev, [accountId]: String(e) }));
        } finally {
            setAvatarLoading((prev) => ({ ...prev, [accountId]: false }));
        }
    }

    async function handleRefreshAvatar(accountId: string) {
        setAvatarLoading((prev) => ({ ...prev, [accountId]: true }));
        setAvatarError((prev) => ({ ...prev, [accountId]: "" }));
        setAvatarSuccess((prev) => ({ ...prev, [accountId]: "" }));
        try {
            await refreshAccountAvatar(accountId);
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setAvatarSuccess((prev) => ({ ...prev, [accountId]: "头像已刷新" }));
        } catch (e) {
            setAvatarError((prev) => ({ ...prev, [accountId]: String(e) }));
        } finally {
            setAvatarLoading((prev) => ({ ...prev, [accountId]: false }));
        }
    }

    async function handleExportAccounts() {
        setAccountError("");
        try {
            const bundle = await exportLauncherAccounts();
            const json = JSON.stringify(bundle, null, 2);
            setExportedJson(json);
            setShowExport(true);
        } catch (e) {
            setAccountError("导出失败: " + String(e));
        }
    }

    async function handleImportAccounts() {
        setImportError("");
        setImportResult("");
        const json = importBundleJson().trim();
        if (!json) {
            setImportError("请粘贴要导入的账户 JSON");
            return;
        }
        setImporting(true);
        try {
            const result = await importLauncherAccounts({
                bundle_json: json,
                dedupe_by_uuid: dedupeByUuid(),
            });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setImportResult(
                `导入完成：成功 ${result.imported}，跳过 ${result.skipped}，失败 ${result.failed}（共 ${result.total}）`,
            );
            setImportBundleJson("");
        } catch (e) {
            setImportError("导入失败: " + String(e));
        } finally {
            setImporting(false);
        }
    }

    async function handleImportExternalAccounts() {
        setExternalImportError("");
        setExternalImportResult("");
        const json = externalAccountsJson().trim();
        if (!json) {
            setExternalImportError("请先粘贴 Prism/MultiMC 的 accounts.json 内容");
            return;
        }
        setExternalImporting(true);
        try {
            const result = await importExternalAccounts({
                source: "prism-multimc",
                accounts_json: json,
                dedupe_by_uuid: externalDedupe(),
            });
            qc.invalidateQueries({ queryKey: ["launcher-accounts"] });
            setExternalImportResult(
                `导入完成：成功 ${result.imported}，跳过 ${result.skipped}，失败 ${result.failed}（共 ${result.total}）`,
            );
            setExternalAccountsJson("");
        } catch (e) {
            setExternalImportError("导入失败: " + String(e));
        } finally {
            setExternalImporting(false);
        }
    }

    return (
        <div class="h-full overflow-y-auto px-8 py-5">
            <h2 class="text-2xl font-black text-stone-950">我的</h2>

            <div class="mt-6 grid gap-5">
                {/* Onboarding / Identity Guidance */}
                <Show when={onboarding.data}>
                    {(status) => (
                        <section class="rounded-xl border border-stone-200 bg-white p-6">
                            <h3 class="font-bold text-stone-800">身份引导</h3>

                            <dl class="mt-4 divide-y divide-stone-100">
                                <Row label="PeerID" value={truncatePeerId(status().peer_id)} mono />
                                <div class="flex items-center justify-between py-2.5 text-sm">
                                    <span class="text-stone-500">身份模式</span>
                                    <span class={onboardingModeBadge(status().is_member, status().is_guest)}>
                                        {status().mode_label}
                                    </span>
                                </div>
                                <Row
                                    label="社团"
                                    value={status().club ?? "未绑定社团"}
                                    accent={status().club ? "success" : "neutral"}
                                />
                                <Show when={status().requires_network}>
                                    <div class="flex items-center justify-between py-2.5 text-sm">
                                        <span class="text-stone-500">吊销列表状态</span>
                                        <span class="font-medium text-amber-600">
                                            ⚠ 吊销列表缓存已过期，需要联网刷新
                                        </span>
                                    </div>
                                </Show>
                            </dl>

                            <Show when={status().is_guest}>
                                <div class="mt-3 rounded-lg bg-amber-50 border border-amber-200 px-4 py-3">
                                    <p class="text-sm text-amber-800">
                                        ⚠ 当前为 MUA 访客模式：仅游戏功能，无治理/积分。
                                    </p>
                                </div>
                            </Show>

                            <Show when={status().next_steps.length > 0}>
                                <div class="mt-4">
                                    <h4 class="text-sm font-semibold text-stone-700">下一步</h4>
                                    <ul class="mt-2 space-y-1.5">
                                        <For each={status().next_steps}>
                                            {(step) => (
                                                <li class="flex items-start gap-2 text-sm text-stone-600">
                                                    <span class="mt-1 shrink-0 text-teal-600">▸</span>
                                                    <span>{step}</span>
                                                </li>
                                            )}
                                        </For>
                                    </ul>
                                </div>
                            </Show>
                        </section>
                    )}
                </Show>

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
                            <Show when={bootstrap.data}>
                                {(bs) => (
                                    <div class="flex items-center justify-between py-2.5 text-sm">
                                        <span class="text-stone-500">引导节点</span>
                                        <span class={
                                            bs().configured
                                                ? "font-medium text-stone-700"
                                                : "font-medium text-amber-600"
                                        }>
                                            {bs().configured
                                                ? `已配置 ${bs().peer_count} 个`
                                                : "未配置引导节点"}
                                        </span>
                                    </div>
                                )}
                            </Show>
                        </dl>
                    </Show>
                </section>

                {/* Network Diagnostics Panel (P1: DESIGN.md section 3.4 bullet 3) */}
                <NetworkDiagnosticsPanel
                    {...(peers.data ? { peers: peers.data } : {})}
                    {...(proxy.data ? { proxyPort: proxy.data.local_port } : {})}
                    {...(bootstrap.data ? { bootstrap: bootstrap.data } : {})}
                />

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

                {/* Minecraft 账户 */}
                <section class="rounded-xl border border-stone-200 bg-white p-6">
                    <h3 class="font-bold text-stone-800">Minecraft 账户</h3>
                    <p class="mt-1 text-xs text-stone-400">
                        离线账户用于本地实例启动；联邦服务器仍需要上方 MUA 登录。
                    </p>

                    <Show when={launcherAccounts.data} keyed>
                        {(accounts) => (
                            <div class="mt-4">
                                <Show when={accounts.length > 0} fallback={
                                    <p class="text-sm text-stone-400">暂无本地账户，请添加离线账户。</p>
                                }>
                                    <div class="grid gap-2">
                                        <For each={accounts}>
                                            {(account: LauncherAccount) => (
                                                <div class="rounded-lg border border-stone-200 px-4 py-3">
                                                    {/* Row 1: avatar + info + actions */}
                                                    <div class="flex items-start gap-3">
                                                        {/* Avatar */}
                                                        <div class="shrink-0">
                                                            <Show
                                                                when={account.avatar_url}
                                                                fallback={
                                                                    <div
                                                                        class="flex h-12 w-12 items-center justify-center rounded-full text-lg font-bold text-white"
                                                                        style={{ "background-color": avatarBgColor(account.kind) }}
                                                                    >
                                                                        {account.username.charAt(0).toUpperCase()}
                                                                    </div>
                                                                }
                                                            >
                                                                {(url) => (
                                                                    <img
                                                                        src={url()}
                                                                        alt={account.username}
                                                                        class="h-12 w-12 rounded-full object-cover border border-stone-200"
                                                                    />
                                                                )}
                                                            </Show>
                                                        </div>

                                                        {/* Info */}
                                                        <div class="min-w-0 flex-1">
                                                            <div class="flex items-center gap-2">
                                                                <span class="font-medium text-stone-800 truncate">
                                                                    {account.username}
                                                                </span>
                                                                <span class="text-xs text-stone-400 font-mono">
                                                                    {accountKindLabel(account.kind)}
                                                                </span>
                                                                <Show when={account.selected}>
                                                                    <span class="inline-flex items-center rounded-full bg-teal-100 px-2 py-0.5 text-xs font-medium text-teal-800">
                                                                        当前
                                                                    </span>
                                                                </Show>
                                                            </div>
                                                            <p class="mt-0.5 font-mono text-xs text-stone-400 truncate">
                                                                {account.uuid}
                                                            </p>
                                                            <Show when={account.kind === "ThirdParty" && account.auth_server_url}>
                                                                {(url) => (
                                                                    <p class="mt-0.5 text-xs text-stone-400 truncate">
                                                                        {shortAuthUrl(url())}
                                                                    </p>
                                                                )}
                                                            </Show>
                                                            <Show when={msRefreshSuccess()[account.id]}>
                                                                <p class="mt-0.5 text-xs text-teal-600">
                                                                    {msRefreshSuccess()[account.id]}
                                                                </p>
                                                            </Show>
                                                            <Show when={msRefreshError()[account.id]}>
                                                                <p class="mt-0.5 text-xs text-red-600">
                                                                    {msRefreshError()[account.id]}
                                                                </p>
                                                            </Show>
                                                        </div>

                                                        {/* Actions */}
                                                        <div class="ml-3 flex shrink-0 gap-1.5">
                                                            <Show when={account.kind === "Microsoft"}>
                                                                <button
                                                                    class="rounded-md px-2.5 py-1 text-xs font-medium text-teal-600 hover:bg-teal-50 border border-teal-200 disabled:opacity-50"
                                                                    disabled={msRefreshLoading()[account.id]}
                                                                    onClick={() => handleRefreshMicrosoftAccount(account.id)}
                                                                >
                                                                    {msRefreshLoading()[account.id] ? "刷新中…" : "刷新"}
                                                                </button>
                                                            </Show>
                                                            <Show when={!account.selected}>
                                                                <button
                                                                    class="rounded-md px-2.5 py-1 text-xs font-medium text-stone-600 hover:bg-stone-100 border border-stone-200"
                                                                    onClick={() => handleSelectAccount(account.id)}
                                                                >
                                                                    选择
                                                                </button>
                                                            </Show>
                                                            <button
                                                                class="rounded-md px-2.5 py-1 text-xs font-medium text-red-600 hover:bg-red-50 border border-red-200"
                                                                onClick={() => handleDeleteAccount(account.id)}
                                                            >
                                                                删除
                                                            </button>
                                                        </div>
                                                    </div>

                                                    {/* Row 2: avatar management */}
                                                    <div class="mt-3 border-t border-stone-100 pt-3">
                                                        <div class="flex gap-2 items-center">
                                                            <input
                                                                class="flex-1 rounded-md border border-stone-300 bg-white px-2 py-1.5 text-xs placeholder:text-stone-400 focus:border-teal-500 focus:outline-none"
                                                                placeholder="自定义头像URL (https://...)"
                                                                value={avatarUrlInput()[account.id] ?? ""}
                                                                onInput={(e) => setAvatarUrlInput((prev) => ({ ...prev, [account.id]: e.currentTarget.value }))}
                                                                onKeyDown={(e) => { if (e.key === "Enter") handleSetAvatarUrl(account.id); }}
                                                            />
                                                            <button
                                                                class="rounded-md px-2.5 py-1 text-xs font-medium text-teal-600 hover:bg-teal-50 border border-teal-200 disabled:opacity-50 shrink-0"
                                                                disabled={avatarLoading()[account.id]}
                                                                onClick={() => handleSetAvatarUrl(account.id)}
                                                            >
                                                                {avatarLoading()[account.id] ? "…" : "设置"}
                                                            </button>
                                                            <button
                                                                class="rounded-md px-2.5 py-1 text-xs font-medium text-amber-600 hover:bg-amber-50 border border-amber-200 disabled:opacity-50 shrink-0"
                                                                disabled={avatarLoading()[account.id]}
                                                                onClick={() => handleRefreshAvatar(account.id)}
                                                            >
                                                                刷新头像
                                                            </button>
                                                            <button
                                                                class="rounded-md px-2.5 py-1 text-xs font-medium text-stone-500 hover:bg-stone-100 border border-stone-200 disabled:opacity-50 shrink-0"
                                                                disabled={avatarLoading()[account.id]}
                                                                onClick={() => handleClearAvatar(account.id)}
                                                            >
                                                                清除
                                                            </button>
                                                        </div>
                                                        <Show when={avatarSuccess()[account.id]}>
                                                            <p class="mt-1 text-xs text-teal-600">
                                                                {avatarSuccess()[account.id]}
                                                            </p>
                                                        </Show>
                                                        <Show when={avatarError()[account.id]}>
                                                            <p class="mt-1 text-xs text-red-600">
                                                                {avatarError()[account.id]}
                                                            </p>
                                                        </Show>
                                                    </div>
                                                </div>
                                            )}
                                        </For>
                                    </div>
                                </Show>

                                {/* Add offline account form */}
                                <div class="mt-4 flex gap-2">
                                    <input
                                        class="flex-1 rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm placeholder:text-stone-400 focus:border-teal-500 focus:outline-none"
                                        placeholder="玩家名称 (3-16 字符，字母数字下划线)"
                                        value={addAccountName()}
                                        onInput={(e) => setAddAccountName(e.currentTarget.value)}
                                        onKeyDown={(e) => { if (e.key === "Enter") handleAddOfflineAccount(); }}
                                    />
                                    <button
                                        class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900 shrink-0"
                                        onClick={handleAddOfflineAccount}
                                    >
                                        添加离线账户
                                    </button>
                                </div>

                                {/* Microsoft login button */}
                                <div class="mt-4 border-t border-stone-100 pt-4">
                                    <h4 class="text-sm font-semibold text-stone-700">导出 / 导入账户</h4>
                                    <p class="mt-1 text-xs text-stone-400">
                                        不会导出或导入登录 token；Microsoft/第三方账户导入后可能需要重新登录/刷新。
                                    </p>

                                    {/* Export */}
                                    <div class="mt-3">
                                        <button
                                            class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100 text-sm"
                                            onClick={handleExportAccounts}
                                        >
                                            导出账户 JSON
                                        </button>
                                        <Show when={showExport()}>
                                            <textarea
                                                class="mt-3 w-full rounded-lg border border-stone-300 bg-stone-50 p-3 font-mono text-xs focus:border-teal-500 focus:outline-none"
                                                rows={8}
                                                readOnly
                                                value={exportedJson()}
                                                onClick={(e) => (e.currentTarget as HTMLTextAreaElement).select()}
                                            />
                                        </Show>
                                    </div>

                                    {/* Import */}
                                    <div class="mt-4">
                                        <label class="text-xs text-stone-500">粘贴导出的 JSON：</label>
                                        <textarea
                                            class="mt-1 w-full rounded-lg border border-stone-300 bg-white p-3 font-mono text-xs focus:border-teal-500 focus:outline-none"
                                            rows={6}
                                            placeholder='粘贴之前导出的 account bundle JSON…'
                                            value={importBundleJson()}
                                            onInput={(e) => setImportBundleJson(e.currentTarget.value)}
                                        />
                                        <div class="mt-2 flex items-center gap-3">
                                            <label class="flex items-center gap-1.5 text-xs text-stone-600 cursor-pointer">
                                                <input
                                                    type="checkbox"
                                                    class="rounded border-stone-300 text-teal-600 focus:ring-teal-500"
                                                    checked={dedupeByUuid()}
                                                    onChange={(e) => setDedupeByUuid(e.currentTarget.checked)}
                                                />
                                                按 kind+UUID 跳过重复
                                            </label>
                                            <button
                                                class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900 text-sm"
                                                disabled={importing() || !importBundleJson().trim()}
                                                onClick={handleImportAccounts}
                                            >
                                                {importing() ? "导入中…" : "导入账户"}
                                            </button>
                                        </div>
                                        <Show when={importResult()}>
                                            <p class="mt-2 text-sm text-teal-700">{importResult()}</p>
                                        </Show>
                                        <Show when={importError()}>
                                            <p class="mt-2 text-sm text-red-600">{importError()}</p>
                                        </Show>
                                    </div>

                                    {/* External (Prism/MultiMC) import */}
                                    <div class="mt-4 border-t border-stone-100 pt-4">
                                        <h4 class="text-sm font-semibold text-stone-700">从 Prism / MultiMC 导入</h4>
                                        <p class="mt-1 text-xs text-stone-400">
                                            从 Prism Launcher 或 MultiMC 的 accounts.json 导入账户元数据。
                                            仅导入账户类型（离线/Microsoft）和公开信息；不导入登录凭据。
                                            Microsoft 账户导入后需重新登录。
                                        </p>
                                        <label class="mt-3 block text-xs text-stone-500">粘贴 accounts.json：</label>
                                        <textarea
                                            class="mt-1 w-full rounded-lg border border-stone-300 bg-white p-3 font-mono text-xs focus:border-teal-500 focus:outline-none"
                                            rows={8}
                                            placeholder='粘贴 Prism/MultiMC 的 accounts.json 内容…'
                                            value={externalAccountsJson()}
                                            onInput={(e) => setExternalAccountsJson(e.currentTarget.value)}
                                        />
                                        <div class="mt-2 flex items-center gap-3">
                                            <label class="flex items-center gap-1.5 text-xs text-stone-600 cursor-pointer">
                                                <input
                                                    type="checkbox"
                                                    class="rounded border-stone-300 text-teal-600 focus:ring-teal-500"
                                                    checked={externalDedupe()}
                                                    onChange={(e) => setExternalDedupe(e.currentTarget.checked)}
                                                />
                                                按 kind+UUID 跳过重复
                                            </label>
                                            <button
                                                class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900 text-sm"
                                                disabled={externalImporting() || !externalAccountsJson().trim()}
                                                onClick={handleImportExternalAccounts}
                                            >
                                                {externalImporting() ? "导入中…" : "导入外部账户"}
                                            </button>
                                        </div>
                                        <Show when={externalImportResult()}>
                                            <p class="mt-2 text-sm text-teal-700">{externalImportResult()}</p>
                                        </Show>
                                        <Show when={externalImportError()}>
                                            <p class="mt-2 text-sm text-red-600">{externalImportError()}</p>
                                        </Show>
                                    </div>
                                </div>

                                {/* Microsoft login button */}
                                <div class="mt-4 border-t border-stone-100 pt-4">
                                    <h4 class="text-sm font-semibold text-stone-700">Microsoft 正版登录</h4>
                                    <p class="mt-1 text-xs text-stone-400">
                                        使用设备码通过浏览器完成 Microsoft 授权，登录后可使用正版皮肤。
                                    </p>
                                    <Show when={msSuccess()}>
                                        <p class="mt-2 text-sm text-teal-700">{msSuccess()}</p>
                                    </Show>
                                    <Show when={!msDialogOpen()}>
                                        <button
                                            class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                            onClick={beginMicrosoftLogin}
                                        >
                                            Microsoft 登录
                                        </button>
                                    </Show>
                                </div>

                                {/* Third-party Yggdrasil login form */}
                                <div class="mt-5 border-t border-stone-100 pt-4">
                                    <h4 class="text-sm font-semibold text-stone-700">第三方 Yggdrasil 登录</h4>
                                    <div class="mt-2 space-y-2">
                                        <input
                                            class="w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm placeholder:text-stone-400 focus:border-teal-500 focus:outline-none"
                                            placeholder="认证服务器地址 (https://...)"
                                            value={tpAuthUrl()}
                                            onInput={(e) => setTpAuthUrl(e.currentTarget.value)}
                                        />
                                        <input
                                            class="w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm placeholder:text-stone-400 focus:border-teal-500 focus:outline-none"
                                            placeholder="用户名 / 邮箱"
                                            value={tpUsername()}
                                            onInput={(e) => setTpUsername(e.currentTarget.value)}
                                        />
                                        <input
                                            type="password"
                                            class="w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm placeholder:text-stone-400 focus:border-teal-500 focus:outline-none"
                                            placeholder="密码"
                                            value={tpPassword()}
                                            onInput={(e) => setTpPassword(e.currentTarget.value)}
                                            onKeyDown={(e) => { if (e.key === "Enter") handleAddThirdPartyAccount(); }}
                                        />
                                        <button
                                            class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900 w-full"
                                            disabled={tpLoading()}
                                            onClick={handleAddThirdPartyAccount}
                                        >
                                            {tpLoading() ? "登录中…" : "第三方登录"}
                                        </button>
                                    </div>
                                    <Show when={tpError()}>
                                        <p class="mt-2 text-sm text-red-600">{tpError()}</p>
                                    </Show>
                                    <Show when={tpSuccess()}>
                                        <p class="mt-2 text-sm text-teal-700">{tpSuccess()}</p>
                                    </Show>
                                </div>

                                <Show when={accountError()}>
                                    <p class="mt-2 text-sm text-red-600">{accountError()}</p>
                                </Show>
                            </div>
                        )}
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
                                <p class="text-sm text-stone-500">暂无有效身份凭证。选择以下方式导入社团签发的 VC：</p>

                                {/* Import method tabs */}
                                <div class="mt-3 flex border-b border-stone-200">
                                    <button
                                        class={`px-4 py-2 text-sm font-medium transition-colors ${vcImportTab() === "paste" ? "border-b-2 border-teal-600 text-teal-700" : "text-stone-500 hover:text-stone-700"}`}
                                        onClick={() => { setVcImportTab("paste"); setVcMessage(""); }}
                                    >
                                        粘贴 JSON
                                    </button>
                                    <button
                                        class={`px-4 py-2 text-sm font-medium transition-colors ${vcImportTab() === "file" ? "border-b-2 border-teal-600 text-teal-700" : "text-stone-500 hover:text-stone-700"}`}
                                        onClick={() => { setVcImportTab("file"); setVcMessage(""); }}
                                    >
                                        从文件导入
                                    </button>
                                    <button
                                        class={`px-4 py-2 text-sm font-medium transition-colors ${vcImportTab() === "scan" ? "border-b-2 border-teal-600 text-teal-700" : "text-stone-500 hover:text-stone-700"}`}
                                        onClick={() => { setVcImportTab("scan"); setVcMessage(""); }}
                                    >
                                        扫码导入
                                    </button>
                                </div>

                                {/* Paste JSON tab */}
                                <Show when={vcImportTab() === "paste"}>
                                    <textarea
                                        class="mt-3 w-full rounded-lg border border-stone-300 bg-white p-3 font-mono text-xs focus:border-teal-500 focus:outline-none"
                                        rows={4}
                                        placeholder='{"@context": [...], "type": [...], ...}'
                                        value={vcInput()}
                                        onInput={(e) => setVcInput(e.currentTarget.value)}
                                    />
                                    <button
                                        class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                        disabled={vcImporting() || !vcInput().trim()}
                                        onClick={handleImportVc}
                                    >
                                        {vcImporting() ? "导入中…" : "导入 VC"}
                                    </button>
                                </Show>

                                {/* File import tab */}
                                <Show when={vcImportTab() === "file"}>
                                    <div class="mt-3">
                                        <p class="text-sm text-stone-500">选择包含 VC 的 JSON 文件：</p>
                                        <Show when={vcFileName()}>
                                            <p class="mt-2 rounded-lg bg-stone-50 px-3 py-2 font-mono text-xs text-stone-700">
                                                已选文件: {vcFileName()}
                                            </p>
                                        </Show>
                                        <button
                                            class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                            disabled={vcImporting()}
                                            onClick={handleImportVcFile}
                                        >
                                            {vcImporting() ? "导入中…" : "选择 JSON 文件并导入"}
                                        </button>
                                    </div>
                                </Show>

                                {/* QR scan tab */}
                                <Show when={vcImportTab() === "scan"}>
                                    <div class="mt-3">
                                        <p class="text-sm text-stone-500">使用摄像头扫描包含 VC 的二维码：</p>
                                        <button
                                            class="btn mt-3 rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                            disabled={vcImporting()}
                                            onClick={() => setVcScanOpen(true)}
                                        >
                                            打开扫码
                                        </button>
                                    </div>
                                </Show>

                                <Show when={vcMessage()}>
                                    <p class={`mt-3 text-sm ${vcMessage().startsWith("✓") ? "text-teal-700" : vcMessage().startsWith("⚠") ? "text-amber-600" : "text-red-600"}`}>
                                        {vcMessage()}
                                    </p>
                                </Show>
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

            {/* QR Scanner Dialog */}
            <Show when={vcScanOpen()}>
                <QrScannerDialog
                    onClose={() => setVcScanOpen(false)}
                    onScan={(data) => handleQrScanned(data)}
                />
            </Show>

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

            {/* Microsoft Device Code Dialog */}
            <Show when={msDialogOpen()}>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
                    <div class="w-full max-w-sm rounded-2xl border border-stone-200 bg-white p-6 shadow-2xl">
                        <h3 class="text-lg font-bold text-stone-900">Microsoft 设备验证</h3>
                        <p class="mt-2 text-sm text-stone-500">
                            请在浏览器中打开以下链接并输入验证码：
                        </p>
                        <div class="mt-4 space-y-3">
                            <div>
                                <p class="text-xs text-stone-400">验证码</p>
                                <div class="rounded-xl bg-stone-50 py-3 text-center">
                                    <p class="font-mono text-2xl font-bold tracking-[0.3em] text-stone-900">
                                        {msUserCode()}
                                    </p>
                                </div>
                            </div>
                            <div>
                                <p class="text-xs text-stone-400">验证链接</p>
                                <p class="mt-1 break-all text-sm font-mono text-stone-600">
                                    {msVerificationUriComplete() ?? msVerificationUri()}
                                </p>
                            </div>
                        </div>
                        <Show when={msError()}>
                            <p class="mt-3 text-sm text-red-600">{msError()}</p>
                        </Show>
                        <div class="mt-5 flex gap-2 justify-end">
                            <button
                                class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                                onClick={() => {
                                    setMsDialogOpen(false);
                                    setMsPolling(false);
                                }}
                            >
                                取消
                            </button>
                            <button
                                class="btn rounded-lg bg-teal-800 text-white hover:bg-teal-900"
                                disabled={msPolling()}
                                onClick={handleMicrosoftPoll}
                            >
                                {msPolling() ? "登录中…" : "我已完成授权，继续"}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
}

function truncatePeerId(peerId: string): string {
    if (peerId.length <= 14) return peerId;
    return peerId.slice(0, 7) + "…" + peerId.slice(-7);
}

function onboardingModeBadge(isMember: boolean, isGuest: boolean): string {
    if (isMember) return "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-teal-100 text-teal-800";
    if (isGuest) return "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-amber-100 text-amber-800";
    return "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium bg-stone-100 text-stone-500";
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

function accountKindLabel(kind: LauncherAccountKind): string {
    switch (kind) {
        case "Offline": return "离线";
        case "Microsoft": return "Microsoft";
        case "ThirdParty": return "第三方";
    }
}

function avatarBgColor(kind: LauncherAccountKind): string {
    switch (kind) {
        case "Offline": return "#78716c"; // stone-500
        case "Microsoft": return "#0d9488"; // teal-600
        case "ThirdParty": return "#d97706"; // amber-600
    }
}

function shortAuthUrl(url: string): string {
    // Remove scheme and trailing path noise for display
    let s = url.replace(/^https?:\/\//, "");
    if (s.length > 36) {
        s = s.slice(0, 33) + "...";
    }
    return s;
}

// ── QR Scanner Dialog ──

function QrScannerDialog(props: {
    onClose: () => void;
    onScan: (data: string) => void;
}) {
    let videoEl: HTMLVideoElement | undefined;
    let canvasEl: HTMLCanvasElement | undefined;
    let stream: MediaStream | undefined;
    let rafId: number | undefined;
    const [scanError, setScanError] = createSignal("");

    onCleanup(() => {
        if (rafId != null) cancelAnimationFrame(rafId);
        if (stream) {
            stream.getTracks().forEach((t) => t.stop());
            stream = undefined;
        }
    });

    onMount(() => {
        startCamera();
    });

    async function startCamera() {
        try {
            const s = await navigator.mediaDevices.getUserMedia({
                video: { facingMode: "environment" },
            });
            stream = s;
            if (videoEl) {
                videoEl.srcObject = s;
                await videoEl.play();
            }
            scanLoop();
        } catch (e: any) {
            if (e.name === "NotAllowedError" || e.name === "PermissionDeniedError") {
                setScanError("摄像头权限被拒绝，请在系统设置中允许相机访问");
            } else if (e.name === "NotFoundError") {
                setScanError("未找到摄像头设备");
            } else {
                setScanError("无法打开摄像头: " + (e.message || String(e)));
            }
        }
    }

    function scanLoop() {
        if (!videoEl || !canvasEl) {
            rafId = requestAnimationFrame(scanLoop);
            return;
        }
        const canvas = canvasEl;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
            rafId = requestAnimationFrame(scanLoop);
            return;
        }
        if (videoEl.readyState >= videoEl.HAVE_ENOUGH_DATA) {
            canvas.width = videoEl.videoWidth;
            canvas.height = videoEl.videoHeight;
            ctx.drawImage(videoEl, 0, 0, canvas.width, canvas.height);
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            try {
                const code = jsQR(imageData.data, imageData.width, imageData.height);
                if (code) {
                    stopCamera();
                    props.onScan(code.data);
                    return;
                }
            } catch {
                // jsQR may throw on non-QR frames; continue
            }
        }
        rafId = requestAnimationFrame(scanLoop);
    }

    function stopCamera() {
        if (rafId != null) {
            cancelAnimationFrame(rafId);
            rafId = undefined;
        }
        if (stream) {
            stream.getTracks().forEach((t) => t.stop());
            stream = undefined;
        }
    }

    return (
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
            <div class="w-full max-w-sm rounded-2xl border border-stone-200 bg-white p-6 shadow-2xl">
                <h3 class="text-lg font-bold text-stone-900">扫码导入 VC</h3>

                <Show when={scanError()}>
                    <div class="mt-3 rounded-lg bg-red-50 border border-red-200 px-4 py-3">
                        <p class="text-sm text-red-700">{scanError()}</p>
                    </div>
                </Show>

                <div class="mt-4 overflow-hidden rounded-lg bg-black">
                    <video
                        ref={(el) => (videoEl = el)}
                        class="w-full"
                        autoplay
                        playsinline
                        muted
                    />
                    <canvas ref={(el) => (canvasEl = el)} class="hidden" />
                </div>

                <Show when={!scanError()}>
                    <p class="mt-3 text-xs text-stone-400">将 VC 二维码对准摄像头，将自动识别</p>
                </Show>

                <div class="mt-4 flex justify-end">
                    <button
                        class="btn rounded-lg border-stone-300 bg-transparent text-stone-700 hover:bg-stone-100"
                        onClick={() => {
                            stopCamera();
                            props.onClose();
                        }}
                    >
                        取消
                    </button>
                </div>
            </div>
        </div>
    );
}

// ── Network Diagnostics Panel (P1: DESIGN.md section 3.4 bullet 3) ──

function NetworkDiagnosticsPanel(props: {
    peers?: string[];
    proxyPort?: number;
    bootstrap?: BootstrapStatus | null;
}) {
    const [diagnostics] = createResource(
        () => (props.peers?.length ?? 0) > 0,
        async () => {
            try {
                return await getNetworkDiagnostics();
            } catch {
                return null;
            }
        },
    );

    const d = () => diagnostics();

    return (
        <section class="rounded-xl border border-stone-200 bg-white p-6">
            <h3 class="font-bold text-stone-800">网络诊断</h3>

            <div class="mt-4 grid gap-3">
                {/* DHT & Connection summary */}
                <div class="grid grid-cols-2 gap-3">
                    <MetricTile
                        label="DHT 节点"
                        value={d()?.dht_peers ?? 0}
                        sub={props.peers ? `${props.peers.length} 已连接` : "加载中…"}
                        kind="info"
                    />
                    <MetricTile
                        label="活跃连接"
                        value={d()?.connected_peers ?? 0}
                        sub={d()?.relay_connected ? "含中继" : "直连"}
                        kind="info"
                    />
                </div>
                <div class="grid grid-cols-2 gap-3">
                    <MetricTile
                        label="代理端口"
                        value={props.proxyPort ?? "—"}
                        sub="本地 Minecraft 代理"
                        kind="neutral"
                    />
                    <MetricTile
                        label="引导节点"
                        value={props.bootstrap?.peer_count ?? 0}
                        sub={
                            props.bootstrap?.configured
                                ? (d()?.bootstrap_reachable ? "可达" : "未连接")
                                : "未配置"
                        }
                        kind={props.bootstrap?.configured ? "info" : "warn"}
                    />
                </div>
                {/* Sessions & bytes */}
                <div class="grid grid-cols-3 gap-3">
                    <MetricTile
                        label="活跃会话"
                        value={d()?.active_sessions ?? 0}
                        sub="代理会话数"
                        kind="info"
                    />
                    <MetricTile
                        label="接收"
                        value={d()?.total_bytes_rx != null ? fmtBytes(d()!.total_bytes_rx) : "—"}
                        sub="累计字节"
                        kind="neutral"
                    />
                    <MetricTile
                        label="发送"
                        value={d()?.total_bytes_tx != null ? fmtBytes(d()!.total_bytes_tx) : "—"}
                        sub="累计字节"
                        kind="neutral"
                    />
                </div>
                {/* NAT traversal */}
                <div class="grid grid-cols-2 gap-3">
                    <MetricTile
                        label="NAT 穿透成功"
                        value={d()?.dcutr_holes_punched ?? 0}
                        sub="DCUtR"
                        kind={d() && d()!.dcutr_holes_punched > 0 ? "info" : "neutral"}
                    />
                    <MetricTile
                        label="连接失败"
                        value={d()?.dcutr_failures ?? 0}
                        sub="累计"
                        kind={d() && d()!.dcutr_failures > 0 ? "warn" : "neutral"}
                    />
                </div>

                {/* Latency table */}
                <Show when={d()?.latencies && d()!.latencies.length > 0}>
                    <div class="mt-2 border-t border-stone-100 pt-3">
                        <p class="text-xs font-semibold text-stone-500 mb-2">
                            节点延迟 (ms)
                        </p>
                        <div class="max-h-40 overflow-y-auto grid gap-1">
                            <For each={d()!.latencies}>
                                {(latency) => (
                                    <div class="flex items-center justify-between text-xs">
                                        <span class="font-mono text-stone-600 truncate mr-2">
                                            {latency.peer_id.slice(0, 12)}…
                                        </span>
                                        <span
                                            class={`font-mono font-semibold ${
                                                latency.stale
                                                    ? "text-stone-400"
                                                    : latency.latency_ms < 50
                                                        ? "text-teal-600"
                                                        : latency.latency_ms < 150
                                                            ? "text-amber-600"
                                                            : "text-red-500"
                                            }`}
                                        >
                                            {latency.latency_ms} ms
                                            {latency.stale ? " (缓存)" : ""}
                                        </span>
                                    </div>
                                )}
                            </For>
                        </div>
                    </div>
                </Show>
            </div>
        </section>
    );
}

function MetricTile(props: {
    label: string;
    value: number | string;
    sub: string;
    kind: "info" | "warn" | "neutral";
}) {
    const valueColor = () => {
        switch (props.kind) {
            case "info": return "text-teal-700";
            case "warn": return "text-amber-700";
            default: return "text-stone-700";
        }
    };
    return (
        <div class="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2.5">
            <p class="text-xs text-stone-500">{props.label}</p>
            <p class={`mt-0.5 text-lg font-bold ${valueColor()}`}>
                {props.value}
            </p>
            <p class="text-xs text-stone-400">{props.sub}</p>
        </div>
    );
}

function fmtBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}
