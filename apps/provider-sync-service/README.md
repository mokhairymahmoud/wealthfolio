# Provider Sync Service

Standalone NestJS service that exposes the normalized API expected by
Wealthfolio's new `Provider Sync` feature.

Current status:

- NestJS scaffold is implemented
- bearer-token auth is implemented
- normalized endpoints are implemented
- fixture-backed provider is implemented
- Powens adapter reads real connectors, connections, accounts, and market orders

## Endpoints

All endpoints are mounted under `/v1` and require:

```text
Authorization: Bearer <SERVICE_TOKEN>
```

Available routes:

- `GET /v1/connectors`
- `GET /v1/connections?userId=...`
- `GET /v1/accounts?userId=...&connectionId=...`
- `GET /v1/transactions?userId=...&connectionId=...&accountId=...`
- `POST /v1/sync`
- `GET /v1/sync-runs/:runId`
- `POST /v1/admin/powens/auth/init`
- `POST /v1/admin/powens/auth/renew`
- `GET /healthz`

## Local run

```bash
cd /Users/mohamedkhairy/dev/wealthfolio/apps/provider-sync-service
cp .env.example .env
npm install
npm run start:dev
```

Default config:

- host: `127.0.0.1`
- port: `3001`
- service token: `dev-token`
- provider: `fixtures`

To switch to Powens:

```bash
export AGGREGATION_PROVIDER=powens
export POWENS_BASE_URL=https://your-domain.biapi.pro/2.0
export POWENS_USER_ACCESS_TOKEN=...
export POWENS_USER_ID=123456
```

Alternative auth path:

```bash
export AGGREGATION_PROVIDER=powens
export POWENS_BASE_URL=https://your-domain.biapi.pro/2.0
export POWENS_CLIENT_ID=...
export POWENS_CLIENT_SECRET=...
export POWENS_USER_ID=123456
```

If `POWENS_USER_ACCESS_TOKEN` is absent, the service calls Powens
`POST /auth/renew` to mint a user token for `POWENS_USER_ID`.

## Powens token helpers

Create a new Powens user and associated token:

```bash
curl -X POST \
  -H "Authorization: Bearer dev-token" \
  -H "Content-Type: application/json" \
  http://localhost:3001/v1/admin/powens/auth/init
```

If `POWENS_CLIENT_ID` and `POWENS_CLIENT_SECRET` are configured, Powens returns
a permanent token and the new `id_user`. Without client credentials, Powens
returns a temporary token and temporary user.

Renew a token for an existing Powens user:

```bash
curl -X POST \
  -H "Authorization: Bearer dev-token" \
  -H "Content-Type: application/json" \
  http://localhost:3001/v1/admin/powens/auth/renew \
  -d '{"userId":123456}'
```

`auth/renew` also accepts request-local `clientId`, `clientSecret`, and
`revokePrevious` overrides. If `userId` is omitted, Powens may create a user,
depending on your account configuration.

## Quick test

```bash
curl -H "Authorization: Bearer dev-token" "http://localhost:3001/v1/connections?userId=local-user"
curl -H "Authorization: Bearer dev-token" "http://localhost:3001/v1/accounts?userId=local-user"
curl -H "Authorization: Bearer dev-token" "http://localhost:3001/v1/transactions?userId=local-user&connectionId=conn_demo_ibkr&accountId=acc_demo_brokerage"
```

## Wealthfolio integration

Run Wealthfolio with:

```bash
export WF_AGGREGATION_ENABLED=true
export WF_AGGREGATION_API_URL=http://localhost:3001
export WF_AGGREGATION_API_TOKEN=dev-token
export WF_AGGREGATION_PROVIDER=powens
export WF_AGGREGATION_USER_ID=local-user
```

Then open:

```text
/settings/provider-sync
```

## Next implementation step

Broaden the Powens adapter beyond market orders:

- recover dividends/fees/interest from additional Powens resources where needed
- persist sync runs and cursors instead of keeping them in memory
- add connection initiation/completion flows if Wealthfolio should start Powens linking itself
