//! Domain models for tax declaration assistance.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TAX_JURISDICTION: &str = "FR";
pub const DEFAULT_TAX_REGIME: &str = "CTO";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxReportStatus {
    Draft,
    Finalized,
    AmendedDraft,
}

impl TaxReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaxReportStatus::Draft => "DRAFT",
            TaxReportStatus::Finalized => "FINALIZED",
            TaxReportStatus::AmendedDraft => "AMENDED_DRAFT",
        }
    }
}

impl TryFrom<&str> for TaxReportStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DRAFT" => Ok(TaxReportStatus::Draft),
            "FINALIZED" => Ok(TaxReportStatus::Finalized),
            "AMENDED_DRAFT" => Ok(TaxReportStatus::AmendedDraft),
            _ => Err(format!("Unsupported tax report status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxProfile {
    pub jurisdiction: String,
    pub tax_residence_country: String,
    pub default_tax_regime: String,
    pub pfu_or_bareme_preference: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxProfileUpdate {
    pub jurisdiction: String,
    pub tax_residence_country: String,
    pub default_tax_regime: String,
    pub pfu_or_bareme_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountTaxProfile {
    pub account_id: String,
    pub jurisdiction: String,
    pub regime: String,
    pub opened_on: Option<String>,
    pub closed_on: Option<String>,
    pub metadata: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountTaxProfileUpdate {
    pub account_id: String,
    pub jurisdiction: String,
    pub regime: String,
    pub opened_on: Option<String>,
    pub closed_on: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxYearReport {
    pub id: String,
    pub tax_year: i32,
    pub jurisdiction: String,
    pub status: TaxReportStatus,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxYearReport {
    pub tax_year: i32,
    pub jurisdiction: Option<String>,
    pub base_currency: Option<String>,
}
