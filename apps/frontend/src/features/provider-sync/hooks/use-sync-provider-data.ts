import { syncProviderData } from "@/adapters";
import { useMutation } from "@tanstack/react-query";
import { toast } from "sonner";

/**
 * Hook to trigger provider data sync.
 * The actual sync runs in the background and results are handled via
 * global event listeners (provider:sync-complete/error events trigger
 * toasts and query invalidation in use-global-event-listener.ts).
 */
export function useSyncProviderData() {
  return useMutation({
    mutationFn: () => syncProviderData(),
    onSuccess: () => {
      toast.loading("Syncing provider data...", { id: "provider-sync-start" });
    },
    onError: (error) => {
      toast.error(
        `Failed to start sync: ${error instanceof Error ? error.message : "Unknown error"}`,
      );
    },
  });
}
