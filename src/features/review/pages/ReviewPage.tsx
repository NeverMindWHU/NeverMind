import { Brain, LibraryBig, PartyPopper, RefreshCw, Sparkles } from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "@/components/Button";
import { EmptyState } from "@/components/EmptyState";
import { Panel } from "@/components/Card";
import { Spinner } from "@/components/Spinner";
import { listRecentBatches } from "@/lib/recent-batches";
import { DashboardSummary } from "../components/DashboardSummary";
import { ReviewCard } from "../components/ReviewCard";
import { useReview } from "../hooks/useReview";

export function ReviewPage() {
  const r = useReview();
  const current = r.queue[0];

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink-900">复习</h1>
          <p className="mt-1 text-sm text-ink-500">
            基于艾宾浩斯遗忘曲线，按到期时间弹出卡片，和 Anki 一样「记住 / 忘记 / 跳过 / 完成」。
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

      <Panel
        title={current ? "到期卡片" : "本轮复习状态"}
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
            description={`本次复习了 ${r.completedThisSession} 张卡片。等待下一轮到期提醒吧。`}
            action={
              <Button variant="secondary" onClick={r.reload}>
                再看一下
              </Button>
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
