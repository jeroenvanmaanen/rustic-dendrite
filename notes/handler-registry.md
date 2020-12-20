# Handler Registry

This project involves code that talks gRPC to [AxonServer](https://axoniq.io/product-overview/axon-server). AxonServer uses quite a number of message types that feature a payload that has a `string` name and a `bytes` data. Git tag `handler-registry` introduces a HandlerRegistry in module [axon_utils](https://github.com/jeroenvanmaanen/rustic-dendrite/blob/handler-registry/src/axon_utils/mod.rs) that makes it quite straight-forward to configure handlers for such payloads if the data is protobuf encoded.

The design revolves around a generic struct `Subscription<'a,T>` that has a property `deserializer` that knows how to deserialize a `Bytes` buffer to a value of type `T` as well as a property `handler` that knows how to handle a payload of type `T`.

```rust
#[derive(Clone)]
struct Subscription<'a, T>
{
    pub name: String,
    pub deserializer: &'a (dyn Fn(Bytes) -> Result<T,prost::DecodeError> + Sync),
    pub handler: &'a (dyn Fn(T) -> Pin<Box<dyn Future<Output=Result<()>> + Send>> + Sync),
}
```

Struct `Subscription` implements a trait `SubscriptionHandle` that is not generic. The implementation hides the fact that the struct knows the type of the payload. This means that `SubscriptionHandle`s for different payload types have the same type and can be stored in the same `HashMap`.

```rust
#[tonic::async_trait]
pub trait SubscriptionHandle: Send + Sync {
    fn name(&self) -> String;
    async fn handle(&self, buf: Vec<u8>) -> Result<()>;
    fn box_clone(&self) -> Box<dyn SubscriptionHandle>;
}
```

Trait `HandlerRegistry`, struct `TheHandlerRegistry`, and fn `empty_handler_registry` together offer the possibility of bundling multiple `SubscriptionHandle`s in a `HashMap`. This registry can be handed over to the command worker. The worker uses the registry to lookup the handler for each incoming payload and let the handler handle it without the `command_worker` library function having to know the types of the incoming payloads. The command worker uses the registry like this:

```rust
async fn handle(command: &Command, handler_registry: &TheHandlerRegistry) -> Result<()> {
    let handler = handler_registry.get(&command.name).ok_or(anyhow!("No handler for: {:?}", command.name))?;
    let data = command.payload.clone().map(|p| p.data).ok_or(anyhow!("No payload data for: {:?}", command.name))?;
    handler.handle(data).await
}
```

All in all the use of this mechanism by the caller of the `command_worker` looks like this:

```rust
async fn handle_commands() -> Result<()> {
    let axon_connection = wait_for_server()?;
    let mut handler_registry = empty_handler_registry();
    handler_registry.insert(
        "GreetCommand",
        &GreetCommand::decode,
        &(|c| Box::pin(handle_greet_command(c)))
    )?;
    command_worker(axon_connection, handler_registry).await
}

async fn handle_greet_command (command: GreetCommand) -> Result<()> {
    debug!("Greet command handler: {:?}", command);
    // Check business logic
    // Emit events
    Ok(())
}
```

The ugly wrapping of the handler in a closure is necessary because the actual desugared type of an `async fn` is an `impl Future` and hence unnameable (See also [Naming the return type of an async function](https://internals.rust-lang.org/t/naming-the-return-type-of-an-async-function/10085)).

The author thanks the compiler for helping out with the lifetimes and `Send`, `Sync`, `Pin`, etc. stuff.