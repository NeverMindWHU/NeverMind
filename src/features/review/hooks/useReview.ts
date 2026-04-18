import { useCallback, useEffect, useState } from "react";
import { useToast } from "@/lib/toast";
import { humanizeError } from "@/lib/format";
import type { CommandError } from "@/types/common";
import type {
  DueReviewCard,
  ReviewDashboardData,
  ReviewResult,
  UpcomingReviewsSummary,
} from "@/types/review";
import {
  getReviewDashboard,
  listDueReviews,
  listUpcomingReviews,
  submitReviewResult,
} from "../services/api";

/**
 * - `due`: 默认模式，队列里是今天及更早到期的卡片。
 * - `upcoming`: "提前复习下一轮"模式，队列里是尚未到期、被用户主动拉进来的卡片。
 *   用户提交后走相同的 Ebbinghaus 推进逻辑；队列空时自动回到 `due` 模式并刷新。
 */
export type ReviewMode = "due" | "upcoming";

interface State {
  loading: boolean;
  refreshing: boolean;
  submitting: boolean;
  error: CommandError | null;
  queue: DueReviewCard[];
  dashboard: ReviewDashboardData | null;
  completedThisSession: number;
  mode: ReviewMode;
  /** 来自最近一次 upcoming 查询，供"下一轮复习"入口展示用。 */
  upcoming: UpcomingReviewsSummary | null;
}

const INITIAL_STATE: State = {
  loading: true,
  refreshing: false,
  submitting: false,
  error: null,
  queue: [],
  dashboard: null,
  completedThisSession: 0,
  mode: "due",
  upcoming: null,
};

/**
 * 复习循环：一次拉取一批到期卡片，按先入先出消耗。队列清空后尝试再拉一次；
 * 若仍为空，则结束本次复习。
 *
 * 额外支持"提前开始下一轮复习"：当今日到期清空后，调用 `startUpcoming()` 拉一批
 * 尚未到期的 pending 卡进同一个队列，提交仍走 `submit_review_result`。
 */
export function useReview() {
  const toast = useToast();
  const [state, setState] = useState<State>(INITIAL_STATE);

  const loadInitial = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null, mode: "due" }));
    try {
      const [list, dashboard, upcoming] = await Promise.all([
        listDueReviews({ limit: 20, includeCompletedToday: true }),
        getReviewDashboard(),
        // 顺带把"下一轮"摘要取回来，让 UI 能马上展示"提前复习 X 张"按钮。
        listUpcomingReviews({ limit: 1 }),
      ]);
      setState((s) => ({
        ...s,
        loading: false,
        queue: list.items,
        dashboard,
        upcoming: upcoming.summary,
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
      const [list, upcoming] = await Promise.all([
        listDueReviews({ limit: 20 }),
        listUpcomingReviews({ limit: 1 }),
      ]);
      setState((s) => ({
        ...s,
        refreshing: false,
        queue: list.items,
        // 回到常规 due 模式；upcoming 摘要同步刷新。
        mode: "due",
        upcoming: upcoming.summary,
      }));
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, refreshing: false }));
      toast.error("刷新失败", humanizeError(ce));
    }
  }, [toast]);

  /**
   * 提前拉下一轮（尚未到期）的 pending 卡入队列。仅在当前队列为空时允许调用，
   * 避免把 upcoming 卡混入真正到期的队列。
   */
  const startUpcoming = useCallback(async () => {
    setState((s) => ({ ...s, refreshing: true }));
    try {
      const list = await listUpcomingReviews({ limit: 20 });
      setState((s) => ({
        ...s,
        refreshing: false,
        queue: list.items,
        mode: "upcoming",
        upcoming: list.summary,
      }));
      if (list.items.length === 0) {
        toast.info("暂无下一轮", "后续没有已安排的复习任务，先去生成更多卡片吧。");
      } else {
        toast.success("已提前加载", `本轮提前复习 ${list.items.length} 张卡片。`);
      }
    } catch (err) {
      const ce = err as CommandError;
      setState((s) => ({ ...s, refreshing: false }));
      toast.error("加载下一轮失败", humanizeError(ce));
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
          // 仅在"常规到期"模式下刷新 dashboard 的"今日"计数；提前复习也算完成，
          // 但它本不计入今日到期，所以 dueToday 不做衰减。completedToday 仍然 +1
          // （因为数据库里也会新增一条 review_log），保持与后端一致。
          const nextDashboard = s.dashboard
            ? {
                ...s.dashboard,
                completedToday: s.dashboard.completedToday + 1,
                dueToday:
                  s.mode === "due"
                    ? Math.max(0, s.dashboard.dueToday - 1)
                    : s.dashboard.dueToday,
                nextDueAt: data.nextReviewAt ?? s.dashboard.nextDueAt,
              }
            : s.dashboard;
          return {
            ...s,
            submitting: false,
            queue: remaining,
            completedThisSession: s.completedThisSession + 1,
            dashboard: nextDashboard,
          };
        });

        if (result === "forgotten") {
          toast.info("已重置", "这张卡明天会再次出现");
        }

        setState((latest) => {
          if (latest.queue.length === 0 && !latest.refreshing) {
            // 无论 due 还是 upcoming 模式，队列见底都统一走 refillIfNeeded：
            // 它会把 upcoming summary 一并刷新，并把 mode 复位为 "due"，
            // UI 就能据此展示"本轮完成，还可以提前复习 X 张"或"已全部完成"的状态。
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
    startUpcoming,
    submit,
  };
}
