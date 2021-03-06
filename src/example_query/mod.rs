use anyhow::{Context,Result};
use elasticsearch::{Elasticsearch, SearchParts};
use log::{debug,error};
use prost::Message;
use super::elastic_search_utils::wait_for_elastic_search;
use crate::axon_utils::{AxonServerHandle, HandlerRegistry, QueryContext, QueryResult, TheHandlerRegistry, empty_handler_registry, query_processor, axon_serialize};
use crate::grpc_example::{SearchQuery,SearchResponse,Greeting};

#[derive(Clone)]
struct ExampleQueryContext {
    es_client: Elasticsearch,
}

impl QueryContext for ExampleQueryContext {}

pub async fn process_queries(axon_server_handle : AxonServerHandle) {
    if let Err(e) = internal_process_queries(axon_server_handle).await {
        error!("Error while handling queries: {:?}", e);
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

    query_processor(axon_server_handle, query_context, query_handler_registry).await.context("Error while handling queries")
}

async fn handle_search_query(search_query: SearchQuery, projection: ExampleQueryContext) -> Result<Option<QueryResult>> {
    let search_response = projection.es_client
        .search(SearchParts::Index(&["greetings"]))
        .q(&search_query.query)
        ._source(&["value"])
        .send()
        .await?;
    let json_value : serde_json::Value = search_response.json().await?;
    debug!("Search response: {:?}", json_value);
    let hits = &json_value["hits"]["hits"];
    debug!("Hits: {:?}", hits);
    let mut greetings = Vec::new();
    if let serde_json::Value::Array(hits) = hits {
        for document in hits {
            if let serde_json::Value::String(message) = &document["_source"]["value"] {
                let greeting = Greeting {
                    message: message.clone(),
                };
                greetings.push(greeting);
            }
        }
    }
    let greeting = Greeting {
        message: "Test!".to_string(),
    };
    greetings.push(greeting);
    let response = SearchResponse {
        greetings,
    };
    let result = axon_serialize("SearchResponse", &response)?;
    let query_result = QueryResult {
        payload: Some(result),
    };
    Ok(Some(query_result))
}
