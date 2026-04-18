import { useEffect } from "react";
import { Link } from "react-router-dom";
import { Info, LibraryBig, Sparkles } from "lucide-react";
import { Button } from "@/components/Button";
import { EmptyState } from "@/components/EmptyState";
import { GenerateInputPanel } from "../components/GenerateInputPanel";
import { BatchPreview } from "../components/BatchPreview";
import { useCardGeneration } from "../hooks/useCardGeneration";
import { useGenerationTasks } from "@/features/generation-tasks/GenerationTasksContext";

/**
 * 生成页（异步化后）：
 *
 * - 点击「生成卡片」会把任务丢给全局 `GenerationTasksProvider` 后台处理，
 *   用户无需在本页傻等。表单立即可用，用户也可以切到其他页去。
 * - 任务完成后：
 *     1. 右上角 toast 通知 + 写入「最近批次」
 *     2. 如果此刻 GeneratePage 处于"空预览区"状态，自动把 batch 填入预览，
 *        方便用户立即筛选/入库；否则什么也不做（用户处理完当前批次后，
 *        下一批会自动补位）。
 */
export function GeneratePage() {
  const g = useCardGeneration();
  const { tasks, runningCount, startGeneration, markConsumed } =
    useGenerationTasks();

  // 从 tasks 里挑一个最早完成且未被消费的 success 任务，自动填充到预览区。
  // 只有当本页当前预览区为空时才填充，避免打断用户正在进行的接受/拒绝决策。
  useEffect(() => {
    if (g.batch) return;
    const next = tasks.find(
      (t) => t.status === "success" && !t.consumed && t.batch
    );
    if (next && next.batch) {
      g.hydrateBatch(next.batch);
      markConsumed(next.id);
    }
  }, [tasks, g, markConsumed]);

  // 预览区已有未处理批次，而后面还有完成任务在排队 → 提示用户。
  // 这些"在排队"的任务会在用户保存/丢弃当前批次后被上面的 effect 依次消费。
  const waitingCount = tasks.filter(
    (t) => t.status === "success" && !t.consumed && t.batch
  ).length;
  const hasExtraWaitingBatch = g.batch !== null && waitingCount > 0;

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">生成卡片</h1>
      </header>

      <GenerateInputPanel
        submitting={false}
        onSubmit={(input) => {
          // fire-and-forget：不再 await；Provider 会 toast + 写入最近批次。
          startGeneration(input);
        }}
      />

      {runningCount > 0 && (
        <div className="flex items-center gap-3 rounded-xl border border-brand-200 bg-brand-50/60 px-4 py-3 text-sm text-brand-900">
          <Sparkles className="h-4 w-4 text-brand-600" />
          <div className="flex-1">
            后台正在处理 <strong>{runningCount}</strong> 个生成任务
            {g.batch ? "；当前预览区是之前完成的批次，你可以先筛选入库，新批次到达时会自动接上。" : "，完成后会自动出现在下方预览区。"}
          </div>
        </div>
      )}

      {hasExtraWaitingBatch && (
        <div className="flex items-start gap-3 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
          <Info className="mt-0.5 h-4 w-4 flex-none" />
          <div className="flex-1">
            还有 <strong>{waitingCount}</strong> 个新批次在排队。保存或丢弃当前批次后，下一批会自动切换进来。
            你也可以
            <Link to="/library" className="mx-1 font-medium text-amber-700 underline-offset-2 hover:underline">
              去知识宝库
            </Link>
            按关键词直接查看。
          </div>
        </div>
      )}

      {g.batch ? (
        <BatchPreview
          batch={g.batch}
          accepted={g.accepted}
          rejected={g.rejected}
          submitting={g.submitting}
          onToggleAccept={g.toggleAccept}
          onToggleReject={g.toggleReject}
          onAcceptAll={g.acceptAll}
          onRejectAll={g.rejectAll}
          onSubmit={g.submit}
          onDiscard={g.reset}
        />
      ) : runningCount > 0 ? (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="生成中，预览区待定"
          description="Ark 正在处理你的输入，可以继续提交新的任务，或切到其他页等通知。"
        />
      ) : (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="还没有生成结果"
          description="在上方填入内容并点击「生成卡片」，Ark 会在后台产出一组结构化的知识卡片供你筛选。"
          action={
            <Link to="/library">
              <Button variant="secondary" leftIcon={<LibraryBig className="h-4 w-4" />}>
                去知识宝库
              </Button>
            </Link>
          }
        />
      )}
    </div>
  );
}
