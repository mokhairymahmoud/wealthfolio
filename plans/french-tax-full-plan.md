# Plan: French Tax Declaration — Full Implementation

> Source PRD: `docs/prd-french-tax-module.md`

## Architectural Decisions

Durable decisions that apply across all phases:

- **Route**: `/taxes` — existing route, extended incrementally
- **Schema**: extend existing tax tables (`tax_profiles`,
  `account_tax_profiles`, `tax_events`, etc.) with new columns/tables per phase.
  New tables for carry-forwards and tax classification.
- **Tax parameters**: versioned TOML config files (`tax_params_YYYY.toml`) for
  rates/thresholds/barème, Rust code for calculation logic. `rule_pack_version`
  on `TaxYearReport` pins to parameter set.
- **Document extraction**: LLM-only via `TaxCloudExtractionTrait`. Blanket
  consent toggle in `TaxProfile`. Per-document-type extraction schemas.
- **Document behavior split**: IFU = reconciliation mode (portfolio is source of
  truth). Fiche de paie / reçus de dons = direct-population mode (document is
  source of truth, user confirms).
- **Account regimes**: required `regime` field on investment accounts (CTO
  default). Tax engine uses `(regime, opened_date, event_date)` for treatment.
- **Tax classification**: `tax_category` field on activities for bank
  transaction categorization.
- **Foyer fiscal**: stored in `TaxProfile`, parts auto-computed from family
  situation.
- **Carry-forwards**: dedicated table, populated on report finalization,
  consumed by next year's engine.
- **Declaration output**: box-by-box summary per formulaire, drill-down to
  source data.
- **LLM optimization**: `TaxDeclarationContext` struct designed as serializable
  snapshot for both UI and AI consumption.

---

## Phase 1: Foyer Fiscal Profile ← DONE

**Status**: COMPLETE

**User stories**: User declares family situation (situation maritale, enfants à
charge, parent isolé, invalidité). System computes nombre de parts
automatically. This is prerequisite for barème calculation, PFU vs barème
simulation, and don ceilings.

### What to build

Extend `TaxProfile` with foyer fiscal fields. Add a form in the tax page for
family situation input. Auto-compute nombre de parts using French rules.
End-to-end: migration → model → service → API/commands → adapter → UI form.

### Acceptance criteria

- [x] `tax_profiles` table has new columns: `situation_familiale`,
      `nombre_enfants`, `nombre_enfants_handicapes`, `parent_isole`,
      `ancien_combattant_ou_invalidite`, `nombre_parts` (computed)
- [x] Rust model `TaxProfile` and `TaxProfileUpdate` include foyer fiscal fields
- [x] `TaxService::update_tax_profile` auto-computes `nombre_parts` from family
      inputs using French rules
- [x] Parts computation covers: célibataire (1), marié/pacsé (2), veuf (1),
      divorcé (1), +0.5 per child for first two, +1 from third child, +0.5 for
      parent isolé, +0.5 per handicapped child, +0.5 for ancien
      combattant/invalidité
- [x] Tauri command and Axum endpoint expose updated profile
- [x] Frontend adapter updated with new fields
- [x] Tax page shows foyer fiscal form section with all fields
- [x] Nombre de parts displayed as computed read-only value
- [x] Rust unit tests for parts computation edge cases
- [x] `pnpm type-check` and `cargo clippy` pass

---

## Phase 2: Tax Parameters Config

**Status**: TODO

**User stories**: Tax calculations use correct yearly rates. Historical reports
stay accurate after parameter updates.

### What to build

Versioned `tax_params_YYYY.toml` files with barème tranches, PFU rates, PS
rates, don ceilings, kilométrique barème, plafond QF. Tax service reads
parameters by year from config. `rule_pack_version` pins reports to a parameter
set.

### Acceptance criteria

- [ ] `tax_params_2025.toml` file with all French tax parameters for 2025
- [ ] Rust struct `TaxParameters` deserializable from TOML
- [ ] Tax service loads parameters by tax year
- [ ] Existing hardcoded values replaced with parameter lookups
- [ ] `rule_pack_version` format updated to reference parameter version
- [ ] Unit tests verify parameter loading and fallback behavior

---

## Phase 3: Fiche de Paie Extraction + Salary Income

**Status**: TODO

**User stories**: User uploads fiche de paie, salary data flows into declaration
automatically via LLM extraction.

### What to build

New document type `FICHE_DE_PAIE`. LLM extraction schema targeting: net
imposable cumulé, CSG déductible, heures sup exonérées. Direct-population mode:
extraction creates salary tax events (box 1AJ), user confirms. Monthly upload
tracking, only latest cumul used for declaration.

### Acceptance criteria

- [ ] `FICHE_DE_PAIE` document type supported in upload/extraction
- [ ] LLM extraction schema defined for fiche de paie fields
- [ ] Extracted salary data creates `TaxEvent` of new type `SALARY_INCOME` (box
      1AJ)
- [ ] User confirms/corrects extracted values before inclusion
- [ ] Multiple fiches per year tracked; only latest cumul annuel used for tax
      calculation
- [ ] UI shows fiche de paie upload section distinct from IFU section

---

## Phase 4: Declaration Summary

**Status**: TODO

**User stories**: User gets a printable box-by-box sheet to fill impots.gouv.fr
side-by-side.

### What to build

Box-by-box summary view organized by formulaire (2042, 2074, 2047). Each box
shows computed value with drill-down to source events/documents. Printable
layout. Summary backed by `TaxDeclarationContext` struct.

### Acceptance criteria

- [ ] `TaxDeclarationContext` struct aggregates all tax data into serializable
      snapshot
- [ ] Declaration summary UI organized by formulaire section
- [ ] Each box (1AJ, 2DC, 2TR, 3VG, etc.) shows value and drill-down
- [ ] Printable/exportable layout (print CSS or PDF generation)
- [ ] Summary updates when tax events or reconciliation change

---

## Phase 5: PFU vs Barème Simulation

**Status**: TODO

**User stories**: User sees which taxation mode saves them money, with concrete
savings amount.

### What to build

Dual computation engine using foyer fiscal parts + all income (salary +
investment). Barème progressif calculation with tranches, 40% dividend
abattement, CSG déductible. Side-by-side display with recommendation.

### Acceptance criteria

- [ ] Barème progressif calculation using tax parameters (tranches, plafond QF)
- [ ] PFU calculation (12.8% IR + 17.2% PS)
- [ ] 40% abattement on dividends applied in barème mode
- [ ] CSG déductible (6.8%) applied in barème mode
- [ ] Side-by-side UI: PFU total vs barème total, savings amount, recommendation
- [ ] Computation uses foyer fiscal nombre_parts for quotient familial
- [ ] Unit tests for various income/family scenarios

---

## Phase 6: Dons Module

**Status**: TODO

**User stories**: User uploads reçus fiscaux, system computes réduction d'impôt
per don type.

### What to build

New document type `RECU_FISCAL_DON`. LLM extraction for don amount and organisme
type. Don entries with recipient classification (75% aide aux personnes / 66%
intérêt général / 66% partis politiques). Réduction computation with ceilings.
Boxes 7UF, 7UD, 7UH.

### Acceptance criteria

- [ ] `RECU_FISCAL_DON` document type with LLM extraction schema
- [ ] Don entries with `recipient_type` classification
- [ ] 75% réduction for aide aux personnes (box 7UD) up to ceiling, then 66%
- [ ] 66% réduction for intérêt général (box 7UF) capped at 20% revenu imposable
- [ ] 66% réduction for partis politiques (box 7UH) with separate ceiling
- [ ] Don amounts included in declaration summary
- [ ] Excess dons flagged for carry-forward (feeds Phase 10)

---

## Phase 7: Frais Réels Module

**Status**: TODO

**User stories**: User declares professional expenses, system recommends frais
réels vs 10% abattement.

### What to build

Frais réels entries with category, amount, proof attachment. Kilométriques
calculator (distance × barème × jours). Per-category proof requirements.
Automatic comparison vs 10% abattement forfaitaire. Proof storage via
`TaxDocument` infrastructure.

### Acceptance criteria

- [ ] Frais réels entry creation with category (transport, repas, matériel,
      kilométriques, etc.)
- [ ] Kilométriques calculator using barème from tax parameters
- [ ] Proof attachment per entry (upload or bank transaction reference)
- [ ] Per-category proof requirements (subscription = bank record, purchase =
      upload, km = calculation)
- [ ] Auto-generated proof documents (Navigo summary, kilométriques calculation
      PDF)
- [ ] Comparison: frais réels total vs 10% abattement, recommendation with
      savings
- [ ] Box 1AK populated in declaration summary when frais réels chosen

---

## Phase 8: Account Regime Engine (PEA/PEE/Assurance-vie)

**Status**: TODO

**User stories**: User with PEA/PEE gets correct tax treatment on withdrawals.

### What to build

Regime-specific plus-value logic. For envelope accounts (PEA/PEE/assurance-vie):
ignore individual SELLs, generate tax events only on WITHDRAWAL. Plus-value via
prorata method. PEA 5-year rule. PEE life-event early release. Assurance-vie
8-year abattement.

### Acceptance criteria

- [ ] PEA regime: no tax events on internal trades, tax event on WITHDRAWAL
- [ ] PEA 5-year rule: tax-free (only 17.2% PS) after 5 years, fully taxed
      before
- [ ] PEE regime: tax-free after 5 years, specific early release cases
- [ ] Assurance-vie: 4,600€/9,200€ abattement after 8 years
- [ ] Prorata method for envelope withdrawal plus-value calculation
- [ ] Account regime selection in account settings
- [ ] Unit tests for each regime's edge cases

---

## Phase 9: Bank Transaction Tax Classification

**Status**: TODO

**User stories**: Bank transactions auto-tagged for tax purposes, user confirms
ambiguous ones.

### What to build

`tax_category` field on activities. Rule-based auto-classification engine using
keyword matching on transaction descriptions, merchant patterns, recurring
amount detection. User review workflow for ambiguous transactions. Feeds frais
réels and dons candidate detection.

### Acceptance criteria

- [ ] `tax_category` column added to activities table (nullable)
- [ ] Classification rules engine (keyword patterns, merchant matching,
      recurring detection)
- [ ] Auto-classification runs on new/synced bank transactions
- [ ] Ambiguous transactions flagged for user review
- [ ] Categories: SALARY, FRAIS_REEL, DON, NON_TAXABLE, UNKNOWN
- [ ] Frais réels module picks up confirmed FRAIS_REEL transactions
- [ ] Dons module picks up confirmed DON transactions as candidates
- [ ] UI for reviewing and confirming/correcting classifications

---

## Phase 10: Multi-Year Carry-Forwards

**Status**: TODO

**User stories**: Reportable losses and excess dons automatically applied across
years.

### What to build

Carry-forward ledger table. On report finalization, persist: minus-values
reportables (10 years), excess dons (5 years). Next year's tax engine loads and
applies carry-forwards. Manual input for initial year (pre-Wealthfolio history).

### Acceptance criteria

- [ ] `tax_carry_forwards` table with type, amount, origin_year, expiry_year,
      remaining_amount
- [ ] Finalized report creates carry-forward entries for losses and excess dons
- [ ] Next year's report compilation loads applicable carry-forwards
- [ ] Minus-values offset against current year gains automatically
- [ ] Excess dons applied to current year réduction
- [ ] Manual input UI for initial carry-forward balances
- [ ] Carry-forward balances visible in declaration summary

---

## Phase 11: Manual Additional Income

**Status**: TODO

**User stories**: User with rental/freelance income adds it to their
declaration.

### What to build

Simple form for additional income categories Wealthfolio can't track
automatically. Amount + box number. Included in declaration summary and
available to LLM context.

### Acceptance criteria

- [ ] Additional income entries: category name, amount, box number, notes
- [ ] Predefined categories: revenus fonciers (4BE), micro-BIC, micro-BNC, other
- [ ] Custom category option for unlisted income types
- [ ] Entries included in declaration summary under correct formulaire
- [ ] Entries included in `TaxDeclarationContext` for LLM consumption

---

## Phase 12: Year-Round Running Estimate

**Status**: TODO

**User stories**: User sees tax situation evolve throughout the year, not just
at declaration time.

### What to build

Live dashboard "Estimation impôt YYYY" updated as data flows in. Salary tracked,
plus-values year-to-date, dons cumulés, frais réels candidates. Missing-data
flags. Progressive readiness indicator.

### Acceptance criteria

- [ ] Dashboard view showing current year running estimate
- [ ] Income sources tracked: salary (from fiches de paie), investment income,
      dons
- [ ] Missing data indicators: "IFU non reçu", "fiche de paie décembre
      manquante", etc.
- [ ] Readiness percentage or checklist
- [ ] Estimate updates automatically when new data arrives
- [ ] Clear distinction between "estimate" and "finalized declaration"

---

## Phase 13: LLM Tax Optimization

**Status**: TODO

**User stories**: AI reviews full tax picture and suggests legal optimization
strategies.

### What to build

`TaxDeclarationContext` fed to AI chat via existing `crates/ai` infrastructure.
Strategy suggestions: PFU/barème switch, loss harvesting, PEA timing, frais
réels marginal analysis, PEE early release eligibility. Disclaimer included.

### Acceptance criteria

- [ ] `TaxDeclarationContext` serializable and complete (all income, deductions,
      regimes, carry-forwards)
- [ ] AI chat integration via existing provider infrastructure
- [ ] Optimization suggestions with concrete savings estimates
- [ ] Multi-year strategy awareness (carry-forwards, PEA age, PEE events)
- [ ] Disclaimer: "not tax advice, verify with a professional"
- [ ] Tax-specific system prompt with French tax knowledge

---

## Verification

Per-phase checks:

- `cargo test -p wealthfolio-core`
- `cargo clippy --workspace`
- `pnpm type-check`
- `pnpm lint`

Full check before marking phase complete:

- `cargo test`
- `pnpm check`
