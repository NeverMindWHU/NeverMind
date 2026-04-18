import { invokeData } from "@/lib/tauri";
import type {
  ListDueReviewsData,
  ListDueReviewsInput,
  ReviewDashboardData,
  SubmitReviewResultData,
  SubmitReviewResultInput,
} from "@/types/review";

export function listDueReviews(input: ListDueReviewsInput = {}) {
  return invokeData<ListDueReviewsData>("list_due_reviews", { input });
}

export function submitReviewResult(input: SubmitReviewResultInput) {
  return invokeData<SubmitReviewResultData>("submit_review_result", { input });
}

export function getReviewDashboard() {
  return invokeData<ReviewDashboardData>("get_review_dashboard");
}
