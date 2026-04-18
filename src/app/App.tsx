import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { ToastProvider } from "@/lib/toast";
import { AppShell } from "@/layouts/AppShell";
import { HomePage } from "@/pages/HomePage";
import { GeneratePage } from "@/features/card-generation/pages/GeneratePage";
import { ReviewPage } from "@/features/review/pages/ReviewPage";
import { LibraryPage } from "@/features/library/pages/LibraryPage";
import { SettingsPage } from "@/features/settings/pages/SettingsPage";
import { GenerationTasksProvider } from "@/features/generation-tasks/GenerationTasksContext";

/**
 * 用 HashRouter：Tauri 本地产物是 file:// 协议下的 index.html，
 * HashRouter 对此兼容最好（不依赖历史 API 的 base）。
 *
 * Provider 嵌套顺序：
 *   ToastProvider           —— 供全局 toast 调用
 *   └─ HashRouter           —— 让内部组件能用 <Link>
 *      └─ GenerationTasksProvider  —— 依赖 toast + Link（任务栏有跳转）
 */
export default function App() {
  return (
    <ToastProvider>
      <HashRouter>
        <GenerationTasksProvider>
          <Routes>
            <Route element={<AppShell />}>
              <Route index element={<HomePage />} />
              <Route path="generate" element={<GeneratePage />} />
              <Route path="review" element={<ReviewPage />} />
              <Route path="library" element={<LibraryPage />} />
              <Route path="settings" element={<SettingsPage />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Route>
          </Routes>
        </GenerationTasksProvider>
      </HashRouter>
    </ToastProvider>
  );
}
