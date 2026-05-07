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
