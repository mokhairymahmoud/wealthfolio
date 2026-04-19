//! Domain models for tax declaration assistance.

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxEventType {
    DividendReceived,
    InterestReceived,
    SecurityDisposal,
    FeePaid,
}

impl TaxEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaxEventType::DividendReceived => "DIVIDEND_RECEIVED",
            TaxEventType::InterestReceived => "INTEREST_RECEIVED",
            TaxEventType::SecurityDisposal => "SECURITY_DISPOSAL",
            TaxEventType::FeePaid => "FEE_PAID",
        }
    }
}

impl TryFrom<&str> for TaxEventType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DIVIDEND_RECEIVED" => Ok(TaxEventType::DividendReceived),
            "INTEREST_RECEIVED" => Ok(TaxEventType::InterestReceived),
            "SECURITY_DISPOSAL" => Ok(TaxEventType::SecurityDisposal),
            "FEE_PAID" => Ok(TaxEventType::FeePaid),
            _ => Err(format!("Unsupported tax event type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxConfidence {
    High,
    Medium,
    Low,
    Excluded,
}

impl TaxConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaxConfidence::High => "HIGH",
            TaxConfidence::Medium => "MEDIUM",
            TaxConfidence::Low => "LOW",
            TaxConfidence::Excluded => "EXCLUDED",
        }
    }
}

impl TryFrom<&str> for TaxConfidence {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "HIGH" => Ok(TaxConfidence::High),
            "MEDIUM" => Ok(TaxConfidence::Medium),
            "LOW" => Ok(TaxConfidence::Low),
            "EXCLUDED" => Ok(TaxConfidence::Excluded),
            _ => Err(format!("Unsupported tax confidence: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxEvent {
    pub id: String,
    pub report_id: String,
    pub event_type: TaxEventType,
    pub category: String,
    pub suggested_box: Option<String>,
    pub account_id: String,
    pub asset_id: Option<String>,
    pub activity_id: Option<String>,
    pub event_date: String,
    pub amount_currency: String,
    pub amount_local: Option<Decimal>,
    pub amount_eur: Option<Decimal>,
    pub taxable_amount_eur: Option<Decimal>,
    pub expenses_eur: Option<Decimal>,
    pub confidence: TaxConfidence,
    pub included: bool,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxEvent {
    pub event_type: TaxEventType,
    pub category: String,
    pub suggested_box: Option<String>,
    pub account_id: String,
    pub asset_id: Option<String>,
    pub activity_id: Option<String>,
    pub event_date: String,
    pub amount_currency: String,
    pub amount_local: Option<Decimal>,
    pub amount_eur: Option<Decimal>,
    pub taxable_amount_eur: Option<Decimal>,
    pub expenses_eur: Option<Decimal>,
    pub confidence: TaxConfidence,
    pub included: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxEventSource {
    pub id: String,
    pub tax_event_id: String,
    pub source_type: String,
    pub source_id: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxEventSource {
    pub source_type: String,
    pub source_id: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxLotAllocation {
    pub id: String,
    pub tax_event_id: String,
    pub source_activity_id: String,
    pub quantity: Decimal,
    pub acquisition_date: String,
    pub cost_basis_eur: Decimal,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxLotAllocation {
    pub source_activity_id: String,
    pub quantity: Decimal,
    pub acquisition_date: String,
    pub cost_basis_eur: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompiledTaxEvent {
    pub event: NewTaxEvent,
    pub sources: Vec<NewTaxEventSource>,
    pub lot_allocations: Vec<NewTaxLotAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxIssue {
    pub id: String,
    pub report_id: String,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub account_id: Option<String>,
    pub activity_id: Option<String>,
    pub tax_event_id: Option<String>,
    pub resolved_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxIssue {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub account_id: Option<String>,
    pub activity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxDocument {
    pub id: String,
    pub report_id: String,
    pub document_type: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub sha256: String,
    pub size_bytes: i32,
    pub uploaded_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxDocumentUpload {
    pub report_id: String,
    pub document_type: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxDocumentExtraction {
    pub id: String,
    pub document_id: String,
    pub method: String,
    pub status: String,
    pub consent_granted: bool,
    pub raw_text_preview: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxDocumentExtractionRequest {
    pub document_id: String,
    pub method: String,
    pub consent_granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedTaxField {
    pub id: String,
    pub extraction_id: String,
    pub field_key: String,
    pub label: String,
    pub mapped_category: Option<String>,
    pub value_text: Option<String>,
    pub amount_eur: Option<Decimal>,
    pub confidence: f64,
    pub status: String,
    pub confirmed_amount_eur: Option<Decimal>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewExtractedTaxField {
    pub field_key: String,
    pub label: String,
    pub mapped_category: Option<String>,
    pub value_text: Option<String>,
    pub amount_eur: Option<Decimal>,
    pub confidence: f64,
    pub status: String,
    pub confirmed_amount_eur: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxDocumentExtractionResult {
    pub extraction: TaxDocumentExtraction,
    pub fields: Vec<ExtractedTaxField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedTaxFieldUpdate {
    pub field_id: String,
    pub status: String,
    pub confirmed_amount_eur: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxReconciliationEntry {
    pub id: String,
    pub report_id: String,
    pub category: String,
    pub suggested_box: Option<String>,
    pub app_amount_eur: Option<Decimal>,
    pub document_amount_eur: Option<Decimal>,
    pub selected_amount_eur: Option<Decimal>,
    pub delta_eur: Option<Decimal>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewTaxReconciliationEntry {
    pub category: String,
    pub suggested_box: Option<String>,
    pub app_amount_eur: Option<Decimal>,
    pub document_amount_eur: Option<Decimal>,
    pub selected_amount_eur: Option<Decimal>,
    pub delta_eur: Option<Decimal>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxReconciliationEntryUpdate {
    pub id: String,
    pub selected_amount_eur: Option<Decimal>,
    pub status: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaxReportDetail {
    pub report: TaxYearReport,
    pub events: Vec<TaxEvent>,
    pub issues: Vec<TaxIssue>,
    pub documents: Vec<TaxDocument>,
    pub extractions: Vec<TaxDocumentExtractionResult>,
    pub reconciliation: Vec<TaxReconciliationEntry>,
}
