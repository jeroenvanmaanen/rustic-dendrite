use log::{debug,error};
use crate::axon_utils::{command_worker,wait_for_server};

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