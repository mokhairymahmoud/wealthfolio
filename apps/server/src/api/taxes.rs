use std::sync::Arc;

use crate::{error::ApiResult, main_lib::AppState};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, ExtractedTaxField, ExtractedTaxFieldUpdate,
    NewTaxYearReport, TaxDocument, TaxDocumentExtractionRequest, TaxDocumentExtractionResult,
    TaxDocumentUpload, TaxProfile, TaxProfileUpdate, TaxReconciliationEntry,
    TaxReconciliationEntryUpdate, TaxReportDetail, TaxYearReport,
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

async fn get_tax_report_detail(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Option<TaxReportDetail>>> {
    let report = state.tax_service.get_tax_report_detail(&id)?;
    Ok(Json(report))
}

async fn regenerate_tax_year_report(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<TaxReportDetail>> {
    let report = state.tax_service.regenerate_tax_year_report(&id).await?;
    Ok(Json(report))
}

async fn finalize_tax_year_report(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<TaxYearReport>> {
    let report = state.tax_service.finalize_tax_year_report(&id).await?;
    Ok(Json(report))
}

async fn upload_tax_document(
    State(state): State<Arc<AppState>>,
    Json(upload): Json<TaxDocumentUpload>,
) -> ApiResult<Json<TaxDocument>> {
    let document = state.tax_service.upload_tax_document(upload).await?;
    Ok(Json(document))
}

async fn list_tax_documents(
    Path(report_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<TaxDocument>>> {
    let documents = state.tax_service.list_tax_documents(&report_id)?;
    Ok(Json(documents))
}

async fn extract_tax_document(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TaxDocumentExtractionRequest>,
) -> ApiResult<Json<TaxDocumentExtractionResult>> {
    let extraction = state.tax_service.extract_tax_document(request).await?;
    Ok(Json(extraction))
}

async fn update_extracted_tax_field(
    State(state): State<Arc<AppState>>,
    Json(update): Json<ExtractedTaxFieldUpdate>,
) -> ApiResult<Json<ExtractedTaxField>> {
    let field = state.tax_service.update_extracted_tax_field(update).await?;
    Ok(Json(field))
}

async fn reconcile_tax_year_report(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<TaxReconciliationEntry>>> {
    let entries = state.tax_service.reconcile_tax_year_report(&id).await?;
    Ok(Json(entries))
}

async fn update_tax_reconciliation_entry(
    State(state): State<Arc<AppState>>,
    Json(update): Json<TaxReconciliationEntryUpdate>,
) -> ApiResult<Json<TaxReconciliationEntry>> {
    let entry = state
        .tax_service
        .update_tax_reconciliation_entry(update)
        .await?;
    Ok(Json(entry))
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
        .route("/taxes/reports/{id}/detail", get(get_tax_report_detail))
        .route(
            "/taxes/reports/{id}/regenerate",
            post(regenerate_tax_year_report),
        )
        .route(
            "/taxes/reports/{id}/finalize",
            post(finalize_tax_year_report),
        )
        .route("/taxes/reports/{id}/documents", get(list_tax_documents))
        .route(
            "/taxes/reports/{id}/reconcile",
            post(reconcile_tax_year_report),
        )
        .route("/taxes/documents", post(upload_tax_document))
        .route("/taxes/documents/extract", post(extract_tax_document))
        .route("/taxes/extracted-fields", post(update_extracted_tax_field))
        .route(
            "/taxes/reconciliation",
            post(update_tax_reconciliation_entry),
        )
}
