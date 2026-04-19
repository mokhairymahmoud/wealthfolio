use rust_decimal::Decimal;
use std::collections::HashMap;

use super::fee_model::{FeeAnalysis, FeeProjection, FeeSeverity, HoldingFee};
use crate::portfolio::holdings::Holding;

const WARNING_THRESHOLD: f64 = 0.005; // 0.5%
const HIGH_THRESHOLD: f64 = 0.01; // 1.0%
const DEFAULT_ANNUAL_RETURN: f64 = 0.07; // 7%
const PROJECTION_HORIZONS: [u32; 3] = [10, 20, 30];

pub fn classify_severity(expense_ratio: Option<f64>) -> FeeSeverity {
    match expense_ratio {
        Some(er) if er >= HIGH_THRESHOLD => FeeSeverity::High,
        Some(er) if er >= WARNING_THRESHOLD => FeeSeverity::Warning,
        _ => FeeSeverity::None,
    }
}

pub fn analyze_fees(
    holdings: &[Holding],
    expense_ratios: &HashMap<String, f64>,
    base_currency: &str,
    account_names: &HashMap<String, String>,
) -> FeeAnalysis {
    let mut holding_fees = Vec::new();
    let mut total_annual_fee = Decimal::ZERO;
    let mut total_mv_with_er = Decimal::ZERO;
    let mut weighted_er_sum = 0.0_f64;
    let mut total_market_value = Decimal::ZERO;

    for h in holdings {
        let er = expense_ratios.get(&h.id).copied();
        let mv_base = h.market_value.base;
        total_market_value += mv_base;

        let annual_fee = er.map(|r| {
            let fee = (mv_base * Decimal::from_f64_retain(r).unwrap_or(Decimal::ZERO)).round_dp(2);
            total_annual_fee += fee;
            total_mv_with_er += mv_base;
            weighted_er_sum += r * decimal_to_f64(mv_base);
            fee
        });

        let symbol = h
            .instrument
            .as_ref()
            .map(|i| i.symbol.clone())
            .unwrap_or_default();
        let name = h
            .instrument
            .as_ref()
            .and_then(|i| i.name.clone())
            .unwrap_or_default();
        let account_name = account_names.get(&h.account_id).cloned();

        holding_fees.push(HoldingFee {
            asset_id: h.id.clone(),
            symbol,
            name,
            account_id: Some(h.account_id.clone()),
            account_name,
            market_value: h.market_value.local,
            market_value_base: mv_base,
            currency: h.local_currency.clone(),
            expense_ratio: er,
            annual_fee,
            severity: classify_severity(er),
        });
    }

    holding_fees.sort_by(|a, b| {
        b.annual_fee
            .unwrap_or(Decimal::ZERO)
            .cmp(&a.annual_fee.unwrap_or(Decimal::ZERO))
    });

    let weighted_avg = if total_mv_with_er > Decimal::ZERO {
        let total_mv_f64 = decimal_to_f64(total_mv_with_er);
        if total_mv_f64 > 0.0 {
            Some(weighted_er_sum / total_mv_f64)
        } else {
            None
        }
    } else {
        None
    };

    let fee_pct = if total_market_value > Decimal::ZERO {
        Some(decimal_to_f64(total_annual_fee) / decimal_to_f64(total_market_value))
    } else {
        None
    };

    let projections = compute_projections(total_market_value, weighted_avg);

    FeeAnalysis {
        holdings: holding_fees,
        total_annual_fee,
        weighted_avg_expense_ratio: weighted_avg,
        fee_pct_of_portfolio: fee_pct,
        total_market_value,
        projections,
        currency: base_currency.to_string(),
    }
}

fn compute_projections(portfolio_value: Decimal, weighted_er: Option<f64>) -> Vec<FeeProjection> {
    let er = match weighted_er {
        Some(er) if er > 0.0 => er,
        _ => {
            return PROJECTION_HORIZONS
                .iter()
                .map(|&y| FeeProjection {
                    years: y,
                    cumulative_fee_drag: Decimal::ZERO,
                })
                .collect()
        }
    };

    let pv = decimal_to_f64(portfolio_value);
    let r = DEFAULT_ANNUAL_RETURN;

    PROJECTION_HORIZONS
        .iter()
        .map(|&years| {
            let n = years as f64;
            // Value with no fees - value with fees = cumulative drag
            let value_no_fees = pv * (1.0 + r).powf(n);
            let value_with_fees = pv * (1.0 + r - er).powf(n);
            let drag = value_no_fees - value_with_fees;
            FeeProjection {
                years,
                cumulative_fee_drag: Decimal::from_f64_retain(drag)
                    .unwrap_or(Decimal::ZERO)
                    .round_dp(2),
            }
        })
        .collect()
}

fn decimal_to_f64(d: Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

#[cfg(test)]
mod tests;
