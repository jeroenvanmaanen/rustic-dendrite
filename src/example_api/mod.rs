use anyhow::{anyhow,Result};
use log::{debug};
use prost::{Message};
use tonic::{Request, Response, Status};
use crate::axon_utils::{init_command_sender, CommandSink, AxonServerHandle, VecU8Message};
use crate::grpc_example::greeter_service_server::GreeterService;
use crate::grpc_example::{Acknowledgement, Greeting, GreetCommand};

#[derive(Debug)]
pub struct GreeterServer {
    axon_server_handle: AxonServerHandle,
}

impl VecU8Message for GreetCommand
    where
        Self: Sized
{
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<()> {
        self.encode(buf).map_err(|e| anyhow!("Prost encode error: {:?}: {:?}", e.required_capacity(), e.remaining()))
    }
}

#[tonic::async_trait]
impl GreeterService for GreeterServer {
    async fn greet(
        &self,
        request: Request<Greeting>,
    ) -> Result<Response<Acknowledgement>, Status> {
        debug!("Got a request: {:?}", request);
        let inner_request = request.into_inner();
        let result_message = inner_request.message.clone();

        let command = GreetCommand {
            aggregate_identifier: "xxx".to_string(),
            message: Some(inner_request),
        };

        self.axon_server_handle.send_command("GreetCommand", Box::new(&command)).await.map_err(|e| Status::unknown(e.to_string()))?;

        let reply = Acknowledgement {
            message: format!("Hello {}!", result_message).into(),
        };

        Ok(Response::new(reply))
    }
}

pub async fn init() -> Result<GreeterServer> {
    init_command_sender().await.map(|command_sink| {GreeterServer{ axon_server_handle: command_sink }})
}