use serde::Deserialize;

/// A single barème tranche. `max = 0` means unbounded (top tranche).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BaremeTranche {
    pub min: u64,
    /// 0 means no upper bound (top tranche)
    pub max: u64,
    pub rate: f64,
}

impl BaremeTranche {
    pub fn is_top(&self) -> bool {
        self.max == 0
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Bareme {
    pub tranches: Vec<BaremeTranche>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PfuParams {
    pub ir_rate: f64,
    pub ps_rate: f64,
    pub total_rate: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PrelevementsSociauxParams {
    pub total_rate: f64,
    pub csg_deductible_rate: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct QuotientFamilialParams {
    pub plafond_demi_part: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DividendesParams {
    pub abattement_rate: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FraisProfessionnelsParams {
    pub abattement_rate: f64,
    pub abattement_min: f64,
    pub abattement_max: f64,
}

/// All French tax parameters for a given year, loaded from a versioned TOML file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TaxParameters {
    /// Identifies which parameter set was used (e.g. "FR-2025-v1").
    pub version: String,
    pub tax_year: i32,
    pub jurisdiction: String,
    pub pfu: PfuParams,
    pub prelevements_sociaux: PrelevementsSociauxParams,
    pub bareme: Bareme,
    pub quotient_familial: QuotientFamilialParams,
    pub dividendes: DividendesParams,
    pub frais_professionnels: FraisProfessionnelsParams,
}

impl TaxParameters {
    /// Load parameters for a given year. Returns `None` if no parameters are
    /// bundled for that year.
    pub fn for_year(year: i32) -> Option<Self> {
        let raw = match year {
            2025 => include_str!("tax_params_2025.toml"),
            _ => return None,
        };
        // Panic on malformed built-in TOML — this is a programming error, not
        // a runtime condition.
        Some(toml::from_str(raw).expect("Built-in tax params TOML is malformed"))
    }

    /// Load parameters for a given year, falling back to the most recent
    /// available year when no exact match exists.
    pub fn for_year_or_latest(year: i32) -> Self {
        Self::for_year(year).unwrap_or_else(|| {
            Self::for_year(2025).expect("Baseline tax params (2025) must always be present")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_2025_params() {
        let params = TaxParameters::for_year(2025).expect("2025 params should load");
        assert_eq!(params.tax_year, 2025);
        assert_eq!(params.jurisdiction, "FR");
        assert_eq!(params.version, "FR-2025-v1");
    }

    #[test]
    fn test_pfu_rates_2025() {
        let params = TaxParameters::for_year(2025).unwrap();
        assert!((params.pfu.ir_rate - 0.128).abs() < f64::EPSILON);
        assert!((params.pfu.ps_rate - 0.172).abs() < f64::EPSILON);
        assert!((params.pfu.total_rate - 0.300).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bareme_tranches_2025() {
        let params = TaxParameters::for_year(2025).unwrap();
        let tranches = &params.bareme.tranches;
        assert_eq!(tranches.len(), 5);
        // First tranche: 0%
        assert_eq!(tranches[0].min, 0);
        assert!((tranches[0].rate - 0.0).abs() < f64::EPSILON);
        // Top tranche: 45%, unbounded
        let top = tranches.last().unwrap();
        assert!(top.is_top());
        assert!((top.rate - 0.45).abs() < f64::EPSILON);
    }

    #[test]
    fn test_unknown_year_falls_back_to_latest() {
        let params = TaxParameters::for_year_or_latest(2030);
        assert_eq!(params.tax_year, 2025);
    }

    #[test]
    fn test_unknown_year_returns_none() {
        assert!(TaxParameters::for_year(2030).is_none());
    }

    #[test]
    fn test_csg_deductible_rate() {
        let params = TaxParameters::for_year(2025).unwrap();
        assert!((params.prelevements_sociaux.csg_deductible_rate - 0.068).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dividendes_abattement() {
        let params = TaxParameters::for_year(2025).unwrap();
        assert!((params.dividendes.abattement_rate - 0.40).abs() < f64::EPSILON);
    }

    #[test]
    fn test_quotient_familial_plafond() {
        let params = TaxParameters::for_year(2025).unwrap();
        assert!((params.quotient_familial.plafond_demi_part - 1771.0).abs() < f64::EPSILON);
    }
}
