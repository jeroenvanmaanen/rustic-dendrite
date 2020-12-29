use anyhow::Result;
use tonic::transport::Channel;
use crate::axon_server::event::{Event,GetAggregateEventsRequest};
use crate::axon_server::event::event_store_client::EventStoreClient;
use super::AxonServerHandle;

pub async fn query_events(axon_server_handle: &AxonServerHandle, aggregate_identifier: &str) -> Result<Vec<Event>> {
    let axon_server_handle = axon_server_handle.clone();
    let conn = axon_server_handle.conn;
    let mut client = EventStoreClient::new(conn);
    query_events_from_client(&mut client, aggregate_identifier).await
}

pub async fn query_events_from_client(client: &mut EventStoreClient<Channel>, aggregate_identifier: &str) -> Result<Vec<Event>> {
    let request = GetAggregateEventsRequest {
        aggregate_id: aggregate_identifier.to_string(),
        allow_snapshots: false,
        initial_sequence: 0,
        max_sequence: std::i64::MAX,
        min_token: 0,
    };
    let mut result = Vec::new();
    let mut stream = client.list_aggregate_events(request).await?.into_inner();
    while let Some(event) = stream.message().await? {
        result.push(event.clone());
    }
    Ok(result)
}