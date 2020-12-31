use std::error::Error;
use log::info;

use tonic::transport::Server;

use rustic_dendrite::example_api::init;
use rustic_dendrite::example_command::handle_commands;
use rustic_dendrite::example_event::process_events;
use rustic_dendrite::grpc_example::greeter_service_server::GreeterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Rustic dendrite API service started");

    let greeter_server = init().await.unwrap();

    tokio::spawn(handle_commands(greeter_server.axon_server_handle.clone()));

    tokio::spawn(process_events(greeter_server.axon_server_handle.clone()));

    let addr = "0.0.0.0:8181".parse()?;
    info!("Starting gRPC server");
    Server::builder()
        .add_service(GreeterServiceServer::new(greeter_server))
        .serve(addr)
        .await?;

    Ok(())
}
