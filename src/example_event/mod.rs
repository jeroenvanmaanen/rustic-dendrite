use anyhow::{Context,Result};
use log::{debug,error};
use super::elastic_search_utils::wait_for_elastic_search;
use crate::axon_utils::{AxonServerHandle,event_processor};

pub async fn process_events(axon_server_handle : AxonServerHandle) {
    if let Err(e) = internal_process_events(axon_server_handle).await {
        error!("Error while handling commands: {:?}", e);
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_process_events(axon_server_handle : AxonServerHandle) -> Result<()> {
    let client = wait_for_elastic_search().await?;
    debug!("Elastic Search client: {:?}", client);
    event_processor(axon_server_handle).await.context("Error while handling commands")
}
