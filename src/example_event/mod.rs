use anyhow::{Context,Result};
use log::{debug,error};
use crate::axon_utils::{AxonServerHandle,event_processor};

pub async fn process_events(axon_server_handle : AxonServerHandle) {
    if let Err(e) = internal_process_events(axon_server_handle).await {
        error!("Error while handling commands: {:?}", e);
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_process_events(axon_server_handle : AxonServerHandle) -> Result<()> {
    event_processor(axon_server_handle).await.context("Error while handling commands")
}
