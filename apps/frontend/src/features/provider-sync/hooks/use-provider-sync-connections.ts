import { listProviderSyncConnections } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { useQuery } from "@tanstack/react-query";
import type { ProviderSyncConnection } from "../types";

export function useProviderSyncConnections(options?: { enabled?: boolean }) {
  return useQuery<ProviderSyncConnection[], Error>({
    queryKey: [QueryKeys.PROVIDER_SYNC_CONNECTIONS],
    queryFn: listProviderSyncConnections,
    staleTime: 30 * 1000,
    enabled: options?.enabled,
  });
}
