use anyhow::{Context,Result};
use log::{debug,error};
use prost::{Message};
use crate::axon_utils::{command_worker, wait_for_server, empty_handler_registry, HandlerRegistry};
use crate::grpc_example::{GreetCommand};

pub async fn handle_commands() {
    if let Err(e) = internal_handle_commands().await {
        error!("Error while handling commands: {:?}", e);
    }
    debug!("Stopped handling commands for example application");
}

async fn internal_handle_commands() -> Result<()> {
    debug!("Handle commands for example application");
    let axon_connection = wait_for_server("proxy", 8124, "Command Processor").await.context("No connection")?;
    debug!("Axon connection: {:?}", axon_connection);

    let mut handler_registry = empty_handler_registry();
    handler_registry.insert(
        "GreetCommand".to_string(),
        &GreetCommand::decode,
        &(|c| Box::pin(handle_greet_command(c)))
    )?;

    command_worker(axon_connection, handler_registry).await.context("Error while handling commands")?;

    Ok(())
}

async fn handle_greet_command (command: GreetCommand) -> Result<()> {
    debug!("Greet command handler: {:?}", command);
    Ok(())
}