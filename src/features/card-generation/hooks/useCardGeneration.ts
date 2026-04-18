import { useCallback, useState } from "react";
import { useToast } from "@/lib/toast";
import { humanizeError } from "@/lib/format";
import { rememberBatch } from "@/lib/recent-batches";
import type { GeneratedCardBatchResult } from "@/types/card";
import type { CommandError } from "@/types/common";
import { reviewGeneratedCards as apiReview } from "../services/api";

export interface CardGenerationState {
  batch: GeneratedCardBatchResult | null;
  submitting: boolean;
  error: CommandError | null;
  accepted: Set<string>;
  rejected: Set<string>;
}

/**
 * 管理一次"预览 → 接受/拒绝 → 保存"的状态机。
 *
 * 设计变更（v2 异步化后）：
 * - **不再**持有 "生成中" 的 loading 状态，生成流程交给全局
 *   `GenerationTasksProvider` 后台处理。
 * - 本 Hook 只关心"已经拿到 batch 后怎么展示 + 入库"。
 * - 调用方通过 `hydrateBatch(batch)` 把任务完成后的结果塞进来，
 *   本 Hook 会自动把所有卡片预选为 accepted，用户可再微调。
 */
export function useCardGeneration() {
  const toast = useToast();
  const [state, setState] = useState<CardGenerationState>({
    batch: null,
    submitting: false,
    error: null,
    accepted: new Set(),
    rejected: new Set(),
  });

  /**
   * 把后台生成好的批次填充到预览区。
   * 默认全部预选 accepted，符合用户"先看后筛"心智。
   * 重复 hydrate 同一个 batchId 会被忽略，避免用户已调整过的 accepted/rejected 被覆盖。
   */
  const hydrateBatch = useCallback((batch: GeneratedCardBatchResult) => {
    setState((prev) => {
      if (prev.batch?.batchId === batch.batchId) return prev;
      return {
        batch,
        submitting: false,
        error: null,
        accepted: new Set(batch.cards.map((c) => c.cardId)),
        rejected: new Set(),
      };
    });
  }, []);

  const toggleAccept = useCallback((cardId: string) => {
    setState((s) => {
      const accepted = new Set(s.accepted);
      const rejected = new Set(s.rejected);
      if (accepted.has(cardId)) {
        accepted.delete(cardId);
      } else {
        accepted.add(cardId);
        rejected.delete(cardId);
      }
      return { ...s, accepted, rejected };
    });
  }, []);

  const toggleReject = useCallback((cardId: string) => {
    setState((s) => {
      const accepted = new Set(s.accepted);
      const rejected = new Set(s.rejected);
      if (rejected.has(cardId)) {
        rejected.delete(cardId);
      } else {
        rejected.add(cardId);
        accepted.delete(cardId);
      }
      return { ...s, accepted, rejected };
    });
  }, []);

  const acceptAll = useCallback(() => {
    setState((s) => ({
      ...s,
      accepted: new Set(s.batch?.cards.map((c) => c.cardId) ?? []),
      rejected: new Set(),
    }));
  }, []);

  const rejectAll = useCallback(() => {
    setState((s) => ({
      ...s,
      accepted: new Set(),
      rejected: new Set(s.batch?.cards.map((c) => c.cardId) ?? []),
    }));
  }, []);

  const submit = useCallback(async () => {
    if (!state.batch) return;
    setState((s) => ({ ...s, submitting: true }));
    try {
      const result = await apiReview({
        batchId: state.batch.batchId,
        acceptCardIds: Array.from(state.accepted),
        rejectCardIds: Array.from(state.rejected),
      });
      if (state.batch) {
        rememberBatch({
          batchId: state.batch.batchId,
          title:
            state.batch.cards[0]?.question ?? state.batch.cards[0]?.keyword,
          cardCount: result.acceptedCount + result.pendingCount,
        });
      }
      toast.success(
        "批次已保存",
        `接受 ${result.acceptedCount} · 拒绝 ${result.rejectedCount}${
          result.pendingCount > 0 ? ` · 未决 ${result.pendingCount}` : ""
        }，可在「知识宝库」中查看`
      );
      setState({
        batch: null,
        submitting: false,
        error: null,
        accepted: new Set(),
        rejected: new Set(),
      });
      return result;
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, submitting: false }));
      toast.error("保存失败", humanizeError(ce));
      throw err;
    }
  }, [state.accepted, state.batch, state.rejected, toast]);

  const reset = useCallback(() => {
    setState({
      batch: null,
      submitting: false,
      error: null,
      accepted: new Set(),
      rejected: new Set(),
    });
  }, []);

  return {
    ...state,
    hydrateBatch,
    toggleAccept,
    toggleReject,
    acceptAll,
    rejectAll,
    submit,
    reset,
  };
}
