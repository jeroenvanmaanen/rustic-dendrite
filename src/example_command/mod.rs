use anyhow::{Context,Result,anyhow};
use log::{debug,error};
use prost::{Message};
use crate::axon_utils::{EmitApplicableEventsAndResponse, HandlerRegistry, command_worker, create_aggregate_definition, emit_applicable, emit_applicable_events_and_response, empty_handler_registry, wait_for_server, ApplicableTo};
use crate::grpc_example::{Acknowledgement,GreetCommand,GreetedEvent,GreeterProjection,RecordCommand,StopCommand};

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

    let mut old_handler_registry = empty_handler_registry();
    let mut handler_registry = empty_handler_registry();

    handler_registry.insert_with_output(
        "GreetCommand",
        &GreetCommand::decode,
        &(|c, p| Box::pin(handle_greet_command(c, p)))
    )?;

    old_handler_registry.insert(
        "RecordCommand",
        &RecordCommand::decode,
        &(|c, _| Box::pin(handle_record_command(c)))
    )?;

    old_handler_registry.insert(
        "StopCommand",
        &StopCommand::decode,
        &(|c, _| Box::pin(handle_stop_command(c)))
    )?;

    let aggregate_definition = create_aggregate_definition(
        "GreeterProjection".to_string(),
        Box::new(|| {
            let mut projection = GreeterProjection::default();
            projection.is_recording = true;
            projection
        }),
        empty_handler_registry(),
        empty_handler_registry()
    );

    command_worker(axon_connection, old_handler_registry, aggregate_definition).await.context("Error while handling commands")
}

impl ApplicableTo for GreetedEvent {
    type Projection = GreeterProjection;

    fn apply_to(self: &Self, _projection: &mut Self::Projection) -> Result<()> {
        Ok(())
    }

    fn box_clone(self: &Self) -> Box<dyn ApplicableTo<Projection=GreeterProjection>> {
        Box::from(GreetedEvent::clone(self))
    }
}

async fn handle_greet_command (command: GreetCommand, projection: GreeterProjection) -> Result<Option<EmitApplicableEventsAndResponse<GreeterProjection>>> {
    debug!("Greet command handler: {:?}", command);
    if !projection.is_recording {
        debug!("Not recording, so no events emitted nor acknowledgement returned");
        return Ok(None);
    }
    let greeting = command.message;
    let message = greeting.clone().map(|g| g.message).unwrap_or("-/-".to_string());
    if message == "ERROR" {
        return Err(anyhow!("Panicked at reading 'ERROR'"));
    }
    let mut emit_events = emit_applicable_events_and_response("xxx", "Acknowledgement", &Acknowledgement {
        message: format!("ACK! {}", message),
    })?;
    emit_applicable(&mut emit_events, "GreetedEvent", Box::from(GreetedEvent {
        message: greeting,
    }))?;
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
