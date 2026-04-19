use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxYearReport,
};

async fn get_tax_profile(State(state): State<Arc<AppState>>) -> ApiResult<Json<TaxProfile>> {
    let profile = state.tax_service.get_tax_profile()?;
    Ok(Json(profile))
}

async fn update_tax_profile(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<TaxProfileUpdate>,
) -> ApiResult<Json<TaxProfile>> {
    let updated = state.tax_service.update_tax_profile(profile).await?;
    Ok(Json(updated))
}

async fn get_account_tax_profiles(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<AccountTaxProfile>>> {
    let profiles = state.tax_service.get_account_tax_profiles()?;
    Ok(Json(profiles))
}

async fn update_account_tax_profile(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<AccountTaxProfileUpdate>,
) -> ApiResult<Json<AccountTaxProfile>> {
    let updated = state
        .tax_service
        .update_account_tax_profile(profile)
        .await?;
    Ok(Json(updated))
}

async fn list_tax_year_reports(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<TaxYearReport>>> {
    let reports = state.tax_service.list_tax_year_reports()?;
    Ok(Json(reports))
}

async fn get_tax_year_report(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Option<TaxYearReport>>> {
    let report = state.tax_service.get_tax_year_report(&id)?;
    Ok(Json(report))
}

async fn create_tax_year_report(
    State(state): State<Arc<AppState>>,
    Json(report): Json<NewTaxYearReport>,
) -> ApiResult<Json<TaxYearReport>> {
    let created = state.tax_service.create_tax_year_report(report).await?;
    Ok(Json(created))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/taxes/profile",
            get(get_tax_profile).put(update_tax_profile),
        )
        .route(
            "/taxes/accounts",
            get(get_account_tax_profiles).put(update_account_tax_profile),
        )
        .route(
            "/taxes/reports",
            get(list_tax_year_reports).post(create_tax_year_report),
        )
        .route("/taxes/reports/{id}", get(get_tax_year_report))
}
