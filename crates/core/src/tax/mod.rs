//! Tax declaration assistant domain models, services, and traits.

mod tax_model;
mod tax_params;
mod tax_service;
mod tax_traits;

pub use tax_model::*;
pub use tax_params::TaxParameters;
pub use tax_service::TaxService;
pub use tax_traits::{TaxCloudExtractionTrait, TaxRepositoryTrait, TaxServiceTrait};
