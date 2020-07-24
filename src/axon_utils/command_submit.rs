use super::AxonConnection;
use super::U8Message;
use log::{debug};

pub fn send_command(command_type: &str, command: Box<dyn U8Message>, connection: &AxonConnection) {
    debug!("Sending command: {:?}: {:?}", command_type, connection);
    let mut buf = Vec::new();
    command.encode_u8(&mut buf).unwrap();
    let buffer_length = buf.len();
    debug!("Buffer length: {:?}", buffer_length);
}