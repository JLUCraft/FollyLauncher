import { createSignal, createResource, createMemo, For, Show } from "solid-js";
import {
  fetchGameVersionList,
  fetchResourceListByName,
  fetchResourceVersionPacks,
  installResourceToInstance,
  startInstallResourceTask,
  listLocalInstances,
  type GameClientResourceInfo,
  type InstallResourceKind,
  type InstallResourceResult,
  type LocalInstance,
  type OtherResourceDependency,
  type OtherResourceFileInfo,
  type OtherResourceInfo,
  type OtherResourceSource,
  type ResourceDependencySummary,
  type AsyncInstallResourceStarted,
} from "../services";

const RESOURCE_TYPES = [
  { key: "mod", label: "Mod" },
  { key: "modpack", label: "Modpack" },
  { key: "resourcepack", label: "Resource Pack" },
  { key: "shader", label: "Shader" },
  { key: "world", label: "World" },
  { key: "datapack", label: "Data Pack" },
];

const INSTALLABLE_TYPES: Set<string> = new Set(["mod", "resourcepack", "shader"]);

function resourceTypeToInstallKind(rt: string): InstallResourceKind | null {
  switch (rt) {
    case "mod":
      return "Mod";
    case "resourcepack":
      return "ResourcePack";
    case "shader":
      return "ShaderPack";
    default:
      return null;
  }
}

/** Frontend mirror of the Rust normalize_dependency_relation.
 *  Returns the canonical category string. */
function normalizeRelation(raw: string): string {
  const s = raw.trim().toLowerCase();
  if (s === "required" || s === "mandatory") return "required";
  if (s === "optional" || s === "suggested" || s === "recommended") return "optional";
  if (s === "embedded" || s === "embeddedlibrary" || s === "included") return "embedded";
  if (s === "incompatible" || s === "conflicting" || s === "breaking") return "incompatible";
  return "other";
}

/** Count required deps in the list (frontend mirror for highlighting). */
function countRequired(deps: OtherResourceDependency[] | undefined): number {
  if (!deps) return 0;
  let count = 0;
  for (const d of deps) {
    if (normalizeRelation(d.relation) === "required") count++;
  }
  return count;
}

const SORT_OPTIONS: Record<string, string> = {
  CurseForge: "Popularity",
  Modrinth: "relevance",
};

export default function DiscoverPage() {
  const [resourceType, setResourceType] = createSignal("mod");
  const [searchQuery, setSearchQuery] = createSignal("");
  const [gameVersion, setGameVersion] = createSignal("");
  const [downloadSource, setDownloadSource] = createSignal<OtherResourceSource>("Modrinth");
  const [page, setPage] = createSignal(0);
  const pageSize = 24;

  const [gameVersions] = createResource(fetchGameVersionList);
  const [instances] = createResource(listLocalInstances);
  const [searchResults, { refetch }] = createResource(
    () => ({ resourceType: resourceType(), searchQuery: searchQuery(), gameVersion: gameVersion(), downloadSource: downloadSource(), page: page() }),
    async ({ resourceType, searchQuery, gameVersion, downloadSource, page }) => {
      if (!searchQuery) return null;
      return fetchResourceListByName(downloadSource, {
        resourceType,
        searchQuery,
        gameVersion,
        selectedTag: "All",
        sortBy: SORT_OPTIONS[downloadSource] || "relevance",
        page,
        pageSize,
      });
    },
  );

  const readyResults = createMemo(() => {
    const r = searchResults();
    return r && !searchResults.loading ? r : null;
  });

  const handleSearch = () => {
    setPage(0);
    refetch();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") handleSearch();
  };

  // ── Install flow state ──────────────────────────────────────────────────

  const [showInstallModal, setShowInstallModal] = createSignal(false);
  const [installResource, setInstallResource] = createSignal<OtherResourceInfo | null>(null);
  const [installFile, setInstallFile] = createSignal<OtherResourceFileInfo | null>(null);
  const [installStatus, setInstallStatus] = createSignal("");
  const [installLoading, setInstallLoading] = createSignal(false);
  const [installStep, setInstallStep] = createSignal<"select" | "confirm" | "done" | "error" | "asyncDone">("select");
  const [installResult, setInstallResult] = createSignal<InstallResourceResult | null>(null);

  // ── Async install state ────────────────────────────────────────
  const [asyncInstallLoading, setAsyncInstallLoading] = createSignal(false);
  const [asyncInstallError, setAsyncInstallError] = createSignal("");
  const [asyncInstallResult, setAsyncInstallResult] =
    createSignal<AsyncInstallResourceStarted | null>(null);

  const openInstallModal = (resource: OtherResourceInfo) => {
    setInstallResource(resource);
    setInstallFile(null);
    setInstallStatus("");
    setInstallStep("select");
    setShowInstallModal(true);
  };

  const closeInstallModal = () => {
    setShowInstallModal(false);
    setInstallResource(null);
    setInstallFile(null);
    setInstallStatus("");
    setInstallStep("select");
    setInstallLoading(false);
    setInstallResult(null);
    setAsyncInstallLoading(false);
    setAsyncInstallError("");
    setAsyncInstallResult(null);
  };

  const handleSelectInstanceForInstall = async (instanceId: string) => {
    const resource = installResource();
    if (!resource) return;
    setInstallLoading(true);
    try {
      const selected = instances()?.find((i) => i.id === instanceId);
      const packs = await fetchResourceVersionPacks(downloadSource(), {
        resourceId: resource.id,
        modLoader: "",
        gameVersions: [selected?.game_version || gameVersion() || ""],
      });
      const firstPack = packs[0];
      const firstItem = firstPack?.items[0];
      if (firstItem) {
        setInstallFile(firstItem);
        setInstallStatus("");
        setInstallStep("confirm");
      } else {
        setInstallStatus("No compatible files found for this version.");
        setInstallStep("error");
      }
    } catch (e) {
      setInstallStatus(`Failed to fetch files: ${e}`);
      setInstallStep("error");
    } finally {
      setInstallLoading(false);
    }
  };

  const handleConfirmInstall = async () => {
    const file = installFile();
    const resource = installResource();
    const instanceId = installSelectedInstanceId();
    if (!file || !resource || !instanceId) return;
    setInstallLoading(true);
    try {
      const kind = resourceTypeToInstallKind(resourceType());
      if (!kind) {
        setInstallStatus("Cannot install this resource type.");
        setInstallStep("error");
        setInstallLoading(false);
        return;
      }
      const result = await installResourceToInstance({
        instanceId,
        kind,
        file: { ...file },
        overwrite: false,
      });
      setInstallResult(result);
      setInstallStatus(`Installed to: ${result.destPath}\nFile: ${result.fileName} (${result.bytesWritten} bytes)`);
      setInstallStep("done");
    } catch (e) {
      setInstallStatus(`Install failed: ${e}`);
      setInstallStep("error");
    } finally {
      setInstallLoading(false);
    }
  };

  const handleAsyncInstall = async () => {
    const file = installFile();
    const resource = installResource();
    const instanceId = installSelectedInstanceId();
    if (!file || !resource || !instanceId) return;
    setAsyncInstallLoading(true);
    setAsyncInstallError("");
    try {
      const kind = resourceTypeToInstallKind(resourceType());
      if (!kind) {
        setAsyncInstallError("Cannot install this resource type.");
        setAsyncInstallLoading(false);
        return;
      }
      const result = await startInstallResourceTask({
        instanceId,
        kind,
        file: { ...file },
        overwrite: false,
      });
      setAsyncInstallResult(result);
      setInstallStep("asyncDone");
    } catch (e) {
      setAsyncInstallError(`后台安装启动失败: ${e}`);
    } finally {
      setAsyncInstallLoading(false);
    }
  };

  const [installSelectedInstanceId, setInstallSelectedInstanceId] = createSignal<string | null>(null);

  const selectInstanceAndFetch = (instanceId: string) => {
    setInstallSelectedInstanceId(instanceId);
    setAsyncInstallError("");
    setAsyncInstallResult(null);
    handleSelectInstanceForInstall(instanceId);
  };

  return (
    <div class="p-6 max-w-7xl mx-auto">
      <h1 class="text-2xl font-bold mb-6">Resource Browser</h1>

      {/* Filters */}
      <div class="flex flex-wrap gap-4 mb-6">
        {/* Resource Type Tabs */}
        <div class="tabs tabs-box">
          <For each={RESOURCE_TYPES}>
            {(rt) => (
              <a
                class={`tab ${resourceType() === rt.key ? "tab-active" : ""}`}
                onClick={() => setResourceType(rt.key)}
              >
                {rt.label}
              </a>
            )}
          </For>
        </div>

        {/* Source Toggle */}
        <div class="join">
          <button
            class={`join-item btn btn-sm ${downloadSource() === "Modrinth" ? "btn-active" : ""}`}
            onClick={() => setDownloadSource("Modrinth")}
          >
            Modrinth
          </button>
          <button
            class={`join-item btn btn-sm ${downloadSource() === "CurseForge" ? "btn-active" : ""}`}
            onClick={() => setDownloadSource("CurseForge")}
          >
            CurseForge
          </button>
        </div>

        {/* Game Version */}
        <select
          class="select select-bordered select-sm"
          value={gameVersion()}
          onChange={(e) => setGameVersion(e.currentTarget.value)}
        >
          <option value="">All Versions</option>
          <For each={(gameVersions() || []).slice(0, 30)}>
            {(v: GameClientResourceInfo) => (
              <option value={v.id}>{v.id}</option>
            )}
          </For>
        </select>

        {/* Search */}
        <div class="flex-1 join">
          <input
            type="text"
            class="input input-bordered input-sm w-full join-item"
            placeholder="Search mods, resource packs, worlds..."
            value={searchQuery()}
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
            onKeyDown={handleKeyDown}
          />
          <button class="btn btn-sm btn-primary join-item" onClick={handleSearch}>
            Search
          </button>
        </div>
      </div>

      {/* Results */}
      <Show when={searchResults.loading}>
        <div class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg" />
        </div>
      </Show>

      <Show when={readyResults()}>
        {(res) => (
          <>
            <p class="text-sm text-base-content/60 mb-4">
              Found {res().total ?? 0} results
            </p>
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
              <For each={res().list ?? []}>
                {(resource: OtherResourceInfo) => (
                  <div class="card bg-base-200 shadow-sm hover:shadow-md transition-shadow">
                    <figure class="px-4 pt-4">
                      <img
                        src={resource.iconSrc || "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'><rect width='64' height='64' fill='%23333'/></svg>"}
                        alt={resource.name}
                        class="w-16 h-16 rounded-lg object-cover"
                        onError={(e) => {
                          const target = e.currentTarget;
                          if (target instanceof HTMLImageElement) {
                            target.src =
                              "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'><rect width='64' height='64' fill='%23333'/></svg>";
                          }
                        }}
                      />
                    </figure>
                    <div class="card-body p-4">
                      <h3 class="card-title text-sm">
                        {resource.translatedName || resource.name}
                      </h3>
                      <p class="text-xs text-base-content/60 line-clamp-2">
                        {resource.translatedDescription || resource.description}
                      </p>
                      <div class="flex flex-wrap gap-1 mt-2">
                        <For each={resource.tags.slice(0, 3)}>
                          {(tag) => (
                            <span class="badge badge-xs badge-outline">{tag}</span>
                          )}
                        </For>
                      </div>
                      <div class="flex justify-between items-center mt-2 text-xs text-base-content/50">
                        <span>{resource.downloads.toLocaleString()} downloads</span>
                        <span>{resource.source}</span>
                      </div>
                      <div class="flex gap-1 mt-2">
                        <a
                          href={resource.websiteUrl}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="btn btn-xs btn-outline btn-primary flex-1"
                        >
                          View Details
                        </a>
                        <Show
                          when={INSTALLABLE_TYPES.has(resourceType())}
                          fallback={
                            <button
                              class="btn btn-xs btn-outline flex-1"
                              disabled
                              title="Install is only available for Mod, Resource Pack, and Shader"
                            >
                              Install
                            </button>
                          }
                        >
                          <button
                            class="btn btn-xs btn-secondary flex-1"
                            onClick={() => openInstallModal(resource)}
                          >
                            Install
                          </button>
                        </Show>
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>

            {/* Pagination */}
            <Show when={(res().total ?? 0) > pageSize}>
              <div class="flex justify-center gap-2 mt-6">
                <button
                  class="btn btn-sm"
                  disabled={page() === 0}
                  onClick={() => { setPage((p) => Math.max(0, p - 1)); }}
                >
                  Previous
                </button>
                <span class="flex items-center px-4 text-sm">
                  Page {page() + 1}
                </span>
                <button
                  class="btn btn-sm"
                  disabled={(res().list.length ?? 0) < pageSize}
                  onClick={() => { setPage((p) => p + 1); }}
                >
                  Next
                </button>
              </div>
            </Show>
          </>
        )}
      </Show>

      <Show when={!searchResults() && !searchResults.loading}>
        <div class="text-center py-12 text-base-content/50">
          Search for mods, resource packs, shaders, worlds, and more from Modrinth and CurseForge.
        </div>
      </Show>

      {/* Install Modal */}
      <Show when={showInstallModal()}>
        <div class="modal modal-open">
          <div class="modal-box max-w-md">
            <h3 class="font-bold text-lg mb-4">
              Install: {installResource()?.translatedName || installResource()?.name}
            </h3>

            <Show when={installLoading()}>
              <div class="flex justify-center py-4">
                <span class="loading loading-spinner loading-md" />
              </div>
            </Show>

            <Show when={!installLoading() && installStep() === "select"}>
              <p class="text-sm mb-4">Select a local instance to install this resource into:</p>
              <Show
                when={(instances()?.length ?? 0) > 0}
                fallback={
                  <p class="text-sm text-warning">
                    No local instances found. Create an instance first.
                  </p>
                }
              >
                <div class="space-y-2 max-h-48 overflow-y-auto">
                  <For each={instances()}>
                    {(inst: LocalInstance) => (
                      <button
                        class="btn btn-sm btn-block justify-start"
                        onClick={() => selectInstanceAndFetch(inst.id)}
                      >
                        <span class="flex-1 text-left">
                          {inst.name}
                          <span class="text-xs text-base-content/50 ml-2">
                            ({inst.game_version})
                          </span>
                        </span>
                        <span class="badge badge-xs">{inst.kind}</span>
                      </button>
                    )}
                  </For>
                </div>
              </Show>
            </Show>

            <Show when={!installLoading() && installStep() === "confirm"}>
              <p class="text-sm mb-4">Confirm installation of file:</p>
              <div class="bg-base-300 rounded-lg p-3 mb-4 text-sm">
                <div>
                  <span class="font-semibold">File: </span>
                  {installFile()?.fileName}
                </div>
                <div>
                  <span class="font-semibold">Size: </span>
                  {installFile()?.downloads.toLocaleString()} downloads
                </div>
                <Show when={installFile()?.sha1}>
                  <div>
                    <span class="font-semibold">SHA1: </span>
                    <span class="text-xs font-mono break-all">{installFile()?.sha1}</span>
                  </div>
                </Show>
              </div>

              {/* Dependencies display */}
              <Show when={(installFile()?.dependencies?.length ?? 0) > 0}>
                <div class="mb-4">
                  <Show when={countRequired(installFile()?.dependencies) > 0}>
                    <div class="alert alert-warning mb-2 text-sm py-2">
                      ⚠ 该资源存在必需前置依赖，建议手动安装列出的前置资源后再启动实例。
                    </div>
                  </Show>
                  <div class="text-sm font-semibold mb-1">Dependencies:</div>
                  <div class="bg-base-300 rounded-lg max-h-40 overflow-y-auto">
                    <For each={installFile()?.dependencies ?? []}>
                      {(dep: OtherResourceDependency) => {
                        const cat = normalizeRelation(dep.relation);
                        const isRequired = cat === "required";
                        return (
                          <div
                            class={`flex justify-between items-center px-3 py-1.5 text-xs border-b border-base-200 last:border-b-0 ${
                              isRequired ? "bg-amber-900/30 border-l-2 border-l-amber-500" : ""
                            }`}
                          >
                            <span class="font-mono">{dep.resourceId}</span>
                            <span class={`badge badge-xs ${isRequired ? "badge-warning" : "badge-ghost"}`}>
                              {dep.relation}
                            </span>
                          </div>
                        );
                      }}
                    </For>
                  </div>
                </div>
              </Show>

              {/* Async install error */}
              <Show when={asyncInstallError()}>
                <div class="alert alert-error mb-4 text-sm py-2">
                  {asyncInstallError()}
                </div>
              </Show>

              <div class="flex gap-2">
                <button
                  class="btn btn-sm btn-primary flex-1"
                  onClick={handleConfirmInstall}
                  disabled={asyncInstallLoading()}
                >
                  Confirm Install
                </button>
                <button
                  class="btn btn-sm btn-secondary flex-1"
                  onClick={handleAsyncInstall}
                  disabled={asyncInstallLoading() || installLoading()}
                >
                  <Show when={asyncInstallLoading()}>
                    <span class="loading loading-spinner loading-xs mr-1" />
                  </Show>
                  后台安装
                </button>
              </div>
              <div class="flex gap-2 mt-2">
                <button
                  class="btn btn-sm btn-ghost"
                  onClick={() => {
                    setInstallStep("select");
                    setInstallFile(null);
                    setInstallSelectedInstanceId(null);
                  }}
                >
                  Back
                </button>
              </div>
            </Show>

            <Show
              when={
                !asyncInstallLoading() &&
                installStep() === "asyncDone"
              }
            >
              <div class="alert alert-success mb-4 text-sm">
                <div class="font-semibold mb-1">已在后台启动资源安装</div>
                <div class="text-xs">
                  Group ID:{" "}
                  <span class="font-mono">{asyncInstallResult()?.groupId}</span>
                </div>
                <div class="text-xs mt-1">
                  可前往「任务」页面查看进度。
                </div>
              </div>
              <button class="btn btn-sm btn-primary w-full" onClick={closeInstallModal}>
                Close
              </button>
            </Show>

            <Show when={!installLoading() && installStep() === "done"}>
              <div class="alert alert-success mb-4">
                <pre class="text-sm whitespace-pre-wrap">{installStatus()}</pre>
              </div>

              {/* Dependency summary */}
              <Show when={installResult()?.dependencySummary} keyed>
                {(ds: ResourceDependencySummary) => (
                  <div class="mb-4">
                    <Show when={ds.required > 0}>
                      <div class="alert alert-warning mb-2 text-sm py-2">
                        ⚠ 该资源存在 {ds.required} 个必需前置依赖未安装。
                      </div>
                    </Show>
                    <div class="bg-base-300 rounded-lg p-3 text-sm">
                      <div class="font-semibold mb-2">Dependency Summary</div>
                      <div class="grid grid-cols-4 gap-2 text-center">
                        <div>
                          <div class="text-amber-400 font-bold text-lg">{ds.required}</div>
                          <div class="text-xs text-base-content/60">Required</div>
                        </div>
                        <div>
                          <div class="text-blue-400 font-bold text-lg">{ds.optional}</div>
                          <div class="text-xs text-base-content/60">Optional</div>
                        </div>
                        <div>
                          <div class="text-green-400 font-bold text-lg">{ds.embedded}</div>
                          <div class="text-xs text-base-content/60">Embedded</div>
                        </div>
                        <div>
                          <div class="text-base-content/50 font-bold text-lg">{ds.other}</div>
                          <div class="text-xs text-base-content/60">Other</div>
                        </div>
                      </div>
                      <Show when={ds.items.length > 0}>
                        <div class="mt-2 pt-2 border-t border-base-200">
                          <div class="text-xs text-base-content/50 mb-1">Items ({ds.items.length})</div>
                          <div class="max-h-24 overflow-y-auto">
                            <For each={ds.items.slice(0, 10)}>
                              {(dep: OtherResourceDependency) => (
                                <div class="flex justify-between text-xs py-0.5">
                                  <span class="font-mono text-base-content/70">{dep.resourceId}</span>
                                  <span class="text-base-content/50">{dep.relation}</span>
                                </div>
                              )}
                            </For>
                            <Show when={ds.items.length > 10}>
                              <div class="text-xs text-base-content/40 italic">
                                ...and {ds.items.length - 10} more
                              </div>
                            </Show>
                          </div>
                        </div>
                      </Show>
                    </div>
                  </div>
                )}
              </Show>

              <button class="btn btn-sm btn-primary w-full" onClick={closeInstallModal}>
                Close
              </button>
            </Show>

            <Show when={!installLoading() && installStep() === "error"}>
              <div class="alert alert-error mb-4">
                <pre class="text-sm whitespace-pre-wrap">{installStatus()}</pre>
              </div>
              <div class="flex gap-2">
                <button
                  class="btn btn-sm btn-ghost flex-1"
                  onClick={() => {
                    setInstallStep("select");
                    setInstallFile(null);
                    setInstallSelectedInstanceId(null);
                    setInstallStatus("");
                  }}
                >
                  Try Again
                </button>
                <button class="btn btn-sm btn-primary flex-1" onClick={closeInstallModal}>
                  Close
                </button>
              </div>
            </Show>
          </div>
          <div class="modal-backdrop" onClick={closeInstallModal}>
            <button class="cursor-default">close</button>
          </div>
        </div>
      </Show>
    </div>
  );
}
