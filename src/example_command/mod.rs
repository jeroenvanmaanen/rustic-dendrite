use anyhow::{Context,Result,anyhow};
use log::{debug,error};
use prost::{Message};
use crate::axon_utils::{EmitEventsAndResponse, HandlerRegistry, command_worker, emit, emit_events_and_response, empty_handler_registry, wait_for_server};
use crate::grpc_example::{Acknowledgement,GreetCommand,GreetedEvent,RecordCommand,StopCommand};

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
        &(|c| Box::pin(handle_greet_command(c)))
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

async fn handle_greet_command (command: GreetCommand) -> Result<Option<EmitEventsAndResponse>> {
    debug!("Greet command handler: {:?}", command);
    let greeting = command.message;
    let message = greeting.clone().map(|g| g.message).unwrap_or("-/-".to_string());
    if message == "ERROR" {
        return Err(anyhow!("Panicked at reading 'ERROR'"));
    }
    let mut emit_events = emit_events_and_response("xxx", "Acknowledgement", &Acknowledgement {
        message: format!("ACK! {}", message),
    })?;
    emit(&mut emit_events, "GreetedEvent", &GreetedEvent {
        message: greeting,
    })?;
    debug!("Emit events and response: {:?}", emit_events);
    Ok(Some(emit_events))
}

async fn handle_record_command (command: RecordCommand) -> Result<()> {
    debug!("Record command handler: {:?}", command);
    Ok(())
}

async fn handle_stop_command (command: StopCommand) -> Result<()> {
    debug!("Stop command handler: {:?}", command);
    Ok(())
}
