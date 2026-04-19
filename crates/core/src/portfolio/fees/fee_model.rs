use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeeSeverity {
    None,
    Warning,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingFee {
    pub asset_id: String,
    pub symbol: String,
    pub name: String,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub market_value: Decimal,
    pub market_value_base: Decimal,
    pub currency: String,
    pub expense_ratio: Option<f64>,
    pub annual_fee: Option<Decimal>,
    pub severity: FeeSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeProjection {
    pub years: u32,
    pub cumulative_fee_drag: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeAnalysis {
    pub holdings: Vec<HoldingFee>,
    pub total_annual_fee: Decimal,
    pub weighted_avg_expense_ratio: Option<f64>,
    pub fee_pct_of_portfolio: Option<f64>,
    pub total_market_value: Decimal,
    pub projections: Vec<FeeProjection>,
    pub currency: String,
}
