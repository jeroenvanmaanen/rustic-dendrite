use tonic::{Request, Response, Status};
use crate::grpc_example::greeter_service_server::GreeterService;
use crate::grpc_example::{Acknowledgement, Greeting};

#[derive(Debug, Default)]
pub struct GreeterServer {}

#[tonic::async_trait]
impl GreeterService for GreeterServer {
    async fn greet(
        &self,
        request: Request<Greeting>,
    ) -> Result<Response<Acknowledgement>, Status> {
        println!("Got a request: {:?}", request);

        let reply = Acknowledgement {
            message: format!("Hello {}!", request.into_inner().message).into(),
        };

        Ok(Response::new(reply))
    }
}
