//! Database models for tax declaration assistance.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, TaxProfile, TaxProfileUpdate, TaxReportStatus,
    TaxYearReport,
};

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
#[diesel(table_name = crate::schema::tax_profiles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxProfileDB {
    pub id: String,
    pub jurisdiction: String,
    pub tax_residence_country: String,
    pub default_tax_regime: String,
    pub pfu_or_bareme_preference: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
#[diesel(primary_key(account_id))]
#[diesel(table_name = crate::schema::account_tax_profiles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AccountTaxProfileDB {
    pub account_id: String,
    pub jurisdiction: String,
    pub regime: String,
    pub opened_on: Option<String>,
    pub closed_on: Option<String>,
    pub metadata: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
    Selectable,
    PartialEq,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
#[diesel(table_name = crate::schema::tax_year_reports)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxYearReportDB {
    pub id: String,
    pub tax_year: i32,
    pub jurisdiction: String,
    pub status: String,
    pub rule_pack_version: String,
    pub base_currency: String,
    pub generated_at: Option<NaiveDateTime>,
    pub finalized_at: Option<NaiveDateTime>,
    pub assumptions_json: String,
    pub summary_json: String,
    pub parent_report_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<TaxProfileDB> for TaxProfile {
    fn from(db: TaxProfileDB) -> Self {
        Self {
            jurisdiction: db.jurisdiction,
            tax_residence_country: db.tax_residence_country,
            default_tax_regime: db.default_tax_regime,
            pfu_or_bareme_preference: db.pfu_or_bareme_preference,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<TaxProfileUpdate> for TaxProfileDB {
    fn from(domain: TaxProfileUpdate) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: "default".to_string(),
            jurisdiction: domain.jurisdiction,
            tax_residence_country: domain.tax_residence_country,
            default_tax_regime: domain.default_tax_regime,
            pfu_or_bareme_preference: domain.pfu_or_bareme_preference,
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<AccountTaxProfileDB> for AccountTaxProfile {
    fn from(db: AccountTaxProfileDB) -> Self {
        Self {
            account_id: db.account_id,
            jurisdiction: db.jurisdiction,
            regime: db.regime,
            opened_on: db.opened_on,
            closed_on: db.closed_on,
            metadata: db.metadata,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<AccountTaxProfileUpdate> for AccountTaxProfileDB {
    fn from(domain: AccountTaxProfileUpdate) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            account_id: domain.account_id,
            jurisdiction: domain.jurisdiction,
            regime: domain.regime,
            opened_on: domain.opened_on,
            closed_on: domain.closed_on,
            metadata: domain.metadata,
            created_at: now,
            updated_at: now,
        }
    }
}

impl From<TaxYearReportDB> for TaxYearReport {
    fn from(db: TaxYearReportDB) -> Self {
        Self {
            id: db.id,
            tax_year: db.tax_year,
            jurisdiction: db.jurisdiction,
            status: TaxReportStatus::try_from(db.status.as_str()).unwrap_or(TaxReportStatus::Draft),
            rule_pack_version: db.rule_pack_version,
            base_currency: db.base_currency,
            generated_at: db.generated_at,
            finalized_at: db.finalized_at,
            assumptions_json: db.assumptions_json,
            summary_json: db.summary_json,
            parent_report_id: db.parent_report_id,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}
