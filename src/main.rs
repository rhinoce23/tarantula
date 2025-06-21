
#[link(
    name = "s2",
    kind = "dylib",
)]
extern {}

use autocxx::prelude::*;
include_cpp! {
    #include "cc/polygon.h"
    safety!(unsafe)
    generate!("Polygons")
    generate!("Polygon")
    generate!("LngLat")
    generate!("LngLats")
    generate!("Loop")
    generate!("SearchResult")
}

use std::fs;
use toml;
mod config;
use config::Config;
mod search;
use search::initialize_global_search;
mod rest;
use once_cell::sync::Lazy;
use tokio::join;
mod grpc;

pub static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(|| {
    let config_file_path = "Config.toml";
    let toml_string = fs::read_to_string(config_file_path)
        .expect("failed to read config file");
    return toml::from_str(&toml_string).expect("failed to parse toml");
});

#[tokio::main]
async fn main() {
    
    initialize_global_search();

    let rest_server = tokio::spawn(async {
        rest::start_server().await;
    });

    let grpc_server = tokio::spawn(async {
        grpc::start_server().await;
    });

    let (rest_result, grpc_result) 
        = join!(rest_server, grpc_server);

    if let Err(e) = rest_result {
        eprintln!("error in rest server: {:?}", e);
    }

    if let Err(e) = grpc_result {
        eprintln!("error in grpc server: {:?}", e);
    }
}
