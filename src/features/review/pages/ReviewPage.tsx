import {
  Brain,
  Clock,
  FastForward,
  LibraryBig,
  PartyPopper,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "@/components/Button";
import { EmptyState } from "@/components/EmptyState";
import { Panel } from "@/components/Card";
import { Spinner } from "@/components/Spinner";
import { listRecentBatches } from "@/lib/recent-batches";
import { DashboardSummary } from "../components/DashboardSummary";
import { ReviewCard } from "../components/ReviewCard";
import { useReview } from "../hooks/useReview";

function formatRelativeDue(dueAt: string | null): string | null {
  if (!dueAt) return null;
  const diffMs = new Date(dueAt).getTime() - Date.now();
  if (!Number.isFinite(diffMs)) return null;
  if (diffMs <= 0) return "已到期";
  const minutes = Math.round(diffMs / 60000);
  if (minutes < 60) return `${minutes} 分钟后到期`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} 小时后到期`;
  const days = Math.round(hours / 24);
  return `${days} 天后到期`;
}

export function ReviewPage() {
  const r = useReview();
  const current = r.queue[0];
  const inUpcomingMode = r.mode === "upcoming";
  const upcomingCount = r.upcoming?.upcomingCount ?? 0;
  const earliestUpcomingHint = formatRelativeDue(r.upcoming?.earliestDueAt ?? null);

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink-900">复习</h1>
          <p className="mt-1 text-sm text-ink-500">
            基于艾宾浩斯遗忘曲线，按到期时间弹出卡片, 选择「记住 / 忘记 / 跳过 / 完成」。
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          leftIcon={<RefreshCw className="h-4 w-4" />}
          loading={r.loading || r.refreshing}
          onClick={r.reload}
        >
          刷新
        </Button>
      </header>

      <DashboardSummary
        dashboard={r.dashboard}
        completedThisSession={r.completedThisSession}
      />

      {inUpcomingMode && current && (
        <div className="flex items-start gap-3 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800">
          <FastForward className="mt-0.5 h-4 w-4 shrink-0" />
          <div>
            <div className="font-medium">正在提前复习下一轮</div>
            <div className="mt-0.5 text-amber-700/90">
              这些卡片本来还没到期，提交后会按正常规则推进到下一次复习时间。
              本轮提前队列剩 {r.queue.length} 张。
            </div>
          </div>
        </div>
      )}

      <Panel
        title={current ? (inUpcomingMode ? "提前复习中" : "到期卡片") : "本轮复习状态"}
        description={
          current
            ? `本批还剩 ${r.queue.length} 张 · 本次已完成 ${r.completedThisSession} 张`
            : undefined
        }
      >
        {r.loading ? (
          <div className="flex justify-center py-16">
            <Spinner label="加载到期卡片…" />
          </div>
        ) : current ? (
          <ReviewCard
            key={current.reviewId}
            card={current}
            submitting={r.submitting}
            onSubmit={(res) => void r.submit(current, res)}
          />
        ) : r.completedThisSession > 0 ? (
          <EmptyState
            icon={<PartyPopper className="h-8 w-8 text-emerald-500" />}
            title="全部完成了！"
            description={
              upcomingCount > 0
                ? `本次复习了 ${r.completedThisSession} 张。还可以提前复习下一轮 ${upcomingCount} 张（${earliestUpcomingHint ?? "即将到期"}）。`
                : `本次复习了 ${r.completedThisSession} 张卡片。等待下一轮到期提醒吧。`
            }
            action={
              <div className="flex flex-wrap justify-center gap-2">
                {upcomingCount > 0 && (
                  <Button
                    variant="primary"
                    leftIcon={<FastForward className="h-4 w-4" />}
                    loading={r.refreshing}
                    onClick={() => void r.startUpcoming()}
                  >
                    提前复习下一轮（{upcomingCount}）
                  </Button>
                )}
                <Button variant="secondary" onClick={r.reload}>
                  再看一下
                </Button>
              </div>
            }
          />
        ) : upcomingCount > 0 ? (
          <EmptyState
            icon={<Clock className="h-8 w-8 text-amber-500" />}
            title="今日到期队列已空"
            description={`后续还有 ${upcomingCount} 张已排好档期的卡片（${earliestUpcomingHint ?? "即将到期"}）。想学就学，提前开始吧。`}
            action={
              <div className="flex flex-wrap justify-center gap-2">
                <Button
                  variant="primary"
                  leftIcon={<FastForward className="h-4 w-4" />}
                  loading={r.refreshing}
                  onClick={() => void r.startUpcoming()}
                >
                  提前复习下一轮（{upcomingCount}）
                </Button>
                <Button variant="secondary" onClick={r.reload}>
                  再看一下
                </Button>
              </div>
            }
          />
        ) : listRecentBatches().length > 0 ? (
          <EmptyState
            icon={<LibraryBig className="h-8 w-8" />}
            title="复习队列为空"
            description={
              <>
                如果最近刚生成了卡片却没有出现在这里，很可能是批次里的卡片还是
                「待定」状态——需要在「知识宝库」里把它们接受入库后才会进入复习队列。
              </>
            }
            action={
              <Link to="/library">
                <Button variant="primary" leftIcon={<LibraryBig className="h-4 w-4" />}>
                  去宝库查看批次
                </Button>
              </Link>
            }
          />
        ) : (
          <EmptyState
            icon={<Brain className="h-8 w-8" />}
            title="暂时没有到期的卡片"
            description="先去「生成卡片」攒几张，或稍后回来看看。"
            action={
              <Link to="/generate">
                <Button variant="secondary" leftIcon={<Sparkles className="h-4 w-4" />}>
                  去生成卡片
                </Button>
              </Link>
            }
          />
        )}
      </Panel>
    </div>
  );
}
