use anyhow::{Context,Result};
use log::{debug,error};
use crate::axon_utils::{command_worker,wait_for_server};

pub async fn handle_commands() {
    match internal_handle_commands().await {
        Err(e) => {
            error!("Error while handling commands: {:?}", e);
        }
        _ => {}
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_handle_commands() -> Result<()> {
    debug!("Handle commands for example application");
    let axon_connection = wait_for_server("proxy", 8124, "Command Processor").await.context("No connection")?;
    debug!("Axon connection: {:?}", axon_connection);
    command_worker(axon_connection, &["GreetCommand"]).await.context("Error while handling commands")?;
    Ok(())
}