import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { apiGet, apiPost } from "./api-client.js";

// -- helpers --

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

function periodToStartDate(period: string): string | undefined {
  const now = new Date();
  switch (period) {
    case "1M":
      now.setMonth(now.getMonth() - 1);
      return now.toISOString().slice(0, 10);
    case "3M":
      now.setMonth(now.getMonth() - 3);
      return now.toISOString().slice(0, 10);
    case "6M":
      now.setMonth(now.getMonth() - 6);
      return now.toISOString().slice(0, 10);
    case "YTD":
      return `${now.getFullYear()}-01-01`;
    case "1Y":
      now.setFullYear(now.getFullYear() - 1);
      return now.toISOString().slice(0, 10);
    case "ALL":
      return undefined;
    default:
      return undefined;
  }
}

function textResult(data: unknown): { content: Array<{ type: "text"; text: string }> } {
  return { content: [{ type: "text" as const, text: JSON.stringify(data, null, 2) }] };
}

const GROUP_BY_KEY: Record<string, string> = {
  class: "assetClasses",
  sector: "sectors",
  region: "regions",
  risk: "riskCategory",
  security_type: "securityTypes",
};

// -- tool registration --

export function registerTools(server: McpServer): void {
  // 1. get_accounts
  server.tool(
    "get_accounts",
    "Get the list of active investment accounts. Returns account id, name, type, and currency for each account.",
    {},
    async () => {
      const data = await apiGet("/accounts");
      return textResult(data);
    },
  );

  // 2. get_holdings
  server.tool(
    "get_holdings",
    "Get portfolio holdings for an account or all accounts. Returns symbol, quantity, market value, cost basis, and gain/loss for each holding. Use accountId='TOTAL' for aggregate holdings across all accounts.",
    { accountId: z.string().default("TOTAL").describe("Account ID, or 'TOTAL' for all accounts") },
    async ({ accountId }) => {
      const data = await apiGet("/holdings", { accountId });
      return textResult(data);
    },
  );

  // 3. get_asset_allocation
  server.tool(
    "get_asset_allocation",
    "Get portfolio asset allocation breakdown. Can group by asset class, sector, region, risk level, or security type.",
    {
      accountId: z.string().default("TOTAL").describe("Account ID, or 'TOTAL' for all accounts"),
      groupBy: z
        .enum(["class", "sector", "region", "risk", "security_type"])
        .default("class")
        .describe(
          "Grouping: 'class' (Equity/Fixed Income/Cash), 'sector' (Technology/Healthcare/etc), 'region' (North America/Europe/etc), 'risk' (Low/Medium/High), 'security_type' (Stock/ETF/Bond)",
        ),
    },
    async ({ accountId, groupBy }) => {
      const data = (await apiGet("/allocations", { accountId })) as Record<string, unknown>;
      const key = GROUP_BY_KEY[groupBy];
      return textResult({ allocation: data[key], totalValue: data["totalValue"], groupBy });
    },
  );

  // 4. get_performance
  server.tool(
    "get_performance",
    "Get portfolio performance metrics including TWR, MWR, volatility, and max drawdown. Use accountId='TOTAL' for aggregate performance.",
    {
      accountId: z.string().default("TOTAL").describe("Account ID, or 'TOTAL' for all accounts"),
      period: z
        .enum(["1M", "3M", "6M", "YTD", "1Y", "ALL"])
        .default("YTD")
        .describe("Time period for performance calculation"),
    },
    async ({ accountId, period }) => {
      const startDate = periodToStartDate(period);
      const endDate = todayStr();
      const data = await apiPost("/performance/history", {
        itemType: "ACCOUNT",
        itemId: accountId,
        startDate,
        endDate,
      });
      return textResult(data);
    },
  );

  // 5. get_valuation_history
  server.tool(
    "get_valuation_history",
    "Get historical portfolio valuations over time. Returns daily valuation points with total value and net contributions. Use accountId='TOTAL' for aggregate valuations. Useful for analyzing portfolio growth and trends.",
    {
      accountId: z.string().default("TOTAL").describe("Account ID, or 'TOTAL' for all accounts"),
      startDate: z.string().optional().describe("Start date YYYY-MM-DD (defaults to 365 days ago)"),
      endDate: z.string().optional().describe("End date YYYY-MM-DD (defaults to today)"),
    },
    async ({ accountId, startDate, endDate }) => {
      const end = endDate ?? todayStr();
      const start =
        startDate ??
        (() => {
          const d = new Date();
          d.setFullYear(d.getFullYear() - 1);
          return d.toISOString().slice(0, 10);
        })();

      if (accountId !== "TOTAL") {
        const data = await apiGet("/valuations/history", {
          accountId,
          startDate: start,
          endDate: end,
        });
        return textResult(data);
      }

      // Aggregate across all active accounts
      const accounts = (await apiGet("/accounts")) as Array<{
        id: string;
        isActive: boolean;
      }>;
      const active = accounts.filter((a) => a.isActive);
      if (active.length === 0) return textResult([]);

      type ValuationPoint = {
        valuationDate: string;
        totalValue: number;
        netContribution: number;
        fxRateToBase: number;
      };

      const allValuations = await Promise.all(
        active.map((a) =>
          apiGet<ValuationPoint[]>("/valuations/history", {
            accountId: a.id,
            startDate: start,
            endDate: end,
          }),
        ),
      );

      const byDate = new Map<string, { totalValue: number; netContribution: number }>();
      for (const accountVals of allValuations) {
        for (const v of accountVals) {
          const existing = byDate.get(v.valuationDate) ?? { totalValue: 0, netContribution: 0 };
          existing.totalValue += v.totalValue;
          existing.netContribution += v.netContribution;
          byDate.set(v.valuationDate, existing);
        }
      }

      const aggregated = Array.from(byDate.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([date, vals]) => ({
          date,
          totalValue: Math.round(vals.totalValue * 100) / 100,
          netContribution: Math.round(vals.netContribution * 100) / 100,
        }));

      return textResult(aggregated);
    },
  );

  // 6. search_activities
  server.tool(
    "search_activities",
    "Search investment activities (transactions) such as buys, sells, dividends, deposits, and withdrawals. Supports filtering by account, type, symbol, and date range. Returns paginated results.",
    {
      accountId: z.string().optional().describe("Filter by account ID"),
      activityType: z
        .enum([
          "BUY",
          "SELL",
          "DIVIDEND",
          "DEPOSIT",
          "WITHDRAWAL",
          "TRANSFER_IN",
          "TRANSFER_OUT",
          "INTEREST",
          "FEE",
          "SPLIT",
          "TAX",
        ])
        .optional()
        .describe("Filter by activity type"),
      symbol: z.string().optional().describe("Filter by symbol or asset keyword"),
      dateFrom: z.string().optional().describe("Start date YYYY-MM-DD"),
      dateTo: z.string().optional().describe("End date YYYY-MM-DD"),
      page: z.number().int().default(1).describe("Page number (1-based)"),
      pageSize: z.number().int().default(50).describe("Results per page (max 200)"),
    },
    async ({ accountId, activityType, symbol, dateFrom, dateTo, page, pageSize }) => {
      const data = await apiPost("/activities/search", {
        page: page - 1,
        pageSize: Math.min(pageSize, 200),
        accountIdFilter: accountId ? [accountId] : undefined,
        activityTypeFilter: activityType ? [activityType] : undefined,
        assetIdKeyword: symbol || undefined,
        sort: { id: "date", desc: true },
        dateFrom,
        dateTo,
      });
      return textResult(data);
    },
  );

  // 7. get_income
  server.tool(
    "get_income",
    "Fetch income summary including dividends, interest, and other income. Returns total income, monthly average, year-over-year growth, breakdown by type, and top income-generating assets.",
    {
      period: z
        .enum(["YTD", "LAST_YEAR", "TOTAL"])
        .default("YTD")
        .describe("Time period: YTD, LAST_YEAR, or TOTAL (all time)"),
    },
    async ({ period }) => {
      const summaries = (await apiGet("/income/summary")) as Array<{ period: string }>;
      const match = summaries.find((s) => s.period === period);
      return textResult(match ?? { error: `No income data for period: ${period}` });
    },
  );

  // 8. get_goals
  server.tool(
    "get_goals",
    "Get investment goals with target amounts and achievement status.",
    {},
    async () => {
      const data = await apiGet("/goals");
      return textResult(data);
    },
  );
}
