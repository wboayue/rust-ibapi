//! Asynchronous implementation of Wall Street Horizon functionality

use time::Date;

use crate::{
    client::ClientRequestBuilders,
    messages::IncomingMessages,
    protocol::{check_version, Features},
    subscriptions::{r#async::AsyncDataStream, Subscription},
    Client, Error,
};

use super::{decoders, encoders, AutoFill, WshEventData, WshMetadata};

impl AsyncDataStream<WshMetadata> for WshMetadata {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshMetadata, Error> {
        match message.message_type() {
            IncomingMessages::WshMetaData => Ok(decoders::decode_wsh_metadata(message.clone())?),
            _ => Err(Error::UnexpectedResponse(message.clone())),
        }
    }
}

impl AsyncDataStream<WshEventData> for WshEventData {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
        decode_event_data_message(message.clone())
    }
}

fn decode_event_data_message(message: crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
    match message.message_type() {
        IncomingMessages::WshEventData => decoders::decode_wsh_event_data(message),
        IncomingMessages::Error => Err(Error::from(message)),
        _ => Err(Error::UnexpectedResponse(message)),
    }
}

pub async fn wsh_metadata(client: &Client) -> Result<WshMetadata, Error> {
    check_version(client.server_version(), Features::WSHE_CALENDAR)?;

    let builder = client.request();
    let request = encoders::encode_request_wsh_metadata(builder.request_id())?;
    let mut subscription = builder.send::<WshMetadata, WshMetadata>(request).await?;

    match subscription.next().await {
        Some(Ok(metadata)) => Ok(metadata),
        Some(Err(Error::ConnectionReset)) => Box::pin(wsh_metadata(client)).await,
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub async fn wsh_event_data_by_contract(
    client: &Client,
    contract_id: i32,
    start_date: Option<Date>,
    end_date: Option<Date>,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<WshEventData, Error> {
    check_version(client.server_version(), Features::WSHE_CALENDAR)?;

    if auto_fill.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;
    }

    if start_date.is_some() || end_date.is_some() || limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version(),
        builder.request_id(),
        Some(contract_id),
        None,
        start_date,
        end_date,
        limit,
        auto_fill,
    )?;
    let mut subscription = builder.send::<WshEventData, WshEventData>(request).await?;

    match subscription.next().await {
        Some(Ok(event_data)) => Ok(event_data),
        Some(Err(Error::ConnectionReset)) => {
            Box::pin(wsh_event_data_by_contract(client, contract_id, start_date, end_date, limit, auto_fill)).await
        }
        Some(Err(e)) => Err(e),
        None => Err(Error::UnexpectedEndOfStream),
    }
}

pub async fn wsh_event_data_by_filter(
    client: &Client,
    filter: &str,
    limit: Option<i32>,
    auto_fill: Option<AutoFill>,
) -> Result<Subscription<WshEventData>, Error> {
    check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS)?;

    if limit.is_some() {
        check_version(client.server_version(), Features::WSH_EVENT_DATA_FILTERS_DATE)?;
    }

    let builder = client.request();
    let request = encoders::encode_request_wsh_event_data(
        client.server_version(),
        builder.request_id(),
        None,
        Some(filter),
        None,  // start_date  
        None,  // end_date
        limit,
        auto_fill,
    )?;
    
    builder.send::<WshEventData, WshEventData>(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};

    #[tokio::test]
    async fn test_wsh_metadata_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["104|9000|{\"validated\":true,\"data\":{\"metadata\":\"test\"}}|".to_owned()],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSHE_CALENDAR);
        let result = wsh_metadata(&client).await;

        assert!(result.is_ok(), "failed to request wsh metadata: {}", result.err().unwrap());
        assert_eq!(
            result.unwrap(),
            WshMetadata {
                data_json: "{\"validated\":true,\"data\":{\"metadata\":\"test\"}}".to_owned()
            }
        );
    }
}