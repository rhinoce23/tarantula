
//! Tarantula search service entry point.
//!
//! This binary builds the search index and starts the REST and gRPC servers.

use tarantula_s2::run;

#[tokio::main]
async fn main() {
    run().await;
}
