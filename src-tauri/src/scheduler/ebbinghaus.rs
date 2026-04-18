use chrono::{DateTime, Duration, Utc};

use crate::{
    models::review::ReviewResult,
    scheduler::rules::{
        interval_days_for_step, max_review_step, normalize_review_step, INITIAL_REVIEW_STATUS,
        INITIAL_REVIEW_STEP, SKIPPED_DELAY_HOURS,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextReviewDecision {
    pub next_step: i64,
    pub next_due_at: DateTime<Utc>,
    pub status: &'static str,
}

/// 新卡的第一次复习安排。
///
/// 对齐 Anki 的「learning queue」心智：卡片一旦接受入库就立即可复习，
/// 让用户当天就能做一次首检。之后的 step+1 间隔沿用艾宾浩斯表。
///
/// 历史上这里是 +1 天，会导致"今天生成的卡明天才能看到"，对用户反直觉。
pub fn first_review(created_at: DateTime<Utc>) -> NextReviewDecision {
    NextReviewDecision {
        next_step: INITIAL_REVIEW_STEP,
        next_due_at: created_at,
        status: INITIAL_REVIEW_STATUS,
    }
}

pub fn next_review(
    current_step: i64,
    result: ReviewResult,
    reviewed_at: DateTime<Utc>,
) -> NextReviewDecision {
    let normalized_step = normalize_review_step(current_step);

    match result {
        ReviewResult::Remembered => {
            let next_step = (normalized_step + 1).min(max_review_step());
            let next_due_at = reviewed_at + Duration::days(interval_days_for_step(next_step));

            NextReviewDecision {
                next_step,
                next_due_at,
                status: INITIAL_REVIEW_STATUS,
            }
        }
        ReviewResult::Forgotten => NextReviewDecision {
            next_step: INITIAL_REVIEW_STEP,
            next_due_at: reviewed_at + Duration::days(interval_days_for_step(INITIAL_REVIEW_STEP)),
            status: INITIAL_REVIEW_STATUS,
        },
        ReviewResult::Skipped => NextReviewDecision {
            next_step: normalized_step,
            next_due_at: reviewed_at + Duration::hours(SKIPPED_DELAY_HOURS),
            status: INITIAL_REVIEW_STATUS,
        },
        ReviewResult::Done => NextReviewDecision {
            next_step: normalized_step,
            next_due_at: reviewed_at,
            status: "done",
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{first_review, next_review};
    use crate::models::review::ReviewResult;

    #[test]
    fn first_review_is_due_immediately_after_creation() {
        let created_at = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = first_review(created_at);

        assert_eq!(decision.next_step, 1);
        assert_eq!(decision.next_due_at, created_at);
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn remembered_moves_to_next_step() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(1, ReviewResult::Remembered, now);

        assert_eq!(decision.next_step, 2);
        assert_eq!(
            decision.next_due_at,
            Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap()
        );
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn remembered_stays_at_last_defined_step() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(6, ReviewResult::Remembered, now);

        assert_eq!(decision.next_step, 6);
        assert_eq!(
            decision.next_due_at,
            Utc.with_ymd_and_hms(2026, 5, 18, 10, 0, 0).unwrap()
        );
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn forgotten_resets_to_first_step() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(4, ReviewResult::Forgotten, now);

        assert_eq!(decision.next_step, 1);
        assert_eq!(
            decision.next_due_at,
            Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap()
        );
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn skipped_keeps_step_and_delays_one_hour() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(3, ReviewResult::Skipped, now);

        assert_eq!(decision.next_step, 3);
        assert_eq!(
            decision.next_due_at,
            Utc.with_ymd_and_hms(2026, 4, 18, 11, 0, 0).unwrap()
        );
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn skipped_normalizes_invalid_step_to_first_step() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(0, ReviewResult::Skipped, now);

        assert_eq!(decision.next_step, 1);
        assert_eq!(
            decision.next_due_at,
            Utc.with_ymd_and_hms(2026, 4, 18, 11, 0, 0).unwrap()
        );
        assert_eq!(decision.status, "pending");
    }

    #[test]
    fn done_marks_schedule_as_completed() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(2, ReviewResult::Done, now);

        assert_eq!(decision.next_step, 2);
        assert_eq!(decision.next_due_at, now);
        assert_eq!(decision.status, "done");
    }

    #[test]
    fn done_normalizes_invalid_step_before_marking_completed() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(-2, ReviewResult::Done, now);

        assert_eq!(decision.next_step, 1);
        assert_eq!(decision.next_due_at, now);
        assert_eq!(decision.status, "done");
    }
}
