# Plan: Provider Selection Drawer (Finary-style)

> Source PRD: In-conversation PRD — custom provider selection UI replacing
> direct Powens redirect

## Architectural decisions

- **Routes**: No new frontend routes. Drawer is a shared component opened from
  existing pages.
- **API**: `GET /provider-sync/connectors` (Axum) / `list_provider_connectors`
  (Tauri command). `GET /provider-sync/connect-url?connectorId={id}` adds
  optional param.
- **Key models**:
  `ConnectorDto { id, provider, name, logo_url, country, capabilities, category }`
  — flows from NestJS → Rust → Frontend.
- **Category mapping**: `investments` → Brokers, `crypto` → Crypto, `insurance`
  → Insurance, `savings` → Savings, default → Banks.
- **Cache**: 24h in-memory cache in Rust layer
  (`RwLock<(Vec<ConnectorDto>, Instant)>`). Frontend uses TanStack Query
  `staleTime: 24h`.
- **Redirect flow**: Powens redirects to existing `/auth/callback` route →
  triggers auto-sync → new-accounts modal.

---

## Phase 1: Connector catalog end-to-end

**User stories**: 1, 4, 10, 14

### What to build

A thin vertical slice that fetches the Powens connector list (with logos,
category, country) from NestJS through the Rust layer, exposes it via API/Tauri
commands, and renders it in a minimal drawer opened from the accounts page. No
search, no filtering, no pre-filtered URL yet — just the list rendering
end-to-end.

### Acceptance criteria

- [ ] NestJS `ConnectorDto` includes `logoUrl` and `category` fields
- [ ] NestJS `PowensProvider.listConnectors()` maps `logo_url` from Powens and
      derives `category` from `capabilities`
- [ ] Rust `AggregationApiClient` has `list_connectors()` method calling NestJS
      `GET /v1/connectors`
- [ ] Axum route `GET /provider-sync/connectors` returns `Vec<ConnectorDto>`
- [ ] Tauri command `list_provider_connectors` returns `Vec<ConnectorDto>`
- [ ] Frontend adapter `listProviderConnectors()` exposed in shared adapter
- [ ] Minimal drawer component renders connector list with logo + name + country
- [ ] "Link account" on accounts page opens the drawer
- [ ] Loading state shown while fetching

---

## Phase 2: Pre-filtered connection flow

**User stories**: 5, 8

### What to build

Clicking a connector in the drawer generates a Powens webview URL pre-filtered
to that specific connector (skipping provider search), and opens it in the
browser. Both entry points (accounts page "Link account" + connect page "Connect
provider") open the same drawer.

### Acceptance criteria

- [ ] NestJS `getConnectUrl()` accepts optional `connectorId`, appends
      `&connector_uuids={id}` to URL
- [ ] Rust `get_connect_url()` passes optional `connector_id` query param to
      NestJS
- [ ] Axum + Tauri endpoints accept optional `connectorId` param
- [ ] Frontend `getProviderConnectUrl(connectorId?)` passes param through
      adapter
- [ ] Clicking a connector in drawer calls `getProviderConnectUrl(connector.id)`
      and opens URL
- [ ] Connect page "Connect provider" button also opens the drawer
- [ ] Powens webview opens directly at credential entry (no provider search
      step)

---

## Phase 3: Search, categories, and UI polish

**User stories**: 2, 3, 7, 11, 12, 13

### What to build

Search input for filtering by name, category tabs (All, Banks, Brokers,
Insurance, Crypto, Savings) with counts, "Enter manually" option in the drawer,
and polished loading/empty/error states.

### Acceptance criteria

- [ ] Search input at top of drawer filters connectors by name (client-side)
- [ ] Category tabs filter by derived category, "All" tab shows everything
- [ ] Each tab shows count of matching connectors
- [ ] "Enter manually" option available in the drawer (opens existing
      AccountEditModal)
- [ ] Empty state when no connectors match search/filter
- [ ] Error state when connector fetch fails
- [ ] Drawer closes on Escape / click outside
- [ ] Search + category filters compose (both active at once)

---

## Phase 4: Auto-sync on redirect + connector cache

**User stories**: 6, 9, 15

### What to build

After Powens authentication, the redirect callback triggers an automatic sync.
New-accounts modal opens if new accounts are discovered. Connector list is
cached for 24h in the Rust layer and frontend.

### Acceptance criteria

- [ ] Powens redirect lands on `/auth/callback` and auto-triggers
      `syncProviderData()`
- [ ] After sync completes, new-accounts-found modal opens automatically if new
      accounts exist
- [ ] Rust layer caches connector list for 24h (subsequent calls skip
      NestJS/Powens)
- [ ] Frontend TanStack Query uses `staleTime: 24h` for connector list
- [ ] Full flow demoable: drawer → select provider → auth → redirect → auto-sync
      → configure accounts
