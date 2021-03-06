# Rustic Dendrite

A [Rust](https://www.rust-lang.org) library to connect to [AxonServer](https://axoniq.io/product-overview/axon-server).

This project has _moved_ to [dendrite2go/rustic-dendrite](https://github.com/dendrite2go/rustic-dendrite).

This project is a sibling of [dendrite2go](https://github.com/dendrite2go) and [archetype-go-axon](https://github.com/dendrite2go/archetype-go-axon), but for the Rust programming language.

# Core concepts

* [Command / Query Responsibility Segregation](http://codebetter.com/gregyoung/2010/02/16/cqrs-task-based-uis-event-sourcing-agh/) (CQRS)
* [Domain-driven Design](https://dddcommunity.org/learning-ddd/what_is_ddd/) (DDD)
* [Event Sourcing](https://axoniq.io/resources/event-sourcing)
* [Futures and async/await](https://rust-lang.github.io/async-book)
* [gRPC](https://grpc.io/)
* [Microservices](https://en.wikipedia.org/wiki/Microservices)
* [Service Mesh](https://buoyant.io/2017/04/25/whats-a-service-mesh-and-why-do-i-need-one/) (video: [talk by Jeroen Reijn](https://2019.jfall.nl/sessions/whats-a-service-mesh-and-why-do-i-need-one/))

# Stack

In alphabetic order:

* [Bash](https://www.gnu.org/software/bash/manual/bash.html): The shell, or command language interpreter, for the GNU operating system — _for building and deploying_
* [AxonServer](https://axoniq.io/product-overview/axon-server): A zero-configuration message router and event store for Axon ([docker image](https://hub.docker.com/r/axoniq/axonserver/)) — _Event Store_
* [Docker compose](https://docs.docker.com/compose/): A tool for defining and running multi-container Docker applications — _for spinning up development and test environments_
* [ElasticSearch](https://www.elastic.co/elasticsearch/) You know, for search — _for query models (though any tokio-compatible persistence engine will do)_
* [Envoy proxy](https://www.envoyproxy.io/): An open source edge and service proxy, designed for cloud-native applications ([docker image](https://hub.docker.com/u/envoyproxy/)) — _to decouple microservices_
* [React](https://reactjs.org/): A JavaScript library for building user interfaces — _for the front-end_
* [Rust](https://www.rust-lang.org): A language empowering everyone to build reliable and efficient software — _for the back-end_
* [Tonic](https://github.com/hyperium/tonic): A Rust implementation of [gRPC](https://grpc.io/) with first class support of async/await — _for the plumbing on the back-end_

## Status

This project has now reached the level of Minimal Viable Deliverable in the sense that the first phase is completed: the current application communicates with AxonServer properly. Like [archetype-go-axon](https://github.com/dendrite2go/archetype-go-axon) it can do the following:
1. ☑ Set up a session with AxonServer
   * ☑ Enable React app to call a RPC endpoint on the example-command-api service through grpc-web
2. ☑ Issue commands
3. ☑ Register a command handler and handle commands
4. ☑ Submit events
   * ☑ Stream events to UI
5. ☑ Retrieve the events for an aggregate and build a projection
   * ☑ Validate commands against the projection
6. ☑ Register a tracking event processor and handle events
7. ☑ Store records in a query model: Elastic Search
   * ☑ Store tracking token in Elastic Search
8. ☑ Register a query handler and handle queries
   * ☑ Show query results in UI

The next task is to split off the example project from the library, and publish the library on [crates.io](https://crates.io/).

After that:

* Add macros to make the definition of handlers more ergonomic
* Add in-memory caching of aggregate projections
* Add support for storing snapshots of aggregate projections in AxonServer.
* Add support for segmentation to distribute the load on tracking event processors.
* Add support for sagas.
* ...
