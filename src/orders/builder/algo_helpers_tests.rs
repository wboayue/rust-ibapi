use super::*;
use crate::orders::builder::algo_builders::AlgoParams;

#[test]
fn test_vwap_helper() {
    let algo: AlgoParams = vwap().max_pct_vol(0.2).build().unwrap();
    assert_eq!(algo.strategy, "Vwap");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_twap_helper() {
    let algo: AlgoParams = twap().start_time("09:00:00 US/Eastern").build().unwrap();
    assert_eq!(algo.strategy, "Twap");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_pct_vol_helper() {
    let algo: AlgoParams = pct_vol().pct_vol(0.15).build().unwrap();
    assert_eq!(algo.strategy, "PctVol");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_arrival_price_helper() {
    let algo: AlgoParams = arrival_price().max_pct_vol(0.1).build().unwrap();
    assert_eq!(algo.strategy, "ArrivalPx");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_adaptive_helper() {
    use crate::orders::builder::AdaptivePriority;
    let algo: AlgoParams = adaptive().priority(AdaptivePriority::Urgent).build().unwrap();
    assert_eq!(algo.strategy, "Adaptive");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_close_price_helper() {
    let algo: AlgoParams = close_price().max_pct_vol(0.2).build().unwrap();
    assert_eq!(algo.strategy, "ClosePx");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_dark_ice_helper() {
    let algo: AlgoParams = dark_ice().display_size(100).build().unwrap();
    assert_eq!(algo.strategy, "DarkIce");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_accumulate_distribute_helper() {
    let algo: AlgoParams = accumulate_distribute().component_size(100).build().unwrap();
    assert_eq!(algo.strategy, "AD");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_balance_impact_risk_helper() {
    let algo: AlgoParams = balance_impact_risk().max_pct_vol(0.2).build().unwrap();
    assert_eq!(algo.strategy, "BalanceImpactRisk");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_minimise_impact_helper() {
    let algo: AlgoParams = minimise_impact().max_pct_vol(0.2).build().unwrap();
    assert_eq!(algo.strategy, "MinImpact");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_pct_vol_price_helper() {
    let algo: AlgoParams = pct_vol_price().pct_vol(0.15).build().unwrap();
    assert_eq!(algo.strategy, "PctVolPx");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_pct_vol_size_helper() {
    let algo: AlgoParams = pct_vol_size().start_pct_vol(0.1).build().unwrap();
    assert_eq!(algo.strategy, "PctVolSz");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_pct_vol_time_helper() {
    let algo: AlgoParams = pct_vol_time().start_pct_vol(0.1).build().unwrap();
    assert_eq!(algo.strategy, "PctVolTm");
    assert_eq!(algo.params.len(), 1);
}

#[test]
fn test_accu_distr_helper() {
    let algo: AlgoParams = accu_distr().component_size(100).build().unwrap();
    assert_eq!(algo.strategy, "AccuDistr");
    assert_eq!(algo.params.len(), 1);
}
