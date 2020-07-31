use log::{debug};
use std::error::Error;
use uuid::Uuid;
use super::AxonConnection;
use crate::axon_server::control::{ClientIdentification};

pub async fn wait_for_server() -> Result<AxonConnection, Box<dyn Error>> {
    let client_identification = ClientIdentification::default();
    debug!("Client identification: {:?}", client_identification);
    let uuid = Uuid::new_v4();
    let connection = AxonConnection {
        id: format!("{:?}", uuid.to_simple()),
    };
    Ok(connection)
}