use prost::EncodeError;
mod command_submit;
mod connection;
// mod pool;

#[derive(Debug)]
pub struct CommandSinkHandle {
    display_name: String,
}

#[derive(Debug)]
pub struct AxonConnection {
    pub id: String,
}

pub trait VecU8Message {
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<(),EncodeError>;
}

pub trait CommandSink {
    fn send_command(&self, command_type: &str, command: Box<dyn VecU8Message>) -> Result<(),EncodeError>;
}

pub use command_submit::init as InitCommandSender;
pub use connection::wait_for_server as WaitForServer;
// pub use pool::AxonServerConnectionManager;
