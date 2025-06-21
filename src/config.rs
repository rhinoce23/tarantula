use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub search: Search,
    pub rest: Rest,
    pub grpc: Grpc,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Attribute {
    pub level: i32,
    pub names: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Shapefile {
    pub path: String,
    pub attributes: HashMap<String, Attribute>,    
}

#[derive(Deserialize, Clone, Debug)]
pub struct Search {
    pub shapefile: Shapefile,
    pub districts: Vec<String>,
    pub hierarchies: Vec<String>,
    pub district_par: Vec<String>,
    pub district_par_any: Vec<String>,
    pub debug: bool,
    pub debug_name: String,
}

#[derive(Deserialize, Clone)]
pub struct Rest {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize, Clone)]
pub struct Grpc {
    pub port: u16,
    pub host: String,
}
