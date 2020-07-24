use log::info;

use tonic::transport::Server;

use rustic_dendrite::example_api::GreeterServer;
use rustic_dendrite::grpc_example::greeter_service_server::GreeterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Rustic dendrite API service started");

    let addr = "0.0.0.0:8181".parse()?;
    let greeter_server = GreeterServer::default();

    Server::builder()
        .add_service(GreeterServiceServer::new(greeter_server))
        .serve(addr)
        .await?;

    Ok(())
}
