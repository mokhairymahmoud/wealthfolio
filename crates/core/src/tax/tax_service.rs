use async_trait::async_trait;
use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::activities::{
    Activity, ActivityServiceTrait, ACTIVITY_TYPE_BUY, ACTIVITY_TYPE_DIVIDEND, ACTIVITY_TYPE_FEE,
    ACTIVITY_TYPE_INTEREST, ACTIVITY_TYPE_SELL,
};
use crate::errors::{Error, Result, ValidationError};
use crate::tax::{
    AccountTaxProfile, AccountTaxProfileUpdate, CompiledTaxEvent, ExtractedTaxField,
    ExtractedTaxFieldUpdate, NewExtractedTaxField, NewTaxEvent, NewTaxEventSource, NewTaxIssue,
    NewTaxLotAllocation, NewTaxReconciliationEntry, NewTaxYearReport, TaxConfidence, TaxDocument,
    TaxDocumentExtractionRequest, TaxDocumentExtractionResult, TaxDocumentUpload, TaxEventType,
    TaxProfile, TaxProfileUpdate, TaxReconciliationEntry, TaxReconciliationEntryUpdate,
    TaxReportDetail, TaxReportStatus, TaxRepositoryTrait, TaxServiceTrait, TaxYearReport,
    DEFAULT_TAX_JURISDICTION, DEFAULT_TAX_REGIME,
};

const TAX_REGIME_CTO: &str = "CTO";
const CATEGORY_DIVIDENDS: &str = "DIVIDENDS";
const CATEGORY_INTEREST: &str = "INTEREST";
const CATEGORY_SECURITY_GAINS: &str = "SECURITY_GAINS";
const CATEGORY_FEES: &str = "FEES";

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
}

impl<T: TaxRepositoryTrait> TaxService<T> {
    pub fn new(repository: Arc<T>, activity_service: Arc<dyn ActivityServiceTrait>) -> Self {
        Self {
            repository,
            activity_service,
        }
    }

    fn rule_pack_version(tax_year: i32, jurisdiction: &str) -> String {
        format!("{jurisdiction}-{tax_year}-securities-v1")
    }

    fn not_found(message: impl Into<String>) -> Error {
        Error::Validation(ValidationError::InvalidInput(message.into()))
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
            if !matches!(
                field.status.as_str(),
                "CONFIRMED" | "CORRECTED" | "SUGGESTED"
            ) {
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
            .filter_map(|line| {
                let lower = line.to_lowercase();
                let (category, label) = if lower.contains("dividend")
                    || lower.contains("dividende")
                    || lower.contains("dividendes")
                {
                    (CATEGORY_DIVIDENDS, "Dividends")
                } else if lower.contains("interest")
                    || lower.contains("intérêt")
                    || lower.contains("interet")
                    || lower.contains("intérêts")
                    || lower.contains("interets")
                {
                    (CATEGORY_INTEREST, "Interest")
                } else if lower.contains("plus-value")
                    || lower.contains("plus value")
                    || lower.contains("gain")
                    || lower.contains("cession")
                {
                    (CATEGORY_SECURITY_GAINS, "Security gains")
                } else if lower.contains("frais") || lower.contains("fee") {
                    (CATEGORY_FEES, "Fees")
                } else {
                    return None;
                };

                Self::parse_decimal_from_line(line).map(|amount| NewExtractedTaxField {
                    field_key: category.to_string(),
                    label: label.to_string(),
                    mapped_category: Some(category.to_string()),
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

        let compiled = self.compile_tax_events(&report)?;
        let extracted_fields = self.extracted_fields_for_report(id)?;
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
        self.repository.finalize_tax_year_report(id).await
    }

    async fn upload_tax_document(&self, upload: TaxDocumentUpload) -> Result<TaxDocument> {
        if upload.content.is_empty() {
            return Err(Error::Validation(ValidationError::MissingField(
                "content".to_string(),
            )));
        }
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

    async fn extract_tax_document(
        &self,
        request: TaxDocumentExtractionRequest,
    ) -> Result<TaxDocumentExtractionResult> {
        if request.method == "CLOUD_AI" && !request.consent_granted {
            return Err(Error::Validation(ValidationError::InvalidInput(
                "Cloud AI extraction requires explicit consent.".to_string(),
            )));
        }
        let content = self
            .repository
            .get_tax_document_content(&request.document_id)?
            .ok_or_else(|| {
                Self::not_found(format!("Tax document {} not found", request.document_id))
            })?;
        let text = String::from_utf8_lossy(&content).to_string();
        let preview: String = text.chars().take(4000).collect();
        let fields = Self::parse_ifu_fields(&preview);
        self.repository
            .create_tax_document_extraction(request, Some(preview), fields)
            .await
    }

    async fn update_extracted_tax_field(
        &self,
        update: ExtractedTaxFieldUpdate,
    ) -> Result<ExtractedTaxField> {
        self.repository.update_extracted_tax_field(update).await
    }

    async fn reconcile_tax_year_report(&self, id: &str) -> Result<Vec<TaxReconciliationEntry>> {
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
        self.repository
            .update_tax_reconciliation_entry(update)
            .await
    }
}
