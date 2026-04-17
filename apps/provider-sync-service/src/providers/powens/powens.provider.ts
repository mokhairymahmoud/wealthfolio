import { Injectable, InternalServerErrorException } from "@nestjs/common";

import { AppConfigService } from "../../config/config.service";
import type {
  AccountDto,
  ConnectionDto,
  ConnectorDto,
  HoldingsDto,
  SyncRunDto,
  TransactionDto,
  TransactionPageDto,
} from "../../provider-sync/dto";
import type {
  AggregationProvider,
  DeleteConnectionInput,
  DisableAccountInput,
  ListAccountsInput,
  ListConnectionsInput,
  ListHoldingsInput,
  ListTransactionsInput,
  TriggerSyncInput,
} from "../provider.types";

interface PowensTokenResponse {
  access_token?: string;
  token?: string;
}

interface PowensConnectorsResponse {
  connectors?: PowensConnector[];
}

interface PowensConnector {
  uuid?: string;
  id?: number;
  name?: string;
  display_name?: string;
  slug?: string;
  color?: string;
  country?: string;
  country_code?: string;
  capabilities?: string[];
  account_types?: string[];
  products?: string[];
}

interface PowensConnectionsResponse {
  connections?: PowensConnection[];
}

interface PowensConnection {
  id?: number;
  id_connector?: number;
  connector_uuid?: string;
  state?: string | null;
  error_message?: string | null;
  last_update?: string | null;
  active?: boolean;
  connector?: PowensConnector;
}

interface PowensAccountsResponse {
  accounts?: PowensAccount[];
}

interface PowensCurrency {
  id?: string;
  symbol?: string;
}

interface PowensAccount {
  id?: number;
  id_connection?: number;
  number?: string | null;
  iban?: string | null;
  label?: string | null;
  currency?: string | PowensCurrency | null;
  type?: string | PowensAccountType | null;
  name?: string | null;
  balance?: string | number | null;
  coming?: string | number | null;
  disabled?: string | null;
}

interface PowensAccountType {
  id?: number;
  name?: string | null;
  is_invest?: boolean;
}

interface PowensMarketOrdersResponse {
  marketorders?: PowensMarketOrder[];
}

interface PowensLink {
  href?: string | null;
}

interface PowensPaginationLinks {
  next?: PowensLink | null;
}

interface PowensBankTransactionsResponse {
  transactions?: PowensBankTransaction[];
  _links?: PowensPaginationLinks | null;
}

interface PowensBankTransaction {
  id?: number;
  id_account?: number;
  application_date?: string | null;
  date?: string | null;
  datetime?: string | null;
  vdate?: string | null;
  vdatetime?: string | null;
  rdate?: string | null;
  rdatetime?: string | null;
  value?: string | number | null;
  gross_value?: string | number | null;
  type?: string | null;
  original_wording?: string | null;
  simplified_wording?: string | null;
  wording?: string | null;
  original_currency?: string | PowensCurrency | null;
  commission?: string | number | null;
  commission_currency?: string | PowensCurrency | null;
  active?: boolean;
  deleted?: string | null;
  coming?: boolean;
  id_cluster?: number | null;
}

interface PowensMarketOrder {
  id?: number;
  id_account?: number;
  label?: string | null;
  code?: string | null;
  stock_symbol?: string | null;
  stock_market?: string | null;
  date?: string | null;
  datetime?: string | null;
  quantity?: string | number | null;
  unitprice?: string | number | null;
  unitvalue?: string | number | null;
  value?: string | number | null;
  operation?: string | null;
  order_type?: string | null;
  currency?: string | null;
  original_currency?: string | null;
  commission?: string | number | null;
  isin?: string | null;
  investment?: PowensInvestment | null;
}

interface PowensInvestment {
  label?: string | null;
  code?: string | null;
  code_type?: string | null;
  stock_symbol?: string | null;
  stock_market?: string | null;
  description?: string | null;
}

interface PowensInvestmentsResponse {
  investments?: PowensInvestmentPosition[];
}

interface PowensInvestmentPosition {
  id?: number;
  id_account?: number;
  label?: string | null;
  code?: string | null;
  code_type?: string | null;
  stock_symbol?: string | null;
  stock_market?: string | null;
  quantity?: string | number | null;
  unitprice?: string | number | null;
  unitvalue?: string | number | null;
  valuation?: string | number | null;
  original_currency?: string | null;
  description?: string | null;
}

@Injectable()
export class PowensProvider implements AggregationProvider {
  private readonly syncRuns = new Map<string, SyncRunDto>();

  constructor(private readonly config: AppConfigService) {}

  async listConnectors(): Promise<ConnectorDto[]> {
    const response = await this.request<PowensConnectorsResponse>("/connectors", {
      query: this.config.powensCountryCodes
        ? { country_codes: this.config.powensCountryCodes }
        : undefined,
    });

    return (response.connectors ?? []).map((connector) => {
      const capabilities = connector.capabilities ?? [];
      const accountTypes = connector.account_types ?? [];
      return {
        id: connector.uuid ?? String(connector.id ?? ""),
        provider: "powens",
        name: connector.display_name ?? connector.name ?? "Unknown connector",
        logoUrl: null,
        color: connector.color ?? null,
        country: connector.country_code ?? connector.country ?? null,
        capabilities,
        category: this.deriveCategory(accountTypes),
      };
    });
  }

  async listConnections(input: ListConnectionsInput): Promise<ConnectionDto[]> {
    const userId = this.resolveUserId(input.userId);
    const response = await this.request<PowensConnectionsResponse>(`/users/${userId}/connections`, {
      auth: "user",
      query: { expand: "connector" },
    });

    return (response.connections ?? [])
      .filter((connection) =>
        input.connectionId ? String(connection.id) === input.connectionId : true,
      )
      .map((connection) => this.mapConnection(connection));
  }

  async listAccounts(input: ListAccountsInput): Promise<AccountDto[]> {
    const userId = this.resolveUserId(input.userId);
    const connections = await this.listConnections({ userId, connectionId: input.connectionId });
    const connectorNames = new Map(connections.map((c) => [c.id, c.connectorName]));

    const accounts = await Promise.all(
      connections.map(async (connection) => {
        const response = await this.request<PowensAccountsResponse>(
          `/users/${userId}/connections/${connection.id}/accounts`,
          {
            auth: "user",
            query: { all: "true" },
          },
        );

        const institutionName = connectorNames.get(connection.id) ?? null;
        return (response.accounts ?? [])
          .filter((account) => !account.disabled)
          .map((account) => this.mapAccount(connection.id, account, institutionName));
      }),
    );

    return accounts.flat();
  }

  async listTransactions(input: ListTransactionsInput): Promise<TransactionPageDto> {
    const userId = this.resolveUserId(input.userId);
    const rawAccount = await this.request<PowensAccount>(
      `/users/${userId}/accounts/${input.accountId}`,
      { auth: "user", query: { all: "true" } },
    );

    if (this.mapAccountType(rawAccount.type) === "cash") {
      return this.listBankTransactions(input, rawAccount);
    }

    return this.listMarketOrders(input);
  }

  private async listMarketOrders(input: ListTransactionsInput): Promise<TransactionPageDto> {
    const userId = this.resolveUserId(input.userId);
    const offset = Number(input.cursor ?? "0") || 0;
    const limit = 200;
    const response = await this.request<PowensMarketOrdersResponse>(
      `/users/${userId}/accounts/${input.accountId}/marketorders`,
      {
        auth: "user",
        query: {
          limit: String(limit),
          offset: String(offset),
        },
      },
    );

    const items = (response.marketorders ?? []).map((order) =>
      this.mapMarketOrder(input.accountId, order),
    );
    const nextOffset = offset + items.length;

    return {
      items,
      nextCursor: items.length === limit ? String(nextOffset) : null,
      hasMore: items.length === limit,
    };
  }

  private async listBankTransactions(
    input: ListTransactionsInput,
    rawAccount: PowensAccount,
  ): Promise<TransactionPageDto> {
    const userId = this.resolveUserId(input.userId);
    const limit = 1000;
    const accountCurrency = this.currencyCode(rawAccount.currency);
    const response = await this.request<PowensBankTransactionsResponse>(
      input.cursor ?? `/users/${userId}/transactions`,
      {
        auth: "user",
        query: input.cursor
          ? undefined
          : {
              limit: String(limit),
              filter: "application_date",
              min_date: input.fromDate,
              max_date: input.toDate,
            },
      },
    );

    const items = (response.transactions ?? [])
      .filter((transaction) => String(transaction.id_account ?? "") === input.accountId)
      .filter((transaction) => transaction.active !== false && !transaction.deleted)
      .map((transaction) => this.mapBankTransaction(input.accountId, transaction, accountCurrency));
    const nextCursor = response._links?.next?.href ?? null;

    return {
      items,
      nextCursor,
      hasMore: Boolean(nextCursor),
    };
  }

  async listHoldings(input: ListHoldingsInput): Promise<HoldingsDto> {
    const userId = this.resolveUserId(input.userId);

    // Fetch account to get its currency as fallback for positions
    const rawAccount = await this.request<PowensAccount>(
      `/users/${userId}/accounts/${input.accountId}`,
      { auth: "user" },
    );
    const accountCurrency =
      typeof rawAccount?.currency === "string"
        ? rawAccount.currency
        : (rawAccount?.currency?.id ?? null);

    const response = await this.request<PowensInvestmentsResponse>(
      `/users/${userId}/accounts/${input.accountId}/investments`,
      { auth: "user" },
    );

    const positions = (response.investments ?? []).map((inv) => ({
      symbol: inv.stock_symbol ?? inv.code ?? null,
      isin: inv.code_type === "ISIN" ? (inv.code ?? null) : null,
      name: inv.label ?? inv.description ?? null,
      quantity: String(inv.quantity ?? "0"),
      unitPrice: this.toStringOrNull(inv.unitvalue ?? inv.unitprice),
      averageCost: this.toStringOrNull(inv.unitprice),
      currency: inv.original_currency ?? accountCurrency,
      exchange: inv.stock_market ?? null,
    }));

    return {
      accountId: input.accountId,
      positions,
      balances: [],
    };
  }

  async triggerSync(input: TriggerSyncInput): Promise<SyncRunDto> {
    const userId = this.resolveUserId(input.userId);
    if (!input.connectionId) {
      throw new InternalServerErrorException(
        "Powens sync requires a connectionId in this implementation.",
      );
    }

    await this.request(`/users/${userId}/connections/${input.connectionId}`, {
      auth: "user",
      method: "PUT",
      body: {
        active: true,
      },
    });

    const run: SyncRunDto = {
      id: `powens-run-${Date.now()}`,
      connectionId: input.connectionId,
      mode: input.mode,
      status: "running",
      startedAt: new Date().toISOString(),
      completedAt: null,
      summary: null,
    };
    this.syncRuns.set(run.id, run);

    return run;
  }

  async getSyncRun(runId: string): Promise<SyncRunDto | null> {
    return this.syncRuns.get(runId) ?? null;
  }

  async disableAccount(input: DisableAccountInput): Promise<void> {
    const userId = this.resolveUserId(input.userId);

    // Fetch the account to get its connection ID before disabling
    const account = await this.request<PowensAccount>(
      `/users/${userId}/accounts/${input.accountId}`,
      { auth: "user", query: { all: "true" } },
    );
    const connectionId = account.id_connection;

    // Disable the account on Powens
    await this.request(`/users/${userId}/accounts/${input.accountId}`, {
      auth: "user",
      method: "PUT",
      query: { all: "true" },
      body: { disabled: true },
    });

    // If no enabled accounts remain on this connection, delete the connection
    if (connectionId) {
      const response = await this.request<PowensAccountsResponse>(
        `/users/${userId}/connections/${connectionId}/accounts`,
        { auth: "user", query: { all: "true" } },
      );
      const enabledAccounts = (response.accounts ?? []).filter((a) => !a.disabled);
      if (enabledAccounts.length === 0) {
        await this.request(`/users/${userId}/connections/${connectionId}`, {
          auth: "user",
          method: "DELETE",
        });
      }
    }
  }

  async deleteConnection(input: DeleteConnectionInput): Promise<void> {
    const userId = this.resolveUserId(input.userId);
    await this.request(`/users/${userId}/connections/${input.connectionId}`, {
      auth: "user",
      method: "DELETE",
    });
  }

  private async resolveConnectionIds(userId: string, connectionId?: string): Promise<string[]> {
    if (connectionId) {
      return [connectionId];
    }

    const connections = await this.listConnections({ userId });
    return connections.map((connection) => connection.id);
  }

  private mapConnection(connection: PowensConnection): ConnectionDto {
    const id = String(connection.id ?? "");
    const connectorId = connection.connector_uuid ?? String(connection.id_connector ?? "");
    const connectorName =
      connection.connector?.display_name ??
      connection.connector?.name ??
      (connectorId || "Unknown connector");

    return {
      id,
      provider: "powens",
      connectorId,
      connectorName,
      institutionName: connectorName,
      status: this.mapConnectionStatus(connection),
      lastSyncedAt: connection.last_update ?? null,
    };
  }

  private mapAccount(
    connectionId: string,
    account: PowensAccount,
    institutionName: string | null,
  ): AccountDto {
    const externalAccountId = String(account.id ?? "");
    const accountType = this.mapAccountType(account.type);

    const rawName = account.label ?? account.name ?? null;
    const currency =
      typeof account.currency === "string" ? account.currency : (account.currency?.id ?? null);
    const name = this.buildAccountName(rawName, institutionName, currency, externalAccountId);

    return {
      id: externalAccountId,
      connectionId,
      externalAccountId,
      name,
      accountType,
      currency,
      institutionName,
      mask: account.number ?? account.iban ?? null,
    };
  }

  private mapMarketOrder(accountId: string, order: PowensMarketOrder): TransactionDto {
    const security =
      order.investment || order.code || order.stock_symbol
        ? {
            id: String(order.id ?? ""),
            symbol: order.stock_symbol ?? order.investment?.stock_symbol ?? null,
            isin:
              order.investment?.code_type === "ISIN"
                ? (order.investment.code ?? null)
                : (order.isin ?? null),
            name: order.investment?.label ?? order.label ?? order.investment?.description ?? null,
            currency: order.currency ?? order.original_currency ?? null,
            exchange: order.stock_market ?? order.investment?.stock_market ?? null,
          }
        : null;

    return {
      id: String(order.id ?? ""),
      accountId,
      security,
      bookedAt: order.datetime ?? order.date ?? new Date().toISOString(),
      settledAt: null,
      transactionType: this.mapOrderType(order.operation ?? order.order_type ?? ""),
      quantity: this.toStringOrNull(order.quantity),
      unitPrice: this.toStringOrNull(order.unitprice ?? order.unitvalue),
      grossAmount: this.toStringOrNull(order.value),
      netAmount: this.toStringOrNull(order.value),
      fee: this.toStringOrNull(order.commission),
      currency: order.currency ?? order.original_currency ?? null,
      description: order.label ?? null,
      externalReference: order.code ?? null,
    };
  }

  private mapBankTransaction(
    accountId: string,
    transaction: PowensBankTransaction,
    accountCurrency: string | null,
  ): TransactionDto {
    const amount = this.absString(transaction.value);
    const grossAmount = this.absString(transaction.gross_value) ?? amount;
    const currency = this.currencyCode(transaction.original_currency) ?? accountCurrency;

    return {
      id: String(transaction.id ?? ""),
      accountId,
      security: null,
      bookedAt:
        transaction.datetime ??
        transaction.application_date ??
        transaction.date ??
        transaction.rdate ??
        new Date().toISOString(),
      settledAt: transaction.vdatetime ?? transaction.vdate ?? null,
      transactionType: this.mapBankTransactionType(transaction.type ?? "", transaction.value),
      quantity: null,
      unitPrice: null,
      grossAmount,
      netAmount: amount,
      fee: this.absString(transaction.commission),
      currency,
      description:
        transaction.wording ??
        transaction.simplified_wording ??
        transaction.original_wording ??
        null,
      externalReference:
        transaction.id_cluster !== null && transaction.id_cluster !== undefined
          ? String(transaction.id_cluster)
          : String(transaction.id ?? ""),
    };
  }

  private mapConnectionStatus(connection: PowensConnection): ConnectionDto["status"] {
    if (connection.active === false) {
      return "failed";
    }

    if (!connection.state) {
      return "connected";
    }

    const state = connection.state.toLowerCase();
    if (state.includes("password") || state.includes("sca") || state.includes("otp")) {
      return "reauth_required";
    }
    if (state.includes("error")) {
      return "failed";
    }

    return "degraded";
  }

  private mapAccountType(
    type: string | PowensAccountType | null | undefined,
  ): AccountDto["accountType"] {
    if (!type) return "other";

    // Powens can return type as a string or as an object { id, name, is_invest }
    if (typeof type === "object") {
      if (type.is_invest) return "brokerage";
      const name = type.name?.toLowerCase() ?? "";
      if (name.includes("retirement")) return "retirement";
      if (name.includes("crypto")) return "crypto";
      if (name.includes("card") || name.includes("cash")) return "cash";
      return "other";
    }

    const t = type.toLowerCase();
    // Investment account types in Powens
    if (
      [
        "pee",
        "pea",
        "market",
        "life_insurance",
        "perp",
        "perco",
        "article83",
        "rsp",
        "pep",
      ].includes(t)
    ) {
      return "brokerage";
    }
    if (t === "checking" || t === "card") return "cash";
    if (t === "savings") return "cash";
    if (t === "deposit") return "cash";
    if (t === "crypto") return "crypto";
    if (t === "retirement" || t === "madelin" || t === "per") return "retirement";
    return "other";
  }

  private mapOrderType(operation: string): TransactionDto["transactionType"] {
    const value = operation.trim().toLowerCase();
    if (value.includes("buy") || value.includes("achat")) return "buy";
    if (value.includes("sell") || value.includes("sale") || value.includes("vente")) return "sell";
    if (value.includes("dividend")) return "dividend";
    if (value.includes("interest")) return "interest";
    if (value.includes("fee") || value.includes("commission")) return "fee";
    if (value.includes("tax")) return "tax";
    if (value.includes("deposit")) return "cash_deposit";
    if (value.includes("withdraw")) return "cash_withdrawal";
    if (value.includes("transfer")) return "transfer_in";
    return "unknown";
  }

  private mapBankTransactionType(
    type: string,
    value: string | number | null | undefined,
  ): TransactionDto["transactionType"] {
    const normalizedType = type.trim().toLowerCase();
    if (normalizedType === "profit") return "interest";
    if (["bank", "fee", "market_fee"].includes(normalizedType)) return "fee";

    const amount = this.toNumber(value);
    if (amount !== null) {
      if (amount > 0) return "cash_deposit";
      if (amount < 0) return "cash_withdrawal";
    }

    if (["deposit", "payback", "payout"].includes(normalizedType)) return "cash_deposit";
    if (
      [
        "withdrawal",
        "loan_repayment",
        "card",
        "deferred_card",
        "summary_card",
        "payment",
      ].includes(normalizedType)
    ) {
      return "cash_withdrawal";
    }

    return "unknown";
  }

  private buildAccountName(
    rawName: string | null,
    institutionName: string | null,
    currency: string | null,
    fallbackId: string,
  ): string {
    const isCurrencyOnly =
      !rawName || (currency && rawName.toUpperCase() === currency.toUpperCase());

    if (institutionName && currency && isCurrencyOnly) {
      return `${institutionName} ${currency}`;
    }
    if (institutionName && rawName && !isCurrencyOnly) {
      return `${institutionName} - ${rawName}`;
    }
    if (rawName && !isCurrencyOnly) {
      return rawName;
    }
    return institutionName ?? `Account ${fallbackId}`;
  }

  private deriveCategory(
    accountTypes: string[],
  ): "banks" | "brokers" | "insurance" | "crypto" | "savings" {
    const types = new Set(accountTypes.map((t) => t.toLowerCase()));

    const brokerTypes = new Set(["market", "pea", "pee", "perco", "pep", "rsp"]);
    const insuranceTypes = new Set([
      "lifeinsurance",
      "capitalisation",
      "perp",
      "per",
      "madelin",
      "article83",
    ]);
    const savingsTypes = new Set([
      "savings",
      "livret_a",
      "livret_b",
      "ldds",
      "pel",
      "cel",
      "csl",
      "cat",
      "deposit",
    ]);
    const bankTypes = new Set([
      "checking",
      "card",
      "loan",
      "mortgage",
      "revolvingcredit",
      "consumercredit",
    ]);

    const hasBroker = [...types].some((t) => brokerTypes.has(t));
    const hasInsurance = [...types].some((t) => insuranceTypes.has(t));
    const hasSavings = [...types].some((t) => savingsTypes.has(t));
    const hasBank = [...types].some((t) => bankTypes.has(t));

    if (hasBroker && !hasBank && !hasInsurance) return "brokers";
    if (hasInsurance && !hasBank && !hasBroker) return "insurance";
    if (hasSavings && !hasBank && !hasBroker && !hasInsurance) return "savings";
    if (hasBroker) return "brokers";
    if (hasInsurance) return "insurance";
    if (hasSavings) return "savings";
    return "banks";
  }

  private toStringOrNull(value: string | number | null | undefined): string | null {
    if (value === undefined || value === null || value === "") {
      return null;
    }

    return String(value);
  }

  private absString(value: string | number | null | undefined): string | null {
    const asString = this.toStringOrNull(value);
    if (!asString) return null;

    const numeric = Number(asString);
    if (Number.isFinite(numeric)) {
      return String(Math.abs(numeric));
    }

    return asString.startsWith("-") || asString.startsWith("+") ? asString.slice(1) : asString;
  }

  private toNumber(value: string | number | null | undefined): number | null {
    if (value === undefined || value === null || value === "") {
      return null;
    }

    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : null;
  }

  private currencyCode(value: string | PowensCurrency | null | undefined): string | null {
    if (!value) return null;
    return typeof value === "string" ? value : (value.id ?? null);
  }

  private resolveUserId(requestUserId: string): string {
    return this.config.powensUserId ?? requestUserId;
  }

  private async getUserAccessToken(): Promise<string> {
    if (this.config.powensUserAccessToken) {
      return this.config.powensUserAccessToken;
    }

    const clientId = this.config.powensClientId;
    const clientSecret = this.config.powensClientSecret;
    const userId = this.config.powensUserId;

    if (!clientId || !clientSecret || !userId) {
      throw new InternalServerErrorException(
        "Powens configuration is incomplete. Provide POWENS_USER_ACCESS_TOKEN or POWENS_CLIENT_ID, POWENS_CLIENT_SECRET, and POWENS_USER_ID.",
      );
    }

    const response = await this.request<PowensTokenResponse>("/auth/renew", {
      method: "POST",
      body: {
        grant_type: "client_credentials",
        client_id: clientId,
        client_secret: clientSecret,
        id_user: Number(userId),
        revoke_previous: false,
      },
    });

    const accessToken = response.access_token;
    if (!accessToken) {
      throw new InternalServerErrorException("Powens token renewal did not return access_token.");
    }

    return accessToken;
  }

  private async request<T = unknown>(
    pathname: string,
    options?: {
      auth?: "user";
      method?: "GET" | "POST" | "PUT" | "DELETE";
      query?: Record<string, string | undefined>;
      body?: unknown;
    },
  ): Promise<T> {
    const method = options?.method ?? "GET";
    const url = new URL(
      pathname.startsWith("/") ? pathname.slice(1) : pathname,
      `${this.config.powensBaseUrl}/`,
    );

    for (const [key, value] of Object.entries(options?.query ?? {})) {
      if (value !== undefined && value !== "") {
        url.searchParams.set(key, value);
      }
    }

    const headers = new Headers({
      Accept: "application/json",
    });

    if (options?.auth === "user") {
      headers.set("Authorization", `Bearer ${await this.getUserAccessToken()}`);
    }

    let body: string | undefined;
    if (options?.body !== undefined) {
      headers.set("Content-Type", "application/json");
      body = JSON.stringify(options.body);
    }

    const response = await fetch(url, {
      method,
      headers,
      body,
    });

    if (!response.ok) {
      const text = await response.text();
      throw new InternalServerErrorException(
        `Powens API request failed (${response.status}): ${text.slice(0, 300)}`,
      );
    }

    if (response.status === 204) {
      return {} as T;
    }

    return (await response.json()) as T;
  }
}
