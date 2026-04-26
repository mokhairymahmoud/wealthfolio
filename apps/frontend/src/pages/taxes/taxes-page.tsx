import {
  amendTaxYearReport,
  createTaxYearReport,
  deleteTaxDocument,
  downloadTaxDocument,
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
  updateTaxProfile,
  updateTaxReconciliationEntry,
  updateTaxEvent,
  uploadTaxDocument,
} from "@/adapters";
import { AccountType } from "@/lib/constants";
import { QueryKeys } from "@/lib/query-keys";
import type {
  Account,
  AccountTaxProfile,
  TaxEvent,
  TaxEventUpdate,
  TaxProfileUpdate,
  TaxReconciliationEntry,
  TaxReconciliationEntryUpdate,
  TaxReportDetail,
  TaxYearReport,
} from "@/lib/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@wealthfolio/ui/components/ui/alert-dialog";
import { Badge } from "@wealthfolio/ui/components/ui/badge";
import { Button } from "@wealthfolio/ui/components/ui/button";
import { Checkbox } from "@wealthfolio/ui/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@wealthfolio/ui/components/ui/dialog";
import { Input } from "@wealthfolio/ui/components/ui/input";
import { Label } from "@wealthfolio/ui/components/ui/label";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wealthfolio/ui/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@wealthfolio/ui/components/ui/dropdown-menu";
import { Switch } from "@wealthfolio/ui/components/ui/switch";
import { Textarea } from "@wealthfolio/ui/components/ui/textarea";
import { useCallback, useMemo, useState } from "react";

function currentTaxYear() {
  return new Date().getFullYear();
}

function findReportForYear(reports: TaxYearReport[] | undefined, year: number) {
  const matches = (reports ?? []).filter(
    (report) => report.taxYear === year && report.jurisdiction === "FR",
  );
  const statusRank: Record<string, number> = {
    DRAFT: 0,
    AMENDED_DRAFT: 1,
    FINALIZED: 2,
  };
  return matches.sort((a, b) => (statusRank[a.status] ?? 99) - (statusRank[b.status] ?? 99))[0];
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
  const results = detail?.extractions ?? [];
  const latestByDocument = new Map<string, TaxReportDetail["extractions"][number]>();
  for (const result of results) {
    if (!latestByDocument.has(result.extraction.documentId)) {
      latestByDocument.set(result.extraction.documentId, result);
    }
  }
  return Array.from(latestByDocument.values()).flatMap((result) => result.fields);
}

function formatSourceLocator(sourceLocatorJson: string | null | undefined) {
  if (!sourceLocatorJson) return null;
  try {
    const parsed = JSON.parse(sourceLocatorJson) as { lineNumber?: number; snippet?: string };
    const parts = [
      parsed.lineNumber != null ? `Line ${parsed.lineNumber}` : null,
      parsed.snippet ? parsed.snippet : null,
    ].filter(Boolean);
    return parts.length > 0 ? parts.join(" · ") : sourceLocatorJson;
  } catch {
    return sourceLocatorJson;
  }
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function TaxEventRow({
  event,
  disabled,
  onUpdate,
}: {
  event: TaxEvent;
  disabled: boolean;
  onUpdate: (update: TaxEventUpdate) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [showTrace, setShowTrace] = useState(false);
  const hasTraceDetails =
    (event.sources?.length ?? 0) > 0 || (event.lotAllocations?.length ?? 0) > 0;

  const startEdit = useCallback(() => {
    if (disabled) return;
    setDraft(event.taxableAmountEur != null ? String(event.taxableAmountEur) : "");
    setEditing(true);
  }, [disabled, event.taxableAmountEur]);

  const commitEdit = useCallback(() => {
    setEditing(false);
    const parsed = draft === "" ? null : Number(draft);
    if (parsed !== null && !Number.isFinite(parsed)) return;
    const current = event.taxableAmountEur != null ? Number(event.taxableAmountEur) : null;
    if (parsed === current) return;
    onUpdate({
      id: event.id,
      included: event.included,
      taxableAmountEur: parsed,
      notes: event.notes ?? null,
    });
  }, [draft, event, onUpdate]);

  return (
    <>
      <TableRow key={event.id} className={event.userOverride ? "bg-muted/40" : undefined}>
        <TableCell>
          <Checkbox
            checked={event.included}
            disabled={disabled}
            onCheckedChange={(checked) =>
              onUpdate({
                id: event.id,
                included: checked === true,
                taxableAmountEur: event.taxableAmountEur as number | null,
                notes: event.notes ?? null,
              })
            }
          />
        </TableCell>
        <TableCell>{event.eventDate}</TableCell>
        <TableCell>{event.eventType}</TableCell>
        <TableCell>{event.accountId}</TableCell>
        <TableCell
          className={!disabled ? "cursor-pointer" : undefined}
          onClick={!editing ? startEdit : undefined}
        >
          {editing ? (
            <Input
              className="h-7 w-28 text-sm"
              type="number"
              step="0.01"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={commitEdit}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitEdit();
                if (e.key === "Escape") setEditing(false);
              }}
              autoFocus
            />
          ) : (
            formatAmount(event.taxableAmountEur)
          )}
        </TableCell>
        <TableCell>
          <div className="flex items-center gap-1">
            <Badge variant={event.included ? "outline" : "destructive"}>{event.confidence}</Badge>
            {event.userOverride && <Icons.Pencil className="text-muted-foreground h-3 w-3" />}
          </div>
        </TableCell>
        <TableCell>
          {hasTraceDetails ? (
            <Button size="sm" variant="outline" onClick={() => setShowTrace((prev) => !prev)}>
              {showTrace ? "Hide" : "Trace"}
            </Button>
          ) : (
            <span className="text-muted-foreground text-xs">-</span>
          )}
        </TableCell>
      </TableRow>
      {showTrace && hasTraceDetails && (
        <TableRow>
          <TableCell colSpan={7} className="bg-muted/20">
            <div className="grid gap-4 py-2 md:grid-cols-2">
              <div className="space-y-2">
                <div className="text-sm font-medium">Sources</div>
                {(event.sources ?? []).length > 0 ? (
                  <div className="space-y-2">
                    {(event.sources ?? []).map((source) => (
                      <div key={source.id} className="rounded-md border p-2 text-sm">
                        <div className="font-medium">{source.sourceType}</div>
                        <div className="text-muted-foreground text-xs">{source.sourceId}</div>
                        {source.description && (
                          <div className="text-muted-foreground mt-1 text-xs">
                            {source.description}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-muted-foreground text-sm">No source rows recorded.</div>
                )}
              </div>
              <div className="space-y-2">
                <div className="text-sm font-medium">Lot Allocations</div>
                {(event.lotAllocations ?? []).length > 0 ? (
                  <div className="space-y-2">
                    {(event.lotAllocations ?? []).map((lot) => (
                      <div key={lot.id} className="rounded-md border p-2 text-sm">
                        <div className="font-medium">Activity {lot.sourceActivityId}</div>
                        <div className="text-muted-foreground text-xs">
                          Acquired {lot.acquisitionDate}
                        </div>
                        <div className="mt-1 text-xs">
                          Qty {lot.quantity} · Cost {formatAmount(lot.costBasisEur)}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-muted-foreground text-sm">No lot allocations recorded.</div>
                )}
              </div>
            </div>
          </TableCell>
        </TableRow>
      )}
    </>
  );
}

function TaxReconciliationRow({
  entry,
  disabled,
  onUpdate,
}: {
  entry: TaxReconciliationEntry;
  disabled: boolean;
  onUpdate: (update: TaxReconciliationEntryUpdate) => void;
}) {
  const [isOverrideEditing, setIsOverrideEditing] = useState(entry.status === "USER_OVERRIDE");
  const [manualAmount, setManualAmount] = useState(
    entry.selectedAmountEur != null ? String(entry.selectedAmountEur) : "",
  );
  const [manualReason, setManualReason] = useState(entry.notes ?? "");

  const selectAppAmount = useCallback(() => {
    onUpdate({
      id: entry.id,
      selectedAmountEur: entry.appAmountEur,
      status: "USER_SELECTED_APP",
      notes: null,
    });
    setIsOverrideEditing(false);
  }, [entry.appAmountEur, entry.id, onUpdate]);

  const selectDocumentAmount = useCallback(() => {
    onUpdate({
      id: entry.id,
      selectedAmountEur: entry.documentAmountEur,
      status: "USER_SELECTED_DOCUMENT",
      notes: null,
    });
    setIsOverrideEditing(false);
  }, [entry.documentAmountEur, entry.id, onUpdate]);

  const saveManualOverride = useCallback(() => {
    const parsed = manualAmount.trim() === "" ? null : Number(manualAmount);
    if (parsed === null || !Number.isFinite(parsed)) return;
    onUpdate({
      id: entry.id,
      selectedAmountEur: parsed,
      status: "USER_OVERRIDE",
      notes: manualReason,
    });
    setIsOverrideEditing(false);
  }, [entry.id, manualAmount, manualReason, onUpdate]);

  return (
    <TableRow key={entry.id}>
      <TableCell className="font-medium">{entry.category}</TableCell>
      <TableCell>{entry.suggestedBox ?? "-"}</TableCell>
      <TableCell>{formatAmount(entry.appAmountEur)}</TableCell>
      <TableCell>{formatAmount(entry.documentAmountEur)}</TableCell>
      <TableCell>{formatAmount(entry.selectedAmountEur)}</TableCell>
      <TableCell>
        <Badge variant={entry.status === "MATCHED" ? "default" : "outline"}>{entry.status}</Badge>
      </TableCell>
      <TableCell className="min-w-72">
        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant={entry.status === "USER_SELECTED_APP" ? "default" : "outline"}
            disabled={disabled || entry.appAmountEur == null}
            onClick={selectAppAmount}
          >
            Use App
          </Button>
          <Button
            size="sm"
            variant={entry.status === "USER_SELECTED_DOCUMENT" ? "default" : "outline"}
            disabled={disabled || entry.documentAmountEur == null}
            onClick={selectDocumentAmount}
          >
            Use IFU
          </Button>
          <Button
            size="sm"
            variant={isOverrideEditing || entry.status === "USER_OVERRIDE" ? "default" : "outline"}
            disabled={disabled}
            onClick={() => setIsOverrideEditing((prev) => !prev)}
          >
            Manual
          </Button>
        </div>
        {isOverrideEditing && (
          <div className="mt-2 flex flex-col gap-2 sm:flex-row">
            <Input
              className="h-8"
              type="number"
              step="0.01"
              value={manualAmount}
              onChange={(event) => setManualAmount(event.target.value)}
              placeholder="Manual EUR amount"
              disabled={disabled}
            />
            <Input
              className="h-8"
              value={manualReason}
              onChange={(event) => setManualReason(event.target.value)}
              placeholder="Reason required"
              disabled={disabled}
            />
            <Button size="sm" disabled={disabled} onClick={saveManualOverride}>
              Save
            </Button>
          </div>
        )}
      </TableCell>
    </TableRow>
  );
}

function ExtractionFieldRow({
  field,
  disabled,
  onConfirm,
  onCorrect,
  onReject,
}: {
  field: TaxReportDetail["extractions"][number]["fields"][number];
  disabled: boolean;
  onConfirm: () => void;
  onCorrect: (amount: number) => void;
  onReject: () => void;
}) {
  const [isCorrecting, setIsCorrecting] = useState(false);
  const [correctedAmount, setCorrectedAmount] = useState(
    field.confirmedAmountEur != null
      ? String(field.confirmedAmountEur)
      : field.amountEur != null
        ? String(field.amountEur)
        : "",
  );

  const saveCorrection = useCallback(() => {
    const parsed = correctedAmount.trim() === "" ? Number.NaN : Number(correctedAmount);
    if (!Number.isFinite(parsed)) return;
    onCorrect(parsed);
    setIsCorrecting(false);
  }, [correctedAmount, onCorrect]);

  return (
    <TableRow key={field.id}>
      <TableCell>
        <div className="font-medium">{field.label}</div>
        <div className="text-muted-foreground max-w-56 truncate text-xs">{field.valueText}</div>
      </TableCell>
      <TableCell>
        <div>{formatAmount(field.confirmedAmountEur ?? field.amountEur)}</div>
        <div className="text-muted-foreground text-xs">
          Confidence {Math.round(field.confidence * 100)}%
        </div>
        {field.suggestedDeclarationBox && (
          <div className="text-muted-foreground text-xs">Box {field.suggestedDeclarationBox}</div>
        )}
        {field.sourceLocatorJson && (
          <div className="text-muted-foreground text-xs">
            {formatSourceLocator(field.sourceLocatorJson)}
          </div>
        )}
      </TableCell>
      <TableCell>
        <Badge
          variant={
            field.status === "CONFIRMED" || field.status === "CORRECTED" ? "default" : "outline"
          }
        >
          {field.status}
        </Badge>
      </TableCell>
      <TableCell className="text-right">
        <div className="flex justify-end gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={onConfirm}
            disabled={disabled || field.status === "CONFIRMED"}
          >
            Confirm
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setIsCorrecting((prev) => !prev)}
            disabled={disabled}
          >
            Correct
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={onReject}
            disabled={disabled || field.status === "REJECTED"}
          >
            Reject
          </Button>
        </div>
        {isCorrecting && (
          <div className="mt-2 flex justify-end gap-2">
            <Input
              className="h-8 w-32"
              type="number"
              step="0.01"
              value={correctedAmount}
              onChange={(event) => setCorrectedAmount(event.target.value)}
              disabled={disabled}
            />
            <Button size="sm" onClick={saveCorrection} disabled={disabled}>
              Save
            </Button>
          </div>
        )}
      </TableCell>
    </TableRow>
  );
}

interface DocumentUploadCardProps {
  title: string;
  documents: Array<{
    id: string;
    filename: string;
    documentType: string;
    sizeBytes: number;
    sha256: string;
  }>;
  isLoading: boolean;
  isReportLocked: boolean;
  selectedFile: File | null;
  onFileChange: (file: File | null) => void;
  onUpload: () => void;
  isUploading: boolean;
  latestExtractionByDocument: Map<string, { extraction: { method: string; status: string } }>;
  rerunExtractionMutation: {
    mutate: (args: { documentId: string; method: string; consentGranted: boolean }) => void;
    isPending: boolean;
  };
  onCloudExtract: (documentId: string) => void;
  onPreview: (documentId: string) => void;
  downloadDocumentMutation: { mutate: (documentId: string) => void; isPending: boolean };
  deleteDocumentMutation: { mutate: (documentId: string) => void; isPending: boolean };
  emptyText: string;
}

function DocumentUploadCard({
  title,
  documents,
  isLoading,
  isReportLocked,
  selectedFile,
  onFileChange,
  onUpload,
  isUploading,
  latestExtractionByDocument,
  rerunExtractionMutation,
  onCloudExtract,
  onPreview,
  downloadDocumentMutation,
  deleteDocumentMutation,
  emptyText,
}: DocumentUploadCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icons.Upload className="h-5 w-5" />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-center gap-2">
          <input
            key={selectedFile ? "has-file" : "empty"}
            className="border-input bg-background h-9 rounded-md border px-3 py-1 text-sm"
            type="file"
            accept=".pdf,.txt,.csv,text/*,application/pdf"
            disabled={isReportLocked || isUploading}
            onChange={(event) => onFileChange(event.target.files?.[0] ?? null)}
          />
          <Button
            size="sm"
            onClick={onUpload}
            disabled={!selectedFile || isUploading || isReportLocked}
          >
            {isUploading ? (
              <Icons.Spinner className="h-4 w-4 animate-spin" />
            ) : (
              <Icons.FileText className="h-4 w-4" />
            )}
            Upload + Extract
          </Button>
        </div>
        <div className="space-y-2">
          {documents.map((document) => (
            <div
              key={document.id}
              className="flex items-center justify-between gap-3 rounded-md border p-3 text-sm"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{document.filename}</div>
                <div className="text-muted-foreground text-xs">
                  {document.documentType} · {Math.round(document.sizeBytes / 1024)} KB
                </div>
                <div className="text-muted-foreground truncate text-xs">
                  SHA-256 {document.sha256}
                </div>
                {latestExtractionByDocument.get(document.id) && (
                  <div className="text-muted-foreground mt-1 text-xs">
                    Latest extraction:{" "}
                    {latestExtractionByDocument.get(document.id)?.extraction.method} ·{" "}
                    <span className="font-medium">
                      {latestExtractionByDocument.get(document.id)?.extraction.status}
                    </span>
                  </div>
                )}
              </div>
              <div className="flex flex-shrink-0 items-center gap-2">
                <Badge variant="outline">Encrypted</Badge>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button size="icon" variant="ghost" aria-label="Document actions">
                      <Icons.MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      disabled={isReportLocked || rerunExtractionMutation.isPending}
                      onClick={() =>
                        rerunExtractionMutation.mutate({
                          documentId: document.id,
                          method: "LOCAL_TEXT",
                          consentGranted: false,
                        })
                      }
                    >
                      Re-extract (local)
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      disabled={isReportLocked || rerunExtractionMutation.isPending}
                      onClick={() => onCloudExtract(document.id)}
                    >
                      Use Cloud AI
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      disabled={!latestExtractionByDocument.get(document.id)}
                      onClick={() => onPreview(document.id)}
                    >
                      Preview text
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      disabled={downloadDocumentMutation.isPending}
                      onClick={() => downloadDocumentMutation.mutate(document.id)}
                    >
                      Download
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      className="text-destructive focus:text-destructive"
                      disabled={deleteDocumentMutation.isPending || isReportLocked}
                      onClick={() => {
                        if (window.confirm(`Delete ${document.filename}?`)) {
                          deleteDocumentMutation.mutate(document.id);
                        }
                      }}
                    >
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </div>
          ))}
          {isLoading && <Skeleton className="h-12" />}
          {!isLoading && documents.length === 0 && (
            <div className="text-muted-foreground text-sm">{emptyText}</div>
          )}
        </div>
      </CardContent>
    </Card>
  );
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
  const [selectedFicheFile, setSelectedFicheFile] = useState<File | null>(null);
  const [cloudExtractionDocumentId, setCloudExtractionDocumentId] = useState<string | null>(null);

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
  const isReportLocked = selectedReport?.status === "FINALIZED";

  const { data: reportDetail, isLoading: isReportDetailLoading } = useQuery({
    queryKey: [QueryKeys.TAX_REPORT_DETAIL, selectedReportId],
    queryFn: () =>
      selectedReportId ? getTaxReportDetail(selectedReportId) : Promise.resolve(null),
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

  const amendReportMutation = useMutation({
    mutationFn: (reportId: string) => amendTaxYearReport(reportId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.TAX_YEAR_REPORTS] });
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
        method: "LOCAL_TEXT",
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

  const uploadFicheDocumentMutation = useMutation({
    mutationFn: async () => {
      if (!selectedReport || !selectedFicheFile) {
        throw new Error("Missing report or document");
      }
      const content = Array.from(new Uint8Array(await selectedFicheFile.arrayBuffer()));
      const document = await uploadTaxDocument({
        reportId: selectedReport.id,
        documentType: "FICHE_DE_PAIE",
        filename: selectedFicheFile.name,
        mimeType: selectedFicheFile.type || null,
        content,
      });
      return extractTaxDocument({
        documentId: document.id,
        method: "LOCAL_TEXT",
        consentGranted: false,
      });
    },
    onSuccess: () => {
      setSelectedFicheFile(null);
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

  const correctFieldMutation = useMutation({
    mutationFn: ({ fieldId, amount }: { fieldId: string; amount: number }) =>
      updateExtractedTaxField({
        fieldId,
        status: "CORRECTED",
        confirmedAmountEur: amount,
      }),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const rejectFieldMutation = useMutation({
    mutationFn: (fieldId: string) =>
      updateExtractedTaxField({
        fieldId,
        status: "REJECTED",
        confirmedAmountEur: null,
      }),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const rerunExtractionMutation = useMutation({
    mutationFn: ({
      documentId,
      method,
      consentGranted,
    }: {
      documentId: string;
      method: string;
      consentGranted: boolean;
    }) => extractTaxDocument({ documentId, method, consentGranted }),
    onSuccess: () => {
      setCloudExtractionDocumentId(null);
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

  const deleteDocumentMutation = useMutation({
    mutationFn: (documentId: string) => deleteTaxDocument(documentId),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const downloadDocumentMutation = useMutation({
    mutationFn: (documentId: string) => downloadTaxDocument(documentId),
    onSuccess: (download) => {
      const buffer = download.content.buffer.slice(
        download.content.byteOffset,
        download.content.byteOffset + download.content.byteLength,
      ) as ArrayBuffer;
      const blob = new Blob([buffer], {
        type: download.mimeType ?? "application/octet-stream",
      });
      const url = URL.createObjectURL(blob);
      const link = window.document.createElement("a");
      link.href = url;
      link.download = download.filename;
      window.document.body.appendChild(link);
      link.click();
      window.document.body.removeChild(link);
      URL.revokeObjectURL(url);
    },
  });

  const updateTaxEventMutation = useMutation({
    mutationFn: (update: TaxEventUpdate) => updateTaxEvent(update),
    onSuccess: () => {
      if (selectedReport) {
        queryClient.invalidateQueries({ queryKey: QueryKeys.taxReportDetail(selectedReport.id) });
      }
    },
  });

  const updateTaxReconciliationEntryMutation = useMutation({
    mutationFn: (update: TaxReconciliationEntryUpdate) => updateTaxReconciliationEntry(update),
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

  const { mutate: mutateTaxProfile, isPending: isTaxProfilePending } = useMutation({
    mutationFn: (update: TaxProfileUpdate) => updateTaxProfile(update),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [QueryKeys.TAX_PROFILE] });
    },
  });

  const handleFoyerFiscalChange = useCallback(
    (field: keyof TaxProfileUpdate, value: string | number | boolean) => {
      if (!profile) return;
      const update: TaxProfileUpdate = {
        jurisdiction: profile.jurisdiction,
        taxResidenceCountry: profile.taxResidenceCountry,
        defaultTaxRegime: profile.defaultTaxRegime,
        pfuOrBaremePreference: profile.pfuOrBaremePreference,
        situationFamiliale: profile.situationFamiliale,
        nombreEnfants: profile.nombreEnfants,
        nombreEnfantsHandicapes: profile.nombreEnfantsHandicapes,
        parentIsole: profile.parentIsole,
        ancienCombattantOuInvalidite: profile.ancienCombattantOuInvalidite,
        [field]: value,
      };
      mutateTaxProfile(update);
    },
    [profile, mutateTaxProfile],
  );

  const securitiesAccounts = useMemo(
    () => (accounts ?? []).filter((account) => account.accountType === AccountType.SECURITIES),
    [accounts],
  );
  const accountProfiles = useMemo(
    () => accountProfileById(accountTaxProfiles),
    [accountTaxProfiles],
  );
  const extractedFields = flattenExtractedFields(reportDetail);
  const latestExtractionByDocument = useMemo(() => {
    const byDocument = new Map<string, TaxReportDetail["extractions"][number]>();
    for (const extraction of reportDetail?.extractions ?? []) {
      if (!byDocument.has(extraction.extraction.documentId)) {
        byDocument.set(extraction.extraction.documentId, extraction);
      }
    }
    return byDocument;
  }, [reportDetail]);
  const [previewDocumentId, setPreviewDocumentId] = useState<string | null>(null);
  const documentNameById = useMemo(() => {
    const byId = new Map<string, string>();
    for (const document of reportDetail?.documents ?? []) {
      byId.set(document.id, document.filename);
    }
    return byId;
  }, [reportDetail]);
  const latestExtractionPreview = previewDocumentId
    ? (latestExtractionByDocument.get(previewDocumentId)?.extraction.rawTextPreview ?? null)
    : null;
  const summary = useMemo(() => {
    const events = reportDetail?.events ?? [];
    const includedEvents = events.filter((event) => event.included);
    const sumCategory = (categories: string[]) =>
      includedEvents.reduce((sum, event) => {
        if (!categories.includes(event.category)) return sum;
        const value = event.taxableAmountEur == null ? 0 : Number(event.taxableAmountEur);
        return Number.isFinite(value) ? sum + value : sum;
      }, 0);

    return {
      salaryIncome: sumCategory(["SALARY_INCOME"]),
      taxableIncome: sumCategory(["DIVIDENDS", "INTEREST"]),
      realizedGains: sumCategory(["SECURITY_GAINS"]),
      withholdingTax: sumCategory(["FOREIGN_WITHHOLDING_TAX"]),
      needsReviewCount:
        (reportDetail?.issues?.length ?? 0) +
        extractedFields.filter((field) => field.status === "SUGGESTED").length,
    };
  }, [extractedFields, reportDetail]);
  const extractionActionsDisabled =
    isReportLocked ||
    confirmFieldMutation.isPending ||
    correctFieldMutation.isPending ||
    rejectFieldMutation.isPending;
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
          {selectedReport?.status === "FINALIZED" ? (
            <Button
              variant="outline"
              onClick={() => selectedReport && amendReportMutation.mutate(selectedReport.id)}
              disabled={!selectedReport || amendReportMutation.isPending}
            >
              {amendReportMutation.isPending ? (
                <Icons.Spinner className="h-4 w-4 animate-spin" />
              ) : (
                <Icons.Pencil className="h-4 w-4" />
              )}
              Amend
            </Button>
          ) : (
            <>
              <Button
                variant="outline"
                onClick={() => selectedReport && regenerateReportMutation.mutate(selectedReport.id)}
                disabled={!selectedReport || regenerateReportMutation.isPending}
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
                disabled={!selectedReport || finalizeReportMutation.isPending}
              >
                {finalizeReportMutation.isPending ? (
                  <Icons.Spinner className="h-4 w-4 animate-spin" />
                ) : (
                  <Icons.ShieldCheck className="h-4 w-4" />
                )}
                Finalize
              </Button>
            </>
          )}
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
              {selectedReport ? (
                <Badge>{selectedReport.status}</Badge>
              ) : (
                <Badge variant="outline">None</Badge>
              )}
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
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">Foyer Fiscal</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-6">
            <div className="space-y-1">
              <Label className="text-xs">Situation familiale</Label>
              <Select
                value={profile?.situationFamiliale ?? "CELIBATAIRE"}
                onValueChange={(value) => handleFoyerFiscalChange("situationFamiliale", value)}
                disabled={isTaxProfilePending}
              >
                <SelectTrigger className="h-8">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="CELIBATAIRE">Celibataire</SelectItem>
                  <SelectItem value="MARIE">Marie(e)</SelectItem>
                  <SelectItem value="PACSE">Pacse(e)</SelectItem>
                  <SelectItem value="DIVORCE">Divorce(e)</SelectItem>
                  <SelectItem value="VEUF">Veuf/Veuve</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Enfants a charge</Label>
              <Input
                type="number"
                min={0}
                className="h-8"
                value={profile?.nombreEnfants ?? 0}
                onChange={(e) =>
                  handleFoyerFiscalChange(
                    "nombreEnfants",
                    Math.max(0, parseInt(e.target.value) || 0),
                  )
                }
                disabled={isTaxProfilePending}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Enfants handicapes</Label>
              <Input
                type="number"
                min={0}
                className="h-8"
                value={profile?.nombreEnfantsHandicapes ?? 0}
                onChange={(e) =>
                  handleFoyerFiscalChange(
                    "nombreEnfantsHandicapes",
                    Math.max(0, parseInt(e.target.value) || 0),
                  )
                }
                disabled={isTaxProfilePending}
              />
            </div>
            <div className="flex items-end gap-2 pb-1">
              <div className="space-y-1">
                <Label className="text-xs">Parent isole</Label>
                <div className="flex h-8 items-center">
                  <Switch
                    checked={profile?.parentIsole ?? false}
                    onCheckedChange={(checked) => handleFoyerFiscalChange("parentIsole", checked)}
                    disabled={isTaxProfilePending}
                  />
                </div>
              </div>
            </div>
            <div className="flex items-end gap-2 pb-1">
              <div className="space-y-1">
                <Label className="text-xs">Invalidite / Ancien combattant</Label>
                <div className="flex h-8 items-center">
                  <Switch
                    checked={profile?.ancienCombattantOuInvalidite ?? false}
                    onCheckedChange={(checked) =>
                      handleFoyerFiscalChange("ancienCombattantOuInvalidite", checked)
                    }
                    disabled={isTaxProfilePending}
                  />
                </div>
              </div>
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Nombre de parts</Label>
              <div className="flex h-8 items-center">
                <span className="text-2xl font-semibold">{profile?.nombreParts ?? 1}</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-5">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Salary Income</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{formatAmount(summary.salaryIncome)}</div>
            <p className="text-muted-foreground text-xs">
              Net imposable from fiche de paie (box 1AJ)
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Taxable Income</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{formatAmount(summary.taxableIncome)}</div>
            <p className="text-muted-foreground text-xs">
              Dividends and interest currently included
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Realized Gains/Losses</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{formatAmount(summary.realizedGains)}</div>
            <p className="text-muted-foreground text-xs">Included disposal events in EUR</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Withholding Tax</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{formatAmount(summary.withholdingTax)}</div>
            <p className="text-muted-foreground text-xs">
              Reserved for supported withholding events
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Needs Review</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{summary.needsReviewCount}</div>
            <p className="text-muted-foreground text-xs">Issues plus suggested extracted fields</p>
          </CardContent>
        </Card>
      </div>

      {selectedReport && (
        <Card>
          <CardHeader>
            <CardTitle>Report Snapshot</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4 md:grid-cols-4">
            <div>
              <div className="text-muted-foreground text-xs uppercase tracking-wide">Rule Pack</div>
              <div className="mt-1 text-sm font-medium">{selectedReport.rulePackVersion}</div>
            </div>
            <div>
              <div className="text-muted-foreground text-xs uppercase tracking-wide">Generated</div>
              <div className="mt-1 text-sm font-medium">
                {formatDateTime(selectedReport.generatedAt)}
              </div>
            </div>
            <div>
              <div className="text-muted-foreground text-xs uppercase tracking-wide">Finalized</div>
              <div className="mt-1 text-sm font-medium">
                {formatDateTime(selectedReport.finalizedAt)}
              </div>
            </div>
            <div>
              <div className="text-muted-foreground text-xs uppercase tracking-wide">Status</div>
              <div className="mt-1 text-sm font-medium">{selectedReport.status}</div>
            </div>
          </CardContent>
        </Card>
      )}

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
                      {taxProfile ? (
                        <Badge>{taxProfile.regime}</Badge>
                      ) : (
                        <Badge variant="outline">Unset</Badge>
                      )}
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
        <div className="grid gap-4 md:grid-cols-3">
          <DocumentUploadCard
            title="IFU Documents"
            documents={(reportDetail?.documents ?? []).filter((d) => d.documentType === "IFU")}
            isLoading={isReportDetailLoading}
            isReportLocked={isReportLocked}
            selectedFile={selectedFile}
            onFileChange={(file) => setSelectedFile(file)}
            onUpload={() => uploadDocumentMutation.mutate()}
            isUploading={uploadDocumentMutation.isPending}
            latestExtractionByDocument={latestExtractionByDocument}
            rerunExtractionMutation={rerunExtractionMutation}
            onCloudExtract={(id) => setCloudExtractionDocumentId(id)}
            onPreview={(id) => setPreviewDocumentId(id)}
            downloadDocumentMutation={downloadDocumentMutation}
            deleteDocumentMutation={deleteDocumentMutation}
            emptyText="No IFU document uploaded."
          />

          <DocumentUploadCard
            title="Fiches de Paie"
            documents={(reportDetail?.documents ?? []).filter(
              (d) => d.documentType === "FICHE_DE_PAIE",
            )}
            isLoading={isReportDetailLoading}
            isReportLocked={isReportLocked}
            selectedFile={selectedFicheFile}
            onFileChange={(file) => setSelectedFicheFile(file)}
            onUpload={() => uploadFicheDocumentMutation.mutate()}
            isUploading={uploadFicheDocumentMutation.isPending}
            latestExtractionByDocument={latestExtractionByDocument}
            rerunExtractionMutation={rerunExtractionMutation}
            onCloudExtract={(id) => setCloudExtractionDocumentId(id)}
            onPreview={(id) => setPreviewDocumentId(id)}
            downloadDocumentMutation={downloadDocumentMutation}
            deleteDocumentMutation={deleteDocumentMutation}
            emptyText="No fiche de paie uploaded. Upload the last pay slip of the year (cumul annuel)."
          />

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
                    <ExtractionFieldRow
                      key={field.id}
                      field={field}
                      disabled={extractionActionsDisabled}
                      onConfirm={() => confirmFieldMutation.mutate(field.id)}
                      onCorrect={(amount) =>
                        correctFieldMutation.mutate({ fieldId: field.id, amount })
                      }
                      onReject={() => rejectFieldMutation.mutate(field.id)}
                    />
                  ))}
                  {extractedFields.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={4} className="text-muted-foreground py-8 text-center">
                        No extracted fields yet.
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
                  {issue.documentId && (
                    <p className="text-muted-foreground mt-1 text-xs">
                      Document {documentNameById.get(issue.documentId) ?? issue.documentId}
                    </p>
                  )}
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
                disabled={!selectedReport || reconcileReportMutation.isPending || isReportLocked}
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
                  <TableHead>App</TableHead>
                  <TableHead>IFU</TableHead>
                  <TableHead>Selected</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Choice</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(reportDetail?.reconciliation ?? []).map((entry) => (
                  <TaxReconciliationRow
                    key={entry.id}
                    entry={entry}
                    disabled={isReportLocked || updateTaxReconciliationEntryMutation.isPending}
                    onUpdate={(update) => updateTaxReconciliationEntryMutation.mutate(update)}
                  />
                ))}
                {(reportDetail?.reconciliation ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7} className="text-muted-foreground py-8 text-center">
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
                  <TableHead className="w-10">Incl.</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Account</TableHead>
                  <TableHead>Taxable EUR</TableHead>
                  <TableHead>Confidence</TableHead>
                  <TableHead>Traceability</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(reportDetail?.events ?? []).map((event) => (
                  <TaxEventRow
                    key={event.id}
                    event={event}
                    disabled={
                      selectedReport.status === "FINALIZED" || updateTaxEventMutation.isPending
                    }
                    onUpdate={(update) => updateTaxEventMutation.mutate(update)}
                  />
                ))}
                {(reportDetail?.events ?? []).length === 0 && (
                  <TableRow>
                    <TableCell colSpan={7} className="text-muted-foreground py-8 text-center">
                      No tax events generated.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}

      <AlertDialog
        open={cloudExtractionDocumentId !== null}
        onOpenChange={(open) => {
          if (!open) setCloudExtractionDocumentId(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Allow cloud extraction for this document?</AlertDialogTitle>
            <AlertDialogDescription>
              Cloud extraction may send document text to an external AI provider. Use this only if
              local extraction was insufficient and you consent to that transfer for this document.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="space-y-2">
            <Label htmlFor="cloud-extraction-notes">What will happen</Label>
            <Textarea
              id="cloud-extraction-notes"
              value="Wealthfolio will record your consent and run the cloud extraction path for this document. Unconfirmed values will still stay out of reconciliation totals until you review them."
              readOnly
              className="min-h-24"
            />
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={rerunExtractionMutation.isPending}>
              Cancel
            </AlertDialogCancel>
            <Button
              onClick={() => {
                if (!cloudExtractionDocumentId) return;
                rerunExtractionMutation.mutate({
                  documentId: cloudExtractionDocumentId,
                  method: "CLOUD_AI",
                  consentGranted: true,
                });
              }}
              disabled={!cloudExtractionDocumentId || rerunExtractionMutation.isPending}
            >
              {rerunExtractionMutation.isPending ? (
                <Icons.Spinner className="h-4 w-4 animate-spin" />
              ) : null}
              I Consent
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <Dialog
        open={previewDocumentId !== null}
        onOpenChange={(open) => {
          if (!open) setPreviewDocumentId(null);
        }}
      >
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Extracted Text Preview</DialogTitle>
            <DialogDescription>
              Local text extracted from{" "}
              {previewDocumentId
                ? documentNameById.get(previewDocumentId)
                : "the selected document"}
              .
            </DialogDescription>
          </DialogHeader>
          <Textarea
            value={latestExtractionPreview ?? "No extracted text preview available."}
            readOnly
            className="min-h-96 font-mono text-xs"
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}
