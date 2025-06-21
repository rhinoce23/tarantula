
use tonic::{transport::Server, Response, Status};
use grpc::service_server::{Service, ServiceServer};
use grpc::{TarantulaReq, TarantulaReply};
use std::net::ToSocketAddrs;
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;
use crate::search::GLOBAL_SEARCH;

pub mod grpc {
    tonic::include_proto!("grpc"); 
}

#[derive(Debug, Default)]
pub struct GrpcService {}

#[tonic::async_trait]
impl Service for GrpcService {
    async fn tarantula(&self, request: tonic::Request<TarantulaReq>) 
        -> Result<Response<TarantulaReply>, Status> { 
        let search = unsafe { &GLOBAL_SEARCH.as_ref() }
            .ok_or_else(|| Status::internal("search not initialized"))?;
        let results 
            = search.search(request.get_ref().lon, request.get_ref().lat);
        match results {
            Ok(res) => {
                let reply = TarantulaReply {
                    infos: res
                        .into_iter()
                        .map(|info| grpc::Info {
                            district: info.district,
                            level: info.level,
                            name: info.name,
                        })
                        .collect(), 
                };
                return Ok(Response::new(reply));
            }
            Err(e) => {
                return Err(Status::internal(format!("search error: {}", e)));
            }
        }
    }
}

pub async fn start_server() {
    let config = crate::GLOBAL_CONFIG.grpc.clone();
    let addr = (config.host, config.port)
        .to_socket_addrs()
        .expect("invalid host or port")
        .next()
        .expect("unable to resolve address");

    let service = GrpcService::default();

    println!("grpc server listening on {}", addr);

    let trace_layer = TraceLayer::new_for_grpc()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Server::builder()
        .layer(trace_layer)
        .add_service(ServiceServer::new(service))
        .serve(addr)
        .await
        .unwrap()
}