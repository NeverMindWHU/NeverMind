/**
 * 小工具：日期 / 相对时间 / 文件大小 / 错误文案映射。
 */

const RTF = new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto" });

const RELATIVE_THRESHOLDS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ["year", 365 * 24 * 60 * 60],
  ["month", 30 * 24 * 60 * 60],
  ["day", 24 * 60 * 60],
  ["hour", 60 * 60],
  ["minute", 60],
  ["second", 1],
];

/** 把 ISO 时间转成「3 分钟前 / 2 小时后」等人类友好串。 */
export function formatRelative(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return "未设定";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const diffSec = Math.round((d.getTime() - now.getTime()) / 1000);
  for (const [unit, s] of RELATIVE_THRESHOLDS) {
    if (Math.abs(diffSec) >= s || unit === "second") {
      return RTF.format(Math.round(diffSec / s), unit);
    }
  }
  return iso;
}

/** 本地化完整时间：2026-04-18 09:30。 */
export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

/** 字节数 → KB/MB 可读串。 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** 错误码 → 用户面向的中文短句。无匹配则退回原始 message。 */
const ERROR_HINT: Record<string, string> = {
  INVALID_INPUT: "输入内容不合法",
  INVALID_SETTINGS: "设置内容不合法",
  INVALID_TIME_FORMAT: "时间格式应为 HH:mm",
  INVALID_PATH: "路径不合法",
  INVALID_REVIEW_OPERATION: "复习操作不合法",
  AI_TIMEOUT: "AI 响应超时，请稍后重试",
  AI_UNAVAILABLE: "AI 服务暂不可用",
  AI_RESPONSE_INVALID: "AI 返回内容无法解析",
  MODEL_CONNECTION_FAILED: "模型连接失败",
  MODEL_AUTH_FAILED: "模型鉴权失败，请检查 API Key",
  MODEL_PROFILE_NOT_FOUND: "模型配置不存在",
  GENERATION_BATCH_NOT_FOUND: "生成批次不存在",
  REVIEW_NOT_FOUND: "复习任务已过期",
  CARD_NOT_FOUND: "卡片不存在",
  DB_WRITE_FAILED: "数据保存失败",
  NOT_IN_TAURI: "请通过桌面应用启动，而不是浏览器",
};

export function humanizeError(
  err: { code?: string; message?: string } | null | undefined
): string {
  if (!err) return "未知错误";
  const hint = err.code ? ERROR_HINT[err.code] : undefined;
  if (hint) return err.message ? `${hint}（${err.message}）` : hint;
  return err.message || err.code || "未知错误";
}
