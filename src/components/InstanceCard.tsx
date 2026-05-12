import { createResource, Show } from "solid-js";
import type { Instance } from "../types";
import { AdmissionBadge } from "./AdmissionBadge";
import { checkInstanceEligibility } from "../services";

interface Props {
  instance: Instance;
  onJoin: () => void;
  onInvite?: () => void;
}

export function InstanceCard(props: Props) {

  const [eligibility] = createResource(
    () => props.instance.id,
    async (id) => {
      try {
        return await checkInstanceEligibility(id);
      } catch {
        return null;
      }
    },
  );

  const isBlocked = () => eligibility()?.eligible === false;

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
        <Show when={props.instance.admission_mode || eligibility()}>
          <div class="mt-1.5">
            <AdmissionBadge
              mode={props.instance.admission_mode ?? "unknown"}
              blockedReason={
                isBlocked() ? (eligibility()?.reason ?? null) : null
              }
            />
          </div>
        </Show>
        <p class="mt-0.5 text-xs text-stone-400">
          {props.instance.version}
        </p>
      </div>
      <p class="font-mono text-sm">{props.instance.players}</p>
      <p class="font-mono text-sm text-teal-800">{props.instance.latency != null ? `${props.instance.latency}ms` : "未测量"}</p>
      <div class="flex items-center gap-1.5">
        <Show when={props.onInvite}>
          <button
            class="btn btn-xs rounded-md border border-stone-300 bg-white text-stone-600 hover:bg-stone-100 hover:text-stone-800"
            onClick={(e) => { e.stopPropagation(); props.onInvite?.(); }}
            title="邀请玩家"
          >
            邀请
          </button>
        </Show>
        <button
          class="btn btn-sm rounded-md bg-teal-800 text-white hover:bg-teal-900 disabled:opacity-50 disabled:cursor-not-allowed"
          onClick={props.onJoin}
          disabled={isBlocked()}
          title={eligibility()?.reason ?? undefined}
        >
          加入
        </button>
      </div>
    </article>
  );
}
