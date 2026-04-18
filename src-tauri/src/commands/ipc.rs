use tauri::State;

use crate::{
    commands::{generate, review},
    models::card::{GeneratedCardBatchResult, ReviewedGeneratedCardsResult},
    state::AppState,
    utils::error::CommandError,
};

#[tauri::command]
pub async fn generate_cards(
    state: State<'_, AppState>,
    input: generate::GenerateCardsInput,
) -> Result<GeneratedCardBatchResult, CommandError> {
    // 使用注入在 AppState 里的 LLM 客户端（生产启动时已绑定为真实豆包客户端）。
    generate::generate_cards(
        state.llm.as_ref(),
        state.card_dao.as_ref(),
        state.review_dao.as_ref(),
        input,
    )
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_generated_cards(
    state: State<'_, AppState>,
    batch_id: String,
) -> Result<GeneratedCardBatchResult, CommandError> {
    generate::list_generated_cards(state.card_dao.as_ref(), &batch_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn review_generated_cards(
    state: State<'_, AppState>,
    input: generate::ReviewGeneratedCardsInput,
) -> Result<ReviewedGeneratedCardsResult, CommandError> {
    generate::review_generated_cards(state.card_dao.as_ref(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_due_reviews(
    state: State<'_, AppState>,
    input: review::ListDueReviewsInput,
) -> Result<review::CommandResponse<review::ListDueReviewsData>, CommandError> {
    review::list_due_reviews(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn submit_review_result(
    state: State<'_, AppState>,
    input: review::SubmitReviewResultInput,
) -> Result<review::CommandResponse<review::SubmitReviewResultData>, CommandError> {
    review::submit_review_result(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_review_dashboard(
    state: State<'_, AppState>,
) -> Result<review::CommandResponse<review::ReviewDashboardData>, CommandError> {
    review::get_review_dashboard(state.inner())
        .await
        .map_err(CommandError::from)
}
