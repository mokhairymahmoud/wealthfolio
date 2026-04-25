# PRD: French Tax Declaration Module

## Vision

Wealthfolio becomes a one-stop shop for French tax declaration, automating data
collection from investment accounts, bank transactions, and uploaded documents,
producing a box-by-box declaration summary ready to fill on impots.gouv.fr.

## Context

An investment-focused tax module already exists: it compiles tax events from
portfolio activities (dividends, interest, capital gains, withholding tax) for
CTO accounts, reconciles them against uploaded IFU documents, and suggests
French tax boxes (2042-2DC, 2042-2TR, 2047, 2074). It does not handle salary
income, frais reels, deductions, dons, or non-CTO account regimes.

Wealthfolio also has infrastructure for bank account tracking (CASH account
type, Plaid sync, unified activity model with DEPOSIT/WITHDRAWAL/FEE types).

---

## Design Decisions

### 1. Tax Classification Layer

Add a `tax_category` field (or dedicated `tax_classification` table) on
activities. First-class, queryable — not buried in metadata JSON.

- Rule-based auto-classification from bank transaction descriptions (keywords
  like "VIREMENT SALAIRE", "DON CROIX ROUGE", merchant patterns, recurring
  amount detection).
- Ambiguous transactions flagged for user review (leverages existing
  `needs_review` pattern).
- Categories include: SALARY, FRAIS_REEL, DON_ASSOCIATION, NON_TAXABLE, etc.

### 2. Account Fiscal Regimes

Required `regime` field on investment accounts. Defaults to CTO for backwards
compatibility. The `AccountTaxProfile` already has `regime`, `opened_date`, and
`closed_date` fields.

Tax engine uses `(regime, opened_date, event_date)` to determine treatment:

- **CTO**: per-trade FIFO plus-values, every sell is a taxable event (2074,
  3VG/3VH).
- **PEA**: only WITHDRAWAL from the envelope triggers a tax event. Tax-free
  after 5 years (only 17.2% PS). Fully taxed before.
- **PEE**: similar to PEA. Tax-free after 5 years. Early release for specific
  life events (mariage, achat residence principale, rupture contrat).
- **PER**: deductible contributions, taxed at withdrawal as income.
- **Assurance-vie**: gains taxed at rachat only. Abattement 4,600/9,200 EUR
  after 8 years. Tiered taxation based on contract age.

For PEA/PEE/assurance-vie, the tax engine ignores individual SELLs and only
generates tax events from WITHDRAWAL activities. Plus-value at withdrawal
computed via prorata method:
`withdrawal_amount - (total_contributions * withdrawal_amount / total_envelope_value)`.

### 3. Document Extraction (LLM-Only)

LLM-based extraction as the primary and only strategy. No local regex parsing.

- Blanket consent toggle in tax profile settings (user opts in once, not per
  document).
- Define structured schemas of expected fields per document type (IFU, fiche de
  paie, attestation de don, releve PEE).
- Send document to LLM with schema, validate output.
- `TaxCloudExtractionTrait` already abstracts the extraction backend.

**Split behavior by document type:**

- **IFU**: reconciliation mode. Portfolio activities are source of truth, IFU
  document is a cross-check. Discrepancies flagged.
- **Fiche de paie, recus fiscaux de dons**: direct-population mode. Document is
  the source of truth. Extraction creates the relevant tax data (salary entry,
  don entry), user confirms extracted values.

### 4. Fiche de Paie

- Monthly uploads supported for income tracking and anomaly detection.
- Only December (or latest available) fiche de paie **required** for tax
  declaration — uses cumul annuel fields.
- Key extracted fields: net imposable cumule (box 1AJ/1BJ), CSG deductible,
  heures supplementaires exonerees (box 1GH).

### 5. Frais Reels

- Auto-detect candidate transactions from bank data (transport subscriptions,
  restaurant transactions near workplace, recurring professional expenses).
- User confirms/rejects candidates and manually adds items not in bank data
  (e.g., indemnites kilometriques based on distance).
- Automatic comparison: frais reels total vs 10% abattement forfaitaire, with
  recommendation showing the more advantageous option and savings amount.

**Proof management (per-category requirements):**

- **Recurring subscriptions** (Navigo, transport): bank transaction record is
  sufficient. System can auto-generate summary document.
- **One-off purchases** (laptop, materiel): flagged as "justificatif manquant"
  if no upload, but still included in calculation.
- **Indemnites kilometriques**: proof is the calculation itself (distance x
  bareme x jours). System generates downloadable PDF.
- Proofs stored using existing `TaxDocument` infrastructure.
- Fisc can request justificatifs up to 3 years back.

### 6. Dons (Charitable Donations)

- Auto-detect candidates from bank transactions (virements to known
  associations, recurring prelevements).
- Recu fiscal is the authoritative source for declared amount (bank amount may
  differ due to cotisations vs dons).
- Workflow: detect candidates -> prompt user to upload recu fiscal or confirm
  amount -> classify by recipient type -> compute reduction.

**Classification and rates:**

- **Organismes d'aide aux personnes en difficulte** (Restos du Coeur, Secours
  Populaire): 75% reduction up to ceiling, then 66% (box 7UD).
- **Organismes d'interet general**: 66% reduction, capped at 20% revenu
  imposable (box 7UF).
- **Partis politiques**: 66% reduction, separate ceiling (box 7UH).

### 7. Foyer Fiscal Profile

Tax profile form — user sets once, updates on life changes:

- Situation maritale (celibataire, marie, pacse, divorce, veuf)
- Nombre d'enfants a charge
- Parent isole (case T)
- Invalidite, ancien combattant (additional half-parts)

System computes nombre de parts automatically. Can be pre-filled/verified from
uploaded avis d'imposition.

### 8. PFU vs Bareme Progressif Simulation

Dual computation for every declaration:

- **PFU**: 30% flat (12.8% IR + 17.2% PS) on investment income.
- **Bareme progressif**: investment income added to salary, taxed at marginal
  rate. Benefits: 40% abattement on dividends, CSG deductible (6.8%), loss
  deduction.

Display: estimated total impot for each option, concrete recommendation with
savings amount. Requires foyer fiscal profile (nombre de parts) and full income
picture (salary + investment).

### 9. Declaration Output

Printable box-by-box summary organized by formulaire:

- **2042**: 1AJ (salaires), 1AK (frais reels), 2DC (dividendes), 2TR (interets),
  7UF/7UD/7UH (dons)
- **2074**: detail des plus-values CTO
- **2047**: revenus encaisses a l'etranger
- **3VG/3VH**: plus-values/moins-values

Drill-down from each box to underlying data (list of transactions, documents,
calculations). Designed for side-by-side use with impots.gouv.fr.

### 10. Multi-Year Carry-Forwards

Automatic tracking across tax years:

- **Moins-values reportables**: 10 years (CTO capital losses offset against
  future gains).
- **Excess dons**: 5 years (dons exceeding 20% ceiling).
- **PEA/PEE age**: 5-year clock from account opening.

Finalized report persists carry-forward balances for next year's engine. Manual
input for initial year (pre-Wealthfolio history).

### 11. Annual Tax Parameters

Hybrid approach:

- **Config files** (`tax_params_YYYY.toml`): rates, thresholds, bareme tranches,
  plafond QF, don ceilings, PFU rates, kilometrique bareme. Pure data, versioned
  per year.
- **Rust code**: calculation logic (FIFO matching, prorata method, envelope
  detection, frais reels comparison). Structural, rarely changes.
- `rule_pack_version` on `TaxYearReport` pins to parameter set.
- Manual yearly update by developer when loi de finances is published.
- LLM can alert to detected changes on impots.gouv.fr but never auto-updates
  parameters.

### 12. Additional Income (Manual Input)

Support manual input for income categories Wealthfolio cannot track:

- Revenus fonciers (box 4BE)
- Micro-entrepreneur BIC/BNC
- Other income sources

Simple amount + box number, no calculation engine. Included in declaration
summary and available to LLM optimization context.

### 13. Year-Round Running Estimate

Live dashboard "Estimation impot YYYY — en cours" updated as data flows in:

- Salary tracked so far
- Plus-values realisees year-to-date
- Dons cumules
- Frais reels candidates detected
- Flags missing data: "IFU non recu", "recu fiscal MSF manquant", "fiche de paie
  decembre manquante"

Turns declaration from annual scramble into passive, incremental process.

### 14. LLM Tax Optimization (Future)

**Architecture (design now):** `TaxDeclarationContext` struct — a complete,
serializable snapshot of the full tax picture:

- Foyer fiscal info (parts, situation)
- All income sources with amounts
- All deductions (frais reels, CSG)
- All plus-values by regime
- All dons with classification
- PFU vs bareme comparison results
- Multi-year carry-forward balances
- Detected issues and missing documents

Serves as input to both the declaration summary UI and LLM context. One model,
two consumers.

**LLM capabilities:**

- PFU vs bareme switch recommendation with savings
- Reportable minus-values reminder
- PEA ceiling optimization
- Frais reels marginal analysis
- PEE early release eligibility detection
- Cross-year strategy suggestions

Carries disclaimer: not a tax advisor, recommendations should be verified.

---

## Phasing

### Phase 1 — Core Declaration Path

Minimum to produce a usable declaration summary for someone with salary + CTO.

- Foyer fiscal profile (situation familiale, nombre de parts)
- Fiche de paie LLM extraction (salary -> box 1AJ)
- Existing CTO plus-values (already implemented)
- Declaration summary with box numbers
- Tax parameters config file for current year

### Phase 2 — Deductions and Credits

- PFU vs bareme progressif dual simulation with recommendation
- Dons module (detection, recu fiscal, 75%/66% classification, reduction)
- Frais reels module (candidate detection, proof management, abattement
  comparison)

### Phase 3 — Advanced Regimes and Automation

- PEA/PEE/assurance-vie regime engine (envelope withdrawal, prorata method)
- Bank transaction tax classification (auto-categorization layer)
- Multi-year carry-forwards (minus-values, excess dons)
- Manual input for additional income categories

### Phase 4 — Intelligence Layer

- Year-round running estimate dashboard
- LLM tax optimization (TaxDeclarationContext -> AI advice)
- Strategy suggestions and legal optimization proposals
