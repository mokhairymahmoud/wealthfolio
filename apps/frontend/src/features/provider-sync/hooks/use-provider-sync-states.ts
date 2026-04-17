import { getProviderSyncStates } from "@/adapters";
import type { BrokerSyncState } from "@/features/wealthfolio-connect/types";
import { QueryKeys } from "@/lib/query-keys";
import { useQuery } from "@tanstack/react-query";

export function useProviderSyncStates(options?: { enabled?: boolean }) {
  return useQuery<BrokerSyncState[], Error>({
    queryKey: [QueryKeys.PROVIDER_SYNC_STATES],
    queryFn: getProviderSyncStates,
    staleTime: 30 * 1000,
    enabled: options?.enabled,
  });
}
