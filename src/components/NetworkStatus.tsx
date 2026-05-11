import { For, Show, createResource } from "solid-js";
import { getNetworkDiagnostics } from "../services";

interface Props {
  peers: string[];
  isLoading: boolean;
}

export function NetworkStatus(props: Props) {
  const [diagnostics] = createResource(
    () => props.peers.length > 0,
    async () => {
      try {
        return await getNetworkDiagnostics();
      } catch {
        return null;
      }
    }
  );

  return (
    <section class="rounded-md border border-stone-300 bg-white/80 p-5">
      <h3 class="font-bold">网络诊断</h3>
      <div class="mt-4 grid gap-4">
        <Metric
          label="DHT 节点"
          value={props.isLoading ? "..." : String(props.peers.length)}
          fill={props.peers.length > 0 ? "62%" : "0%"}
        />
        <Show when={diagnostics()}>
          {(d) => (
            <>
              <Metric
                label="活跃连接"
                value={String(d().connected_peers)}
                fill={d().connected_peers > 0 ? "62%" : "0%"}
              />
              <Metric
                label="中继状态"
                value={d().relay_connected ? "已连接" : "未连接"}
                fill={d().relay_connected ? "62%" : "0%"}
              />
              <Metric
                label="NAT 穿透成功"
                value={String(d().dcutr_holes_punched)}
                fill={d().dcutr_holes_punched > 0 ? "44%" : "0%"}
              />
              <Metric
                label="连接失败"
                value={String(d().dcutr_failures)}
                fill={d().dcutr_failures > 0 ? "20%" : "0%"}
              />

              <Show when={d().latencies.length > 0}>
                <div class="mt-4 border-t border-stone-200 pt-3">
                  <p class="text-xs font-semibold text-stone-500 mb-2">节点延迟 (ms)</p>
                  <div class="max-h-32 overflow-y-auto grid gap-1">
                    <For each={d().latencies}>
                      {(latency) => (
                        <div class="flex items-center justify-between text-xs">
                          <span class="font-mono text-stone-600 truncate mr-2">
                            {latency.peer_id.slice(0, 12)}...
                          </span>
                          <span class={`font-mono font-semibold ${latency.stale ? "text-stone-400" : latency.latency_ms < 50 ? "text-teal-600" : latency.latency_ms < 150 ? "text-amber-600" : "text-red-500"}`}>
                            {latency.latency_ms} ms {latency.stale ? "(缓存)" : ""}
                          </span>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>
            </>
          )}
        </Show>
      </div>

      <Show when={props.peers.length > 0 && !diagnostics()}>
        <div class="mt-4 border-t border-stone-200 pt-3">
          <p class="text-xs font-semibold text-stone-500 mb-2">已连接节点</p>
          <div class="max-h-24 overflow-y-auto grid gap-1">
            <For each={props.peers}>
              {(peer) => (
                <p class="text-xs font-mono text-stone-600 truncate">
                  {peer.slice(0, 20)}...
                </p>
              )}
            </For>
          </div>
        </div>
      </Show>
    </section>
  );
}

function Metric(props: { label: string; value: string; fill: string }) {
  return (
    <div>
      <div class="mb-2 flex items-center justify-between text-sm">
        <span class="text-stone-600">{props.label}</span>
        <span class="font-mono font-semibold">{props.value}</span>
      </div>
      <div class="meter h-2 rounded-full" style={{ "--value": props.fill }} />
    </div>
  );
}
