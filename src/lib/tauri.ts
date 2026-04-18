import { invoke as tauriInvoke, type InvokeArgs } from "@tauri-apps/api/core";
import { normalizeError, type CommandError } from "@/types/common";

/**
 * Tauri IPC 封装。
 *
 * 特性：
 * - 类型化泛型 `<T>`，直接拿到后端 `CommandResponse<T>.data` 或 原始返回值
 * - 统一 reject 归一化为 `CommandError`（后端本来就是这个结构，本地异常也会被包裹）
 * - 非 Tauri 环境（如在浏览器里用 `vite`）下也能可见地失败，而不是卡死
 */
export async function invoke<T>(command: string, args?: InvokeArgs): Promise<T> {
  if (!isTauri()) {
    throw <CommandError>{
      code: "NOT_IN_TAURI",
      message: `命令「${command}」只能在 Tauri 运行时内调用（当前在普通浏览器里）。`,
    };
  }
  try {
    return await tauriInvoke<T>(command, args);
  } catch (err) {
    throw normalizeError(err);
  }
}

export function isTauri(): boolean {
  if (typeof window === "undefined") return false;
  const w = window as unknown as {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(w.__TAURI_INTERNALS__ ?? w.__TAURI__);
}

/**
 * 把后端命令返回的 `CommandResponse<T>` 解包，直接拿到 `data`。
 * 用于 `list_due_reviews` / `submit_review_result` / settings 相关命令。
 */
export async function invokeData<T>(
  command: string,
  args?: InvokeArgs
): Promise<T> {
  const resp = await invoke<{ success: boolean; data: T }>(command, args);
  return resp.data;
}
