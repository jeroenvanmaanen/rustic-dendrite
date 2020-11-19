use log::{debug};
use prost::EncodeError;
use std::collections::HashMap;
use std::pin::Pin;
use std::vec::Vec;
use uuid::Uuid;
use super::{CommandSink, AxonServerHandle, wait_for_server, VecU8Message};
use crate::axon_server::SerializedObject;
use crate::axon_server::command::Command;
use crate::axon_server::command::command_service_client::CommandServiceClient;

pub async fn init() -> Result<AxonServerHandle, Box<dyn std::error::Error>> {
    let axon_connection = wait_for_server("proxy", 8124, "API").await.unwrap();
    debug!("Axon connection: {:?}", axon_connection);
    let command_sink = AxonServerHandle { display_name: axon_connection.id, conn: axon_connection.conn };
    Ok(command_sink)
}

#[tonic::async_trait]
impl CommandSink for AxonServerHandle {
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Pin<Box<Result<(), EncodeError>>> {
        debug!("Sending command: {:?}: {:?}", command_type, self.display_name);
        let mut buf = Vec::new();
        command.encode_u8(&mut buf).unwrap();
        let buffer_length = buf.len();
        debug!("Buffer length: {:?}", buffer_length);
        let serialized_command = SerializedObject {
            r#type: command_type.to_string(),
            revision: "1".to_string(),
            data: buf,
        };
        submit_command(self, &serialized_command).await;
        Box::pin(Ok(()))
    }
}

async fn submit_command(this: &AxonServerHandle, message: &SerializedObject) {
    debug!("Message: {:?}", message);
    let this = this.clone();
    let conn = this.conn;
    let mut client = CommandServiceClient::new(conn);
    debug!("Command Service Client: {:?}", client);
    let uuid = Uuid::new_v4();
    let command = Command {
        message_identifier: format!("{:?}", uuid.to_simple()),
        name: message.r#type.clone(),
        payload: Some(message.clone()),
        client_id: "yyy".to_string(),
        component_name: "RustCommandClient".to_string(),
        meta_data: HashMap::new(),
        processing_instructions: Vec::new(),
        timestamp: 0,
    };
    let response = client.dispatch(command).await.unwrap();
    debug!("Response: {:?}", response);
}
