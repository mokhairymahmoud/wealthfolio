use async_trait::async_trait;
use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::activities::{
    Activity, ActivityServiceTrait, ACTIVITY_TYPE_BUY, ACTIVITY_TYPE_DIVIDEND, ACTIVITY_TYPE_FEE,
    ACTIVITY_TYPE_INTEREST, ACTIVITY_TYPE_SELL, ACTIVITY_TYPE_TAX,
};
use crate::errors::{Error, Result, ValidationError};
use crate::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, CompiledTaxEvent, ExtractedTaxField,
    ExtractedTaxFieldUpdate, NewExtractedTaxField, NewTaxEvent, NewTaxEventSource, NewTaxIssue,
    NewTaxLotAllocation, NewTaxReconciliationEntry, NewTaxYearReport, TaxCloudExtractionTrait,
    TaxConfidence, TaxDocument, TaxDocumentDownload, TaxDocumentExtractionRequest,
    TaxDocumentExtractionResult, TaxDocumentUpload, TaxEvent, TaxEventType, TaxEventUpdate,
    TaxParameters, TaxProfile, TaxProfileUpdate, TaxReconciliationEntry,
    TaxReconciliationEntryUpdate, TaxReportDetail, TaxReportStatus, TaxRepositoryTrait,
    TaxServiceTrait, TaxYearReport, DEFAULT_TAX_JURISDICTION, DEFAULT_TAX_REGIME,
};

const TAX_REGIME_CTO: &str = "CTO";
const CATEGORY_DIVIDENDS: &str = "DIVIDENDS";
const CATEGORY_INTEREST: &str = "INTEREST";
const CATEGORY_SECURITY_GAINS: &str = "SECURITY_GAINS";
const CATEGORY_FEES: &str = "FEES";
const CATEGORY_FOREIGN_WITHHOLDING_TAX: &str = "FOREIGN_WITHHOLDING_TAX";
const CATEGORY_SALARY_INCOME: &str = "SALARY_INCOME";
const DOCUMENT_TYPE_FICHE_DE_PAIE: &str = "FICHE_DE_PAIE";
/// Sentinel account_id for salary events that originate from fiche de paie documents
/// (no investment account association).
const SALARY_VIRTUAL_ACCOUNT_ID: &str = "__SALARY__";
const EXTRACTION_CONFIDENCE_WARNING_THRESHOLD: f64 = 0.7;

#[derive(Debug, Clone)]
struct TaxLot {
    activity_id: String,
    quantity_remaining: Decimal,
    acquisition_date: String,
    unit_cost_eur: Decimal,
}

#[derive(Default)]
struct CompileOutput {
    events: Vec<CompiledTaxEvent>,
    issues: Vec<NewTaxIssue>,
}

pub struct TaxService<T: TaxRepositoryTrait> {
    repository: Arc<T>,
    activity_service: Arc<dyn ActivityServiceTrait>,
    cloud_extractor: Option<Arc<dyn TaxCloudExtractionTrait>>,
}

impl<T: TaxRepositoryTrait> TaxService<T> {
    pub fn new(repository: Arc<T>, activity_service: Arc<dyn ActivityServiceTrait>) -> Self {
        Self {
            repository,
            activity_service,
            cloud_extractor: None,
        }
    }

    pub fn with_cloud_extractor(
        mut self,
        cloud_extractor: Arc<dyn TaxCloudExtractionTrait>,
    ) -> Self {
        self.cloud_extractor = Some(cloud_extractor);
        self
    }

    fn rule_pack_version(tax_year: i32, jurisdiction: &str) -> String {
        let params = TaxParameters::for_year_or_latest(tax_year);
        if params.jurisdiction.eq_ignore_ascii_case(jurisdiction) {
            params.version
        } else {
            format!("{jurisdiction}-{tax_year}-securities-v1")
        }
    }

    fn not_found(message: impl Into<String>) -> Error {
        Error::Validation(ValidationError::InvalidInput(message.into()))
    }

    fn invalid_input(message: impl Into<String>) -> Error {
        Error::Validation(ValidationError::InvalidInput(message.into()))
    }

    fn ensure_report_editable(report: &TaxYearReport) -> Result<()> {
        if report.status == TaxReportStatus::Finalized {
            return Err(Self::invalid_input(format!(
                "Tax report {} is finalized and read-only",
                report.id
            )));
        }

        Ok(())
    }

    fn editable_report_by_id(&self, report_id: &str) -> Result<TaxYearReport> {
        let report = self
            .repository
            .get_tax_year_report(report_id)?
            .ok_or_else(|| Self::not_found(format!("Tax report {report_id} not found")))?;
        Self::ensure_report_editable(&report)?;
        Ok(report)
    }

    fn editable_report_for_document(&self, document_id: &str) -> Result<TaxYearReport> {
        let document = self
            .repository
            .get_tax_document(document_id)?
            .ok_or_else(|| Self::not_found(format!("Tax document {document_id} not found")))?;
        self.editable_report_by_id(&document.report_id)
    }

    fn editable_report_for_extracted_field(&self, field_id: &str) -> Result<TaxYearReport> {
        let field = self
            .repository
            .get_extracted_tax_field(field_id)?
            .ok_or_else(|| Self::not_found(format!("Extracted tax field {field_id} not found")))?;
        let extraction = self
            .repository
            .get_tax_document_extraction(&field.extraction_id)?
            .ok_or_else(|| {
                Self::not_found(format!(
                    "Tax document extraction {} not found",
                    field.extraction_id
                ))
            })?;
        self.editable_report_for_document(&extraction.document_id)
    }

    fn editable_report_for_reconciliation_entry(&self, entry_id: &str) -> Result<TaxYearReport> {
        let entry = self
            .repository
            .get_tax_reconciliation_entry(entry_id)?
            .ok_or_else(|| {
                Self::not_found(format!("Tax reconciliation entry {entry_id} not found"))
            })?;
        self.editable_report_by_id(&entry.report_id)
    }

    fn editable_report_for_tax_event(&self, event_id: &str) -> Result<TaxYearReport> {
        let event = self
            .repository
            .get_tax_event(event_id)?
            .ok_or_else(|| Self::not_found(format!("Tax event {event_id} not found")))?;
        self.editable_report_by_id(&event.report_id)
    }

    fn amount_eur(activity: &Activity, amount: Decimal) -> Option<Decimal> {
        if activity.currency.eq_ignore_ascii_case("EUR") {
            return Some(amount);
        }
        activity.fx_rate.map(|rate| amount * rate)
    }

    fn activity_amount(activity: &Activity) -> Decimal {
        if activity.amount.is_some() {
            activity.amt()
        } else if activity.quantity.is_some() && activity.unit_price.is_some() {
            activity.qty() * activity.price()
        } else {
            activity.price()
        }
    }

    fn event_year(activity: &Activity) -> i32 {
        activity.activity_date.naive_utc().date().year()
    }

    fn normalize_extraction_method(method: &str) -> Result<String> {
        match method {
            "LOCAL_TEXT" | "LOCAL_HEURISTIC" => Ok("LOCAL_TEXT".to_string()),
            "CLOUD_AI" => Ok("CLOUD_AI".to_string()),
            _ => Err(Self::invalid_input(format!(
                "Unsupported tax extraction method: {method}"
            ))),
        }
    }

    fn extract_local_document_text(
        content: &[u8],
        mime_type: Option<&str>,
        filename: &str,
    ) -> Result<String> {
        let is_pdf = mime_type
            .map(|mime| mime.eq_ignore_ascii_case("application/pdf"))
            .unwrap_or(false)
            || filename.to_ascii_lowercase().ends_with(".pdf");

        if is_pdf {
            return pdf_extract::extract_text_from_mem(content).map_err(|error| {
                Self::invalid_input(format!("Failed to extract text from PDF document: {error}"))
            });
        }

        Ok(String::from_utf8_lossy(content).to_string())
    }

    fn cto_account_ids(&self) -> Result<HashSet<String>> {
        Ok(self
            .repository
            .get_account_tax_profiles()?
            .into_iter()
            .filter(|profile| profile.regime == TAX_REGIME_CTO)
            .map(|profile| profile.account_id)
            .collect())
    }

    fn compile_tax_events(&self, report: &TaxYearReport) -> Result<CompileOutput> {
        let cto_account_ids = self.cto_account_ids()?;
        let mut output = CompileOutput::default();

        if cto_account_ids.is_empty() {
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "NO_CTO_ACCOUNTS".to_string(),
                message: "No securities account is marked as CTO for this French tax report."
                    .to_string(),
                document_id: None,
                account_id: None,
                activity_id: None,
            });
            return Ok(output);
        }

        let mut activities = self.activity_service.get_activities()?;
        activities.retain(|activity| {
            activity.is_posted() && cto_account_ids.contains(&activity.account_id)
        });
        activities.sort_by_key(|activity| activity.activity_date);

        let mut lots_by_position: HashMap<(String, String), Vec<TaxLot>> = HashMap::new();
        for activity in activities {
            let activity_type = activity.effective_type();
            match activity_type {
                ACTIVITY_TYPE_BUY => {
                    self.add_buy_lot(&mut lots_by_position, &activity, &mut output);
                }
                ACTIVITY_TYPE_SELL => {
                    self.compile_sell(&mut lots_by_position, &activity, report, &mut output);
                }
                ACTIVITY_TYPE_DIVIDEND => {
                    if Self::event_year(&activity) == report.tax_year {
                        self.compile_income(
                            &activity,
                            TaxEventType::DividendReceived,
                            report,
                            &mut output,
                        );
                    }
                }
                ACTIVITY_TYPE_INTEREST => {
                    if Self::event_year(&activity) == report.tax_year {
                        self.compile_income(
                            &activity,
                            TaxEventType::InterestReceived,
                            report,
                            &mut output,
                        );
                    }
                }
                ACTIVITY_TYPE_FEE => {
                    if Self::event_year(&activity) == report.tax_year {
                        self.compile_fee(&activity, report, &mut output);
                    }
                }
                ACTIVITY_TYPE_TAX => {
                    if Self::event_year(&activity) == report.tax_year {
                        self.compile_foreign_withholding_tax(&activity, report, &mut output);
                    }
                }
                _ => {}
            }
        }

        Ok(output)
    }

    fn add_buy_lot(
        &self,
        lots_by_position: &mut HashMap<(String, String), Vec<TaxLot>>,
        activity: &Activity,
        output: &mut CompileOutput,
    ) {
        let Some(asset_id) = activity.asset_id.clone() else {
            return;
        };
        let quantity = activity.qty();
        if quantity <= Decimal::ZERO {
            return;
        }
        let gross = Self::activity_amount(activity) + activity.fee_amt();
        let Some(cost_eur) = Self::amount_eur(activity, gross) else {
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "MISSING_FX".to_string(),
                message: format!(
                    "Missing FX rate for acquisition activity {} in {}.",
                    activity.id, activity.currency
                ),
                document_id: None,
                account_id: Some(activity.account_id.clone()),
                activity_id: Some(activity.id.clone()),
            });
            return;
        };

        lots_by_position
            .entry((activity.account_id.clone(), asset_id))
            .or_default()
            .push(TaxLot {
                activity_id: activity.id.clone(),
                quantity_remaining: quantity,
                acquisition_date: activity.effective_date().to_string(),
                unit_cost_eur: cost_eur / quantity,
            });
    }

    fn compile_income(
        &self,
        activity: &Activity,
        event_type: TaxEventType,
        report: &TaxYearReport,
        output: &mut CompileOutput,
    ) {
        let amount = Self::activity_amount(activity);
        let amount_eur = Self::amount_eur(activity, amount);
        if amount_eur.is_none() {
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "MISSING_FX".to_string(),
                message: format!(
                    "Missing FX rate for income activity {} in {}.",
                    activity.id, activity.currency
                ),
                document_id: None,
                account_id: Some(activity.account_id.clone()),
                activity_id: Some(activity.id.clone()),
            });
        }

        let (category, suggested_box) = match event_type {
            TaxEventType::DividendReceived => (CATEGORY_DIVIDENDS, Some("2042-2DC".to_string())),
            TaxEventType::InterestReceived => (CATEGORY_INTEREST, Some("2042-2TR".to_string())),
            _ => (CATEGORY_DIVIDENDS, None),
        };

        output.events.push(CompiledTaxEvent {
            event: NewTaxEvent {
                event_type,
                category: category.to_string(),
                suggested_box,
                account_id: activity.account_id.clone(),
                asset_id: activity.asset_id.clone(),
                activity_id: Some(activity.id.clone()),
                event_date: activity.effective_date().to_string(),
                amount_currency: activity.currency.clone(),
                amount_local: Some(amount),
                amount_eur,
                taxable_amount_eur: amount_eur,
                expenses_eur: Some(activity.fee_amt()),
                confidence: if amount_eur.is_some() {
                    TaxConfidence::High
                } else {
                    TaxConfidence::Low
                },
                included: amount_eur.is_some(),
                notes: Some(format!("Generated by {}", report.rule_pack_version)),
            },
            sources: vec![NewTaxEventSource {
                source_type: "ACTIVITY".to_string(),
                source_id: activity.id.clone(),
                description: Some("Income activity".to_string()),
            }],
            lot_allocations: Vec::new(),
        });
    }

    fn compile_fee(&self, activity: &Activity, report: &TaxYearReport, output: &mut CompileOutput) {
        let amount = Self::activity_amount(activity).max(activity.fee_amt());
        let amount_eur = Self::amount_eur(activity, amount);
        output.events.push(CompiledTaxEvent {
            event: NewTaxEvent {
                event_type: TaxEventType::FeePaid,
                category: CATEGORY_FEES.to_string(),
                suggested_box: None,
                account_id: activity.account_id.clone(),
                asset_id: activity.asset_id.clone(),
                activity_id: Some(activity.id.clone()),
                event_date: activity.effective_date().to_string(),
                amount_currency: activity.currency.clone(),
                amount_local: Some(amount),
                amount_eur,
                taxable_amount_eur: amount_eur.map(|value| -value),
                expenses_eur: amount_eur,
                confidence: if amount_eur.is_some() {
                    TaxConfidence::Medium
                } else {
                    TaxConfidence::Low
                },
                included: amount_eur.is_some(),
                notes: Some(format!("Generated by {}", report.rule_pack_version)),
            },
            sources: vec![NewTaxEventSource {
                source_type: "ACTIVITY".to_string(),
                source_id: activity.id.clone(),
                description: Some("Fee activity".to_string()),
            }],
            lot_allocations: Vec::new(),
        });
    }

    fn compile_foreign_withholding_tax(
        &self,
        activity: &Activity,
        report: &TaxYearReport,
        output: &mut CompileOutput,
    ) {
        let amount = Self::activity_amount(activity).max(activity.fee_amt());
        let amount_eur = Self::amount_eur(activity, amount);
        if amount_eur.is_none() {
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "MISSING_FX".to_string(),
                message: format!(
                    "Missing FX rate for withholding tax activity {} in {}.",
                    activity.id, activity.currency
                ),
                document_id: None,
                account_id: Some(activity.account_id.clone()),
                activity_id: Some(activity.id.clone()),
            });
        }

        output.events.push(CompiledTaxEvent {
            event: NewTaxEvent {
                event_type: TaxEventType::ForeignWithholdingTax,
                category: CATEGORY_FOREIGN_WITHHOLDING_TAX.to_string(),
                suggested_box: Some("2047".to_string()),
                account_id: activity.account_id.clone(),
                asset_id: activity.asset_id.clone(),
                activity_id: Some(activity.id.clone()),
                event_date: activity.effective_date().to_string(),
                amount_currency: activity.currency.clone(),
                amount_local: Some(amount),
                amount_eur,
                taxable_amount_eur: amount_eur,
                expenses_eur: None,
                confidence: if amount_eur.is_some() {
                    TaxConfidence::Medium
                } else {
                    TaxConfidence::Low
                },
                included: amount_eur.is_some(),
                notes: Some(format!("Generated by {}", report.rule_pack_version)),
            },
            sources: vec![NewTaxEventSource {
                source_type: "ACTIVITY".to_string(),
                source_id: activity.id.clone(),
                description: Some("Foreign withholding tax activity".to_string()),
            }],
            lot_allocations: Vec::new(),
        });
    }

    fn compile_sell(
        &self,
        lots_by_position: &mut HashMap<(String, String), Vec<TaxLot>>,
        activity: &Activity,
        report: &TaxYearReport,
        output: &mut CompileOutput,
    ) {
        let Some(asset_id) = activity.asset_id.clone() else {
            return;
        };
        let quantity = activity.qty();
        if quantity <= Decimal::ZERO {
            return;
        }

        let proceeds_local = Self::activity_amount(activity) - activity.fee_amt();
        let proceeds_eur = Self::amount_eur(activity, proceeds_local);
        let mut remaining = quantity;
        let mut cost_basis_eur = Decimal::ZERO;
        let mut allocations = Vec::new();

        if let Some(lots) =
            lots_by_position.get_mut(&(activity.account_id.clone(), asset_id.clone()))
        {
            for lot in lots.iter_mut() {
                if remaining <= Decimal::ZERO {
                    break;
                }
                if lot.quantity_remaining <= Decimal::ZERO {
                    continue;
                }
                let allocated = remaining.min(lot.quantity_remaining);
                let allocated_cost = allocated * lot.unit_cost_eur;
                cost_basis_eur += allocated_cost;
                lot.quantity_remaining -= allocated;
                remaining -= allocated;
                allocations.push(NewTaxLotAllocation {
                    source_activity_id: lot.activity_id.clone(),
                    quantity: allocated,
                    acquisition_date: lot.acquisition_date.clone(),
                    cost_basis_eur: allocated_cost,
                });
            }
        }

        let mut confidence = TaxConfidence::High;
        let mut included = true;
        if proceeds_eur.is_none() {
            confidence = TaxConfidence::Low;
            included = false;
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "MISSING_FX".to_string(),
                message: format!(
                    "Missing FX rate for disposal activity {} in {}.",
                    activity.id, activity.currency
                ),
                document_id: None,
                account_id: Some(activity.account_id.clone()),
                activity_id: Some(activity.id.clone()),
            });
        }
        if remaining > Decimal::ZERO {
            confidence = TaxConfidence::Low;
            included = false;
            output.issues.push(NewTaxIssue {
                severity: "WARNING".to_string(),
                code: "MISSING_COST_BASIS".to_string(),
                message: format!(
                    "Missing cost basis for {} units sold in activity {}.",
                    remaining, activity.id
                ),
                document_id: None,
                account_id: Some(activity.account_id.clone()),
                activity_id: Some(activity.id.clone()),
            });
        }

        if Self::event_year(activity) != report.tax_year {
            return;
        }

        let gain_eur = proceeds_eur.map(|value| value - cost_basis_eur);
        output.events.push(CompiledTaxEvent {
            event: NewTaxEvent {
                event_type: TaxEventType::SecurityDisposal,
                category: CATEGORY_SECURITY_GAINS.to_string(),
                suggested_box: Some("2074 / 2042-3VG-3VH".to_string()),
                account_id: activity.account_id.clone(),
                asset_id: Some(asset_id),
                activity_id: Some(activity.id.clone()),
                event_date: activity.effective_date().to_string(),
                amount_currency: activity.currency.clone(),
                amount_local: Some(proceeds_local),
                amount_eur: proceeds_eur,
                taxable_amount_eur: gain_eur,
                expenses_eur: Self::amount_eur(activity, activity.fee_amt()),
                confidence,
                included,
                notes: Some(format!(
                    "FIFO cost basis generated by {}",
                    report.rule_pack_version
                )),
            },
            sources: vec![NewTaxEventSource {
                source_type: "ACTIVITY".to_string(),
                source_id: activity.id.clone(),
                description: Some("Security disposal activity".to_string()),
            }],
            lot_allocations: allocations,
        });
    }

    fn build_reconciliation(
        &self,
        report_id: &str,
        events: &[CompiledTaxEvent],
        extracted_fields: &[ExtractedTaxField],
    ) -> Vec<NewTaxReconciliationEntry> {
        let mut app_totals: HashMap<String, Decimal> = HashMap::new();
        let mut boxes: HashMap<String, String> = HashMap::new();
        for event in events.iter().filter(|event| event.event.included) {
            if let Some(amount) = event.event.taxable_amount_eur {
                *app_totals.entry(event.event.category.clone()).or_default() += amount;
            }
            if let Some(suggested_box) = &event.event.suggested_box {
                boxes
                    .entry(event.event.category.clone())
                    .or_insert_with(|| suggested_box.clone());
            }
        }

        let mut document_totals: HashMap<String, Decimal> = HashMap::new();
        for field in extracted_fields {
            if !matches!(field.status.as_str(), "CONFIRMED" | "CORRECTED") {
                continue;
            }
            let Some(category) = field.mapped_category.clone() else {
                continue;
            };
            let amount = field
                .confirmed_amount_eur
                .or(field.amount_eur)
                .unwrap_or(Decimal::ZERO);
            *document_totals.entry(category).or_default() += amount;
        }

        let categories: HashSet<String> = app_totals
            .keys()
            .chain(document_totals.keys())
            .cloned()
            .collect();

        categories
            .into_iter()
            .map(|category| {
                let app_amount = app_totals.get(&category).copied();
                let document_amount = document_totals.get(&category).copied();
                let selected_amount = document_amount.or(app_amount);
                let delta = match (app_amount, document_amount) {
                    (Some(app), Some(document)) => Some(document - app),
                    _ => None,
                };
                let status = match (app_amount, document_amount, delta) {
                    (Some(_), Some(_), Some(value)) if value == Decimal::ZERO => "MATCHED",
                    (Some(_), Some(_), _) => "DELTA_REVIEW",
                    (Some(_), None, _) => "APP_ONLY",
                    (None, Some(_), _) => "DOCUMENT_ONLY",
                    _ => "EMPTY",
                };

                let suggested_box = boxes.get(&category).cloned();
                NewTaxReconciliationEntry {
                    category,
                    suggested_box,
                    app_amount_eur: app_amount,
                    document_amount_eur: document_amount,
                    selected_amount_eur: selected_amount,
                    delta_eur: delta,
                    status: status.to_string(),
                    notes: Some(format!("Reconciled for report {report_id}")),
                }
            })
            .collect()
    }

    fn extracted_fields_for_report(&self, report_id: &str) -> Result<Vec<ExtractedTaxField>> {
        Ok(self
            .repository
            .list_tax_document_extractions(report_id)?
            .into_iter()
            .flat_map(|result| result.fields)
            .collect())
    }

    fn parse_ifu_fields(text: &str) -> Vec<NewExtractedTaxField> {
        text.lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let lower = line.to_lowercase();
                let (category, label, suggested_box) = if lower.contains("dividend")
                    || lower.contains("dividende")
                    || lower.contains("dividendes")
                {
                    (CATEGORY_DIVIDENDS, "Dividends", Some("2042-2DC"))
                } else if lower.contains("withholding")
                    || lower.contains("retenue")
                    || lower.contains("prélevement")
                    || lower.contains("prelevement")
                    || lower.contains("foreign tax")
                {
                    (
                        CATEGORY_FOREIGN_WITHHOLDING_TAX,
                        "Foreign withholding tax",
                        Some("2047"),
                    )
                } else if lower.contains("interest")
                    || lower.contains("intérêt")
                    || lower.contains("interet")
                    || lower.contains("intérêts")
                    || lower.contains("interets")
                {
                    (CATEGORY_INTEREST, "Interest", Some("2042-2TR"))
                } else if lower.contains("plus-value")
                    || lower.contains("plus value")
                    || lower.contains("gain")
                    || lower.contains("cession")
                {
                    (
                        CATEGORY_SECURITY_GAINS,
                        "Security gains",
                        Some("2074 / 2042-3VG-3VH"),
                    )
                } else if lower.contains("frais") || lower.contains("fee") {
                    (CATEGORY_FEES, "Fees", None)
                } else {
                    return None;
                };

                Self::parse_decimal_from_line(line).map(|amount| NewExtractedTaxField {
                    field_key: category.to_string(),
                    label: label.to_string(),
                    mapped_category: Some(category.to_string()),
                    suggested_declaration_box: suggested_box.map(ToString::to_string),
                    source_locator_json: Some(
                        json!({
                            "lineNumber": index + 1,
                            "snippet": line.trim(),
                        })
                        .to_string(),
                    ),
                    value_text: Some(line.trim().to_string()),
                    amount_eur: Some(amount),
                    confidence: 0.55,
                    status: "SUGGESTED".to_string(),
                    confirmed_amount_eur: None,
                })
            })
            .collect()
    }

    /// Heuristic extraction for fiche de paie (salary slip) documents.
    ///
    /// Direct-population mode: the document is the source of truth. Extracted
    /// fields are presented to the user for confirmation before inclusion.
    ///
    /// Targets:
    /// - `NET_IMPOSABLE`    → net imposable cumulé (box 1AJ)
    /// - `CSG_DEDUCTIBLE`   → CSG déductible cumulée (reduces revenu imposable)
    /// - `HEURES_SUP`       → heures supplémentaires exonérées (box 1GH)
    fn parse_fiche_fields(text: &str) -> Vec<NewExtractedTaxField> {
        text.lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let lower = line.to_lowercase();
                let (field_key, label, suggested_box) = if lower.contains("net imposable")
                    || lower.contains("revenu imposable")
                    || lower.contains("net fiscal")
                    || lower.contains("cumul imposable")
                {
                    ("NET_IMPOSABLE", "Net imposable cumulé", Some("1AJ"))
                } else if lower.contains("csg déductible")
                    || lower.contains("csg deductible")
                    || lower.contains("csg non imposable")
                {
                    ("CSG_DEDUCTIBLE", "CSG déductible", None)
                } else if lower.contains("heures sup")
                    || lower.contains("heures supplémentaires")
                    || lower.contains("h. sup")
                    || lower.contains("exonér")
                    || lower.contains("exoner")
                {
                    (
                        "HEURES_SUP",
                        "Heures supplémentaires exonérées",
                        Some("1GH"),
                    )
                } else {
                    return None;
                };

                Self::parse_decimal_from_line(line).map(|amount| NewExtractedTaxField {
                    field_key: field_key.to_string(),
                    label: label.to_string(),
                    mapped_category: Some(CATEGORY_SALARY_INCOME.to_string()),
                    suggested_declaration_box: suggested_box.map(ToString::to_string),
                    source_locator_json: Some(
                        json!({
                            "lineNumber": index + 1,
                            "snippet": line.trim(),
                        })
                        .to_string(),
                    ),
                    value_text: Some(line.trim().to_string()),
                    amount_eur: Some(amount),
                    confidence: 0.55,
                    status: "SUGGESTED".to_string(),
                    confirmed_amount_eur: None,
                })
            })
            .collect()
    }

    fn parse_decimal_from_line(line: &str) -> Option<Decimal> {
        let normalized = line.replace(',', ".");
        normalized
            .split(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-'))
            .filter(|part| !part.is_empty() && part.chars().any(|ch| ch.is_ascii_digit()))
            .filter_map(|part| part.parse::<Decimal>().ok())
            .next_back()
    }

    fn extraction_issue_codes(document_id: &str) -> Vec<String> {
        vec![
            format!("DOCUMENT_EXTRACTION_EMPTY:{document_id}"),
            format!("DOCUMENT_EXTRACTION_LOW_CONFIDENCE:{document_id}"),
        ]
    }

    fn build_extraction_issues(
        document: &TaxDocument,
        method: &str,
        fields: &[NewExtractedTaxField],
    ) -> Vec<NewTaxIssue> {
        if fields.is_empty() {
            return vec![NewTaxIssue {
                severity: "WARNING".to_string(),
                code: format!("DOCUMENT_EXTRACTION_EMPTY:{}", document.id),
                message: format!(
                    "No tax fields were extracted from {} using {}. Review the document manually or retry extraction.",
                    document.filename, method
                ),
                document_id: Some(document.id.clone()),
                account_id: None,
                activity_id: None,
            }];
        }

        if fields
            .iter()
            .all(|field| field.confidence < EXTRACTION_CONFIDENCE_WARNING_THRESHOLD)
        {
            return vec![NewTaxIssue {
                severity: "WARNING".to_string(),
                code: format!("DOCUMENT_EXTRACTION_LOW_CONFIDENCE:{}", document.id),
                message: format!(
                    "Extracted values from {} using {} are low confidence and should be reviewed before reconciliation.",
                    document.filename, method
                ),
                document_id: Some(document.id.clone()),
                account_id: None,
                activity_id: None,
            }];
        }

        Vec::new()
    }

    fn extraction_status(fields: &[NewExtractedTaxField]) -> &'static str {
        if fields.is_empty() {
            return "NO_FIELDS_FOUND";
        }

        if fields
            .iter()
            .all(|field| field.confidence < EXTRACTION_CONFIDENCE_WARNING_THRESHOLD)
        {
            return "LOW_CONFIDENCE";
        }

        "READY_FOR_REVIEW"
    }

    /// Build salary income events from confirmed/corrected NET_IMPOSABLE fiche de paie fields.
    ///
    /// Only the latest cumul (highest amount) per report is used — fiche de paie amounts
    /// are cumulative YTD, so only the December slip's total matters.
    fn compile_salary_events(
        report: &TaxYearReport,
        extracted_fields: &[ExtractedTaxField],
    ) -> Vec<CompiledTaxEvent> {
        // Collect confirmed NET_IMPOSABLE amounts from fiche de paie extractions.
        let mut net_imposable_amounts: Vec<Decimal> = extracted_fields
            .iter()
            .filter(|field| {
                matches!(field.status.as_str(), "CONFIRMED" | "CORRECTED")
                    && field.field_key == "NET_IMPOSABLE"
                    && field.mapped_category.as_deref() == Some(CATEGORY_SALARY_INCOME)
            })
            .filter_map(|field| field.confirmed_amount_eur.or(field.amount_eur))
            .collect();

        if net_imposable_amounts.is_empty() {
            return Vec::new();
        }

        // Use the largest value — latest cumul annuel.
        net_imposable_amounts.sort();
        let amount = *net_imposable_amounts.last().unwrap();

        vec![CompiledTaxEvent {
            event: NewTaxEvent {
                event_type: TaxEventType::SalaryIncome,
                category: CATEGORY_SALARY_INCOME.to_string(),
                suggested_box: Some("1AJ".to_string()),
                account_id: SALARY_VIRTUAL_ACCOUNT_ID.to_string(),
                asset_id: None,
                activity_id: None,
                event_date: format!("{}-12-31", report.tax_year),
                amount_currency: report.base_currency.clone(),
                amount_local: Some(amount),
                amount_eur: Some(amount),
                taxable_amount_eur: Some(amount),
                expenses_eur: None,
                confidence: TaxConfidence::High,
                included: true,
                notes: Some(format!(
                    "Net imposable cumulé from fiche de paie ({})",
                    report.rule_pack_version
                )),
            },
            sources: Vec::new(),
            lot_allocations: Vec::new(),
        }]
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
            situation_familiale: "CELIBATAIRE".to_string(),
            nombre_enfants: 0,
            nombre_enfants_handicapes: 0,
            parent_isole: false,
            ancien_combattant_ou_invalidite: false,
            nombre_parts: 1.0,
            created_at: now,
            updated_at: now,
        })
    }

    async fn update_tax_profile(&self, profile: TaxProfileUpdate) -> Result<TaxProfile> {
        use crate::tax::compute_nombre_parts;
        let parts = compute_nombre_parts(
            &profile.situation_familiale,
            profile.nombre_enfants,
            profile.nombre_enfants_handicapes,
            profile.parent_isole,
            profile.ancien_combattant_ou_invalidite,
        );
        self.repository.upsert_tax_profile(profile, parts).await
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

    fn get_tax_report_detail(&self, id: &str) -> Result<Option<TaxReportDetail>> {
        self.repository.get_tax_report_detail(id)
    }

    async fn regenerate_tax_year_report(&self, id: &str) -> Result<TaxReportDetail> {
        let report = self
            .repository
            .get_tax_year_report(id)?
            .ok_or_else(|| Self::not_found(format!("Tax report {id} not found")))?;
        if report.status == TaxReportStatus::Finalized {
            return Err(Self::not_found(
                "Finalized tax reports cannot be regenerated",
            ));
        }

        let mut compiled = self.compile_tax_events(&report)?;
        let extracted_fields = self.extracted_fields_for_report(id)?;
        let salary_events = Self::compile_salary_events(&report, &extracted_fields);
        compiled.events.extend(salary_events);
        let reconciliation = self.build_reconciliation(id, &compiled.events, &extracted_fields);
        let summary_json = json!({
            "eventCount": compiled.events.len(),
            "includedEventCount": compiled.events.iter().filter(|event| event.event.included).count(),
            "issueCount": compiled.issues.len(),
            "reconciliationCount": reconciliation.len(),
            "generatedAt": Utc::now().to_rfc3339(),
        })
        .to_string();

        self.repository
            .replace_generated_report_data(
                id,
                summary_json,
                compiled.events,
                compiled.issues,
                reconciliation,
            )
            .await
    }

    async fn finalize_tax_year_report(&self, id: &str) -> Result<TaxYearReport> {
        let report = self
            .repository
            .get_tax_year_report(id)?
            .ok_or_else(|| Self::not_found(format!("Tax report {id} not found")))?;
        if report.status == TaxReportStatus::Finalized {
            return Err(Self::invalid_input(format!(
                "Tax report {id} is already finalized"
            )));
        }

        let has_blocking_issues = self
            .repository
            .list_tax_issues(id)?
            .into_iter()
            .any(|issue| {
                issue.resolved_at.is_none() && issue.severity.eq_ignore_ascii_case("ERROR")
            });
        if has_blocking_issues {
            return Err(Self::invalid_input(
                "Tax report has unresolved blocking issues and cannot be finalized",
            ));
        }

        self.repository.finalize_tax_year_report(id).await
    }

    async fn amend_tax_year_report(&self, id: &str) -> Result<TaxYearReport> {
        let parent = self
            .repository
            .get_tax_year_report(id)?
            .ok_or_else(|| Self::not_found(format!("Tax report {id} not found")))?;
        if parent.status != TaxReportStatus::Finalized {
            return Err(Self::not_found("Only finalized tax reports can be amended"));
        }
        self.repository.create_amended_report(parent).await
    }

    async fn upload_tax_document(&self, upload: TaxDocumentUpload) -> Result<TaxDocument> {
        if upload.content.is_empty() {
            return Err(Error::Validation(ValidationError::MissingField(
                "content".to_string(),
            )));
        }
        self.editable_report_by_id(&upload.report_id)?;
        self.repository
            .upload_tax_document(
                upload.report_id,
                upload.document_type,
                upload.filename,
                upload.mime_type,
                upload.content,
            )
            .await
    }

    fn list_tax_documents(&self, report_id: &str) -> Result<Vec<TaxDocument>> {
        self.repository.list_tax_documents(report_id)
    }

    async fn delete_tax_document(&self, document_id: &str) -> Result<()> {
        self.editable_report_for_document(document_id)?;
        self.repository.delete_tax_document(document_id).await
    }

    fn get_tax_document_download(&self, document_id: &str) -> Result<Option<TaxDocumentDownload>> {
        let document = match self.repository.get_tax_document(document_id)? {
            Some(document) => document,
            None => return Ok(None),
        };
        let content = self
            .repository
            .get_tax_document_content(document_id)?
            .ok_or_else(|| Self::not_found(format!("Tax document {document_id} not found")))?;
        Ok(Some(TaxDocumentDownload {
            filename: document.filename,
            mime_type: document.mime_type,
            content,
        }))
    }

    async fn extract_tax_document(
        &self,
        request: TaxDocumentExtractionRequest,
    ) -> Result<TaxDocumentExtractionResult> {
        let normalized_method = Self::normalize_extraction_method(&request.method)?;
        if normalized_method == "CLOUD_AI" && !request.consent_granted {
            return Err(Error::Validation(ValidationError::InvalidInput(
                "Cloud AI extraction requires explicit consent.".to_string(),
            )));
        }
        let document = self
            .repository
            .get_tax_document(&request.document_id)?
            .ok_or_else(|| {
                Self::not_found(format!("Tax document {} not found", request.document_id))
            })?;
        self.editable_report_by_id(&document.report_id)?;
        let content = self
            .repository
            .get_tax_document_content(&request.document_id)?
            .ok_or_else(|| {
                Self::not_found(format!("Tax document {} not found", request.document_id))
            })?;
        let text = Self::extract_local_document_text(
            &content,
            document.mime_type.as_deref(),
            &document.filename,
        )?;
        let preview: String = text.chars().take(4000).collect();
        let fields = if normalized_method == "CLOUD_AI" {
            let extractor = self.cloud_extractor.as_ref().ok_or_else(|| {
                Self::invalid_input(
                    "Cloud AI extraction is unavailable in this runtime or has not been configured",
                )
            })?;
            extractor
                .extract_tax_fields(&document, &content, &preview)
                .await?
        } else if document.document_type == DOCUMENT_TYPE_FICHE_DE_PAIE {
            Self::parse_fiche_fields(&preview)
        } else {
            Self::parse_ifu_fields(&preview)
        };
        let extraction_issues =
            Self::build_extraction_issues(&document, &normalized_method, &fields);
        let extraction_status = Self::extraction_status(&fields).to_string();
        let request = TaxDocumentExtractionRequest {
            method: normalized_method,
            ..request
        };
        let extraction = self
            .repository
            .create_tax_document_extraction(request, extraction_status, Some(preview), fields)
            .await?;
        self.repository
            .replace_tax_issues_by_code(
                &document.report_id,
                Self::extraction_issue_codes(&document.id),
                extraction_issues,
            )
            .await?;
        Ok(extraction)
    }

    async fn update_extracted_tax_field(
        &self,
        update: ExtractedTaxFieldUpdate,
    ) -> Result<ExtractedTaxField> {
        self.editable_report_for_extracted_field(&update.field_id)?;
        self.repository.update_extracted_tax_field(update).await
    }

    async fn reconcile_tax_year_report(&self, id: &str) -> Result<Vec<TaxReconciliationEntry>> {
        self.editable_report_by_id(id)?;
        let events = self.repository.list_tax_events(id)?;
        let compiled_events: Vec<CompiledTaxEvent> = events
            .into_iter()
            .map(|event| CompiledTaxEvent {
                event: NewTaxEvent {
                    event_type: event.event_type,
                    category: event.category,
                    suggested_box: event.suggested_box,
                    account_id: event.account_id,
                    asset_id: event.asset_id,
                    activity_id: event.activity_id,
                    event_date: event.event_date,
                    amount_currency: event.amount_currency,
                    amount_local: event.amount_local,
                    amount_eur: event.amount_eur,
                    taxable_amount_eur: event.taxable_amount_eur,
                    expenses_eur: event.expenses_eur,
                    confidence: event.confidence,
                    included: event.included,
                    notes: event.notes,
                },
                sources: Vec::new(),
                lot_allocations: Vec::new(),
            })
            .collect();
        let extracted_fields = self.extracted_fields_for_report(id)?;
        let entries = self.build_reconciliation(id, &compiled_events, &extracted_fields);
        self.repository
            .replace_reconciliation_entries(id, entries)
            .await
    }

    async fn update_tax_reconciliation_entry(
        &self,
        update: TaxReconciliationEntryUpdate,
    ) -> Result<TaxReconciliationEntry> {
        self.editable_report_for_reconciliation_entry(&update.id)?;
        let existing = self
            .repository
            .get_tax_reconciliation_entry(&update.id)?
            .ok_or_else(|| {
                Self::not_found(format!("Tax reconciliation entry {} not found", update.id))
            })?;

        let status = update.status.trim();
        let normalized_update = match status {
            "USER_SELECTED_APP" => TaxReconciliationEntryUpdate {
                selected_amount_eur: existing.app_amount_eur,
                status: status.to_string(),
                notes: update.notes,
                ..update
            },
            "USER_SELECTED_DOCUMENT" => TaxReconciliationEntryUpdate {
                selected_amount_eur: existing.document_amount_eur,
                status: status.to_string(),
                notes: update.notes,
                ..update
            },
            "USER_OVERRIDE" => {
                let has_reason = update
                    .notes
                    .as_ref()
                    .map(|notes| !notes.trim().is_empty())
                    .unwrap_or(false);
                if update.selected_amount_eur.is_none() {
                    return Err(Self::invalid_input(
                        "Manual tax reconciliation overrides require a selected amount",
                    ));
                }
                if !has_reason {
                    return Err(Self::invalid_input(
                        "Manual tax reconciliation overrides require a reason",
                    ));
                }
                TaxReconciliationEntryUpdate {
                    status: status.to_string(),
                    notes: update.notes.map(|notes| notes.trim().to_string()),
                    ..update
                }
            }
            _ => TaxReconciliationEntryUpdate {
                status: status.to_string(),
                notes: update.notes,
                ..update
            },
        };
        self.repository
            .update_tax_reconciliation_entry(normalized_update)
            .await
    }

    async fn update_tax_event(&self, update: TaxEventUpdate) -> Result<TaxEvent> {
        self.editable_report_for_tax_event(&update.id)?;
        self.repository.update_tax_event(update).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activities::{
        Activity, ActivityBulkMutationRequest, ActivityBulkMutationResult, ActivityImport,
        ActivitySearchResponse, ActivitySearchResponseMeta, ActivityServiceTrait, ActivityUpdate,
        ActivityUpsert, BrokerSyncProfileData, BulkUpsertResult, ImportActivitiesResult,
        ImportAssetCandidate, ImportAssetPreviewItem, ImportMappingData, ImportTemplateData,
        NewActivity, PrepareActivitiesResult, SaveBrokerSyncProfileRulesRequest, Sort,
    };
    use crate::tax::TaxIssue;
    use async_trait::async_trait;
    use chrono::{DateTime, NaiveDate, Utc};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, RwLock};

    struct MockActivityService;

    #[async_trait]
    impl ActivityServiceTrait for MockActivityService {
        fn get_activity(&self, _activity_id: &str) -> Result<Activity> {
            unimplemented!()
        }

        fn get_activities(&self) -> Result<Vec<Activity>> {
            Ok(Vec::new())
        }

        fn get_activities_by_account_id(&self, _account_id: &str) -> Result<Vec<Activity>> {
            unimplemented!()
        }

        fn get_activities_by_account_ids(&self, _account_ids: &[String]) -> Result<Vec<Activity>> {
            unimplemented!()
        }

        fn get_trading_activities(&self) -> Result<Vec<Activity>> {
            unimplemented!()
        }

        fn get_income_activities(&self) -> Result<Vec<Activity>> {
            unimplemented!()
        }

        fn search_activities(
            &self,
            _page: i64,
            _page_size: i64,
            _account_id_filter: Option<Vec<String>>,
            _activity_type_filter: Option<Vec<String>>,
            _asset_id_keyword: Option<String>,
            _sort: Option<Sort>,
            _needs_review_filter: Option<bool>,
            _date_from: Option<NaiveDate>,
            _date_to: Option<NaiveDate>,
            _instrument_type_filter: Option<Vec<String>>,
        ) -> Result<ActivitySearchResponse> {
            Ok(ActivitySearchResponse {
                data: Vec::new(),
                meta: ActivitySearchResponseMeta { total_row_count: 0 },
            })
        }

        fn get_first_activity_date(
            &self,
            _account_ids: Option<&[String]>,
        ) -> Result<Option<DateTime<Utc>>> {
            unimplemented!()
        }

        fn get_import_mapping(
            &self,
            _account_id: String,
            _context_kind: String,
        ) -> Result<ImportMappingData> {
            unimplemented!()
        }

        fn list_import_templates(&self) -> Result<Vec<ImportTemplateData>> {
            unimplemented!()
        }

        fn get_import_template(&self, _template_id: String) -> Result<ImportTemplateData> {
            unimplemented!()
        }

        async fn create_activity(&self, _activity: NewActivity) -> Result<Activity> {
            unimplemented!()
        }

        async fn update_activity(&self, _activity: ActivityUpdate) -> Result<Activity> {
            unimplemented!()
        }

        async fn delete_activity(&self, _activity_id: String) -> Result<Activity> {
            unimplemented!()
        }

        async fn link_transfer_activities(
            &self,
            _activity_a_id: String,
            _activity_b_id: String,
        ) -> Result<(Activity, Activity)> {
            unimplemented!()
        }

        async fn bulk_mutate_activities(
            &self,
            _request: ActivityBulkMutationRequest,
        ) -> Result<ActivityBulkMutationResult> {
            unimplemented!()
        }

        async fn check_activities_import(
            &self,
            _activities: Vec<ActivityImport>,
        ) -> Result<Vec<ActivityImport>> {
            unimplemented!()
        }

        async fn preview_import_assets(
            &self,
            _candidates: Vec<ImportAssetCandidate>,
        ) -> Result<Vec<ImportAssetPreviewItem>> {
            unimplemented!()
        }

        async fn import_activities(
            &self,
            _activities: Vec<ActivityImport>,
        ) -> Result<ImportActivitiesResult> {
            unimplemented!()
        }

        async fn link_account_template(
            &self,
            _account_id: String,
            _template_id: String,
            _context_kind: String,
        ) -> Result<()> {
            unimplemented!()
        }

        async fn save_import_mapping(
            &self,
            _mapping_data: ImportMappingData,
        ) -> Result<ImportMappingData> {
            unimplemented!()
        }

        async fn save_import_template(
            &self,
            _template_data: ImportTemplateData,
        ) -> Result<ImportTemplateData> {
            unimplemented!()
        }

        async fn delete_import_template(&self, _template_id: String) -> Result<()> {
            unimplemented!()
        }

        fn check_existing_duplicates(
            &self,
            _idempotency_keys: Vec<String>,
        ) -> Result<HashMap<String, String>> {
            unimplemented!()
        }

        fn parse_csv(
            &self,
            _content: &[u8],
            _config: &crate::activities::ParseConfig,
        ) -> Result<crate::activities::ParsedCsvResult> {
            unimplemented!()
        }

        async fn upsert_activities_bulk(
            &self,
            _activities: Vec<ActivityUpsert>,
        ) -> Result<BulkUpsertResult> {
            unimplemented!()
        }

        async fn prepare_activities_for_save(
            &self,
            _activities: Vec<NewActivity>,
            _account: &crate::accounts::Account,
        ) -> Result<PrepareActivitiesResult> {
            unimplemented!()
        }

        async fn prepare_activities_for_import(
            &self,
            _activities: Vec<NewActivity>,
            _account: &crate::accounts::Account,
        ) -> Result<PrepareActivitiesResult> {
            unimplemented!()
        }

        async fn prepare_activities_for_sync(
            &self,
            _activities: Vec<NewActivity>,
            _account: &crate::accounts::Account,
        ) -> Result<PrepareActivitiesResult> {
            unimplemented!()
        }

        fn get_broker_sync_profile(
            &self,
            _account_id: String,
            _source_system: String,
        ) -> Result<BrokerSyncProfileData> {
            unimplemented!()
        }

        async fn save_broker_sync_profile_rules(
            &self,
            _request: SaveBrokerSyncProfileRulesRequest,
        ) -> Result<BrokerSyncProfileData> {
            unimplemented!()
        }
    }

    struct MockTaxRepository {
        reports: RwLock<HashMap<String, TaxYearReport>>,
        issues_by_report: RwLock<HashMap<String, Vec<TaxIssue>>>,
        documents: RwLock<HashMap<String, TaxDocument>>,
        document_contents: RwLock<HashMap<String, Vec<u8>>>,
        extractions: RwLock<HashMap<String, crate::tax::TaxDocumentExtraction>>,
        fields: RwLock<HashMap<String, ExtractedTaxField>>,
        reconciliation_entries: RwLock<HashMap<String, TaxReconciliationEntry>>,
        events: RwLock<HashMap<String, TaxEvent>>,
        amended_from: Mutex<Vec<String>>,
        last_reconciliation_update: Mutex<Option<TaxReconciliationEntryUpdate>>,
    }

    struct MockCloudExtractor {
        called: AtomicBool,
        fields: Vec<NewExtractedTaxField>,
    }

    #[async_trait]
    impl TaxCloudExtractionTrait for MockCloudExtractor {
        async fn extract_tax_fields(
            &self,
            _document: &TaxDocument,
            _content: &[u8],
            _local_text_preview: &str,
        ) -> Result<Vec<NewExtractedTaxField>> {
            self.called.store(true, Ordering::Relaxed);
            Ok(self.fields.clone())
        }
    }

    impl MockTaxRepository {
        fn new(report: TaxYearReport) -> Self {
            let report_id = report.id.clone();
            let mut reports = HashMap::new();
            reports.insert(report_id.clone(), report);
            let mut issues_by_report = HashMap::new();
            issues_by_report.insert(report_id, Vec::new());

            Self {
                reports: RwLock::new(reports),
                issues_by_report: RwLock::new(issues_by_report),
                documents: RwLock::new(HashMap::new()),
                document_contents: RwLock::new(HashMap::new()),
                extractions: RwLock::new(HashMap::new()),
                fields: RwLock::new(HashMap::new()),
                reconciliation_entries: RwLock::new(HashMap::new()),
                events: RwLock::new(HashMap::new()),
                amended_from: Mutex::new(Vec::new()),
                last_reconciliation_update: Mutex::new(None),
            }
        }

        fn add_issue(&self, issue: TaxIssue) {
            self.issues_by_report
                .write()
                .unwrap()
                .entry(issue.report_id.clone())
                .or_default()
                .push(issue);
        }

        fn add_document(&self, document: TaxDocument) {
            let document_id = document.id.clone();
            self.documents
                .write()
                .unwrap()
                .insert(document.id.clone(), document);
            self.document_contents
                .write()
                .unwrap()
                .insert(document_id, b"Dividends 10".to_vec());
        }

        fn add_extraction(&self, extraction: crate::tax::TaxDocumentExtraction) {
            self.extractions
                .write()
                .unwrap()
                .insert(extraction.id.clone(), extraction);
        }

        fn add_field(&self, field: ExtractedTaxField) {
            self.fields.write().unwrap().insert(field.id.clone(), field);
        }

        fn add_reconciliation_entry(&self, entry: TaxReconciliationEntry) {
            self.reconciliation_entries
                .write()
                .unwrap()
                .insert(entry.id.clone(), entry);
        }

        fn add_event(&self, event: TaxEvent) {
            self.events.write().unwrap().insert(event.id.clone(), event);
        }
    }

    #[async_trait]
    impl TaxRepositoryTrait for MockTaxRepository {
        fn get_tax_profile(&self) -> Result<Option<TaxProfile>> {
            unimplemented!()
        }

        async fn upsert_tax_profile(
            &self,
            _profile: TaxProfileUpdate,
            _nombre_parts: f64,
        ) -> Result<TaxProfile> {
            unimplemented!()
        }

        fn get_account_tax_profiles(&self) -> Result<Vec<AccountTaxProfile>> {
            Ok(Vec::new())
        }

        fn get_account_tax_profile(&self, _account_id: &str) -> Result<Option<AccountTaxProfile>> {
            Ok(None)
        }

        async fn upsert_account_tax_profile(
            &self,
            _profile: AccountTaxProfileUpdate,
        ) -> Result<AccountTaxProfile> {
            unimplemented!()
        }

        fn list_tax_year_reports(&self) -> Result<Vec<TaxYearReport>> {
            Ok(self.reports.read().unwrap().values().cloned().collect())
        }

        fn get_tax_year_report(&self, id: &str) -> Result<Option<TaxYearReport>> {
            Ok(self.reports.read().unwrap().get(id).cloned())
        }

        fn find_draft_tax_year_report(
            &self,
            _tax_year: i32,
            _jurisdiction: &str,
        ) -> Result<Option<TaxYearReport>> {
            Ok(None)
        }

        async fn create_tax_year_report(
            &self,
            _report: NewTaxYearReport,
            _jurisdiction: String,
            _base_currency: String,
            _rule_pack_version: String,
        ) -> Result<TaxYearReport> {
            unimplemented!()
        }

        async fn create_amended_report(&self, parent: TaxYearReport) -> Result<TaxYearReport> {
            self.amended_from.lock().unwrap().push(parent.id.clone());
            let amended = make_report("amended-report", TaxReportStatus::AmendedDraft);
            let amended = TaxYearReport {
                parent_report_id: Some(parent.id),
                ..amended
            };
            self.reports
                .write()
                .unwrap()
                .insert(amended.id.clone(), amended.clone());
            Ok(amended)
        }

        async fn replace_generated_report_data(
            &self,
            _report_id: &str,
            _summary_json: String,
            _events: Vec<CompiledTaxEvent>,
            _issues: Vec<NewTaxIssue>,
            _reconciliation: Vec<NewTaxReconciliationEntry>,
        ) -> Result<TaxReportDetail> {
            unimplemented!()
        }

        async fn finalize_tax_year_report(&self, report_id: &str) -> Result<TaxYearReport> {
            let mut reports = self.reports.write().unwrap();
            let report = reports.get_mut(report_id).expect("report exists");
            report.status = TaxReportStatus::Finalized;
            Ok(report.clone())
        }

        fn get_tax_report_detail(&self, _report_id: &str) -> Result<Option<TaxReportDetail>> {
            unimplemented!()
        }

        async fn upload_tax_document(
            &self,
            _report_id: String,
            _document_type: String,
            _filename: String,
            _mime_type: Option<String>,
            _content: Vec<u8>,
        ) -> Result<TaxDocument> {
            panic!("upload_tax_document should not be called in these tests")
        }

        fn list_tax_documents(&self, _report_id: &str) -> Result<Vec<TaxDocument>> {
            Ok(Vec::new())
        }

        fn get_tax_document(&self, document_id: &str) -> Result<Option<TaxDocument>> {
            Ok(self.documents.read().unwrap().get(document_id).cloned())
        }

        fn get_tax_document_content(&self, document_id: &str) -> Result<Option<Vec<u8>>> {
            Ok(self
                .document_contents
                .read()
                .unwrap()
                .get(document_id)
                .cloned())
        }

        fn get_tax_document_extraction(
            &self,
            extraction_id: &str,
        ) -> Result<Option<crate::tax::TaxDocumentExtraction>> {
            Ok(self.extractions.read().unwrap().get(extraction_id).cloned())
        }

        fn get_extracted_tax_field(&self, field_id: &str) -> Result<Option<ExtractedTaxField>> {
            Ok(self.fields.read().unwrap().get(field_id).cloned())
        }

        async fn delete_tax_document(&self, _document_id: &str) -> Result<()> {
            panic!("delete_tax_document should not be called in these tests")
        }

        async fn create_tax_document_extraction(
            &self,
            request: TaxDocumentExtractionRequest,
            status: String,
            raw_text_preview: Option<String>,
            fields: Vec<crate::tax::NewExtractedTaxField>,
        ) -> Result<TaxDocumentExtractionResult> {
            let now = Utc::now().naive_utc();
            let extraction = crate::tax::TaxDocumentExtraction {
                id: "mock-extraction".to_string(),
                document_id: request.document_id,
                method: request.method,
                status,
                consent_granted: request.consent_granted,
                raw_text_preview,
                created_at: now,
                updated_at: now,
            };
            let fields = fields
                .into_iter()
                .enumerate()
                .map(|(index, field)| ExtractedTaxField {
                    id: format!("field-{index}"),
                    extraction_id: extraction.id.clone(),
                    field_key: field.field_key,
                    label: field.label,
                    mapped_category: field.mapped_category,
                    suggested_declaration_box: field.suggested_declaration_box,
                    source_locator_json: field.source_locator_json,
                    value_text: field.value_text,
                    amount_eur: field.amount_eur,
                    confidence: field.confidence,
                    status: field.status,
                    confirmed_amount_eur: field.confirmed_amount_eur,
                    created_at: now,
                    updated_at: now,
                })
                .collect();
            Ok(TaxDocumentExtractionResult { extraction, fields })
        }

        async fn replace_tax_issues_by_code(
            &self,
            report_id: &str,
            issue_codes: Vec<String>,
            issues: Vec<NewTaxIssue>,
        ) -> Result<Vec<TaxIssue>> {
            let mut issues_by_report = self.issues_by_report.write().unwrap();
            let existing = issues_by_report.entry(report_id.to_string()).or_default();
            existing.retain(|issue| !issue_codes.contains(&issue.code));

            let now = Utc::now().naive_utc();
            let new_rows = issues
                .into_iter()
                .map(|issue| TaxIssue {
                    id: format!("issue-{}", issue.code),
                    report_id: report_id.to_string(),
                    severity: issue.severity,
                    code: issue.code,
                    message: issue.message,
                    document_id: issue.document_id,
                    account_id: issue.account_id,
                    activity_id: issue.activity_id,
                    tax_event_id: None,
                    resolved_at: None,
                    created_at: now,
                })
                .collect::<Vec<_>>();
            existing.extend(new_rows.clone());
            Ok(new_rows)
        }

        fn list_tax_document_extractions(
            &self,
            _report_id: &str,
        ) -> Result<Vec<TaxDocumentExtractionResult>> {
            Ok(Vec::new())
        }

        async fn update_extracted_tax_field(
            &self,
            _update: ExtractedTaxFieldUpdate,
        ) -> Result<ExtractedTaxField> {
            panic!("update_extracted_tax_field should not be called in these tests")
        }

        async fn replace_reconciliation_entries(
            &self,
            _report_id: &str,
            _entries: Vec<NewTaxReconciliationEntry>,
        ) -> Result<Vec<TaxReconciliationEntry>> {
            panic!("replace_reconciliation_entries should not be called in these tests")
        }

        async fn update_tax_reconciliation_entry(
            &self,
            update: TaxReconciliationEntryUpdate,
        ) -> Result<TaxReconciliationEntry> {
            *self.last_reconciliation_update.lock().unwrap() = Some(update.clone());
            let mut entries = self.reconciliation_entries.write().unwrap();
            let entry = entries
                .get_mut(&update.id)
                .expect("reconciliation entry should exist");
            entry.selected_amount_eur = update.selected_amount_eur;
            entry.status = update.status;
            entry.notes = update.notes;
            Ok(entry.clone())
        }

        fn get_tax_event(&self, event_id: &str) -> Result<Option<TaxEvent>> {
            Ok(self.events.read().unwrap().get(event_id).cloned())
        }

        async fn update_tax_event(&self, _update: TaxEventUpdate) -> Result<TaxEvent> {
            panic!("update_tax_event should not be called in these tests")
        }

        fn list_tax_events(&self, _report_id: &str) -> Result<Vec<TaxEvent>> {
            Ok(Vec::new())
        }

        fn list_tax_issues(&self, report_id: &str) -> Result<Vec<TaxIssue>> {
            Ok(self
                .issues_by_report
                .read()
                .unwrap()
                .get(report_id)
                .cloned()
                .unwrap_or_default())
        }

        fn get_tax_reconciliation_entry(
            &self,
            entry_id: &str,
        ) -> Result<Option<TaxReconciliationEntry>> {
            Ok(self
                .reconciliation_entries
                .read()
                .unwrap()
                .get(entry_id)
                .cloned())
        }

        fn list_tax_reconciliation_entries(
            &self,
            _report_id: &str,
        ) -> Result<Vec<TaxReconciliationEntry>> {
            Ok(Vec::new())
        }
    }

    fn make_report(id: &str, status: TaxReportStatus) -> TaxYearReport {
        let now = Utc::now().naive_utc();
        TaxYearReport {
            id: id.to_string(),
            tax_year: 2025,
            jurisdiction: DEFAULT_TAX_JURISDICTION.to_string(),
            status,
            rule_pack_version: "FR-2025-securities-v1".to_string(),
            base_currency: "EUR".to_string(),
            generated_at: Some(now),
            finalized_at: None,
            assumptions_json: "{}".to_string(),
            summary_json: "{}".to_string(),
            parent_report_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_event(report_id: &str) -> TaxEvent {
        let now = Utc::now().naive_utc();
        TaxEvent {
            id: "event-1".to_string(),
            report_id: report_id.to_string(),
            event_type: TaxEventType::DividendReceived,
            category: CATEGORY_DIVIDENDS.to_string(),
            suggested_box: Some("2042-2DC".to_string()),
            account_id: "account-1".to_string(),
            asset_id: None,
            activity_id: Some("activity-1".to_string()),
            event_date: "2025-01-15".to_string(),
            amount_currency: "EUR".to_string(),
            amount_local: Some(Decimal::ONE),
            amount_eur: Some(Decimal::ONE),
            taxable_amount_eur: Some(Decimal::ONE),
            expenses_eur: None,
            confidence: TaxConfidence::High,
            included: true,
            notes: None,
            user_override: false,
            sources: Vec::new(),
            lot_allocations: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_issue(report_id: &str, severity: &str) -> TaxIssue {
        TaxIssue {
            id: format!("issue-{severity}"),
            report_id: report_id.to_string(),
            severity: severity.to_string(),
            code: "TEST".to_string(),
            message: "test issue".to_string(),
            document_id: None,
            account_id: None,
            activity_id: None,
            tax_event_id: None,
            resolved_at: None,
            created_at: Utc::now().naive_utc(),
        }
    }

    fn make_document(report_id: &str) -> TaxDocument {
        let now = Utc::now().naive_utc();
        TaxDocument {
            id: "document-1".to_string(),
            report_id: report_id.to_string(),
            document_type: "IFU".to_string(),
            filename: "ifu.pdf".to_string(),
            mime_type: Some("application/pdf".to_string()),
            sha256: "hash".to_string(),
            size_bytes: 128,
            uploaded_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_text_document(report_id: &str) -> TaxDocument {
        let mut document = make_document(report_id);
        document.filename = "ifu.txt".to_string();
        document.mime_type = Some("text/plain".to_string());
        document
    }

    fn make_extraction() -> crate::tax::TaxDocumentExtraction {
        let now = Utc::now().naive_utc();
        crate::tax::TaxDocumentExtraction {
            id: "extraction-1".to_string(),
            document_id: "document-1".to_string(),
            method: "LOCAL_TEXT".to_string(),
            status: "READY_FOR_REVIEW".to_string(),
            consent_granted: false,
            raw_text_preview: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_field() -> ExtractedTaxField {
        let now = Utc::now().naive_utc();
        ExtractedTaxField {
            id: "field-1".to_string(),
            extraction_id: "extraction-1".to_string(),
            field_key: CATEGORY_DIVIDENDS.to_string(),
            label: "Dividends".to_string(),
            mapped_category: Some(CATEGORY_DIVIDENDS.to_string()),
            suggested_declaration_box: Some("2042-2DC".to_string()),
            source_locator_json: Some(
                json!({ "lineNumber": 1, "snippet": "Dividends 10" }).to_string(),
            ),
            value_text: Some("Dividends 10".to_string()),
            amount_eur: Some(Decimal::from(10u32)),
            confidence: 0.9,
            status: "SUGGESTED".to_string(),
            confirmed_amount_eur: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_reconciliation_entry(report_id: &str) -> TaxReconciliationEntry {
        let now = Utc::now().naive_utc();
        TaxReconciliationEntry {
            id: "reconciliation-1".to_string(),
            report_id: report_id.to_string(),
            category: CATEGORY_DIVIDENDS.to_string(),
            suggested_box: Some("2042-2DC".to_string()),
            app_amount_eur: Some(Decimal::from(10u32)),
            document_amount_eur: Some(Decimal::from(10u32)),
            selected_amount_eur: Some(Decimal::from(10u32)),
            delta_eur: Some(Decimal::ZERO),
            status: "MATCHED".to_string(),
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_compiled_dividend_event() -> CompiledTaxEvent {
        CompiledTaxEvent {
            event: NewTaxEvent {
                event_type: TaxEventType::DividendReceived,
                category: CATEGORY_DIVIDENDS.to_string(),
                suggested_box: Some("2042-2DC".to_string()),
                account_id: "account-1".to_string(),
                asset_id: None,
                activity_id: Some("activity-1".to_string()),
                event_date: "2025-01-15".to_string(),
                amount_currency: "EUR".to_string(),
                amount_local: Some(Decimal::ONE),
                amount_eur: Some(Decimal::ONE),
                taxable_amount_eur: Some(Decimal::ONE),
                expenses_eur: None,
                confidence: TaxConfidence::High,
                included: true,
                notes: None,
            },
            sources: Vec::new(),
            lot_allocations: Vec::new(),
        }
    }

    fn service_with_repo(repo: Arc<MockTaxRepository>) -> TaxService<MockTaxRepository> {
        TaxService::new(repo, Arc::new(MockActivityService))
    }

    #[tokio::test]
    async fn update_tax_event_rejects_finalized_report() {
        let report = make_report("report-1", TaxReportStatus::Finalized);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_event(make_event(&report.id));
        let service = service_with_repo(repo);

        let err = service
            .update_tax_event(TaxEventUpdate {
                id: "event-1".to_string(),
                included: true,
                taxable_amount_eur: Some(Decimal::ONE),
                notes: None,
            })
            .await
            .expect_err("finalized report should reject event edits");

        assert!(err.to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn update_extracted_tax_field_rejects_finalized_report() {
        let report = make_report("report-1", TaxReportStatus::Finalized);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_document(make_document(&report.id));
        repo.add_extraction(make_extraction());
        repo.add_field(make_field());
        let service = service_with_repo(repo);

        let err = service
            .update_extracted_tax_field(ExtractedTaxFieldUpdate {
                field_id: "field-1".to_string(),
                status: "CONFIRMED".to_string(),
                confirmed_amount_eur: Some(Decimal::TEN),
            })
            .await
            .expect_err("finalized report should reject extracted field edits");

        assert!(err.to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn reconcile_rejects_finalized_report() {
        let report = make_report("report-1", TaxReportStatus::Finalized);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_reconciliation_entry(make_reconciliation_entry(&report.id));
        let service = service_with_repo(repo);

        let err = service
            .reconcile_tax_year_report(&report.id)
            .await
            .expect_err("finalized report should reject reconciliation regeneration");

        assert!(err.to_string().contains("read-only"));
    }

    #[tokio::test]
    async fn finalize_rejects_reports_with_blocking_issues() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_issue(make_issue(&report.id, "ERROR"));
        let service = service_with_repo(repo);

        let err = service
            .finalize_tax_year_report(&report.id)
            .await
            .expect_err("blocking issues should prevent finalization");

        assert!(err.to_string().contains("blocking issues"));
    }

    #[tokio::test]
    async fn amend_creates_amended_draft_from_finalized_report() {
        let report = make_report("report-1", TaxReportStatus::Finalized);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        let service = service_with_repo(repo.clone());

        let amended = service
            .amend_tax_year_report(&report.id)
            .await
            .expect("finalized report should allow amendment");

        assert_eq!(amended.status, TaxReportStatus::AmendedDraft);
        assert_eq!(
            amended.parent_report_id.as_deref(),
            Some(report.id.as_str())
        );
        assert_eq!(repo.amended_from.lock().unwrap().as_slice(), &[report.id]);
    }

    #[test]
    fn reconciliation_ignores_suggested_fields_until_confirmed() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report));
        let service = service_with_repo(repo);

        let entries = service.build_reconciliation(
            "report-1",
            &[make_compiled_dividend_event()],
            &[make_field()],
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].app_amount_eur, Some(Decimal::ONE));
        assert_eq!(entries[0].document_amount_eur, None);
        assert_eq!(entries[0].selected_amount_eur, Some(Decimal::ONE));
        assert_eq!(entries[0].status, "APP_ONLY");
    }

    #[tokio::test]
    async fn update_reconciliation_entry_uses_app_amount_for_app_selection() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_reconciliation_entry(make_reconciliation_entry(&report.id));
        let service = service_with_repo(repo.clone());

        let entry = service
            .update_tax_reconciliation_entry(TaxReconciliationEntryUpdate {
                id: "reconciliation-1".to_string(),
                selected_amount_eur: None,
                status: "USER_SELECTED_APP".to_string(),
                notes: None,
            })
            .await
            .expect("app selection should derive selected amount from app total");

        assert_eq!(entry.selected_amount_eur, Some(Decimal::from(10u32)));
        assert_eq!(entry.status, "USER_SELECTED_APP");
    }

    #[tokio::test]
    async fn update_reconciliation_entry_requires_reason_for_manual_override() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_reconciliation_entry(make_reconciliation_entry(&report.id));
        let service = service_with_repo(repo);

        let err = service
            .update_tax_reconciliation_entry(TaxReconciliationEntryUpdate {
                id: "reconciliation-1".to_string(),
                selected_amount_eur: Some(Decimal::from(12u32)),
                status: "USER_OVERRIDE".to_string(),
                notes: Some("   ".to_string()),
            })
            .await
            .expect_err("manual override should require a reason");

        assert!(err.to_string().contains("require a reason"));
    }

    #[test]
    fn normalize_extraction_method_maps_legacy_local_method() {
        assert_eq!(
            TaxService::<MockTaxRepository>::normalize_extraction_method("LOCAL_HEURISTIC")
                .expect("legacy method should be accepted"),
            "LOCAL_TEXT"
        );
    }

    #[test]
    fn extract_local_document_text_reads_plain_text_documents() {
        let text = TaxService::<MockTaxRepository>::extract_local_document_text(
            b"Dividends 123.45\nInterest 67.89",
            Some("text/plain"),
            "ifu.txt",
        )
        .expect("plain text extraction should succeed");

        assert!(text.contains("Dividends 123.45"));
        assert!(text.contains("Interest 67.89"));
    }

    #[test]
    fn build_extraction_issues_warns_when_no_fields_are_found() {
        let document = make_document("report-1");
        let issues =
            TaxService::<MockTaxRepository>::build_extraction_issues(&document, "LOCAL_TEXT", &[]);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, "WARNING");
        assert!(issues[0].code.starts_with("DOCUMENT_EXTRACTION_EMPTY:"));
    }

    #[test]
    fn extraction_status_marks_empty_extractions() {
        assert_eq!(
            TaxService::<MockTaxRepository>::extraction_status(&[]),
            "NO_FIELDS_FOUND"
        );
    }

    #[test]
    fn build_extraction_issues_warns_when_all_fields_are_low_confidence() {
        let document = make_document("report-1");
        let issues = TaxService::<MockTaxRepository>::build_extraction_issues(
            &document,
            "LOCAL_TEXT",
            &[NewExtractedTaxField {
                field_key: CATEGORY_DIVIDENDS.to_string(),
                label: "Dividends".to_string(),
                mapped_category: Some(CATEGORY_DIVIDENDS.to_string()),
                suggested_declaration_box: Some("2042-2DC".to_string()),
                source_locator_json: None,
                value_text: Some("Dividends 10".to_string()),
                amount_eur: Some(Decimal::from(10u32)),
                confidence: 0.55,
                status: "SUGGESTED".to_string(),
                confirmed_amount_eur: None,
            }],
        );

        assert_eq!(issues.len(), 1);
        assert!(issues[0]
            .code
            .starts_with("DOCUMENT_EXTRACTION_LOW_CONFIDENCE:"));
    }

    #[test]
    fn extraction_status_marks_low_confidence_results() {
        assert_eq!(
            TaxService::<MockTaxRepository>::extraction_status(&[NewExtractedTaxField {
                field_key: CATEGORY_DIVIDENDS.to_string(),
                label: "Dividends".to_string(),
                mapped_category: Some(CATEGORY_DIVIDENDS.to_string()),
                suggested_declaration_box: Some("2042-2DC".to_string()),
                source_locator_json: None,
                value_text: Some("Dividends 10".to_string()),
                amount_eur: Some(Decimal::from(10u32)),
                confidence: 0.55,
                status: "SUGGESTED".to_string(),
                confirmed_amount_eur: None,
            }]),
            "LOW_CONFIDENCE"
        );
    }

    #[test]
    fn extraction_status_marks_review_ready_results() {
        assert_eq!(
            TaxService::<MockTaxRepository>::extraction_status(&[NewExtractedTaxField {
                field_key: CATEGORY_DIVIDENDS.to_string(),
                label: "Dividends".to_string(),
                mapped_category: Some(CATEGORY_DIVIDENDS.to_string()),
                suggested_declaration_box: Some("2042-2DC".to_string()),
                source_locator_json: None,
                value_text: Some("Dividends 10".to_string()),
                amount_eur: Some(Decimal::from(10u32)),
                confidence: 0.9,
                status: "SUGGESTED".to_string(),
                confirmed_amount_eur: None,
            }]),
            "READY_FOR_REVIEW"
        );
    }

    #[test]
    fn parse_ifu_fields_detects_foreign_withholding_tax_lines() {
        let fields =
            TaxService::<MockTaxRepository>::parse_ifu_fields("Foreign tax withholding 12.34");

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_key, CATEGORY_FOREIGN_WITHHOLDING_TAX);
        assert_eq!(fields[0].suggested_declaration_box.as_deref(), Some("2047"));
    }

    #[tokio::test]
    async fn cloud_extraction_requires_configured_extractor() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_document(make_text_document(&report.id));
        let service = service_with_repo(repo);

        let err = service
            .extract_tax_document(TaxDocumentExtractionRequest {
                document_id: "document-1".to_string(),
                method: "CLOUD_AI".to_string(),
                consent_granted: true,
            })
            .await
            .expect_err("cloud extraction should require configured extractor");

        assert!(err.to_string().contains("unavailable"));
    }

    #[tokio::test]
    async fn cloud_extraction_uses_cloud_extractor_when_configured() {
        let report = make_report("report-1", TaxReportStatus::Draft);
        let repo = Arc::new(MockTaxRepository::new(report.clone()));
        repo.add_document(make_text_document(&report.id));
        let extractor = Arc::new(MockCloudExtractor {
            called: AtomicBool::new(false),
            fields: vec![NewExtractedTaxField {
                field_key: CATEGORY_DIVIDENDS.to_string(),
                label: "Dividends".to_string(),
                mapped_category: Some(CATEGORY_DIVIDENDS.to_string()),
                suggested_declaration_box: Some("2042-2DC".to_string()),
                source_locator_json: None,
                value_text: Some("Cloud extracted dividends".to_string()),
                amount_eur: Some(Decimal::from(25u32)),
                confidence: 0.9,
                status: "SUGGESTED".to_string(),
                confirmed_amount_eur: None,
            }],
        });
        let service = TaxService::new(repo, Arc::new(MockActivityService))
            .with_cloud_extractor(extractor.clone());

        let result = service
            .extract_tax_document(TaxDocumentExtractionRequest {
                document_id: "document-1".to_string(),
                method: "CLOUD_AI".to_string(),
                consent_granted: true,
            })
            .await
            .expect("cloud extraction should succeed with configured extractor");

        assert_eq!(result.extraction.method, "CLOUD_AI");
        assert_eq!(result.fields.len(), 1);
        assert!(extractor.called.load(Ordering::Relaxed));
    }

    #[test]
    fn test_nombre_parts_celibataire_no_children() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("CELIBATAIRE", 0, 0, false, false), 1.0);
    }

    #[test]
    fn test_nombre_parts_marie_no_children() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("MARIE", 0, 0, false, false), 2.0);
    }

    #[test]
    fn test_nombre_parts_pacse_no_children() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("PACSE", 0, 0, false, false), 2.0);
    }

    #[test]
    fn test_nombre_parts_celibataire_one_child() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("CELIBATAIRE", 1, 0, false, false), 1.5);
    }

    #[test]
    fn test_nombre_parts_marie_two_children() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("MARIE", 2, 0, false, false), 3.0);
    }

    #[test]
    fn test_nombre_parts_marie_three_children() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("MARIE", 3, 0, false, false), 4.0);
    }

    #[test]
    fn test_nombre_parts_parent_isole_with_children() {
        use crate::tax::compute_nombre_parts;
        // celibataire(1) + 1 child(0.5) + parent isolé(0.5) = 2.0
        assert_eq!(compute_nombre_parts("CELIBATAIRE", 1, 0, true, false), 2.0);
    }

    #[test]
    fn test_nombre_parts_parent_isole_no_children_ignored() {
        use crate::tax::compute_nombre_parts;
        // parent isolé bonus only applies when there are children
        assert_eq!(compute_nombre_parts("CELIBATAIRE", 0, 0, true, false), 1.0);
    }

    #[test]
    fn test_nombre_parts_handicapped_children() {
        use crate::tax::compute_nombre_parts;
        // marie(2) + 1 normal child + 1 handicapped = 2 total children(1.0) + handicap bonus(0.5) = 3.5
        assert_eq!(compute_nombre_parts("MARIE", 1, 1, false, false), 3.5);
    }

    #[test]
    fn test_nombre_parts_ancien_combattant() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("CELIBATAIRE", 0, 0, false, true), 1.5);
    }

    #[test]
    fn test_nombre_parts_all_bonuses() {
        use crate::tax::compute_nombre_parts;
        // marie(2) + 3 children(2 normal + 1 handicapped = 3 total -> 1.0 + 1.0 = 2.0)
        // + handicap bonus(0.5) + parent isolé ignored (married) + ancien combattant(0.5)
        // parent_isole doesn't apply for MARIE, but the function doesn't check marital status for it
        // Actually: parent_isole is about having children, function checks total_children > 0
        // marie(2) + 3 children(2.0) + handicap(0.5) + parent_isole(0.5) + combattant(0.5) = 5.5
        assert_eq!(compute_nombre_parts("MARIE", 2, 1, true, true), 5.5);
    }

    #[test]
    fn test_nombre_parts_veuf() {
        use crate::tax::compute_nombre_parts;
        assert_eq!(compute_nombre_parts("VEUF", 0, 0, false, false), 1.0);
    }

    #[test]
    fn test_nombre_parts_divorce_with_children() {
        use crate::tax::compute_nombre_parts;
        // divorcé(1) + 2 children(1.0) = 2.0
        assert_eq!(compute_nombre_parts("DIVORCE", 2, 0, false, false), 2.0);
    }

    #[test]
    fn parse_fiche_fields_detects_net_imposable() {
        let fields =
            TaxService::<MockTaxRepository>::parse_fiche_fields("Net imposable cumulé 35000,00");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_key, "NET_IMPOSABLE");
        assert_eq!(fields[0].suggested_declaration_box.as_deref(), Some("1AJ"));
        assert_eq!(
            fields[0].mapped_category.as_deref(),
            Some(CATEGORY_SALARY_INCOME)
        );
        assert_eq!(
            fields[0].amount_eur,
            Some("35000.00".parse::<Decimal>().unwrap())
        );
    }

    #[test]
    fn parse_fiche_fields_detects_csg_deductible() {
        let fields = TaxService::<MockTaxRepository>::parse_fiche_fields("CSG déductible 2380.00");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_key, "CSG_DEDUCTIBLE");
        assert!(fields[0].suggested_declaration_box.is_none());
    }

    #[test]
    fn parse_fiche_fields_detects_heures_sup() {
        let fields =
            TaxService::<MockTaxRepository>::parse_fiche_fields("Heures supplémentaires 1200.00");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_key, "HEURES_SUP");
        assert_eq!(fields[0].suggested_declaration_box.as_deref(), Some("1GH"));
    }

    #[test]
    fn parse_fiche_fields_no_match_returns_empty() {
        let fields = TaxService::<MockTaxRepository>::parse_fiche_fields(
            "Salaire brut 5000.00\nCotisations 1000.00",
        );
        assert!(fields.is_empty());
    }

    #[test]
    fn compile_salary_events_uses_highest_net_imposable() {
        let report = make_report("r1", TaxReportStatus::Draft);
        // Simulate three confirmed NET_IMPOSABLE fields (Jan, Jun, Dec cumuls)
        let fields: Vec<ExtractedTaxField> = vec![
            make_confirmed_field("NET_IMPOSABLE", Decimal::from(10000u32)),
            make_confirmed_field("NET_IMPOSABLE", Decimal::from(20000u32)),
            make_confirmed_field("NET_IMPOSABLE", Decimal::from(35000u32)),
        ];

        let events = TaxService::<MockTaxRepository>::compile_salary_events(&report, &fields);

        assert_eq!(events.len(), 1);
        let event = &events[0].event;
        assert_eq!(event.event_type, TaxEventType::SalaryIncome);
        assert_eq!(event.taxable_amount_eur, Some(Decimal::from(35000u32)));
        assert_eq!(event.suggested_box.as_deref(), Some("1AJ"));
        assert_eq!(event.account_id, SALARY_VIRTUAL_ACCOUNT_ID);
    }

    #[test]
    fn compile_salary_events_empty_when_no_confirmed_fields() {
        let report = make_report("r1", TaxReportStatus::Draft);
        let fields: Vec<ExtractedTaxField> = vec![make_suggested_field(
            "NET_IMPOSABLE",
            Decimal::from(35000u32),
        )];

        let events = TaxService::<MockTaxRepository>::compile_salary_events(&report, &fields);
        assert!(events.is_empty());
    }

    fn make_confirmed_field(key: &str, amount: Decimal) -> ExtractedTaxField {
        use chrono::NaiveDateTime;
        ExtractedTaxField {
            id: uuid::Uuid::new_v4().to_string(),
            extraction_id: "extraction-1".to_string(),
            field_key: key.to_string(),
            label: key.to_string(),
            mapped_category: Some(CATEGORY_SALARY_INCOME.to_string()),
            suggested_declaration_box: None,
            source_locator_json: None,
            value_text: None,
            amount_eur: Some(amount),
            confidence: 0.9,
            status: "CONFIRMED".to_string(),
            confirmed_amount_eur: None,
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
        }
    }

    fn make_suggested_field(key: &str, amount: Decimal) -> ExtractedTaxField {
        ExtractedTaxField {
            status: "SUGGESTED".to_string(),
            ..make_confirmed_field(key, amount)
        }
    }
}
