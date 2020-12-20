use anyhow::{anyhow,Result};
use bytes::Bytes;
use futures_core::Future;
use futures_util::__private::Pin;
use prost::{Message, DecodeError};
use std::collections::HashMap;
use tonic::transport::Channel;

mod command_submit;
mod command_worker;
mod connection;

pub use command_submit::init as init_command_sender;
pub use command_worker::command_worker as command_worker;
pub use connection::wait_for_server as wait_for_server;

#[derive(Debug, Clone)]
pub struct AxonServerHandle {
    pub display_name: String,
    pub conn: Channel,
}

#[derive(Debug)]
pub struct AxonConnection {
    pub id: String,
    pub conn: Channel,
}

// I tried to make it possible to pass an `async fn` directly to parameter `handler`, but the return
// type after desugaring is unnameable
// (https://internals.rust-lang.org/t/naming-the-return-type-of-an-async-function/10085).
// So it has to be wrapped in a closure that `Box::pin`s the return value, lik this:
//
// ```rust
//     handler_registry.insert(
//         "GreetCommand",
//         &GreetCommand::decode,
//         &(|c| Box::pin(handle_greet_command(c)))
//     )?;
// ```
pub trait HandlerRegistry: Send {
    fn insert<T: Send + Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync)
    ) -> Result<()>;
    fn get(&self, name: &str) -> Option<&Box<dyn SubscriptionHandle>>;
}

pub struct TheHandlerRegistry {
    handlers: HashMap<String,Box<dyn SubscriptionHandle>>,
}

impl HandlerRegistry for TheHandlerRegistry {
    fn insert<T: Send + Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T, DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync)
    ) -> Result<()> {
        let name = name.to_string();
        let key = name.clone();
        let handle: Box<dyn SubscriptionHandle> = Box::new(Subscription{
            name,
            deserializer,
            handler
        });
        if (*self).handlers.contains_key(&key) {
            return Err(anyhow!("Handler already registered: {:?}", key))
        }
        (*self).handlers.insert(key.clone(), handle.box_clone());
        Ok(())
    }

    fn get(&self, name: &str) -> Option<&Box<dyn SubscriptionHandle>> {
        (*self).handlers.get(name)
    }
}

pub fn empty_handler_registry() -> TheHandlerRegistry {
    TheHandlerRegistry {
        handlers: HashMap::new()
    }
}

#[derive(Clone)]
struct Subscription<'a, T>
{
    pub name: String,
    pub deserializer: &'a (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
    pub handler: &'a (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync),
}

#[tonic::async_trait]
pub trait SubscriptionHandle: Send + Sync {
    fn name(&self) -> String;
    async fn handle(&self, buf: Vec<u8>) -> Result<()>;
    fn box_clone(&self) -> Box<dyn SubscriptionHandle>;
}

#[tonic::async_trait]
impl<T: Send + Clone> SubscriptionHandle for Subscription<'static, T>
{
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn handle(&self, buf: Vec<u8>) -> Result<()> {
        let message: T = (self.deserializer)(Bytes::from(buf))?;
        (self.handler)(message).await
    }

    fn box_clone(&self) -> Box<dyn SubscriptionHandle> {
        Box::from(Subscription::clone(&self))
    }
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
    async fn send_command(&self, command_type: &str, command: Box<&(dyn VecU8Message + Sync)>) -> Result<()>;
}
