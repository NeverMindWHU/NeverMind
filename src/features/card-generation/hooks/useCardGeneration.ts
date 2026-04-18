import { useCallback, useState } from "react";
import { useToast } from "@/lib/toast";
import { humanizeError } from "@/lib/format";
import { rememberBatch } from "@/lib/recent-batches";
import type { GenerateCardsInput, GeneratedCardBatchResult } from "@/types/card";
import type { CommandError } from "@/types/common";
import {
  generateCards as apiGenerate,
  reviewGeneratedCards as apiReview,
} from "../services/api";

export interface CardGenerationState {
  batch: GeneratedCardBatchResult | null;
  loading: boolean;
  submitting: boolean;
  error: CommandError | null;
  accepted: Set<string>;
  rejected: Set<string>;
}

/**
 * 统一管理一次"生成 → 预览 → 接受/拒绝"的状态机。
 *
 * 设计取舍：
 * - 本批次结束前都驻留在 state.batch，用户切走再回来不丢失（页面卸载才清空）。
 * - accepted / rejected 用 `Set<cardId>`，提交前两边互斥，未归类的卡片视为"未决"。
 */
export function useCardGeneration() {
  const toast = useToast();
  const [state, setState] = useState<CardGenerationState>({
    batch: null,
    loading: false,
    submitting: false,
    error: null,
    accepted: new Set(),
    rejected: new Set(),
  });

  const generate = useCallback(
    async (input: GenerateCardsInput) => {
      setState((s) => ({ ...s, loading: true, error: null }));
      try {
        const batch = await apiGenerate(input);
        setState({
          batch,
          loading: false,
          submitting: false,
          error: null,
          accepted: new Set(batch.cards.map((c) => c.cardId)),
          rejected: new Set(),
        });
        // 生成当下就把 batch 登记到"最近批次"，即使用户没点保存，
        // 也能在宝库页面按 batchId 找回（后端已完整落库 pending 卡片）。
        rememberBatch({
          batchId: batch.batchId,
          title: batch.cards[0]?.keyword,
          cardCount: batch.cards.length,
          sourceType: input.sourceType,
        });
        toast.success("生成成功", `共产出 ${batch.cards.length} 张卡片`);
        return batch;
      } catch (err) {
        const ce = err as CommandError;
        setState((s) => ({ ...s, loading: false, error: ce }));
        toast.error("生成失败", humanizeError(ce));
        throw err;
      }
    },
    [toast]
  );

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
      // 刷新"最近批次"里的卡片数，保证宝库统计同步。
      if (state.batch) {
        rememberBatch({
          batchId: state.batch.batchId,
          title: state.batch.cards[0]?.keyword,
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
        loading: false,
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
      loading: false,
      submitting: false,
      error: null,
      accepted: new Set(),
      rejected: new Set(),
    });
  }, []);

  return {
    ...state,
    generate,
    toggleAccept,
    toggleReject,
    acceptAll,
    rejectAll,
    submit,
    reset,
  };
}
