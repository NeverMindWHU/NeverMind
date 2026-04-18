import { Check, CheckCheck, RefreshCw, X } from "lucide-react";
import { Button } from "@/components/Button";
import { Panel, Tag } from "@/components/Card";
import type { GeneratedCardBatchResult } from "@/types/card";
import { GeneratedCardItem } from "./GeneratedCardItem";

interface Props {
  batch: GeneratedCardBatchResult;
  accepted: Set<string>;
  rejected: Set<string>;
  submitting: boolean;
  onToggleAccept: (id: string) => void;
  onToggleReject: (id: string) => void;
  onAcceptAll: () => void;
  onRejectAll: () => void;
  onSubmit: () => void;
  onDiscard: () => void;
}

/**
 * 批次预览区：列出本次生成的全部卡片，提供批量/单张的 accept/reject，
 * 最终一键调用 review_generated_cards 入库。
 */
export function BatchPreview({
  batch,
  accepted,
  rejected,
  submitting,
  onToggleAccept,
  onToggleReject,
  onAcceptAll,
  onRejectAll,
  onSubmit,
  onDiscard,
}: Props) {
  const total = batch.cards.length;
  const acceptCount = accepted.size;
  const rejectCount = rejected.size;
  const pendingCount = total - acceptCount - rejectCount;

  return (
    <Panel
      title={
        <div className="flex items-center gap-2">
          <span>本次生成</span>
          <Tag tone="brand">{total} 张</Tag>
        </div>
      }
      description={`批次 ID：${batch.batchId}`}
      actions={
        <>
          <Button
            size="sm"
            variant="ghost"
            leftIcon={<X className="h-3.5 w-3.5" />}
            onClick={onRejectAll}
            disabled={submitting}
          >
            全部拒绝
          </Button>
          <Button
            size="sm"
            variant="secondary"
            leftIcon={<CheckCheck className="h-3.5 w-3.5" />}
            onClick={onAcceptAll}
            disabled={submitting}
          >
            全部接受
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {batch.cards.map((card, i) => {
          const decision = accepted.has(card.cardId)
            ? "accept"
            : rejected.has(card.cardId)
              ? "reject"
              : "pending";
          return (
            <GeneratedCardItem
              key={card.cardId}
              index={i}
              card={card}
              decision={decision}
              onAccept={() => onToggleAccept(card.cardId)}
              onReject={() => onToggleReject(card.cardId)}
            />
          );
        })}
      </div>

      <div className="mt-6 flex items-center justify-between border-t border-ink-100 pt-5">
        <div className="flex items-center gap-2 text-xs text-ink-600">
          <Tag tone="success">接受 {acceptCount}</Tag>
          <Tag tone="warn">未决 {pendingCount}</Tag>
          <Tag>拒绝 {rejectCount}</Tag>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            leftIcon={<RefreshCw className="h-4 w-4" />}
            onClick={onDiscard}
            disabled={submitting}
          >
            丢弃并重来
          </Button>
          <Button
            variant="primary"
            leftIcon={<Check className="h-4 w-4" />}
            onClick={onSubmit}
            loading={submitting}
          >
            保存选择入库
          </Button>
        </div>
      </div>
    </Panel>
  );
}
