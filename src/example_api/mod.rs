use prost::{EncodeError,Message};
use tonic::{Request, Response, Status};
use crate::grpc_example::greeter_service_server::GreeterService;
use crate::grpc_example::{Acknowledgement, Greeting};
use super::axon_utils::{SendCommand,U8Message,WaitForServer};


#[derive(Debug, Default)]
pub struct GreeterServer {}

impl U8Message for Greeting
where
    Self: Sized
{
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<(),EncodeError> {
        self.encode(buf)
    }
}

#[tonic::async_trait]
impl GreeterService for GreeterServer {
    async fn greet(
        &self,
        request: Request<Greeting>,
    ) -> Result<Response<Acknowledgement>, Status> {
        println!("Got a request: {:?}", request);
        let inner_request = request.into_inner();
        let result_message = inner_request.message.clone();

        let axon_connection = WaitForServer().await.unwrap();
        SendCommand("GreetCommand", Box::new(inner_request), &axon_connection);

        let reply = Acknowledgement {
            message: format!("Hello {}!", result_message).into(),
        };

        Ok(Response::new(reply))
    }
}
