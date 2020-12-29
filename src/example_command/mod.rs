use anyhow::{Context,Result,anyhow};
use log::{debug,error};
use prost::{Message};
use crate::axon_utils::{EmitApplicableEventsAndResponse, HandlerRegistry, TheHandlerRegistry, command_worker, create_aggregate_definition, emit_applicable, emit_applicable_events_and_response, empty_handler_registry, wait_for_server, ApplicableTo};
use crate::grpc_example::{Acknowledgement,GreetCommand,GreetedEvent,GreeterProjection,RecordCommand,StartedRecordingEvent,StopCommand,StoppedRecordingEvent};

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

    let mut sourcing_registry: TheHandlerRegistry<GreeterProjection,GreeterProjection>  = empty_handler_registry();
    let mut handler_registry = empty_handler_registry();

    sourcing_registry.insert_with_output(
        "GreetedEvent",
        &GreetedEvent::decode,
        &(|c, p| Box::pin(handle_sourcing_event(Box::from(c), p)))
    )?;

    sourcing_registry.insert_with_output(
        "StoppedRecordingEvent",
        &StoppedRecordingEvent::decode,
        &(|c, p| Box::pin(handle_sourcing_event(Box::from(c), p)))
    )?;

    sourcing_registry.insert_with_output(
        "StartedRecordingEvent",
        &StartedRecordingEvent::decode,
        &(|c, p| Box::pin(handle_sourcing_event(Box::from(c), p)))
    )?;

    handler_registry.insert_with_output(
        "GreetCommand",
        &GreetCommand::decode,
        &(|c, p| Box::pin(handle_greet_command(c, p)))
    )?;

    handler_registry.insert_with_output(
        "RecordCommand",
        &RecordCommand::decode,
        &(|c, p| Box::pin(handle_record_command(c, p)))
    )?;

    handler_registry.insert_with_output(
        "StopCommand",
        &StopCommand::decode,
        &(|c, p| Box::pin(handle_stop_command(c, p)))
    )?;

    let aggregate_definition = create_aggregate_definition(
        "GreeterProjection".to_string(),
        Box::new(|| {
            let mut projection = GreeterProjection::default();
            projection.is_recording = true;
            projection
        }),
        handler_registry,
        sourcing_registry
    );

    command_worker(axon_connection, aggregate_definition).await.context("Error while handling commands")
}

async fn handle_sourcing_event<T: ApplicableTo<Projection=P>,P: Clone>(event: Box<T>, projection: P) -> Result<Option<P>> {
    let mut p = projection.clone();
    event.apply_to(&mut p)?;
    Ok(Some(p))
}

impl ApplicableTo for GreetedEvent {
    type Projection = GreeterProjection;

    fn apply_to(self: &Self, projection: &mut Self::Projection) -> Result<()> {
        debug!("Apply greeted event to GreeterProjection: {:?}", projection.is_recording);
        Ok(())
    }

    fn box_clone(self: &Self) -> Box<dyn ApplicableTo<Projection=GreeterProjection>> {
        Box::from(GreetedEvent::clone(self))
    }
}

impl ApplicableTo for StartedRecordingEvent {
    type Projection = GreeterProjection;

    fn apply_to(self: &Self, projection: &mut Self::Projection) -> Result<()> {
        debug!("Apply StartedRecordingEvent to GreeterProjection: {:?}", projection.is_recording);
        projection.is_recording = true;
        Ok(())
    }

    fn box_clone(self: &Self) -> Box<dyn ApplicableTo<Projection=GreeterProjection>> {
        Box::from(StartedRecordingEvent::clone(self))
    }
}

impl ApplicableTo for StoppedRecordingEvent {
    type Projection = GreeterProjection;

    fn apply_to(self: &Self, projection: &mut Self::Projection) -> Result<()> {
        debug!("Apply StoppedRecordingEvent to GreeterProjection: {:?}", projection.is_recording);
        projection.is_recording = false;
        Ok(())
    }

    fn box_clone(self: &Self) -> Box<dyn ApplicableTo<Projection=GreeterProjection>> {
        Box::from(StoppedRecordingEvent::clone(self))
    }
}

async fn handle_greet_command (command: GreetCommand, projection: GreeterProjection) -> Result<Option<EmitApplicableEventsAndResponse<GreeterProjection>>> {
    debug!("Greet command handler: {:?}", command);
    if !projection.is_recording {
        debug!("Not recording, so no events emitted nor acknowledgement returned");
        return Ok(None);
    }
    debug!("Recording, so proceed");
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

async fn handle_record_command (command: RecordCommand, projection: GreeterProjection) -> Result<Option<EmitApplicableEventsAndResponse<GreeterProjection>>> {
    debug!("Record command handler: {:?}", command);
    if projection.is_recording {
        return Ok(None)
    }
    let mut emit_events = emit_applicable_events_and_response("xxx", "Acknowledgement", &())?;
    emit_applicable(&mut emit_events, "StartedRecordingEvent", Box::from(StartedRecordingEvent {}))?;
    Ok(Some(emit_events))
}

async fn handle_stop_command (command: StopCommand, projection: GreeterProjection) -> Result<Option<EmitApplicableEventsAndResponse<GreeterProjection>>> {
    debug!("Stop command handler: {:?}", command);
    if !projection.is_recording {
        return Ok(None)
    }
    let mut emit_events = emit_applicable_events_and_response("xxx", "Acknowledgement", &())?;
    emit_applicable(&mut emit_events, "StoppedRecordingEvent", Box::from(StoppedRecordingEvent {}))?;
    Ok(Some(emit_events))
}
