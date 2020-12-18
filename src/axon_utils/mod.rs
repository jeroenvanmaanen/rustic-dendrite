use anyhow::{Result};
use tonic::transport::Channel;

mod command_submit;
mod command_worker;
mod connection;

#[derive(Debug, Clone)]
pub struct AxonServerHandle {
    pub display_name: String,
    pub conn: Channel,
}

#[derive(Debug)]
pub struct AxonConnection {
    pub id: String,
    pub conn: Channel,
}

pub trait VecU8Message {
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<()>;
}

#[tonic::async_trait]
pub trait CommandSink {
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Result<()>;
}

pub use command_submit::init as init_command_sender;
pub use connection::wait_for_server as wait_for_server;
pub use command_worker::command_worker as command_worker;
