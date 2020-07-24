use prost::EncodeError;
mod command_submit;
mod connection;
// mod pool;

#[derive(Debug)]
pub struct AxonConnection {
    id : String,
}

pub trait U8Message {
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<(),EncodeError>;
}

pub use command_submit::send_command as SendCommand;
pub use connection::wait_for_server as WaitForServer;
// pub use pool::AxonServerConnectionManager;
