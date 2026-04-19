import { getFeeAnalysis, updateExpenseRatio } from "@/adapters";
import { useBalancePrivacy } from "@/hooks/use-balance-privacy";
import { PORTFOLIO_ACCOUNT_ID } from "@/lib/constants";
import { QueryKeys } from "@/lib/query-keys";
import type { FeeAnalysis, HoldingFee } from "@/lib/types";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { AmountDisplay } from "@wealthfolio/ui";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@wealthfolio/ui/components/ui/card";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@wealthfolio/ui/components/ui/tooltip";
import { useState } from "react";

function formatPercent(value: number | null | undefined): string {
  if (value == null) return "-";
  return `${(value * 100).toFixed(2)}%`;
}

function SeverityBadge({ severity }: { severity: HoldingFee["severity"] }) {
  if (severity === "HIGH") {
    return (
      <Badge variant="destructive" className="text-xs">
        High
      </Badge>
    );
  }
  if (severity === "WARNING") {
    return (
      <Badge variant="outline" className="border-yellow-500 text-xs text-yellow-500">
        Warning
      </Badge>
    );
  }
  return null;
}

function InlineExpenseRatioEditor({
  holding,
  onSave,
}: {
  holding: HoldingFee;
  onSave: (assetId: string, value: number | null) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(
    holding.expenseRatio != null ? (holding.expenseRatio * 100).toFixed(2) : "",
  );

  if (!editing) {
    return (
      <button
        className="hover:bg-muted flex items-center gap-1 rounded px-1 py-0.5 text-sm"
        onClick={() => {
          setDraft(holding.expenseRatio != null ? (holding.expenseRatio * 100).toFixed(2) : "");
          setEditing(true);
        }}
      >
        {holding.expenseRatio != null ? formatPercent(holding.expenseRatio) : "-"}
        <Icons.Pencil className="text-muted-foreground h-3 w-3" />
      </button>
    );
  }

  const handleSave = () => {
    const trimmed = draft.trim();
    if (trimmed === "") {
      onSave(holding.assetId, null);
    } else {
      const parsed = parseFloat(trimmed);
      if (!isNaN(parsed) && parsed >= 0 && parsed <= 100) {
        onSave(holding.assetId, parsed / 100);
      }
    }
    setEditing(false);
  };

  return (
    <div className="flex items-center gap-1">
      <Input
        className="h-7 w-20 text-right text-sm"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") handleSave();
          if (e.key === "Escape") setEditing(false);
        }}
        onBlur={handleSave}
        autoFocus
        placeholder="0.00"
      />
      <span className="text-muted-foreground text-xs">%</span>
    </div>
  );
}

export default function FeeScannerPage() {
  const { isBalanceHidden } = useBalancePrivacy();
  const queryClient = useQueryClient();

  const {
    data: feeData,
    isLoading,
    error,
  } = useQuery<FeeAnalysis, Error>({
    queryKey: [QueryKeys.FEE_ANALYSIS],
    queryFn: () => getFeeAnalysis(PORTFOLIO_ACCOUNT_ID),
  });

  const updateMutation = useMutation({
    mutationFn: ({ assetId, expenseRatio }: { assetId: string; expenseRatio: number | null }) =>
      updateExpenseRatio(assetId, expenseRatio),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.FEE_ANALYSIS] });
    },
  });

  const handleExpenseRatioSave = (assetId: string, value: number | null) => {
    updateMutation.mutate({ assetId, expenseRatio: value });
  };

  if (isLoading) return <FeeScannerSkeleton />;
  if (error || !feeData)
    return <div>Failed to load fee analysis: {error?.message || "Unknown error"}</div>;

  const {
    holdings,
    totalAnnualFee,
    weightedAvgExpenseRatio,
    feePctOfPortfolio,
    projections,
    currency,
  } = feeData;
  const holdingsWithFees = holdings.filter((h) => h.expenseRatio != null);
  const holdingsWithoutFees = holdings.filter((h) => h.expenseRatio == null);

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Annual Fee Cost</CardTitle>
            <Icons.Receipt className="text-muted-foreground h-4 w-4" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              <AmountDisplay
                value={totalAnnualFee}
                currency={currency}
                isHidden={isBalanceHidden}
              />
            </div>
            {feePctOfPortfolio != null && (
              <p className="text-muted-foreground text-xs">
                {formatPercent(feePctOfPortfolio)} of portfolio
              </p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Weighted Avg Expense Ratio</CardTitle>
            <Icons.TrendingDown className="text-muted-foreground h-4 w-4" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {weightedAvgExpenseRatio != null ? formatPercent(weightedAvgExpenseRatio) : "-"}
            </div>
            <p className="text-muted-foreground text-xs">
              Across {holdingsWithFees.length} holdings with known fees
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Projected Fee Drag</CardTitle>
            <Icons.TrendingUp className="text-muted-foreground h-4 w-4" />
          </CardHeader>
          <CardContent>
            {projections.length > 0 ? (
              <div className="space-y-1">
                {projections.map((p) => (
                  <div key={p.years} className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">{p.years}yr</span>
                    <span className="font-medium text-red-500">
                      <AmountDisplay
                        value={p.cumulativeFeeDrag}
                        currency={currency}
                        isHidden={isBalanceHidden}
                      />
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-muted-foreground text-sm">No fee data available</p>
            )}
            <p className="text-muted-foreground mt-1 text-xs">Assuming 7% annual return</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-medium">Holdings Fee Breakdown</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Holding</TableHead>
                <TableHead className="hidden sm:table-cell">Account</TableHead>
                <TableHead className="text-right">Market Value</TableHead>
                <TableHead className="text-right">Expense Ratio</TableHead>
                <TableHead className="text-right">Annual Fee</TableHead>
                <TableHead className="text-right">Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {holdings.map((h) => (
                <TableRow key={`${h.assetId}-${h.accountId}`}>
                  <TableCell>
                    <div>
                      <div className="font-medium">{h.symbol}</div>
                      <div className="text-muted-foreground max-w-[200px] truncate text-xs">
                        {h.name}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground hidden text-sm sm:table-cell">
                    {h.accountName ?? "-"}
                  </TableCell>
                  <TableCell className="text-right">
                    <AmountDisplay
                      value={h.marketValueBase}
                      currency={currency}
                      isHidden={isBalanceHidden}
                    />
                  </TableCell>
                  <TableCell className="text-right">
                    <InlineExpenseRatioEditor holding={h} onSave={handleExpenseRatioSave} />
                  </TableCell>
                  <TableCell className="text-right">
                    {h.annualFee != null ? (
                      <AmountDisplay
                        value={h.annualFee}
                        currency={currency}
                        isHidden={isBalanceHidden}
                      />
                    ) : (
                      "-"
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    <SeverityBadge severity={h.severity} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {holdingsWithoutFees.length > 0 && (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="text-muted-foreground flex items-center gap-1 text-xs">
                <Icons.Info className="h-3 w-3" />
                {holdingsWithoutFees.length} holding(s) without expense ratio data. Click the pencil
                icon to add manually.
              </div>
            </TooltipTrigger>
            <TooltipContent>
              <p>{holdingsWithoutFees.map((h) => h.symbol).join(", ")}</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
    </div>
  );
}

function FeeScannerSkeleton() {
  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-3">
        {[...Array(3)].map((_, i) => (
          <Card key={i}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <Skeleton className="h-4 w-[120px]" />
              <Skeleton className="h-4 w-4" />
            </CardHeader>
            <CardContent>
              <Skeleton className="h-8 w-[100px]" />
              <Skeleton className="mt-2 h-4 w-[80px]" />
            </CardContent>
          </Card>
        ))}
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-[180px]" />
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {[...Array(5)].map((_, i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
