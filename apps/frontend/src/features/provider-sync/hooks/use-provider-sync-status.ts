import { getProviderSyncStatus } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { useQuery } from "@tanstack/react-query";
import type { ProviderSyncStatus } from "../types";

export function useProviderSyncStatus() {
  return useQuery<ProviderSyncStatus, Error>({
    queryKey: [QueryKeys.PROVIDER_SYNC_STATUS],
    queryFn: getProviderSyncStatus,
    staleTime: 60 * 1000,
  });
}
