use anyhow::{anyhow,Result};
use async_stream::stream;
use futures_core::stream::Stream;
use log::{debug,error,warn};
use prost::Message;
use std::collections::HashMap;
use tokio::sync::mpsc::{Sender,Receiver, channel};
use tonic::Request;
use tonic::transport::Channel;
use uuid::Uuid;
use super::{ApplicableTo, AxonConnection, VecU8Message, axon_serialize};
use super::event_query::query_events_from_client;
use super::handler_registry::{HandlerRegistry,TheHandlerRegistry};
use crate::axon_server::{ErrorMessage,FlowControl,SerializedObject};
use crate::axon_server::command::{CommandProviderOutbound,CommandResponse,CommandSubscription};
use crate::axon_server::command::{command_provider_inbound,Command};
use crate::axon_server::command::command_provider_outbound;
use crate::axon_server::command::command_service_client::CommandServiceClient;
use crate::axon_server::event::{Event,ReadHighestSequenceNrRequest};
use crate::axon_server::event::event_store_client::EventStoreClient;
use std::fmt::Debug;

pub fn emit_events(target_aggregate_identifier: &str) -> EmitEventsAndResponse {
    EmitEventsAndResponse {
        target_aggregate_identifier: target_aggregate_identifier.to_string(),
        events: Vec::new(),
        response: None,
    }
}

pub fn emit_events_and_response<T: Message>(
    target_aggregate_identifier: &str,
    type_name: &str,
    response: &T
) -> Result<EmitEventsAndResponse> {
    let payload = axon_serialize(type_name, response)?;
    Ok(EmitEventsAndResponse {
        target_aggregate_identifier: target_aggregate_identifier.to_string(),
        events: Vec::new(),
        response: Some(payload),
    })
}

pub fn emit_applicable_events_and_response<T: Message, P: VecU8Message + Send + Clone>(
    target_aggregate_identifier: &str,
    type_name: &str,
    response: &T
) -> Result<EmitApplicableEventsAndResponse<P>> {
    let payload = axon_serialize(type_name, response)?;
    Ok(EmitApplicableEventsAndResponse {
        target_aggregate_identifier: target_aggregate_identifier.to_string(),
        events: Vec::new(),
        response: Some(payload),
    })
}

#[derive(Clone,Debug)]
pub struct EmitEventsAndResponse {
    target_aggregate_identifier: String,
    events: Vec<SerializedObject>,
    response: Option<SerializedObject>,
}

#[derive(Debug)]
pub struct EmitApplicableEventsAndResponse<P> {
    target_aggregate_identifier: String,
    events: Vec<(String,Box<dyn ApplicableTo<Projection=P>>)>,
    response: Option<SerializedObject>,
}

impl<P> Clone for EmitApplicableEventsAndResponse<P> {
    fn clone(&self) -> Self {
        EmitApplicableEventsAndResponse {
            target_aggregate_identifier: self.target_aggregate_identifier.clone(),
            events: self.events.iter().map(|(n,b)| (n.clone(), b.box_clone())).collect(),
            response: self.response.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.target_aggregate_identifier = source.target_aggregate_identifier.clone();
        self.events = source.events.iter().map(|(n, b)| (n.clone(), b.box_clone())).collect();
        self.response = source.response.clone();
    }
}

pub struct AggregateDefinition<P: VecU8Message + Send + Clone + 'static> {
    projection_name: String,
    empty_projection: Box<dyn Fn() -> P + Send + Sync>,
    command_handler_registry: TheHandlerRegistry<P,EmitApplicableEventsAndResponse<P>>,
    sourcing_handler_registry: TheHandlerRegistry<P,P>,
}

pub fn create_aggregate_definition<P: VecU8Message + Send + Clone>(
    projection_name: String,
    empty_projection: Box<dyn Fn() -> P + Send + Sync>,
    command_handler_registry: TheHandlerRegistry<P,EmitApplicableEventsAndResponse<P>>,
    sourcing_handler_registry: TheHandlerRegistry<P,P>
) -> AggregateDefinition<P>{
    AggregateDefinition {
        projection_name, empty_projection, command_handler_registry, sourcing_handler_registry,
    }
}

async fn handle_command<P: VecU8Message + Send + Clone + std::fmt::Debug + 'static>(
    command: &Command,
    aggregate_definition: &AggregateDefinition<P>,
    client: &mut EventStoreClient<Channel>
) -> Result<Option<EmitApplicableEventsAndResponse<P>>> {
    debug!("Incoming command: {:?}", command);
    let data = command.payload.clone().map(|p| p.data).ok_or(anyhow!("No payload data for: {:?}", command.name))?;
    let handler = aggregate_definition.command_handler_registry.get(&command.name).ok_or(anyhow!("No handler for: {:?}", command.name))?;
    let mut projection = (aggregate_definition.empty_projection)();
    let events = query_events_from_client(client, "xxx").await?;
    for event in events {
        debug!("Replaying event: {:?}", event);
        if let Some(payload) = event.payload {
            let sourcing_handler = aggregate_definition.sourcing_handler_registry.get(&payload.r#type).ok_or(anyhow!("Missing sourcing handler for {:?}", payload.r#type))?;
            if let Some(p) = (sourcing_handler).handle(payload.data, projection.clone()).await? {
                projection = p;
            }
        }
    }
    debug!("Restored projection: {:?}", projection);
    Ok(handler.handle(data, projection).await?)
}

pub fn emit<T: Message>(holder: &mut EmitEventsAndResponse, type_name: &str, event: &T) -> Result<()> {
    let payload = axon_serialize(type_name, event)?;
    holder.events.push(payload);
    Ok(())
}

pub fn emit_applicable<P: VecU8Message + Send + Clone>(holder: &mut EmitApplicableEventsAndResponse<P>, type_name: &str, event: Box<dyn ApplicableTo<Projection=P>>) -> Result<()> {
    holder.events.push((type_name.to_string(), event));
    Ok(())
}

#[derive(Debug)]
struct AxonCommandResult {
    message_identifier: String,
    result: Result<Option<EmitEventsAndResponse>>,
}

pub async fn command_worker<P: VecU8Message + Send + Clone + std::fmt::Debug + 'static>(
    axon_connection: AxonConnection,
    aggregate_definition: AggregateDefinition<P>
) -> Result<()> {
    debug!("Command worker: start");

    debug!("Projection name: {:?}", aggregate_definition.projection_name);
    let empty_projection = (aggregate_definition.empty_projection)();
    debug!("Empty projection: {:?}", empty_projection);

    let axon_connection_clone = axon_connection.clone();
    let mut client = CommandServiceClient::new(axon_connection.conn);
    let mut event_store_client = EventStoreClient::new(axon_connection_clone.conn);
    let client_id = axon_connection.id.clone();

    let mut command_vec: Vec<String> = vec![];
    command_vec.extend(aggregate_definition.command_handler_registry.handlers.keys().map(|x| x.clone()));
    let command_box = Box::new(command_vec);

    let (mut tx, rx): (Sender<AxonCommandResult>, Receiver<AxonCommandResult>) = channel(10);

    let outbound = create_output_stream(client_id, command_box, rx);

    debug!("Command worker: calling open_stream");
    let response = client.open_stream(Request::new(outbound)).await?;
    debug!("Stream response: {:?}", response);

    let mut inbound = response.into_inner();
    loop {
        match inbound.message().await {
            Ok(Some(inbound)) => {
                debug!("Inbound message: {:?}", inbound);
                if let Some(command_provider_inbound::Request::Command(command)) = inbound.request {
                    let result = handle_command(&command, &aggregate_definition, &mut event_store_client).await;
                    match result.as_ref() {
                        Err(e) => warn!("Error while handling command: {:?}", e),
                        Ok(result) => debug!("Result from command handler: {:?}", result),
                    }

                    let result = match result {
                        Ok(Some(result)) => { Some(result) }
                        _ => None
                    };

                    if let Some(result) = result.as_ref() {
                        debug!("Emit events: {:?}", &result.events);
                        store_events(&mut event_store_client, &result).await.unwrap(); // TODO: error handling
                    }

                    let wrapped_result = result.map(
                        |r| EmitEventsAndResponse {
                            target_aggregate_identifier: r.target_aggregate_identifier,
                            events: vec![],
                            response: r.response
                        }
                    );

                    let axon_command_result = AxonCommandResult {
                        message_identifier: command.message_identifier,
                        result: Ok(wrapped_result)
                    };
                    tx.send(axon_command_result).await.unwrap();
                }
            }
            Ok(None) => {
                debug!("None incoming");
            }
            Err(e) => {
                error!("Error from AxonServer: {:?}", e);
                return Err(anyhow!(e.code()));
            }
        }
    }
}

fn create_output_stream(client_id: String, command_box: Box<Vec<String>>, mut rx: Receiver<AxonCommandResult>) -> impl Stream<Item = CommandProviderOutbound> {
    stream! {
        debug!("Command worker: stream: start: {:?}", rx);
        for command_name in command_box.iter() {
            debug!("Command worker: stream: subscribe to command type: {:?}", command_name);
            let subscription_id = Uuid::new_v4();
            let subscription = CommandSubscription {
                message_id: format!("{:?}", subscription_id.to_simple()),
                command: command_name.to_string().clone(),
                client_id: client_id.clone(),
                component_name: client_id.clone(),
                load_factor: 100,
            };
            debug!("Subscribe command: Subscription: {:?}", subscription);
            let instruction_id = Uuid::new_v4();
            debug!("Subscribe command: Instruction ID: {:?}", instruction_id);
            let instruction = CommandProviderOutbound {
                instruction_id: format!("{:?}", instruction_id.to_simple()),
                request: Some(command_provider_outbound::Request::Subscribe(subscription)),
            };
            yield instruction.to_owned();
        }

        let permits_batch_size: i64 = 3;
        let mut permits = permits_batch_size * 2;
        debug!("Command worker: stream: send initial flow-control permits: amount: {:?}", permits);
        let flow_control = FlowControl {
            client_id: client_id.clone(),
            permits,
        };
        let instruction_id = Uuid::new_v4();
        let instruction = CommandProviderOutbound {
            instruction_id: format!("{:?}", instruction_id.to_simple()),
            request: Some(command_provider_outbound::Request::FlowControl(flow_control)),
        };
        yield instruction.to_owned();

        while let Some(axon_command_result) = rx.recv().await {
            debug!("Send command response: {:?}", axon_command_result);
            let response_id = Uuid::new_v4();
            let mut response = CommandResponse {
                message_identifier: format!("{:?}", response_id.to_simple()),
                request_identifier: axon_command_result.message_identifier.clone(),
                payload: None,
                error_code: "".to_string(),
                error_message: None,
                meta_data: HashMap::new(),
                processing_instructions: Vec::new(),
            };
            match axon_command_result.result {
                Ok(result) => {
                    response.payload = result.map(|r| r.response).flatten();
                }
                Err(e) => {
                    response.error_code = "ERROR".to_string();
                    response.error_message = Some(ErrorMessage {
                        message: e.to_string(),
                        location: "".to_string(),
                        details: Vec::new(),
                        error_code: "ERROR".to_string(),
                    });
                }
            }
            let instruction_id = Uuid::new_v4();
            let instruction = CommandProviderOutbound {
                instruction_id: format!("{:?}", instruction_id.to_simple()),
                request: Some(command_provider_outbound::Request::CommandResponse(response)),
            };
            yield instruction.to_owned();
            permits -= 1;
            if permits <= permits_batch_size {
                debug!("Command worker: stream: send more flow-control permits: amount: {:?}", permits_batch_size);
                let flow_control = FlowControl {
                    client_id: client_id.clone(),
                    permits: permits_batch_size,
                };
                let instruction_id = Uuid::new_v4();
                let instruction = CommandProviderOutbound {
                    instruction_id: format!("{:?}", instruction_id.to_simple()),
                    request: Some(command_provider_outbound::Request::FlowControl(flow_control)),
                };
                yield instruction.to_owned();
                permits += permits_batch_size;
            }
            debug!("Command worker: stream: flow-control permits: balance: {:?}", permits);
        }

        // debug!("Command worker: stream: stop");
    }
}

async fn store_events<P: std::fmt::Debug>(client: &mut EventStoreClient<Channel>, events: &EmitApplicableEventsAndResponse<P>) -> Result<()>{
    debug!("Client: {:?}: events: {:?}", client, events);
    let request = ReadHighestSequenceNrRequest {
        aggregate_id: events.target_aggregate_identifier.clone(),
        from_sequence_nr: 0,
    };
    let response = client.read_highest_sequence_nr(request).await?.into_inner();

    let message_identifier = Uuid::new_v4();
    let now = std::time::SystemTime::now();
    let timestamp = now.duration_since(std::time::UNIX_EPOCH)?.as_millis() as i64;
    let event_messages: Vec<Event> = events.events.iter().map(move |e| {
        let (type_name, event) = e;
        let mut buf = Vec::new();
        event.encode_u8(&mut buf).unwrap();
        let e = SerializedObject {
            r#type: type_name.to_string(),
            revision: "".to_string(),
            data: buf,
        };
        Event {
            message_identifier: format!("{:?}", message_identifier.to_simple()),
            timestamp,
            aggregate_identifier: events.target_aggregate_identifier.clone(),
            aggregate_sequence_number: response.to_sequence_nr + 1,
            aggregate_type: "Greeting".to_string(),
            payload: Some(e),
            meta_data: HashMap::new(),
            snapshot: false,
        }
    }).collect();
    let request = Request::new(futures_util::stream::iter(event_messages));
    client.append_event(request).await?;
    Ok(())
}