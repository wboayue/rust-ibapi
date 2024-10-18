use crate::testdata::responses::MARKET_RULE;

use super::*;

#[test]
fn test_decode_market_rule() {
    let mut message = ResponseMessage::from_simple(MARKET_RULE);

    let market_rule = decode_market_rule(&mut message).expect("error decoding market rule");

    assert_eq!(market_rule.market_rule_id, 26, "market_rule.market_rule_id");

    assert_eq!(market_rule.price_increments.len(), 1, "market_rule.price_increments.len()");
    assert_eq!(market_rule.price_increments[0].low_edge, 0.0, "market_rule.price_increments[0].low_edge");
    assert_eq!(
        market_rule.price_increments[0].increment, 0.01,
        "market_rule.price_increments[0].increment"
    );
}
