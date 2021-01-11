use anyhow::Result;
use log::{debug};
use std::collections::HashMap;
use std::vec::Vec;
use uuid::Uuid;
use super::{QuerySink, AxonServerHandle, VecU8Message};
use crate::axon_server::SerializedObject;
use crate::axon_server::query::{QueryRequest,QueryResponse};
use crate::axon_server::query::query_service_client::QueryServiceClient;

#[tonic::async_trait]
impl QuerySink for AxonServerHandle {
    async fn send_query<'a>(&self, query_type: &str, query: Box<&(dyn VecU8Message + Sync)>) -> Result<Vec<SerializedObject>> {
        debug!("Sending query: {:?}: {:?}", query_type, self.display_name);
        let mut buf = Vec::new();
        query.encode_u8(&mut buf).unwrap();
        let buffer_length = buf.len();
        debug!("Buffer length: {:?}", buffer_length);
        let serialized_command = SerializedObject {
            r#type: query_type.to_string(),
            revision: "1".to_string(),
            data: buf,
        };
        submit_query(self, &serialized_command).await
    }
}

async fn submit_query<'a>(this: &AxonServerHandle, message: &SerializedObject) -> Result<Vec<SerializedObject>> {
    debug!("Message: {:?}", message);
    let this = this.clone();
    let conn = this.conn;
    let client_id = this.display_name;
    let mut client = QueryServiceClient::new(conn);
    debug!("Query Service Client: {:?}", client);
    let uuid = Uuid::new_v4();
    let query_request = QueryRequest {
        message_identifier: format!("{:?}", uuid.to_simple()),
        query: message.r#type.clone(),
        response_type: None,
        payload: Some(message.clone()),
        client_id,
        component_name: "RustCommandClient".to_string(),
        meta_data: HashMap::new(),
        processing_instructions: Vec::new(),
        timestamp: 0,
    };
    let response = client.query(query_request).await?;
    debug!("Response: {:?}", response);
    let mut response = response.into_inner();

    let mut result = Vec::new();
    loop {
        let query_response = response.message().await?;

        if let Some(QueryResponse { payload: Some(payload), ..}) = query_response {
            let payload = payload.clone();
            debug!("Query response: payload: {:?}", payload);
            result.push(payload);
        } else {
            break;
        }
    }
    Ok(result)
}