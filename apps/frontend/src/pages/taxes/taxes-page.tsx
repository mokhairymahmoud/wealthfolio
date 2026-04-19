import {
  createTaxYearReport,
  extractTaxDocument,
  finalizeTaxYearReport,
  getAccountTaxProfiles,
  getAccounts,
  getTaxProfile,
  getTaxReportDetail,
  listTaxYearReports,
  regenerateTaxYearReport,
  reconcileTaxYearReport,
  updateAccountTaxProfile,
  updateExtractedTaxField,
  uploadTaxDocument,
} from "@/adapters";
import { AccountType } from "@/lib/constants";
import { QueryKeys } from "@/lib/query-keys";
import type { Account, AccountTaxProfile, TaxReportDetail, TaxYearReport } from "@/lib/types";
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

function formatAmount(value: number | string | null | undefined) {
  if (value === null || value === undefined) {
    return "-";
  }
  const amount = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(amount)) {
    return String(value);
  }
  return new Intl.NumberFormat("fr-FR", {
    style: "currency",
    currency: "EUR",
    maximumFractionDigits: 2,
  }).format(amount);
}

function flattenExtractedFields(detail: TaxReportDetail | null | undefined) {
  return (detail?.extractions ?? []).flatMap((result) => result.fields);
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
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

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
  const selectedReport = findReportForYear(reports, taxYear);
  const selectedReportId = selectedReport?.id;

  const { data: reportDetail, isLoading: isReportDetailLoading } = useQuery({
    queryKey: [QueryKeys.TAX_REPORT_DETAIL, selectedReportId],
    queryFn: () => (selectedReportId ? getTaxReportDetail(selectedReportId) : Promise.resolve(null)),
    enabled: Boolean(selectedReportId),
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

  const regenerateReportMutation = useMutation({
    mutationFn: (reportId: string) => regenerateTaxYearReport(reportId),
    onSuccess: (detail) => {
      queryClient.setQueryData(QueryKeys.taxReportDetail(detail.report.id), detail);
      queryClient.invalidateQueries({ queryKey: [QueryKeys.TAX_YEAR_REPORTS] });
    },
  });

  const finalizeReportMutation = useMutation({
    mutationFn: (reportId: string) => finalizeTaxYearReport(reportId),
    onSuccess: (report) => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.TAX_YEAR_REPORTS] });
      queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(report.id) });
    },
  });

  const uploadDocumentMutation = useMutation({
    mutationFn: async () => {
      if (!selectedReport || !selectedFile) {
        throw new Error("Missing report or document");
      }
      const content = Array.from(new Uint8Array(await selectedFile.arrayBuffer()));
      const document = await uploadTaxDocument({
        reportId: selectedReport.id,
        documentType: "IFU",
        filename: selectedFile.name,
        mimeType: selectedFile.type || null,
        content,
      });
      return extractTaxDocument({
        documentId: document.id,
        method: "LOCAL_HEURISTIC",
        consentGranted: false,
      });
    },
    onSuccess: () => {
      setSelectedFile(null);
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const confirmFieldMutation = useMutation({
    mutationFn: (fieldId: string) =>
      updateExtractedTaxField({
        fieldId,
        status: "CONFIRMED",
        confirmedAmountEur: null,
      }),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const reconcileReportMutation = useMutation({
    mutationFn: (reportId: string) => reconcileTaxYearReport(reportId),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
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
  const extractedFields = flattenExtractedFields(reportDetail);
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
          <Button
            variant="outline"
            onClick={() => selectedReport && regenerateReportMutation.mutate(selectedReport.id)}
            disabled={
              !selectedReport ||
              selectedReport.status === "FINALIZED" ||
              regenerateReportMutation.isPending
            }
          >
            {regenerateReportMutation.isPending ? (
              <Icons.Spinner className="h-4 w-4 animate-spin" />
            ) : (
              <Icons.RefreshCw className="h-4 w-4" />
            )}
            Generate
          </Button>
          <Button
            variant="outline"
            onClick={() => selectedReport && finalizeReportMutation.mutate(selectedReport.id)}
            disabled={
              !selectedReport ||
              selectedReport.status === "FINALIZED" ||
              finalizeReportMutation.isPending
            }
          >
            {finalizeReportMutation.isPending ? (
              <Icons.Spinner className="h-4 w-4 animate-spin" />
            ) : (
              <Icons.ShieldCheck className="h-4 w-4" />
            )}
            Finalize
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

      {selectedReport && (
        <div className="grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Icons.Upload className="h-5 w-5" />
                IFU Documents
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className="border-input bg-background h-9 rounded-md border px-3 py-1 text-sm"
                  type="file"
                  accept=".pdf,.txt,.csv,text/*,application/pdf"
                  onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
                />
                <Button
                  size="sm"
                  onClick={() => uploadDocumentMutation.mutate()}
                  disabled={!selectedFile || uploadDocumentMutation.isPending}
                >
                  {uploadDocumentMutation.isPending ? (
                    <Icons.Spinner className="h-4 w-4 animate-spin" />
                  ) : (
                    <Icons.FileText className="h-4 w-4" />
                  )}
                  Upload + Extract
                </Button>
              </div>
              <div className="space-y-2">
                {(reportDetail?.documents ?? []).map((document) => (
                  <div
                    key={document.id}
                    className="flex items-center justify-between gap-3 rounded-md border p-3 text-sm"
                  >
                    <div>
                      <div className="font-medium">{document.filename}</div>
                      <div className="text-muted-foreground text-xs">
                        {document.documentType} · {Math.round(document.sizeBytes / 1024)} KB
                      </div>
                    </div>
                    <Badge variant="outline">Encrypted</Badge>
                  </div>
                ))}
                {isReportDetailLoading && <Skeleton className="h-12" />}
                {!isReportDetailLoading && (reportDetail?.documents ?? []).length === 0 && (
                  <div className="text-muted-foreground text-sm">No IFU document uploaded.</div>
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Extraction Review</CardTitle>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Field</TableHead>
                    <TableHead>Amount</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-right">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {extractedFields.map((field) => (
                    <TableRow key={field.id}>
                      <TableCell>
                        <div className="font-medium">{field.label}</div>
                        <div className="text-muted-foreground max-w-56 truncate text-xs">
                          {field.valueText}
                        </div>
                      </TableCell>
                      <TableCell>{formatAmount(field.confirmedAmountEur ?? field.amountEur)}</TableCell>
                      <TableCell>
                        <Badge variant={field.status === "CONFIRMED" ? "default" : "outline"}>
                          {field.status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => confirmFieldMutation.mutate(field.id)}
                          disabled={
                            field.status === "CONFIRMED" || confirmFieldMutation.isPending
                          }
                        >
                          Confirm
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                  {extractedFields.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={4} className="text-muted-foreground py-8 text-center">
                        No extracted IFU fields.
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Issues</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {(reportDetail?.issues ?? []).map((issue) => (
                <div key={issue.id} className="rounded-md border p-3 text-sm">
                  <div className="flex items-center gap-2">
                    <Badge variant={issue.severity === "ERROR" ? "destructive" : "outline"}>
                      {issue.severity}
                    </Badge>
                    <span className="font-medium">{issue.code}</span>
                  </div>
                  <p className="text-muted-foreground mt-1">{issue.message}</p>
                </div>
              ))}
              {(reportDetail?.issues ?? []).length === 0 && (
                <div className="text-muted-foreground text-sm">No generated issues.</div>
              )}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between gap-2">
              <span>Declaration Helper</span>
              <Button
                size="sm"
                variant="outline"
                onClick={() => selectedReport && reconcileReportMutation.mutate(selectedReport.id)}
                disabled={!selectedReport || reconcileReportMutation.isPending}
              >
                {reconcileReportMutation.isPending ? (
                  <Icons.Spinner className="h-4 w-4 animate-spin" />
                ) : (
                  <Icons.ListChecks className="h-4 w-4" />
                )}
                Reconcile
              </Button>
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Category</TableHead>
                  <TableHead>Box</TableHead>
                  <TableHead>Selected</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(reportDetail?.reconciliation ?? []).map((entry) => (
                  <TableRow key={entry.id}>
                    <TableCell className="font-medium">{entry.category}</TableCell>
                    <TableCell>{entry.suggestedBox ?? "-"}</TableCell>
                    <TableCell>{formatAmount(entry.selectedAmountEur)}</TableCell>
                    <TableCell>
                      <Badge variant={entry.status === "MATCHED" ? "default" : "outline"}>
                        {entry.status}
                      </Badge>
                    </TableCell>
                  </TableRow>
                ))}
                {(reportDetail?.reconciliation ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={4} className="text-muted-foreground py-8 text-center">
                      No declaration lines generated.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>

      {selectedReport && (
        <Card>
          <CardHeader>
            <CardTitle>Tax Event Ledger</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Account</TableHead>
                  <TableHead>Taxable EUR</TableHead>
                  <TableHead>Confidence</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(reportDetail?.events ?? []).map((event) => (
                  <TableRow key={event.id}>
                    <TableCell>{event.eventDate}</TableCell>
                    <TableCell>{event.eventType}</TableCell>
                    <TableCell>{event.accountId}</TableCell>
                    <TableCell>{formatAmount(event.taxableAmountEur)}</TableCell>
                    <TableCell>
                      <Badge variant={event.included ? "outline" : "destructive"}>
                        {event.confidence}
                      </Badge>
                    </TableCell>
                  </TableRow>
                ))}
                {(reportDetail?.events ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={5} className="text-muted-foreground py-8 text-center">
                      No tax events generated.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
