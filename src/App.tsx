import { Show, type ParentProps } from "solid-js";
import { A, useLocation } from "@solidjs/router";
import { createQuery } from "@tanstack/solid-query";
import { getIdentity, getProxyPort, listPeers } from "./services";
import "./App.css";

const NAV = [
  { href: "/", label: "服务器" },
  { href: "/league", label: "联赛" },
  { href: "/discover", label: "资源" },
  { href: "/tasks", label: "任务" },
  { href: "/news", label: "新闻" },
  { href: "/profile", label: "我的" },
  { href: "/launch", label: "启动" },
  { href: "/settings", label: "设置" },
] as const;

function App(props: ParentProps) {
  const location = useLocation();
  const identity = createQuery(() => ({
    queryKey: ["identity"],
    queryFn: getIdentity,
  }));
  const proxy = createQuery(() => ({
    queryKey: ["proxy"],
    queryFn: getProxyPort,
  }));
  const peers = createQuery(() => ({
    queryKey: ["peers"],
    queryFn: listPeers,
    refetchInterval: 60000,
  }));

  const isActive = (href: string) =>
    href === "/" ? location.pathname === "/" : location.pathname.startsWith(href);

  return (
    <div class="grid h-screen grid-cols-[220px_1fr] overflow-hidden">
      {}
      <aside class="flex flex-col border-r border-stone-200 bg-stone-50/80 backdrop-blur">
        {}
        <div class="border-b border-stone-200 px-5 py-5">
          <p class="text-[10px] font-bold uppercase tracking-[0.2em] text-teal-700">
            FollyLauncher
          </p>
          <h1 class="mt-1.5 text-xl font-black leading-tight text-stone-900">
            MC 联邦入口
          </h1>
        </div>

        {}
        <nav class="flex flex-col gap-1 p-3">
          {NAV.map((item) => (
            <A
              href={item.href}
              class={`flex h-10 items-center rounded-lg px-3 text-sm font-medium transition-colors ${isActive(item.href)
                  ? "bg-teal-800 text-white shadow-sm"
                  : "text-stone-600 hover:bg-stone-200 hover:text-stone-900"
                }`}
            >
              {item.label}
            </A>
          ))}
        </nav>

        {}
        <div class="mt-auto border-t border-stone-200 px-5 py-4">
          <Show
            when={identity.data}
            fallback={<p class="text-xs text-stone-400">身份加载中…</p>}
          >
            <p class="text-xs font-semibold text-stone-700">
              {identity.data?.club ?? "未签发"}
            </p>
            <p class="mt-0.5 truncate font-mono text-[10px] text-stone-400">
              {identity.data?.peer_id.slice(0, 22)}…
            </p>
          </Show>
          <div class="mt-2 flex items-center justify-between text-[10px] text-stone-400">
            <Show when={proxy.data}>
              <span class="text-teal-600">代理 :{proxy.data?.local_port}</span>
            </Show>
            <span>{peers.data ? `${peers.data.length} 节点` : "节点数不可用"}</span>
          </div>
        </div>
      </aside>

      {}
      <main class="overflow-hidden bg-white">
        {props.children}
      </main>
    </div>
  );
}

export default App;
