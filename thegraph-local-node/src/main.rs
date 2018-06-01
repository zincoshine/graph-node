extern crate clap;
extern crate futures;
#[macro_use]
extern crate sentry;
#[macro_use]
extern crate slog;
extern crate tempfile;
extern crate thegraph;
extern crate thegraph_core;
extern crate thegraph_hyper;
extern crate thegraph_mock;
extern crate thegraph_runtime_nodejs;
extern crate thegraph_store_postgres_diesel;
extern crate tokio;
extern crate tokio_core;

use clap::{App, Arg};
use sentry::integrations::panic::register_panic_handler;
use std::env;
use thegraph::prelude::*;
use thegraph::util::log::logger;
use thegraph_hyper::GraphQLServer as HyperGraphQLServer;
use thegraph_mock as mock;
use thegraph_runtime_nodejs::{RuntimeAdapter as NodeRuntimeAdapter, RuntimeAdapterConfig};
use thegraph_store_postgres_diesel::{Store as DieselStore, StoreConfig};
use tokio::prelude::*;
use tokio_core::reactor::Core;

fn main() {
    let mut core = Core::new().unwrap();
    let logger = logger();

    // Setup CLI using Clap, provide general info and capture postgres url
    let matches = App::new("thegraph-local-node")
        .version("0.1.0")
        .author("Graph Protocol, Inc.")
        .about("Scalable queries for a decentralized future (local node)")
        .arg(
            Arg::with_name("data-source-definition")
                .takes_value(true)
                .required(false)
                .long("data-source-definition")
                .value_name("FILE")
                .help("Path to the data source definition file"),
        )
        .arg(
            Arg::with_name("data-source-runtime")
                .takes_value(true)
                .required(false)
                .long("data-source-runtime")
                .value_name("DIR")
                .help("Path to the data source runtime source directory"),
        )
        .arg(
            Arg::with_name("postgres-url")
                .takes_value(true)
                .required(true)
                .long("postgres-url")
                .value_name("URL")
                .help("Location of the Postgres database used for storing entities"),
        )
        .get_matches();

    // Safe to unwrap because a value is required by CLI
    let postgres_url = matches.value_of("postgres-url").unwrap().to_string();

    // Obtain data source related command-line arguments
    let data_source_definition = matches.value_of("data-source-definition");
    let data_source_runtime = matches.value_of("data-source-runtime");

    debug!(logger, "Setting up Sentry");

    // Set up Sentry, with release tracking and panic handling;
    // fall back to an empty URL, which will result in no errors being reported
    let sentry_url = env::var_os("THEGRAPH_SENTRY_URL")
        .or(Some("".into()))
        .unwrap();
    let _sentry = sentry::init((
        sentry_url,
        sentry::ClientOptions {
            release: sentry_crate_release!(),
            ..Default::default()
        },
    ));
    register_panic_handler();

    info!(logger, "Starting up");

    // Create temporary directory for the data source runtime adapter
    let temp_dir = tempfile::tempdir()
        .expect("Failed to create temporary directory for data source runtime adapter");

    // Create system components
    let mut data_source_provider = mock::MockDataSourceProvider::new(&logger);
    let mut schema_provider = thegraph_core::SchemaProvider::new(&logger, core.handle());
    let mut store = DieselStore::new(StoreConfig { url: postgres_url }, &logger, core.handle());
    let mut graphql_server = HyperGraphQLServer::new(&logger, core.handle());
    let mut data_source_runtime_adapter = match (data_source_definition, data_source_runtime) {
        (Some(definition), Some(runtime)) => Some(NodeRuntimeAdapter::new(
            &logger,
            core.handle(),
            RuntimeAdapterConfig {
                temp_dir: String::from(temp_dir.path().to_str().unwrap()),
                data_source_definition: definition.to_string(),
                runtime_source_dir: runtime.to_string(),
                json_rpc_url: "localhost:8545".to_string(),
            },
        )),
        _ => {
            warn!(
                logger,
                "No data source arguments provided. Will not index anything"
            );
            None
        }
    };

    // Forward schema events from the data source provider to the schema provider
    let schema_stream = data_source_provider.schema_event_stream().unwrap();
    let schema_sink = schema_provider.schema_event_sink();
    core.handle().spawn({
        schema_stream
            .forward(schema_sink.sink_map_err(|e| {
                panic!("Failed to send schema event to schema provider: {:?}", e);
            }))
            .and_then(|_| Ok(()))
    });

    // Forward schema events from the schema provider to the store and GraphQL server
    let schema_stream = schema_provider.event_stream().unwrap();
    core.handle().spawn({
        schema_stream
            .forward(
                store
                    .schema_provider_event_sink()
                    .fanout(graphql_server.schema_provider_event_sink())
                    .sink_map_err(|e| {
                        panic!("Failed to send event to store and server: {:?}", e);
                    }),
            )
            .and_then(|_| Ok(()))
    });

    // Forward store events to the GraphQL server
    let store_stream = store.event_stream().unwrap();
    core.handle().spawn({
        store_stream
            .forward(graphql_server.store_event_sink().sink_map_err(|e| {
                panic!("Failed to send store event to the GraphQL server: {:?}", e);
            }))
            .and_then(|_| Ok(()))
    });

    // Forward incoming queries from the GraphQL server to the query runner
    let mut query_runner = thegraph_core::QueryRunner::new(&logger, core.handle(), store);
    let query_stream = graphql_server.query_stream().unwrap();
    core.handle().spawn({
        query_stream
            .forward(query_runner.query_sink().sink_map_err(|e| {
                panic!("Failed to send query to query runner: {:?}", e);
            }))
            .and_then(|_| Ok(()))
    });

    // If we have a runtime adapter, connect and start it now
    if let Some(ref mut runtime_adapter) = data_source_runtime_adapter {
        // TODO Connect the adapter to the store
        let runtime_stream = runtime_adapter.event_stream().unwrap();
        let logger = logger.clone();
        core.handle().spawn({
            runtime_stream
                .for_each(move |event| {
                    debug!(logger, "Runtime event"; "event" => format!("{:?}", event));
                    Ok(())
                })
                .and_then(|_| Ok(()))
        });

        // Start the adapter
        runtime_adapter.start();
    }

    // Serve GraphQL server over HTTP
    let http_server = graphql_server
        .serve()
        .expect("Failed to start GraphQL server");
    core.run(http_server).unwrap();

    // Stop the runtime adapter
    if let Some(ref mut runtime_adapter) = data_source_runtime_adapter {
        runtime_adapter.stop();
    }
}
