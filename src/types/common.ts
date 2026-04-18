/**
 * 后端 `utils::error::CommandError` 的 TS 镜像。
 * Tauri 会以 `Promise.reject(CommandError)` 抛出，前端 catch 到的就是这个对象。
 */
export interface CommandError {
  code: string;
  message: string;
}

/**
 * 后端多处用到的统一成功包装 `CommandResponse<T>`。
 * 失败走 Promise reject，前端不需要判断 `success` 字段。
 */
export interface CommandResponse<T> {
  success: true;
  data: T;
}

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as CommandError).code === "string" &&
    typeof (value as CommandError).message === "string"
  );
}

/**
 * 把任意错误归一化为用户可读的文案 + 稳定 code。
 */
export function normalizeError(err: unknown): CommandError {
  if (isCommandError(err)) return err;
  if (err instanceof Error) {
    return { code: "UNKNOWN", message: err.message };
  }
  return { code: "UNKNOWN", message: String(err) };
}
