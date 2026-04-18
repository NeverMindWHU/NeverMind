pub const REVIEW_INTERVAL_DAYS: [i64; 6] = [1, 1, 3, 7, 15, 30];
pub const SKIPPED_DELAY_HOURS: i64 = 1;

pub fn interval_days_for_step(step: i64) -> i64 {
    let normalized_step = step.max(1) as usize;
    let index = normalized_step
        .saturating_sub(1)
        .min(REVIEW_INTERVAL_DAYS.len().saturating_sub(1));

    REVIEW_INTERVAL_DAYS[index]
}
