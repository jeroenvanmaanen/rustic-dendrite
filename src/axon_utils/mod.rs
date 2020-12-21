use anyhow::{anyhow,Result};
use log::debug;
use prost::Message;
use tonic::transport::Channel;

use crate::axon_server::SerializedObject;

mod command_submit;
mod command_worker;
mod connection;
mod handler_registry;

pub use command_submit::init as init_command_sender;
pub use command_worker::command_worker as command_worker;
pub use connection::wait_for_server as wait_for_server;
pub use handler_registry::empty_handler_registry as empty_handler_registry;
pub use handler_registry::HandlerRegistry as HandlerRegistry;

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

impl<T> VecU8Message for T
where T: Message + Sized
{
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<()> {
        self.encode(buf).map_err(|e| anyhow!("Prost encode error: {:?}: {:?}", e.required_capacity(), e.remaining()))
    }
}

#[tonic::async_trait]
pub trait CommandSink {
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Result<Option<SerializedObject>>;
}

pub fn axon_serialize<T: Message>(type_name: &str, message: &T) -> Result<SerializedObject> {
    let mut buf = Vec::new();
    message.encode(& mut buf)?;
    let result = SerializedObject {
        r#type: type_name.to_string(),
        revision: "".to_string(),
        data: buf,
    };
    debug!("Encoded output: {:?}", result);
    Ok(result)
}