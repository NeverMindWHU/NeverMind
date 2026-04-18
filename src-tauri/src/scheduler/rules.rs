pub const INITIAL_REVIEW_STEP: i64 = 1;
pub const INITIAL_REVIEW_STATUS: &str = "pending";
pub const REVIEW_INTERVAL_DAYS: [i64; 6] = [1, 1, 3, 7, 15, 30];
pub const SKIPPED_DELAY_HOURS: i64 = 1;

pub fn max_review_step() -> i64 {
    REVIEW_INTERVAL_DAYS.len() as i64
}

pub fn normalize_review_step(step: i64) -> i64 {
    step.clamp(INITIAL_REVIEW_STEP, max_review_step())
}

pub fn interval_days_for_step(step: i64) -> i64 {
    let normalized_step = normalize_review_step(step) as usize;
    let index = normalized_step
        .saturating_sub(1)
        .min(REVIEW_INTERVAL_DAYS.len().saturating_sub(1));

    REVIEW_INTERVAL_DAYS[index]
}

#[cfg(test)]
mod tests {
    use super::{interval_days_for_step, max_review_step, normalize_review_step};

    #[test]
    fn normalize_review_step_raises_small_values_to_first_step() {
        assert_eq!(normalize_review_step(0), 1);
        assert_eq!(normalize_review_step(-3), 1);
    }

    #[test]
    fn normalize_review_step_caps_values_at_last_defined_step() {
        assert_eq!(normalize_review_step(6), 6);
        assert_eq!(normalize_review_step(9), 6);
        assert_eq!(max_review_step(), 6);
    }

    #[test]
    fn interval_days_for_step_uses_normalized_boundaries() {
        assert_eq!(interval_days_for_step(0), 1);
        assert_eq!(interval_days_for_step(3), 3);
        assert_eq!(interval_days_for_step(99), 30);
    }
}
