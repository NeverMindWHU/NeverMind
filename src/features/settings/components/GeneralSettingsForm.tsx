import { useEffect, useState } from "react";
import { Save } from "lucide-react";
import { Button } from "@/components/Button";
import { FieldLabel, Input, Panel, Select } from "@/components/Card";
import type {
  AppSettingsData,
  Language,
  ThemeMode,
  UpdateSettingsInput,
} from "@/types/settings";

interface Props {
  settings: AppSettingsData;
  saving: boolean;
  onSave: (next: UpdateSettingsInput) => void | Promise<unknown>;
}

export function GeneralSettingsForm({ settings, saving, onSave }: Props) {
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

  useEffect(() => {
    setTheme(settings.theme);
    setLanguage(settings.language);
    setNotificationEnabled(settings.notificationEnabled);
    setReviewReminderEnabled(settings.reviewReminderEnabled);
    setReviewReminderTime(settings.reviewReminderTime);
    setExportDirectory(settings.storage.exportDirectory ?? "");
  }, [settings]);

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
