use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::Sender;
use hyper;
use hyper::server::conn::AddrIncoming;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json;
use slog;

struct RuntimeAdapterService {
    logger: slog::Logger,
    json_sender: Sender<serde_json::Value>,
}

impl RuntimeAdapterService {
    pub fn new(logger: slog::Logger, json_sender: Sender<serde_json::Value>) -> Self {
        RuntimeAdapterService {
            logger,
            json_sender,
        }
    }
}

impl Service for RuntimeAdapterService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ReqBody>, Error = hyper::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let read_logger = self.logger.clone();
        let deserialize_logger = self.logger.clone();
        let send_logger = self.logger.clone();

        let json_sender = self.json_sender.clone();

        match request.method() {
            &Method::POST => Box::new({
                debug!(self.logger, "Received event from data source runtime");

                let deserialize_logger = self.logger.clone();

                // Read the request body into a single chunk
                request
                    .into_body()
                    .concat2()
                    .map(move |body| {
                        // Deserialize the JSON body and send it to the runtime adapter
                        serde_json::from_slice(&body)
                            .map_err(|e| {
                                error!(deserialize_logger,
                                       "Failed to deserialize JSON runtime event";
                                       "error" => format!("{}", e));
                            })
                            .map(|json| json_sender.clone().send(json).wait())
                    })
                    .and_then(|_| {
                        // Once we're done with the above, send 200 OK response back
                        future::ok(
                            Response::builder()
                                .status(StatusCode::CREATED)
                                .body(Body::from("OK"))
                                .unwrap(),
                        )
                    })
            }),
            _ => Box::new(future::ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap(),
            )),
        }
    }
}

/// Starts the runtime adpater's HTTP server to handle events from the runtime.
pub fn start_server(
    logger: slog::Logger,
    json_sender: Sender<serde_json::Value>,
    addr: &str,
) -> Box<Future<Item = (), Error = ()> + Send> {
    // Parse the server address
    let addr = addr.parse().expect(
        "Invalid network address provided \
         for the Node.js runtime adapter server",
    );

    // Create a new service factory
    let new_service = move || {
        future::ok::<RuntimeAdapterService, hyper::Error>(RuntimeAdapterService::new(
            logger.clone(),
            json_sender.clone(),
        ))
    };

    // Start serving requests
    Box::new(
        Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| panic!("Failed to start runtime adapter server: {}", e)),
    )
}
