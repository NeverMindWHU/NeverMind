import { NavLink, Outlet } from "react-router-dom";
import clsx from "clsx";
import {
  BookOpen,
  Brain,
  Settings as SettingsIcon,
  Sparkles,
  LibraryBig,
} from "lucide-react";
import type { ComponentType, SVGProps } from "react";

interface NavItem {
  to: string;
  label: string;
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  description: string;
}

const NAV: NavItem[] = [
  { to: "/", label: "首页", icon: BookOpen, description: "今日任务概览" },
  { to: "/generate", label: "生成卡片", icon: Sparkles, description: "从文本/图片提炼卡片" },
  { to: "/review", label: "复习", icon: Brain, description: "艾宾浩斯翻卡" },
  { to: "/library", label: "知识宝库", icon: LibraryBig, description: "批次与卡片检索" },
  { to: "/settings", label: "设置", icon: SettingsIcon, description: "模型与通用偏好" },
];

export function AppShell() {
  return (
    <div className="flex h-full min-h-screen">
      <aside className="flex w-60 flex-none flex-col border-r border-ink-200 bg-white">
        <div className="drag-region flex items-center gap-2 px-5 pb-3 pt-5">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-brand-500 to-brand-700 text-white shadow-card">
            <Brain className="h-5 w-5" />
          </div>
          <div className="leading-tight">
            <div className="text-sm font-semibold text-ink-900">NeverMind</div>
            <div className="text-[11px] text-ink-500">AI 卡片 · 艾宾浩斯复习</div>
          </div>
        </div>

        <nav className="mt-2 flex-1 space-y-0.5 px-3">
          {NAV.map((n) => (
            <NavLink
              key={n.to}
              to={n.to}
              end={n.to === "/"}
              className={({ isActive }) =>
                clsx(
                  "group flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition",
                  isActive
                    ? "bg-brand-50 text-brand-700"
                    : "text-ink-700 hover:bg-ink-100 hover:text-ink-900"
                )
              }
            >
              {({ isActive }) => (
                <>
                  <n.icon
                    className={clsx(
                      "h-4 w-4 flex-none",
                      isActive ? "text-brand-600" : "text-ink-500 group-hover:text-ink-700"
                    )}
                  />
                  <div className="min-w-0">
                    <div className="truncate font-medium">{n.label}</div>
                    <div
                      className={clsx(
                        "truncate text-[11px]",
                        isActive ? "text-brand-600/80" : "text-ink-500"
                      )}
                    >
                      {n.description}
                    </div>
                  </div>
                </>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="border-t border-ink-100 px-5 py-3 text-[11px] leading-5 text-ink-500">
          本地优先 · 数据存 SQLite
        </div>
      </aside>

      <main className="flex-1 overflow-auto bg-ink-50">
        <div className="mx-auto w-full max-w-5xl px-6 py-8">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
