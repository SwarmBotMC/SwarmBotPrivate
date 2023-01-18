use std::time::Duration;

use swarmbot_lib::CliOptions;
use tokio::{runtime::Runtime, task};

use crate::server::Server;

mod server;

fn main() {
    let _server = Server::init().expect("could not start server");
    std::thread::sleep(Duration::from_secs(15));
    // create the single-threaded async runtime
    // we still leverage threads—however in a non-async context.
    // For instance, A* and other CPU-heavy tasks are spawned into threads
    let rt = Runtime::new().unwrap();
    let local = task::LocalSet::new();
    local.block_on(&rt, async move {
        match test_bot().await {
            // this should never happen as this should be an infinite loop
            Ok(_) => println!("Program exited without errors somehow"),

            // print the error in non-debug fashion
            Err(err) => println!("{err:?}"),
        }
    });
}

async fn test_bot() -> anyhow::Result<()> {
    println!("starting up bot");
    swarmbot_lib::run(CliOptions::default()).await?;
    println!("bot finished running");
    Ok(())
}
