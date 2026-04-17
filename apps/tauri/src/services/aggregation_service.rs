use std::sync::Arc;

use wealthfolio_connect::{
    AggregationApiClient, AggregationProviderNotifier, AggregationStatus, AggregationSyncService,
    BrokerSyncServiceTrait,
};

pub fn aggregation_api_base_url() -> Option<String> {
    std::env::var("WF_AGGREGATION_API_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
}

pub fn aggregation_api_token() -> Option<String> {
    std::env::var("WF_AGGREGATION_API_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn aggregation_provider() -> String {
    std::env::var("WF_AGGREGATION_PROVIDER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "powens".to_string())
}

pub fn aggregation_user_id() -> String {
    std::env::var("WF_AGGREGATION_USER_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "local-user".to_string())
}

pub fn aggregation_enabled() -> bool {
    if let Ok(value) = std::env::var("WF_AGGREGATION_ENABLED") {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    } else {
        aggregation_api_base_url().is_some() && aggregation_api_token().is_some()
    }
}

pub fn aggregation_status() -> AggregationStatus {
    AggregationStatus {
        enabled: aggregation_enabled(),
        provider: aggregation_provider(),
    }
}

pub fn build_aggregation_sync_service(
    sync_service: Arc<dyn BrokerSyncServiceTrait>,
) -> Result<AggregationSyncService, String> {
    let base_url = aggregation_api_base_url().ok_or_else(|| {
        "Aggregation API base URL is unavailable. Provider Sync is disabled.".to_string()
    })?;
    let token = aggregation_api_token().ok_or_else(|| {
        "Aggregation API token is unavailable. Provider Sync is disabled.".to_string()
    })?;
    let client = AggregationApiClient::new(&base_url, &token).map_err(|e| e.to_string())?;

    Ok(AggregationSyncService::new(
        client,
        sync_service,
        aggregation_user_id(),
    ))
}

pub fn build_provider_notifier() -> Option<Arc<AggregationProviderNotifier>> {
    let base_url = aggregation_api_base_url()?;
    let token = aggregation_api_token()?;
    let client = AggregationApiClient::new(&base_url, &token).ok()?;
    Some(Arc::new(AggregationProviderNotifier::new(
        client,
        aggregation_user_id(),
    )))
}
