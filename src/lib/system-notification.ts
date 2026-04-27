import { getSettings } from "@/features/settings/services/api";
import { isTauri } from "@/lib/tauri";

const TITLE = "NeverMind";

export type SystemNotificationResult =
  | { ok: true }
  | { ok: false; reason: "disabled" | "unsupported" | "denied" | "error" };

/**
 * Tauri 桌面端使用插件走原生通知（Linux 上为 D-Bus/libnotify，不依赖 WebView 的 Notification API）。
 * 浏览器/Vite 预览仍回退到 Web Notification。
 */
export async function sendNeverMindSystemNotification(
  body: string,
  options?: { honorNotificationSetting?: boolean }
): Promise<SystemNotificationResult> {
  const honor = options?.honorNotificationSetting ?? true;
  if (honor) {
    try {
      const settings = await getSettings();
      if (!settings.notificationEnabled) {
        return { ok: false, reason: "disabled" };
      }
    } catch {
      return { ok: false, reason: "error" };
    }
  }

  if (isTauri()) {
    return sendViaTauriPlugin(body);
  }
  return sendViaWebNotification(body);
}

async function sendViaTauriPlugin(body: string): Promise<SystemNotificationResult> {
  try {
    const {
      isPermissionGranted,
      requestPermission,
      sendNotification,
    } = await import("@tauri-apps/plugin-notification");

    let granted = await isPermissionGranted();
    if (!granted) {
      const p = await requestPermission();
      granted = p === "granted";
    }
    if (!granted) {
      return { ok: false, reason: "denied" };
    }

    sendNotification({ title: TITLE, body });
    return { ok: true };
  } catch {
    return { ok: false, reason: "error" };
  }
}

async function sendViaWebNotification(body: string): Promise<SystemNotificationResult> {
  if (typeof Notification === "undefined") {
    return { ok: false, reason: "unsupported" };
  }

  try {
    if (Notification.permission === "default") {
      await Notification.requestPermission();
    }
    if (Notification.permission !== "granted") {
      return { ok: false, reason: "denied" };
    }
    new Notification(TITLE, { body });
    return { ok: true };
  } catch {
    return { ok: false, reason: "error" };
  }
}

/** 截图生成任务已开始（受设置约束；失败时静默）。 */
export async function notifyScreenshotCardGenerationStarted(): Promise<void> {
  await sendNeverMindSystemNotification(
    "已开始根据截图生成知识卡片，可在应用内查看进度。"
  );
}

const TEST_BODY = "这是一条测试通知。若你看到本消息，说明系统通知工作正常。";

/** 设置页「测试通知」用；受「桌面通知」开关约束。 */
export async function sendTestSystemNotification(): Promise<SystemNotificationResult> {
  return sendNeverMindSystemNotification(TEST_BODY);
}
