use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{self, Pool};
use diesel::SqliteConnection;
use std::sync::Arc;
use uuid::Uuid;

use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxReportStatus, TaxRepositoryTrait, TaxYearReport,
};
use wealthfolio_core::Result;

use super::model::{AccountTaxProfileDB, TaxProfileDB, TaxYearReportDB};
use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::{account_tax_profiles, tax_profiles, tax_year_reports};

const DEFAULT_PROFILE_ID: &str = "default";

pub struct TaxRepository {
    pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
}

impl TaxRepository {
    pub fn new(
        pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
        writer: WriteHandle,
    ) -> Self {
        Self { pool, writer }
    }
}

#[async_trait]
impl TaxRepositoryTrait for TaxRepository {
    fn get_tax_profile(&self) -> Result<Option<TaxProfile>> {
        let mut conn = get_connection(&self.pool)?;
        let result = tax_profiles::table
            .select(TaxProfileDB::as_select())
            .find(DEFAULT_PROFILE_ID)
            .first::<TaxProfileDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;
        Ok(result.map(TaxProfile::from))
    }

    async fn upsert_tax_profile(&self, profile: TaxProfileUpdate) -> Result<TaxProfile> {
        let mut profile_db = TaxProfileDB::from(profile);
        self.writer
            .exec_tx(move |tx| -> Result<TaxProfile> {
                if let Some(existing) = tax_profiles::table
                    .select(TaxProfileDB::as_select())
                    .find(DEFAULT_PROFILE_ID)
                    .first::<TaxProfileDB>(tx.conn())
                    .optional()
                    .map_err(StorageError::from)?
                {
                    profile_db.created_at = existing.created_at;
                }

                diesel::insert_into(tax_profiles::table)
                    .values(&profile_db)
                    .on_conflict(tax_profiles::id)
                    .do_update()
                    .set(&profile_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                Ok(TaxProfile::from(profile_db))
            })
            .await
    }

    fn get_account_tax_profiles(&self) -> Result<Vec<AccountTaxProfile>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = account_tax_profiles::table
            .select(AccountTaxProfileDB::as_select())
            .order(account_tax_profiles::account_id.asc())
            .load::<AccountTaxProfileDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(AccountTaxProfile::from).collect())
    }

    fn get_account_tax_profile(&self, account_id: &str) -> Result<Option<AccountTaxProfile>> {
        let mut conn = get_connection(&self.pool)?;
        let row = account_tax_profiles::table
            .select(AccountTaxProfileDB::as_select())
            .find(account_id)
            .first::<AccountTaxProfileDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;
        Ok(row.map(AccountTaxProfile::from))
    }

    async fn upsert_account_tax_profile(
        &self,
        profile: AccountTaxProfileUpdate,
    ) -> Result<AccountTaxProfile> {
        let mut profile_db = AccountTaxProfileDB::from(profile);
        self.writer
            .exec_tx(move |tx| -> Result<AccountTaxProfile> {
                if let Some(existing) = account_tax_profiles::table
                    .select(AccountTaxProfileDB::as_select())
                    .find(profile_db.account_id.clone())
                    .first::<AccountTaxProfileDB>(tx.conn())
                    .optional()
                    .map_err(StorageError::from)?
                {
                    profile_db.created_at = existing.created_at;
                }

                diesel::insert_into(account_tax_profiles::table)
                    .values(&profile_db)
                    .on_conflict(account_tax_profiles::account_id)
                    .do_update()
                    .set(&profile_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                Ok(AccountTaxProfile::from(profile_db))
            })
            .await
    }

    fn list_tax_year_reports(&self) -> Result<Vec<TaxYearReport>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = tax_year_reports::table
            .select(TaxYearReportDB::as_select())
            .order((
                tax_year_reports::tax_year.desc(),
                tax_year_reports::created_at.desc(),
            ))
            .load::<TaxYearReportDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(TaxYearReport::from).collect())
    }

    fn get_tax_year_report(&self, report_id: &str) -> Result<Option<TaxYearReport>> {
        let mut conn = get_connection(&self.pool)?;
        let row = tax_year_reports::table
            .select(TaxYearReportDB::as_select())
            .find(report_id)
            .first::<TaxYearReportDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;
        Ok(row.map(TaxYearReport::from))
    }

    fn find_draft_tax_year_report(
        &self,
        tax_year: i32,
        jurisdiction: &str,
    ) -> Result<Option<TaxYearReport>> {
        let mut conn = get_connection(&self.pool)?;
        let row = tax_year_reports::table
            .select(TaxYearReportDB::as_select())
            .filter(tax_year_reports::tax_year.eq(tax_year))
            .filter(tax_year_reports::jurisdiction.eq(jurisdiction))
            .filter(tax_year_reports::status.eq(TaxReportStatus::Draft.as_str()))
            .first::<TaxYearReportDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;
        Ok(row.map(TaxYearReport::from))
    }

    async fn create_tax_year_report(
        &self,
        report: NewTaxYearReport,
        jurisdiction: String,
        base_currency: String,
        rule_pack_version: String,
    ) -> Result<TaxYearReport> {
        self.writer
            .exec_tx(move |tx| -> Result<TaxYearReport> {
                let now = chrono::Utc::now().naive_utc();
                let report_db = TaxYearReportDB {
                    id: Uuid::new_v4().to_string(),
                    tax_year: report.tax_year,
                    jurisdiction,
                    status: TaxReportStatus::Draft.as_str().to_string(),
                    rule_pack_version,
                    base_currency,
                    generated_at: None,
                    finalized_at: None,
                    assumptions_json: "{}".to_string(),
                    summary_json: "{}".to_string(),
                    parent_report_id: None,
                    created_at: now,
                    updated_at: now,
                };

                diesel::insert_into(tax_year_reports::table)
                    .values(&report_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                Ok(TaxYearReport::from(report_db))
            })
            .await
    }
}
