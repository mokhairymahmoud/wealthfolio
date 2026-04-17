# Powens Integration Study For Wealthfolio

## Summary

This document studies whether Wealthfolio Connect can be replaced with a
Powens-backed service owned by us, implemented in NestJS, and integrated into
Wealthfolio without recreating the entire hosted Connect platform.

Conclusion:

- Yes, replacing the current hosted Connect dependency with a Powens-backed
  service is technically feasible.
- The recommended architecture is not a direct Powens integration inside
  Wealthfolio. Instead, build a separate NestJS aggregation service and make
  Wealthfolio consume our own normalized API.
- This approach avoids coupling Wealthfolio core to Powens-specific concepts and
  keeps the door open for future providers such as Plaid, Snaptrade, or CSV.

## Scope

This study covers:

- The current Wealthfolio Connect architecture in this repository
- What Powens appears to provide
- A proposed NestJS service architecture
- How that service can integrate with Wealthfolio
- Risks, limitations, and recommended rollout phases

This study does not cover:

- Device sync replacement
- Subscription billing implementation
- A complete OpenAPI spec
- Exact frontend UI mockups

## Current Wealthfolio Connect Architecture

Wealthfolio Connect in this repository is a hosted cloud integration, not a
local module.

### Frontend

The frontend Connect provider is built around hosted auth defaults:

- `CONNECT_AUTH_URL` defaults to `https://auth.wealthfolio.app`
- `CONNECT_AUTH_PUBLISHABLE_KEY` defaults to a publishable key
- A hosted OAuth callback defaults to `https://connect.wealthfolio.app/deeplink`

Relevant files:

- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/features/wealthfolio-connect/providers/wealthfolio-connect-provider.tsx`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/lib/connect-config.ts`

The Connect feature is optional and only enabled when auth env vars are present.
Without them, the app remains fully offline.

### Backend

The backend Connect layer:

- stores refresh tokens in the app's secret store
- refreshes access tokens against a Supabase-compatible auth endpoint
- calls a hosted cloud API
- exposes local `/connect/*` routes as adapters over that remote service

Relevant files:

- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/client.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/token_lifecycle.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/connect.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/services/connect_service.rs`

### What Connect Bundles Today

The current Connect path implicitly includes several concerns in one system:

- hosted auth
- broker or aggregator connectivity
- account and activity fetch APIs
- subscription plan checks
- device sync support

This matters because a Powens replacement should not attempt to clone all of
that. The valuable replacement target is the broker aggregation path, not the
entire hosted platform.

## Why Powens Instead Of Reusing Wealthfolio Connect

The current Wealthfolio Connect architecture is optimized for Wealthfolio's own
hosted ecosystem. Recreating that stack would add unnecessary work:

- hosted auth and refresh lifecycle
- plan and billing concepts
- cloud tenancy concerns
- extra UX around Connect account management

If the real goal is portfolio synchronization from external institutions into a
local-first Wealthfolio instance, a narrower architecture is better:

- provider integration in our own backend
- normalized account and transaction DTOs
- import into Wealthfolio's existing local data model

## Why Powens Is A Plausible Upstream Provider

Powens documents a REST API and a "wealth aggregation" product rather than only
plain PSD2 transaction access.

From Powens documentation:

- the API is REST over HTTPS
- authentication uses bearer tokens
- connectors can be listed and inspected
- wealth aggregation includes accounts, investments, and related resources

Primary sources consulted:

- API overview:
  `https://docs.powens.com/documentation/integration-guides/quick-start/api-overview`
- Wealth overview:
  `https://docs.powens.com/documentation/integration-guides/wealth`
- Investments API:
  `https://docs.powens.com/api-reference/products/wealth-aggregation/investments`

### What Makes Powens Attractive

Powens appears to be better aligned with a European multi-asset aggregation use
case than a pure bank-transactions API. Based on the vendor's documentation, the
wealth product covers more than checking accounts, including market and savings
products.

Inference from the docs:

- Powens is likely a better fit than Plaid when the target user base is heavily
  European.
- Powens may provide broader wealth-account aggregation than a PSD2-only
  banking integration.

This is an inference from product positioning and documentation structure, not a
guarantee of actual institution coverage for specific brokers.

## Core Decision

### Recommended Decision

Build a separate NestJS service that:

- integrates with Powens
- owns all provider-specific auth and connector workflows
- exposes our own normalized API to Wealthfolio

Do not integrate Powens directly into Wealthfolio frontend or core crates.

### Why

This gives us:

- provider abstraction
- easier testing
- fewer changes to Wealthfolio core
- freedom to replace Powens later
- a clean boundary for secrets and external credentials

## Target Architecture

```text
Wealthfolio Frontend
  -> Wealthfolio Server / Tauri Commands
    -> Internal Aggregation Client
      -> Our NestJS Aggregation Service
        -> Powens API

Powens data
  -> normalized DTOs
  -> Wealthfolio local import
  -> SQLite in Wealthfolio
```

### Separation Of Responsibilities

#### NestJS Service

Responsible for:

- connector discovery
- connection creation and challenge flow
- token and credential custody
- periodic refresh or sync orchestration
- provider-specific retries and error handling
- normalized DTO output

#### Wealthfolio

Responsible for:

- account, asset, and activity persistence
- mapping normalized data into local domain models
- sync state visualization
- local-first UX
- reporting, analytics, and portfolio calculations

## Proposed NestJS Service Design

## Modules

### 1. `providers`

Contains the provider abstraction.

Interface sketch:

```ts
export interface AggregationProvider {
  getConnectors(): Promise<ConnectorDto[]>;
  createConnectionSession(input: CreateConnectionSessionInput): Promise<ConnectionSessionDto>;
  completeConnection(input: CompleteConnectionInput): Promise<ProviderConnectionDto>;
  refreshConnection(connectionId: string): Promise<void>;
  listAccounts(connectionId: string): Promise<AccountDto[]>;
  listHoldings(connectionId: string): Promise<HoldingDto[]>;
  listTransactions(connectionId: string, cursor?: string): Promise<TransactionPageDto>;
  getConnectionStatus(connectionId: string): Promise<ConnectionStatusDto>;
}
```

Initial implementation:

- `PowensProvider`

Possible later implementations:

- `PlaidProvider`
- `CsvProvider`
- `ManualProvider`

### 2. `connections`

Responsible for:

- storing user-to-provider connection records
- connector metadata
- connection state
- challenge or MFA state if Powens requires multi-step flows

Suggested persistence:

- `provider_connections`
- `provider_accounts`
- `provider_sync_state`

### 3. `ingestion`

Responsible for:

- pulling holdings and transactions from Powens
- preserving raw payloads for replay/debugging
- producing normalized internal records

Suggested persistence:

- `raw_accounts`
- `raw_holdings`
- `raw_transactions`
- `sync_runs`

### 4. `mapping`

Responsible for:

- security identifier normalization
- currency normalization
- account-type mapping
- transaction-type mapping
- dedupe fingerprint generation

This module is critical. It should be isolated and heavily unit-tested.

### 5. `wealthfolio-bridge`

Responsible for:

- exposing a stable internal API for Wealthfolio
- serving normalized DTOs
- optionally pushing sync webhooks or status updates later

## DTO Contract Between NestJS And Wealthfolio

Wealthfolio should consume our own contracts, not Powens payloads.

### Connection DTO

```ts
export interface ConnectionDto {
  id: string;
  provider: "powens";
  connectorId: string;
  connectorName: string;
  institutionName: string | null;
  status: "pending" | "connected" | "degraded" | "reauth_required" | "failed";
  lastSyncedAt: string | null;
}
```

### Account DTO

```ts
export interface AccountDto {
  id: string;
  connectionId: string;
  externalAccountId: string;
  name: string;
  type: "brokerage" | "retirement" | "cash" | "crypto" | "other";
  currency: string | null;
  institutionName: string | null;
  mask: string | null;
}
```

### Security DTO

```ts
export interface SecurityDto {
  id: string;
  symbol: string | null;
  isin: string | null;
  name: string | null;
  currency: string | null;
  exchange: string | null;
}
```

### Holding DTO

```ts
export interface HoldingDto {
  accountId: string;
  securityId: string;
  quantity: string;
  price: string | null;
  marketValue: string | null;
  currency: string | null;
  asOf: string;
}
```

### Transaction DTO

```ts
export interface TransactionDto {
  id: string;
  accountId: string;
  securityId: string | null;
  bookedAt: string;
  settledAt: string | null;
  type:
    | "buy"
    | "sell"
    | "dividend"
    | "interest"
    | "fee"
    | "tax"
    | "transfer_in"
    | "transfer_out"
    | "cash_deposit"
    | "cash_withdrawal"
    | "unknown";
  quantity: string | null;
  unitPrice: string | null;
  grossAmount: string | null;
  netAmount: string | null;
  fee: string | null;
  currency: string | null;
  description: string | null;
  externalReference: string | null;
}
```

## Wealthfolio Integration Strategy

## Principle

Keep Powens-specific logic out of Wealthfolio core as much as possible.

### Recommended Integration Points

#### 1. New Internal Client Layer

Add a new client in Wealthfolio for our own service instead of reusing
`ConnectApiClient`.

Suggested location:

- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/` only if we generalize
  it into a provider-neutral aggregation crate
- or a new crate such as
  `/Users/mohamedkhairy/dev/wealthfolio/crates/aggregation-client`

The second option is cleaner because the current `connect` crate is strongly
associated with Wealthfolio Connect semantics.

#### 2. New Server Endpoints

Add Wealthfolio server endpoints that speak to our NestJS service:

- `GET /api/aggregation/connections`
- `POST /api/aggregation/connections/start`
- `POST /api/aggregation/connections/complete`
- `POST /api/aggregation/sync`
- `GET /api/aggregation/accounts`
- `GET /api/aggregation/holdings`
- `GET /api/aggregation/transactions`

These should remain thin and delegate to core services.

#### 3. Local Import Pipeline

Do not directly overwrite local data from provider responses.

Instead:

- fetch normalized DTOs
- map into Wealthfolio domain commands
- upsert accounts/assets
- import activities
- record import runs and sync state

This matches the repository's preference for thin server commands and business
logic inside core crates.

## Wealthfolio Domain Mapping

The service must map upstream provider data into Wealthfolio's local concepts.

### Accounts

Map provider account categories into Wealthfolio account types conservatively.

Example direction:

- brokerage -> securities account
- retirement -> securities account with metadata
- cash wallet -> cash account
- crypto account -> cryptocurrency account

If an account type cannot be safely mapped, default to a generic supported local
type plus metadata instead of inventing a new Wealthfolio account type too early.

### Assets

Asset identity resolution should prefer stable identifiers:

1. ISIN
2. Symbol plus exchange
3. provider-specific security ID stored as metadata

Do not rely on names alone.

### Activities

Initial MVP mapping should support only high-confidence activity types:

- buy
- sell
- dividend
- fee
- interest

Treat transfers, taxes, and corporate actions as phase-two work unless their
mapping is already straightforward.

### Holdings

Holdings should be used initially for:

- reconciliation
- sync diagnostics
- missing-activity detection

Do not use holdings as the sole source of truth for portfolio history. Wealthfolio
is transaction-centric, so holdings alone are insufficient for correct historical
performance calculations.

## Authentication And Secrets

## NestJS Service

Store provider credentials or connection tokens encrypted at rest.

Requirements:

- use a server-side encryption key
- separate user identifiers from provider credentials
- support credential rotation
- audit sync failures without logging secrets

## Wealthfolio

Wealthfolio should store only what it needs:

- the URL of the NestJS service
- an internal API token or user session for that service
- no direct Powens end-user credentials

This keeps the boundary clean.

## UI And UX Strategy

## Recommendation

Do not retrofit the existing `Wealthfolio Connect` auth UX to pretend it is
Powens.

Instead:

- add a new feature flag
- expose a new "Broker Sync" or "Institution Sync" entry point
- label it clearly as provider-backed sync

This avoids inheriting the wrong mental model from Connect, which currently
assumes hosted auth, plans, and Connect-specific flows.

### Suggested UX Phases

#### Phase 1

- admin-configured service URL
- manual "Connect institution" flow
- manual "Sync now" button
- readonly sync status

#### Phase 2

- account selection during link
- richer sync result summaries
- re-auth flows
- holdings reconciliation warnings

#### Phase 3

- scheduled sync
- notification or event-driven refresh
- multiple provider backends

## Main Risks

## 1. Institution Coverage Risk

Powens may look like a good fit from public documentation, but actual broker
coverage for the target user base must be validated with real institutions.

This is the biggest pre-build risk.

Mitigation:

- validate 10 to 20 real target institutions before implementation
- classify them by:
  - connectable
  - holdings only
  - holdings plus transactions
  - unstable
  - unsupported

## 2. Security Identity Resolution

Even when accounts and transactions are available, security identity can be
messy:

- missing ISIN
- symbols reused across exchanges
- fund naming inconsistencies
- currency mismatches

Mitigation:

- build a dedicated mapping layer
- persist upstream raw identifiers
- maintain a local resolution cache

## 3. Activity Semantics

Provider transaction taxonomies rarely line up perfectly with local portfolio
activity models.

Mitigation:

- start with a narrow activity subset
- record unmapped transaction kinds for later classification
- support a review queue if needed

## 4. Sync Idempotency

Without strict dedupe rules, repeated syncs will create duplicate local
activities.

Mitigation:

- use provider transaction IDs as primary dedupe keys
- add a secondary fingerprint for providers that mutate IDs
- persist import-run metadata

## 5. Product Scope Drift

Rebuilding Wealthfolio Connect features such as plans, hosted auth, or device
sync would slow delivery and add architectural noise.

Mitigation:

- explicitly keep this initiative limited to provider-backed broker sync

## Recommended Delivery Plan

## Phase 0: Validation

- confirm target-country and target-broker coverage with Powens
- capture sample payloads for:
  - account list
  - holdings
  - transactions
- verify whether the institutions needed actually expose investment history at
  the required fidelity

Exit criteria:

- at least 5 representative institutions validated

## Phase 1: NestJS MVP

- build `PowensProvider`
- persist connections and encrypted credentials
- expose normalized account, holding, and transaction DTOs
- add manual sync endpoint
- store raw payloads and sync runs

Exit criteria:

- one user can connect one institution and fetch normalized data successfully

## Phase 2: Wealthfolio Backend Bridge

- add internal client in Wealthfolio
- add server/Tauri command wrappers
- map DTOs into local accounts, assets, and activities
- persist sync state and import runs

Exit criteria:

- one connected institution can be imported into local Wealthfolio storage

## Phase 3: Frontend UX

- add provider sync settings
- add account-link flow
- add "Sync now"
- add basic sync results screen

Exit criteria:

- user can connect and sync without manual API calls

## Phase 4: Reliability

- retries
- re-auth required handling
- scheduled sync
- sync health diagnostics

Exit criteria:

- sync remains stable across repeated runs and token refreshes

## What Should Not Be Built First

Avoid these in the first iteration:

- multi-provider marketplace
- direct Powens integration in frontend
- recreating Connect subscription plans
- replacing device sync
- holdings-only portfolio reconstruction
- automatic support for every transaction type

## Proposed Repository Changes In Wealthfolio

These are the most likely first changes for an MVP.

### New docs

- this document

### New backend integration layer

Possible new paths:

- `/Users/mohamedkhairy/dev/wealthfolio/crates/aggregation-client`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/aggregation.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/services/aggregation_service.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/features/provider-sync/`

### Existing paths likely to be touched

- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/commands/`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/storage-sqlite/`

## Recommended Final Architecture

The recommended end state is:

- Wealthfolio remains local-first
- our NestJS service owns external aggregation
- Powens is hidden behind our own provider abstraction
- Wealthfolio imports normalized portfolio data into its local domain model
- Connect-specific hosted auth and plan logic are not reused

This is the highest-leverage path because it solves the actual problem,
minimizes coupling, and preserves optionality.

## External Sources

- Powens API overview:
  `https://docs.powens.com/documentation/integration-guides/quick-start/api-overview`
- Powens wealth guide:
  `https://docs.powens.com/documentation/integration-guides/wealth`
- Powens investments API:
  `https://docs.powens.com/api-reference/products/wealth-aggregation/investments`
- Plaid Link overview:
  `https://plaid.com/docs/link/`
- Plaid Transactions:
  `https://plaid.com/docs/transactions/`
- Plaid Investments API:
  `https://plaid.com/docs/api/products/investments/`

## Repository References

- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/features/wealthfolio-connect/providers/wealthfolio-connect-provider.tsx`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/lib/connect-config.ts`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/client.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/token_lifecycle.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/connect.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/services/connect_service.rs`
