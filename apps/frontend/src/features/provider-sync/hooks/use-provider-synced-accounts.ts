import { getProviderSyncedAccounts } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type { Account } from "@/lib/types";
import { useQuery } from "@tanstack/react-query";

export function useProviderSyncedAccounts(options?: { enabled?: boolean }) {
  return useQuery<Account[], Error>({
    queryKey: [QueryKeys.PROVIDER_SYNCED_ACCOUNTS],
    queryFn: getProviderSyncedAccounts,
    staleTime: 30 * 1000,
    enabled: options?.enabled,
  });
}
