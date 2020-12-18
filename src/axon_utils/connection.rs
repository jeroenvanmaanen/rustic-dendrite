use log::{debug};
use std::{thread,time};
use std::error::Error;
use tonic::Request;
use tonic::transport::Channel;
use uuid::Uuid;
use super::AxonConnection;
use crate::axon_server::control::{ClientIdentification};
use crate::axon_server::control::platform_service_client::{PlatformServiceClient};

pub async fn wait_for_server(host: &str, port: u32, label: &str) -> Result<AxonConnection, Box<dyn Error>> {
    let url = format!("http://{}:{}", host, port);
    let conn = wait_for_connection(&url, label).await;
    debug!("Connection: {:?}", conn);
    let uuid = Uuid::new_v4();
    let connection = AxonConnection {
        id: format!("{:?}", uuid.to_simple()),
        conn
    };
    Ok(connection)
}

async fn wait_for_connection(url: &str, label: &str) -> Channel {
    let interval = time::Duration::from_secs(1);
    loop {
        match try_to_connect(url, label).await {
            Ok(conn) => return conn,
            Err(e) => debug!(". ({:?})", e)
        }
        thread::sleep(interval);
        continue;
    }
}

async fn try_to_connect(url: &str, label: &str) -> Result<Channel, Box<dyn Error>> {
    let conn = tonic::transport::Endpoint::from_shared(url.to_string())?.connect().await?;
    let mut client = PlatformServiceClient::new(conn.clone());
    let mut client_identification = ClientIdentification::default();
    client_identification.component_name = format!("Rust client {}", &*label);
    let response = client.get_platform_server(Request::new(client_identification)).await?;
    debug!("Response: {:?}", response);
    return Ok(conn);
}