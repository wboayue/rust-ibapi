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

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        encoders::encode_cancel_wsh_metadata(request_id.ok_or(Error::Simple("request_id required".into()))?)
    }
}

impl AsyncDataStream<WshEventData> for WshEventData {
    fn decode(_client: &Client, message: &mut crate::messages::ResponseMessage) -> Result<WshEventData, Error> {
        decode_event_data_message(message.clone())
    }

    fn cancel_message(
        _server_version: i32,
        request_id: Option<i32>,
        _context: &crate::client::builders::ResponseContext,
    ) -> Result<crate::messages::RequestMessage, Error> {
        encoders::encode_cancel_wsh_event_data(request_id.ok_or(Error::Simple("request_id required".into()))?)
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
    let mut subscription = builder.send::<WshMetadata>(request).await?;

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
    let mut subscription = builder.send::<WshEventData>(request).await?;

    match subscription.next().await {
        Some(Ok(event_data)) => Ok(event_data),
        Some(Err(Error::ConnectionReset)) => Box::pin(wsh_event_data_by_contract(client, contract_id, start_date, end_date, limit, auto_fill)).await,
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
        None, // start_date
        None, // end_date
        limit,
        auto_fill,
    )?;

    builder.send::<WshEventData>(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::MessageBusStub;
    use std::sync::{Arc, RwLock};
    use time::macros::date;

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

    #[tokio::test]
    async fn test_wsh_event_data_by_contract_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["105|9001|{\"validated\":true,\"data\":{\"events\":[]}}|".to_owned()],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let result = wsh_event_data_by_contract(
            &client,
            12345,
            Some(date!(2024 - 01 - 01)),
            Some(date!(2024 - 12 - 31)),
            Some(100),
            Some(AutoFill {
                competitors: true,
                portfolio: false,
                watchlist: true,
            }),
        )
        .await;

        assert!(result.is_ok(), "failed to request wsh event data: {}", result.err().unwrap());
        assert_eq!(
            result.unwrap(),
            WshEventData {
                data_json: "{\"validated\":true,\"data\":{\"events\":[]}}".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn test_wsh_event_data_by_contract_no_filters_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec!["105|9002|{\"events\":[{\"type\":\"earnings\"}]}|".to_owned()],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSHE_CALENDAR);
        let result = wsh_event_data_by_contract(&client, 12345, None, None, None, None).await;

        assert!(result.is_ok(), "failed to request wsh event data: {}", result.err().unwrap());
        assert_eq!(
            result.unwrap(),
            WshEventData {
                data_json: "{\"events\":[{\"type\":\"earnings\"}]}".to_owned()
            }
        );
    }

    #[tokio::test]
    async fn test_wsh_event_data_by_filter_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![
                "105|9003|{\"event\":\"earnings\",\"date\":\"2024-01-15\"}|".to_owned(),
                "105|9003|{\"event\":\"dividend\",\"date\":\"2024-02-01\"}|".to_owned(),
            ],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let filter = "earnings";
        let mut subscription = wsh_event_data_by_filter(&client, filter, Some(50), None)
            .await
            .expect("failed to create subscription");

        // First event
        let first = subscription.next().await;
        assert!(first.is_some());
        assert!(first.unwrap().is_ok());

        // Second event
        let second = subscription.next().await;
        assert!(second.is_some());
        assert!(second.unwrap().is_ok());

        // No more events
        let third = subscription.next().await;
        assert!(third.is_none());
    }

    #[tokio::test]
    async fn test_wsh_metadata_server_version_error_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, 100); // Old server version
        let result = wsh_metadata(&client).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ServerVersion(_, _, _)));
    }

    #[tokio::test]
    async fn test_wsh_event_data_server_version_error_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSHE_CALENDAR);

        // Test filter version requirement
        let result = wsh_event_data_by_filter(&client, "filter", None, None).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, Error::ServerVersion(_, _, _)));
        }
    }

    #[tokio::test]
    async fn test_wsh_event_data_date_filter_version_error_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![],
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS);

        // Test date filter version requirement
        let result = wsh_event_data_by_contract(&client, 12345, Some(date!(2024 - 01 - 01)), None, None, None).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ServerVersion(_, _, _)));
    }

    #[tokio::test]
    async fn test_empty_subscription_async() {
        let message_bus = Arc::new(MessageBusStub {
            request_messages: RwLock::new(vec![]),
            response_messages: vec![], // No messages
        });

        let client = Client::stubbed(message_bus, crate::server_versions::WSH_EVENT_DATA_FILTERS_DATE);
        let mut subscription = wsh_event_data_by_filter(&client, "filter", None, None)
            .await
            .expect("failed to create subscription");

        let result = subscription.next().await;
        assert!(result.is_none());
    }
}
