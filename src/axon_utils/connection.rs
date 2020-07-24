use std::error::Error;
use uuid::Uuid;
use super::AxonConnection;

pub async fn wait_for_server() -> Result<AxonConnection, Box<dyn Error>> {
    let uuid = Uuid::new_v4();
    let connection = AxonConnection {
        id: format!("{:?}", uuid.to_simple()),
    };
    Ok(connection)
}