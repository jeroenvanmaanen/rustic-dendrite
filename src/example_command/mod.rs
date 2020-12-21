use anyhow::{Context,Result,anyhow};
use log::{debug,error};
use prost::{Message};
use crate::axon_utils::{HandlerRegistry, axon_serialize, command_worker, empty_handler_registry, wait_for_server};
use crate::grpc_example::{Acknowledgement,GreetCommand,RecordCommand,StopCommand};

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

    handler_registry.insert_with_output(
        "GreetCommand",
        &GreetCommand::decode,
        &(|c| Box::pin(handle_greet_command(c))),
        "Acknowledgement",
        &axon_serialize
    )?;

    handler_registry.insert(
        "RecordCommand",
        &RecordCommand::decode,
        &(|c| Box::pin(handle_record_command(c)))
    )?;

    handler_registry.insert(
        "StopCommand",
        &StopCommand::decode,
        &(|c| Box::pin(handle_stop_command(c)))
    )?;

    command_worker(axon_connection, handler_registry).await.context("Error while handling commands")
}

async fn handle_greet_command (command: GreetCommand) -> Result<Option<Acknowledgement>> {
    debug!("Greet command handler: {:?}", command);
    let message = command.message.map(|g| g.message).unwrap_or("-/-".to_string());
    if message == "ERROR" {
        return Err(anyhow!("Panicked at reading 'ERROR'"));
    }
    Ok(Some(Acknowledgement {
        message: format!("ACK! {}", message),
    }))
}

async fn handle_record_command (command: RecordCommand) -> Result<()> {
    debug!("Record command handler: {:?}", command);
    Ok(())
}

async fn handle_stop_command (command: StopCommand) -> Result<()> {
    debug!("Stop command handler: {:?}", command);
    Ok(())
}
