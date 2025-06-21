use axum::{
    routing::get,
    Router,
    http::StatusCode,
    extract::Query,
    Json
};
use std::net::ToSocketAddrs;
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;
use tower::ServiceBuilder;
use crate::search::GLOBAL_SEARCH;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct LonLatParams {
    lon: f64,
    lat: f64,
}

pub async fn start_server() {
    let config = crate::GLOBAL_CONFIG.rest.clone();
    tracing_subscriber::fmt::init();
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO)) 
        .on_response(DefaultOnResponse::new().level(Level::INFO)); 

    let app = Router::new()
        .route("/tarantula", get(tarantula))
        .layer(ServiceBuilder::new().layer(trace_layer));

    let addr = (config.host, config.port)
        .to_socket_addrs()
        .expect("invalid host or port")
        .next()
        .expect("unable to resolve address");
    
    println!("rest api server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn tarantula(Query(params): Query<LonLatParams>) 
    -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let search = unsafe { &GLOBAL_SEARCH.as_ref() }
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, 
            "search not initialized".to_string()))?;
    let result = search.search(params.lon, params.lat);
    match result {
        Ok(res) => Ok(Json(json!(res))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
