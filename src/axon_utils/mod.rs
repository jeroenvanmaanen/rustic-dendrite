use anyhow::{anyhow,Result};
use bytes::Bytes;
use futures_core::Future;
use futures_util::__private::Pin;
use log::debug;
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
pub trait HandlerRegistry<W>: Send {
    fn insert<T: Send + Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync)
    ) -> Result<()>;
    fn insert_with_output<T: Send + Clone, R: Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<Option<R>>> + Send>> + Sync),
        wrapper: Option<&'static (dyn Fn(R) -> Result<W> + Sync)>
    ) -> Result<()>;
    fn get(&self, name: &str) -> Option<&Box<dyn SubscriptionHandle<W>>>;
}

pub struct TheHandlerRegistry<W: Clone> {
    handlers: HashMap<String,Box<dyn SubscriptionHandle<W>>>,
}

impl<W: Clone + 'static> HandlerRegistry<W> for TheHandlerRegistry<W> {
    fn insert<T: Send + Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T, DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync)
    ) -> Result<()> {
        let name = name.to_string();
        let key = name.clone();
        let handle: Box<dyn SubscriptionHandle<W>> = Box::new(SubscriptionVoid{
            name,
            deserializer,
            handler,
        });
        if (*self).handlers.contains_key(&key) {
            return Err(anyhow!("Handler already registered: {:?}", key))
        }
        (*self).handlers.insert(key.clone(), handle.box_clone());
        Ok(())
    }

    fn insert_with_output<T: Send + Clone, R: Clone>(
        &mut self,
        name: &str,
        deserializer: &'static (dyn Fn(Bytes) -> Result<T, DecodeError> + Sync),
        handler: &'static (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<Option<R>>> + Send>> + Sync),
        wrapper: Option<&'static (dyn Fn(R) -> Result<W> + Sync)>
    ) -> Result<()> {
        let name = name.to_string();
        let key = name.clone();
        let handle: Box<dyn SubscriptionHandle<W>> = Box::new(Subscription{
            name,
            deserializer,
            handler,
            wrapper,
        });
        if (*self).handlers.contains_key(&key) {
            return Err(anyhow!("Handler already registered: {:?}", key))
        }
        (*self).handlers.insert(key.clone(), handle.box_clone());
        Ok(())
    }

    fn get(&self, name: &str) -> Option<&Box<dyn SubscriptionHandle<W>>> {
        (*self).handlers.get(name)
    }
}

pub fn empty_handler_registry<W: Clone>() -> TheHandlerRegistry<W> {
    TheHandlerRegistry {
        handlers: HashMap::new(),
    }
}

#[tonic::async_trait]
pub trait SubscriptionHandle<W>: Send + Sync {
    fn name(&self) -> String;
    async fn handle(&self, buf: Vec<u8>) -> Result<Option<W>>;
    fn box_clone(&self) -> Box<dyn SubscriptionHandle<W>>;
}

#[derive(Clone)]
struct Subscription<'a, T, R, W>
{
    pub name: String,
    pub deserializer: &'a (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
    pub handler: &'a (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<Option<R>>> + Send>> + Sync),
    pub wrapper: Option<&'a (dyn Fn(R) -> Result<W> + Sync)>,
}

#[tonic::async_trait]
impl<T: Send + Clone, R: Clone, W: Clone> SubscriptionHandle<W> for Subscription<'static, T, R, W>
{
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn handle(&self, buf: Vec<u8>) -> Result<Option<W>> {
        let message: T = (self.deserializer)(Bytes::from(buf))?;
        if let Some(result) = (self.handler)(message).await? {
            if let Some(wrapper) = self.wrapper {
                return Ok(Some((wrapper)(result)?));
            }
        }
        Ok(None)
    }

    fn box_clone(&self) -> Box<dyn SubscriptionHandle<W>> {
        Box::from(Subscription::clone(&self))
    }
}

#[derive(Clone)]
struct SubscriptionVoid<'a, T>
{
    pub name: String,
    pub deserializer: &'a (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
    pub handler: &'a (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync),
}

#[tonic::async_trait]
impl<T: Send + Clone, W: Clone + 'static> SubscriptionHandle<W> for SubscriptionVoid<'static, T>
{
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn handle(&self, buf: Vec<u8>) -> Result<Option<W>> {
        let message: T = (self.deserializer)(Bytes::from(buf))?;
        (self.handler)(message).await?;
        Ok(None)
    }

    fn box_clone(&self) -> Box<dyn SubscriptionHandle<W>> {
        Box::from(SubscriptionVoid::clone(&self))
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

pub fn proto_encode<T: Message>(message: T) -> Result<Box<Vec<u8>>> {
    debug!("Encode output: {:?}", message);
    let mut buf = Vec::new();
    message.encode(&mut buf)?;
    Ok(Box::from(buf))
}
