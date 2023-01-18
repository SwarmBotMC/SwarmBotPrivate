use clap::Parser;
use tokio::{runtime::Runtime, task};

fn main() {
    let opts = swarmbot_lib::CliOptions::parse();
    // create the single-threaded async runtime
    // we still leverage threads—however in a non-async context.
    // For instance, A* and other CPU-heavy tasks are spawned into threads
    let rt = Runtime::new().unwrap();
    let local = task::LocalSet::new();
    local.block_on(&rt, async move {
        match swarmbot_lib::run(opts).await {
            // this should never happen as this should be an infinite loop
            Ok(_) => println!("Program exited without errors somehow"),

            // print the error in non-debug fashion
            Err(err) => println!("{err}"),
        }
    });
}
