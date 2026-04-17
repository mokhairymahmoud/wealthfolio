import type {
  AccountDto,
  ConnectionDto,
  ConnectorDto,
  HoldingsDto,
  SyncRunDto,
  TransactionPageDto,
} from "../provider-sync/dto";

export interface ListConnectionsInput {
  userId: string;
  connectionId?: string;
}

export interface ListAccountsInput {
  userId: string;
  connectionId?: string;
}

export interface ListTransactionsInput {
  userId: string;
  connectionId: string;
  accountId: string;
  cursor?: string;
}

export interface ListHoldingsInput {
  userId: string;
  connectionId: string;
  accountId: string;
}

export interface TriggerSyncInput {
  userId: string;
  connectionId?: string | null;
  mode: "initial" | "incremental" | "backfill" | "repair";
}

export interface DisableAccountInput {
  userId: string;
  accountId: string;
}

export interface DeleteConnectionInput {
  userId: string;
  connectionId: string;
}

export interface AggregationProvider {
  listConnectors(): Promise<ConnectorDto[]>;
  listConnections(input: ListConnectionsInput): Promise<ConnectionDto[]>;
  listAccounts(input: ListAccountsInput): Promise<AccountDto[]>;
  listTransactions(input: ListTransactionsInput): Promise<TransactionPageDto>;
  listHoldings(input: ListHoldingsInput): Promise<HoldingsDto>;
  triggerSync(input: TriggerSyncInput): Promise<SyncRunDto>;
  getSyncRun(runId: string): Promise<SyncRunDto | null>;
  disableAccount(input: DisableAccountInput): Promise<void>;
  deleteConnection(input: DeleteConnectionInput): Promise<void>;
}
