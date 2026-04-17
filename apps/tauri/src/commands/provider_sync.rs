use std::sync::Arc;

use log::{error, warn};
use tauri::{AppHandle, Emitter, State};

use crate::context::ServiceContext;
use crate::events::{PROVIDER_SYNC_COMPLETE, PROVIDER_SYNC_ERROR, PROVIDER_SYNC_START};
use crate::services;
use wealthfolio_connect::{
    AggregationAccount, AggregationConnection, AggregationStatus, AggregationSyncMode,
    AggregationSyncOptions, BrokerSyncState, ConnectUrlResponse, ConnectorDto, ImportRun,
};

fn provider_matches(value: &str) -> bool {
    value.eq_ignore_ascii_case(&services::aggregation_provider())
}

fn aggregation_sync_mode(value: Option<&str>) -> AggregationSyncMode {
    match value {
        Some(value) if value.eq_ignore_ascii_case("backfill") => AggregationSyncMode::Backfill,
        _ => AggregationSyncMode::Incremental,
    }
}

#[tauri::command]
pub async fn list_provider_connectors(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<ConnectorDto>, String> {
    let service = services::build_aggregation_sync_service(state.sync_service())?;
    service.list_connectors().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_sync_status() -> Result<AggregationStatus, String> {
    Ok(services::aggregation_status())
}

#[tauri::command]
pub async fn list_provider_sync_connections(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<AggregationConnection>, String> {
    let service = services::build_aggregation_sync_service(state.sync_service())?;
    service.list_connections().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_provider_sync_accounts(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<AggregationAccount>, String> {
    let service = services::build_aggregation_sync_service(state.sync_service())?;
    service.list_accounts(None).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_provider_data(
    app: AppHandle,
    state: State<'_, Arc<ServiceContext>>,
    connection_id: Option<String>,
    mode: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<(), String> {
    let sync_service = state.sync_service();
    let service = services::build_aggregation_sync_service(sync_service)?;
    let options = AggregationSyncOptions {
        connection_id: connection_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        mode: aggregation_sync_mode(mode.as_deref()),
        from_date: from_date
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        to_date: to_date
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    };

    // Emit start event
    app.emit(PROVIDER_SYNC_START, ())
        .unwrap_or_else(|e| error!("Failed to emit provider:sync-start: {}", e));

    // Spawn the sync as a background task
    tauri::async_runtime::spawn(async move {
        match service.sync_with_options(options).await {
            Ok(result) => {
                app.emit(PROVIDER_SYNC_COMPLETE, &result)
                    .unwrap_or_else(|e| {
                        error!("Failed to emit provider:sync-complete: {}", e);
                    });
            }
            Err(e) => {
                app.emit(
                    PROVIDER_SYNC_ERROR,
                    serde_json::json!({ "error": e.to_string() }),
                )
                .unwrap_or_else(|err| {
                    error!("Failed to emit provider:sync-error: {}", err);
                });
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn delete_provider_connection(
    state: State<'_, Arc<ServiceContext>>,
    connection_id: String,
) -> Result<(), String> {
    let base_url = services::aggregation_api_base_url()
        .ok_or_else(|| "Aggregation API URL is unavailable.".to_string())?;
    let token = services::aggregation_api_token()
        .ok_or_else(|| "Aggregation API token is unavailable.".to_string())?;
    let client = wealthfolio_connect::AggregationApiClient::new(&base_url, &token)
        .map_err(|e| e.to_string())?;
    let user_id = services::aggregation_service::aggregation_user_id();

    // Find local accounts linked to this connection's remote accounts
    let remote_accounts = client
        .list_accounts(&user_id, Some(&connection_id))
        .await
        .map_err(|e| e.to_string())?;
    let remote_ids: std::collections::HashSet<String> = remote_accounts
        .iter()
        .map(|a| a.external_account_id.clone())
        .collect();

    let account_service = state.account_service();
    let local_accounts = account_service
        .get_all_accounts()
        .map_err(|e| e.to_string())?;
    for account in &local_accounts {
        if let Some(ref provider_account_id) = account.provider_account_id {
            if remote_ids.contains(provider_account_id) {
                if let Err(e) = account_service.delete_account(&account.id).await {
                    warn!(
                        "Failed to delete local account {} during connection disconnect: {}",
                        account.id, e
                    );
                }
            }
        }
    }

    // Delete the connection on the provider
    client
        .delete_connection(&user_id, &connection_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_connect_url(
    connector_id: Option<String>,
    redirect_uri: Option<String>,
) -> Result<ConnectUrlResponse, String> {
    let base_url = services::aggregation_api_base_url()
        .ok_or_else(|| "Aggregation API URL is unavailable.".to_string())?;
    let token = services::aggregation_api_token()
        .ok_or_else(|| "Aggregation API token is unavailable.".to_string())?;
    let client = wealthfolio_connect::AggregationApiClient::new(&base_url, &token)
        .map_err(|e| e.to_string())?;
    client
        .get_connect_url(connector_id.as_deref(), redirect_uri.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_synced_accounts(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<wealthfolio_core::accounts::Account>, String> {
    state
        .sync_service()
        .get_synced_accounts()
        .map(|accounts| {
            accounts
                .into_iter()
                .filter(|account| account.provider.as_deref().is_some_and(provider_matches))
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_sync_states(
    state: State<'_, Arc<ServiceContext>>,
) -> Result<Vec<BrokerSyncState>, String> {
    state
        .sync_service()
        .get_all_sync_states()
        .map(|states| {
            states
                .into_iter()
                .filter(|sync_state| provider_matches(&sync_state.provider))
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_provider_sync_import_runs(
    state: State<'_, Arc<ServiceContext>>,
    run_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ImportRun>, String> {
    state
        .sync_service()
        .get_import_runs(
            run_type.as_deref(),
            limit.unwrap_or(50),
            offset.unwrap_or(0),
        )
        .map(|runs| {
            runs.into_iter()
                .filter(|run| provider_matches(&run.source_system))
                .collect()
        })
        .map_err(|e| e.to_string())
}
