use std::sync::Arc;

use crate::context::ServiceContext;
use log::debug;
use tauri::State;
use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxYearReport,
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
