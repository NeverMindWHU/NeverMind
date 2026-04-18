import { useCallback, useEffect, useState } from "react";
import { useToast } from "@/lib/toast";
import { humanizeError } from "@/lib/format";
import type { CommandError } from "@/types/common";
import type {
  DueReviewCard,
  ReviewDashboardData,
  ReviewResult,
} from "@/types/review";
import { getReviewDashboard, listDueReviews, submitReviewResult } from "../services/api";

interface State {
  loading: boolean;
  refreshing: boolean;
  submitting: boolean;
  error: CommandError | null;
  queue: DueReviewCard[];
  dashboard: ReviewDashboardData | null;
  completedThisSession: number;
}

/**
 * 复习循环：一次拉取一批到期卡片，按先入先出消耗。队列清空后尝试再拉一次；
 * 若仍为空，则结束本次复习。
 */
export function useReview() {
  const toast = useToast();
  const [state, setState] = useState<State>({
    loading: true,
    refreshing: false,
    submitting: false,
    error: null,
    queue: [],
    dashboard: null,
    completedThisSession: 0,
  });

  const loadInitial = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const [list, dashboard] = await Promise.all([
        listDueReviews({ limit: 20, includeCompletedToday: true }),
        getReviewDashboard(),
      ]);
      setState((s) => ({
        ...s,
        loading: false,
        queue: list.items,
        dashboard,
      }));
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, loading: false, error: ce }));
      toast.error("加载失败", humanizeError(ce));
    }
  }, [toast]);

  useEffect(() => {
    void loadInitial();
  }, [loadInitial]);

  const refillIfNeeded = useCallback(async () => {
    setState((s) => ({ ...s, refreshing: true }));
    try {
      const list = await listDueReviews({ limit: 20 });
      setState((s) => ({ ...s, refreshing: false, queue: list.items }));
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, refreshing: false }));
      toast.error("刷新失败", humanizeError(ce));
    }
  }, [toast]);

  const submit = useCallback(
    async (card: DueReviewCard, result: ReviewResult) => {
      setState((s) => ({ ...s, submitting: true }));
      try {
        const data = await submitReviewResult({
          reviewId: card.reviewId,
          cardId: card.cardId,
          result,
          reviewedAt: new Date().toISOString(),
        });

        setState((s) => {
          const remaining = s.queue.filter((x) => x.reviewId !== card.reviewId);
          return {
            ...s,
            submitting: false,
            queue: remaining,
            completedThisSession: s.completedThisSession + 1,
            dashboard: s.dashboard
              ? {
                  ...s.dashboard,
                  completedToday: s.dashboard.completedToday + 1,
                  dueToday: Math.max(0, s.dashboard.dueToday - 1),
                  nextDueAt: data.nextReviewAt ?? s.dashboard.nextDueAt,
                }
              : s.dashboard,
          };
        });

        if (result === "forgotten") {
          toast.info("已重置", "这张卡明天会再次出现");
        }

        // 队列见底时尝试再取一批
        setState((latest) => {
          if (latest.queue.length === 0 && !latest.refreshing) {
            void refillIfNeeded();
          }
          return latest;
        });

        return data;
      } catch (err) {
        const ce = err as CommandError;
        setState((s) => ({ ...s, submitting: false }));
        toast.error("提交失败", humanizeError(ce));
        throw err;
      }
    },
    [refillIfNeeded, toast]
  );

  return {
    ...state,
    reload: loadInitial,
    submit,
  };
}
