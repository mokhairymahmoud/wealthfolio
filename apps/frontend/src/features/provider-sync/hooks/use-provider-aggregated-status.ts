import { useMemo } from "react";
import type { BrokerSyncState } from "@/features/wealthfolio-connect/types";
import { useProviderSyncStatus } from "./use-provider-sync-status";
import { useProviderSyncStates } from "./use-provider-sync-states";

export type ProviderAggregatedStatus = "disabled" | "idle" | "running" | "needs_review" | "failed";

function determineStatus(syncStates: BrokerSyncState[]): ProviderAggregatedStatus {
  if (syncStates.length === 0) return "idle";
  if (syncStates.some((s) => s.syncStatus === "RUNNING")) return "running";
  if (syncStates.some((s) => s.syncStatus === "NEEDS_REVIEW")) return "needs_review";
  if (syncStates.some((s) => s.syncStatus === "FAILED")) return "failed";
  return "idle";
}

export function useProviderAggregatedStatus() {
  const { data: providerStatus, isLoading: statusLoading } = useProviderSyncStatus();
  const enabled = providerStatus?.enabled === true;
  const { data: syncStates = [], isLoading: statesLoading } = useProviderSyncStates({ enabled });

  const status = useMemo<ProviderAggregatedStatus>(() => {
    if (!enabled) return "disabled";
    return determineStatus(syncStates);
  }, [enabled, syncStates]);

  const lastSyncTime = useMemo(() => {
    if (!enabled || syncStates.length === 0) return null;
    const successfulSyncs = syncStates
      .filter((s) => s.lastSuccessfulAt)
      .map((s) => new Date(s.lastSuccessfulAt!).getTime());
    if (successfulSyncs.length === 0) return null;
    return new Date(Math.max(...successfulSyncs)).toISOString();
  }, [enabled, syncStates]);

  const issueCount = useMemo(() => {
    return syncStates.filter((s) => s.syncStatus === "NEEDS_REVIEW" || s.syncStatus === "FAILED")
      .length;
  }, [syncStates]);

  return {
    status,
    enabled,
    provider: providerStatus?.provider ?? "",
    isLoading: statusLoading || statesLoading,
    lastSyncTime,
    hasIssues: issueCount > 0,
    issueCount,
    syncStates,
  };
}
