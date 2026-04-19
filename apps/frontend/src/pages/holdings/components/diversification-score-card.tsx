import { Card } from "@wealthfolio/ui/components/ui/card";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import { cn } from "@/lib/utils";
import type { PortfolioAllocations } from "@/lib/types";
import { useMemo } from "react";
import { calculateDiversificationScore, type DimensionScore } from "../utils/diversification-score";

interface DiversificationScoreCardProps {
  allocations: PortfolioAllocations | undefined;
  isLoading: boolean;
  className?: string;
}

function scoreColor(score: number): string {
  if (score >= 70) return "text-green-600 dark:text-green-400";
  if (score >= 40) return "text-yellow-600 dark:text-yellow-400";
  return "text-red-600 dark:text-red-400";
}

function barColor(score: number): string {
  if (score >= 70) return "bg-green-600 dark:bg-green-400";
  if (score >= 40) return "bg-yellow-600 dark:bg-yellow-400";
  return "bg-red-600 dark:bg-red-400";
}

function DimensionBar({ dim }: { dim: DimensionScore }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-muted-foreground w-24 shrink-0 text-[11px]">{dim.dimension}</span>
      <div className="bg-muted h-1.5 flex-1 overflow-hidden rounded-full">
        <div
          className={cn("h-full rounded-full transition-all", barColor(dim.score))}
          style={{ width: `${dim.score}%` }}
        />
      </div>
      <span className={cn("w-7 text-right text-xs font-medium", scoreColor(dim.score))}>
        {dim.score}
      </span>
    </div>
  );
}

export const DiversificationScoreCard = ({
  allocations,
  isLoading,
  className,
}: DiversificationScoreCardProps) => {
  const result = useMemo(
    () => (allocations ? calculateDiversificationScore(allocations) : null),
    [allocations],
  );

  if (isLoading) {
    return (
      <Card className={cn("p-3 sm:p-3.5", className)}>
        <div className="flex items-center justify-between gap-6">
          <div>
            <p className="text-muted-foreground text-xs font-medium uppercase tracking-wider">
              Diversification Score
            </p>
            <Skeleton className="mt-1 h-6 w-12" />
          </div>
          <div className="flex max-w-xs flex-1 flex-col gap-1.5">
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-full" />
          </div>
        </div>
      </Card>
    );
  }

  if (!result || !allocations || allocations.totalValue <= 0) {
    return null;
  }

  return (
    <Card className={cn("p-3 sm:p-3.5", className)}>
      <div className="flex items-center justify-between gap-6">
        <div className="shrink-0">
          <p className="text-muted-foreground text-xs font-medium uppercase tracking-wider">
            Diversification Score
          </p>
          <p className={cn("mt-0.5 text-2xl font-bold tracking-tight", scoreColor(result.overall))}>
            {result.overall}
            <span className="text-muted-foreground text-sm font-normal">/100</span>
          </p>
        </div>
        <div className="flex min-w-0 max-w-xs flex-1 flex-col gap-1.5">
          {result.dimensions.map((dim) => (
            <DimensionBar key={dim.dimension} dim={dim} />
          ))}
        </div>
      </div>
    </Card>
  );
};
