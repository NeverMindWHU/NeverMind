import { invoke } from "@/lib/tauri";

/** 与 `GenerationTasksContext` 任务列表对齐的最小形状，避免循环依赖 */
export interface TraySyncTask {
  status: "running" | "success" | "failed";
  consumed?: boolean;
  batch?: unknown;
}

export type TrayGenerationState = "idle" | "running" | "ready" | "mixed";

export function computeTrayGenerationState(tasks: TraySyncTask[]): TrayGenerationState {
  const running = tasks.filter((t) => t.status === "running").length;
  const ready = tasks.filter(
    (t) => t.status === "success" && !t.consumed && t.batch
  ).length;
  if (running > 0 && ready > 0) return "mixed";
  if (running > 0) return "running";
  if (ready > 0) return "ready";
  return "idle";
}

export async function syncTrayGenerationState(state: TrayGenerationState): Promise<void> {
  await invoke("sync_tray_generation_state", { state });
}
