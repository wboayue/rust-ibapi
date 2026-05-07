use super::*;

#[test]
fn test_algo_params_from_string() {
    let params: AlgoParams = "Vwap".into();
    assert_eq!(params.strategy, "Vwap");
    assert!(params.params.is_empty());
}

#[test]
fn test_vwap_builder() {
    let params = VwapBuilder::new()
        .max_pct_vol(0.2)
        .start_time("09:00:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .allow_past_end_time(true)
        .no_take_liq(true)
        .speed_up(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "Vwap");
    assert_eq!(params.params.len(), 6);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.2".to_string()));
    assert_eq!(find_param("startTime"), Some(&"09:00:00 US/Eastern".to_string()));
    assert_eq!(find_param("endTime"), Some(&"16:00:00 US/Eastern".to_string()));
    assert_eq!(find_param("allowPastEndTime"), Some(&"1".to_string()));
    assert_eq!(find_param("noTakeLiq"), Some(&"1".to_string()));
    assert_eq!(find_param("speedUp"), Some(&"1".to_string()));
}

#[test]
fn test_twap_builder() {
    let params = TwapBuilder::new()
        .strategy_type(TwapStrategyType::MatchingMidpoint)
        .start_time("09:00:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .allow_past_end_time(false)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "Twap");
    assert_eq!(params.params.len(), 4);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("strategyType"), Some(&"Matching Midpoint".to_string()));
    assert_eq!(find_param("allowPastEndTime"), Some(&"0".to_string()));
}

#[test]
fn test_pct_vol_builder() {
    let params = PctVolBuilder::new()
        .pct_vol(0.15)
        .start_time("09:30:00 US/Eastern")
        .end_time("15:30:00 US/Eastern")
        .no_take_liq(false)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "PctVol");
    assert_eq!(params.params.len(), 4);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("pctVol"), Some(&"0.15".to_string()));
    assert_eq!(find_param("noTakeLiq"), Some(&"0".to_string()));
}

#[test]
fn test_arrival_price_builder() {
    let params = ArrivalPriceBuilder::new()
        .max_pct_vol(0.1)
        .risk_aversion(RiskAversion::Aggressive)
        .start_time("09:00:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .force_completion(true)
        .allow_past_end_time(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "ArrivalPx");
    assert_eq!(params.params.len(), 6);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("riskAversion"), Some(&"Aggressive".to_string()));
    assert_eq!(find_param("forceCompletion"), Some(&"1".to_string()));
}

#[test]
fn test_builder_minimal() {
    // Test that builders work with no params set
    let vwap = VwapBuilder::new().build().unwrap();
    assert_eq!(vwap.strategy, "Vwap");
    assert!(vwap.params.is_empty());

    let twap = TwapBuilder::new().build().unwrap();
    assert_eq!(twap.strategy, "Twap");
    assert!(twap.params.is_empty());
}

#[test]
fn test_pct_vol_out_of_range_errors() {
    // Values above 0.5 should return error
    let result = PctVolBuilder::new().pct_vol(0.8).build();
    assert!(matches!(result, Err(ValidationError::InvalidPercentage { field: "pct_vol", .. })));

    let result = VwapBuilder::new().max_pct_vol(1.0).build();
    assert!(matches!(result, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));

    // Values below 0.1 should return error
    let result = PctVolBuilder::new().pct_vol(0.05).build();
    assert!(matches!(result, Err(ValidationError::InvalidPercentage { field: "pct_vol", .. })));

    let result = ArrivalPriceBuilder::new().max_pct_vol(0.01).build();
    assert!(matches!(result, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
}

#[test]
fn test_pct_vol_valid_values_succeed() {
    // Values within 0.1-0.5 should pass through unchanged
    let params = PctVolBuilder::new().pct_vol(0.25).build().unwrap();
    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("pctVol"), Some(&"0.25".to_string()));

    let params = VwapBuilder::new().max_pct_vol(0.1).build().unwrap();
    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.1".to_string()));

    let params = VwapBuilder::new().max_pct_vol(0.5).build().unwrap();
    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.5".to_string()));
}

#[test]
fn test_pct_vol_boundary_values() {
    // Exactly 0.1 should succeed
    assert!(VwapBuilder::new().max_pct_vol(0.1).build().is_ok());
    assert!(PctVolBuilder::new().pct_vol(0.1).build().is_ok());
    assert!(ArrivalPriceBuilder::new().max_pct_vol(0.1).build().is_ok());

    // Exactly 0.5 should succeed
    assert!(VwapBuilder::new().max_pct_vol(0.5).build().is_ok());
    assert!(PctVolBuilder::new().pct_vol(0.5).build().is_ok());
    assert!(ArrivalPriceBuilder::new().max_pct_vol(0.5).build().is_ok());

    // Just outside boundaries should fail
    assert!(VwapBuilder::new().max_pct_vol(0.09).build().is_err());
    assert!(VwapBuilder::new().max_pct_vol(0.51).build().is_err());
}

#[test]
fn test_adaptive_builder() {
    let params = AdaptiveBuilder::new().priority(AdaptivePriority::Urgent).build().unwrap();

    assert_eq!(params.strategy, "Adaptive");
    assert_eq!(params.params.len(), 1);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("adaptivePriority"), Some(&"Urgent".to_string()));
}

#[test]
fn test_adaptive_builder_priority_variants() {
    let urgent = AdaptiveBuilder::new().priority(AdaptivePriority::Urgent).build().unwrap();
    assert_eq!(urgent.params[0].value, "Urgent");

    let normal = AdaptiveBuilder::new().priority(AdaptivePriority::Normal).build().unwrap();
    assert_eq!(normal.params[0].value, "Normal");

    let patient = AdaptiveBuilder::new().priority(AdaptivePriority::Patient).build().unwrap();
    assert_eq!(patient.params[0].value, "Patient");
}

#[test]
fn test_adaptive_builder_minimal() {
    let params = AdaptiveBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "Adaptive");
    assert!(params.params.is_empty());
}

#[test]
fn test_close_price_builder() {
    let params = ClosePriceBuilder::new()
        .max_pct_vol(0.3)
        .risk_aversion(RiskAversion::Aggressive)
        .start_time("15:30:00 US/Eastern")
        .force_completion(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "ClosePx");
    assert_eq!(params.params.len(), 4);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.3".to_string()));
    assert_eq!(find_param("riskAversion"), Some(&"Aggressive".to_string()));
    assert_eq!(find_param("startTime"), Some(&"15:30:00 US/Eastern".to_string()));
    assert_eq!(find_param("forceCompletion"), Some(&"1".to_string()));
}

#[test]
fn test_close_price_builder_minimal() {
    let params = ClosePriceBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "ClosePx");
    assert!(params.params.is_empty());
}

#[test]
fn test_close_price_pct_vol_boundary_values() {
    // Exactly 0.1 and 0.5 should succeed
    assert!(ClosePriceBuilder::new().max_pct_vol(0.1).build().is_ok());
    assert!(ClosePriceBuilder::new().max_pct_vol(0.5).build().is_ok());

    // Outside the range should fail
    let err = ClosePriceBuilder::new().max_pct_vol(0.09).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
    let err = ClosePriceBuilder::new().max_pct_vol(0.51).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
}

#[test]
fn test_dark_ice_builder() {
    let params = DarkIceBuilder::new()
        .display_size(100)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .allow_past_end_time(false)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "DarkIce");
    assert_eq!(params.params.len(), 4);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("displaySize"), Some(&"100".to_string()));
    assert_eq!(find_param("startTime"), Some(&"09:30:00 US/Eastern".to_string()));
    assert_eq!(find_param("endTime"), Some(&"16:00:00 US/Eastern".to_string()));
    assert_eq!(find_param("allowPastEndTime"), Some(&"0".to_string()));
}

#[test]
fn test_dark_ice_builder_minimal() {
    let params = DarkIceBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "DarkIce");
    assert!(params.params.is_empty());
}

#[test]
fn test_accumulate_distribute_builder() {
    let params = AccumulateDistributeBuilder::new()
        .component_size(100)
        .time_between_orders(60)
        .randomize_time_20(true)
        .randomize_size_55(false)
        .give_up(0)
        .catch_up(true)
        .wait_for_fill(false)
        .active_time_start("20260101-09:30:00 US/Eastern")
        .active_time_end("20260101-16:00:00 US/Eastern")
        .build()
        .unwrap();

    assert_eq!(params.strategy, "AD");
    assert_eq!(params.params.len(), 9);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("componentSize"), Some(&"100".to_string()));
    assert_eq!(find_param("timeBetweenOrders"), Some(&"60".to_string()));
    assert_eq!(find_param("randomizeTime20"), Some(&"1".to_string()));
    assert_eq!(find_param("randomizeSize55"), Some(&"0".to_string()));
    assert_eq!(find_param("giveUp"), Some(&"0".to_string()));
    assert_eq!(find_param("catchUp"), Some(&"1".to_string()));
    assert_eq!(find_param("waitForFill"), Some(&"0".to_string()));
    assert_eq!(find_param("activeTimeStart"), Some(&"20260101-09:30:00 US/Eastern".to_string()));
    assert_eq!(find_param("activeTimeEnd"), Some(&"20260101-16:00:00 US/Eastern".to_string()));
}

#[test]
fn test_accumulate_distribute_builder_minimal() {
    let params = AccumulateDistributeBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "AD");
    assert!(params.params.is_empty());
}

#[test]
fn test_balance_impact_risk_builder() {
    let params = BalanceImpactRiskBuilder::new()
        .max_pct_vol(0.25)
        .risk_aversion(RiskAversion::Passive)
        .force_completion(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "BalanceImpactRisk");
    assert_eq!(params.params.len(), 3);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.25".to_string()));
    assert_eq!(find_param("riskAversion"), Some(&"Passive".to_string()));
    assert_eq!(find_param("forceCompletion"), Some(&"1".to_string()));
}

#[test]
fn test_balance_impact_risk_builder_minimal() {
    let params = BalanceImpactRiskBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "BalanceImpactRisk");
    assert!(params.params.is_empty());
}

#[test]
fn test_balance_impact_risk_pct_vol_boundary_values() {
    assert!(BalanceImpactRiskBuilder::new().max_pct_vol(0.1).build().is_ok());
    assert!(BalanceImpactRiskBuilder::new().max_pct_vol(0.5).build().is_ok());

    let err = BalanceImpactRiskBuilder::new().max_pct_vol(0.09).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
    let err = BalanceImpactRiskBuilder::new().max_pct_vol(0.51).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
}

#[test]
fn test_minimise_impact_builder() {
    let params = MinimiseImpactBuilder::new().max_pct_vol(0.3).build().unwrap();

    assert_eq!(params.strategy, "MinImpact");
    assert_eq!(params.params.len(), 1);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("maxPctVol"), Some(&"0.3".to_string()));
}

#[test]
fn test_minimise_impact_builder_minimal() {
    let params = MinimiseImpactBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "MinImpact");
    assert!(params.params.is_empty());
}

#[test]
fn test_minimise_impact_pct_vol_boundary_values() {
    assert!(MinimiseImpactBuilder::new().max_pct_vol(0.1).build().is_ok());
    assert!(MinimiseImpactBuilder::new().max_pct_vol(0.5).build().is_ok());

    let err = MinimiseImpactBuilder::new().max_pct_vol(0.09).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
    let err = MinimiseImpactBuilder::new().max_pct_vol(0.51).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "max_pct_vol", .. })));
}

#[test]
fn test_pct_vol_price_builder() {
    let params = PctVolPriceBuilder::new()
        .pct_vol(0.2)
        .delta_pct_vol(-0.05)
        .min_pct_vol_4_px(0.05)
        .max_pct_vol_4_px(0.8)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .no_take_liq(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "PctVolPx");
    assert_eq!(params.params.len(), 7);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("pctVol"), Some(&"0.2".to_string()));
    assert_eq!(find_param("deltaPctVol"), Some(&"-0.05".to_string()));
    assert_eq!(find_param("minPctVol4Px"), Some(&"0.05".to_string()));
    assert_eq!(find_param("maxPctVol4Px"), Some(&"0.8".to_string()));
    assert_eq!(find_param("startTime"), Some(&"09:30:00 US/Eastern".to_string()));
    assert_eq!(find_param("endTime"), Some(&"16:00:00 US/Eastern".to_string()));
    assert_eq!(find_param("noTakeLiq"), Some(&"1".to_string()));
}

#[test]
fn test_pct_vol_price_builder_minimal() {
    let params = PctVolPriceBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "PctVolPx");
    assert!(params.params.is_empty());
}

#[test]
fn test_pct_vol_price_only_pct_vol_validated() {
    // pct_vol enforces 10-50%
    assert!(PctVolPriceBuilder::new().pct_vol(0.1).build().is_ok());
    assert!(PctVolPriceBuilder::new().pct_vol(0.5).build().is_ok());
    let err = PctVolPriceBuilder::new().pct_vol(0.09).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "pct_vol", .. })));

    // delta_pct_vol can be negative; min/max bounds 0-1 — none are validated
    assert!(PctVolPriceBuilder::new().delta_pct_vol(-0.5).build().is_ok());
    assert!(PctVolPriceBuilder::new().min_pct_vol_4_px(0.0).build().is_ok());
    assert!(PctVolPriceBuilder::new().max_pct_vol_4_px(1.0).build().is_ok());
}

#[test]
fn test_pct_vol_size_builder() {
    let params = PctVolSizeBuilder::new()
        .start_pct_vol(0.1)
        .end_pct_vol(0.4)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .no_take_liq(false)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "PctVolSz");
    assert_eq!(params.params.len(), 5);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("startPctVol"), Some(&"0.1".to_string()));
    assert_eq!(find_param("endPctVol"), Some(&"0.4".to_string()));
    assert_eq!(find_param("startTime"), Some(&"09:30:00 US/Eastern".to_string()));
    assert_eq!(find_param("endTime"), Some(&"16:00:00 US/Eastern".to_string()));
    assert_eq!(find_param("noTakeLiq"), Some(&"0".to_string()));
}

#[test]
fn test_pct_vol_size_builder_minimal() {
    let params = PctVolSizeBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "PctVolSz");
    assert!(params.params.is_empty());
}

#[test]
fn test_pct_vol_size_validates_both_rates() {
    assert!(PctVolSizeBuilder::new().start_pct_vol(0.1).end_pct_vol(0.5).build().is_ok());

    let err = PctVolSizeBuilder::new().start_pct_vol(0.05).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "start_pct_vol", .. })));

    let err = PctVolSizeBuilder::new().end_pct_vol(0.6).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "end_pct_vol", .. })));
}

#[test]
fn test_pct_vol_time_builder() {
    let params = PctVolTimeBuilder::new()
        .start_pct_vol(0.15)
        .end_pct_vol(0.35)
        .start_time("09:30:00 US/Eastern")
        .end_time("16:00:00 US/Eastern")
        .no_take_liq(true)
        .build()
        .unwrap();

    assert_eq!(params.strategy, "PctVolTm");
    assert_eq!(params.params.len(), 5);

    let find_param = |tag: &str| params.params.iter().find(|p| p.tag == tag).map(|p| &p.value);
    assert_eq!(find_param("startPctVol"), Some(&"0.15".to_string()));
    assert_eq!(find_param("endPctVol"), Some(&"0.35".to_string()));
    assert_eq!(find_param("noTakeLiq"), Some(&"1".to_string()));
}

#[test]
fn test_pct_vol_time_builder_minimal() {
    let params = PctVolTimeBuilder::new().build().unwrap();
    assert_eq!(params.strategy, "PctVolTm");
    assert!(params.params.is_empty());
}

#[test]
fn test_pct_vol_time_validates_both_rates() {
    assert!(PctVolTimeBuilder::new().start_pct_vol(0.1).end_pct_vol(0.5).build().is_ok());

    let err = PctVolTimeBuilder::new().start_pct_vol(0.05).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "start_pct_vol", .. })));

    let err = PctVolTimeBuilder::new().end_pct_vol(0.6).build();
    assert!(matches!(err, Err(ValidationError::InvalidPercentage { field: "end_pct_vol", .. })));
}
