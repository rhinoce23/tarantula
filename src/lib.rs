//! Tarantula search library.
//!
//! This crate exposes the search index implementation so it can be reused from
//! other Rust projects, while the binary entry point still starts the REST and
//! gRPC servers.

#[cfg(not(docsrs))]
#[link(name = "s2", kind = "dylib")]
extern {}

#[cfg(not(docsrs))]
use autocxx::prelude::*;
#[cfg(not(docsrs))]
include_cpp! {
    #include "cc/polygon.h"
    safety!(unsafe)
    opaque!("absl::container_internal::btree_map_container")
    generate!("Polygons")
    generate!("Polygon")
    generate!("LngLat")
    generate!("LngLats")
    generate!("Loop")
    generate!("SearchResult")
}

#[cfg(docsrs)]
pub mod ffi {
    #[derive(Default)]
    pub struct Polygons;

    impl Polygons {
        pub fn new() -> Self { Self }
        pub fn within_box(self) -> Self { self }
        pub fn add(&mut self, _value: impl std::any::Any) {}
        pub fn search(&self, _lon: f64, _lat: f64) -> i32 { -1 }
        pub fn search_polygon(&self, _lon: f64, _lat: f64) -> SearchResult { SearchResult::default() }
    }

    #[derive(Default)]
    pub struct Polygon;

    impl Polygon {
        pub fn new() -> Self { Self }
        pub fn within_box(self) -> Self { self }
        pub fn add(&mut self, _loop: Loop) {}
    }

    #[derive(Default)]
    pub struct LngLat;

    impl LngLat {
        pub fn lng(&self) -> f64 { 0.0 }
        pub fn lat(&self) -> f64 { 0.0 }
    }

    #[derive(Default)]
    pub struct LngLats;

    impl LngLats {
        pub fn new() -> Self { Self }
        pub fn within_box(self) -> Self { self }
        pub fn add(&mut self, _lng: f64, _lat: f64) {}
        pub fn size(&self) -> usize { 0 }
        pub fn pop_back(&mut self) {}
    }

    #[derive(Default)]
    pub struct Loop;

    impl Loop {
        pub fn new() -> Self { Self }
        pub fn within_box(self) -> Self { self }
        pub fn init(&mut self, _lnglats: LngLats, _outer: bool, _debug: bool) -> i32 { 4 }
    }

    #[derive(Default, Clone)]
    pub struct SearchResult {
        index: i32,
        lnglats: Vec<LngLat>,
    }

    impl SearchResult {
        pub fn index(&self) -> i32 { self.index }
        pub fn lnglats(&self) -> &[LngLat] { &self.lnglats }
    }
}

pub mod config;
pub mod search;
pub mod rest;
pub mod grpc;
pub mod utils;

pub use search::{Info, Search};
pub use config::Config;

use std::fs;
use once_cell::sync::Lazy;
use toml;

pub static GLOBAL_CONFIG: Lazy<config::Config> = Lazy::new(|| {
    let config_file_path = "Config.toml";
    let toml_string = fs::read_to_string(config_file_path)
        .expect("failed to read config file");
    toml::from_str(&toml_string).expect("failed to parse toml")
});

pub async fn run() {
    search::initialize_global_search();

    let rest_server = tokio::spawn(async {
        rest::start_server().await;
    });

    let grpc_server = tokio::spawn(async {
        grpc::start_server().await;
    });

    let (rest_result, grpc_result) = tokio::join!(rest_server, grpc_server);

    if let Err(e) = rest_result {
        eprintln!("error in rest server: {:?}", e);
    }

    if let Err(e) = grpc_result {
        eprintln!("error in grpc server: {:?}", e);
    }
}
