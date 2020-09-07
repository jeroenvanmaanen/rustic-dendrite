use std::pin::Pin;

use prost::EncodeError;
use tonic::transport::Channel;

mod command_submit;
mod connection;

#[derive(Debug, Clone)]
pub struct CommandSinkHandle {
    display_name: String,
    pub conn: Channel,
}

#[derive(Debug)]
pub struct AxonConnection {
    pub id: String,
    pub conn: Channel,
}

pub trait VecU8Message {
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<(),EncodeError>;
}

#[tonic::async_trait]
pub trait CommandSink {
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Pin<Box<Result<(),EncodeError>>>;
}

pub use command_submit::init as InitCommandSender;
pub use connection::wait_for_server as WaitForServer;
