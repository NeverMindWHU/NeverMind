use tauri::State;

use crate::{
    commands::{generate, library, review, settings},
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
pub async fn list_upcoming_reviews(
    state: State<'_, AppState>,
    input: review::ListUpcomingReviewsInput,
) -> Result<review::CommandResponse<review::ListUpcomingReviewsData>, CommandError> {
    review::list_upcoming_reviews(state.inner(), input)
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

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<settings::CommandResponse<settings::AppSettingsData>, CommandError> {
    settings::get_settings(state.inner())
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: settings::UpdateSettingsInput,
) -> Result<settings::CommandResponse<settings::UpdateSettingsData>, CommandError> {
    settings::update_settings(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_model_profiles(
    state: State<'_, AppState>,
) -> Result<settings::CommandResponse<settings::ListModelProfilesData>, CommandError> {
    settings::list_model_profiles(state.inner())
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn save_model_profile(
    state: State<'_, AppState>,
    input: settings::SaveModelProfileInput,
) -> Result<settings::CommandResponse<settings::SaveModelProfileData>, CommandError> {
    settings::save_model_profile(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn test_model_profile(
    input: settings::TestModelProfileInput,
) -> Result<settings::CommandResponse<settings::TestModelProfileData>, CommandError> {
    settings::test_model_profile(input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn clear_library(
    state: State<'_, AppState>,
) -> Result<settings::CommandResponse<settings::ClearLibraryData>, CommandError> {
    settings::clear_library(state.inner())
        .await
        .map_err(CommandError::from)
}

// ---- 宝库（Library）-------------------------------------------------------

#[tauri::command]
pub async fn library_search_by_keyword(
    state: State<'_, AppState>,
    input: library::SearchByKeywordInput,
) -> Result<library::SearchCardsResult, CommandError> {
    library::search_by_keyword(state.card_dao.as_ref(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn library_search_by_question(
    state: State<'_, AppState>,
    input: library::SearchByQuestionInput,
) -> Result<library::SearchCardsResult, CommandError> {
    library::search_by_question(state.card_dao.as_ref(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn library_list_keyword_buckets(
    state: State<'_, AppState>,
    input: library::ListKeywordBucketsInput,
) -> Result<library::KeywordBucketsResult, CommandError> {
    library::list_keyword_buckets(state.card_dao.as_ref(), input)
        .await
        .map_err(CommandError::from)
}
