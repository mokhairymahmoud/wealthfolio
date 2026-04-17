# Fee Scanner — PRD & Implementation Plan

## Problem Statement

Wealthfolio users who hold ETFs and mutual funds pay hidden ongoing fees
(expense ratios / TER) that silently erode returns over time. There is no
visibility into these costs. Users cannot see how much they lose annually to
fund fees, how that compounds over decades, or whether cheaper alternatives
exist. Finary offers a Fee Scanner that surfaces this data — Wealthfolio has
nothing equivalent. The existing `investment-fees-tracker` addon only covers
transaction-level brokerage fees, not fund-level expense ratios.

## Solution

A **Fee Scanner** tab inside the Insights page that:

- Fetches and stores expense ratios for ETFs (via Alpha Vantage) and allows
  manual entry for any holding
- Shows per-holding annual fee cost and severity (warning >0.5%, high >1.0%)
- Shows portfolio-level totals: total annual fee drag, weighted average expense
  ratio, fee as % of portfolio
- Projects compound fee impact over 10/20/30 years
- Integrates with the AI assistant to suggest lower-cost alternatives (per
  holding or bulk)

## User Stories

1. As an investor, I want to see the expense ratio of each fund I hold, so that
   I understand the ongoing cost of my investments
2. As an investor, I want to see the annual fee cost in my base currency per
   holding, so that I can compare costs across my portfolio
3. As an investor, I want to see the total annual fee drag across my entire
   portfolio, so that I understand my overall cost burden
4. As an investor, I want to see the weighted average expense ratio of my
   portfolio, so that I can benchmark against industry norms
5. As an investor, I want to see fee as a percentage of total portfolio value,
   so that I can evaluate whether fees are proportionate
6. As an investor, I want to see the projected compound fee impact over 10, 20,
   and 30 years, so that I understand long-term cost erosion
7. As an investor, I want high-fee holdings flagged visually (warning/high
   badges), so that I can quickly identify the most expensive holdings
8. As an investor, I want to manually enter or override the expense ratio for
   any holding, so that I have accurate data even when automatic fetching is
   unavailable
9. As an investor, I want to edit expense ratios inline on the fee scanner
   table, so that I stay in context without navigating elsewhere
10. As an investor, I want a "Find cheaper alternative" button per holding that
    opens the AI assistant with context, so that I get personalized suggestions
11. As an investor, I want an "Analyze all high-fee holdings" bulk action that
    sends my full fee data to the AI assistant, so that I get portfolio-wide
    optimization advice
12. As an investor, I want expense ratios fetched automatically for ETFs when
    market data syncs, so that I don't have to manually look them up
13. As an investor, I want the fee scanner sorted by annual cost descending by
    default, so that the biggest cost items are at the top
14. As an investor, I want to filter the fee table by account, so that I can
    analyze fees per brokerage
15. As an investor, I want to see which holdings have no expense ratio data yet,
    so that I know where manual entry is needed

## Implementation Decisions

### Storage

- Add a nullable `expense_ratio` REAL column to the `assets` table via Diesel
  migration
- This is first-class queryable data, not buried in the JSON `metadata` field
- Manual edits write to the same column (no separate "override" mechanism)

### Data Fetching

- Extend the Alpha Vantage ETF_PROFILE handler to persist `net_expense_ratio`
  into the new `expense_ratio` column when fetching asset profiles
- Only ETFs are auto-populated via Alpha Vantage; mutual funds and other
  instruments require manual entry
- Expense ratio is fetched during the existing quote sync / asset profile flow —
  no new background jobs

### Fee Analysis Service

- New module at `crates/core/src/portfolio/fees/`
- Pure calculation service, no database access — receives holdings + asset data
- Per-holding output: expense_ratio, annual_fee (market_value × expense_ratio),
  severity level (none/warning/high)
- Portfolio-level output: total_annual_fee, weighted_avg_expense_ratio,
  fee_pct_of_portfolio, projected compound impact at 10/20/30 years
- Compound projection formula:
  `fee_drag_at_year_N = portfolio_value × ((1 + avg_return) ^ N - (1 + avg_return - weighted_er) ^ N)`
  where avg_return is configurable (default 7%)
- Fixed severity thresholds: >0.5% = WARNING, >1.0% = HIGH

### API Endpoints

- `GET /fees/analysis` — returns full fee analysis (per-holding + portfolio
  summary + projections)
- `PUT /assets/{id}/expense-ratio` — update expense ratio for an asset (manual
  entry / override)
- Both exposed via Tauri IPC commands and Axum HTTP handlers

### Frontend

- New "Fees" tab on the Insights page (`/insights/fees`)
- Top section: summary cards (total annual fees, weighted avg ER, fee % of
  portfolio)
- Projection section: compound fee impact at 10/20/30 years (simple bar or value
  cards)
- Main section: holdings fee table sorted by annual cost descending
  - Columns: symbol, name, account, market value, expense ratio (editable),
    annual cost, severity badge
  - Inline expense ratio editing (click cell → input → save)
  - "Find alternative" button per row → opens AI assistant with pre-filled
    prompt containing holding details
  - "Analyze all" button in table header → opens AI assistant with all high-fee
    holdings
- Filter by account
- Holdings with missing expense ratio shown with "Add" placeholder

### AI Integration

- Per-holding prompt template:
  `"I hold {name} ({symbol}) with an expense ratio of {er}%. It costs me {annual_cost}/year. Suggest lower-cost ETF alternatives that track a similar index or asset class."`
- Bulk prompt template:
  `"Here are my holdings with high fees: {list of name, symbol, ER, annual_cost}. Suggest lower-cost alternatives for each and estimate total savings."`
- Both open the existing AI assistant page with the prompt pre-filled

## Testing Decisions

Good tests verify external behavior through the public interface, not
implementation details.

### Modules to test

**Fee Analysis Service (Rust unit tests):**

- Per-holding fee calculation (market_value × expense_ratio)
- Portfolio-level aggregation (total, weighted average, percentage)
- Compound projection math at various horizons
- Severity classification at boundary values (0.5%, 1.0%)
- Edge cases: zero holdings, all missing expense ratios, single holding, mixed
  currencies
- Prior art: `crates/core/src/portfolio/income/` tests follow the same pattern
  of pure calculation testing

**API endpoints (integration tests):**

- Fee analysis returns correct structure with valid holdings
- Expense ratio update persists and reflects in subsequent analysis
- Prior art: existing Tauri command tests and Axum handler tests

**Frontend:** Manual verification — the table rendering, inline editing, and AI
integration are UI-driven and better tested by using the feature.

## Out of Scope

- Replacing or merging the existing `investment-fees-tracker` addon (transaction
  fees remain separate)
- Automated alternative fund matching / recommendation engine (AI handles this
  conversationally)
- Mutual fund expense ratio auto-fetching (no reliable free data source)
- Configurable severity thresholds (fixed for v1, can add to settings later)
- Expense ratio historical tracking (only current value stored)
- Integration with Morningstar or other premium fund data providers
- Fee comparison against benchmark portfolios
- Scoring system (Finary's 1/10 style) — we show raw data + severity badges

## Further Notes

- The Alpha Vantage provider already fetches `net_expense_ratio` in the
  ETF_PROFILE response but discards it. The implementation is mostly wiring up
  storage for data that's already available.
- The compound fee projection is the highest-impact insight — a 1% expense ratio
  on a $100k portfolio costs ~$28k over 20 years at 7% return. Making this
  visible drives user action.
- The AI integration avoids building a fund comparison database by leveraging
  the AI's knowledge of ETF alternatives. This is pragmatic for v1 and naturally
  improves as AI models improve.

---

## Implementation Plan

### Phase 1: Backend — Storage & Data Pipeline

**Step 1: Database migration**

- Add `expense_ratio` REAL nullable column to `assets` table
- Create migration in `crates/storage-sqlite/migrations/`
- Update `schema.rs` (auto-generated by Diesel)
- Update `Asset` model in `crates/core/src/assets/assets_model.rs`
- Verify: `cargo test -p wealthfolio-storage-sqlite`

**Step 2: Persist expense ratio from Alpha Vantage**

- In `crates/market-data/src/provider/alpha_vantage/mod.rs`, include
  `net_expense_ratio` in the `AssetProfile` return
- Update `AssetProfile` struct in `crates/market-data/src/models/profile.rs` to
  carry `expense_ratio: Option<f64>`
- In the asset profile save logic, write `expense_ratio` to the assets table
- Verify: trigger a quote sync for an ETF, confirm expense_ratio is stored

**Step 3: Expense ratio update endpoint**

- Add repository method to update `expense_ratio` on an asset
- Add service method in `crates/core/src/assets/`
- Expose as Tauri command + Axum handler
- Verify: call endpoint, confirm value persists

### Phase 2: Backend — Fee Analysis Service

**Step 4: Fee analysis calculation module**

- Create `crates/core/src/portfolio/fees/` module
  - `fee_model.rs` — `HoldingFee`, `FeeSummary`, `FeeProjection` structs
  - `fee_service.rs` — pure functions: per-holding fees, portfolio aggregation,
    compound projections, severity classification
- Verify: unit tests covering all calculation paths

**Step 5: Fee analysis endpoint**

- Wire fee service into a new endpoint that:
  1. Fetches holdings (reuse existing holdings service)
  2. Joins with asset expense ratios
  3. Runs fee calculations
  4. Returns structured response
- Expose as Tauri command + Axum handler
- Verify: integration test with sample data

### Phase 3: Frontend — Fee Scanner Tab

**Step 6: Insights page — add Fees tab**

- Add new tab to the Insights page routing
- Create fee scanner page component
- Wire up TanStack Query hook to fetch fee analysis data

**Step 7: Summary cards**

- Total annual fees, weighted avg ER, fee % of portfolio
- Projection cards: compound fee impact at 10/20/30 years

**Step 8: Holdings fee table**

- Sortable table with: symbol, name, account, market value, expense ratio,
  annual cost, severity badge
- Default sort: annual cost descending
- Account filter
- Missing-data rows with "Add" placeholder

**Step 9: Inline expense ratio editing**

- Click expense ratio cell → input field → save via update endpoint
- Optimistic update with TanStack Query invalidation

**Step 10: AI integration**

- "Find alternative" button per holding row
- "Analyze all high-fee holdings" bulk button
- Both navigate to AI assistant page with pre-filled prompt
