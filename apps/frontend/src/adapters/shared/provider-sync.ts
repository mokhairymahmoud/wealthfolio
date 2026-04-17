import type { Account } from "@/lib/types";
import type { BrokerSyncState, ImportRun } from "@/features/wealthfolio-connect/types";
import type {
  ProviderConnector,
  ProviderSyncAccount,
  ProviderSyncConnection,
  ProviderSyncStatus,
  SyncProviderDataRequest,
} from "@/features/provider-sync/types";
import type { ImportRunsRequest } from "../types";
import { invoke } from "./platform";

export async function listProviderConnectors(): Promise<ProviderConnector[]> {
  return invoke<ProviderConnector[]>("list_provider_connectors");
}

export async function getProviderSyncStatus(): Promise<ProviderSyncStatus> {
  return invoke<ProviderSyncStatus>("get_provider_sync_status");
}

export async function listProviderSyncConnections(): Promise<ProviderSyncConnection[]> {
  return invoke<ProviderSyncConnection[]>("list_provider_sync_connections");
}

export async function listProviderSyncAccounts(): Promise<ProviderSyncAccount[]> {
  return invoke<ProviderSyncAccount[]>("list_provider_sync_accounts");
}

export async function syncProviderData(
  request?: string | SyncProviderDataRequest,
): Promise<void> {
  const payload = typeof request === "string" ? { connectionId: request } : (request ?? {});
  return invoke<void>("sync_provider_data", payload as Record<string, unknown>);
}

export async function getProviderSyncedAccounts(): Promise<Account[]> {
  return invoke<Account[]>("get_provider_synced_accounts");
}

export async function getProviderSyncStates(): Promise<BrokerSyncState[]> {
  return invoke<BrokerSyncState[]>("get_provider_sync_states");
}

export async function getProviderSyncImportRuns(request?: ImportRunsRequest): Promise<ImportRun[]> {
  return invoke<ImportRun[]>("get_provider_sync_import_runs", {
    runType: request?.runType,
    limit: request?.limit,
    offset: request?.offset,
  });
}

export async function getProviderConnectUrl(
  connectorId?: string,
  redirectUri?: string,
): Promise<{ url: string }> {
  return invoke<{ url: string }>("get_provider_connect_url", { connectorId, redirectUri });
}

export async function deleteProviderConnection(connectionId: string): Promise<void> {
  return invoke<void>("delete_provider_connection", { connectionId });
}
