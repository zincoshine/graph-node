extern crate jsonrpc_http_server;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate graph;

use graph::prelude::{JsonRpcServer as JsonRpcServerTrait, *};
use jsonrpc_http_server::{
    jsonrpc_core::{self, IoHandler, Params, Value},
    RestApi, Server, ServerBuilder,
};
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct SubgraphAddParams {
    subgraph_hash: String,
}

impl fmt::Display for SubgraphAddParams {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self)
    }
}

pub struct JsonRpcServer {}

impl JsonRpcServerTrait for JsonRpcServer {
    fn serve(
        port: u16,
        provider: Arc<impl SubgraphProvider>,
        logger: Logger,
    ) -> Result<Server, io::Error> {
        let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);

        // Configure handler.
        let mut handler = IoHandler::new();
        let add_provider = provider.clone();
        let add_logger = logger.clone();
        handler.add_method("subgraph_add", move |params: Params| {
            let provider = add_provider.clone();
            let logger = add_logger.clone();
            future::result(params.parse())
                .and_then(move |params: SubgraphAddParams| {
                    info!(logger, "Received subgraph_add request"; "params" => params.to_string());

                    provider
                        .add(format!("/ipfs/{}", params.subgraph_hash))
                        .map_err(|e| jsonrpc_core::Error {
                            code: jsonrpc_core::ErrorCode::ServerError(0),
                            message: e.to_string(),
                            data: None,
                        })
                        .map(|_| Ok(Value::Null))
                })
                .flatten()
        });

        ServerBuilder::new(handler)
            // Enable REST API:
            // POST /<method>/<param1>/<param2>
            .rest_api(RestApi::Secure)
            .start_http(&addr.into())
    }
}
