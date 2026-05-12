import { Show } from "solid-js";
import type { AdmissionMode } from "../types";
import { admissionLabel, admissionColor } from "../types";

export interface AdmissionBadgeProps {
  mode: AdmissionMode;
  blockedReason?: string | null;
}


export function AdmissionBadge(props: AdmissionBadgeProps) {
  return (
    <div class="flex items-center gap-2">
      <span
        class={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold ${admissionColor(props.mode)}`}
        title={`准入模式: ${admissionLabel(props.mode)}`}
      >
        {admissionLabel(props.mode)}
      </span>
      <Show when={props.blockedReason}>
        {(reason) => (
          <span
            class="inline-flex items-center gap-1 rounded-full bg-red-50 border border-red-200 px-2 py-0.5 text-xs text-red-700"
            title={reason()}
          >
            <svg
              class="h-3 w-3 shrink-0"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z"
              />
            </svg>
            <span class="truncate max-w-[200px]">{reason()}</span>
          </span>
        )}
      </Show>
    </div>
  );
}
