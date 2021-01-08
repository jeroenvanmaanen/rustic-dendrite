use anyhow::{Context,Result};
use elasticsearch::Elasticsearch;
use log::{debug,error};
use prost::Message;
use super::elastic_search_utils::wait_for_elastic_search;
use crate::axon_utils::{AxonServerHandle, HandlerRegistry, QueryContext, QueryResult, TheHandlerRegistry, empty_handler_registry, query_processor};
use crate::grpc_example::SearchQuery;

#[derive(Clone)]
struct ExampleQueryContext {
    es_client: Elasticsearch,
}

impl QueryContext for ExampleQueryContext {}

pub async fn process_queries(axon_server_handle : AxonServerHandle) {
    if let Err(e) = internal_process_queries(axon_server_handle).await {
        error!("Error while handling commands: {:?}", e);
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_process_queries(axon_server_handle : AxonServerHandle) -> Result<()> {
    let client = wait_for_elastic_search().await?;
    debug!("Elastic Search client: {:?}", client);

    let query_context = ExampleQueryContext {
        es_client: client,
    };

    let mut query_handler_registry: TheHandlerRegistry<ExampleQueryContext,QueryResult> = empty_handler_registry();

    query_handler_registry.insert_with_output(
        "SearchQuery",
        &SearchQuery::decode,
        &(|c, p| Box::pin(handle_search_query(c, p)))
    )?;

    query_processor(axon_server_handle, query_context, query_handler_registry).await.context("Error while handling commands")
}

async fn handle_search_query(_event: SearchQuery, _projection: ExampleQueryContext) -> Result<Option<QueryResult>> {
    Ok(None)
}
