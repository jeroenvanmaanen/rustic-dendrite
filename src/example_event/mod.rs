use anyhow::{Context,Result};
use elasticsearch::{Elasticsearch, IndexParts};
use log::{debug,error};
use prost::Message;
use serde_json::json;
use sha2::{Sha256, Digest};
use super::elastic_search_utils::wait_for_elastic_search;
use crate::axon_utils::{AsyncApplicableTo, AxonServerHandle, HandlerRegistry, event_processor, empty_handler_registry, TheHandlerRegistry};
use crate::grpc_example::{GreetedEvent,Greeting};

#[derive(Clone)]
struct ExampleQueryModel {
    es_client: Elasticsearch,
}

pub async fn process_events(axon_server_handle : AxonServerHandle) {
    if let Err(e) = internal_process_events(axon_server_handle).await {
        error!("Error while handling commands: {:?}", e);
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_process_events(axon_server_handle : AxonServerHandle) -> Result<()> {
    let client = wait_for_elastic_search().await?;
    debug!("Elastic Search client: {:?}", client);

    let query_model = ExampleQueryModel {
        es_client: client,
    };

    let mut event_handler_registry: TheHandlerRegistry<ExampleQueryModel,Option<ExampleQueryModel>> = empty_handler_registry();

    event_handler_registry.insert(
        "GreetedEvent",
        &GreetedEvent::decode,
        &(|c, p| Box::pin(handle_event(Box::from(c), p)))
    )?;

    event_processor(axon_server_handle, query_model, event_handler_registry).await.context("Error while handling commands")
}

async fn handle_event<T: AsyncApplicableTo<P>,P: Clone>(event: Box<T>, projection: P) -> Result<()> {
    let mut p = projection.clone();
    event.apply_to(&mut p).await?;
    Ok(())
}

#[tonic::async_trait]
impl AsyncApplicableTo<ExampleQueryModel> for GreetedEvent {

    async fn apply_to(self: &Self, projection: &mut ExampleQueryModel) -> Result<()> {
        debug!("Apply greeted event to ExampleQueryModel");
        let es_client = projection.es_client.clone();
        if let Some(Greeting {message}) = self.message.clone() {
            let value = message.clone();
            let mut hasher = Sha256::new();
            Digest::update(&mut hasher,&message);
            let hash: Vec<u8> = hasher.finalize().to_vec();
            let hash = base64::encode(hash);
            let response = es_client
                .index(IndexParts::IndexId("greetings", hash.as_str()))
                .body(json!({
                    "id": hash,
                    "value": value,
                }))
                .send()
                .await
            ;
            debug!("Elastic Search response: {:?}", response);
        }
        Ok(())
    }

    fn box_clone(self: &Self) -> Box<dyn AsyncApplicableTo<ExampleQueryModel>> {
        Box::from(GreetedEvent::clone(self))
    }
}
