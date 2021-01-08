use anyhow::{anyhow,Result};
use log::debug;
use prost::Message;
use tonic::transport::Channel;

use crate::axon_server::SerializedObject;

mod command_submit;
mod command_worker;
mod connection;
mod event_processor;
mod event_query;
mod handler_registry;
mod query_processor;

pub use command_submit::init as init_command_sender;
pub use command_worker::command_worker as command_worker;
pub use command_worker::{AggregateDefinition,AggregateRegistry,EmitApplicableEventsAndResponse,EmitEventsAndResponse,TheAggregateRegistry,create_aggregate_definition,emit,emit_applicable,emit_applicable_events_and_response,emit_events,emit_events_and_response,empty_aggregate_registry};
pub use connection::wait_for_server as wait_for_server;
pub use handler_registry::empty_handler_registry as empty_handler_registry;
pub use handler_registry::{HandlerRegistry,TheHandlerRegistry};
pub use event_processor::{TokenStore,event_processor};
pub use event_query::query_events;
pub use query_processor::{QueryContext,QueryResult,query_processor};

#[derive(Debug, Clone)]
pub struct AxonServerHandle {
    pub display_name: String,
    pub conn: Channel,
}

#[derive(Debug,Clone)]
pub struct AxonConnection {
    pub id: String,
    pub conn: Channel,
}

pub trait VecU8Message {
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<()>;
}

impl<T> VecU8Message for T
where T: Message + Sized
{
    fn encode_u8(&self, buf: &mut Vec<u8>) -> Result<()> {
        self.encode(buf).map_err(|e| anyhow!("Prost encode error: {:?}: {:?}", e.required_capacity(), e.remaining()))
    }
}

#[tonic::async_trait]
pub trait CommandSink {
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Result<Option<SerializedObject>>;
}

pub fn axon_serialize<T: Message>(type_name: &str, message: &T) -> Result<SerializedObject> {
    let mut buf = Vec::new();
    message.encode(& mut buf)?;
    let result = SerializedObject {
        r#type: type_name.to_string(),
        revision: "".to_string(),
        data: buf,
    };
    debug!("Encoded output: {:?}", result);
    Ok(result)
}

pub trait ApplicableTo<Projection> where Self: VecU8Message + Send + Sync + std::fmt::Debug {

    /// Applies this message to the given projection.
    fn apply_to(self: &Self, projection: &mut Projection) -> Result<()>;

    fn box_clone(self: &Self) -> Box<dyn ApplicableTo<Projection>>;
}

#[tonic::async_trait]
pub trait AsyncApplicableTo<Projection> where Self: VecU8Message + Send + Sync + std::fmt::Debug {

    /// Applies this message to the given projection.
    async fn apply_to(self: &Self, projection: &mut Projection) -> Result<()>;

    fn box_clone(self: &Self) -> Box<dyn AsyncApplicableTo<Projection>>;
}
