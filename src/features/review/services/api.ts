import { invokeData } from "@/lib/tauri";
import type {
  ListDueReviewsData,
  ListDueReviewsInput,
  ListUpcomingReviewsData,
  ListUpcomingReviewsInput,
  ReviewDashboardData,
  SubmitReviewResultData,
  SubmitReviewResultInput,
} from "@/types/review";

export function listDueReviews(input: ListDueReviewsInput = {}) {
  return invokeData<ListDueReviewsData>("list_due_reviews", { input });
}

/**
 * 提前拉取下一轮（尚未到期）的 pending review。
 * 与 listDueReviews 返回结构对齐：items 是 `DueReviewCard[]`，可直接喂给同一个 ReviewCard 组件。
 */
export function listUpcomingReviews(input: ListUpcomingReviewsInput = {}) {
  return invokeData<ListUpcomingReviewsData>("list_upcoming_reviews", { input });
}

export function submitReviewResult(input: SubmitReviewResultInput) {
  return invokeData<SubmitReviewResultData>("submit_review_result", { input });
}

export function getReviewDashboard() {
  return invokeData<ReviewDashboardData>("get_review_dashboard");
}
