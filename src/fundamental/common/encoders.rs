use prost::Message;

use crate::contracts::Contract;
use crate::fundamental::FundamentalReportType;
use crate::messages::{encode_protobuf_message, OutgoingMessages};
use crate::proto;
use crate::proto::encoders::encode_contract;
use crate::Error;

pub(in crate::fundamental) fn encode_request_fundamental_data(
    request_id: i32,
    contract: &Contract,
    report_type: FundamentalReportType,
) -> Result<Vec<u8>, Error> {
    let request = proto::FundamentalsDataRequest {
        req_id: Some(request_id),
        contract: Some(encode_contract(contract)),
        report_type: Some(report_type.to_string()),
        fundamentals_data_options: Default::default(),
    };
    Ok(encode_protobuf_message(
        OutgoingMessages::RequestFundamentalData as i32,
        &request.encode_to_vec(),
    ))
}
