import { createSignal, createResource, createMemo, For, Show } from "solid-js";
import {
  fetchGameVersionList,
  fetchResourceListByName,
  type GameClientResourceInfo,
  type OtherResourceInfo,
  type OtherResourceSource,
} from "../api/tauri";

const RESOURCE_TYPES = [
  { key: "mod", label: "Mod" },
  { key: "modpack", label: "Modpack" },
  { key: "resourcepack", label: "Resource Pack" },
  { key: "shader", label: "Shader" },
  { key: "world", label: "World" },
  { key: "datapack", label: "Data Pack" },
];

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
                      <a
                        href={resource.websiteUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn btn-xs btn-outline btn-primary mt-2"
                      >
                        View Details
                      </a>
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
    </div>
  );
}
