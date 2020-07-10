use tonic::transport::Server;

use rustic_dendrite::example_api::MyGreeter;
use rustic_dendrite::grpc_example::greeter_service_server::GreeterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:8181".parse()?;
    let greeter_service = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServiceServer::new(greeter_service))
        .serve(addr)
        .await?;

    Ok(())
}
