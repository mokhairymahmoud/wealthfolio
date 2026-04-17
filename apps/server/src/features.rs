use wealthfolio_connect::DEFAULT_CLOUD_API_URL;

pub fn connect_sync_enabled() -> bool {
    cfg!(feature = "connect-sync")
}

pub fn device_sync_enabled() -> bool {
    cfg!(feature = "device-sync")
}

pub fn cloud_sync_enabled() -> bool {
    connect_sync_enabled() || device_sync_enabled()
}

pub fn cloud_api_base_url() -> Option<String> {
    if !cloud_sync_enabled() {
        return None;
    }

    std::env::var("CONNECT_API_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| Some(DEFAULT_CLOUD_API_URL.to_string()))
}

// ── Aggregation / provider sync helpers ──────────────────────────────────────

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
