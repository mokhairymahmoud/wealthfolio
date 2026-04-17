import type {
  AccountDto,
  ConnectionDto,
  ConnectorDto,
  SyncRunDto,
  TransactionDto,
} from "../../provider-sync/dto";

export interface FixtureTransaction
  extends Omit<TransactionDto, "transactionType"> {
  type: TransactionDto["transactionType"];
}

export interface FixtureDataset {
  connectors: ConnectorDto[];
  connections: ConnectionDto[];
  accounts: AccountDto[];
  transactions: FixtureTransaction[];
  syncRuns?: SyncRunDto[];
}
