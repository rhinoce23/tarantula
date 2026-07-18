use std::collections::HashMap;

use tarantula_s2::search::Search;
use tarantula_s2::config::{Attribute, Config};

#[tokio::main]
async fn main() {
    let mut attributes = HashMap::new();
    attributes.insert(
        "TL_SCCO_CTPRVN".to_string(),
        Attribute {
            level: 1,
            names: vec![
                "CTPRVN_CD".to_string(),
                "CTP_KOR_NM".to_string(),
                "CTP_ENG_NM".to_string(),
            ],
        },
    );

    let config = Config {
        search: tarantula_s2::config::Search {
            shapefile: tarantula_s2::config::Shapefile {
                path: "./data/converted".to_string(),
                attributes,
            },
            districts: vec!["36000".to_string()],
            hierarchies: vec!["TL_SCCO_CTPRVN".to_string()],
            district_par: vec![],
            district_par_any: vec![],
            debug: true,
            debug_name: String::new(),
        },
        rest: tarantula_s2::config::Rest {
            port: 8080,
            host: "127.0.0.1".to_string(),
        },
        grpc: tarantula_s2::config::Grpc {
            port: 8090,
            host: "127.0.0.1".to_string(),
        },
    };

    let mut search = Search::new(config.search).expect("create search");
    if let Err(err) = search.load() {
        eprintln!("failed to load search: {err}");
        return;
    }

    let results = search.search(127.285709646, 36.506549596)
        .expect("search");
    println!("results: {results:?}");
}
