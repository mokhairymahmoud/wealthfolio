use async_trait::async_trait;

use crate::errors::Result;
use crate::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, CompiledTaxEvent, ExtractedTaxField,
    ExtractedTaxFieldUpdate, NewTaxIssue, NewTaxReconciliationEntry, NewTaxYearReport, TaxDocument,
    TaxDocumentDownload, TaxDocumentExtraction, TaxDocumentExtractionRequest,
    TaxDocumentExtractionResult, TaxEvent, TaxEventUpdate, TaxIssue, TaxProfile, TaxProfileUpdate,
    TaxReconciliationEntry, TaxReconciliationEntryUpdate, TaxReportDetail, TaxYearReport,
};

/// A single transaction sent to the LLM for frais réels classification.
#[derive(Debug, serde::Serialize)]
pub struct ActivitySummary {
    pub id: String,
    pub date: String,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub notes: String,
}

/// One transaction that the LLM identified as a professional expense candidate.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraisReelsClassification {
    pub id: String,
    pub category: String,
    pub confidence: f64,
    pub rationale: String,
}

#[async_trait]
pub trait TaxCloudExtractionTrait: Send + Sync {
    async fn extract_tax_fields(
        &self,
        document: &TaxDocument,
        content: &[u8],
        local_text_preview: &str,
    ) -> Result<Vec<crate::tax::NewExtractedTaxField>>;

    /// Classify a batch of WITHDRAWAL transactions and return only those that
    /// are likely professional expense candidates eligible as frais réels.
    async fn classify_frais_reels(
        &self,
        activities: &[ActivitySummary],
    ) -> Result<Vec<FraisReelsClassification>>;
}

#[async_trait]
pub trait TaxRepositoryTrait: Send + Sync {
    fn get_tax_profile(&self) -> Result<Option<TaxProfile>>;
    async fn upsert_tax_profile(
        &self,
        profile: TaxProfileUpdate,
        nombre_parts: f64,
    ) -> Result<TaxProfile>;

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
    async fn create_amended_report(&self, parent: TaxYearReport) -> Result<TaxYearReport>;
    async fn replace_generated_report_data(
        &self,
        report_id: &str,
        summary_json: String,
        events: Vec<CompiledTaxEvent>,
        issues: Vec<NewTaxIssue>,
        reconciliation: Vec<NewTaxReconciliationEntry>,
    ) -> Result<TaxReportDetail>;
    async fn finalize_tax_year_report(&self, report_id: &str) -> Result<TaxYearReport>;
    fn get_tax_report_detail(&self, report_id: &str) -> Result<Option<TaxReportDetail>>;

    async fn upload_tax_document(
        &self,
        report_id: String,
        document_type: String,
        filename: String,
        mime_type: Option<String>,
        content: Vec<u8>,
    ) -> Result<TaxDocument>;
    fn list_tax_documents(&self, report_id: &str) -> Result<Vec<TaxDocument>>;
    fn get_tax_document(&self, document_id: &str) -> Result<Option<TaxDocument>>;
    fn get_tax_document_content(&self, document_id: &str) -> Result<Option<Vec<u8>>>;
    fn get_tax_document_extraction(
        &self,
        extraction_id: &str,
    ) -> Result<Option<TaxDocumentExtraction>>;
    fn get_extracted_tax_field(&self, field_id: &str) -> Result<Option<ExtractedTaxField>>;
    async fn delete_tax_document(&self, document_id: &str) -> Result<()>;
    async fn create_tax_document_extraction(
        &self,
        request: TaxDocumentExtractionRequest,
        status: String,
        raw_text_preview: Option<String>,
        fields: Vec<crate::tax::NewExtractedTaxField>,
    ) -> Result<TaxDocumentExtractionResult>;
    async fn replace_tax_issues_by_code(
        &self,
        report_id: &str,
        issue_codes: Vec<String>,
        issues: Vec<NewTaxIssue>,
    ) -> Result<Vec<TaxIssue>>;
    fn list_tax_document_extractions(
        &self,
        report_id: &str,
    ) -> Result<Vec<TaxDocumentExtractionResult>>;
    async fn update_extracted_tax_field(
        &self,
        update: ExtractedTaxFieldUpdate,
    ) -> Result<ExtractedTaxField>;
    async fn replace_reconciliation_entries(
        &self,
        report_id: &str,
        entries: Vec<NewTaxReconciliationEntry>,
    ) -> Result<Vec<TaxReconciliationEntry>>;
    async fn update_tax_reconciliation_entry(
        &self,
        update: TaxReconciliationEntryUpdate,
    ) -> Result<TaxReconciliationEntry>;

    fn get_tax_event(&self, event_id: &str) -> Result<Option<TaxEvent>>;
    async fn update_tax_event(&self, update: TaxEventUpdate) -> Result<TaxEvent>;

    fn list_tax_events(&self, report_id: &str) -> Result<Vec<TaxEvent>>;
    fn list_tax_issues(&self, report_id: &str) -> Result<Vec<TaxIssue>>;
    fn get_tax_reconciliation_entry(
        &self,
        entry_id: &str,
    ) -> Result<Option<TaxReconciliationEntry>>;
    fn list_tax_reconciliation_entries(
        &self,
        report_id: &str,
    ) -> Result<Vec<TaxReconciliationEntry>>;
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
    fn get_tax_report_detail(&self, id: &str) -> Result<Option<TaxReportDetail>>;
    async fn regenerate_tax_year_report(&self, id: &str) -> Result<TaxReportDetail>;
    async fn finalize_tax_year_report(&self, id: &str) -> Result<TaxYearReport>;
    async fn amend_tax_year_report(&self, id: &str) -> Result<TaxYearReport>;

    async fn upload_tax_document(
        &self,
        upload: crate::tax::TaxDocumentUpload,
    ) -> Result<TaxDocument>;
    fn list_tax_documents(&self, report_id: &str) -> Result<Vec<TaxDocument>>;
    async fn delete_tax_document(&self, document_id: &str) -> Result<()>;
    fn get_tax_document_download(&self, document_id: &str) -> Result<Option<TaxDocumentDownload>>;
    async fn extract_tax_document(
        &self,
        request: TaxDocumentExtractionRequest,
    ) -> Result<TaxDocumentExtractionResult>;
    async fn update_extracted_tax_field(
        &self,
        update: ExtractedTaxFieldUpdate,
    ) -> Result<ExtractedTaxField>;
    async fn reconcile_tax_year_report(&self, id: &str) -> Result<Vec<TaxReconciliationEntry>>;
    async fn update_tax_reconciliation_entry(
        &self,
        update: TaxReconciliationEntryUpdate,
    ) -> Result<TaxReconciliationEntry>;
    async fn update_tax_event(&self, update: TaxEventUpdate) -> Result<TaxEvent>;
}
