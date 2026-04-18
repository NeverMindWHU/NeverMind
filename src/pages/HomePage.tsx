import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Brain, LibraryBig, Settings as SettingsIcon, Sparkles } from "lucide-react";
import { Button } from "@/components/Button";
import { Panel, StatTile } from "@/components/Card";
import { EmptyState } from "@/components/EmptyState";
import { Spinner } from "@/components/Spinner";
import { isTauri } from "@/lib/tauri";
import { humanizeError, formatRelative } from "@/lib/format";
import { getReviewDashboard } from "@/features/review/services/api";
import type { ReviewDashboardData } from "@/types/review";
import type { CommandError } from "@/types/common";

export function HomePage() {
  const [dashboard, setDashboard] = useState<ReviewDashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const inTauri = isTauri();

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!inTauri) {
        setLoading(false);
        return;
      }
      try {
        const data = await getReviewDashboard();
        if (!cancelled) setDashboard(data);
      } catch (err) {
        if (!cancelled) setError(humanizeError(err as CommandError));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [inTauri]);

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">欢迎回到 NeverMind</h1>
        <p className="mt-1 text-sm text-ink-500">
          把看到的知识变成卡片
        </p>
      </header>

      {!inTauri && (
        <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-800">
          当前运行在普通浏览器中，Tauri 原生命令不可用。功能页面仅展示 UI 骨架；
          请用 <code className="rounded bg-white px-1 py-0.5">npm run tauri:dev</code>{" "}
          启动桌面应用以获得完整能力。
        </div>
      )}

      {loading ? (
        <div className="flex justify-center py-8">
          <Spinner label="加载概览…" />
        </div>
      ) : error ? (
        <EmptyState title="概览加载失败" description={error} />
      ) : (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
          <StatTile
            icon={<Brain className="h-4 w-4" />}
            label="今日待复习"
            value={dashboard?.dueToday ?? "—"}
            hint="包含今天到期但未完成的卡片"
          />
          <StatTile
            icon={<Sparkles className="h-4 w-4" />}
            label="今日已完成"
            value={dashboard?.completedToday ?? "—"}
            hint="今天已提交结果的复习次数"
          />
          <StatTile
            icon={<LibraryBig className="h-4 w-4" />}
            label="连击天数"
            value={dashboard?.streakDays ?? "—"}
            hint="连续有复习记录的自然日"
          />
          <StatTile
            icon={<SettingsIcon className="h-4 w-4" />}
            label="下一张到期"
            value={formatRelative(dashboard?.nextDueAt)}
            hint={dashboard?.nextDueAt ?? "全部卡片都暂无安排"}
          />
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <Panel
          title="开始生成新卡片"
          description="粘贴一段文本或上传图片，豆包会提炼出结构化卡片供你挑选。"
        >
          <Link to="/generate">
            <Button leftIcon={<Sparkles className="h-4 w-4" />} size="lg">
              进入生成页面
            </Button>
          </Link>
        </Panel>
        <Panel
          title="开始今日复习"
          description="按照艾宾浩斯曲线，NeverMind 会把该复习的卡片排到最前面。"
        >
          <Link to="/review">
            <Button variant="secondary" leftIcon={<Brain className="h-4 w-4" />} size="lg">
              进入复习页面
            </Button>
          </Link>
        </Panel>
      </div>
    </div>
  );
}
