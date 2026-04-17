use std::{collections::HashMap, sync::Arc, sync::OnceLock, time::Duration, time::Instant};

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    broker::{
        AccountUniversalActivity, AccountUniversalActivityCurrency,
        AccountUniversalActivityExchange, AccountUniversalActivitySymbol,
        AccountUniversalActivitySymbolType, BrokerAccount, BrokerAccountBalance,
        BrokerBalanceTotal, BrokerConnection, BrokerConnectionBrokerage, BrokerSyncServiceTrait,
        SyncAccountsResponse, SyncConnectionsResponse,
    },
    broker_ingest::{ImportRunMode, ImportRunStatus, ImportRunSummary},
};
use wealthfolio_core::errors::{Error, Result};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const CONNECTOR_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

type ConnectorCache = tokio::sync::RwLock<Option<(Vec<ConnectorDto>, Instant)>>;

fn connector_cache() -> &'static ConnectorCache {
    static CACHE: OnceLock<ConnectorCache> = OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::RwLock::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorDto {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub color: Option<String>,
    pub country: Option<String>,
    pub capabilities: Vec<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationStatus {
    pub enabled: bool,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDto {
    pub id: String,
    pub provider: String,
    pub connector_id: String,
    pub connector_name: String,
    pub institution_name: Option<String>,
    pub status: String,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub id: String,
    pub connection_id: String,
    pub external_account_id: String,
    pub name: String,
    pub account_type: String,
    pub currency: Option<String>,
    pub institution_name: Option<String>,
    pub mask: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityDto {
    pub id: String,
    pub symbol: Option<String>,
    pub isin: Option<String>,
    pub name: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub id: String,
    pub account_id: String,
    pub security: Option<SecurityDto>,
    pub booked_at: String,
    pub settled_at: Option<String>,
    pub transaction_type: String,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub gross_amount: Option<String>,
    pub net_amount: Option<String>,
    pub fee: Option<String>,
    pub currency: Option<String>,
    pub description: Option<String>,
    pub external_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransactionPageDto {
    pub items: Vec<TransactionDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsPositionDto {
    pub symbol: Option<String>,
    pub isin: Option<String>,
    pub name: Option<String>,
    pub quantity: String,
    pub unit_price: Option<String>,
    pub average_cost: Option<String>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsBalanceDto {
    pub currency: String,
    pub cash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsDto {
    pub account_id: String,
    pub positions: Vec<HoldingsPositionDto>,
    pub balances: Vec<HoldingsBalanceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregationSyncResult {
    pub provider: String,
    pub connections_synced: SyncConnectionsResponse,
    pub accounts_synced: SyncAccountsResponse,
    pub transactions_fetched: usize,
    pub transactions_imported: usize,
    pub assets_created: usize,
    pub accounts_failed: usize,
    pub accounts_warned: usize,
    pub holdings_synced: usize,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ItemsEnvelope<T> {
    Items { items: Vec<T> },
    Data { data: Vec<T> },
    Bare(Vec<T>),
}

impl<T> ItemsEnvelope<T> {
    fn into_items(self) -> Vec<T> {
        match self {
            Self::Items { items } | Self::Data { data: items } | Self::Bare(items) => items,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggregationApiClient {
    client: reqwest::Client,
    base_url: String,
    auth_header: HeaderValue,
}

impl AggregationApiClient {
    pub fn new(base_url: &str, access_token: &str) -> Result<Self> {
        let auth_header = HeaderValue::from_str(&format!("Bearer {}", access_token))
            .map_err(|e| Error::Unexpected(format!("Invalid aggregation token format: {}", e)))?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                Error::Unexpected(format!(
                    "Failed to initialize aggregation HTTP client: {}",
                    e
                ))
            })?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        })
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, self.auth_header.clone());
        headers
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .headers(self.headers())
            .query(query)
            .send()
            .await
            .map_err(|e| Error::Unexpected(format!("Aggregation request failed: {}", e)))?;

        self.parse_response(response).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .query(query)
            .send()
            .await
            .map_err(|e| Error::Unexpected(format!("Aggregation request failed: {}", e)))?;

        self.parse_response(response).await
    }

    async fn parse_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();
        let body = response.text().await.map_err(|e| {
            Error::Unexpected(format!("Failed to read aggregation response: {}", e))
        })?;

        if !status.is_success() {
            return Err(Error::Unexpected(format!(
                "Aggregation API error {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            )));
        }

        serde_json::from_str(&body).map_err(|e| {
            Error::Unexpected(format!(
                "Failed to parse aggregation response: {} - {}",
                e, body
            ))
        })
    }

    pub async fn list_connectors(&self) -> Result<Vec<ConnectorDto>> {
        {
            let guard = connector_cache().read().await;
            if let Some((ref cached, fetched_at)) = *guard {
                if fetched_at.elapsed() < CONNECTOR_CACHE_TTL {
                    return Ok(cached.clone());
                }
            }
        }

        let envelope: ItemsEnvelope<ConnectorDto> = self.get("/v1/connectors", &[]).await?;
        let items = envelope.into_items();

        {
            let mut guard = connector_cache().write().await;
            *guard = Some((items.clone(), Instant::now()));
        }

        Ok(items)
    }

    pub async fn list_connections(
        &self,
        user_id: &str,
        connection_id: Option<&str>,
    ) -> Result<Vec<ConnectionDto>> {
        let mut query = vec![("userId", user_id.to_string())];
        if let Some(connection_id) = connection_id {
            query.push(("connectionId", connection_id.to_string()));
        }

        let envelope: ItemsEnvelope<ConnectionDto> = self.get("/v1/connections", &query).await?;
        Ok(envelope.into_items())
    }

    pub async fn list_accounts(
        &self,
        user_id: &str,
        connection_id: Option<&str>,
    ) -> Result<Vec<AccountDto>> {
        let mut query = vec![("userId", user_id.to_string())];
        if let Some(connection_id) = connection_id {
            query.push(("connectionId", connection_id.to_string()));
        }

        let envelope: ItemsEnvelope<AccountDto> = self.get("/v1/accounts", &query).await?;
        Ok(envelope.into_items())
    }

    pub async fn list_transactions(
        &self,
        user_id: &str,
        connection_id: &str,
        account_id: &str,
        cursor: Option<&str>,
    ) -> Result<TransactionPageDto> {
        let mut query = vec![
            ("userId", user_id.to_string()),
            ("connectionId", connection_id.to_string()),
            ("accountId", account_id.to_string()),
        ];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor.to_string()));
        }

        self.get("/v1/transactions", &query).await
    }

    pub async fn list_holdings(
        &self,
        user_id: &str,
        connection_id: &str,
        account_id: &str,
    ) -> Result<HoldingsDto> {
        let query = vec![
            ("userId", user_id.to_string()),
            ("connectionId", connection_id.to_string()),
            ("accountId", account_id.to_string()),
        ];

        self.get("/v1/holdings", &query).await
    }

    pub async fn get_connect_url(
        &self,
        connector_id: Option<&str>,
        redirect_uri: Option<&str>,
    ) -> Result<ConnectUrlResponse> {
        let mut query = Vec::new();
        if let Some(id) = connector_id {
            query.push(("connectorId", id.to_string()));
        }
        if let Some(uri) = redirect_uri {
            query.push(("redirectUri", uri.to_string()));
        }
        self.get("/v1/connect-url", &query).await
    }

    pub async fn disable_account(&self, user_id: &str, provider_account_id: &str) -> Result<()> {
        let query = vec![
            ("userId", user_id.to_string()),
            ("accountId", provider_account_id.to_string()),
        ];

        let _: serde_json::Value = self.post("/v1/accounts/disable", &query).await?;
        Ok(())
    }

    pub async fn delete_connection(&self, user_id: &str, connection_id: &str) -> Result<()> {
        let query = vec![
            ("userId", user_id.to_string()),
            ("connectionId", connection_id.to_string()),
        ];

        let _: serde_json::Value = self.post("/v1/connections/delete", &query).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectUrlResponse {
    pub url: String,
}

pub struct AggregationProviderNotifier {
    client: AggregationApiClient,
    user_id: String,
}

impl AggregationProviderNotifier {
    pub fn new(client: AggregationApiClient, user_id: impl Into<String>) -> Self {
        Self {
            client,
            user_id: user_id.into(),
        }
    }
}

#[async_trait::async_trait]
impl wealthfolio_core::accounts::ProviderAccountNotifier for AggregationProviderNotifier {
    async fn disable_account(&self, _provider: &str, provider_account_id: &str) -> Result<()> {
        self.client
            .disable_account(&self.user_id, provider_account_id)
            .await
    }
}

pub struct AggregationSyncService {
    client: AggregationApiClient,
    sync_service: Arc<dyn BrokerSyncServiceTrait>,
    user_id: String,
}

impl AggregationSyncService {
    pub fn new(
        client: AggregationApiClient,
        sync_service: Arc<dyn BrokerSyncServiceTrait>,
        user_id: impl Into<String>,
    ) -> Self {
        Self {
            client,
            sync_service,
            user_id: user_id.into(),
        }
    }

    pub async fn list_connectors(&self) -> Result<Vec<ConnectorDto>> {
        self.client.list_connectors().await
    }

    pub async fn list_connections(&self) -> Result<Vec<ConnectionDto>> {
        self.client.list_connections(&self.user_id, None).await
    }

    pub async fn list_accounts(&self, connection_id: Option<&str>) -> Result<Vec<AccountDto>> {
        self.client
            .list_accounts(&self.user_id, connection_id)
            .await
    }

    pub async fn sync(&self, connection_id: Option<&str>) -> Result<AggregationSyncResult> {
        let connections = self
            .client
            .list_connections(&self.user_id, connection_id)
            .await?;

        let mut accounts = self
            .client
            .list_accounts(&self.user_id, connection_id)
            .await?;

        // Powens may not have indexed bank accounts yet after a new connection.
        // Retry a few times with increasing delays.
        if accounts.is_empty() && !connections.is_empty() {
            for wait_secs in [5, 10, 15] {
                log::info!(
                    "No accounts found but {} connection(s) exist — retrying in {}s",
                    connections.len(),
                    wait_secs
                );
                tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                accounts = self
                    .client
                    .list_accounts(&self.user_id, connection_id)
                    .await?;
                if !accounts.is_empty() {
                    break;
                }
            }
        }

        let connections_synced = self
            .sync_service
            .sync_connections(
                connections
                    .iter()
                    .map(map_connection_to_broker_connection)
                    .collect(),
            )
            .await?;

        let connection_lookup: HashMap<String, ConnectionDto> = connections
            .iter()
            .cloned()
            .map(|connection| (connection.id.clone(), connection))
            .collect();

        let accounts_synced = self
            .sync_service
            .sync_accounts(
                accounts
                    .iter()
                    .map(|account| map_account_to_broker_account(account, &connection_lookup))
                    .collect(),
            )
            .await?;

        let local_accounts = self.sync_service.get_synced_accounts()?;
        let local_account_lookup: HashMap<String, wealthfolio_core::accounts::Account> =
            local_accounts
                .into_iter()
                .filter_map(|account| account.provider_account_id.clone().map(|id| (id, account)))
                .collect();

        let mut transactions_fetched = 0usize;
        let mut transactions_imported = 0usize;
        let mut assets_created = 0usize;
        let mut accounts_failed = 0usize;
        let mut accounts_warned = 0usize;
        let mut holdings_synced = 0usize;

        for account in &accounts {
            let Some(local_account) = local_account_lookup.get(&account.external_account_id) else {
                continue;
            };

            if local_account.tracking_mode == wealthfolio_core::accounts::TrackingMode::NotSet {
                continue;
            }

            if local_account.account_type.eq_ignore_ascii_case("CASH") {
                continue;
            }

            let import_mode = match self
                .sync_service
                .get_activity_sync_state(&local_account.id)?
            {
                Some(state) if state.last_successful_at.is_some() => ImportRunMode::Incremental,
                _ => ImportRunMode::Initial,
            };

            self.sync_service
                .mark_activity_sync_attempt(local_account.id.clone())
                .await?;

            let import_run = self
                .sync_service
                .create_import_run(&local_account.id, import_mode)
                .await?;

            let fetched_transactions = self
                .fetch_all_transactions(&account.connection_id, &account.id)
                .await;

            match fetched_transactions {
                Ok(transactions) => {
                    let fetched_count = transactions.len();
                    transactions_fetched += fetched_count;

                    let activities = transactions
                        .iter()
                        .map(|transaction| {
                            map_transaction_to_broker_activity(
                                transaction,
                                account,
                                connection_lookup.get(&account.connection_id),
                            )
                        })
                        .collect();

                    let (upserted, created_assets, _new_asset_ids, needs_review_count) = self
                        .sync_service
                        .upsert_account_activities(
                            local_account.id.clone(),
                            Some(import_run.id.clone()),
                            activities,
                        )
                        .await?;

                    transactions_imported += upserted;
                    assets_created += created_assets;

                    let summary = ImportRunSummary {
                        fetched: fetched_count as u32,
                        inserted: upserted as u32,
                        updated: 0,
                        skipped: fetched_count.saturating_sub(upserted) as u32,
                        warnings: needs_review_count as u32,
                        errors: 0,
                        removed: 0,
                        assets_created: created_assets as u32,
                    };

                    if needs_review_count > 0 {
                        accounts_warned += 1;
                        self.sync_service
                            .finalize_activity_sync_needs_review(
                                local_account.id.clone(),
                                format!("{} transaction(s) require review", needs_review_count),
                                Some(import_run.id.clone()),
                            )
                            .await?;
                        self.sync_service
                            .finalize_import_run(
                                &import_run.id,
                                summary,
                                ImportRunStatus::NeedsReview,
                                None,
                            )
                            .await?;
                    } else {
                        self.sync_service
                            .finalize_activity_sync_success(
                                local_account.id.clone(),
                                chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                Some(import_run.id.clone()),
                            )
                            .await?;
                        self.sync_service
                            .finalize_import_run(
                                &import_run.id,
                                summary,
                                ImportRunStatus::Applied,
                                None,
                            )
                            .await?;
                    }
                }
                Err(error) => {
                    accounts_failed += 1;
                    let error_message = error.to_string();
                    self.sync_service
                        .finalize_activity_sync_failure(
                            local_account.id.clone(),
                            error_message.clone(),
                            Some(import_run.id.clone()),
                        )
                        .await?;
                    self.sync_service
                        .finalize_import_run(
                            &import_run.id,
                            ImportRunSummary::default(),
                            ImportRunStatus::Failed,
                            Some(error_message),
                        )
                        .await?;
                }
            }
        }

        // Holdings sync for accounts with Holdings tracking mode
        for account in &accounts {
            let Some(local_account) = local_account_lookup.get(&account.external_account_id) else {
                continue;
            };

            if local_account.tracking_mode != wealthfolio_core::accounts::TrackingMode::Holdings {
                continue;
            }

            if local_account.account_type.eq_ignore_ascii_case("CASH") {
                continue;
            }

            match self
                .client
                .list_holdings(&self.user_id, &account.connection_id, &account.id)
                .await
            {
                Ok(holdings_dto) => {
                    let (balances, positions) = map_holdings_dto_to_broker(&holdings_dto);
                    match self
                        .sync_service
                        .save_broker_holdings(
                            local_account.id.clone(),
                            balances,
                            positions,
                            Vec::new(),
                        )
                        .await
                    {
                        Ok((_diff, created, _ids)) => {
                            holdings_synced += 1;
                            assets_created += created;
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to save holdings for account {}: {}",
                                local_account.id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to fetch holdings for account {}: {}",
                        local_account.id,
                        e
                    );
                }
            }
        }

        let provider = connections
            .first()
            .map(|connection| connection.provider.to_ascii_uppercase())
            .or_else(|| {
                accounts.first().and_then(|account| {
                    connection_lookup
                        .get(&account.connection_id)
                        .map(|connection| connection.provider.to_ascii_uppercase())
                })
            })
            .unwrap_or_else(|| "AGGREGATION".to_string());

        Ok(AggregationSyncResult {
            provider,
            connections_synced,
            accounts_synced,
            transactions_fetched,
            transactions_imported,
            assets_created,
            accounts_failed,
            accounts_warned,
            holdings_synced,
        })
    }

    async fn fetch_all_transactions(
        &self,
        connection_id: &str,
        account_id: &str,
    ) -> Result<Vec<TransactionDto>> {
        let mut cursor: Option<String> = None;
        let mut items = Vec::new();

        loop {
            let page = self
                .client
                .list_transactions(&self.user_id, connection_id, account_id, cursor.as_deref())
                .await?;

            let has_more = page.has_more;
            cursor = page.next_cursor.clone();
            items.extend(page.items);

            if !has_more || cursor.is_none() {
                break;
            }
        }

        Ok(items)
    }
}

fn map_connection_to_broker_connection(connection: &ConnectionDto) -> BrokerConnection {
    BrokerConnection {
        id: connection.id.clone(),
        brokerage: Some(BrokerConnectionBrokerage {
            id: Some(connection.connector_id.clone()),
            slug: Some(slugify_connector(&connection.connector_id)),
            name: Some(connection.connector_name.clone()),
            display_name: Some(connection.connector_name.clone()),
            aws_s3_logo_url: None,
            aws_s3_square_logo_url: None,
        }),
        connection_type: Some("read".to_string()),
        status: Some(connection.status.clone()),
        disabled: matches!(
            connection.status.as_str(),
            "failed" | "disconnected" | "disabled" | "reauth_required"
        ),
        disabled_date: None,
        updated_at: connection.last_synced_at.clone(),
        name: Some(connection.connector_name.clone()),
    }
}

fn map_account_to_broker_account(
    account: &AccountDto,
    connections: &HashMap<String, ConnectionDto>,
) -> BrokerAccount {
    let connection = connections.get(&account.connection_id);
    let institution_name = account
        .institution_name
        .clone()
        .or_else(|| connection.and_then(|item| item.institution_name.clone()))
        .or_else(|| connection.map(|item| item.connector_name.clone()));
    let provider = connection
        .map(|item| item.provider.to_ascii_uppercase())
        .unwrap_or_else(|| "AGGREGATION".to_string());

    BrokerAccount {
        id: Some(account.external_account_id.clone()),
        name: Some(account.name.clone()),
        account_number: account.mask.clone(),
        account_type: Some(normalize_account_type(&account.account_type)),
        currency: account.currency.clone(),
        balance: Some(BrokerAccountBalance {
            total: Some(BrokerBalanceTotal {
                amount: None,
                currency: account.currency.clone(),
            }),
        }),
        meta: None,
        owner: None,
        brokerage_authorization: Some(account.connection_id.clone()),
        institution_name,
        created_date: None,
        sync_status: None,
        status: Some("open".to_string()),
        raw_type: Some(account.account_type.to_ascii_uppercase()),
        is_paper: false,
        sync_enabled: true,
        shared_with_household: false,
        provider_type: Some(provider),
    }
}

fn map_transaction_to_broker_activity(
    transaction: &TransactionDto,
    account: &AccountDto,
    connection: Option<&ConnectionDto>,
) -> AccountUniversalActivity {
    let activity_type = normalize_transaction_type(&transaction.transaction_type);
    let symbol =
        transaction
            .security
            .as_ref()
            .map(|security| AccountUniversalActivitySymbol {
                id: Some(security.id.clone()),
                symbol: security.symbol.clone(),
                raw_symbol: security.symbol.clone(),
                description: security.name.clone(),
                symbol_type: Some(AccountUniversalActivitySymbolType {
                    id: None,
                    code: None,
                    description: None,
                    is_supported: Some(true),
                }),
                exchange: security.exchange.as_ref().map(|exchange| {
                    AccountUniversalActivityExchange {
                        id: None,
                        code: Some(exchange.clone()),
                        mic_code: Some(exchange.clone()),
                        name: Some(exchange.clone()),
                    }
                }),
                currency: security.currency.as_ref().map(|currency| {
                    AccountUniversalActivityCurrency {
                        id: None,
                        code: Some(currency.clone()),
                        name: Some(currency.clone()),
                    }
                }),
                figi_code: security.isin.clone(),
            });
    let provider = connection
        .map(|item| item.provider.to_ascii_uppercase())
        .unwrap_or_else(|| "AGGREGATION".to_string());

    AccountUniversalActivity {
        id: Some(transaction.id.clone()),
        symbol,
        option_symbol: None,
        price: parse_optional_f64(transaction.unit_price.as_deref()),
        units: parse_optional_f64(transaction.quantity.as_deref()),
        amount: parse_optional_f64(
            transaction
                .net_amount
                .as_deref()
                .or(transaction.gross_amount.as_deref()),
        ),
        currency: transaction
            .currency
            .as_ref()
            .map(|currency| AccountUniversalActivityCurrency {
                id: None,
                code: Some(currency.clone()),
                name: Some(currency.clone()),
            }),
        activity_type: Some(activity_type.clone()),
        subtype: None,
        raw_type: Some(transaction.transaction_type.clone()),
        option_type: None,
        description: transaction.description.clone(),
        trade_date: Some(transaction.booked_at.clone()),
        settlement_date: transaction.settled_at.clone(),
        fee: parse_optional_f64(transaction.fee.as_deref()),
        fx_rate: None,
        institution: account
            .institution_name
            .clone()
            .or_else(|| connection.and_then(|item| item.institution_name.clone())),
        external_reference_id: transaction.external_reference.clone(),
        provider_type: Some(provider.clone()),
        source_system: Some(provider),
        source_record_id: Some(transaction.id.clone()),
        source_group_id: None,
        mapping_metadata: None,
        needs_review: activity_type == "UNKNOWN",
    }
}

fn normalize_account_type(account_type: &str) -> String {
    match account_type.trim().to_ascii_lowercase().as_str() {
        "brokerage" => "INVESTMENT".to_string(),
        "retirement" => "IRA".to_string(),
        "cash" => "CASH".to_string(),
        "crypto" => "SECURITIES".to_string(),
        other if !other.is_empty() => other.to_ascii_uppercase(),
        _ => "SECURITIES".to_string(),
    }
}

fn normalize_transaction_type(transaction_type: &str) -> String {
    match transaction_type.trim().to_ascii_lowercase().as_str() {
        "buy" => "BUY".to_string(),
        "sell" => "SELL".to_string(),
        "dividend" => "DIVIDEND".to_string(),
        "interest" => "INTEREST".to_string(),
        "fee" => "FEE".to_string(),
        "tax" => "TAX".to_string(),
        "transfer_in" => "TRANSFER_IN".to_string(),
        "transfer_out" => "TRANSFER_OUT".to_string(),
        "cash_deposit" => "DEPOSIT".to_string(),
        "cash_withdrawal" => "WITHDRAWAL".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    value.and_then(|value| value.trim().parse::<f64>().ok())
}

fn slugify_connector(connector_id: &str) -> String {
    connector_id
        .trim()
        .to_ascii_lowercase()
        .replace([' ', ':'], "-")
}

fn map_holdings_dto_to_broker(
    holdings: &HoldingsDto,
) -> (
    Vec<crate::broker::HoldingsBalance>,
    Vec<crate::broker::HoldingsPosition>,
) {
    use crate::broker::{
        HoldingsBalance as BrokerBalance, HoldingsCurrency, HoldingsExchange, HoldingsInnerSymbol,
        HoldingsPosition as BrokerPosition, HoldingsSymbol,
    };

    let make_currency = |code: &str| HoldingsCurrency {
        id: None,
        code: Some(code.to_string()),
        name: None,
    };

    let balances = holdings
        .balances
        .iter()
        .map(|b| BrokerBalance {
            currency: Some(make_currency(&b.currency)),
            cash: b.cash.parse::<f64>().ok(),
            buying_power: None,
        })
        .collect();

    let positions = holdings
        .positions
        .iter()
        .map(|p| {
            let symbol_str = p
                .symbol
                .clone()
                .or_else(|| p.isin.clone())
                .unwrap_or_default();

            BrokerPosition {
                symbol: Some(HoldingsSymbol {
                    symbol: Some(HoldingsInnerSymbol {
                        id: None,
                        symbol: Some(symbol_str),
                        raw_symbol: p.isin.clone(),
                        description: p.name.clone(),
                        name: p.name.clone(),
                        currency: p.currency.as_ref().map(|c| make_currency(c)),
                        symbol_type: None,
                        exchange: p.exchange.as_ref().map(|e| HoldingsExchange {
                            id: None,
                            code: None,
                            mic_code: Some(e.clone()),
                            name: None,
                            suffix: None,
                        }),
                    }),
                    id: None,
                    description: p.name.clone(),
                }),
                units: p.quantity.parse::<f64>().ok(),
                price: parse_optional_f64(p.unit_price.as_deref()),
                open_pnl: None,
                average_purchase_price: parse_optional_f64(p.average_cost.as_deref()),
                currency: p.currency.as_ref().map(|c| make_currency(c)),
                cash_equivalent: Some(false),
            }
        })
        .collect();

    (balances, positions)
}
