use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use wealthfolio_core::accounts::AccountServiceTrait;

use crate::{
    error::{ApiError, ApiResult},
    events::ServerEvent,
    features,
    main_lib::AppState,
};
use wealthfolio_connect::{
    AggregationApiClient, AggregationStatus, AggregationSyncService, ConnectUrlResponse,
    ConnectorDto,
};

const PROVIDER_SYNC_START: &str = "provider:sync-start";
const PROVIDER_SYNC_COMPLETE: &str = "provider:sync-complete";
const PROVIDER_SYNC_ERROR: &str = "provider:sync-error";

fn ensure_provider_sync_enabled() -> ApiResult<()> {
    if features::aggregation_enabled() {
        Ok(())
    } else {
        Err(ApiError::NotImplemented(
            "Provider Sync is not configured for this build.".to_string(),
        ))
    }
}

fn provider_matches(value: &str) -> bool {
    value.eq_ignore_ascii_case(&features::aggregation_provider())
}

fn build_service(state: &Arc<AppState>) -> ApiResult<AggregationSyncService> {
    ensure_provider_sync_enabled()?;
    let base_url = features::aggregation_api_base_url().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API URL is unavailable.".to_string())
    })?;
    let token = features::aggregation_api_token().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API token is unavailable.".to_string())
    })?;
    let client = AggregationApiClient::new(&base_url, &token)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(AggregationSyncService::new(
        client,
        state.connect_sync_service.clone(),
        features::aggregation_user_id(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRunsQuery {
    run_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_provider_connectors(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ConnectorDto>>> {
    let service = build_service(&state)?;
    let connectors = service
        .list_connectors()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(connectors))
}

async fn get_provider_sync_status() -> Json<AggregationStatus> {
    Json(AggregationStatus {
        enabled: features::aggregation_enabled(),
        provider: features::aggregation_provider(),
    })
}

async fn list_provider_sync_connections(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<wealthfolio_connect::AggregationConnection>>> {
    let service = build_service(&state)?;
    let connections = service
        .list_connections()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(connections))
}

async fn list_provider_sync_accounts(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<wealthfolio_connect::AggregationAccount>>> {
    let service = build_service(&state)?;
    let accounts = service
        .list_accounts(None)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(accounts))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncProviderDataRequest {
    connection_id: Option<String>,
}

async fn sync_provider_data(
    State(state): State<Arc<AppState>>,
    payload: Option<Json<SyncProviderDataRequest>>,
) -> ApiResult<Json<()>> {
    let service = build_service(&state)?;
    let event_bus = state.event_bus.clone();
    let connection_id = payload.and_then(|Json(payload)| {
        payload
            .connection_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    });

    event_bus.publish(ServerEvent::new(PROVIDER_SYNC_START));

    tokio::spawn(async move {
        match service.sync(connection_id.as_deref()).await {
            Ok(result) => {
                event_bus.publish(ServerEvent::with_payload(
                    PROVIDER_SYNC_COMPLETE,
                    json!(result),
                ));
            }
            Err(e) => {
                event_bus.publish(ServerEvent::with_payload(
                    PROVIDER_SYNC_ERROR,
                    json!({ "error": e.to_string() }),
                ));
            }
        }
    });

    Ok(Json(()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteConnectionRequest {
    connection_id: String,
}

async fn delete_provider_connection(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteConnectionRequest>,
) -> ApiResult<Json<()>> {
    ensure_provider_sync_enabled()?;
    let base_url = features::aggregation_api_base_url().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API URL is unavailable.".to_string())
    })?;
    let token = features::aggregation_api_token().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API token is unavailable.".to_string())
    })?;
    let client = AggregationApiClient::new(&base_url, &token)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let user_id = features::aggregation_user_id();

    // Delete local accounts linked to this connection's remote accounts
    let remote_accounts = client
        .list_accounts(&user_id, Some(&req.connection_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let remote_ids: std::collections::HashSet<String> = remote_accounts
        .iter()
        .map(|a| a.external_account_id.clone())
        .collect();

    let local_accounts = state
        .account_service
        .get_all_accounts()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    for account in &local_accounts {
        if let Some(ref provider_account_id) = account.provider_account_id {
            if remote_ids.contains(provider_account_id) {
                if let Err(e) = state.account_service.delete_account(&account.id).await {
                    tracing::warn!(
                        "Failed to delete local account {} during connection disconnect: {}",
                        account.id,
                        e
                    );
                }
            }
        }
    }

    // Delete the connection on the provider
    client
        .delete_connection(&user_id, &req.connection_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectUrlQuery {
    connector_id: Option<String>,
    redirect_uri: Option<String>,
}

async fn get_provider_connect_url(
    Query(query): Query<ConnectUrlQuery>,
) -> ApiResult<Json<ConnectUrlResponse>> {
    ensure_provider_sync_enabled()?;
    let base_url = features::aggregation_api_base_url().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API URL is unavailable.".to_string())
    })?;
    let token = features::aggregation_api_token().ok_or_else(|| {
        ApiError::NotImplemented("Provider Sync API token is unavailable.".to_string())
    })?;
    let client = AggregationApiClient::new(&base_url, &token)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let result = client
        .get_connect_url(query.connector_id.as_deref(), query.redirect_uri.as_deref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(result))
}

async fn get_provider_synced_accounts(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<crate::models::Account>>> {
    let accounts = state
        .connect_sync_service
        .get_synced_accounts()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(
        accounts
            .into_iter()
            .filter(|account| account.provider.as_deref().is_some_and(provider_matches))
            .map(Into::into)
            .collect(),
    ))
}

async fn get_provider_sync_states(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<wealthfolio_connect::BrokerSyncState>>> {
    let states = state
        .connect_sync_service
        .get_all_sync_states()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(
        states
            .into_iter()
            .filter(|sync_state| provider_matches(&sync_state.provider))
            .collect(),
    ))
}

async fn get_provider_sync_import_runs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ImportRunsQuery>,
) -> ApiResult<Json<Vec<wealthfolio_connect::ImportRun>>> {
    let runs = state
        .connect_sync_service
        .get_import_runs(
            query.run_type.as_deref(),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(
        runs.into_iter()
            .filter(|run| provider_matches(&run.source_system))
            .collect(),
    ))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/provider-sync/connectors", get(list_provider_connectors))
        .route("/provider-sync/status", get(get_provider_sync_status))
        .route(
            "/provider-sync/connections",
            get(list_provider_sync_connections),
        )
        .route("/provider-sync/accounts", get(list_provider_sync_accounts))
        .route("/provider-sync/sync", post(sync_provider_data))
        .route(
            "/provider-sync/synced-accounts",
            get(get_provider_synced_accounts),
        )
        .route("/provider-sync/sync-states", get(get_provider_sync_states))
        .route(
            "/provider-sync/import-runs",
            get(get_provider_sync_import_runs),
        )
        .route("/provider-sync/connect-url", get(get_provider_connect_url))
        .route(
            "/provider-sync/connections/delete",
            post(delete_provider_connection),
        )
}
