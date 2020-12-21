use anyhow::Result;
use log::debug;
use std::time;
use tokio::time::delay_for;
use tonic;
use tonic::Request;
use tonic::transport::Channel;
use uuid::Uuid;
use super::AxonConnection;
use crate::axon_server::control::ClientIdentification;
use crate::axon_server::control::platform_service_client::PlatformServiceClient;

pub async fn wait_for_server(host: &str, port: u32, label: &str) -> Result<AxonConnection> {
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
        if let Some(conn)= try_to_connect(url, label).await {
            return conn;
        }
        delay_for(interval).await;
        continue;
    }
}

async fn try_to_connect(url: &str, label: &str) -> Option<Channel> {
    connect(url, label).await
        .map_err(|e| {
            debug!("Error while trying to connect to AxonServer: {:?}", e);
        })
        .ok().flatten()
}

async fn connect(url: &str, label: &str) -> Result<Option<Channel>> {
    let conn = tonic::transport::Endpoint::from_shared(url.to_string())?.connect().await
        .map_err(|_| debug!(". Can't connect to AxonServer (yet)"))
        .ok();
    let conn = match conn {
        Some(conn) => conn,
        None => { return Ok(None) },
    };
    let mut client = PlatformServiceClient::new(conn.clone());
    let mut client_identification = ClientIdentification::default();
    client_identification.component_name = format!("Rust client {}", &*label);
    let response = client.get_platform_server(Request::new(client_identification)).await
        .map_err(|_| debug!(". AxonServer is not available (yet)"))
        .ok();
    if response.is_none() {
        return Ok(None);
    }
    debug!("Response: {:?}", response);
    return Ok(Some(conn));
}
