use std::sync::Arc;

use crate::context::ServiceContext;
use log::debug;
use tauri::State;
use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, ExtractedTaxField, ExtractedTaxFieldUpdate,
    NewTaxYearReport, TaxDocument, TaxDocumentExtractionRequest, TaxDocumentExtractionResult,
    TaxDocumentUpload, TaxProfile, TaxProfileUpdate, TaxReconciliationEntry,
    TaxReconciliationEntryUpdate, TaxReportDetail, TaxYearReport,
};

#[tauri::command]
pub async fn get_tax_profile(state: State<'_, Arc<ServiceContext>>) -> Result<TaxProfile, String> {
    debug!("Fetching tax profile...");
    state
        .tax_service()
        .get_tax_profile()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tax_profile(
    profile: TaxProfileUpdate,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxProfile, String> {
    debug!("Updating tax profile...");
    state
        .tax_service()
        .update_tax_profile(profile)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_account_tax_profiles(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<AccountTaxProfile>, String> {
    debug!("Fetching account tax profiles...");
    state
        .tax_service()
        .get_account_tax_profiles()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_account_tax_profile(
    profile: AccountTaxProfileUpdate,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<AccountTaxProfile, String> {
    debug!("Updating account tax profile...");
    state
        .tax_service()
        .update_account_tax_profile(profile)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_tax_year_reports(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<TaxYearReport>, String> {
    debug!("Listing tax year reports...");
    state
        .tax_service()
        .list_tax_year_reports()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tax_year_report(
    id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Option<TaxYearReport>, String> {
    debug!("Fetching tax year report...");
    state
        .tax_service()
        .get_tax_year_report(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tax_year_report(
    report: NewTaxYearReport,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxYearReport, String> {
    debug!("Creating tax year report...");
    state
        .tax_service()
        .create_tax_year_report(report)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tax_report_detail(
    id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Option<TaxReportDetail>, String> {
    debug!("Fetching tax report detail...");
    state
        .tax_service()
        .get_tax_report_detail(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn regenerate_tax_year_report(
    id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxReportDetail, String> {
    debug!("Regenerating tax year report...");
    state
        .tax_service()
        .regenerate_tax_year_report(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn finalize_tax_year_report(
    id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxYearReport, String> {
    debug!("Finalizing tax year report...");
    state
        .tax_service()
        .finalize_tax_year_report(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_tax_document(
    upload: TaxDocumentUpload,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxDocument, String> {
    debug!("Uploading tax document...");
    state
        .tax_service()
        .upload_tax_document(upload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_tax_documents(
    report_id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<TaxDocument>, String> {
    debug!("Listing tax documents...");
    state
        .tax_service()
        .list_tax_documents(&report_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_tax_document(
    request: TaxDocumentExtractionRequest,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxDocumentExtractionResult, String> {
    debug!("Extracting tax document...");
    state
        .tax_service()
        .extract_tax_document(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_extracted_tax_field(
    update: ExtractedTaxFieldUpdate,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<ExtractedTaxField, String> {
    debug!("Updating extracted tax field...");
    state
        .tax_service()
        .update_extracted_tax_field(update)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reconcile_tax_year_report(
    id: String,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<TaxReconciliationEntry>, String> {
    debug!("Reconciling tax year report...");
    state
        .tax_service()
        .reconcile_tax_year_report(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tax_reconciliation_entry(
    update: TaxReconciliationEntryUpdate,
    state: State<'_, Arc<ServiceContext>>,
) -> Result<TaxReconciliationEntry, String> {
    debug!("Updating tax reconciliation entry...");
    state
        .tax_service()
        .update_tax_reconciliation_entry(update)
        .await
        .map_err(|e| e.to_string())
}
