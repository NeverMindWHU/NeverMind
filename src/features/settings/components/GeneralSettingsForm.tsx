import { useEffect, useState } from "react";
import { Bell, Save } from "lucide-react";
import { Button } from "@/components/Button";
import { FieldLabel, Input, Panel, Select } from "@/components/Card";
import type {
  AppSettingsData,
  Language,
  ThemeMode,
  UpdateSettingsInput,
} from "@/types/settings";
import { useToast } from "@/lib/toast";
import { sendTestSystemNotification } from "@/lib/system-notification";

interface Props {
  settings: AppSettingsData;
  saving: boolean;
  onSave: (next: UpdateSettingsInput) => void | Promise<unknown>;
}

export function GeneralSettingsForm({ settings, saving, onSave }: Props) {
  const toast = useToast();
  const [testingNotify, setTestingNotify] = useState(false);
  const [theme, setTheme] = useState<ThemeMode>(settings.theme);
  const [language, setLanguage] = useState<Language>(settings.language);
  const [notificationEnabled, setNotificationEnabled] = useState(
    settings.notificationEnabled
  );
  const [reviewReminderEnabled, setReviewReminderEnabled] = useState(
    settings.reviewReminderEnabled
  );
  const [reviewReminderTime, setReviewReminderTime] = useState(
    settings.reviewReminderTime
  );
  const [exportDirectory, setExportDirectory] = useState(
    settings.storage.exportDirectory ?? ""
  );
  const [screenshotShortcut, setScreenshotShortcut] = useState(
    settings.screenshotShortcut
  );

  useEffect(() => {
    setTheme(settings.theme);
    setLanguage(settings.language);
    setNotificationEnabled(settings.notificationEnabled);
    setReviewReminderEnabled(settings.reviewReminderEnabled);
    setReviewReminderTime(settings.reviewReminderTime);
    setExportDirectory(settings.storage.exportDirectory ?? "");
    setScreenshotShortcut(settings.screenshotShortcut);
  }, [settings]);

  async function handleTestNotification() {
    setTestingNotify(true);
    try {
      const r = await sendTestSystemNotification();
      if (r.ok) {
        toast.success(
          "已发送测试通知",
          "若未看到系统气泡，请检查系统与桌面环境的通知权限。"
        );
        return;
      }
      if (r.reason === "disabled") {
        toast.error("无法测试", "请先开启上方的「桌面通知」开关。");
        return;
      }
      if (r.reason === "denied") {
        toast.error("无法测试", "通知权限被拒绝，请在系统设置中允许 NeverMind 发送通知。");
        return;
      }
      if (r.reason === "unsupported") {
        toast.error("无法测试", "当前环境不支持系统通知。");
        return;
      }
      toast.error("无法测试", "发送失败，请稍后再试。");
    } finally {
      setTestingNotify(false);
    }
  }

  function handleSave() {
    void onSave({
      theme,
      language,
      notificationEnabled,
      reviewReminderEnabled,
      reviewReminderTime,
      storage: {
        exportDirectory: exportDirectory.trim() || null,
      },
      screenshotShortcut: screenshotShortcut.trim() || "ctrl+shift+a",
    });
  }

  return (
    <Panel title="通用设置" description="外观、语言、提醒与数据导出位置。">
      <div className="space-y-5">
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>外观</FieldLabel>
            <Select
              value={theme}
              onChange={(e) => setTheme(e.target.value as ThemeMode)}
              options={[
                { value: "system", label: "跟随系统" },
                { value: "light", label: "浅色" },
                { value: "dark", label: "深色" },
              ]}
            />
          </div>
          <div>
            <FieldLabel>语言</FieldLabel>
            <Select
              value={language}
              onChange={(e) => setLanguage(e.target.value as Language)}
              options={[
                { value: "zh-CN", label: "简体中文" },
                { value: "en-US", label: "English" },
              ]}
            />
          </div>
        </div>

        <ToggleRow
          label="桌面通知"
          description="允许完成复习、生成卡片后弹出系统通知。"
          checked={notificationEnabled}
          onChange={setNotificationEnabled}
        />

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="secondary"
            size="sm"
            leftIcon={<Bell className="h-3.5 w-3.5" />}
            loading={testingNotify}
            disabled={!notificationEnabled || testingNotify}
            onClick={() => void handleTestNotification()}
          >
            发送测试通知
          </Button>
          <span className="text-xs text-ink-500">
            需先开启「桌面通知」。首次使用可能弹出系统权限请求。
          </span>
        </div>

        <ToggleRow
          label="每日复习提醒"
          description="在指定时间提醒你进入复习。"
          checked={reviewReminderEnabled}
          onChange={setReviewReminderEnabled}
        />

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>复习提醒时间</FieldLabel>
            <Input
              type="time"
              value={reviewReminderTime}
              disabled={!reviewReminderEnabled}
              onChange={(e) => setReviewReminderTime(e.target.value)}
            />
          </div>
          <div>
            <FieldLabel>导出目录</FieldLabel>
            <Input
              type="text"
              placeholder="留空则使用应用默认目录"
              value={exportDirectory}
              onChange={(e) => setExportDirectory(e.target.value)}
            />
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <div>
            <FieldLabel>截图全局快捷键</FieldLabel>
            <Input
              type="text"
              placeholder="例如: ctrl+shift+a"
              value={screenshotShortcut}
              onChange={(e) => setScreenshotShortcut(e.target.value)}
            />
            <p className="mt-1 text-xs text-ink-500">
              格式如：ctrl+shift+a 或 CommandOrControl+Shift+A
            </p>
          </div>
        </div>

        <div className="flex justify-end border-t border-ink-100 pt-4">
          <Button
            leftIcon={<Save className="h-4 w-4" />}
            loading={saving}
            onClick={handleSave}
          >
            保存设置
          </Button>
        </div>
      </div>
    </Panel>
  );
}

function ToggleRow({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-lg border border-ink-100 bg-ink-50/40 px-4 py-3">
      <div>
        <div className="text-sm font-medium text-ink-800">{label}</div>
        <div className="mt-0.5 text-xs text-ink-500">{description}</div>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={
          "relative inline-flex h-6 w-11 flex-none items-center rounded-full transition " +
          (checked ? "bg-brand-600" : "bg-ink-300")
        }
      >
        <span
          className={
            "inline-block h-5 w-5 transform rounded-full bg-white shadow transition " +
            (checked ? "translate-x-5" : "translate-x-1")
          }
        />
      </button>
    </div>
  );
}
