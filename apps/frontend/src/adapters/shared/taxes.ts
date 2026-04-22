import type {
  AccountTaxProfile,
  AccountTaxProfileUpdate,
  ExtractedTaxField,
  ExtractedTaxFieldUpdate,
  NewTaxYearReport,
  TaxDocument,
  TaxDocumentDownload,
  TaxDocumentExtractionRequest,
  TaxDocumentExtractionResult,
  TaxDocumentUpload,
  TaxEvent,
  TaxEventUpdate,
  TaxProfile,
  TaxProfileUpdate,
  TaxReconciliationEntry,
  TaxReconciliationEntryUpdate,
  TaxReportDetail,
  TaxYearReport,
} from "@/lib/types";

import { invoke } from "./platform";

export const getTaxProfile = async (): Promise<TaxProfile> => {
  return invoke<TaxProfile>("get_tax_profile");
};

export const updateTaxProfile = async (profile: TaxProfileUpdate): Promise<TaxProfile> => {
  return invoke<TaxProfile>("update_tax_profile", { profile });
};

export const getAccountTaxProfiles = async (): Promise<AccountTaxProfile[]> => {
  return invoke<AccountTaxProfile[]>("get_account_tax_profiles");
};

export const updateAccountTaxProfile = async (
  profile: AccountTaxProfileUpdate,
): Promise<AccountTaxProfile> => {
  return invoke<AccountTaxProfile>("update_account_tax_profile", { profile });
};

export const listTaxYearReports = async (): Promise<TaxYearReport[]> => {
  return invoke<TaxYearReport[]>("list_tax_year_reports");
};

export const getTaxYearReport = async (id: string): Promise<TaxYearReport | null> => {
  return invoke<TaxYearReport | null>("get_tax_year_report", { id });
};

export const createTaxYearReport = async (report: NewTaxYearReport): Promise<TaxYearReport> => {
  return invoke<TaxYearReport>("create_tax_year_report", { report });
};

export const getTaxReportDetail = async (id: string): Promise<TaxReportDetail | null> => {
  return invoke<TaxReportDetail | null>("get_tax_report_detail", { id });
};

export const regenerateTaxYearReport = async (id: string): Promise<TaxReportDetail> => {
  return invoke<TaxReportDetail>("regenerate_tax_year_report", { id });
};

export const finalizeTaxYearReport = async (id: string): Promise<TaxYearReport> => {
  return invoke<TaxYearReport>("finalize_tax_year_report", { id });
};

export const amendTaxYearReport = async (id: string): Promise<TaxYearReport> => {
  return invoke<TaxYearReport>("amend_tax_year_report", { id });
};

export const uploadTaxDocument = async (upload: TaxDocumentUpload): Promise<TaxDocument> => {
  return invoke<TaxDocument>("upload_tax_document", { upload });
};

export const listTaxDocuments = async (reportId: string): Promise<TaxDocument[]> => {
  return invoke<TaxDocument[]>("list_tax_documents", { reportId });
};

export const deleteTaxDocument = async (documentId: string): Promise<void> => {
  await invoke<void>("delete_tax_document", { documentId });
};

interface TaxDocumentDownloadPayload {
  filename: string;
  mimeType?: string | null;
  content?: number[] | Uint8Array;
}

export const downloadTaxDocument = async (documentId: string): Promise<TaxDocumentDownload> => {
  const payload = await invoke<TaxDocumentDownloadPayload>("get_tax_document_download", {
    documentId,
  });
  const rawContent = payload.content ?? [];
  const content = rawContent instanceof Uint8Array ? rawContent : Uint8Array.from(rawContent);
  return {
    filename: payload.filename,
    mimeType: payload.mimeType ?? null,
    content,
  };
};

export const extractTaxDocument = async (
  request: TaxDocumentExtractionRequest,
): Promise<TaxDocumentExtractionResult> => {
  return invoke<TaxDocumentExtractionResult>("extract_tax_document", { request });
};

export const updateExtractedTaxField = async (
  update: ExtractedTaxFieldUpdate,
): Promise<ExtractedTaxField> => {
  return invoke<ExtractedTaxField>("update_extracted_tax_field", { update });
};

export const reconcileTaxYearReport = async (id: string): Promise<TaxReconciliationEntry[]> => {
  return invoke<TaxReconciliationEntry[]>("reconcile_tax_year_report", { id });
};

export const updateTaxReconciliationEntry = async (
  update: TaxReconciliationEntryUpdate,
): Promise<TaxReconciliationEntry> => {
  return invoke<TaxReconciliationEntry>("update_tax_reconciliation_entry", { update });
};

export const updateTaxEvent = async (update: TaxEventUpdate): Promise<TaxEvent> => {
  return invoke<TaxEvent>("update_tax_event", { update });
};
