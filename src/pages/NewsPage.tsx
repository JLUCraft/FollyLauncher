import { createResource, For, Show } from "solid-js";
import { fetchNewsPostSummaries, type NewsPostSummary } from "../api/tauri";

const MC_NEWS_URL = "https://net-secondary.web.minecraft-services.net/api/v1.0";

export default function NewsPage() {
  const [mcPosts] = createResource(async () => {
    const res = await fetchNewsPostSummaries([{ url: MC_NEWS_URL, cursor: null }]);
    return res.posts.slice(0, 12);
  });

  return (
    <div class="p-6 max-w-7xl mx-auto">
      <h1 class="text-2xl font-bold mb-6">News</h1>

      <Show when={mcPosts.loading}>
        <div class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg" />
        </div>
      </Show>

      <Show when={mcPosts() && !mcPosts.loading}>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <For each={mcPosts()}>
            {(post: NewsPostSummary) => (
              <a
                href={post.link}
                target="_blank"
                rel="noopener noreferrer"
                class="card bg-base-200 shadow-sm hover:shadow-md transition-shadow no-underline"
              >
                <Show when={post.imageSrc}>
                  <figure>
                    <img
                      src={post.imageSrc?.[0] ?? ""}
                      alt={post.title}
                      class="w-full h-48 object-cover"
                      onError={(e) => {
                        const target = e.currentTarget;
                        if (target instanceof HTMLImageElement) {
                          target.style.display = "none";
                        }
                      }}
                    />
                  </figure>
                </Show>
                <div class="card-body p-4">
                  <h3 class="card-title text-base">{post.title}</h3>
                  <p class="text-sm text-base-content/60 line-clamp-3">
                    {post.abstract || ""}
                  </p>
                  <div class="flex items-center gap-2 mt-2 text-xs text-base-content/40">
                    <span>{post.source?.name}</span>
                    <span>·</span>
                    <span>{new Date(post.createAt).toLocaleDateString()}</span>
                  </div>
                </div>
              </a>
            )}
          </For>
        </div>
      </Show>

      <Show when={!mcPosts() && !mcPosts.loading}>
        <div class="text-center py-12 text-base-content/50">
          No news available. Check back later for Minecraft and community updates.
        </div>
      </Show>
    </div>
  );
}
