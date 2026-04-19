use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::errors::Result;
use crate::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxRepositoryTrait, TaxServiceTrait, TaxYearReport, DEFAULT_TAX_JURISDICTION,
    DEFAULT_TAX_REGIME,
};

pub struct TaxService<T: TaxRepositoryTrait> {
    repository: Arc<T>,
}

impl<T: TaxRepositoryTrait> TaxService<T> {
    pub fn new(repository: Arc<T>) -> Self {
        Self { repository }
    }

    fn rule_pack_version(tax_year: i32, jurisdiction: &str) -> String {
        format!("{jurisdiction}-{tax_year}-securities-v1")
    }
}

#[async_trait]
impl<T: TaxRepositoryTrait + Send + Sync> TaxServiceTrait for TaxService<T> {
    fn get_tax_profile(&self) -> Result<TaxProfile> {
        if let Some(profile) = self.repository.get_tax_profile()? {
            return Ok(profile);
        }

        let now = Utc::now().naive_utc();
        Ok(TaxProfile {
            jurisdiction: DEFAULT_TAX_JURISDICTION.to_string(),
            tax_residence_country: DEFAULT_TAX_JURISDICTION.to_string(),
            default_tax_regime: DEFAULT_TAX_REGIME.to_string(),
            pfu_or_bareme_preference: Some("PFU".to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_tax_profile(&self, profile: TaxProfileUpdate) -> Result<TaxProfile> {
        self.repository.upsert_tax_profile(profile).await
    }

    fn get_account_tax_profiles(&self) -> Result<Vec<AccountTaxProfile>> {
        self.repository.get_account_tax_profiles()
    }

    fn get_account_tax_profile(&self, account_id: &str) -> Result<Option<AccountTaxProfile>> {
        self.repository.get_account_tax_profile(account_id)
    }

    async fn update_account_tax_profile(
        &self,
        profile: AccountTaxProfileUpdate,
    ) -> Result<AccountTaxProfile> {
        self.repository.upsert_account_tax_profile(profile).await
    }

    fn list_tax_year_reports(&self) -> Result<Vec<TaxYearReport>> {
        self.repository.list_tax_year_reports()
    }

    fn get_tax_year_report(&self, id: &str) -> Result<Option<TaxYearReport>> {
        self.repository.get_tax_year_report(id)
    }

    async fn create_tax_year_report(&self, report: NewTaxYearReport) -> Result<TaxYearReport> {
        let jurisdiction = report
            .jurisdiction
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_TAX_JURISDICTION.to_string());
        let base_currency = report
            .base_currency
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "EUR".to_string());

        if let Some(existing) = self
            .repository
            .find_draft_tax_year_report(report.tax_year, &jurisdiction)?
        {
            return Ok(existing);
        }

        let rule_pack_version = Self::rule_pack_version(report.tax_year, &jurisdiction);
        self.repository
            .create_tax_year_report(report, jurisdiction, base_currency, rule_pack_version)
            .await
    }
}
