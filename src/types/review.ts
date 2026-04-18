/**
 * 对齐 `docs/architecture/contracts/review.md`
 * 和 Rust `commands::review`。
 */

export type ReviewResult = "done" | "remembered" | "forgotten" | "skipped";

export interface ListDueReviewsInput {
  limit?: number | null;
  cursor?: string | null;
  includeCompletedToday?: boolean | null;
}

export interface DueReviewCard {
  reviewId: string;
  cardId: string;
  keyword: string;
  definition: string;
  explanation: string;
  reviewStep: number;
  dueAt: string;
  tags: string[];
}

export interface DueReviewsSummary {
  dueCount: number;
  completedToday: number;
}

export interface ListDueReviewsData {
  items: DueReviewCard[];
  nextCursor: string | null;
  summary: DueReviewsSummary;
}

export interface SubmitReviewResultInput {
  reviewId: string;
  cardId: string;
  result: ReviewResult;
  reviewedAt: string;
}

export interface SubmitReviewResultData {
  cardId: string;
  result: ReviewResult;
  previousStep: number;
  nextStep: number;
  nextReviewAt: string;
  remainingDueCount: number;
}

export interface ReviewDashboardData {
  dueToday: number;
  completedToday: number;
  streakDays: number;
  nextDueAt: string | null;
}
