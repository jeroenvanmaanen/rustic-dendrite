use log::{debug};
use std::{thread,time};
use std::error::Error;
use tonic::Request;
use tonic::transport::Channel;
use uuid::Uuid;
use super::AxonConnection;
use crate::axon_server::control::{ClientIdentification};
use crate::axon_server::control::platform_service_client::{PlatformServiceClient};

pub async fn wait_for_server() -> Result<AxonConnection, Box<dyn Error>> {
    let client = wait_for_connection().await;
    debug!("PlatformServiceClient: {:?}", client);
    let uuid = Uuid::new_v4();
    let connection = AxonConnection {
        id: format!("{:?}", uuid.to_simple()),
    };
    Ok(connection)
}

async fn wait_for_connection() -> PlatformServiceClient<Channel> {
    let interval = time::Duration::from_secs(1);
    loop {
        let client = PlatformServiceClient::connect("http://proxy:8124").await;
        let mut client = match client {
            Ok(client) => client,
            Err(_) => {
                debug!(".");
                thread::sleep(interval);
                continue;
            }
        };
        let mut client_identification = ClientIdentification::default();
        client_identification.component_name = "Xyz".to_string();
        debug!("Client identification: {:?}", client_identification);
        let response = client.get_platform_server(Request::new(client_identification)).await;
        let response = match response {
            Ok(client) => client,
            Err(e) => {
                debug!(".({:?})", e);
                thread::sleep(interval);
                continue;
            }
        };
        debug!("Response: {:?}", response);
        return client;
    }
}