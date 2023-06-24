use crate::contracts::{ComboLegOpenClose, SecurityType};
use crate::orders::{Action, OrderCondition, OrderOpenClose, Rule80A};

use super::*;

#[test]
fn test_message_encodes_bool() {
    let mut message = RequestMessage::new();

    message.push_field(&false);
    message.push_field(&true);

    assert_eq!(2, message.fields.len());
    assert_eq!("0\01\0", message.encode());
}

#[test]
fn test_message_encodes_i32() {
    let mut message = RequestMessage::new();

    message.push_field(&1);
    message.push_field(&Some(2));
    message.push_field(&Option::<i32>::None);

    assert_eq!(3, message.fields.len());
    assert_eq!("1\02\0\0", message.encode());
}

#[test]
fn test_message_encodes_f64() {
    let mut message = RequestMessage::new();

    message.push_field(&2.0);
    message.push_field(&Some(3.0));
    message.push_field(&Option::<f64>::None);

    assert_eq!(3, message.fields.len());
    // assert_eq!("2.0\03.0\0\0", message.encode());
}

#[test]
fn test_message_encodes_string() {
    let mut message = RequestMessage::new();

    message.push_field(&"interactive");
    message.push_field(&"brokers");

    assert_eq!(2, message.fields.len());
    assert_eq!("interactive\0brokers\0", message.encode());
}

#[test]
fn test_message_encodes_rule_80_a() {
    let mut message = RequestMessage::new();

    message.push_field(&Some(Rule80A::Individual));
    message.push_field(&Some(Rule80A::Agency));
    message.push_field(&Some(Rule80A::AgentOtherMember));
    message.push_field(&Some(Rule80A::IndividualPTIA));
    message.push_field(&Some(Rule80A::AgencyPTIA));
    message.push_field(&Some(Rule80A::AgentOtherMemberPTIA));
    message.push_field(&Some(Rule80A::IndividualPT));
    message.push_field(&Some(Rule80A::AgencyPT));
    message.push_field(&Some(Rule80A::AgentOtherMemberPT));
    message.push_field(&Option::<Rule80A>::None);

    assert_eq!(10, message.fields.len());
    assert_eq!("I\0A\0W\0J\0U\0M\0K\0Y\0N\0\0", message.encode());
}

#[test]
fn test_message_encodes_order_condition() {
    let mut message = RequestMessage::new();

    message.push_field(&OrderCondition::Price);
    message.push_field(&OrderCondition::Time);
    message.push_field(&OrderCondition::Margin);
    message.push_field(&OrderCondition::Execution);
    message.push_field(&OrderCondition::Volume);
    message.push_field(&OrderCondition::PercentChange);

    assert_eq!(6, message.fields.len());
    assert_eq!("1\03\04\05\06\07\0", message.encode());
}

#[test]
fn test_message_encodes_action() {
    let mut message = RequestMessage::new();

    message.push_field(&Action::Buy);
    message.push_field(&Action::Sell);
    message.push_field(&Action::SellShort);
    message.push_field(&Action::SellLong);

    assert_eq!(4, message.fields.len());
    assert_eq!("BUY\0SELL\0SSHORT\0SLONG\0", message.encode());
}

#[test]
fn test_message_encodes_security_type() {
    let mut message = RequestMessage::new();

    message.push_field(&SecurityType::Stock);
    message.push_field(&SecurityType::Option);
    message.push_field(&SecurityType::Future);
    message.push_field(&SecurityType::Index);
    message.push_field(&SecurityType::FuturesOption);
    message.push_field(&SecurityType::ForexPair);
    message.push_field(&SecurityType::Spread);
    message.push_field(&SecurityType::Warrant);
    message.push_field(&SecurityType::Bond);
    message.push_field(&SecurityType::Commodity);
    message.push_field(&SecurityType::News);
    message.push_field(&SecurityType::MutualFund);

    assert_eq!(12, message.fields.len());
    assert_eq!("STK\0OPT\0FUT\0IND\0FOP\0CASH\0BAG\0WAR\0BOND\0CMDTY\0NEWS\0FUND\0", message.encode());
}

#[test]
fn test_message_encodes_outgoing_message() {
    let mut message = RequestMessage::new();

    message.push_field(&OutgoingMessages::RequestMarketData);
    message.push_field(&OutgoingMessages::CancelMarketData);
    message.push_field(&OutgoingMessages::PlaceOrder);
    message.push_field(&OutgoingMessages::ReqUserInfo);

    assert_eq!(4, message.fields.len());
    assert_eq!("1\02\03\0104\0", message.encode());
}

#[test]
fn test_message_encodes_order_open_close() {
    let mut message = RequestMessage::new();

    message.push_field(&Option::<OrderOpenClose>::None);
    message.push_field(&OrderOpenClose::Open);
    message.push_field(&OrderOpenClose::Close);

    assert_eq!(3, message.fields.len());
    assert_eq!("\0O\0C\0", message.encode());
}

#[test]
fn test_message_encodes_combo_leg_open_close() {
    let mut message = RequestMessage::new();

    message.push_field(&ComboLegOpenClose::Same);
    message.push_field(&ComboLegOpenClose::Open);
    message.push_field(&ComboLegOpenClose::Close);
    message.push_field(&ComboLegOpenClose::Unknown);

    assert_eq!(4, message.fields.len());
    assert_eq!("0\01\02\03\0", message.encode());
}

#[test]
fn test_incoming_message_from_i32() {
    assert_eq!(IncomingMessages::from(1), IncomingMessages::TickPrice);
    assert_eq!(IncomingMessages::from(2), IncomingMessages::TickSize);
    assert_eq!(IncomingMessages::from(3), IncomingMessages::OrderStatus);
    assert_eq!(IncomingMessages::from(4), IncomingMessages::Error);
    assert_eq!(IncomingMessages::from(5), IncomingMessages::OpenOrder);
    assert_eq!(IncomingMessages::from(6), IncomingMessages::AccountValue);
    assert_eq!(IncomingMessages::from(7), IncomingMessages::PortfolioValue);
    assert_eq!(IncomingMessages::from(8), IncomingMessages::AccountUpdateTime);
    assert_eq!(IncomingMessages::from(9), IncomingMessages::NextValidId);
    assert_eq!(IncomingMessages::from(10), IncomingMessages::ContractData);
    assert_eq!(IncomingMessages::from(11), IncomingMessages::ExecutionData);
    assert_eq!(IncomingMessages::from(12), IncomingMessages::MarketDepth);
    assert_eq!(IncomingMessages::from(13), IncomingMessages::MarketDepthL2);
    assert_eq!(IncomingMessages::from(14), IncomingMessages::NewsBulletins);
    assert_eq!(IncomingMessages::from(15), IncomingMessages::ManagedAccounts);
    assert_eq!(IncomingMessages::from(16), IncomingMessages::ReceiveFA);
    assert_eq!(IncomingMessages::from(17), IncomingMessages::HistoricalData);
    assert_eq!(IncomingMessages::from(18), IncomingMessages::BondContractData);
    assert_eq!(IncomingMessages::from(19), IncomingMessages::ScannerParameters);
    assert_eq!(IncomingMessages::from(20), IncomingMessages::ScannerData);
    assert_eq!(IncomingMessages::from(21), IncomingMessages::TickOptionComputation);
    assert_eq!(IncomingMessages::from(45), IncomingMessages::TickGeneric);
    assert_eq!(IncomingMessages::from(46), IncomingMessages::Tickstring);
    assert_eq!(IncomingMessages::from(47), IncomingMessages::TickEFP);
    assert_eq!(IncomingMessages::from(49), IncomingMessages::CurrentTime);
    assert_eq!(IncomingMessages::from(50), IncomingMessages::RealTimeBars);
    assert_eq!(IncomingMessages::from(51), IncomingMessages::FundamentalData);
    assert_eq!(IncomingMessages::from(52), IncomingMessages::ContractDataEnd);
    assert_eq!(IncomingMessages::from(53), IncomingMessages::OpenOrderEnd);
    assert_eq!(IncomingMessages::from(54), IncomingMessages::AccountDownloadEnd);
    assert_eq!(IncomingMessages::from(55), IncomingMessages::ExecutionDataEnd);
    assert_eq!(IncomingMessages::from(56), IncomingMessages::DeltaNeutralValidation);
    assert_eq!(IncomingMessages::from(57), IncomingMessages::TickSnapshotEnd);
    assert_eq!(IncomingMessages::from(58), IncomingMessages::MarketDataType);
    assert_eq!(IncomingMessages::from(59), IncomingMessages::CommissionsReport);
    assert_eq!(IncomingMessages::from(61), IncomingMessages::Position);
    assert_eq!(IncomingMessages::from(62), IncomingMessages::PositionEnd);
    assert_eq!(IncomingMessages::from(63), IncomingMessages::AccountSummary);
    assert_eq!(IncomingMessages::from(64), IncomingMessages::AccountSummaryEnd);
    assert_eq!(IncomingMessages::from(65), IncomingMessages::VerifyMessageApi);
    assert_eq!(IncomingMessages::from(66), IncomingMessages::VerifyCompleted);
    assert_eq!(IncomingMessages::from(67), IncomingMessages::DisplayGroupList);
    assert_eq!(IncomingMessages::from(68), IncomingMessages::DisplayGroupUpdated);
    assert_eq!(IncomingMessages::from(69), IncomingMessages::VerifyAndAuthMessageApi);
    assert_eq!(IncomingMessages::from(70), IncomingMessages::VerifyAndAuthCompleted);
    assert_eq!(IncomingMessages::from(71), IncomingMessages::PositionMulti);
    assert_eq!(IncomingMessages::from(72), IncomingMessages::PositionMultiEnd);
    assert_eq!(IncomingMessages::from(73), IncomingMessages::AccountUpdateMulti);
    assert_eq!(IncomingMessages::from(74), IncomingMessages::AccountUpdateMultiEnd);
    assert_eq!(IncomingMessages::from(75), IncomingMessages::SecurityDefinitionOptionParameter);
    assert_eq!(IncomingMessages::from(76), IncomingMessages::SecurityDefinitionOptionParameterEnd);
    assert_eq!(IncomingMessages::from(77), IncomingMessages::SoftDollarTier);
    assert_eq!(IncomingMessages::from(78), IncomingMessages::FamilyCodes);
    assert_eq!(IncomingMessages::from(79), IncomingMessages::SymbolSamples);
    assert_eq!(IncomingMessages::from(80), IncomingMessages::MktDepthExchanges);
    assert_eq!(IncomingMessages::from(81), IncomingMessages::TickReqParams);
    assert_eq!(IncomingMessages::from(82), IncomingMessages::SmartComponents);
    assert_eq!(IncomingMessages::from(83), IncomingMessages::NewsArticle);
    assert_eq!(IncomingMessages::from(84), IncomingMessages::TickNews);
    assert_eq!(IncomingMessages::from(85), IncomingMessages::NewsProviders);
    assert_eq!(IncomingMessages::from(86), IncomingMessages::HistoricalNews);
    assert_eq!(IncomingMessages::from(87), IncomingMessages::HistoricalNewsEnd);
    assert_eq!(IncomingMessages::from(88), IncomingMessages::HeadTimestamp);
    assert_eq!(IncomingMessages::from(89), IncomingMessages::HistogramData);
    assert_eq!(IncomingMessages::from(90), IncomingMessages::HistoricalDataUpdate);
    assert_eq!(IncomingMessages::from(91), IncomingMessages::RerouteMktDataReq);
    assert_eq!(IncomingMessages::from(92), IncomingMessages::RerouteMktDepthReq);
    assert_eq!(IncomingMessages::from(93), IncomingMessages::MarketRule);
    assert_eq!(IncomingMessages::from(94), IncomingMessages::PnL);
    assert_eq!(IncomingMessages::from(95), IncomingMessages::PnLSingle);
    assert_eq!(IncomingMessages::from(96), IncomingMessages::HistoricalTick);
    assert_eq!(IncomingMessages::from(97), IncomingMessages::HistoricalTickBidAsk);
    assert_eq!(IncomingMessages::from(98), IncomingMessages::HistoricalTickLast);
    assert_eq!(IncomingMessages::from(99), IncomingMessages::TickByTick);
    assert_eq!(IncomingMessages::from(100), IncomingMessages::OrderBound);
    assert_eq!(IncomingMessages::from(101), IncomingMessages::CompletedOrder);
    assert_eq!(IncomingMessages::from(102), IncomingMessages::CompletedOrdersEnd);
    assert_eq!(IncomingMessages::from(103), IncomingMessages::ReplaceFAEnd);
    assert_eq!(IncomingMessages::from(104), IncomingMessages::WshMetaData);
    assert_eq!(IncomingMessages::from(105), IncomingMessages::WshEventData);
    assert_eq!(IncomingMessages::from(106), IncomingMessages::HistoricalSchedule);
    assert_eq!(IncomingMessages::from(107), IncomingMessages::UserInfo);
    assert_eq!(IncomingMessages::from(108), IncomingMessages::NotValid);
}

#[test]
fn test_order_id_index() {
    assert_eq!(order_id_index(IncomingMessages::OpenOrder), Some(1));
    assert_eq!(order_id_index(IncomingMessages::OrderStatus), Some(1));

    assert_eq!(order_id_index(IncomingMessages::ExecutionData), Some(2));
    assert_eq!(order_id_index(IncomingMessages::ExecutionDataEnd), Some(2));

    assert_eq!(order_id_index(IncomingMessages::NotValid), None);
}

#[test]
fn test_request_id_index() {
    assert_eq!(request_id_index(IncomingMessages::ContractData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::TickByTick), Some(1));
    assert_eq!(request_id_index(IncomingMessages::SymbolSamples), Some(1));
    assert_eq!(request_id_index(IncomingMessages::OpenOrder), Some(1));
    assert_eq!(request_id_index(IncomingMessages::ExecutionData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HeadTimestamp), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HistoricalData), Some(1));
    assert_eq!(request_id_index(IncomingMessages::HistoricalSchedule), Some(1));

    assert_eq!(request_id_index(IncomingMessages::ContractDataEnd), Some(2));
    assert_eq!(request_id_index(IncomingMessages::RealTimeBars), Some(2));
    assert_eq!(request_id_index(IncomingMessages::Error), Some(2));
    assert_eq!(request_id_index(IncomingMessages::ExecutionDataEnd), Some(2));
}

#[test]
#[should_panic]
fn test_request_id_index_invalid() {
    assert_eq!(request_id_index(IncomingMessages::NotValid), None);
}
