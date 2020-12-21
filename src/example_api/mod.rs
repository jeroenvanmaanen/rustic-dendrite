use anyhow::{Result};
use log::{debug};
use tonic::{Request, Response, Status};
use crate::axon_utils::{init_command_sender, CommandSink, AxonServerHandle};
use crate::grpc_example::greeter_service_server::GreeterService;
use crate::grpc_example::{Acknowledgement, Empty, Greeting, GreetCommand, RecordCommand, StopCommand};

#[derive(Debug)]
pub struct GreeterServer {
    axon_server_handle: AxonServerHandle,
}

#[tonic::async_trait]
impl GreeterService for GreeterServer {
    async fn greet(
        &self,
        request: Request<Greeting>,
    ) -> Result<Response<Acknowledgement>, Status> {
        debug!("Got a greet request: {:?}", request);
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

    async fn record(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        debug!("Got a record request: {:?}", request);

        let command = RecordCommand {
            aggregate_identifier: "xxx".to_string(),
        };

        self.axon_server_handle.send_command("RecordCommand", Box::new(&command)).await.map_err(|e| Status::unknown(e.to_string()))?;

        let reply = Empty { };

        Ok(Response::new(reply))
    }

    async fn stop(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Empty>, Status> {
        debug!("Got a stop request: {:?}", request);

        let command = StopCommand {
            aggregate_identifier: "xxx".to_string(),
        };

        self.axon_server_handle.send_command("StopCommand", Box::new(&command)).await.map_err(|e| Status::unknown(e.to_string()))?;

        let reply = Empty { };

        Ok(Response::new(reply))
    }
}

pub async fn init() -> Result<GreeterServer> {
    init_command_sender().await.map(|command_sink| {GreeterServer{ axon_server_handle: command_sink }})
}