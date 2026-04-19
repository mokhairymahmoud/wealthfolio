# Plan: France Tax Declaration Assistant - Phase 1

> Source PRD: grill session on France-first tax declaration support for Wealthfolio.

## Architectural Decisions

Durable decisions that apply across this phase:

- **Product posture**: tax declaration assistant, not official filing software or legal/tax advice.
- **User scope**: one local French tax-resident individual. No multi-client workspace in phase 1.
- **Route**: add a primary workspace at `/taxes`, with a default current-year report view.
- **Tax year model**: draft reports recalculate; finalized reports snapshot rule version, source document hashes, FX assumptions, events, reconciliation values, and user overrides.
- **Jurisdiction and rules**: first rule pack is France-only, versioned as `FR-<tax-year>-securities-v1`.
- **Phase 1 account scope**: one CTO account path, while storing account tax regime metadata in a way that can later add PEA.
- **Ledger shape**: generate report-scoped tax events from existing activities. Events must link back to source activity IDs and, for realized gains, acquisition/disposal lot allocations.
- **Document posture**: IFU/releve fiscal PDFs are stored locally encrypted. Extraction values are never used in totals until reviewed or explicitly accepted.
- **Extraction posture**: local text extraction first. Cloud AI extraction only after explicit user consent per document.
- **Currency**: report totals in EUR. Prefer IFU EUR values for reconciliation; use activity FX rates or stored exchange rates for app-calculated EUR amounts.
- **Shared access pattern**: expose every operation through Tauri IPC and Axum HTTP, then call both through existing frontend adapter `invoke` wrappers.

---

## Data Model

Add a new tax domain rather than overloading generic activity metadata.

### Core Entities

- `TaxProfile`
  - Stores France-specific defaults for this local user.
  - Phase 1 fields: `jurisdiction`, `taxResidenceCountry`, `defaultTaxRegime`, `pfoOrBaremePreference`, `createdAt`, `updatedAt`.
  - Keep loss carryforwards and PEA details out of phase 1 unless required by the CTO flow.

- `AccountTaxProfile`
  - Links existing accounts to a tax regime.
  - Fields: `accountId`, `jurisdiction`, `regime`, `openedOn`, `closedOn`, `metadata`.
  - Phase 1 supports `CTO`; schema should allow `PEA` later.

- `TaxYearReport`
  - One report per tax year and jurisdiction.
  - Fields: `id`, `taxYear`, `jurisdiction`, `status`, `rulePackVersion`, `baseCurrency`, `generatedAt`, `finalizedAt`, `assumptionsJson`, `summaryJson`.
  - Status values: `DRAFT`, `FINALIZED`, `AMENDED_DRAFT`.

- `TaxEvent`
  - Report-scoped normalized tax event.
  - Fields: `id`, `reportId`, `eventType`, `domain`, `accountId`, `accountRegime`, `assetId`, `eventDate`, `amountLocal`, `currency`, `amountEur`, `fxRate`, `fxSource`, `confidence`, `inclusionStatus`, `calculationJson`.
  - Phase 1 event types: `DIVIDEND_RECEIVED`, `INTEREST_RECEIVED`, `SECURITY_DISPOSAL`, `FEE_PAID`, `FOREIGN_WITHHOLDING_TAX`.
  - Inclusion statuses: `INCLUDED`, `EXCLUDED_NEEDS_REVIEW`, `UNSUPPORTED`.

- `TaxEventSource`
  - Traceability from tax events to source activities.
  - Fields: `taxEventId`, `activityId`, `role`, `amountJson`.
  - Roles include `INCOME`, `DISPOSAL`, `ACQUISITION`, `FEE`, `WITHHOLDING`.

- `TaxLotAllocation`
  - Lot-level traceability for realized gains.
  - Fields: `taxEventId`, `acquisitionActivityId`, `disposalActivityId`, `quantity`, `costBasisEur`, `proceedsEur`, `gainLossEur`, `method`, `calculationJson`.
  - Method is rule-pack determined, not user preference.

- `TaxIssue`
  - Blocking and non-blocking report issues.
  - Fields: `id`, `reportId`, `taxEventId`, `documentId`, `severity`, `code`, `message`, `blocking`, `status`, `resolutionJson`.
  - Phase 1 examples: missing cost basis, missing FX, unconfirmed IFU field, unsupported account regime.

- `TaxDocument`
  - Metadata for encrypted IFU/source PDFs.
  - Fields: `id`, `reportId`, `documentType`, `brokerName`, `taxYear`, `originalFilename`, `mimeType`, `sha256`, `encryptedBlobPath`, `encryptionKeyRef`, `uploadedAt`.
  - Store encrypted files outside SQLite in the app/server data directory; keep metadata in SQLite.

- `TaxDocumentExtraction`
  - One extraction attempt per document.
  - Fields: `id`, `documentId`, `method`, `status`, `consentAt`, `rawTextHash`, `extractedJson`, `confidence`, `createdAt`.
  - Methods: `LOCAL_TEXT`, `CLOUD_AI`.

- `ExtractedTaxField`
  - User-reviewable values extracted from an IFU.
  - Fields: `id`, `extractionId`, `fieldKey`, `label`, `mappedCategory`, `suggestedDeclarationBox`, `amount`, `currency`, `confidence`, `sourceLocatorJson`, `status`, `confirmedAmount`, `confirmedAt`.
  - Status values: `SUGGESTED`, `CONFIRMED`, `CORRECTED`, `REJECTED`.

- `TaxReconciliationEntry`
  - Compares app-calculated values with confirmed IFU values.
  - Fields: `id`, `reportId`, `category`, `suggestedDeclarationBox`, `appAmountEur`, `documentAmountEur`, `selectedAmountEur`, `deltaEur`, `status`, `overrideReason`, `updatedAt`.
  - Status values: `MATCHED`, `DELTA_REVIEW`, `USER_SELECTED_APP`, `USER_SELECTED_DOCUMENT`, `USER_OVERRIDE`.

---

## Service Boundaries

Use existing architecture: frontend -> adapter -> Tauri/Axum -> core services -> storage repositories.

- `TaxProfileService`
  - Reads and updates the local French tax profile.
  - Reads and updates account tax regime metadata.

- `TaxReportService`
  - Creates, regenerates, reads, finalizes, and amends tax year reports.
  - Coordinates event generation, issue generation, reconciliation, and snapshots.

- `TaxEventCompiler`
  - Converts existing activities into tax events for the selected tax year/account scope.
  - Phase 1 supports CTO dividends, interest, fees, and security disposals.
  - Emits issues instead of silently calculating through missing acquisition cost, missing FX, or unsupported activity shapes.

- `FranceTaxRulePack`
  - Deterministic rule-pack interface for France.
  - Owns supported event types, lot method policy, declaration category mapping, and issue rules.
  - Phase 1 can be compiled Rust/config, but the report stores a stable rule-pack version string.

- `TaxDocumentService`
  - Stores encrypted documents, reads metadata, deletes documents, and links documents to reports.
  - Uses existing secret-store direction for document encryption key material and existing XChaCha20-Poly1305 crypto primitives where practical.

- `TaxDocumentExtractionService`
  - Runs local text extraction first.
  - Records explicit consent before cloud AI extraction.
  - Produces extracted fields with confidence and source location metadata.

- `TaxReconciliationService`
  - Builds reconciliation entries from tax events and confirmed extracted fields.
  - Stores selected final value and override reason.

---

## API Surface

Expose equivalent Tauri commands and Axum HTTP endpoints.

- `get_tax_profile`
- `update_tax_profile`
- `get_account_tax_profiles`
- `update_account_tax_profile`
- `create_tax_report`
- `get_tax_report`
- `list_tax_reports`
- `regenerate_tax_report`
- `finalize_tax_report`
- `amend_tax_report`
- `upload_tax_document`
- `list_tax_documents`
- `delete_tax_document`
- `extract_tax_document`
- `confirm_extracted_tax_field`
- `reject_extracted_tax_field`
- `get_tax_reconciliation`
- `update_tax_reconciliation_entry`

HTTP paths should live under `/api/v1/taxes`, for example:

- `GET /taxes/profile`
- `PUT /taxes/profile`
- `POST /taxes/reports`
- `GET /taxes/reports/{id}`
- `POST /taxes/reports/{id}/regenerate`
- `POST /taxes/reports/{id}/finalize`
- `POST /taxes/reports/{id}/documents`
- `POST /taxes/documents/{id}/extract`
- `PUT /taxes/extracted-fields/{id}`
- `GET /taxes/reports/{id}/reconciliation`
- `PUT /taxes/reconciliation/{id}`

---

## Frontend Workspace

Create `/taxes` as a usable tax workspace, not a marketing page.

Expected screen regions:

- **Tax year selector**: current year by default, with report status.
- **Assumptions strip**: jurisdiction, rule pack, base currency, account regime, PFU/barème preference.
- **Summary cards**: taxable income, realized gains/losses, withholding tax, needs-review count.
- **Issue list**: blocking first, linked to event/document/source activity.
- **IFU document panel**: uploaded documents, extraction status, confidence, review actions.
- **Extraction review table**: extracted field, source, confidence, corrected/confirmed amount.
- **Reconciliation table**: app amount, IFU amount, delta, selected final value, status.
- **Declaration helper section**: likely declaration categories/boxes with caveats and drill-down.
- **Event ledger table**: normalized tax events with source activity links and inclusion status.

The first UI should be functional for the vertical slice:

1. Pick/create a tax year report.
2. Mark one account as CTO if needed.
3. Upload an IFU PDF.
4. Review extracted fields.
5. Regenerate/reconcile.
6. See likely declaration helper values and unresolved issues.

---

## Phase 1.1: Report Shell and CTO Account Regime

**User stories covered**: create a France tax report, make tax assumptions explicit, classify the source account as CTO.

### What to build

Create the `/taxes` workspace and the minimum backend path to create/read a draft France tax year report. Add account tax profile storage so the report can constrain phase 1 to CTO accounts without relying on free-form account names.

### Acceptance Criteria

- [ ] User can open `/taxes` from main navigation.
- [ ] User can create or load a draft report for a selected tax year.
- [ ] Report stores `jurisdiction = FR`, `baseCurrency = EUR`, and a rule-pack version.
- [ ] User can mark an existing securities account as `CTO` for tax purposes.
- [ ] Report shows assumptions and empty-state sections for issues, documents, reconciliation, and declaration helper.
- [ ] Tauri and web mode expose the same report/profile operations.

---

## Phase 1.2: Securities Tax Event Ledger

**User stories covered**: calculate app-side tax events from existing activities with traceability and review issues.

### What to build

Generate report-scoped tax events for one CTO account and one tax year from existing activities. Include dividends, interest, fees, and realized security disposals. Link every event back to source activities. For disposals, record lot allocation details. Emit blocking issues for missing cost basis, missing FX, unsupported activity shape, or non-CTO account.

### Acceptance Criteria

- [ ] Regenerating a draft report replaces previous generated draft events for that report.
- [ ] Dividend and interest activities become EUR tax events with source activity links.
- [ ] Sell activities generate disposal events with proceeds, cost basis, fees, and gain/loss in EUR when data is complete.
- [ ] Missing cost basis or FX produces `TaxIssue` records and excludes unsafe values from totals.
- [ ] Event ledger table displays included, excluded-needs-review, and unsupported statuses.
- [ ] Core Rust tests cover dividend, interest, fee, realized gain, realized loss, missing cost basis, and missing FX cases.

---

## Phase 1.3: Encrypted IFU Document Upload

**User stories covered**: store IFU/source PDFs locally encrypted and attach them to a tax report.

### What to build

Add report document upload and encrypted local storage. Store document metadata and content hash in SQLite; store encrypted bytes in the app/server data directory. Keep the encryption key reference out of ordinary report data.

### Acceptance Criteria

- [ ] User can upload a PDF to a draft tax report.
- [ ] Document bytes are encrypted before being written to disk.
- [ ] Document metadata includes filename, MIME type, SHA-256 hash, broker name if supplied, tax year, and upload timestamp.
- [ ] User can list and delete documents attached to the report.
- [ ] Deleting a document removes its encrypted blob and related extraction/reconciliation records.
- [ ] Tests verify encryption round trip, hash persistence, and delete cleanup.

---

## Phase 1.4: IFU Extraction Review

**User stories covered**: AI-assisted IFU extraction with explicit consent and user confirmation.

### What to build

Run local text extraction first. If local extraction cannot confidently map fields, allow cloud AI extraction only after explicit consent. Store extraction attempts and extracted fields. Add a review table where users confirm, correct, or reject extracted values before reconciliation.

### Acceptance Criteria

- [ ] Extraction starts with local text extraction.
- [ ] Cloud AI extraction is blocked until the user grants consent for that document.
- [ ] Extracted fields include confidence, mapped tax category, suggested declaration box when available, and source locator metadata.
- [ ] Extracted values are not used in reconciliation until confirmed or corrected.
- [ ] User can confirm, correct, and reject extracted fields.
- [ ] Tests cover extraction state transitions and the rule that unconfirmed fields are ignored.

---

## Phase 1.5: IFU Reconciliation and Declaration Helper

**User stories covered**: compare app-calculated values with IFU values and produce likely declaration helper output.

### What to build

Aggregate tax events by France rule-pack category, compare those totals to confirmed IFU fields, and create reconciliation entries. Let the user select app value, IFU value, or an override with a reason. Surface likely declaration boxes with caveats and source drill-down.

### Acceptance Criteria

- [ ] Reconciliation table shows app amount, confirmed IFU amount, delta, selected final amount, and status.
- [ ] Matching values are marked `MATCHED`; deltas are marked `DELTA_REVIEW`.
- [ ] User can select app value, IFU value, or manual override with a required reason.
- [ ] Declaration helper shows likely categories/boxes for phase 1 supported domains.
- [ ] Each declaration helper line drills down to reconciliation entry and source tax events.
- [ ] Unsupported or excluded events never contribute to selected final totals.

---

## Phase 1.6: Finalization Snapshot

**User stories covered**: preserve an auditable, reproducible report.

### What to build

Allow finalizing a report once blocking issues are resolved or explicitly acknowledged. Finalization snapshots rule version, assumptions, generated events, selected reconciliation values, source document hashes, and override history. After finalization, edits create an amended draft.

### Acceptance Criteria

- [ ] User can finalize a report from the tax workspace.
- [ ] Finalized reports are read-only in the UI.
- [ ] Regeneration is disabled for finalized reports.
- [ ] Source document hashes and rule-pack version remain visible.
- [ ] Amending a finalized report creates a new draft linked to the finalized report.
- [ ] Tests verify finalized report values do not change when source activities are later edited.

---

## Verification

Run targeted checks as each slice lands:

- `cargo test -p wealthfolio-core tax`
- `cargo test -p wealthfolio-storage-sqlite tax`
- `cargo test -p wealthfolio-device-sync crypto`
- `pnpm test -- tax`
- `pnpm type-check`
- `pnpm lint`

Before merging the full phase:

- `cargo test`
- `pnpm test`
- `pnpm check`

---

## Explicit Non-Goals for Phase 1

- No official form generation or e-filing.
- No multi-client/advisor workspace.
- No assurance-vie, PER, or corporate treasury tax.
- No full PEA tax consequences beyond storing schema-compatible account regime metadata.
- No crypto/options/alternative-asset calculations beyond appearing later as unsupported or needs-review events.
- No automatic use of unconfirmed AI-extracted values.
- No cloud upload of IFU documents without explicit consent.
