import { Spinner } from "@/components/Spinner";
import { GeneralSettingsForm } from "../components/GeneralSettingsForm";
import { ModelProfilesPanel } from "../components/ModelProfilesPanel";
import { useSettings } from "../hooks/useSettings";

export function SettingsPage() {
  const s = useSettings();

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">设置</h1>
        <p className="mt-1 text-sm text-ink-500">
          NeverMind 所有数据与配置都保存在本地 SQLite，API Key 不会上传到任何第三方。
        </p>
      </header>

      {s.loading || !s.settings ? (
        <div className="flex justify-center py-16">
          <Spinner label="加载设置…" />
        </div>
      ) : (
        <>
          <GeneralSettingsForm
            settings={s.settings}
            saving={s.saving}
            onSave={s.saveSettings}
          />
          <ModelProfilesPanel
            profiles={s.profiles}
            onSave={s.saveProfile}
            onTest={s.testProfile}
          />
        </>
      )}
    </div>
  );
}
