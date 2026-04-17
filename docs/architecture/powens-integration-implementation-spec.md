# Powens Integration Implementation Spec For Wealthfolio

## Purpose

This document translates the Powens integration study into an executable
implementation spec for a Powens-backed NestJS aggregation service and its
integration into Wealthfolio.

It is intentionally concrete:

- proposed modules
- DTO contracts
- Wealthfolio touchpoints
- schema expectations
- rollout phases
- acceptance criteria

This spec assumes the architectural recommendation from
`/Users/mohamedkhairy/dev/wealthfolio/docs/architecture/powens-integration-study.md`:

- Powens stays behind a separate NestJS service
- Wealthfolio consumes a provider-neutral internal API
- Wealthfolio remains local-first

## Goals

### Primary Goals

- Replace the broker-sync portion of Wealthfolio Connect with a Powens-backed
  service
- Keep Wealthfolio decoupled from Powens-specific payloads and auth flows
- Reuse Wealthfolio's existing local import-run and sync-state concepts where
  possible
- Preserve local SQLite as the system of record for portfolio data

### Non-Goals

- Replace device sync
- Recreate Wealthfolio Connect billing or plans
- Add direct frontend integration with Powens
- Implement every transaction type in the first version
- Build a multi-provider marketplace in v1

## High-Level Design

```text
Powens
  -> NestJS Aggregation Service
    -> provider-neutral DTOs
      -> Wealthfolio aggregation client
        -> core import services
          -> SQLite
            -> frontend sync UI
```

## Delivery Strategy

Implement in two systems:

1. External NestJS service
2. Wealthfolio integration layer in this repository

The provider-specific complexity lives almost entirely in `1`.

## NestJS Service Spec

## Service Modules

### `auth`

Responsibilities:

- service-to-service auth for Wealthfolio
- credential encryption utilities
- secret rotation support

Notes:

- This is not end-user auth for institutions.
- End-user institution auth is driven by Powens connector flow.

### `powens`

Responsibilities:

- raw Powens HTTP client
- request signing/authentication
- response decoding
- retry policies
- rate-limit handling

This module must not leak raw Powens models into other modules.

### `providers`

Responsibilities:

- provider abstraction
- current implementation: `PowensProvider`

Contract:

```ts
export interface AggregationProvider {
  listConnectors(input: ListConnectorsInput): Promise<ConnectorDto[]>;
  beginConnection(input: BeginConnectionInput): Promise<BeginConnectionResult>;
  completeConnection(input: CompleteConnectionInput): Promise<ConnectionDto>;
  refreshConnection(input: RefreshConnectionInput): Promise<void>;
  getConnection(input: GetConnectionInput): Promise<ConnectionDto>;
  listAccounts(input: ListAccountsInput): Promise<AccountDto[]>;
  listHoldings(input: ListHoldingsInput): Promise<HoldingDto[]>;
  listTransactions(input: ListTransactionsInput): Promise<TransactionPageDto>;
}
```

### `connections`

Responsibilities:

- create and persist provider connections
- map user -> connection -> provider accounts
- connection lifecycle state
- re-auth required state
- sync timestamps

### `sync`

Responsibilities:

- orchestrate account, holdings, and transaction refresh
- schedule manual and periodic sync runs
- write raw payload snapshots
- maintain cursors and checkpoints
- emit sync summaries

### `mapping`

Responsibilities:

- map provider-specific raw models into normalized DTOs
- normalize account type, identifiers, currencies, transaction categories
- produce deterministic dedupe fingerprints

### `api`

Responsibilities:

- expose Wealthfolio-facing REST endpoints
- validate service tokens
- return stable normalized DTOs

## NestJS Database Schema

Suggested schema entities:

### `service_users`

- `id`
- `external_user_id`
- `created_at`
- `updated_at`

### `provider_connections`

- `id`
- `user_id`
- `provider`
- `provider_connection_id`
- `connector_id`
- `connector_name`
- `institution_name`
- `status`
- `last_synced_at`
- `reauth_required_at`
- `created_at`
- `updated_at`

### `provider_secrets`

- `connection_id`
- `encrypted_access_blob`
- `key_version`
- `created_at`
- `updated_at`

This table should be isolated and access-controlled in code.

### `provider_accounts`

- `id`
- `connection_id`
- `provider_account_id`
- `name`
- `account_type`
- `currency`
- `mask`
- `institution_name`
- `ghost_link_state`
- `created_at`
- `updated_at`

### `provider_sync_state`

- `connection_id`
- `transactions_cursor`
- `last_accounts_sync_at`
- `last_holdings_sync_at`
- `last_transactions_sync_at`
- `last_success_at`
- `last_failure_at`
- `error_code`
- `error_message`

### `raw_payloads`

- `id`
- `connection_id`
- `payload_type`
- `external_id`
- `payload_json`
- `captured_at`

### `sync_runs`

- `id`
- `connection_id`
- `mode`
- `status`
- `summary_json`
- `started_at`
- `completed_at`

## NestJS API Contract

All endpoints below are internal to our system and should not expose Powens
payloads directly.

## Authentication

Service auth options:

- static service token in header for local deployment
- JWT between Wealthfolio and NestJS later if needed

For v1, prefer a simple static service token:

- `Authorization: Bearer <internal-service-token>`

## Endpoints

### Connectors

`GET /v1/connectors`

Response:

```json
{
  "items": [
    {
      "id": "powens-123",
      "provider": "powens",
      "name": "Interactive Brokers",
      "country": "FR",
      "capabilities": ["accounts", "holdings", "transactions"]
    }
  ]
}
```

### Begin Connection

`POST /v1/connections/begin`

Request:

```json
{
  "userId": "wf-user-1",
  "connectorId": "powens-123"
}
```

Response:

```json
{
  "sessionId": "sess_123",
  "flow": {
    "type": "redirect",
    "url": "https://provider-flow.example"
  }
}
```

Note:

- The exact Powens flow may be redirect-based, field-based, or multi-step.
- The DTO must support that without leaking raw Powens semantics.

### Complete Connection

`POST /v1/connections/complete`

Request:

```json
{
  "userId": "wf-user-1",
  "sessionId": "sess_123",
  "callbackPayload": {}
}
```

Response:

```json
{
  "connection": {
    "id": "conn_123",
    "provider": "powens",
    "connectorId": "powens-123",
    "connectorName": "Interactive Brokers",
    "institutionName": "Interactive Brokers",
    "status": "connected",
    "lastSyncedAt": null
  }
}
```

### List Connections

`GET /v1/connections?userId=wf-user-1`

### List Accounts

`GET /v1/accounts?userId=wf-user-1&connectionId=conn_123`

### List Holdings

`GET /v1/holdings?userId=wf-user-1&connectionId=conn_123`

### List Transactions

`GET /v1/transactions?userId=wf-user-1&connectionId=conn_123`

### Trigger Sync

`POST /v1/sync`

Request:

```json
{
  "userId": "wf-user-1",
  "connectionId": "conn_123",
  "mode": "incremental"
}
```

Response:

```json
{
  "runId": "run_123",
  "status": "running"
}
```

### Get Sync Run

`GET /v1/sync-runs/:runId`

### Webhook Receiver

`POST /v1/provider-webhooks/powens`

Used by Powens callbacks. Not called by Wealthfolio.

## Normalized DTO Definitions

The DTOs below are the contract Wealthfolio should consume.

### `ConnectionDto`

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

### `AccountDto`

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

### `SecurityDto`

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

### `HoldingDto`

```ts
export interface HoldingDto {
  accountId: string;
  security: SecurityDto;
  quantity: string;
  price: string | null;
  marketValue: string | null;
  currency: string | null;
  asOf: string;
}
```

### `TransactionDto`

```ts
export interface TransactionDto {
  id: string;
  accountId: string;
  security: SecurityDto | null;
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

### `TransactionPageDto`

```ts
export interface TransactionPageDto {
  items: TransactionDto[];
  nextCursor: string | null;
  hasMore: boolean;
}
```

### `SyncRunDto`

```ts
export interface SyncRunDto {
  id: string;
  connectionId: string;
  mode: "initial" | "incremental" | "backfill" | "repair";
  status: "running" | "applied" | "needs_review" | "failed" | "cancelled";
  startedAt: string;
  completedAt: string | null;
  summary: {
    accountsDiscovered: number;
    holdingsFetched: number;
    transactionsFetched: number;
    transactionsImported: number;
    transactionsSkipped: number;
    assetsCreated: number;
    errors: number;
  } | null;
}
```

## Wealthfolio Integration Spec

## Principle

Reuse existing Wealthfolio sync-state and import-run primitives where possible.

Existing relevant concepts already present in the repo:

- broker sync states
- import runs
- synced accounts
- platforms

These concepts exist across:

- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/broker_ingest/models.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/activities/`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/connect.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/commands/brokers_sync.rs`

### Recommended Wealthfolio Additions

Add a new provider-neutral integration path rather than hijacking current
Wealthfolio Connect code too deeply.

## New Files

Suggested new files:

- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/aggregation/mod.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/aggregation/models.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/aggregation/service.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/aggregation/client_traits.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/aggregation.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/services/aggregation_service.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/commands/aggregation.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/features/provider-sync/`

## Existing Files Likely To Change

### Backend

- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/main_lib.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/lib.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/commands/mod.rs`

### Frontend

- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/routes.tsx`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/lib/query-keys.ts`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/adapters/`

## Backend Client Contract

Define a provider-neutral client trait in core:

```rust
pub trait AggregationClientTrait: Send + Sync {
    async fn list_connections(&self, user_id: &str) -> Result<Vec<ConnectionDto>>;
    async fn list_accounts(&self, user_id: &str, connection_id: &str) -> Result<Vec<AccountDto>>;
    async fn list_holdings(&self, user_id: &str, connection_id: &str) -> Result<Vec<HoldingDto>>;
    async fn list_transactions(
        &self,
        user_id: &str,
        connection_id: &str,
        cursor: Option<&str>,
    ) -> Result<TransactionPageDto>;
    async fn trigger_sync(&self, user_id: &str, connection_id: &str) -> Result<SyncRunDto>;
}
```

Implementation options:

- HTTP client in server mode
- HTTP client in Tauri mode

Do not embed Powens-specific models in this trait.

## Core Service Spec

Add a new `AggregationImportService` in `crates/core`.

Responsibilities:

- fetch normalized accounts and transactions from aggregation client
- map accounts into local account records
- resolve or create local assets
- import activities through existing activity services
- update import runs and sync states

### Why A Core Service

This keeps:

- server endpoints thin
- Tauri commands thin
- business rules centralized

That matches the existing repository architecture guidance.

## Suggested Core Flow

### `sync_connection(connection_id)`

1. Fetch remote accounts from NestJS
2. Upsert local platforms and synced-account metadata
3. Fetch remote transactions
4. Convert transactions to Wealthfolio activity upserts
5. Resolve assets
6. Create or update import run
7. Persist results
8. Update broker sync state

### Holdings Usage

For v1:

- fetch holdings
- use them for diagnostics and future reconciliation
- do not use holdings as the authoritative import source

This avoids incorrect historical reconstructions.

## Wealthfolio Persistence Mapping

## Accounts

Map external account DTOs to local accounts plus provider metadata.

Suggested metadata to store:

- `provider`: `powens`
- `provider_connection_id`
- `provider_account_id`
- `connector_name`
- `institution_name`

If a dedicated local metadata field already exists, use it. Otherwise use the
existing synced-account/provider account mechanism rather than inventing a third
representation.

## Platforms

Wealthfolio already has a concept of local "platforms" in the existing Connect
path. Reuse that concept for institutions or upstream providers where possible.

Do not create a second institution table if the existing one is sufficient.

## Activities

Recommended v1 transaction mapping:

- `buy` -> local buy activity
- `sell` -> local sell activity
- `dividend` -> local dividend activity
- `fee` -> local fee activity
- `interest` -> local interest activity

Everything else:

- either mark as needs review
- or skip with structured import-run diagnostics

## Asset Resolution Rules

Preferred order:

1. ISIN exact match
2. Symbol plus exchange
3. instrument key based on normalized provider identifiers
4. create new asset stub if safe and required

Avoid creating duplicate local assets when identifier confidence is weak.

## Frontend Spec

## New Feature

Add a new feature folder:

- `/Users/mohamedkhairy/dev/wealthfolio/apps/frontend/src/features/provider-sync/`

Initial sub-files:

- `services/provider-sync-service.ts`
- `hooks/use-provider-connections.ts`
- `hooks/use-provider-sync.ts`
- `components/provider-sync-page.tsx`
- `components/connection-card.tsx`
- `components/sync-run-card.tsx`

## Initial UX

### V1 Screens

- list connected institutions
- show sync status
- manual sync button
- sync history

### Not In V1

- direct connector selection UI if it is easier to open an external service flow
- complex re-auth wizard
- holdings reconciliation dashboard

## Adapters

Add frontend adapters matching current desktop/web dual-runtime pattern.

For example:

- `listAggregationConnections`
- `syncAggregationConnection`
- `getAggregationAccounts`
- `getAggregationSyncRuns`

These should follow the same web/Tauri split as other command wrappers in the
repo.

## Config

Add new env vars for Wealthfolio:

- `WF_AGGREGATION_API_URL`
- `WF_AGGREGATION_API_TOKEN`
- `WF_AGGREGATION_PROVIDER` default `powens`
- `WF_AGGREGATION_ENABLED`

Rules:

- if not set, feature remains disabled
- local app should continue to run fully offline

## Phased Backlog

## Phase 0: Validation

Tasks:

1. Validate 5 to 10 real target institutions on Powens
2. Capture example payloads for accounts, holdings, transactions
3. Confirm transaction categories that matter to Wealthfolio are present
4. Confirm security identifiers are sufficient for asset resolution

Deliverables:

- institution coverage spreadsheet
- sample payload corpus
- final go/no-go decision

Acceptance:

- at least 70 percent of target institutions support the required data

## Phase 1: NestJS Provider MVP

Tasks:

1. create NestJS skeleton
2. implement Powens API client
3. implement provider abstraction
4. persist connections and encrypted credentials
5. expose normalized endpoints for:
   - connections
   - accounts
   - holdings
   - transactions
   - sync runs

Acceptance:

- test user can connect one institution and read normalized data

## Phase 2: Wealthfolio Backend Bridge

Tasks:

1. add aggregation client trait in core
2. implement server HTTP client
3. implement Tauri service wrapper
4. add server/Tauri endpoints and commands
5. add import service to map normalized transactions into activities

Acceptance:

- one connection can be synced into SQLite from server mode
- same path works in Tauri mode

## Phase 3: Frontend MVP

Tasks:

1. add feature flag
2. add provider sync page
3. show connections and sync status
4. add manual sync button
5. add import-run history

Acceptance:

- end user can sync one institution without manual API calls

## Phase 4: Hardening

Tasks:

1. scheduled sync
2. re-auth handling
3. retry rules
4. sync diagnostics
5. structured error UX

Acceptance:

- repeated syncs are idempotent and recover cleanly from expired connections

## Testing Strategy

## NestJS

- unit tests for Powens mapping layer
- unit tests for transaction classification
- integration tests with recorded payload fixtures
- contract tests for Wealthfolio-facing DTOs

## Wealthfolio

- core service tests for DTO -> activity mapping
- repository tests for sync-state updates
- frontend hook tests for provider-sync feature
- end-to-end test with mocked aggregation backend

## Must-Have Test Cases

- duplicate transaction import
- incremental sync with unchanged data
- unknown transaction type
- missing symbol with valid ISIN
- missing ISIN with symbol collision risk
- re-auth required status
- partial import failure

## Open Questions

These must be resolved before implementation starts in earnest.

1. Which target brokers and countries matter most?
2. Do we need cash account import, or only investment accounts?
3. Should account creation in Wealthfolio be automatic or review-based?
4. Do we want holdings reconciliation in v1 or only transaction import?
5. Will the NestJS service be single-user, household, or multi-tenant?

## Recommended First Sprint

If implementation starts now, the best first sprint is:

1. Validate Powens coverage for target institutions
2. Create NestJS service skeleton
3. Define normalized DTOs and freeze them
4. Build a fake provider endpoint returning fixtures
5. Add Wealthfolio aggregation client using the fake endpoint
6. Prove one end-to-end import into SQLite with stubbed data

This de-risks the contract before dealing with Powens-specific complexity.

## Acceptance Criteria For The Initiative

The initiative is successful when:

- Wealthfolio can sync at least one supported institution via our NestJS service
- imported activities land in local SQLite correctly
- repeat syncs do not create duplicates
- the feature remains optional and offline mode still works
- Wealthfolio contains no hard dependency on Powens-specific frontend logic

## References

Repository references:

- `/Users/mohamedkhairy/dev/wealthfolio/docs/architecture/powens-integration-study.md`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/server/src/api/connect.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/apps/tauri/src/commands/brokers_sync.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/connect/src/broker_ingest/models.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/activities/activities_service.rs`
- `/Users/mohamedkhairy/dev/wealthfolio/crates/core/src/activities/import_run_model.rs`

External references:

- Powens API overview:
  `https://docs.powens.com/documentation/integration-guides/quick-start/api-overview`
- Powens wealth guide:
  `https://docs.powens.com/documentation/integration-guides/wealth`
- Powens investments API:
  `https://docs.powens.com/api-reference/products/wealth-aggregation/investments`
