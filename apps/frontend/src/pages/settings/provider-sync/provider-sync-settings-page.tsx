import {
  deleteProviderConnection,
  getProviderSyncImportRuns,
  getProviderSyncStates,
  getProviderSyncStatus,
  getProviderSyncedAccounts,
  listProviderSyncAccounts,
  listProviderSyncConnections,
  syncProviderData,
} from "@/adapters";
import type {
  ProviderSyncAccount,
  ProviderSyncConnection,
  SyncProviderDataRequest,
} from "@/features/provider-sync/types";
import type { ImportRun } from "@/features/wealthfolio-connect/types";
import type { Account } from "@/lib/types";
import { QueryKeys } from "@/lib/query-keys";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@wealthfolio/ui";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@wealthfolio/ui/components/ui/alert-dialog";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Label } from "@wealthfolio/ui/components/ui/label";
import { Separator } from "@wealthfolio/ui/components/ui/separator";
import { formatDistanceToNow } from "date-fns";
import { ProviderSelectionDrawer } from "@/features/provider-sync/components/provider-selection-drawer";
import { useState } from "react";
import { toast } from "sonner";
import { SettingsHeader } from "../settings-header";

function statusTone(status: string): "default" | "secondary" | "destructive" | "outline" {
  switch (status.toLowerCase()) {
    case "connected":
    case "applied":
    case "idle":
      return "default";
    case "running":
      return "secondary";
    case "failed":
    case "reauth_required":
      return "destructive";
    default:
      return "outline";
  }
}

function formatRelative(value: string | null | undefined): string {
  if (!value) return "Never";
  return formatDistanceToNow(new Date(value), { addSuffix: true });
}

function accountLinkStatus(account: ProviderSyncAccount, linkedAccounts: Account[]): string {
  return linkedAccounts.some(
    (linkedAccount) => linkedAccount.providerAccountId === account.externalAccountId,
  )
    ? "Linked"
    : "Pending import";
}

function latestRunForAccount(
  account: ProviderSyncAccount,
  runs: ImportRun[],
  linkedAccounts: Account[],
): ImportRun | undefined {
  const linked = linkedAccounts.find(
    (linkedAccount) => linkedAccount.providerAccountId === account.externalAccountId,
  );
  if (!linked) return undefined;
  return runs.find((run) => run.accountId === linked.id);
}

function ConnectionCard({ connection }: { connection: ProviderSyncConnection }) {
  const [showDisconnect, setShowDisconnect] = useState(false);
  const queryClient = useQueryClient();

  const disconnectMutation = useMutation({
    mutationFn: () => deleteProviderConnection(connection.id),
    onSuccess: () => {
      setShowDisconnect(false);
      toast.success(`Disconnected ${connection.connectorName}`);
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACCOUNTS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_CONNECTIONS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_ACCOUNTS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNCED_ACCOUNTS] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_STATES] });
      queryClient.invalidateQueries({ queryKey: [QueryKeys.PROVIDER_SYNC_IMPORT_RUNS] });
    },
    onError: (error) => {
      toast.error("Failed to disconnect", {
        description: error instanceof Error ? error.message : "Unknown error",
      });
    },
  });

  return (
    <>
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between gap-3">
            <div>
              <CardTitle className="text-base">{connection.connectorName}</CardTitle>
              <CardDescription>
                {connection.institutionName ?? connection.connectorId}
              </CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant={statusTone(connection.status)}>{connection.status}</Badge>
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive hover:text-destructive h-7 px-2 text-xs"
                onClick={() => setShowDisconnect(true)}
              >
                Disconnect
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-1 text-sm">
          <div className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Provider</span>
            <span className="font-medium uppercase">{connection.provider}</span>
          </div>
          <div className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Last sync</span>
            <span>{formatRelative(connection.lastSyncedAt)}</span>
          </div>
        </CardContent>
      </Card>

      <AlertDialog open={showDisconnect} onOpenChange={setShowDisconnect}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Disconnect {connection.connectorName}?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently remove the connection and all its accounts from the provider.
              Local account data will not be affected.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button
              onClick={() => disconnectMutation.mutate()}
              disabled={disconnectMutation.isPending}
              className="bg-red-600 focus:ring-red-600"
            >
              {disconnectMutation.isPending ? (
                <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Icons.Trash className="mr-2 h-4 w-4" />
              )}
              Disconnect
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

export default function ProviderSyncSettingsPage() {
  const { data: status, isLoading: statusLoading } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNC_STATUS],
    queryFn: getProviderSyncStatus,
  });

  const { data: connections = [], isLoading: connectionsLoading } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNC_CONNECTIONS],
    queryFn: listProviderSyncConnections,
    enabled: status?.enabled === true,
  });

  const { data: accounts = [], isLoading: accountsLoading } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNC_ACCOUNTS],
    queryFn: listProviderSyncAccounts,
    enabled: status?.enabled === true,
  });

  const { data: linkedAccounts = [] } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNCED_ACCOUNTS],
    queryFn: getProviderSyncedAccounts,
    enabled: status?.enabled === true,
  });

  const { data: syncStates = [] } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNC_STATES],
    queryFn: getProviderSyncStates,
    enabled: status?.enabled === true,
  });

  const { data: importRuns = [] } = useQuery({
    queryKey: [QueryKeys.PROVIDER_SYNC_IMPORT_RUNS],
    queryFn: () => getProviderSyncImportRuns({ limit: 20 }),
    enabled: status?.enabled === true,
  });

  const syncMutation = useMutation<void, Error, SyncProviderDataRequest | undefined>({
    mutationFn: (request) => syncProviderData(request),
    onSuccess: () => {
      toast.loading("Syncing provider data...", { id: "provider-sync-start" });
    },
    onError: (error) => {
      toast.error("Failed to start sync", {
        description: error instanceof Error ? error.message : "Unknown error",
      });
    },
  });

  const [providerDrawerOpen, setProviderDrawerOpen] = useState(false);
  const [backfillFromDate, setBackfillFromDate] = useState("");
  const [backfillToDate, setBackfillToDate] = useState("");

  function runBackfill() {
    if (!backfillFromDate || !backfillToDate) {
      toast.error("Select a backfill date range");
      return;
    }
    if (backfillFromDate > backfillToDate) {
      toast.error("Backfill start date must be before end date");
      return;
    }

    syncMutation.mutate({
      mode: "backfill",
      fromDate: backfillFromDate,
      toDate: backfillToDate,
    });
  }

  if (statusLoading) {
    return (
      <div className="space-y-6">
        <SettingsHeader
          heading="Provider Sync"
          text="Sync external brokerage data through your own aggregation service."
        />
        <Separator />
        <div className="flex items-center justify-center py-12">
          <Icons.Spinner className="text-muted-foreground h-8 w-8 animate-spin" />
        </div>
      </div>
    );
  }

  if (!status?.enabled) {
    return (
      <div className="space-y-6">
        <SettingsHeader
          heading="Provider Sync"
          text="Sync external brokerage data through your own aggregation service."
        />
        <Separator />
        <Card>
          <CardHeader className="items-center text-center">
            <div className="bg-muted mb-2 flex h-12 w-12 items-center justify-center rounded-full">
              <Icons.CloudOff className="text-muted-foreground h-6 w-6" />
            </div>
            <CardTitle>Not Configured</CardTitle>
            <CardDescription>
              Set `WF_AGGREGATION_API_URL` and `WF_AGGREGATION_API_TOKEN` to enable Provider Sync.
            </CardDescription>
          </CardHeader>
        </Card>
      </div>
    );
  }

  const recentRuns = importRuns.slice(0, 8);

  return (
    <div className="space-y-6">
      <SettingsHeader
        heading="Provider Sync"
        text={`External ${status.provider.toUpperCase()} bridge for account discovery and transaction imports.`}
      />
      <Separator />

      <Card>
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <CardTitle className="text-base">Local importer</CardTitle>
              <CardDescription>
                Fetches normalized data from your external aggregation service and writes it into
                the local ledger.
              </CardDescription>
            </div>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => setProviderDrawerOpen(true)}>
                <Icons.Plus className="mr-2 h-4 w-4" />
                Connect Bank
              </Button>
              <Button
                onClick={() => syncMutation.mutate(undefined)}
                disabled={syncMutation.isPending}
              >
                {syncMutation.isPending ? (
                  <>
                    <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                    Syncing
                  </>
                ) : (
                  <>
                    <Icons.RefreshCw className="mr-2 h-4 w-4" />
                    Sync now
                  </>
                )}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="rounded-lg border p-4">
              <div className="text-muted-foreground text-xs uppercase tracking-wide">
                Connections
              </div>
              <div className="mt-2 text-2xl font-semibold">{connections.length}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-muted-foreground text-xs uppercase tracking-wide">
                Remote Accounts
              </div>
              <div className="mt-2 text-2xl font-semibold">{accounts.length}</div>
            </div>
            <div className="rounded-lg border p-4">
              <div className="text-muted-foreground text-xs uppercase tracking-wide">
                Linked Locally
              </div>
              <div className="mt-2 text-2xl font-semibold">{linkedAccounts.length}</div>
            </div>
          </div>

          <div className="rounded-lg border p-4">
            <div className="flex flex-wrap items-end gap-3">
              <div className="min-w-40 flex-1">
                <Label htmlFor="provider-backfill-from" className="text-xs">
                  Backfill from
                </Label>
                <Input
                  id="provider-backfill-from"
                  type="date"
                  value={backfillFromDate}
                  onChange={(event) => setBackfillFromDate(event.target.value)}
                />
              </div>
              <div className="min-w-40 flex-1">
                <Label htmlFor="provider-backfill-to" className="text-xs">
                  Backfill to
                </Label>
                <Input
                  id="provider-backfill-to"
                  type="date"
                  value={backfillToDate}
                  onChange={(event) => setBackfillToDate(event.target.value)}
                />
              </div>
              <Button
                variant="outline"
                onClick={runBackfill}
                disabled={syncMutation.isPending}
                className="shrink-0"
              >
                {syncMutation.isPending ? (
                  <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Icons.History className="mr-2 h-4 w-4" />
                )}
                Backfill
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 xl:grid-cols-[1.2fr,0.8fr]">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Connections</CardTitle>
            <CardDescription>
              Institutions currently exposed by the aggregation service.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {connectionsLoading ? (
              <div className="text-muted-foreground text-sm">Loading connections…</div>
            ) : connections.length > 0 ? (
              connections.map((connection) => (
                <ConnectionCard key={connection.id} connection={connection} />
              ))
            ) : (
              <div className="flex flex-col items-center gap-3 py-4 text-center">
                <div className="text-muted-foreground text-sm">
                  No provider connections found yet.
                </div>
                <Button variant="outline" size="sm" onClick={() => setProviderDrawerOpen(true)}>
                  <Icons.Plus className="mr-2 h-4 w-4" />
                  Connect your first bank
                </Button>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Recent import runs</CardTitle>
            <CardDescription>Latest local imports for this provider.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {recentRuns.length > 0 ? (
              recentRuns.map((run) => (
                <div key={run.id} className="rounded-lg border p-3">
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-sm font-medium">{run.accountId}</div>
                    <Badge variant={statusTone(run.status)}>{run.status}</Badge>
                  </div>
                  <div className="text-muted-foreground mt-1 text-xs">
                    {formatRelative(run.startedAt)}
                  </div>
                  <div className="mt-2 text-xs">
                    Imported {run.summary?.inserted ?? 0} / fetched {run.summary?.fetched ?? 0}
                  </div>
                </div>
              ))
            ) : (
              <div className="text-muted-foreground text-sm">No provider import runs yet.</div>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Accounts</CardTitle>
          <CardDescription>
            Remote accounts discovered from the aggregation service and their local link status.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {accountsLoading ? (
            <div className="text-muted-foreground text-sm">Loading accounts…</div>
          ) : accounts.length > 0 ? (
            accounts.map((account) => {
              const linkStatus = accountLinkStatus(account, linkedAccounts);
              const linkedRun = latestRunForAccount(account, importRuns, linkedAccounts);
              return (
                <ProviderAccountRow
                  key={account.id}
                  account={account}
                  linkStatus={linkStatus}
                  linkedRun={linkedRun}
                />
              );
            })
          ) : (
            <div className="text-muted-foreground text-sm">No remote accounts found.</div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Sync state</CardTitle>
          <CardDescription>Last known per-account sync state stored locally.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {syncStates.length > 0 ? (
            syncStates.map((state) => (
              <div key={state.accountId} className="rounded-lg border p-3">
                <div className="flex items-center justify-between gap-3">
                  <div className="text-sm font-medium">{state.accountId}</div>
                  <Badge variant={statusTone(state.syncStatus)}>{state.syncStatus}</Badge>
                </div>
                <div className="text-muted-foreground mt-1 text-xs">
                  Last success: {formatRelative(state.lastSuccessfulAt)}
                </div>
                {state.lastError && (
                  <div className="mt-2 text-xs text-red-500">{state.lastError}</div>
                )}
              </div>
            ))
          ) : (
            <div className="text-muted-foreground text-sm">No sync state recorded yet.</div>
          )}
        </CardContent>
      </Card>
      <ProviderSelectionDrawer isOpen={providerDrawerOpen} onOpenChange={setProviderDrawerOpen} />
    </div>
  );
}

function ProviderAccountRow({
  account,
  linkStatus,
  linkedRun,
}: {
  account: ProviderSyncAccount;
  linkStatus: string;
  linkedRun?: ImportRun;
}) {
  return (
    <div className="rounded-lg border p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="text-sm font-medium">{account.name}</div>
          <div className="text-muted-foreground mt-1 text-xs">
            {account.institutionName ?? "Unknown institution"} • {account.accountType}
            {account.mask ? ` • ${account.mask}` : ""}
          </div>
        </div>
        <Badge variant={linkStatus === "Linked" ? "default" : "outline"}>{linkStatus}</Badge>
      </div>
      <div className="mt-3 grid gap-2 text-xs sm:grid-cols-3">
        <div>
          <div className="text-muted-foreground">Remote account ID</div>
          <div className="font-mono">{account.externalAccountId}</div>
        </div>
        <div>
          <div className="text-muted-foreground">Currency</div>
          <div>{account.currency ?? "Unknown"}</div>
        </div>
        <div>
          <div className="text-muted-foreground">Latest import</div>
          <div>{linkedRun ? formatRelative(linkedRun.startedAt) : "Not imported yet"}</div>
        </div>
      </div>
    </div>
  );
}
