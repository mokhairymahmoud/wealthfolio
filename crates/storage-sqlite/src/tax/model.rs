//! Database models for tax declaration assistance.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, ExtractedTaxField, NewExtractedTaxField,
    NewTaxReconciliationEntry, TaxConfidence, TaxDocument, TaxDocumentExtraction, TaxEvent,
    TaxEventSource, TaxEventType, TaxIssue, TaxLotAllocation, TaxProfile, TaxProfileUpdate,
    TaxReconciliationEntry, TaxReportStatus, TaxYearReport,
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

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxEventDB {
    pub id: String,
    pub report_id: String,
    pub event_type: String,
    pub category: String,
    pub suggested_box: Option<String>,
    pub account_id: String,
    pub asset_id: Option<String>,
    pub activity_id: Option<String>,
    pub event_date: String,
    pub amount_currency: String,
    pub amount_local: Option<String>,
    pub amount_eur: Option<String>,
    pub taxable_amount_eur: Option<String>,
    pub expenses_eur: Option<String>,
    pub confidence: String,
    pub included: i32,
    pub notes: Option<String>,
    pub user_override: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_event_sources)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxEventSourceDB {
    pub id: String,
    pub tax_event_id: String,
    pub source_type: String,
    pub source_id: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_lot_allocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxLotAllocationDB {
    pub id: String,
    pub tax_event_id: String,
    pub source_activity_id: String,
    pub quantity: String,
    pub acquisition_date: String,
    pub cost_basis_eur: String,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_issues)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxIssueDB {
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

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxDocumentDB {
    pub id: String,
    pub report_id: String,
    pub document_type: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub sha256: String,
    pub encrypted_content: String,
    pub encrypted_blob_path: Option<String>,
    pub encryption_key_ref: String,
    pub size_bytes: i32,
    pub uploaded_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_document_extractions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxDocumentExtractionDB {
    pub id: String,
    pub document_id: String,
    pub method: String,
    pub status: String,
    pub consent_granted: i32,
    pub raw_text_preview: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::extracted_tax_fields)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ExtractedTaxFieldDB {
    pub id: String,
    pub extraction_id: String,
    pub field_key: String,
    pub label: String,
    pub mapped_category: Option<String>,
    pub value_text: Option<String>,
    pub amount_eur: Option<String>,
    pub confidence: f64,
    pub status: String,
    pub confirmed_amount_eur: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable, Identifiable, Insertable, AsChangeset, Selectable, Debug, Clone)]
#[diesel(table_name = crate::schema::tax_reconciliation_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TaxReconciliationEntryDB {
    pub id: String,
    pub report_id: String,
    pub category: String,
    pub suggested_box: Option<String>,
    pub app_amount_eur: Option<String>,
    pub document_amount_eur: Option<String>,
    pub selected_amount_eur: Option<String>,
    pub delta_eur: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub fn decimal_to_db(value: Option<Decimal>) -> Option<String> {
    value.map(|amount| amount.normalize().to_string())
}

pub fn decimal_from_db(value: Option<String>) -> Option<Decimal> {
    value.and_then(|amount| amount.parse::<Decimal>().ok())
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

impl From<TaxEventDB> for TaxEvent {
    fn from(db: TaxEventDB) -> Self {
        Self {
            id: db.id,
            report_id: db.report_id,
            event_type: TaxEventType::try_from(db.event_type.as_str())
                .unwrap_or(TaxEventType::DividendReceived),
            category: db.category,
            suggested_box: db.suggested_box,
            account_id: db.account_id,
            asset_id: db.asset_id,
            activity_id: db.activity_id,
            event_date: db.event_date,
            amount_currency: db.amount_currency,
            amount_local: decimal_from_db(db.amount_local),
            amount_eur: decimal_from_db(db.amount_eur),
            taxable_amount_eur: decimal_from_db(db.taxable_amount_eur),
            expenses_eur: decimal_from_db(db.expenses_eur),
            confidence: TaxConfidence::try_from(db.confidence.as_str())
                .unwrap_or(TaxConfidence::Low),
            included: db.included != 0,
            notes: db.notes,
            user_override: db.user_override != 0,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<TaxEventSourceDB> for TaxEventSource {
    fn from(db: TaxEventSourceDB) -> Self {
        Self {
            id: db.id,
            tax_event_id: db.tax_event_id,
            source_type: db.source_type,
            source_id: db.source_id,
            description: db.description,
            created_at: db.created_at,
        }
    }
}

impl From<TaxLotAllocationDB> for TaxLotAllocation {
    fn from(db: TaxLotAllocationDB) -> Self {
        Self {
            id: db.id,
            tax_event_id: db.tax_event_id,
            source_activity_id: db.source_activity_id,
            quantity: db.quantity.parse::<Decimal>().unwrap_or(Decimal::ZERO),
            acquisition_date: db.acquisition_date,
            cost_basis_eur: db
                .cost_basis_eur
                .parse::<Decimal>()
                .unwrap_or(Decimal::ZERO),
            created_at: db.created_at,
        }
    }
}

impl From<TaxIssueDB> for TaxIssue {
    fn from(db: TaxIssueDB) -> Self {
        Self {
            id: db.id,
            report_id: db.report_id,
            severity: db.severity,
            code: db.code,
            message: db.message,
            account_id: db.account_id,
            activity_id: db.activity_id,
            tax_event_id: db.tax_event_id,
            resolved_at: db.resolved_at,
            created_at: db.created_at,
        }
    }
}

impl From<TaxDocumentDB> for TaxDocument {
    fn from(db: TaxDocumentDB) -> Self {
        Self {
            id: db.id,
            report_id: db.report_id,
            document_type: db.document_type,
            filename: db.filename,
            mime_type: db.mime_type,
            sha256: db.sha256,
            size_bytes: db.size_bytes,
            uploaded_at: db.uploaded_at,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<TaxDocumentExtractionDB> for TaxDocumentExtraction {
    fn from(db: TaxDocumentExtractionDB) -> Self {
        Self {
            id: db.id,
            document_id: db.document_id,
            method: db.method,
            status: db.status,
            consent_granted: db.consent_granted != 0,
            raw_text_preview: db.raw_text_preview,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<ExtractedTaxFieldDB> for ExtractedTaxField {
    fn from(db: ExtractedTaxFieldDB) -> Self {
        Self {
            id: db.id,
            extraction_id: db.extraction_id,
            field_key: db.field_key,
            label: db.label,
            mapped_category: db.mapped_category,
            value_text: db.value_text,
            amount_eur: decimal_from_db(db.amount_eur),
            confidence: db.confidence,
            status: db.status,
            confirmed_amount_eur: decimal_from_db(db.confirmed_amount_eur),
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl From<TaxReconciliationEntryDB> for TaxReconciliationEntry {
    fn from(db: TaxReconciliationEntryDB) -> Self {
        Self {
            id: db.id,
            report_id: db.report_id,
            category: db.category,
            suggested_box: db.suggested_box,
            app_amount_eur: decimal_from_db(db.app_amount_eur),
            document_amount_eur: decimal_from_db(db.document_amount_eur),
            selected_amount_eur: decimal_from_db(db.selected_amount_eur),
            delta_eur: decimal_from_db(db.delta_eur),
            status: db.status,
            notes: db.notes,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

impl ExtractedTaxFieldDB {
    pub fn new(extraction_id: String, field: NewExtractedTaxField) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            extraction_id,
            field_key: field.field_key,
            label: field.label,
            mapped_category: field.mapped_category,
            value_text: field.value_text,
            amount_eur: decimal_to_db(field.amount_eur),
            confidence: field.confidence,
            status: field.status,
            confirmed_amount_eur: decimal_to_db(field.confirmed_amount_eur),
            created_at: now,
            updated_at: now,
        }
    }
}

impl TaxReconciliationEntryDB {
    pub fn new(report_id: String, entry: NewTaxReconciliationEntry) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            report_id,
            category: entry.category,
            suggested_box: entry.suggested_box,
            app_amount_eur: decimal_to_db(entry.app_amount_eur),
            document_amount_eur: decimal_to_db(entry.document_amount_eur),
            selected_amount_eur: decimal_to_db(entry.selected_amount_eur),
            delta_eur: decimal_to_db(entry.delta_eur),
            status: entry.status,
            notes: entry.notes,
            created_at: now,
            updated_at: now,
        }
    }
}
