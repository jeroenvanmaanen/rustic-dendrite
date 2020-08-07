use super::{CommandSink, CommandSinkHandle, WaitForServer, VecU8Message};
use log::{debug};
use prost::EncodeError;

pub async fn init() -> Result<CommandSinkHandle, Box<dyn std::error::Error>>{
    let axon_connection = WaitForServer("proxy", 8124, "API").await.unwrap();
    debug!("Axon connection: {:?}", axon_connection);
    let command_sink = CommandSinkHandle { display_name: axon_connection.id };
    Ok(command_sink)
}

impl CommandSink for CommandSinkHandle {
    fn send_command(&self, command_type: &str, command: Box<dyn VecU8Message>) -> Result<(), EncodeError> {
        debug!("Sending command: {:?}: {:?}", command_type, self.display_name);
        let mut buf = Vec::new();
        command.encode_u8(&mut buf).unwrap();
        let buffer_length = buf.len();
        debug!("Buffer length: {:?}", buffer_length);
        Ok(())
    }
}