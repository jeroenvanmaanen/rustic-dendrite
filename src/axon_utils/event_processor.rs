use anyhow::Result;
use async_stream::stream;
use futures_core::stream::Stream;
use log::debug;
use tokio::sync::mpsc::{Sender,Receiver, channel};
use super::AxonServerHandle;
use super::handler_registry::TheHandlerRegistry;
use crate::axon_server::event::{Event,EventWithToken,GetEventsRequest};
use crate::axon_server::event::event_store_client::EventStoreClient;

#[derive(Debug)]
struct AxonEventProcessed {
    message_identifier: String,
}

#[tonic::async_trait]
pub trait TokenStore {
    async fn store_token(&self, token: i64);
    async fn retrieve_token(&self) -> Result<i64>;
}

pub async fn event_processor<Q: TokenStore + Send + Sync + Clone>(
    axon_server_handle: AxonServerHandle,
    query_model: Q,
    event_handler_registry: TheHandlerRegistry<Q,Option<Q>>
) -> Result<()> {
    let conn = axon_server_handle.conn;
    let mut client = EventStoreClient::new(conn);

    let (mut tx, rx): (Sender<AxonEventProcessed>, Receiver<AxonEventProcessed>) = channel(10);

    let initial_token = query_model.retrieve_token().await.unwrap_or(-1) + 1;
    debug!("Initial token: {:?}", initial_token);
    let outbound = create_output_stream(axon_server_handle.display_name, initial_token, rx);

    debug!("Event Processor: calling open_stream");
    let response = client.list_events(outbound).await?;
    debug!("Stream response: {:?}", response);

    let mut events = response.into_inner();
    loop {
        let event_with_token = events.message().await?;
        debug!("Event with token: {:?}", event_with_token);

        if let Some(EventWithToken { event: Some(event), token, ..}) = event_with_token {
            if let Event { payload: Some(serialized_object), .. } = event {
                if let Some(event_handler) = event_handler_registry.handlers.get(&serialized_object.r#type) {
                    (event_handler).handle(serialized_object.data, query_model.clone()).await?;
                }
            }

            query_model.store_token(token).await;

            tx.send(AxonEventProcessed {
                message_identifier: event.message_identifier,
            }).await?;
        }
    }
}

fn create_output_stream(client_id: String, initial_token: i64, mut rx: Receiver<AxonEventProcessed>) -> impl Stream<Item = GetEventsRequest> {
    stream! {
        debug!("Event Processor: stream: start: {:?}", rx);

        let permits_batch_size: i64 = 3;
        let mut permits = permits_batch_size * 2;

        let mut request = GetEventsRequest {
            tracking_token: initial_token,
            number_of_permits: permits,
            client_id: client_id,
            component_name: "Dendrite".to_string(),
            processor: "Event Processor".to_string(),
            blacklist: Vec::new(),
            force_read_from_leader: false,
        };
        yield request.clone();

        request.number_of_permits = permits_batch_size;

        while let Some(axon_event_processed) = rx.recv().await {
            debug!("Event processed: {:?}", axon_event_processed);
            permits -= 1;
            if permits <= permits_batch_size {
                debug!("Event Processor: stream: send more flow-control permits: amount: {:?}", permits_batch_size);
                yield request.clone();
                permits += permits_batch_size;
            }
            debug!("Event Processor: stream: flow-control permits: balance: {:?}", permits);
        }
    }
}
