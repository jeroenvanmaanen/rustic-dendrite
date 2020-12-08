use async_stream::{stream};
use futures_core::stream::{Stream};
use log::{debug,error};
use tonic::{Request};
use uuid::Uuid;
use super::AxonConnection;
use crate::axon_server::FlowControl;
use crate::axon_server::command::{CommandProviderOutbound,CommandSubscription};
use crate::axon_server::command::command_provider_outbound;
use crate::axon_server::command::command_service_client::CommandServiceClient;
use std::alloc::dealloc;

pub async fn command_worker(axon_connection: AxonConnection, commands: &'static[&'static str]) -> Result<(),&str> {
    debug!("Command worker: start");
    let mut client = CommandServiceClient::new(axon_connection.conn);
    let client_id = axon_connection.id.clone();

    let mut command_vec: Vec<String> = vec![];
    command_vec.extend(commands.iter().map(|x| String::from(*x)));
    let command_box = Box::new(command_vec);

    let outbound = create_output_stream(client_id, command_box);

    debug!("Command worker: calling open_stream");
    let result = client.open_stream(Request::new(outbound)).await;
    debug!("Stream result: {:?}", result);

    // match result {
    //     Ok(response) => {
    //         let mut inbound = response.into_inner();
    //         loop {
    //             let message = inbound.message().await;
    //             match message {
    //                 Ok(Some(command)) => {
    //                     debug!("Incoming command: {:?}", command);
    //                 }
    //                 Ok(None) => {
    //                     debug!("None incoming");
    //                 }
    //                 Err(e) => {
    //                     error!("Error from AxonServer: {:?}", e);
    //                     return Ok(()); // return Err(e.code().to_string().into())
    //                 }
    //             }
    //         };
    //     }
    //     Err(e) => {
    //         error!("gRPC error: {:?}", e);
    //         return Ok(()); // return Err(e.code().to_string().into())
    //     }
    // }
    Ok(())
}

fn create_output_stream(client_id: String, command_box: Box<Vec<String>>) -> impl Stream<Item = CommandProviderOutbound> {
    stream! {
        debug!("Command worker: stream: start");
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
            let instruction = CommandProviderOutbound {
                instruction_id: format!("{:?}", instruction_id.to_simple()),
                request: Some(command_provider_outbound::Request::Subscribe(subscription)),
            };
            yield instruction.to_owned();
        }
        debug!("Command worker: stream: send one flow-control permit");
        let flow_control = FlowControl {
            client_id,
            permits: 1,
        };
        let instruction_id = Uuid::new_v4();
        let instruction = CommandProviderOutbound {
            instruction_id: format!("{:?}", instruction_id.to_simple()),
            request: Some(command_provider_outbound::Request::FlowControl(flow_control)),
        };
        yield instruction.to_owned();
        debug!("Command worker: stream: stop");
    }
}
