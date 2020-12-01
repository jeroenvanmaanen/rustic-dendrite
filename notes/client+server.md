# Tonic: Client + Server

I am struggling trying to combine a Tonic server with an independent Tonic client process.

A bit of context: I would like to write an Event Sourced application in Rust that uses [Axon Server](https://axoniq.io/product-overview/axon-server) as an Event Store and Message Routing subsystem. Axon Server publishes a gRPC API, so I figured Tonic would be the perfect match for it. The current state of the project can be found at [rustic-dendrite](https://github.com/jeroenvanmaanen/rustic-dendrite).

Most of the application consists of processes that subscribe to some topic on Axon Server and process items (Commands, Business Events and Queries). In addition to that, the API part exposes its own gRPC API that turns client requests into Commands and sends them off to Axon Server to be routed to a suitable node.

I got the API part working by adapting the material in the Tonic tutorials. Now, I am trying to add a process that handles commands. The structure is something like this (the full version can be found on branch [command-worker](https://github.com/jeroenvanmaanen/rustic-dendrite/tree/command-worker)):

main:
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Rustic dendrite API service started");

    tokio::spawn(handle_commands());

    let addr = "0.0.0.0:8181".parse()?;
    let greeter_server = init().await.unwrap();
    Server::builder()
        .add_service(GreeterServiceServer::new(greeter_server))
        .serve(addr)
        .await?;

    Ok(())
}
```

In my application module “example_command”
```rust
pub async fn handle_commands() {
    debug!("Handle commands for example application");
    let axon_connection = wait_for_server("proxy", 8124, "Command Processor").await.unwrap();
    debug!("Axon connection: {:?}", axon_connection);
    match command_worker(axon_connection, &["GreetCommand"]).await {
        Err(e) => {
            error!("Error while handling commands: {:?}", e);
        }
        _ => {}
    };
    debug!("Stopped handling commands for example application");
}
```

In my library module “axon_utils”
```rust
pub async fn command_worker(
    axon_connection: AxonConnection, commands: &'static [&'static str]
) -> Result<(),&str> {
    // ...
    let outbound = stream! {
        // ... Create an instruction to issue a single flow control permit.
        // ... Coordinating permits with the incoming stream
        // ... is the next challenge.
        yield instruction.to_owned();
    };

    // let result = client.open_stream(Request::new(outbound)).await;
    // debug!("Stream result: {:?}", result);

    // ...
    Ok(())
}
```

Much to my surprise, if I uncomment the `let result = client.open_stream(Request::new(outbound)).await;` then I get a compile error in the `main` function!

```
error[E0477]: the type `async_stream::async_stream::AsyncStream<rustic_dendrite::axon_server::command::CommandProviderOutbound, impl std::future::Future>` does not fulfill the required lifetime
  --> src/main.rs:15:5
   |
15 |     tokio::spawn(handle_commands());
   |     ^^^^^^^^^^^^
   |
   = note: type must satisfy the static lifetime
```

Apparently the real signature of an async function depends on the await macros in the body and it propagates from `command_worker` via `handle_commands` to `main`.

I tried using async-scoped (see branch [async-scoped](https://github.com/jeroenvanmaanen/rustic-dendrite/tree/async-scoped)). That had the effect of containing the error in the `command_worker` function, but it did not solve the problem.

```rust
    let outbound_ref = &outbound;

    async_scoped::TokioScope::scope_and_block(|s| {
        let proc = || async {
            let result = client.open_stream(Request::new(*outbound_ref)).await;
            debug!("Stream result: {:?}", result);
        };
        s.spawn(proc());
    });
```

```
error[E0477]: the type `async_stream::AsyncStream<axon_server::command::CommandProviderOutbound, impl std::future::Future>` does not fulfill the required lifetime
  --> src/axon_utils/command_worker.rs:57:11
   |
57 |         s.spawn(proc());
   |           ^^^^^
   |
   = note: type must satisfy the static lifetime
```

What can I do to make rustc happy? Or is there a better approach altogether?
