//! Background schedulers for periodic broker and provider sync.
//!
//! Broker sync: 4-hour interval with retry backoff.
//! Provider (aggregation) sync: 4-hour interval, staggered 90 seconds.

use std::sync::Arc;

#[cfg(feature = "connect-sync")]
use tokio::time::Duration;
#[cfg(not(feature = "connect-sync"))]
use tracing::info;
#[cfg(feature = "connect-sync")]
use tracing::{debug, info, warn};

#[cfg(feature = "connect-sync")]
use crate::api::connect::{has_broker_sync, perform_broker_sync};
use crate::main_lib::AppState;

/// Broker sync interval: 4 hours.
#[cfg(feature = "connect-sync")]
const BROKER_SYNC_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// Initial delay before first broker sync (60 seconds to let server fully start).
#[cfg(feature = "connect-sync")]
const BROKER_INITIAL_DELAY_SECS: u64 = 60;

/// Provider sync stagger delay: 90 seconds after server start.
const PROVIDER_SYNC_INITIAL_DELAY_SECS: u64 = 90;

/// Provider sync interval: 4 hours.
const PROVIDER_SYNC_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// Starts the background broker sync scheduler.
#[cfg(feature = "connect-sync")]
pub fn start_broker_sync_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Broker sync scheduler started (4-hour interval)");

        tokio::time::sleep(Duration::from_secs(BROKER_INITIAL_DELAY_SECS)).await;

        let mut consecutive_failures: u32 = 0;

        loop {
            match run_scheduled_broker_sync(&state).await {
                Ok(()) => {
                    consecutive_failures = 0;
                    tokio::time::sleep(Duration::from_secs(BROKER_SYNC_INTERVAL_SECS)).await;
                }
                Err(()) => {
                    consecutive_failures += 1;
                    let backoff = backoff_secs(BROKER_SYNC_INTERVAL_SECS, consecutive_failures);
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                }
            }
        }
    });
}

/// Starts the background broker sync scheduler (no-op when feature disabled).
#[cfg(not(feature = "connect-sync"))]
pub fn start_broker_sync_scheduler(_state: Arc<AppState>) {
    info!("Broker sync scheduler disabled: connect-sync feature is not compiled");
}

/// Starts the background provider/aggregation sync scheduler.
pub fn start_provider_sync_scheduler(state: Arc<AppState>) {
    use crate::features;
    use tracing::{debug as trace_debug, info as trace_info, warn as trace_warn};
    use wealthfolio_connect::{AggregationApiClient, AggregationSyncService};

    if !features::aggregation_enabled() {
        trace_info!("Provider sync scheduler disabled: aggregation not configured");
        return;
    }

    tokio::spawn(async move {
        trace_info!("Provider sync scheduler started (4-hour interval)");

        tokio::time::sleep(tokio::time::Duration::from_secs(
            PROVIDER_SYNC_INITIAL_DELAY_SECS,
        ))
        .await;

        let mut consecutive_failures: u32 = 0;

        loop {
            // Re-check each iteration in case env vars changed
            if features::aggregation_enabled() {
                let result = (|| {
                    let base_url = features::aggregation_api_base_url()
                        .ok_or_else(|| "Aggregation API URL unavailable".to_string())?;
                    let token = features::aggregation_api_token()
                        .ok_or_else(|| "Aggregation API token unavailable".to_string())?;
                    let client =
                        AggregationApiClient::new(&base_url, &token).map_err(|e| e.to_string())?;
                    Ok::<_, String>(AggregationSyncService::new(
                        client,
                        state.connect_sync_service.clone(),
                        features::aggregation_user_id(),
                    ))
                })();

                match result {
                    Ok(service) => match service.sync(None).await {
                        Ok(sync_result) => {
                            trace_info!(
                                "Scheduled provider sync completed: {} transactions imported",
                                sync_result.transactions_imported
                            );
                            consecutive_failures = 0;
                        }
                        Err(e) => {
                            trace_warn!("Scheduled provider sync failed: {}", e);
                            consecutive_failures += 1;
                        }
                    },
                    Err(e) => {
                        trace_debug!("Scheduled provider sync skipped: {}", e);
                    }
                }
            } else {
                trace_debug!("Scheduled provider sync skipped: aggregation not enabled");
            }

            let sleep_secs = if consecutive_failures > 0 {
                backoff_secs(PROVIDER_SYNC_INTERVAL_SECS, consecutive_failures)
            } else {
                PROVIDER_SYNC_INTERVAL_SECS
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;
        }
    });
}

/// Runs a single scheduled broker sync. Returns Ok(()) on success/skip, Err(()) on failure.
#[cfg(feature = "connect-sync")]
async fn run_scheduled_broker_sync(state: &Arc<AppState>) -> Result<(), ()> {
    info!("Running scheduled broker sync...");

    let has_token = state
        .secret_store
        .get_secret("sync_refresh_token")
        .map(|t| t.is_some())
        .unwrap_or(false);

    if !has_token {
        debug!("Scheduled sync skipped: no refresh token configured");
        return Ok(());
    }

    match has_broker_sync(state).await {
        Ok(true) => {}
        Ok(false) => {
            debug!("Scheduled sync skipped: plan does not include broker sync");
            return Ok(());
        }
        Err(e) => {
            debug!(
                "Scheduled sync skipped: could not verify broker sync access ({})",
                e
            );
            return Ok(());
        }
    }

    match perform_broker_sync(state).await {
        Ok(result) => {
            let activities_count = result
                .activities_synced
                .as_ref()
                .map(|a| a.activities_upserted)
                .unwrap_or(0);
            info!(
                "Scheduled broker sync completed: {} activities synced",
                activities_count
            );
            Ok(())
        }
        Err(e) => {
            if e.contains("No refresh token")
                || e.contains("not authenticated")
                || e.contains("Session expired")
            {
                debug!("Scheduled sync skipped: user not authenticated");
                Ok(())
            } else {
                warn!("Scheduled broker sync failed: {}", e);
                Err(())
            }
        }
    }
}

/// Computes backoff sleep duration: min(normal_interval, 60s * 2^failures), capped at 6 doublings.
fn backoff_secs(normal_interval_secs: u64, consecutive_failures: u32) -> u64 {
    let capped = consecutive_failures.min(6);
    let backoff = 60u64.saturating_mul(1u64 << capped);
    backoff.min(normal_interval_secs)
}
