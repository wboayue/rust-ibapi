use prost::Message;

use crate::contracts::ContractDetails;
use crate::Error;

#[allow(dead_code)]
pub(crate) fn decode_contract_data_proto(bytes: &[u8]) -> Result<ContractDetails, Error> {
    let p: crate::proto::ContractData = Message::decode(bytes)?;
    let default_contract = crate::proto::Contract::default();
    let default_details = crate::proto::ContractDetails::default();
    let proto_contract = p.contract.as_ref().unwrap_or(&default_contract);
    let proto_details = p.contract_details.as_ref().unwrap_or(&default_details);
    Ok(crate::proto::decoders::decode_contract_details(proto_contract, proto_details))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn test_decode_contract_data_proto() {
        let proto_msg = crate::proto::ContractData {
            req_id: Some(1),
            contract: Some(crate::proto::Contract {
                con_id: Some(265598),
                symbol: Some("AAPL".into()),
                sec_type: Some("STK".into()),
                exchange: Some("SMART".into()),
                currency: Some("USD".into()),
                local_symbol: Some("AAPL".into()),
                trading_class: Some("NMS".into()),
                ..Default::default()
            }),
            contract_details: Some(crate::proto::ContractDetails {
                market_name: Some("NMS".into()),
                min_tick: Some("0.01".into()),
                long_name: Some("APPLE INC".into()),
                industry: Some("Technology".into()),
                category: Some("Computers".into()),
                subcategory: Some("Consumer Electronics".into()),
                ..Default::default()
            }),
        };

        let mut bytes = Vec::new();
        proto_msg.encode(&mut bytes).unwrap();

        let result = decode_contract_data_proto(&bytes).unwrap();
        assert_eq!(result.contract.contract_id, 265598);
        assert_eq!(result.contract.symbol.to_string(), "AAPL");
        assert_eq!(result.contract.currency.to_string(), "USD");
        assert_eq!(result.contract.local_symbol, "AAPL");
        assert_eq!(result.market_name, "NMS");
        assert_eq!(result.min_tick, 0.01);
        assert_eq!(result.long_name, "APPLE INC");
        assert_eq!(result.industry, "Technology");
        assert_eq!(result.category, "Computers");
        assert_eq!(result.subcategory, "Consumer Electronics");
    }
}
