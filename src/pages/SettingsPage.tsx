import { createSignal, Show, createEffect, onCleanup } from "solid-js";
import { createQuery, useQueryClient } from "@tanstack/solid-query";
import { getGameSettings, updateGameSettings } from "../api/tauri";
import type { GameSettings } from "../api/tauri";

export function SettingsPage() {
    const qc = useQueryClient();
    const [saving, setSaving] = createSignal(false);
    const [savedMsg, setSavedMsg] = createSignal("");

    const settingsQuery = createQuery(() => ({
        queryKey: ["game-settings"],
        queryFn: getGameSettings,
    }));

    const [form, setForm] = createSignal<GameSettings>({
        java_path: "",
        max_memory_mb: 4096,
        jvm_args: "",
        game_directory: "",
        resolution_width: 1280,
        resolution_height: 720,
        fullscreen: false,
        show_game_log: false,
    });

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
        if (!current.java_path?.trim()) {
            setSavedMsg("保存失败: Java 路径不能为空");
            setSaving(false);
            return;
        }
        if (!current.game_directory?.trim()) {
            setSavedMsg("保存失败: 游戏目录不能为空");
            setSaving(false);
            return;
        }
        try {
            await updateGameSettings({
                java_path: current.java_path.trim(),
                max_memory_mb: current.max_memory_mb,
                jvm_args: current.jvm_args.trim(),
                game_directory: current.game_directory.trim(),
                resolution_width: current.resolution_width,
                resolution_height: current.resolution_height,
                fullscreen: current.fullscreen,
                show_game_log: current.show_game_log,
            });
            qc.invalidateQueries({ queryKey: ["game-settings"] });
            setSavedMsg("设置已保存");
            savedTimer = setTimeout(() => setSavedMsg(""), 3000);
        } catch (e) {
            setSavedMsg("保存失败: " + String(e));
        } finally {
            setSaving(false);
        }
    }

    return (
        <div class="h-full overflow-y-auto px-8 py-5">
            <h2 class="text-2xl font-black text-stone-950">游戏设置</h2>

            <Show when={settingsQuery.isLoading}>
                <p class="mt-6 text-sm text-stone-400">加载中…</p>
            </Show>

            <Show when={!settingsQuery.isLoading}>
                <div class="mt-6 max-w-xl space-y-6">
                    {/* Java */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">Java 运行时</h3>
                        <div class="mt-4 space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-stone-700">Java 路径</label>
                                <input
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                    placeholder="例如: C:\\Program Files\\Java\\jdk-21\\bin\\java.exe"
                                    value={f().java_path ?? ""}
                                    onInput={(e) => setForm((p) => ({ ...p, java_path: e.currentTarget.value }))}
                                />
                            </div>
                        </div>
                    </section>

                    {/* Memory */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">内存与性能</h3>
                        <div class="mt-4 space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-stone-700">
                                    最大内存: {f().max_memory_mb ?? 0} MB
                                </label>
                                <input
                                    type="range"
                                    min={512}
                                    max={32768}
                                    step={512}
                                    class="mt-2 w-full"
                                    value={f().max_memory_mb ?? ""}
                                    onInput={(e) => setForm((p) => ({ ...p, max_memory_mb: Number(e.currentTarget.value) }))}
                                />
                                <p class="mt-1 text-xs text-stone-400">
                                    建议: 原版 2-4GB, 模组包 6-8GB+
                                </p>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-stone-700">自定义 JVM 参数</label>
                                <textarea
                                    class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm font-mono focus:border-teal-500 focus:outline-none"
                                    rows={3}
                                    placeholder="-XX:+UseG1GC -XX:+UnlockExperimentalVMOptions"
                                    value={f().jvm_args ?? ""}
                                    onInput={(e) => setForm((p) => ({ ...p, jvm_args: e.currentTarget.value }))}
                                />
                            </div>
                        </div>
                    </section>

                    {/* Game directory */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">游戏目录</h3>
                        <div class="mt-4">
                            <label class="block text-sm font-medium text-stone-700">游戏目录</label>
                            <input
                                class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                placeholder="例如: D:\\Games\\.minecraft"
                                value={f().game_directory ?? ""}
                                onInput={(e) => setForm((p) => ({ ...p, game_directory: e.currentTarget.value }))}
                            />
                        </div>
                    </section>

                    {/* Resolution */}
                    <section class="rounded-xl border border-stone-200 bg-white p-6">
                        <h3 class="font-bold text-stone-800">窗口设置</h3>
                        <div class="mt-4 space-y-4">
                            <div class="flex gap-4">
                                <div class="flex-1">
                                    <label class="block text-sm font-medium text-stone-700">宽度</label>
                                    <input
                                        type="number"
                                        class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        value={f().resolution_width ?? ""}
                                        onInput={(e) => setForm((p) => ({ ...p, resolution_width: Number(e.currentTarget.value) }))}
                                    />
                                </div>
                                <div class="flex-1">
                                    <label class="block text-sm font-medium text-stone-700">高度</label>
                                    <input
                                        type="number"
                                        class="mt-1 w-full rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
                                        value={f().resolution_height ?? ""}
                                        onInput={(e) => setForm((p) => ({ ...p, resolution_height: Number(e.currentTarget.value) }))}
                                    />
                                </div>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="fullscreen"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().fullscreen ?? false}
                                    onChange={(e) => setForm((p) => ({ ...p, fullscreen: e.currentTarget.checked }))}
                                />
                                <label for="fullscreen" class="text-sm text-stone-700">全屏启动</label>
                            </div>
                            <div class="flex items-center gap-3">
                                <input
                                    type="checkbox"
                                    id="showlog"
                                    class="h-4 w-4 rounded border-stone-300 text-teal-800 focus:ring-teal-500"
                                    checked={f().show_game_log ?? false}
                                    onChange={(e) => setForm((p) => ({ ...p, show_game_log: e.currentTarget.checked }))}
                                />
                                <label for="showlog" class="text-sm text-stone-700">显示游戏日志窗口</label>
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
                            <p class={`text-sm ${savedMsg().startsWith("保存失败") ? "text-red-600" : "text-teal-700"}`}>
                                {savedMsg()}
                            </p>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    );
}
