use async_stream::{stream};
use futures_core::stream::{Stream};
use log::{debug,error};
use std::collections::HashMap;
use tokio::sync::mpsc::{Sender,Receiver, channel};
use tonic::{Request};
use uuid::Uuid;
use super::AxonConnection;
use crate::{axon_server};
use crate::axon_server::FlowControl;
use crate::axon_server::command::{CommandProviderOutbound,CommandResponse,CommandSubscription};
use crate::axon_server::command::command_provider_outbound;
use crate::axon_server::command::command_service_client::CommandServiceClient;

pub async fn command_worker(axon_connection: AxonConnection, commands: &[&str]) -> Result<(),String> {
    debug!("Command worker: start");
    let mut client = CommandServiceClient::new(axon_connection.conn);
    let client_id = axon_connection.id.clone();

    let mut command_vec: Vec<String> = vec![];
    command_vec.extend(commands.iter().map(|x| String::from(*x)));
    let command_box = Box::new(command_vec);

    let (mut tx, rx): (Sender<String>, Receiver<String>) = channel(10);

    let outbound = create_output_stream(client_id, command_box, rx);

    debug!("Command worker: calling open_stream");
    let result = client.open_stream(Request::new(outbound)).await;
    debug!("Stream result: {:?}", result);

    match result {
        Ok(response) => {
            let mut inbound = response.into_inner();
            loop {
                let message = inbound.message().await;
                match message {
                    Ok(Some(inbound)) => {
                        debug!("Inbound message: {:?}", inbound);
                        if let Some(axon_server::command::command_provider_inbound::Request::Command(command)) = inbound.request {
                            debug!("Incoming command: {:?}", command);
                            debug!("Do something useful ;-)");
                            tx.send(command.message_identifier).await.unwrap();
                        }
                    }
                    Ok(None) => {
                        debug!("None incoming");
                    }
                    Err(e) => {
                        error!("Error from AxonServer: {:?}", e);
                        return Err(e.code().to_string());
                    }
                }
            };
        }
        Err(e) => {
            error!("gRPC error: {:?}", e);
            return Err(e.code().to_string());
        }
    }
}

fn create_output_stream(client_id: String, command_box: Box<Vec<String>>, mut rx: Receiver<String>) -> impl Stream<Item = CommandProviderOutbound> {
    stream! {
        debug!("Command worker: stream: start: {:?}", rx);
        for command_name in command_box.iter() {
            debug!("Command worker: stream: subscribe to command type: {:?}", command_name);
            let subscription_id = Uuid::new_v4();
            let subscription = CommandSubscription {
                message_id: format!("{:?}", subscription_id.to_simple()),
                command: command_name.to_string().clone(),
                client_id: client_id.clone(),
                component_name: client_id.clone(),
                load_factor: 100,
            };
            debug!("Subscribe command: Subscription: {:?}", subscription);
            let instruction_id = Uuid::new_v4();
            debug!("Subscribe command: Instruction ID: {:?}", instruction_id);
            let instruction = CommandProviderOutbound {
                instruction_id: format!("{:?}", instruction_id.to_simple()),
                request: Some(command_provider_outbound::Request::Subscribe(subscription)),
            };
            yield instruction.to_owned();
        }

        let permits_batch_size: i64 = 3;
        let mut permits = permits_batch_size * 2;
        debug!("Command worker: stream: send initial flow-control permits: amount: {:?}", permits);
        let flow_control = FlowControl {
            client_id: client_id.clone(),
            permits,
        };
        let instruction_id = Uuid::new_v4();
        let instruction = CommandProviderOutbound {
            instruction_id: format!("{:?}", instruction_id.to_simple()),
            request: Some(command_provider_outbound::Request::FlowControl(flow_control)),
        };
        yield instruction.to_owned();

        while let Some(command_id) = rx.recv().await {
            debug!("Send command response: {:?}", command_id);
            let response_id = Uuid::new_v4();
            let response = CommandResponse {
                message_identifier: format!("{:?}", response_id.to_simple()),
                request_identifier: command_id.clone(),
                payload: None,
                error_code: "".to_string(),
                error_message: None,
                meta_data: HashMap::new(),
                processing_instructions: Vec::new(),
            };
            let instruction_id = Uuid::new_v4();
            let instruction = CommandProviderOutbound {
                instruction_id: format!("{:?}", instruction_id.to_simple()),
                request: Some(command_provider_outbound::Request::CommandResponse(response)),
            };
            yield instruction.to_owned();
            permits -= 1;
            if permits <= permits_batch_size {
                debug!("Command worker: stream: send more flow-control permits: amount: {:?}", permits_batch_size);
                let flow_control = FlowControl {
                    client_id: client_id.clone(),
                    permits: permits_batch_size,
                };
                let instruction_id = Uuid::new_v4();
                let instruction = CommandProviderOutbound {
                    instruction_id: format!("{:?}", instruction_id.to_simple()),
                    request: Some(command_provider_outbound::Request::FlowControl(flow_control)),
                };
                yield instruction.to_owned();
                permits += permits_batch_size;
            }
            debug!("Command worker: stream: flow-control permits: balance: {:?}", permits);
        }

        // debug!("Command worker: stream: stop");
    }
}
