import { listProviderConnectors, getProviderConnectUrl, openUrlInBrowser } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type { ProviderConnector } from "@/features/provider-sync/types";
import { useQuery, useMutation } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@wealthfolio/ui/components/ui/sheet";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Skeleton } from "@wealthfolio/ui";
import { ToggleGroup, ToggleGroupItem } from "@wealthfolio/ui";
import { ScrollArea } from "@wealthfolio/ui/components/ui/scroll-area";

const CATEGORY_LABELS: Record<string, string> = {
  all: "All",
  banks: "Banks",
  brokers: "Brokers",
  insurance: "Insurance",
  crypto: "Crypto",
  savings: "Savings",
};

const CATEGORIES = Object.keys(CATEGORY_LABELS);

interface ProviderSelectionDrawerProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onManualAdd?: () => void;
}

export function ProviderSelectionDrawer({
  isOpen,
  onOpenChange,
  onManualAdd,
}: ProviderSelectionDrawerProps) {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("all");

  const {
    data: connectors,
    isLoading,
    isError,
    refetch,
  } = useQuery({
    queryKey: [QueryKeys.PROVIDER_CONNECTORS],
    queryFn: listProviderConnectors,
    staleTime: 24 * 60 * 60 * 1000,
    enabled: isOpen,
  });

  const connectMutation = useMutation({
    mutationFn: async (connectorId: string) => {
      const { url } = await getProviderConnectUrl(connectorId);
      await openUrlInBrowser(url);
    },
    onSuccess: () => {
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error("Failed to open bank connection", {
        description: error instanceof Error ? error.message : "Unknown error",
      });
    },
  });

  const categoryCounts = useMemo(() => {
    if (!connectors) return {};
    const counts: Record<string, number> = { all: connectors.length };
    for (const c of connectors) {
      counts[c.category] = (counts[c.category] ?? 0) + 1;
    }
    return counts;
  }, [connectors]);

  const filtered = useMemo(() => {
    if (!connectors) return [];
    let result = connectors;
    if (category !== "all") {
      result = result.filter((c) => c.category === category);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(
        (c) => c.name.toLowerCase().includes(q) || c.country?.toLowerCase().includes(q),
      );
    }
    return result;
  }, [connectors, category, search]);

  const handleManualAdd = () => {
    onOpenChange(false);
    onManualAdd?.();
  };

  return (
    <Sheet open={isOpen} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="flex w-full flex-col sm:max-w-md">
        <SheetHeader className="shrink-0 space-y-4 pb-4">
          <SheetTitle>Connect an institution</SheetTitle>

          <div className="relative">
            <Icons.Search className="text-muted-foreground absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search institutions..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="!h-9 pl-9 pr-9 text-sm"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch("")}
                className="text-muted-foreground hover:text-foreground absolute right-3 top-1/2 -translate-y-1/2"
                aria-label="Clear search"
              >
                <Icons.Close className="h-4 w-4" />
              </button>
            )}
          </div>

          <ToggleGroup
            type="single"
            value={category}
            onValueChange={(value) => value && setCategory(value)}
            className="flex flex-wrap justify-start gap-1"
          >
            {CATEGORIES.map((cat) => (
              <ToggleGroupItem
                key={cat}
                value={cat}
                className="data-[state=on]:bg-primary data-[state=on]:text-primary-foreground h-7 rounded-full px-3 text-xs"
              >
                {CATEGORY_LABELS[cat]}
                {categoryCounts[cat] != null && (
                  <span className="ml-1 opacity-60">{categoryCounts[cat]}</span>
                )}
              </ToggleGroupItem>
            ))}
          </ToggleGroup>
        </SheetHeader>

        <ScrollArea className="min-h-0 flex-1">
          {isError ? (
            <div className="flex flex-col items-center gap-3 py-12 text-center">
              <Icons.AlertTriangle className="text-muted-foreground h-8 w-8" />
              <p className="text-muted-foreground text-sm">Failed to load institutions.</p>
              <Button variant="outline" size="sm" onClick={() => refetch()}>
                Try again
              </Button>
            </div>
          ) : isLoading ? (
            <div className="space-y-2 p-1">
              {Array.from({ length: 8 }).map((_, i) => (
                <Skeleton key={i} className="h-14 w-full rounded-md" />
              ))}
            </div>
          ) : filtered.length === 0 ? (
            <div className="text-muted-foreground py-12 text-center text-sm">
              No institutions found.
            </div>
          ) : (
            <div className="space-y-1 p-1">
              {filtered.map((connector) => (
                <ConnectorItem
                  key={connector.id}
                  connector={connector}
                  isPending={connectMutation.isPending}
                  onClick={() => connectMutation.mutate(connector.id)}
                />
              ))}
            </div>
          )}
        </ScrollArea>

        {onManualAdd && (
          <div className="shrink-0 border-t pt-4">
            <Button variant="outline" className="w-full" onClick={handleManualAdd}>
              <Icons.Pencil className="mr-2 h-4 w-4" />
              Enter manually
            </Button>
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}

function ConnectorItem({
  connector,
  isPending,
  onClick,
}: {
  connector: ProviderConnector;
  isPending: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={isPending}
      onClick={onClick}
      className="hover:bg-accent flex w-full items-center gap-3 rounded-md px-3 py-3 text-left transition-colors disabled:opacity-50"
    >
      {connector.logoUrl ? (
        <img
          src={connector.logoUrl}
          alt=""
          className="h-8 w-8 shrink-0 rounded-md object-contain"
        />
      ) : (
        <div
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-sm font-semibold text-white"
          style={{
            backgroundColor: connector.color ? `#${connector.color}` : undefined,
          }}
        >
          {connector.name.charAt(0).toUpperCase()}
        </div>
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{connector.name}</p>
        {connector.country && (
          <p className="text-muted-foreground truncate text-xs">{connector.country}</p>
        )}
      </div>
      <Icons.ChevronRight className="text-muted-foreground h-4 w-4 shrink-0" />
    </button>
  );
}
