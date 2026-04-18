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
  /** 主关键词（v1 兼容）。 */
  keyword: string;
  /** v2：完整问题文本（疑问句）。正面展示。 */
  question: string;
  /** v2：3 个关键词。背面 tag 展示。 */
  keywords: string[];
  definition: string;
  explanation: string;
  reviewStep: number;
  dueAt: string;
  /** 旧字段，服务端会把它填成与 keywords 相同的内容，保持 UI 兼容。 */
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

/** 提前开始下一轮复习：拉取 `dueAt > now` 的 pending 卡片。 */
export interface ListUpcomingReviewsInput {
  limit?: number | null;
}

export interface UpcomingReviewsSummary {
  /** 尚未到期的 pending review 总数。 */
  upcomingCount: number;
  /** 最早到期时间；若无 upcoming 则为 null。 */
  earliestDueAt: string | null;
}

export interface ListUpcomingReviewsData {
  items: DueReviewCard[];
  summary: UpcomingReviewsSummary;
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
