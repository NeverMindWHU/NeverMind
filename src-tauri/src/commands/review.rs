use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    models::review::{DueReviewItem, NewReviewLog, ReviewResult},
    scheduler::ebbinghaus::next_review,
    state::AppState,
    utils::error::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> CommandResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListDueReviewsInput {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub include_completed_today: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReviewCard {
    pub review_id: String,
    pub card_id: String,
    pub keyword: String,
    pub definition: String,
    pub explanation: String,
    pub review_step: i64,
    pub due_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReviewsSummary {
    pub due_count: i64,
    pub completed_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDueReviewsData {
    pub items: Vec<DueReviewCard>,
    pub next_cursor: Option<String>,
    pub summary: DueReviewsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewResultInput {
    pub review_id: String,
    pub card_id: String,
    pub result: ReviewResult,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewResultData {
    pub card_id: String,
    pub result: ReviewResult,
    pub previous_step: i64,
    pub next_step: i64,
    pub next_review_at: DateTime<Utc>,
    pub remaining_due_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDashboardData {
    pub due_today: i64,
    pub completed_today: i64,
    pub streak_days: i64,
    pub next_due_at: Option<DateTime<Utc>>,
}

pub async fn list_due_reviews(
    state: &AppState,
    input: ListDueReviewsInput,
) -> AppResult<CommandResponse<ListDueReviewsData>> {
    let limit = input.limit.unwrap_or(20).clamp(1, 100);
    let items = state.review_dao.list_due_reviews(limit).await?;
    let due_count = state.review_dao.count_due_reviews().await?;
    let completed_today = if input.include_completed_today.unwrap_or(false) {
        count_completed_today(state, Utc::now()).await?
    } else {
        0
    };

    Ok(CommandResponse::ok(ListDueReviewsData {
        items: items.into_iter().map(map_due_review_item).collect(),
        next_cursor: input.cursor.and(None),
        summary: DueReviewsSummary {
            due_count,
            completed_today,
        },
    }))
}

pub async fn submit_review_result(
    state: &AppState,
    input: SubmitReviewResultInput,
) -> AppResult<CommandResponse<SubmitReviewResultData>> {
    let schedule = state.review_dao.get_schedule(&input.review_id).await?;
    if schedule.card_id != input.card_id {
        return Err(AppError::Validation {
            message: "reviewId 与 cardId 不匹配".to_string(),
        });
    }

    let decision = next_review(schedule.review_step, input.result, input.reviewed_at);
    state
        .review_dao
        .update_schedule_after_review(
            &input.review_id,
            decision.next_step,
            decision.next_due_at,
            decision.status,
        )
        .await?;

    let log = NewReviewLog {
        id: Uuid::new_v4().to_string(),
        review_schedule_id: input.review_id.clone(),
        card_id: input.card_id.clone(),
        result: input.result.as_str().to_string(),
        previous_step: schedule.review_step,
        next_step: decision.next_step,
        reviewed_at: input.reviewed_at,
    };
    state.review_dao.insert_review_log(&log).await?;

    let remaining_due_count = state.review_dao.count_due_reviews().await?;

    Ok(CommandResponse::ok(SubmitReviewResultData {
        card_id: input.card_id,
        result: input.result,
        previous_step: schedule.review_step,
        next_step: decision.next_step,
        next_review_at: decision.next_due_at,
        remaining_due_count,
    }))
}

pub async fn get_review_dashboard(
    state: &AppState,
) -> AppResult<CommandResponse<ReviewDashboardData>> {
    let now = Utc::now();
    let due_today = state.review_dao.count_due_reviews().await?;
    let completed_today = count_completed_today(state, now).await?;
    let next_due_at = state.review_dao.get_next_due_at().await?;
    let streak_days = calculate_streak_days(
        state.review_dao.list_completed_review_days_desc(30).await?,
        now.date_naive(),
    )?;

    Ok(CommandResponse::ok(ReviewDashboardData {
        due_today,
        completed_today,
        streak_days,
        next_due_at,
    }))
}

fn map_due_review_item(item: DueReviewItem) -> DueReviewCard {
    DueReviewCard {
        review_id: item.review_id,
        card_id: item.card_id,
        keyword: item.keyword,
        definition: item.definition,
        explanation: item.explanation,
        review_step: item.review_step,
        due_at: item.due_at,
        tags: Vec::new(),
    }
}

async fn count_completed_today(state: &AppState, now: DateTime<Utc>) -> AppResult<i64> {
    let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = (now.date_naive() + Days::new(1))
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    state
        .review_dao
        .count_completed_reviews_between(start, end)
        .await
}

fn calculate_streak_days(days: Vec<String>, today: NaiveDate) -> AppResult<i64> {
    if days.is_empty() {
        return Ok(0);
    }

    let mut parsed_days = Vec::with_capacity(days.len());
    for day in days {
        let parsed = NaiveDate::parse_from_str(&day, "%Y-%m-%d").map_err(|_| AppError::Validation {
            message: "review_logs 日期格式非法".to_string(),
        })?;
        parsed_days.push(parsed);
    }

    if parsed_days.first().copied() != Some(today) {
        return Ok(0);
    }

    let mut streak = 1_i64;
    let mut expected_day = today;

    for day in parsed_days.into_iter().skip(1) {
        let Some(previous_day) = expected_day.checked_sub_days(Days::new(1)) else {
            break;
        };

        if day == previous_day {
            streak += 1;
            expected_day = day;
        } else if day < previous_day {
            break;
        }
    }

    Ok(streak)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::calculate_streak_days;

    #[test]
    fn streak_counts_consecutive_days_ending_today() {
        let streak = calculate_streak_days(
            vec![
                "2026-04-18".to_string(),
                "2026-04-17".to_string(),
                "2026-04-16".to_string(),
            ],
            NaiveDate::from_ymd_opt(2026, 4, 18).unwrap(),
        )
        .unwrap();

        assert_eq!(streak, 3);
    }

    #[test]
    fn streak_resets_when_today_has_no_completed_reviews() {
        let streak = calculate_streak_days(
            vec!["2026-04-17".to_string(), "2026-04-16".to_string()],
            NaiveDate::from_ymd_opt(2026, 4, 18).unwrap(),
        )
        .unwrap();

        assert_eq!(streak, 0);
    }
}
