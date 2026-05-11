import { createSignal, Show, For, createEffect, onCleanup } from "solid-js";
import { createQuery, useQueryClient } from "@tanstack/solid-query";
import {
    retrieveLauncherConfig,
    updateLauncherConfig,
    selectGameDir,
    selectJavaPath,
    retrieveJavaList,
    validateJava,
} from "../services";
import type { LauncherConfig, CloseBehavior, JavaRuntime } from "../services";

function defaultLauncherConfig(): LauncherConfig {
    return {
        basic: {
            language: "zh-CN",
            theme: "system",
            close_behavior: "Ask",
            download_threads: 4,
        },
        game: {
            java_path: "",
            max_memory_mb: 4096,
            jvm_args: "",
            game_directory: "",
            resolution_width: 1280,
            resolution_height: 720,
            fullscreen: false,
            show_game_log: false,
        },
        java: {
            auto_scan: true,
            auto_select: true,
            preferred_java_path: null,
        },
        advanced: {
            enable_process_monitor: true,
            enable_crash_report: true,
            keep_launcher_open: true,
        },
    };
}

export function SettingsPage() {
    const qc = useQueryClient();
    const [saving, setSaving] = createSignal(false);
    const [savedMsg, setSavedMsg] = createSignal("");
    const [javaScanning, setJavaScanning] = createSignal(false);
    const [javaList, setJavaList] = createSignal<JavaRuntime[] | null>(null);
    const [javaScanError, setJavaScanError] = createSignal<string | null>(null);
    const [javaValidating, setJavaValidating] = createSignal<string | null>(null);
    const [javaValidation, setJavaValidation] = createSignal<Record<string, boolean | null>>({});

    const settingsQuery = createQuery(() => ({
        queryKey: ["launcher-config"],
        queryFn: retrieveLauncherConfig,
    }));

    const [form, setForm] = createSignal<LauncherConfig>(defaultLauncherConfig());

    createEffect(() => {
        const s = settingsQuery.data;
        if (!s) return;
        setForm(s);
    });

    let savedTimer: ReturnType<typeof setTimeout>;
    onCleanup(() => clearTimeout(savedTimer));

    const f = () => form();

    async function handleSave() {
        setSaving(true);
        setSavedMsg("");
        const current = f();
        try {
            await updateLauncherConfig(current);
            qc.invalidateQueries({ queryKey: ["launcher-config"] });
            qc.invalidateQueries({ queryKey: ["game-settings"] });
            setSavedMsg("设置已保存");
            savedTimer = setTimeout(() => setSavedMsg(""), 3000);
        } catch (e) {
            setSavedMsg("保存失败: " + String(e));
        } finally {
            setSaving(false);
        }
    }

    async function handleScanJava() {
        setJavaScanning(true);
        setJavaScanError(null);
        setJavaList(null);
        setJavaValidation({});
        try {
            const list = await retrieveJavaList();
            setJavaList(list);
            if (list.length === 0) {
                setJavaScanError("未找到已安装的 Java 运行时");
            }
        } catch (e) {
            setJavaScanError(String(e));
        } finally {
            setJavaScanning(false);
        }
    }

    async function handleValidateJava(javaPath: string) {
        setJavaValidating(javaPath);
        try {
            const ok = await validateJava(javaPath);
            setJavaValidation((prev) => ({ ...prev, [javaPath]: ok }));
        } catch (e) {
            setJavaValidation((prev) => ({ ...prev, [javaPath]: false }));
        } finally {
            setJavaValidating(null);
        }
    }

    return (
        <div class="h-full overflow-y-auto px-8 py-5">
            <h2 class="text-2xl font-black text-stone-950">启动器设置</h2>

            <Show when={settingsQuery.isLoading}>
                <p class="mt-6 text-sm text-stone-400">加载中…</p>
            </Show>

            <Show when={!settingsQuery.isLoading}>
                <div class="mt-6 max-w-xl space-y-6">
                    {/* ── 启动器 ── */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">启动器</h3>
                        <div class="mt-4 space-y-4">
                            {/* Language */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">语言</label>
                                <select
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                    value={f().basic.language}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            basic: { ...p.basic, language: e.currentTarget.value },
                                        }))
                                    }
                                >
                                    <option value="zh-CN">简体中文</option>
                                    <option value="en-US">English</option>
                                </select>
                            </div>

                            {/* Theme */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">界面主题</label>
                                <select
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                    value={f().basic.theme}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            basic: { ...p.basic, theme: e.currentTarget.value },
                                        }))
                                    }
                                >
                                    <option value="system">跟随系统</option>
                                    <option value="light">浅色</option>
                                    <option value="dark">深色</option>
                                </select>
                                <p class="mt-1 text-xs text-stone-400">
                                    主题切换将在下次启动生效
                                </p>
                            </div>

                            {/* Close behavior */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">关闭行为</label>
                                <select
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                    value={f().basic.close_behavior}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            basic: {
                                                ...p.basic,
                                                close_behavior: e.currentTarget.value as CloseBehavior,
                                            },
                                        }))
                                    }
                                >
                                    <option value="Ask">询问</option>
                                    <option value="MinimizeToTray">最小化到托盘</option>
                                    <option value="Exit">直接退出</option>
                                </select>
                            </div>

                            {/* Download threads */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">
                                    下载线程数: {f().basic.download_threads}
                                </label>
                                <input
                                    type="range"
                                    min={1}
                                    max={16}
                                    step={1}
                                    class="mt-2 w-full"
                                    value={f().basic.download_threads}
                                    onInput={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            basic: {
                                                ...p.basic,
                                                download_threads: Number(e.currentTarget.value),
                                            },
                                        }))
                                    }
                                />
                            </div>
                        </div>
                    </section>

                    {/* ── 游戏 ── */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">游戏</h3>
                        <div class="mt-4 space-y-4">
                            {/* Java path */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">Java 路径</label>
                                <div class="mt-1 flex gap-2">
                                    <input
                                        class="flex-1 rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        placeholder="例如: C:\\Program Files\\Java\\jdk-21\\bin\\java.exe"
                                        value={f().game.java_path ?? ""}
                                        onInput={(e) =>
                                            setForm((p) => ({
                                                ...p,
                                                game: { ...p.game, java_path: e.currentTarget.value },
                                            }))
                                        }
                                    />
                                    <button
                                        type="button"
                                        class="btn rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-600 hover:bg-stone-50"
                                        onClick={async () => {
                                            const path = await selectJavaPath();
                                            if (path)
                                                setForm((p) => ({
                                                    ...p,
                                                    game: { ...p.game, java_path: path },
                                                }));
                                        }}
                                    >
                                        浏览…
                                    </button>
                                </div>
                            </div>

                            {/* Memory */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">
                                    最大内存: {f().game.max_memory_mb ?? 0} MB
                                </label>
                                <input
                                    type="range"
                                    min={512}
                                    max={32768}
                                    step={512}
                                    class="mt-2 w-full"
                                    value={f().game.max_memory_mb ?? 4096}
                                    onInput={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            game: {
                                                ...p.game,
                                                max_memory_mb: Number(e.currentTarget.value),
                                            },
                                        }))
                                    }
                                />
                                <p class="mt-1 text-xs text-stone-400">
                                    建议: 原版 2-4GB, 模组包 6-8GB+
                                </p>
                            </div>

                            {/* JVM args */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">自定义 JVM 参数</label>
                                <textarea
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm font-mono focus:border-teal-500 focus:outline-none"
                                    rows={3}
                                    placeholder="-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
                                    value={f().game.jvm_args ?? ""}
                                    onInput={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            game: { ...p.game, jvm_args: e.currentTarget.value },
                                        }))
                                    }
                                />
                            </div>

                            {/* Game directory */}
                            <div>
                                <label class="block text-sm font-medium text-stone-700">游戏目录</label>
                                <div class="mt-1 flex gap-2">
                                    <input
                                        class="flex-1 rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        placeholder="例如: D:\\Games\\.minecraft"
                                        value={f().game.game_directory ?? ""}
                                        onInput={(e) =>
                                            setForm((p) => ({
                                                ...p,
                                                game: {
                                                    ...p.game,
                                                    game_directory: e.currentTarget.value,
                                                },
                                            }))
                                        }
                                    />
                                    <button
                                        type="button"
                                        class="btn rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm text-stone-600 hover:bg-stone-50"
                                        onClick={async () => {
                                            const dir = await selectGameDir();
                                            if (dir)
                                                setForm((p) => ({
                                                    ...p,
                                                    game: { ...p.game, game_directory: dir },
                                                }));
                                        }}
                                    >
                                        浏览…
                                    </button>
                                </div>
                            </div>

                            {/* Resolution */}
                            <div class="flex gap-4">
                                <div class="flex-1">
                                    <label class="block text-sm font-medium text-stone-700">宽度</label>
                                    <input
                                        type="number"
                                        class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        value={f().game.resolution_width ?? 1280}
                                        onInput={(e) =>
                                            setForm((p) => ({
                                                ...p,
                                                game: {
                                                    ...p.game,
                                                    resolution_width: Number(e.currentTarget.value),
                                                },
                                            }))
                                        }
                                    />
                                </div>
                                <div class="flex-1">
                                    <label class="block text-sm font-medium text-stone-700">高度</label>
                                    <input
                                        type="number"
                                        class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        value={f().game.resolution_height ?? 720}
                                        onInput={(e) =>
                                            setForm((p) => ({
                                                ...p,
                                                game: {
                                                    ...p.game,
                                                    resolution_height: Number(e.currentTarget.value),
                                                },
                                            }))
                                        }
                                    />
                                </div>
                            </div>

                            {/* Fullscreen & Log */}
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="fullscreen"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().game.fullscreen ?? false}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            game: { ...p.game, fullscreen: e.currentTarget.checked },
                                        }))
                                    }
                                />
                                <label for="fullscreen" class="text-sm text-stone-700">全屏启动</label>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="showlog"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().game.show_game_log ?? false}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            game: { ...p.game, show_game_log: e.currentTarget.checked },
                                        }))
                                    }
                                />
                                <label for="showlog" class="text-sm text-stone-700">显示游戏日志窗口</label>
                            </div>
                        </div>
                    </section>

                    {/* ── Java ── */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">Java</h3>
                        <div class="mt-4 space-y-4">
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="auto_scan"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().java.auto_scan ?? true}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            java: { ...p.java, auto_scan: e.currentTarget.checked },
                                        }))
                                    }
                                />
                                <label for="auto_scan" class="text-sm text-stone-700">自动扫描 Java 运行时</label>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="auto_select"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().java.auto_select ?? true}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            java: { ...p.java, auto_select: e.currentTarget.checked },
                                        }))
                                    }
                                />
                                <label for="auto_select" class="text-sm text-stone-700">自动选择最高版本 Java</label>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-stone-700">首选 Java 路径</label>
                                <input
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                    placeholder="留空表示自动检测"
                                    value={f().java.preferred_java_path ?? ""}
                                    onInput={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            java: {
                                                ...p.java,
                                                preferred_java_path: e.currentTarget.value || null,
                                            },
                                        }))
                                    }
                                />
                            </div>
                        </div>

                        {/* Scan installed Java runtimes */}
                        <div class="mt-4 border-t border-stone-100 pt-4">
                            <button
                                class="btn rounded-lg bg-teal-800 px-4 py-2 text-sm text-white hover:bg-teal-900 disabled:opacity-50"
                                disabled={javaScanning()}
                                onClick={handleScanJava}
                            >
                                {javaScanning() ? "扫描中…" : "扫描已安装的 Java"}
                            </button>
                            <Show when={javaScanError()}>
                                <p class="mt-2 text-xs text-amber-700">{javaScanError()}</p>
                            </Show>
                            <Show when={javaList()}>
                                <div class="mt-3 max-h-60 overflow-y-auto rounded border border-stone-200 bg-stone-50">
                                    <For each={javaList()!}>
                                        {(java: JavaRuntime) => {
                                            const validated = javaValidation()[java.exec_path];
                                            const isChecking = javaValidating() === java.exec_path;
                                            return (
                                                <div class="flex items-center justify-between border-b border-stone-100 px-3 py-2 text-xs last:border-b-0">
                                                    <div class="min-w-0 flex-1">
                                                        <p class="truncate font-medium text-stone-800">{java.exec_path}</p>
                                                        <p class="text-stone-500">版本 {java.version} · {java.vendor}</p>
                                                    </div>
                                                    <button
                                                        class="btn ml-2 shrink-0 rounded bg-stone-200 px-2 py-1 text-xs text-stone-700 hover:bg-stone-300 disabled:opacity-50"
                                                        disabled={isChecking}
                                                        onClick={() => handleValidateJava(java.exec_path)}
                                                    >
                                                        {isChecking ? "…" : validated === true ? "✓ 有效" : validated === false ? "✗ 无效" : "验证"}
                                                    </button>
                                                </div>
                                            );
                                        }}
                                    </For>
                                </div>
                            </Show>
                        </div>
                    </section>

                    {/* ── 高级 ── */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">高级</h3>
                        <div class="mt-4 space-y-4">
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="enable_process_monitor"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().advanced.enable_process_monitor ?? true}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            advanced: {
                                                ...p.advanced,
                                                enable_process_monitor: e.currentTarget.checked,
                                            },
                                        }))
                                    }
                                />
                                <label for="enable_process_monitor" class="text-sm text-stone-700">
                                    启用进程监控
                                </label>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="enable_crash_report"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().advanced.enable_crash_report ?? true}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            advanced: {
                                                ...p.advanced,
                                                enable_crash_report: e.currentTarget.checked,
                                            },
                                        }))
                                    }
                                />
                                <label for="enable_crash_report" class="text-sm text-stone-700">
                                    启用崩溃报告
                                </label>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="keep_launcher_open"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().advanced.keep_launcher_open ?? true}
                                    onChange={(e) =>
                                        setForm((p) => ({
                                            ...p,
                                            advanced: {
                                                ...p.advanced,
                                                keep_launcher_open: e.currentTarget.checked,
                                            },
                                        }))
                                    }
                                />
                                <label for="keep_launcher_open" class="text-sm text-stone-700">
                                    启动游戏后保持启动器打开
                                </label>
                            </div>
                        </div>
                    </section>

                    <div class="flex items-center gap-4 pt-2">
                        <button
                            class="btn rounded-lg bg-teal-800 px-6 py-2 text-white hover:bg-teal-900"
                            disabled={saving()}
                            onClick={handleSave}
                        >
                            {saving() ? "保存中…" : "保存设置"}
                        </button>
                        <Show when={savedMsg()}>
                            <p
                                class={`text-sm ${
                                    savedMsg().startsWith("保存失败") ? "text-red-600" : "text-teal-700"
                                }`}
                            >
                                {savedMsg()}
                            </p>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    );
}
