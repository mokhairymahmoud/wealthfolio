export interface ProviderConnector {
  id: string;
  provider: string;
  name: string;
  logoUrl: string | null;
  color: string | null;
  country: string | null;
  capabilities: string[];
  category: "banks" | "brokers" | "insurance" | "crypto" | "savings";
}

export interface ProviderSyncStatus {
  enabled: boolean;
  provider: string;
}

export interface ProviderSyncConnection {
  id: string;
  provider: string;
  connectorId: string;
  connectorName: string;
  institutionName: string | null;
  status: string;
  lastSyncedAt: string | null;
}

export interface ProviderSyncAccount {
  id: string;
  connectionId: string;
  externalAccountId: string;
  name: string;
  accountType: string;
  currency: string | null;
  institutionName: string | null;
  mask: string | null;
}

export interface ProviderSyncResult {
  provider: string;
  connectionsSynced: {
    synced: number;
    platformsCreated: number;
    platformsUpdated: number;
  };
  accountsSynced: {
    synced: number;
    created: number;
    updated: number;
    skipped: number;
  };
  transactionsFetched: number;
  transactionsImported: number;
  assetsCreated: number;
  accountsFailed: number;
  accountsWarned: number;
}
