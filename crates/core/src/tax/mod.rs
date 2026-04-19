//! Tax declaration assistant domain models, services, and traits.

mod tax_model;
mod tax_service;
mod tax_traits;

pub use tax_model::{
    AccountTaxProfile, AccountTaxProfileUpdate, NewTaxYearReport, TaxProfile, TaxProfileUpdate,
    TaxReportStatus, TaxYearReport, DEFAULT_TAX_JURISDICTION, DEFAULT_TAX_REGIME,
};
pub use tax_service::TaxService;
pub use tax_traits::{TaxRepositoryTrait, TaxServiceTrait};
