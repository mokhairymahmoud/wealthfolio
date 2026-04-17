import { listenProviderSyncComplete, listenProviderSyncError, syncProviderData } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import { useQueryClient } from "@tanstack/react-query";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { useEffect, useRef } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { toast } from "sonner";

const SYNC_TIMEOUT_MS = 120_000;

async function createProviderSyncWaiter() {
  let settled = false;
  const cleanupFns: Array<() => void | Promise<void>> = [];

  const cleanup = () => {
    window.clearTimeout(timeoutId);
    cleanupFns.forEach((cleanupFn) => void cleanupFn());
  };

  let resolvePromise: () => void = () => {};
  let rejectPromise: (error: Error) => void = () => {};
  const promise = new Promise<void>((resolve, reject) => {
    resolvePromise = resolve;
    rejectPromise = reject;
  });

  const settle = (callback: () => void) => {
    if (settled) return;
    settled = true;
    cleanup();
    callback();
  };

  const timeoutId = window.setTimeout(() => {
    settle(() => rejectPromise(new Error("Provider sync timed out.")));
  }, SYNC_TIMEOUT_MS);

  try {
    const [unlistenComplete, unlistenError] = await Promise.all([
      listenProviderSyncComplete(() => {
        settle(resolvePromise);
      }),
      listenProviderSyncError((event: { payload: { error?: string } }) => {
        const message = event.payload?.error ?? "Provider sync failed.";
        settle(() => rejectPromise(new Error(message)));
      }),
    ]);

    cleanupFns.push(unlistenComplete, unlistenError);
  } catch (error) {
    settle(() => rejectPromise(toError(error)));
  }

  return {
    promise,
    cancel: () => {
      if (settled) return;
      settled = true;
      cleanup();
    },
  };
}

function toError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

export default function ProviderCallbackPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const hasSynced = useRef(false);

  useEffect(() => {
    if (hasSynced.current) return;
    hasSynced.current = true;

    toast.loading("Syncing your new connection...", {
      id: "provider-callback-sync",
      duration: Infinity,
    });

    const connectionId = searchParams.get("connection_id")?.trim() || undefined;
    let syncWaiter: Awaited<ReturnType<typeof createProviderSyncWaiter>> | undefined;

    void (async () => {
      try {
        const waiter = await createProviderSyncWaiter();
        syncWaiter = waiter;
        await syncProviderData(connectionId);
        await waiter.promise;
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: [QueryKeys.ACCOUNTS] }),
          queryClient.invalidateQueries({ queryKey: [QueryKeys.PLATFORMS] }),
          queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_CONNECTIONS] }),
          queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNCED_ACCOUNTS] }),
          queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_STATES] }),
          queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_IMPORT_RUNS] }),
        ]);
        toast.dismiss("provider-callback-sync");
      } catch (_error) {
        toast.dismiss("provider-callback-sync");
        toast.error("Sync failed. You can try again from the Connect page.");
      } finally {
        syncWaiter?.cancel();
        navigate("/connect", { replace: true });
      }
    })();
  }, [navigate, queryClient, searchParams]);

  return (
    <div className="bg-background text-foreground flex min-h-screen flex-col items-center justify-center">
      <div className="flex flex-col items-center gap-4">
        <Icons.Spinner className="text-muted-foreground h-8 w-8 animate-spin" />
        <p className="text-muted-foreground text-sm">
          Connection successful. Syncing your accounts...
        </p>
      </div>
    </div>
  );
}
