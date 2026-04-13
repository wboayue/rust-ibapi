use time::OffsetDateTime;

use crate::contracts::Contract;
use crate::messages::OutgoingMessages;
use crate::orders::{ExecutionFilter, ExerciseAction, Order};
use crate::Error;

pub(crate) fn encode_place_order(order_id: i32, contract: &Contract, order: &Order) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::PlaceOrderRequest {
        order_id: Some(order_id),
        contract: Some(crate::proto::encoders::encode_contract_with_order(contract, Some(order))),
        order: Some(crate::proto::encoders::encode_order(order)),
        attached_orders: None,
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::PlaceOrder as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_cancel_order(order_id: i32, manual_order_cancel_time: &str) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::CancelOrderRequest {
        order_id: Some(order_id),
        order_cancel: Some(crate::proto::encoders::encode_order_cancel(manual_order_cancel_time)),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::CancelOrder as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_open_orders() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(OpenOrdersRequest, OutgoingMessages::RequestOpenOrders)
}

pub(crate) fn encode_all_open_orders() -> Result<Vec<u8>, Error> {
    crate::proto::encoders::encode_empty_proto!(AllOpenOrdersRequest, OutgoingMessages::RequestAllOpenOrders)
}

pub(crate) fn encode_auto_open_orders(auto_bind: bool) -> Result<Vec<u8>, Error> {
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::AutoOpenOrdersRequest {
        auto_bind: some_bool(auto_bind),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestAutoOpenOrders as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_completed_orders(api_only: bool) -> Result<Vec<u8>, Error> {
    use crate::proto::encoders::some_bool;
    use prost::Message;
    let request = crate::proto::CompletedOrdersRequest {
        api_only: some_bool(api_only),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestCompletedOrders as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_executions(request_id: i32, filter: &ExecutionFilter) -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::ExecutionRequest {
        req_id: Some(request_id),
        execution_filter: Some(crate::proto::encoders::encode_execution_filter(filter)),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestExecutions as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_global_cancel() -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::GlobalCancelRequest {
        order_cancel: Some(crate::proto::encoders::encode_order_cancel("")),
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestGlobalCancel as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_next_valid_order_id() -> Result<Vec<u8>, Error> {
    use prost::Message;
    let request = crate::proto::IdsRequest { num_ids: Some(0) };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::RequestIds as i32,
        &request.encode_to_vec(),
    ))
}

pub(crate) fn encode_exercise_options(
    order_id: i32,
    contract: &Contract,
    exercise_action: ExerciseAction,
    exercise_quantity: i32,
    account: &str,
    ovrd: bool,
    manual_order_time: Option<OffsetDateTime>,
) -> Result<Vec<u8>, Error> {
    use crate::proto::encoders::{some_bool, some_str};
    use prost::Message;
    use time::macros::format_description;
    use time_tz::OffsetDateTimeExt;

    let manual_order_time_str = manual_order_time.map(|dt| {
        let adjusted = dt.to_timezone(time_tz::timezones::db::UTC);
        let fmt = format_description!("[year][month][day] [hour]:[minute]:[second]");
        format!("{} UTC", adjusted.format(fmt).unwrap())
    });

    let request = crate::proto::ExerciseOptionsRequest {
        order_id: Some(order_id),
        contract: Some(crate::proto::encoders::encode_contract(contract)),
        exercise_action: Some(exercise_action as i32),
        exercise_quantity: Some(exercise_quantity),
        account: some_str(account),
        r#override: some_bool(ovrd),
        manual_order_time: manual_order_time_str,
        customer_account: None,
        professional_customer: None,
    };
    Ok(crate::messages::encode_protobuf_message(
        OutgoingMessages::ExerciseOptions as i32,
        &request.encode_to_vec(),
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::contracts::Contract;
    use crate::orders::{Action, ExecutionFilter, Order};

    #[test]
    fn test_encode_place_order() {
        let contract = Contract::stock("AAPL").build();
        let order = Order {
            action: Action::Buy,
            total_quantity: 100.0,
            order_type: "LMT".to_string(),
            limit_price: Some(150.0),
            ..Default::default()
        };
        let bytes = encode_place_order(1001, &contract, &order).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::PlaceOrder as i32 + 200);
    }

    #[test]
    fn test_encode_place_order_roundtrip() {
        use prost::Message;
        let contract = Contract::stock("AAPL").build();
        let order = Order {
            action: Action::Buy,
            total_quantity: 100.0,
            order_type: "LMT".to_string(),
            limit_price: Some(150.0),
            transmit: true,
            ..Default::default()
        };
        let bytes = encode_place_order(1001, &contract, &order).unwrap();
        let request = crate::proto::PlaceOrderRequest::decode(&bytes[4..]).unwrap();
        assert_eq!(request.order_id, Some(1001));
        assert_eq!(request.contract.unwrap().symbol.as_deref(), Some("AAPL"));
        let proto_order = request.order.unwrap();
        assert_eq!(proto_order.action.as_deref(), Some("BUY"));
        assert_eq!(proto_order.lmt_price, Some(150.0));
    }

    #[test]
    fn test_encode_cancel_order() {
        let bytes = encode_cancel_order(1001, "").unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::CancelOrder as i32 + 200);
    }

    #[test]
    fn test_encode_open_orders() {
        let bytes = encode_open_orders().unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestOpenOrders as i32 + 200);
    }

    #[test]
    fn test_encode_global_cancel() {
        let bytes = encode_global_cancel().unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestGlobalCancel as i32 + 200);
    }

    #[test]
    fn test_encode_executions() {
        let filter = ExecutionFilter::default();
        let bytes = encode_executions(9000, &filter).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestExecutions as i32 + 200);
    }

    #[test]
    fn test_encode_next_valid_order_id() {
        let bytes = encode_next_valid_order_id().unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestIds as i32 + 200);
    }

    #[test]
    fn test_encode_exercise_options() {
        let contract = Contract::stock("AAPL").build();
        let bytes = encode_exercise_options(1001, &contract, ExerciseAction::Exercise, 1, "DU123456", false, None).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::ExerciseOptions as i32 + 200);
    }

    #[test]
    fn test_encode_completed_orders() {
        let bytes = encode_completed_orders(true).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestCompletedOrders as i32 + 200);
    }

    #[test]
    fn test_encode_all_open_orders() {
        let bytes = encode_all_open_orders().unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestAllOpenOrders as i32 + 200);
    }

    #[test]
    fn test_encode_auto_open_orders() {
        let bytes = encode_auto_open_orders(true).unwrap();
        let msg_id = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(msg_id, crate::messages::OutgoingMessages::RequestAutoOpenOrders as i32 + 200);
    }
}
