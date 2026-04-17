//! Background schedulers for periodic broker and provider sync.
//!
//! Broker sync runs immediately on startup, then every 4 hours.
//! Provider (aggregation) sync runs after a 90-second stagger, then every 4 hours.

#[cfg(feature = "connect-sync")]
use std::sync::Arc;

#[cfg(feature = "connect-sync")]
use log::{debug, info, warn};
#[cfg(not(feature = "connect-sync"))]
use tauri::AppHandle;
#[cfg(feature = "connect-sync")]
use tauri::AppHandle;

#[cfg(feature = "connect-sync")]
use wealthfolio_core::quotes::MarketSyncMode;

#[cfg(feature = "connect-sync")]
use crate::commands::brokers_sync::perform_broker_sync;
use crate::context::ServiceContext;

/// Default sync interval: 4 hours.
#[cfg(feature = "connect-sync")]
const BROKER_SYNC_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// Provider sync stagger delay: 90 seconds after app start.
const PROVIDER_SYNC_INITIAL_DELAY_SECS: u64 = 90;

/// Default provider sync interval: 4 hours.
const PROVIDER_SYNC_INTERVAL_SECS: u64 = 4 * 60 * 60;

/// Runs broker sync periodically: once immediately, then every 4 hours.
#[cfg(feature = "connect-sync")]
pub async fn run_periodic_broker_sync(handle: &AppHandle, context: &Arc<ServiceContext>) {
    let mut consecutive_failures: u32 = 0;

    // First sync immediately (startup)
    match run_single_broker_sync(handle, context).await {
        Ok(()) => consecutive_failures = 0,
        Err(()) => consecutive_failures += 1,
    }

    // Periodic loop
    loop {
        let sleep_secs = if consecutive_failures > 0 {
            backoff_secs(BROKER_SYNC_INTERVAL_SECS, consecutive_failures)
        } else {
            BROKER_SYNC_INTERVAL_SECS
        };
        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;

        match run_single_broker_sync(handle, context).await {
            Ok(()) => consecutive_failures = 0,
            Err(()) => consecutive_failures += 1,
        }
    }
}

#[cfg(not(feature = "connect-sync"))]
pub async fn run_periodic_broker_sync(
    _handle: &AppHandle,
    _context: &std::sync::Arc<ServiceContext>,
) {
}

/// Runs provider/aggregation sync periodically (e.g., Powens).
/// Staggered 90 seconds from broker sync to avoid contention.
pub async fn run_periodic_provider_sync(context: &std::sync::Arc<ServiceContext>) {
    use crate::services::aggregation_service::{
        aggregation_enabled, build_aggregation_sync_service,
    };
    use log::{debug, info, warn};

    tokio::time::sleep(std::time::Duration::from_secs(
        PROVIDER_SYNC_INITIAL_DELAY_SECS,
    ))
    .await;

    let mut consecutive_failures: u32 = 0;

    loop {
        if aggregation_enabled() {
            match build_aggregation_sync_service(context.sync_service()) {
                Ok(service) => match service.sync(None).await {
                    Ok(result) => {
                        info!(
                            "Scheduled provider sync completed: {} transactions imported",
                            result.transactions_imported
                        );
                        consecutive_failures = 0;
                    }
                    Err(e) => {
                        warn!("Scheduled provider sync failed: {}", e);
                        consecutive_failures += 1;
                    }
                },
                Err(e) => {
                    debug!("Scheduled provider sync skipped: {}", e);
                    // Config error, not a transient failure — don't backoff
                }
            }
        } else {
            debug!("Scheduled provider sync skipped: aggregation not enabled");
        }

        let sleep_secs = if consecutive_failures > 0 {
            backoff_secs(PROVIDER_SYNC_INTERVAL_SECS, consecutive_failures)
        } else {
            PROVIDER_SYNC_INTERVAL_SECS
        };
        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }
}

/// Runs a single broker sync cycle. Returns Ok(()) on success/skip, Err(()) on failure.
#[cfg(feature = "connect-sync")]
async fn run_single_broker_sync(
    handle: &AppHandle,
    context: &Arc<ServiceContext>,
) -> Result<(), ()> {
    info!("Running scheduled broker sync...");

    // Check if user's plan includes broker sync
    match context.connect_service().has_broker_sync().await {
        Ok(true) => {}
        Ok(false) => {
            debug!("Scheduled broker sync skipped: plan does not include broker sync");
            return Ok(());
        }
        Err(e) => {
            debug!(
                "Scheduled broker sync skipped: could not verify broker sync access ({})",
                e
            );
            return Ok(());
        }
    }

    // Perform sync (orchestrator emits broker:sync-start and broker:sync-complete events)
    match perform_broker_sync(context, Some(handle)).await {
        Ok(result) => {
            info!(
                "Scheduled broker sync completed: success={}, message={}",
                result.success, result.message
            );

            if result.success {
                if let Some(ref activities) = result.activities_synced {
                    if activities.activities_upserted > 0 {
                        info!(
                            "Triggering portfolio update after broker sync ({} activities synced)",
                            activities.activities_upserted
                        );
                        crate::events::emit_portfolio_trigger_recalculate(
                            handle,
                            crate::events::PortfolioRequestPayload::builder()
                                .market_sync_mode(MarketSyncMode::Incremental { asset_ids: None })
                                .build(),
                        );
                    }
                }

                if let Some(ref holdings) = result.holdings_synced {
                    if holdings.positions_upserted > 0 {
                        info!(
                            "Triggering portfolio update after holdings sync ({} positions synced)",
                            holdings.positions_upserted
                        );
                        crate::events::emit_portfolio_trigger_recalculate(
                            handle,
                            crate::events::PortfolioRequestPayload::builder()
                                .market_sync_mode(MarketSyncMode::Incremental { asset_ids: None })
                                .build(),
                        );
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            if e.contains("No access token") || e.contains("not authenticated") {
                debug!("Scheduled broker sync skipped: user not authenticated");
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
