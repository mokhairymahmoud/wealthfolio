use async_trait::async_trait;

use crate::errors::Result;
use crate::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxYearReport,
};

#[async_trait]
pub trait TaxRepositoryTrait: Send + Sync {
    fn get_tax_profile(&self) -> Result<Option<TaxProfile>>;
    async fn upsert_tax_profile(&self, profile: TaxProfileUpdate) -> Result<TaxProfile>;

    fn get_account_tax_profiles(&self) -> Result<Vec<AccountTaxProfile>>;
    fn get_account_tax_profile(&self, account_id: &str) -> Result<Option<AccountTaxProfile>>;
    async fn upsert_account_tax_profile(
        &self,
        profile: AccountTaxProfileUpdate,
    ) -> Result<AccountTaxProfile>;

    fn list_tax_year_reports(&self) -> Result<Vec<TaxYearReport>>;
    fn get_tax_year_report(&self, id: &str) -> Result<Option<TaxYearReport>>;
    fn find_draft_tax_year_report(
        &self,
        tax_year: i32,
        jurisdiction: &str,
    ) -> Result<Option<TaxYearReport>>;
    async fn create_tax_year_report(
        &self,
        report: NewTaxYearReport,
        jurisdiction: String,
        base_currency: String,
        rule_pack_version: String,
    ) -> Result<TaxYearReport>;
}

#[async_trait]
pub trait TaxServiceTrait: Send + Sync {
    fn get_tax_profile(&self) -> Result<TaxProfile>;
    async fn update_tax_profile(&self, profile: TaxProfileUpdate) -> Result<TaxProfile>;

    fn get_account_tax_profiles(&self) -> Result<Vec<AccountTaxProfile>>;
    fn get_account_tax_profile(&self, account_id: &str) -> Result<Option<AccountTaxProfile>>;
    async fn update_account_tax_profile(
        &self,
        profile: AccountTaxProfileUpdate,
    ) -> Result<AccountTaxProfile>;

    fn list_tax_year_reports(&self) -> Result<Vec<TaxYearReport>>;
    fn get_tax_year_report(&self, id: &str) -> Result<Option<TaxYearReport>>;
    async fn create_tax_year_report(&self, report: NewTaxYearReport) -> Result<TaxYearReport>;
}
