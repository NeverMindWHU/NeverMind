import { Sparkles } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { GenerateInputPanel } from "../components/GenerateInputPanel";
import { BatchPreview } from "../components/BatchPreview";
import { useCardGeneration } from "../hooks/useCardGeneration";

export function GeneratePage() {
  const g = useCardGeneration();

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold text-ink-900">生成卡片</h1>
        <p className="mt-1 text-sm text-ink-500">
          文本 / 图片 / 图文混合 → 豆包多模态提炼 → Anki 风格预览 → 你来决定入库哪些。
        </p>
      </header>

      <GenerateInputPanel submitting={g.loading} onSubmit={g.generate} />

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
      ) : (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="还没有生成结果"
          description="在上方填入内容并点击「生成卡片」，Ark 会产出一组结构化的知识卡片供你筛选。"
        />
      )}
    </div>
  );
}
