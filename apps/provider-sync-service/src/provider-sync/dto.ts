export interface ConnectorDto {
  id: string;
  provider: string;
  name: string;
  logoUrl: string | null;
  color: string | null;
  country: string | null;
  capabilities: string[];
  category: "banks" | "brokers" | "insurance" | "crypto" | "savings";
}

export interface ConnectionDto {
  id: string;
  provider: string;
  connectorId: string;
  connectorName: string;
  institutionName: string | null;
  status: "pending" | "connected" | "degraded" | "reauth_required" | "failed";
  lastSyncedAt: string | null;
}

export interface AccountDto {
  id: string;
  connectionId: string;
  externalAccountId: string;
  name: string;
  accountType: "brokerage" | "retirement" | "cash" | "crypto" | "other";
  currency: string | null;
  institutionName: string | null;
  mask: string | null;
}

export interface SecurityDto {
  id: string;
  symbol: string | null;
  isin: string | null;
  name: string | null;
  currency: string | null;
  exchange: string | null;
}

export interface TransactionDto {
  id: string;
  accountId: string;
  security: SecurityDto | null;
  bookedAt: string;
  settledAt: string | null;
  transactionType:
    | "buy"
    | "sell"
    | "dividend"
    | "interest"
    | "fee"
    | "tax"
    | "transfer_in"
    | "transfer_out"
    | "cash_deposit"
    | "cash_withdrawal"
    | "unknown";
  quantity: string | null;
  unitPrice: string | null;
  grossAmount: string | null;
  netAmount: string | null;
  fee: string | null;
  currency: string | null;
  description: string | null;
  externalReference: string | null;
}

export interface TransactionPageDto {
  items: TransactionDto[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface HoldingsPositionDto {
  symbol: string | null;
  isin: string | null;
  name: string | null;
  quantity: string;
  unitPrice: string | null;
  averageCost: string | null;
  currency: string | null;
  exchange: string | null;
}

export interface HoldingsBalanceDto {
  currency: string;
  cash: string;
}

export interface HoldingsDto {
  accountId: string;
  positions: HoldingsPositionDto[];
  balances: HoldingsBalanceDto[];
}

export interface SyncRunDto {
  id: string;
  connectionId: string;
  mode: "initial" | "incremental" | "backfill" | "repair";
  status: "running" | "applied" | "needs_review" | "failed" | "cancelled";
  startedAt: string;
  completedAt: string | null;
  summary: {
    accountsDiscovered: number;
    holdingsFetched: number;
    transactionsFetched: number;
    transactionsImported: number;
    transactionsSkipped: number;
    assetsCreated: number;
    errors: number;
  } | null;
}

export interface SyncRequestDto {
  userId: string;
  connectionId: string | null;
  mode: "initial" | "incremental" | "backfill" | "repair";
}
