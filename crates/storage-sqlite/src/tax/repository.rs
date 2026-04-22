use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use diesel::prelude::*;
use diesel::r2d2::{self, Pool};
use diesel::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use wealthfolio_device_sync::crypto::{decrypt, encrypt, sha256_checksum};

use wealthfolio_core::errors::DatabaseError;
use wealthfolio_core::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, CompiledTaxEvent, ExtractedTaxField,
    ExtractedTaxFieldUpdate, NewExtractedTaxField, NewTaxIssue, NewTaxReconciliationEntry,
    NewTaxYearReport, TaxDocument, TaxDocumentExtractionRequest, TaxDocumentExtractionResult,
    TaxEvent, TaxEventUpdate, TaxIssue, TaxProfile, TaxProfileUpdate, TaxReconciliationEntry,
    TaxReconciliationEntryUpdate, TaxReportDetail, TaxReportStatus, TaxRepositoryTrait,
    TaxYearReport,
};
use wealthfolio_core::{Error, Result};

use super::model::{
    decimal_to_db, AccountTaxProfileDB, ExtractedTaxFieldDB, TaxDocumentDB,
    TaxDocumentExtractionDB, TaxEventDB, TaxEventSourceDB, TaxIssueDB, TaxLotAllocationDB,
    TaxProfileDB, TaxReconciliationEntryDB, TaxYearReportDB,
};
use crate::db::{get_connection, WriteHandle};
use crate::errors::StorageError;
use crate::schema::{
    account_tax_profiles, extracted_tax_fields, tax_document_extractions, tax_documents,
    tax_event_sources, tax_events, tax_issues, tax_lot_allocations, tax_profiles,
    tax_reconciliation_entries, tax_year_reports,
};

const DEFAULT_PROFILE_ID: &str = "default";
const DOCUMENT_KEY_REF: &str = "tax_documents_v1";

pub struct TaxRepository {
    pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
    writer: WriteHandle,
    document_key: String,
}

impl TaxRepository {
    pub fn new(
        pool: Arc<Pool<r2d2::ConnectionManager<SqliteConnection>>>,
        writer: WriteHandle,
        document_key: String,
    ) -> Self {
        Self {
            pool,
            writer,
            document_key,
        }
    }

    fn load_report_detail(
        conn: &mut SqliteConnection,
        report: TaxYearReport,
    ) -> Result<TaxReportDetail> {
        let events = tax_events::table
            .select(TaxEventDB::as_select())
            .filter(tax_events::report_id.eq(&report.id))
            .order(tax_events::event_date.asc())
            .load::<TaxEventDB>(conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(TaxEvent::from)
            .collect();

        let issues = tax_issues::table
            .select(TaxIssueDB::as_select())
            .filter(tax_issues::report_id.eq(&report.id))
            .order(tax_issues::created_at.asc())
            .load::<TaxIssueDB>(conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(TaxIssue::from)
            .collect();

        let documents = tax_documents::table
            .select(TaxDocumentDB::as_select())
            .filter(tax_documents::report_id.eq(&report.id))
            .order(tax_documents::uploaded_at.desc())
            .load::<TaxDocumentDB>(conn)
            .map_err(StorageError::from)?;

        let extractions = Self::load_extractions_for_documents(
            conn,
            &documents
                .iter()
                .map(|document| document.id.clone())
                .collect::<Vec<_>>(),
        )?;

        let reconciliation = tax_reconciliation_entries::table
            .select(TaxReconciliationEntryDB::as_select())
            .filter(tax_reconciliation_entries::report_id.eq(&report.id))
            .order(tax_reconciliation_entries::category.asc())
            .load::<TaxReconciliationEntryDB>(conn)
            .map_err(StorageError::from)?
            .into_iter()
            .map(TaxReconciliationEntry::from)
            .collect();

        Ok(TaxReportDetail {
            report,
            events,
            issues,
            documents: documents.into_iter().map(TaxDocument::from).collect(),
            extractions,
            reconciliation,
        })
    }

    fn load_extractions_for_documents(
        conn: &mut SqliteConnection,
        document_ids: &[String],
    ) -> Result<Vec<TaxDocumentExtractionResult>> {
        if document_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = tax_document_extractions::table
            .select(TaxDocumentExtractionDB::as_select())
            .filter(tax_document_extractions::document_id.eq_any(document_ids))
            .order(tax_document_extractions::created_at.desc())
            .load::<TaxDocumentExtractionDB>(conn)
            .map_err(StorageError::from)?;

        let extraction_ids = rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<String>>();
        let field_rows = if extraction_ids.is_empty() {
            Vec::new()
        } else {
            extracted_tax_fields::table
                .select(ExtractedTaxFieldDB::as_select())
                .filter(extracted_tax_fields::extraction_id.eq_any(&extraction_ids))
                .order(extracted_tax_fields::field_key.asc())
                .load::<ExtractedTaxFieldDB>(conn)
                .map_err(StorageError::from)?
        };

        let mut fields_by_extraction: std::collections::HashMap<String, Vec<ExtractedTaxField>> =
            std::collections::HashMap::new();
        for field in field_rows {
            fields_by_extraction
                .entry(field.extraction_id.clone())
                .or_default()
                .push(ExtractedTaxField::from(field));
        }

        Ok(rows
            .into_iter()
            .map(|row| {
                let extraction_id = row.id.clone();
                TaxDocumentExtractionResult {
                    extraction: row.into(),
                    fields: fields_by_extraction
                        .remove(&extraction_id)
                        .unwrap_or_default(),
                }
            })
            .collect())
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

    async fn create_amended_report(&self, parent: TaxYearReport) -> Result<TaxYearReport> {
        self.writer
            .exec_tx(move |tx| -> Result<TaxYearReport> {
                let now = chrono::Utc::now().naive_utc();
                let report_db = TaxYearReportDB {
                    id: Uuid::new_v4().to_string(),
                    tax_year: parent.tax_year,
                    jurisdiction: parent.jurisdiction,
                    status: TaxReportStatus::AmendedDraft.as_str().to_string(),
                    rule_pack_version: parent.rule_pack_version,
                    base_currency: parent.base_currency,
                    generated_at: None,
                    finalized_at: None,
                    assumptions_json: "{}".to_string(),
                    summary_json: "{}".to_string(),
                    parent_report_id: Some(parent.id),
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

    async fn replace_generated_report_data(
        &self,
        report_id: &str,
        summary_json: String,
        events: Vec<CompiledTaxEvent>,
        issues: Vec<NewTaxIssue>,
        reconciliation: Vec<NewTaxReconciliationEntry>,
    ) -> Result<TaxReportDetail> {
        let report_id = report_id.to_string();
        self.writer
            .exec_tx(move |tx| -> Result<TaxReportDetail> {
                let overrides: HashMap<(String, String), (i32, Option<String>, Option<String>)> =
                    tax_events::table
                        .select(TaxEventDB::as_select())
                        .filter(tax_events::report_id.eq(&report_id))
                        .filter(tax_events::user_override.eq(1))
                        .load::<TaxEventDB>(tx.conn())
                        .map_err(StorageError::from)?
                        .into_iter()
                        .filter_map(|e| {
                            e.activity_id.map(|aid| {
                                (
                                    (aid, e.event_type),
                                    (e.included, e.taxable_amount_eur, e.notes),
                                )
                            })
                        })
                        .collect();

                diesel::delete(
                    tax_reconciliation_entries::table
                        .filter(tax_reconciliation_entries::report_id.eq(&report_id)),
                )
                .execute(tx.conn())
                .map_err(StorageError::from)?;
                diesel::delete(tax_issues::table.filter(tax_issues::report_id.eq(&report_id)))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                diesel::delete(tax_events::table.filter(tax_events::report_id.eq(&report_id)))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let now = chrono::Utc::now().naive_utc();
                for compiled in events {
                    let event_id = Uuid::new_v4().to_string();
                    let event_type_str = compiled.event.event_type.as_str().to_string();
                    let event_db = TaxEventDB {
                        id: event_id.clone(),
                        report_id: report_id.clone(),
                        event_type: event_type_str.clone(),
                        category: compiled.event.category,
                        suggested_box: compiled.event.suggested_box,
                        account_id: compiled.event.account_id,
                        asset_id: compiled.event.asset_id,
                        activity_id: compiled.event.activity_id,
                        event_date: compiled.event.event_date,
                        amount_currency: compiled.event.amount_currency,
                        amount_local: decimal_to_db(compiled.event.amount_local),
                        amount_eur: decimal_to_db(compiled.event.amount_eur),
                        taxable_amount_eur: decimal_to_db(compiled.event.taxable_amount_eur),
                        expenses_eur: decimal_to_db(compiled.event.expenses_eur),
                        confidence: compiled.event.confidence.as_str().to_string(),
                        included: if compiled.event.included { 1 } else { 0 },
                        notes: compiled.event.notes,
                        user_override: 0,
                        created_at: now,
                        updated_at: now,
                    };
                    diesel::insert_into(tax_events::table)
                        .values(&event_db)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;

                    if let Some(ref activity_id) = event_db.activity_id {
                        if let Some((incl, tax_amt, notes)) =
                            overrides.get(&(activity_id.clone(), event_type_str))
                        {
                            diesel::update(tax_events::table.find(&event_id))
                                .set((
                                    tax_events::included.eq(incl),
                                    tax_events::taxable_amount_eur.eq(tax_amt),
                                    tax_events::notes.eq(notes),
                                    tax_events::user_override.eq(1),
                                    tax_events::updated_at.eq(now),
                                ))
                                .execute(tx.conn())
                                .map_err(StorageError::from)?;
                        }
                    }

                    let source_rows = compiled
                        .sources
                        .into_iter()
                        .map(|source| TaxEventSourceDB {
                            id: Uuid::new_v4().to_string(),
                            tax_event_id: event_id.clone(),
                            source_type: source.source_type,
                            source_id: source.source_id,
                            description: source.description,
                            created_at: now,
                        })
                        .collect::<Vec<_>>();
                    if !source_rows.is_empty() {
                        diesel::insert_into(tax_event_sources::table)
                            .values(&source_rows)
                            .execute(tx.conn())
                            .map_err(StorageError::from)?;
                    }

                    let lot_rows = compiled
                        .lot_allocations
                        .into_iter()
                        .map(|lot| TaxLotAllocationDB {
                            id: Uuid::new_v4().to_string(),
                            tax_event_id: event_id.clone(),
                            source_activity_id: lot.source_activity_id,
                            quantity: lot.quantity.normalize().to_string(),
                            acquisition_date: lot.acquisition_date,
                            cost_basis_eur: lot.cost_basis_eur.normalize().to_string(),
                            created_at: now,
                        })
                        .collect::<Vec<_>>();
                    if !lot_rows.is_empty() {
                        diesel::insert_into(tax_lot_allocations::table)
                            .values(&lot_rows)
                            .execute(tx.conn())
                            .map_err(StorageError::from)?;
                    }
                }

                for issue in issues {
                    let issue_db = TaxIssueDB {
                        id: Uuid::new_v4().to_string(),
                        report_id: report_id.clone(),
                        severity: issue.severity,
                        code: issue.code,
                        message: issue.message,
                        account_id: issue.account_id,
                        activity_id: issue.activity_id,
                        tax_event_id: None,
                        resolved_at: None,
                        created_at: now,
                    };
                    diesel::insert_into(tax_issues::table)
                        .values(&issue_db)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;
                }

                let reconciliation_rows = reconciliation
                    .into_iter()
                    .map(|entry| TaxReconciliationEntryDB::new(report_id.clone(), entry))
                    .collect::<Vec<_>>();
                if !reconciliation_rows.is_empty() {
                    diesel::insert_into(tax_reconciliation_entries::table)
                        .values(&reconciliation_rows)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;
                }

                diesel::update(tax_year_reports::table.find(&report_id))
                    .set((
                        tax_year_reports::generated_at.eq(Some(now)),
                        tax_year_reports::summary_json.eq(summary_json),
                        tax_year_reports::updated_at.eq(now),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let report = tax_year_reports::table
                    .select(TaxYearReportDB::as_select())
                    .find(&report_id)
                    .first::<TaxYearReportDB>(tx.conn())
                    .map_err(StorageError::from)?;
                Self::load_report_detail(tx.conn(), TaxYearReport::from(report))
            })
            .await
    }

    async fn finalize_tax_year_report(&self, report_id: &str) -> Result<TaxYearReport> {
        let report_id = report_id.to_string();
        self.writer
            .exec_tx(move |tx| -> Result<TaxYearReport> {
                let now = chrono::Utc::now().naive_utc();
                diesel::update(tax_year_reports::table.find(&report_id))
                    .set((
                        tax_year_reports::status.eq(TaxReportStatus::Finalized.as_str()),
                        tax_year_reports::finalized_at.eq(Some(now)),
                        tax_year_reports::updated_at.eq(now),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let report = tax_year_reports::table
                    .select(TaxYearReportDB::as_select())
                    .find(&report_id)
                    .first::<TaxYearReportDB>(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(TaxYearReport::from(report))
            })
            .await
    }

    fn get_tax_report_detail(&self, report_id: &str) -> Result<Option<TaxReportDetail>> {
        let mut conn = get_connection(&self.pool)?;
        let report = tax_year_reports::table
            .select(TaxYearReportDB::as_select())
            .find(report_id)
            .first::<TaxYearReportDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;

        report
            .map(|report| Self::load_report_detail(&mut conn, TaxYearReport::from(report)))
            .transpose()
    }

    async fn upload_tax_document(
        &self,
        report_id: String,
        document_type: String,
        filename: String,
        mime_type: Option<String>,
        content: Vec<u8>,
    ) -> Result<TaxDocument> {
        let document_key = self.document_key.clone();
        self.writer
            .exec_tx(move |tx| -> Result<TaxDocument> {
                let now = chrono::Utc::now().naive_utc();
                let plaintext = BASE64.encode(&content);
                let encrypted_content =
                    encrypt(&document_key, &plaintext).map_err(Error::Secret)?;
                let document_db = TaxDocumentDB {
                    id: Uuid::new_v4().to_string(),
                    report_id,
                    document_type,
                    filename,
                    mime_type,
                    sha256: sha256_checksum(&content),
                    encrypted_content,
                    encryption_key_ref: DOCUMENT_KEY_REF.to_string(),
                    size_bytes: content.len() as i32,
                    uploaded_at: now,
                    created_at: now,
                    updated_at: now,
                };
                diesel::insert_into(tax_documents::table)
                    .values(&document_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(TaxDocument::from(document_db))
            })
            .await
    }

    fn list_tax_documents(&self, report_id: &str) -> Result<Vec<TaxDocument>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = tax_documents::table
            .select(TaxDocumentDB::as_select())
            .filter(tax_documents::report_id.eq(report_id))
            .order(tax_documents::uploaded_at.desc())
            .load::<TaxDocumentDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(TaxDocument::from).collect())
    }

    fn get_tax_document(&self, document_id: &str) -> Result<Option<TaxDocument>> {
        let mut conn = get_connection(&self.pool)?;
        let row = tax_documents::table
            .select(TaxDocumentDB::as_select())
            .find(document_id)
            .first::<TaxDocumentDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;
        Ok(row.map(TaxDocument::from))
    }

    fn get_tax_document_content(&self, document_id: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = get_connection(&self.pool)?;
        let row = tax_documents::table
            .select(TaxDocumentDB::as_select())
            .find(document_id)
            .first::<TaxDocumentDB>(&mut conn)
            .optional()
            .map_err(StorageError::from)?;

        row.map(|document| {
            let plaintext =
                decrypt(&self.document_key, &document.encrypted_content).map_err(Error::Secret)?;
            BASE64
                .decode(plaintext)
                .map_err(|error| Error::Unexpected(error.to_string()))
        })
        .transpose()
    }

    async fn delete_tax_document(&self, document_id: &str) -> Result<()> {
        let document_id = document_id.to_string();
        self.writer
            .exec_tx(move |tx| -> Result<()> {
                let affected = diesel::delete(tax_documents::table.find(&document_id))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;
                if affected == 0 {
                    return Err(Error::Database(DatabaseError::NotFound(format!(
                        "Tax document {document_id} not found"
                    ))));
                }
                Ok(())
            })
            .await
    }

    async fn create_tax_document_extraction(
        &self,
        request: TaxDocumentExtractionRequest,
        raw_text_preview: Option<String>,
        fields: Vec<NewExtractedTaxField>,
    ) -> Result<TaxDocumentExtractionResult> {
        self.writer
            .exec_tx(move |tx| -> Result<TaxDocumentExtractionResult> {
                let now = chrono::Utc::now().naive_utc();
                let extraction_db = TaxDocumentExtractionDB {
                    id: Uuid::new_v4().to_string(),
                    document_id: request.document_id,
                    method: request.method,
                    status: "READY_FOR_REVIEW".to_string(),
                    consent_granted: if request.consent_granted { 1 } else { 0 },
                    raw_text_preview,
                    created_at: now,
                    updated_at: now,
                };

                diesel::insert_into(tax_document_extractions::table)
                    .values(&extraction_db)
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let field_rows = fields
                    .into_iter()
                    .map(|field| ExtractedTaxFieldDB::new(extraction_db.id.clone(), field))
                    .collect::<Vec<_>>();
                if !field_rows.is_empty() {
                    diesel::insert_into(extracted_tax_fields::table)
                        .values(&field_rows)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;
                }

                Ok(TaxDocumentExtractionResult {
                    extraction: extraction_db.into(),
                    fields: field_rows
                        .into_iter()
                        .map(ExtractedTaxField::from)
                        .collect(),
                })
            })
            .await
    }

    fn list_tax_document_extractions(
        &self,
        report_id: &str,
    ) -> Result<Vec<TaxDocumentExtractionResult>> {
        let mut conn = get_connection(&self.pool)?;
        let documents = tax_documents::table
            .select(TaxDocumentDB::as_select())
            .filter(tax_documents::report_id.eq(report_id))
            .load::<TaxDocumentDB>(&mut conn)
            .map_err(StorageError::from)?;
        let document_ids = documents.into_iter().map(|row| row.id).collect::<Vec<_>>();
        Self::load_extractions_for_documents(&mut conn, &document_ids)
    }

    async fn update_extracted_tax_field(
        &self,
        update: ExtractedTaxFieldUpdate,
    ) -> Result<ExtractedTaxField> {
        self.writer
            .exec_tx(move |tx| -> Result<ExtractedTaxField> {
                let now = chrono::Utc::now().naive_utc();
                diesel::update(extracted_tax_fields::table.find(&update.field_id))
                    .set((
                        extracted_tax_fields::status.eq(update.status),
                        extracted_tax_fields::confirmed_amount_eur
                            .eq(decimal_to_db(update.confirmed_amount_eur)),
                        extracted_tax_fields::updated_at.eq(now),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let field = extracted_tax_fields::table
                    .select(ExtractedTaxFieldDB::as_select())
                    .find(&update.field_id)
                    .first::<ExtractedTaxFieldDB>(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(ExtractedTaxField::from(field))
            })
            .await
    }

    async fn replace_reconciliation_entries(
        &self,
        report_id: &str,
        entries: Vec<NewTaxReconciliationEntry>,
    ) -> Result<Vec<TaxReconciliationEntry>> {
        let report_id = report_id.to_string();
        self.writer
            .exec_tx(move |tx| -> Result<Vec<TaxReconciliationEntry>> {
                diesel::delete(
                    tax_reconciliation_entries::table
                        .filter(tax_reconciliation_entries::report_id.eq(&report_id)),
                )
                .execute(tx.conn())
                .map_err(StorageError::from)?;

                let rows = entries
                    .into_iter()
                    .map(|entry| TaxReconciliationEntryDB::new(report_id.clone(), entry))
                    .collect::<Vec<_>>();
                if !rows.is_empty() {
                    diesel::insert_into(tax_reconciliation_entries::table)
                        .values(&rows)
                        .execute(tx.conn())
                        .map_err(StorageError::from)?;
                }
                Ok(rows.into_iter().map(TaxReconciliationEntry::from).collect())
            })
            .await
    }

    async fn update_tax_reconciliation_entry(
        &self,
        update: TaxReconciliationEntryUpdate,
    ) -> Result<TaxReconciliationEntry> {
        self.writer
            .exec_tx(move |tx| -> Result<TaxReconciliationEntry> {
                let now = chrono::Utc::now().naive_utc();
                diesel::update(tax_reconciliation_entries::table.find(&update.id))
                    .set((
                        tax_reconciliation_entries::selected_amount_eur
                            .eq(decimal_to_db(update.selected_amount_eur)),
                        tax_reconciliation_entries::status.eq(update.status),
                        tax_reconciliation_entries::notes.eq(update.notes),
                        tax_reconciliation_entries::updated_at.eq(now),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let entry = tax_reconciliation_entries::table
                    .select(TaxReconciliationEntryDB::as_select())
                    .find(&update.id)
                    .first::<TaxReconciliationEntryDB>(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(TaxReconciliationEntry::from(entry))
            })
            .await
    }

    async fn update_tax_event(&self, update: TaxEventUpdate) -> Result<TaxEvent> {
        self.writer
            .exec_tx(move |tx| -> Result<TaxEvent> {
                let now = chrono::Utc::now().naive_utc();
                diesel::update(tax_events::table.find(&update.id))
                    .set((
                        tax_events::included.eq(if update.included { 1 } else { 0 }),
                        tax_events::taxable_amount_eur.eq(decimal_to_db(update.taxable_amount_eur)),
                        tax_events::notes.eq(update.notes),
                        tax_events::user_override.eq(1),
                        tax_events::updated_at.eq(now),
                    ))
                    .execute(tx.conn())
                    .map_err(StorageError::from)?;

                let event = tax_events::table
                    .select(TaxEventDB::as_select())
                    .find(&update.id)
                    .first::<TaxEventDB>(tx.conn())
                    .map_err(StorageError::from)?;
                Ok(TaxEvent::from(event))
            })
            .await
    }

    fn list_tax_events(&self, report_id: &str) -> Result<Vec<TaxEvent>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = tax_events::table
            .select(TaxEventDB::as_select())
            .filter(tax_events::report_id.eq(report_id))
            .order(tax_events::event_date.asc())
            .load::<TaxEventDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(TaxEvent::from).collect())
    }

    fn list_tax_issues(&self, report_id: &str) -> Result<Vec<TaxIssue>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = tax_issues::table
            .select(TaxIssueDB::as_select())
            .filter(tax_issues::report_id.eq(report_id))
            .order(tax_issues::created_at.asc())
            .load::<TaxIssueDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(TaxIssue::from).collect())
    }

    fn list_tax_reconciliation_entries(
        &self,
        report_id: &str,
    ) -> Result<Vec<TaxReconciliationEntry>> {
        let mut conn = get_connection(&self.pool)?;
        let rows = tax_reconciliation_entries::table
            .select(TaxReconciliationEntryDB::as_select())
            .filter(tax_reconciliation_entries::report_id.eq(report_id))
            .order(tax_reconciliation_entries::category.asc())
            .load::<TaxReconciliationEntryDB>(&mut conn)
            .map_err(StorageError::from)?;
        Ok(rows.into_iter().map(TaxReconciliationEntry::from).collect())
    }
}
