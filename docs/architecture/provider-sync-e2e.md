# Provider Sync E2E

This stack runs:

- `wealthfolio` built from the current repo
- `provider-sync` built from `apps/provider-sync-service`
- fixture-backed provider data for a full local import flow

## Start

```bash
cd /Users/mohamedkhairy/dev/wealthfolio
docker compose -f compose.e2e.yml up --build
```

Run detached:

```bash
docker compose -f compose.e2e.yml up --build -d
```

## URLs

- Wealthfolio: `http://localhost:8088`
- Provider Sync service: `http://localhost:3001`

## What is configured

The compose file sets:

- `WF_AUTH_REQUIRED=false`
- `WF_AGGREGATION_ENABLED=true`
- `WF_AGGREGATION_API_URL=http://provider-sync:3001`
- `WF_AGGREGATION_API_TOKEN=dev-token`
- `WF_AGGREGATION_PROVIDER=powens`
- `WF_AGGREGATION_USER_ID=local-user`

The provider service runs in `fixtures` mode so the full flow is testable without
Powens credentials.

## End-to-end test flow

1. Open `http://localhost:8088/settings/provider-sync`
2. Confirm the page shows configured status
3. Confirm one connection and two remote accounts are listed
4. Click `Sync now`
5. Verify:
   - local linked accounts appear on the page
   - import runs appear
   - sync state appears
6. Open Wealthfolio pages and confirm imported data:
   - `Accounts`
   - `Activities`
   - `Holdings`

## Useful checks

Service health:

```bash
curl http://localhost:3001/v1/healthz
curl http://localhost:8088/api/v1/healthz
```

Provider service:

```bash
curl -H "Authorization: Bearer dev-token" \
  "http://localhost:3001/v1/connections?userId=local-user"

curl -H "Authorization: Bearer dev-token" \
  "http://localhost:3001/v1/accounts?userId=local-user"
```

Wealthfolio server endpoints:

```bash
curl http://localhost:8088/api/v1/provider-sync/status
curl -X POST http://localhost:8088/api/v1/provider-sync/sync
curl http://localhost:8088/api/v1/provider-sync/import-runs
```

Logs:

```bash
docker compose -f compose.e2e.yml logs -f provider-sync
docker compose -f compose.e2e.yml logs -f wealthfolio
```

Stop:

```bash
docker compose -f compose.e2e.yml down
```
