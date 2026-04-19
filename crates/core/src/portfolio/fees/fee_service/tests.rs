use super::*;
use crate::portfolio::holdings::{Holding, HoldingType, Instrument, MonetaryValue};
use chrono::{NaiveDate, Utc};
use rust_decimal_macros::dec;
use std::collections::{HashMap, VecDeque};

fn make_holding(id: &str, account_id: &str, mv_base: Decimal) -> Holding {
    Holding {
        id: id.to_string(),
        account_id: account_id.to_string(),
        holding_type: HoldingType::Security,
        instrument: Some(Instrument {
            id: id.to_string(),
            symbol: id.to_string(),
            name: Some(format!("{} Fund", id)),
            currency: "USD".to_string(),
            notes: None,
            pricing_mode: "MARKET".to_string(),
            preferred_provider: None,
            classifications: None,
        }),
        asset_kind: None,
        quantity: dec!(10),
        open_date: None,
        lots: Some(VecDeque::new()),
        contract_multiplier: dec!(1),
        local_currency: "USD".to_string(),
        base_currency: "USD".to_string(),
        fx_rate: Some(dec!(1)),
        market_value: MonetaryValue {
            local: mv_base,
            base: mv_base,
        },
        cost_basis: None,
        price: None,
        purchase_price: None,
        unrealized_gain: None,
        unrealized_gain_pct: None,
        realized_gain: None,
        realized_gain_pct: None,
        total_gain: None,
        total_gain_pct: None,
        day_change: None,
        day_change_pct: None,
        prev_close_value: None,
        weight: dec!(0.5),
        as_of_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        metadata: None,
    }
}

fn empty_accounts() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("acc1".to_string(), "Account 1".to_string());
    m
}

#[test]
fn test_single_holding_with_expense_ratio() {
    let holdings = vec![make_holding("VTI", "acc1", dec!(100_000))];
    let mut ers = HashMap::new();
    ers.insert("VTI".to_string(), 0.003); // 0.3%

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    assert_eq!(result.holdings.len(), 1);
    assert_eq!(result.holdings[0].annual_fee, Some(dec!(300)));
    assert_eq!(result.total_annual_fee, dec!(300));
    assert_eq!(result.holdings[0].severity, FeeSeverity::None);
}

#[test]
fn test_multiple_holdings_weighted_avg() {
    let holdings = vec![
        make_holding("VTI", "acc1", dec!(80_000)),
        make_holding("ARKK", "acc1", dec!(20_000)),
    ];
    let mut ers = HashMap::new();
    ers.insert("VTI".to_string(), 0.003); // 0.3%
    ers.insert("ARKK".to_string(), 0.0075); // 0.75%

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    // Total annual: 80000 * 0.003 + 20000 * 0.0075 = 240 + 150 = 390
    assert_eq!(result.total_annual_fee, dec!(390));
    // Weighted avg: (0.003 * 80000 + 0.0075 * 20000) / 100000 = 0.0039
    let wavg = result.weighted_avg_expense_ratio.unwrap();
    assert!((wavg - 0.0039).abs() < 0.0001);
}

#[test]
fn test_missing_expense_ratio_excluded_from_totals() {
    let holdings = vec![
        make_holding("VTI", "acc1", dec!(50_000)),
        make_holding("AAPL", "acc1", dec!(50_000)),
    ];
    let mut ers = HashMap::new();
    ers.insert("VTI".to_string(), 0.003);
    // AAPL has no expense ratio

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    assert_eq!(result.total_annual_fee, dec!(150));
    assert_eq!(result.holdings[1].annual_fee, None);
    assert_eq!(result.total_market_value, dec!(100_000));
    // Weighted avg only considers VTI
    let wavg = result.weighted_avg_expense_ratio.unwrap();
    assert!((wavg - 0.003).abs() < 0.0001);
}

#[test]
fn test_severity_classification() {
    assert_eq!(classify_severity(None), FeeSeverity::None);
    assert_eq!(classify_severity(Some(0.001)), FeeSeverity::None);
    assert_eq!(classify_severity(Some(0.004)), FeeSeverity::None);
    assert_eq!(classify_severity(Some(0.005)), FeeSeverity::Warning);
    assert_eq!(classify_severity(Some(0.008)), FeeSeverity::Warning);
    assert_eq!(classify_severity(Some(0.01)), FeeSeverity::High);
    assert_eq!(classify_severity(Some(0.02)), FeeSeverity::High);
}

#[test]
fn test_severity_at_boundaries() {
    assert_eq!(classify_severity(Some(0.0049)), FeeSeverity::None);
    assert_eq!(classify_severity(Some(0.005)), FeeSeverity::Warning);
    assert_eq!(classify_severity(Some(0.0099)), FeeSeverity::Warning);
    assert_eq!(classify_severity(Some(0.01)), FeeSeverity::High);
}

#[test]
fn test_empty_holdings() {
    let result = analyze_fees(&[], &HashMap::new(), "USD", &HashMap::new());

    assert!(result.holdings.is_empty());
    assert_eq!(result.total_annual_fee, Decimal::ZERO);
    assert_eq!(result.weighted_avg_expense_ratio, None);
    assert_eq!(result.fee_pct_of_portfolio, None);
    assert_eq!(result.total_market_value, Decimal::ZERO);
}

#[test]
fn test_all_holdings_missing_expense_ratio() {
    let holdings = vec![
        make_holding("AAPL", "acc1", dec!(50_000)),
        make_holding("GOOGL", "acc1", dec!(50_000)),
    ];

    let result = analyze_fees(&holdings, &HashMap::new(), "USD", &empty_accounts());

    assert_eq!(result.total_annual_fee, Decimal::ZERO);
    assert_eq!(result.weighted_avg_expense_ratio, None);
    assert_eq!(result.total_market_value, dec!(100_000));
    assert!(result.holdings.iter().all(|h| h.annual_fee.is_none()));
}

#[test]
fn test_projections_with_fees() {
    let holdings = vec![make_holding("VTI", "acc1", dec!(100_000))];
    let mut ers = HashMap::new();
    ers.insert("VTI".to_string(), 0.01); // 1%

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    assert_eq!(result.projections.len(), 3);
    assert_eq!(result.projections[0].years, 10);
    assert_eq!(result.projections[1].years, 20);
    assert_eq!(result.projections[2].years, 30);
    // All projections should be positive
    assert!(result
        .projections
        .iter()
        .all(|p| p.cumulative_fee_drag > Decimal::ZERO));
    // Longer horizons = more drag
    assert!(result.projections[1].cumulative_fee_drag > result.projections[0].cumulative_fee_drag);
    assert!(result.projections[2].cumulative_fee_drag > result.projections[1].cumulative_fee_drag);
}

#[test]
fn test_projections_without_fees() {
    let result = analyze_fees(&[], &HashMap::new(), "USD", &HashMap::new());

    assert_eq!(result.projections.len(), 3);
    assert!(result
        .projections
        .iter()
        .all(|p| p.cumulative_fee_drag == Decimal::ZERO));
}

#[test]
fn test_sorted_by_annual_fee_descending() {
    let holdings = vec![
        make_holding("CHEAP", "acc1", dec!(100_000)),
        make_holding("EXPENSIVE", "acc1", dec!(100_000)),
    ];
    let mut ers = HashMap::new();
    ers.insert("CHEAP".to_string(), 0.001);
    ers.insert("EXPENSIVE".to_string(), 0.02);

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    assert_eq!(result.holdings[0].symbol, "EXPENSIVE");
    assert_eq!(result.holdings[1].symbol, "CHEAP");
}

#[test]
fn test_fee_pct_of_portfolio() {
    let holdings = vec![make_holding("VTI", "acc1", dec!(100_000))];
    let mut ers = HashMap::new();
    ers.insert("VTI".to_string(), 0.005); // 0.5%

    let result = analyze_fees(&holdings, &ers, "USD", &empty_accounts());

    let pct = result.fee_pct_of_portfolio.unwrap();
    assert!((pct - 0.005).abs() < 0.0001);
}
