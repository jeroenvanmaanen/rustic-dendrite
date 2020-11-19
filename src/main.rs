use std::error::Error;
use log::info;

use tonic::transport::Server;

use rustic_dendrite::example_api::init;
use rustic_dendrite::example_command::handle_commands;
use rustic_dendrite::grpc_example::greeter_service_server::GreeterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Rustic dendrite API service started");

    tokio::spawn(handle_commands());

    let addr = "0.0.0.0:8181".parse()?;
    let greeter_server = init().await.unwrap();
    Server::builder()
        .add_service(GreeterServiceServer::new(greeter_server))
        .serve(addr)
        .await?;

    Ok(())
}
