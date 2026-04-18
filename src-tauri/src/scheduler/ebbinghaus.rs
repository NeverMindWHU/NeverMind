use chrono::{DateTime, Duration, Utc};

use crate::{
    models::review::ReviewResult,
    scheduler::rules::{interval_days_for_step, SKIPPED_DELAY_HOURS},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextReviewDecision {
    pub next_step: i64,
    pub next_due_at: DateTime<Utc>,
    pub status: &'static str,
}

pub fn next_review(
    current_step: i64,
    result: ReviewResult,
    reviewed_at: DateTime<Utc>,
) -> NextReviewDecision {
    match result {
        ReviewResult::Remembered => {
            let next_step = current_step.max(1) + 1;
            let next_due_at = reviewed_at + Duration::days(interval_days_for_step(next_step));

            NextReviewDecision {
                next_step,
                next_due_at,
                status: "pending",
            }
        }
        ReviewResult::Forgotten => NextReviewDecision {
            next_step: 1,
            next_due_at: reviewed_at + Duration::days(interval_days_for_step(1)),
            status: "pending",
        },
        ReviewResult::Skipped => NextReviewDecision {
            next_step: current_step.max(1),
            next_due_at: reviewed_at + Duration::hours(SKIPPED_DELAY_HOURS),
            status: "pending",
        },
        ReviewResult::Done => NextReviewDecision {
            next_step: current_step.max(1),
            next_due_at: reviewed_at,
            status: "done",
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::next_review;
    use crate::models::review::ReviewResult;

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
    fn done_marks_schedule_as_completed() {
        let now = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let decision = next_review(2, ReviewResult::Done, now);

        assert_eq!(decision.next_step, 2);
        assert_eq!(decision.next_due_at, now);
        assert_eq!(decision.status, "done");
    }
}
