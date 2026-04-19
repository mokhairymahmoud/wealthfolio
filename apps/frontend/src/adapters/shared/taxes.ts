import type {
  AccountTaxProfile,
  AccountTaxProfileUpdate,
  NewTaxYearReport,
  TaxProfile,
  TaxProfileUpdate,
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
