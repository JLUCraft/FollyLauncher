import type { Instance } from "../types";

interface Props {
  instance: Instance;
  onJoin: () => void;
}

export function InstanceCard(props: Props) {
  return (
    <article class="grid grid-cols-[1fr_110px_100px_92px] items-center border-b border-stone-200 px-5 py-4 last:border-b-0"
    >
      <div>
        <div class="flex items-center gap-3">
          <h3 class="text-lg font-bold">{props.instance.name}</h3>
          <span
            class={`badge rounded ${
              props.instance.type === "service"
                ? "badge-primary"
                : "badge-secondary"
            }`}
          >
            {props.instance.type === "service" ? "服务" : "房间"}
          </span>
        </div>
        <p class="mt-1 text-sm text-stone-600">
          {props.instance.mode} · {props.instance.club} · {props.instance.state}
        </p>
        <p class="mt-0.5 text-xs text-stone-400">
          {props.instance.version}
        </p>
      </div>
      <p class="font-mono text-sm">{props.instance.players}</p>
      <p class="font-mono text-sm text-teal-800">{props.instance.latency != null ? `${props.instance.latency}ms` : "未测量"}</p>
      <button
        class="btn btn-sm rounded-md bg-teal-800 text-white hover:bg-teal-900"
        onClick={props.onJoin}
      >
        加入
      </button>
    </article>
  );
}
