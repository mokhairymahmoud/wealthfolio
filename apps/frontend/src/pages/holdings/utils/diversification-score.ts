import type { PortfolioAllocations, TaxonomyAllocation } from "@/lib/types";

export interface DimensionScore {
  dimension: string;
  score: number;
  categoryCount: number;
}

export interface DiversificationScore {
  overall: number;
  dimensions: DimensionScore[];
}

function computeDimensionScore(
  allocation: TaxonomyAllocation | undefined,
  label: string,
): DimensionScore {
  if (!allocation) return { dimension: label, score: 0, categoryCount: 0 };

  const categories = allocation.categories.filter(
    (c) => c.percentage > 0 && c.categoryName.toLowerCase() !== "unknown",
  );
  const n = categories.length;
  if (n <= 1) return { dimension: label, score: 0, categoryCount: n };

  const hhi = categories.reduce((sum, c) => sum + (c.percentage / 100) ** 2, 0);
  const hhiMin = 1 / n;
  const score = Math.round(100 * (1 - (hhi - hhiMin) / (1 - hhiMin)));

  return { dimension: label, score: Math.max(0, Math.min(100, score)), categoryCount: n };
}

export function calculateDiversificationScore(
  allocations: PortfolioAllocations,
): DiversificationScore {
  if (allocations.totalValue <= 0) {
    return {
      overall: 0,
      dimensions: [
        { dimension: "Sectors", score: 0, categoryCount: 0 },
        { dimension: "Regions", score: 0, categoryCount: 0 },
        { dimension: "Asset Classes", score: 0, categoryCount: 0 },
      ],
    };
  }

  const dimensions = [
    computeDimensionScore(allocations.sectors, "Sectors"),
    computeDimensionScore(allocations.regions, "Regions"),
    computeDimensionScore(allocations.assetClasses, "Asset Classes"),
  ];

  const activeDimensions = dimensions.filter((d) => d.categoryCount > 0);
  const overall =
    activeDimensions.length > 0
      ? Math.round(activeDimensions.reduce((sum, d) => sum + d.score, 0) / activeDimensions.length)
      : 0;

  return { overall, dimensions };
}
