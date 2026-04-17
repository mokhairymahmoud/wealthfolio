import { getProviderSyncImportRuns } from "@/adapters";
import type { ImportRun } from "@/features/wealthfolio-connect/types";
import { QueryKeys } from "@/lib/query-keys";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

interface UseProviderImportRunsOptions {
  limit?: number;
  enabled?: boolean;
}

export function useProviderImportRuns(options: UseProviderImportRunsOptions = {}) {
  const { limit = 50, enabled = true } = options;

  return useQuery<ImportRun[], Error>({
    queryKey: [QueryKeys.PROVIDER_SYNC_IMPORT_RUNS, limit],
    queryFn: () => getProviderSyncImportRuns({ runType: "SYNC", limit, offset: 0 }),
    staleTime: 30 * 1000,
    enabled,
  });
}

interface UseProviderImportRunsInfiniteOptions {
  pageSize?: number;
  enabled?: boolean;
}

export function useProviderImportRunsInfinite(options: UseProviderImportRunsInfiniteOptions = {}) {
  const { pageSize = 10, enabled = true } = options;

  return useInfiniteQuery<ImportRun[], Error>({
    queryKey: [QueryKeys.PROVIDER_SYNC_IMPORT_RUNS, "infinite", pageSize],
    queryFn: async ({ pageParam = 0 }) => {
      return getProviderSyncImportRuns({
        runType: "SYNC",
        limit: pageSize,
        offset: pageParam as number,
      });
    },
    initialPageParam: 0,
    getNextPageParam: (lastPage, allPages) => {
      if (lastPage.length < pageSize) {
        return undefined;
      }
      return allPages.reduce((acc, page) => acc + page.length, 0);
    },
    staleTime: 30 * 1000,
    enabled,
  });
}
