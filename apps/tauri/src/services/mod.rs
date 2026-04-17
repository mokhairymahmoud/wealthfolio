//! Application services for the Tauri app.

pub mod aggregation_service;
mod connect_service;

pub use aggregation_service::{
    aggregation_api_base_url, aggregation_api_token, aggregation_provider, aggregation_status,
    build_aggregation_sync_service, build_provider_notifier,
};
pub use connect_service::{cloud_api_base_url, ConnectService};
