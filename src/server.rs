use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

use hello_world::greeter_service_server::{GreeterService, GreeterServiceServer};
use hello_world::{Acknowledgement,Empty,Greeting};

pub mod hello_world {
    tonic::include_proto!("hello_world"); // The string specified here must match the proto package name
    tonic::include_proto!("grpc_example");
}
#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> { // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

#[tonic::async_trait]
impl GreeterService for MyGreeter {
    async fn greet(
        &self,
        request: Request<Greeting>,
    ) -> Result<Response<Acknowledgement>, Status> {
        println!("Got a request: {:?}", request);

        let reply = hello_world::Acknowledgement {
            message: format!("Hello {}!", request.into_inner().message).into(),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let greeter = MyGreeter::default();
    let greeter_service = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .add_service(GreeterServiceServer::new(greeter_service))
        .serve(addr)
        .await?;

    Ok(())
}
