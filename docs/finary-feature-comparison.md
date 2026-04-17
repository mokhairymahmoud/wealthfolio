# Finary vs Wealthfolio — Feature Comparison

## Features Wealthfolio ALREADY Has

| Feature                                        | Notes                                                            |
| ---------------------------------------------- | ---------------------------------------------------------------- |
| Multi-account portfolio tracking               | Full support                                                     |
| Net worth tracking                             | Historical daily valuations                                      |
| Multi-currency support                         | Configurable base currency (Finary is EUR-only)                  |
| Stocks, ETFs, Funds, Crypto                    | Full asset type support                                          |
| Real estate & alternative assets               | Properties, vehicles, collectibles, precious metals, liabilities |
| Activity management (buy/sell/dividend/etc.)   | 10+ activity types                                               |
| CSV import with templates                      | Column mapping & preview                                         |
| Performance tracking (TWR, annualized)         | Plus volatility, max drawdown                                    |
| Benchmark comparison                           | Compare against market indices                                   |
| Asset allocation (by class, geography, sector) | Via custom taxonomies                                            |
| Income/dividend tracking                       | History charts & period analysis                                 |
| FIRE planning & Monte Carlo simulation         | More advanced than Finary's simulator                            |
| Financial goals                                | With account/position allocation                                 |
| Contribution limit tracking                    | Per account group, multi-year                                    |
| Broker sync (Wealthfolio Connect)              | External broker integration                                      |
| Multi-device sync                              | E2E encrypted                                                    |
| Provider aggregation sync                      | In progress (Powens)                                             |
| Market data from multiple providers            | Yahoo, IEX, Coinbase + custom providers                          |
| Health checks & data validation                | 6+ check categories                                              |
| AI assistant                                   | Multi-provider chat with financial tools                         |
| Dark mode / theming                            | Light and dark themes                                            |
| Addon/extension system                         | Finary has nothing like this                                     |
| Data export & backup                           | Full database backup/restore                                     |
| Local-first / offline                          | Finary requires cloud                                            |

## Missing Features (Present in Finary)

### Tier 1 — High Impact, Core Wealth Tracking

| #   | Feature               | Description                                                                                                      | Complexity |
| --- | --------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------- |
| 1   | Budgeting & Cash Flow | Transaction categorization (AI-powered), income vs expense tracking, Sankey diagram visualization, savings rate  | Large      |
| 2   | Subscription Scanner  | Detect recurring charges, identify subscriptions, suggest optimizations                                          | Medium     |
| 3   | Fee Scanner           | Analyze hidden fund fees (expense ratios, management fees), suggest lower-cost alternatives                      | Medium     |
| 4   | Diversification Score | Proprietary score evaluating portfolio diversification quality across geography, sector, asset class, market cap | Medium     |

### Tier 2 — Analytics & Insights

| #   | Feature                            | Description                                                                           | Complexity |
| --- | ---------------------------------- | ------------------------------------------------------------------------------------- | ---------- |
| 5   | Investor Profile                   | Generate a profile based on holdings & behavior (risk tolerance, style, preferences)  | Medium     |
| 6   | Popular Assets Leaderboard         | Show what the community is investing in (requires opt-in data sharing or public data) | Medium     |
| 7   | Monthly/Weekly Performance Reports | Automated PDF/email summary reports on a schedule                                     | Medium     |
| 8   | Market Segment Breakdown           | Large cap / mid cap / small cap allocation analysis                                   | Small      |

### Tier 3 — Crypto Enhancements

| #   | Feature                           | Description                                                         | Complexity |
| --- | --------------------------------- | ------------------------------------------------------------------- | ---------- |
| 9   | Crypto Wallet Address Tracking    | Track BTC/ETH wallet balances directly from blockchain addresses    | Medium     |
| 10  | DCA Plans (Dollar-Cost Averaging) | Set up recurring automated investment plans                         | Medium     |
| 11  | Crypto Staking/Lending Tracking   | Track staking rewards and lending interest as separate income types | Small      |

### Tier 4 — Real Estate Enhancements

| #   | Feature                      | Description                                            | Complexity |
| --- | ---------------------------- | ------------------------------------------------------ | ---------- |
| 12  | Rental Income Tracking       | Track rental income from properties, yield calculation | Small      |
| 13  | Mortgage Calculator          | Monthly payment calculator tool                        | Small      |
| 14  | Property Valuation Estimates | Estimated market values using indices or external data | Medium     |

### Tier 5 — Planning & Projections

| #   | Feature                              | Description                                                                   | Complexity                   |
| --- | ------------------------------------ | ----------------------------------------------------------------------------- | ---------------------------- |
| 15  | Financial Freedom Predictor          | Calculate age at which financial independence is reached                      | Small (extend existing FIRE) |
| 16  | Compound/Simple Interest Calculators | Standalone financial calculator tools                                         | Small                        |
| 17  | Wealth Simulator Enhancements        | 30-year projection with per-asset-class tax rates, withdrawal rate, inflation | Small (extend existing FIRE) |

### Tier 6 — Social & Community

| #   | Feature                     | Description                            | Complexity |
| --- | --------------------------- | -------------------------------------- | ---------- |
| 18  | Community Forum Integration | Link to or embed community discussions | Small      |
| 19  | Portfolio Sharing           | Share anonymized portfolio snapshots   | Medium     |

### Tier 7 — Notifications & Alerts

| #   | Feature            | Description                                                | Complexity |
| --- | ------------------ | ---------------------------------------------------------- | ---------- |
| 20  | Price Alerts       | Notify when an asset hits a target price                   | Medium     |
| 21  | Rebalancing Alerts | Notify when allocation drifts beyond threshold from target | Medium     |
| 22  | Sync Status Alerts | Alert on connection failures, stale data                   | Small      |

### Tier 8 — Platform & UX

| #   | Feature                    | Description                                                                   | Complexity |
| --- | -------------------------- | ----------------------------------------------------------------------------- | ---------- |
| 23  | Mobile App (iOS/Android)   | Native mobile experience                                                      | Very Large |
| 24  | Family/Household Mode      | Multi-profile support for household wealth tracking                           | Medium     |
| 25  | Smart Categorization Rules | Define rules for auto-classifying transactions (by merchant, amount, pattern) | Medium     |

### Tier 9 — Tax & Professional

| #   | Feature                        | Description                                            | Complexity |
| --- | ------------------------------ | ------------------------------------------------------ | ---------- |
| 26  | Tax Reporting/Optimization     | Capital gains reports, tax-loss harvesting suggestions | Large      |
| 27  | Wealth Statements/Declarations | Generate formal patrimony declarations                 | Medium     |
| 28  | Professional/Business Accounts | Holding company mode, separate business vs personal    | Medium     |

## Where Wealthfolio Is Already Stronger Than Finary

- FIRE planning (Monte Carlo, sequence-of-returns risk, glide paths)
- Custom taxonomies (more flexible than Finary's fixed categories)
- Local-first architecture (privacy advantage)
- Addon system (extensibility)
- Multi-base-currency support
- AI assistant with financial tools
- Health check system

## Recommended Priority Order

1. Budgeting & Cash Flow
2. Fee Scanner
3. Diversification Score
4. Automated Reports
5. Price/Rebalancing Alerts
6. Family Mode
