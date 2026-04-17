import { Injectable, Logger } from "@nestjs/common";
import { readFileSync } from "node:fs";

import { AppConfigService } from "../../config/config.service";
import type {
  AccountDto,
  ConnectionDto,
  ConnectorDto,
  HoldingsDto,
  SyncRunDto,
  TransactionPageDto,
} from "../../provider-sync/dto";
import type {
  AggregationProvider,
  ListAccountsInput,
  ListConnectionsInput,
  ListHoldingsInput,
  ListTransactionsInput,
  TriggerSyncInput,
} from "../provider.types";
import type { FixtureDataset } from "./fixture.types";

@Injectable()
export class FixtureProvider implements AggregationProvider {
  private readonly logger = new Logger(FixtureProvider.name);

  constructor(private readonly config: AppConfigService) {}

  async listConnectors(): Promise<ConnectorDto[]> {
    return this.loadData().connectors;
  }

  async listConnections(input: ListConnectionsInput): Promise<ConnectionDto[]> {
    const dataset = this.loadData();
    const scoped = dataset.connections.filter((connection) =>
      input.connectionId ? connection.id === input.connectionId : true,
    );

    return scoped;
  }

  async listAccounts(input: ListAccountsInput): Promise<AccountDto[]> {
    const dataset = this.loadData();
    return dataset.accounts.filter((account) =>
      input.connectionId ? account.connectionId === input.connectionId : true,
    );
  }

  async listTransactions(input: ListTransactionsInput): Promise<TransactionPageDto> {
    const dataset = this.loadData();
    const scoped = dataset.transactions.filter(
      (transaction) =>
        transaction.accountId === input.accountId &&
        (!input.fromDate || transaction.bookedAt >= input.fromDate) &&
        (!input.toDate || transaction.bookedAt <= input.toDate),
    );

    const pageSize = 200;
    const startIndex = input.cursor ? Number(input.cursor) || 0 : 0;
    const items = scoped.slice(startIndex, startIndex + pageSize).map((transaction) => ({
      ...transaction,
      transactionType: transaction.type,
    }));
    const nextIndex = startIndex + items.length;

    return {
      items,
      nextCursor: nextIndex < scoped.length ? String(nextIndex) : null,
      hasMore: nextIndex < scoped.length,
    };
  }

  async listHoldings(_input: ListHoldingsInput): Promise<HoldingsDto> {
    return {
      accountId: _input.accountId,
      positions: [],
      balances: [],
    };
  }

  async triggerSync(input: TriggerSyncInput): Promise<SyncRunDto> {
    const accounts = await this.listAccounts({
      userId: input.userId,
      connectionId: input.connectionId ?? undefined,
    });
    const transactions = this.loadData().transactions.filter((transaction) =>
      accounts.some((account) => account.id === transaction.accountId),
    );
    const now = new Date().toISOString();

    return {
      id: `fixture-run-${Date.now()}`,
      connectionId: input.connectionId ?? accounts[0]?.connectionId ?? "fixture-connection",
      mode: input.mode,
      status: "applied",
      startedAt: now,
      completedAt: now,
      summary: {
        accountsDiscovered: accounts.length,
        holdingsFetched: 0,
        transactionsFetched: transactions.length,
        transactionsImported: transactions.length,
        transactionsSkipped: 0,
        assetsCreated: 0,
        errors: 0,
      },
    };
  }

  async getSyncRun(runId: string): Promise<SyncRunDto | null> {
    return this.loadData().syncRuns?.find((run) => run.id === runId) ?? null;
  }

  async disableAccount(): Promise<void> {
    this.logger.debug("disableAccount is a no-op in fixture provider");
  }

  async deleteConnection(): Promise<void> {
    this.logger.debug("deleteConnection is a no-op in fixture provider");
  }

  private loadData(): FixtureDataset {
    const file = this.config.fixtureDataFile;
    const raw = readFileSync(file, "utf8");
    const parsed = JSON.parse(raw) as FixtureDataset;
    this.logger.debug(`Loaded fixture dataset from ${file}`);
    return parsed;
  }
}
