import {
  createTaxYearReport,
  getAccountTaxProfiles,
  getAccounts,
  getTaxProfile,
  listTaxYearReports,
  updateAccountTaxProfile,
} from "@/adapters";
import { AccountType } from "@/lib/constants";
import { QueryKeys } from "@/lib/query-keys";
import type { Account, AccountTaxProfile, TaxYearReport } from "@/lib/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@wealthfolio/ui/components/ui/card";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { Skeleton } from "@wealthfolio/ui/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@wealthfolio/ui/components/ui/table";
import { useMemo, useState } from "react";

function currentTaxYear() {
  return new Date().getFullYear();
}

function findReportForYear(reports: TaxYearReport[] | undefined, year: number) {
  return reports?.find((report) => report.taxYear === year && report.jurisdiction === "FR");
}

function accountProfileById(profiles: AccountTaxProfile[] | undefined) {
  return new Map((profiles ?? []).map((profile) => [profile.accountId, profile]));
}

function TaxesSkeleton() {
  return (
    <div className="space-y-4 p-4">
      <Skeleton className="h-10 w-56" />
      <div className="grid gap-4 md:grid-cols-3">
        <Skeleton className="h-28" />
        <Skeleton className="h-28" />
        <Skeleton className="h-28" />
      </div>
      <Skeleton className="h-72" />
    </div>
  );
}

export default function TaxesPage() {
  const queryClient = useQueryClient();
  const [taxYear, setTaxYear] = useState(currentTaxYear());

  const { data: profile, isLoading: isProfileLoading } = useQuery({
    queryKey: [QueryKeys.TAX_PROFILE],
    queryFn: getTaxProfile,
  });

  const { data: accounts, isLoading: areAccountsLoading } = useQuery<Account[]>({
    queryKey: [QueryKeys.ACCOUNTS],
    queryFn: () => getAccounts(),
  });

  const { data: accountTaxProfiles, isLoading: areAccountProfilesLoading } = useQuery({
    queryKey: [QueryKeys.ACCOUNT_TAX_PROFILES],
    queryFn: getAccountTaxProfiles,
  });

  const { data: reports, isLoading: areReportsLoading } = useQuery({
    queryKey: [QueryKeys.TAX_YEAR_REPORTS],
    queryFn: listTaxYearReports,
  });

  const createReportMutation = useMutation({
    mutationFn: () =>
      createTaxYearReport({
        taxYear,
        jurisdiction: "FR",
        baseCurrency: "EUR",
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.TAX_YEAR_REPORTS] });
    },
  });

  const updateAccountTaxProfileMutation = useMutation({
    mutationFn: (account: Account) =>
      updateAccountTaxProfile({
        accountId: account.id,
        jurisdiction: "FR",
        regime: "CTO",
        openedOn: null,
        closedOn: null,
        metadata: null,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.ACCOUNT_TAX_PROFILES] });
    },
  });

  const securitiesAccounts = useMemo(
    () => (accounts ?? []).filter((account) => account.accountType === AccountType.SECURITIES),
    [accounts],
  );
  const accountProfiles = useMemo(() => accountProfileById(accountTaxProfiles), [accountTaxProfiles]);
  const selectedReport = findReportForYear(reports, taxYear);
  const isLoading =
    isProfileLoading || areAccountsLoading || areAccountProfilesLoading || areReportsLoading;

  if (isLoading) {
    return <TaxesSkeleton />;
  }

  return (
    <div className="space-y-6 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">Taxes</h1>
          <p className="text-muted-foreground text-sm">France declaration assistant</p>
        </div>
        <div className="flex items-center gap-2">
          <select
            className="border-input bg-background h-9 rounded-md border px-3 text-sm"
            value={taxYear}
            onChange={(event) => setTaxYear(Number(event.target.value))}
          >
            {[currentTaxYear(), currentTaxYear() - 1, currentTaxYear() - 2].map((year) => (
              <option key={year} value={year}>
                {year}
              </option>
            ))}
          </select>
          <Button
            onClick={() => createReportMutation.mutate()}
            disabled={createReportMutation.isPending}
          >
            {createReportMutation.isPending ? (
              <Icons.Spinner className="h-4 w-4 animate-spin" />
            ) : (
              <Icons.FileText className="h-4 w-4" />
            )}
            {selectedReport ? "Open Draft" : "Create Draft"}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Jurisdiction</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{profile?.jurisdiction ?? "FR"}</div>
            <p className="text-muted-foreground text-xs">
              Residence {profile?.taxResidenceCountry ?? "FR"}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Report</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <span className="text-2xl font-semibold">{taxYear}</span>
              {selectedReport ? <Badge>{selectedReport.status}</Badge> : <Badge variant="outline">None</Badge>}
            </div>
            <p className="text-muted-foreground text-xs">
              {selectedReport?.rulePackVersion ?? `FR-${taxYear}-securities-v1`}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Base Currency</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{selectedReport?.baseCurrency ?? "EUR"}</div>
            <p className="text-muted-foreground text-xs">
              {profile?.pfuOrBaremePreference ?? "PFU"} preference
            </p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Icons.ReceiptText className="h-5 w-5" />
            Account Tax Regimes
          </CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Account</TableHead>
                <TableHead>Currency</TableHead>
                <TableHead>Regime</TableHead>
                <TableHead className="text-right">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {securitiesAccounts.map((account) => {
                const taxProfile = accountProfiles.get(account.id);
                return (
                  <TableRow key={account.id}>
                    <TableCell className="font-medium">{account.name}</TableCell>
                    <TableCell>{account.currency}</TableCell>
                    <TableCell>
                      {taxProfile ? <Badge>{taxProfile.regime}</Badge> : <Badge variant="outline">Unset</Badge>}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        size="sm"
                        variant={taxProfile?.regime === "CTO" ? "outline" : "default"}
                        onClick={() => updateAccountTaxProfileMutation.mutate(account)}
                        disabled={updateAccountTaxProfileMutation.isPending}
                      >
                        {taxProfile?.regime === "CTO" ? "Refresh CTO" : "Mark CTO"}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {securitiesAccounts.length === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground py-8 text-center">
                    No securities accounts.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Issues</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground text-sm">No issues generated yet.</CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Declaration Helper</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground text-sm">
            No declaration lines generated yet.
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
