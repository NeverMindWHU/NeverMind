use chrono::{DateTime, Utc};

use crate::nevermind::{
    models::review::NewReviewSchedule,
    scheduler::ebbinghaus::{first_review, NextReviewDecision},
};

pub fn build_initial_schedule(
    schedule_id: String,
    card_id: String,
    created_at: DateTime<Utc>,
) -> NewReviewSchedule {
    let NextReviewDecision {
        next_step,
        next_due_at,
        status,
    } = first_review(created_at);

    NewReviewSchedule {
        id: schedule_id,
        card_id,
        review_step: next_step,
        due_at: next_due_at,
        status: status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::build_initial_schedule;

    #[test]
    fn build_initial_schedule_follows_first_review_rule() {
        let created_at = Utc.with_ymd_and_hms(2026, 4, 18, 10, 0, 0).unwrap();
        let schedule = build_initial_schedule(
            "schedule-1".to_string(),
            "card-1".to_string(),
            created_at,
        );

        assert_eq!(schedule.id, "schedule-1");
        assert_eq!(schedule.card_id, "card-1");
        assert_eq!(schedule.review_step, 1);
        assert_eq!(
            schedule.due_at,
            Utc.with_ymd_and_hms(2026, 4, 19, 10, 0, 0).unwrap()
        );
        assert_eq!(schedule.status, "pending");
    }
}
