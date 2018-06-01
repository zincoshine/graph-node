use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::{channel, Receiver, Sender};
use serde_json;
use slog;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::{tempdir, TempDir};
use tokio_core::reactor::Handle;

use thegraph::components::data_sources::RuntimeAdapterEvent;
use thegraph::components::store::StoreKey;
use thegraph::prelude::{*, RuntimeAdapter as RuntimeAdapterTrait};
use thegraph::util::stream::StreamError;

use server;

/// Configuration for the runtime adapter.
pub struct RuntimeAdapterConfig {
    pub temp_dir: String,
    pub data_source_definition: String,
    pub runtime_source_dir: String,
    pub json_rpc_url: String,
}

/// Adapter to set up and connect to a Node.js based data source runtime.
pub struct RuntimeAdapter {
    logger: slog::Logger,
    config: RuntimeAdapterConfig,
    runtime: Handle,
    event_sink: Arc<Mutex<Option<Sender<RuntimeAdapterEvent>>>>,
    temp_dir: Option<TempDir>,
    child_process: Option<Child>,
}

impl RuntimeAdapter {
    /// Creates a new runtiem adapter for a Node.js based data source runtime.
    pub fn new(logger: &slog::Logger, runtime: Handle, config: RuntimeAdapterConfig) -> Self {
        // Create the runtime adapter
        let runtime_adapter = RuntimeAdapter {
            logger: logger.new(o!("component" => "RuntimeAdapter")),
            config,
            runtime,
            event_sink: Arc::new(Mutex::new(None)),
            temp_dir: None,
            child_process: None,
        };

        // Return the runtime adapter
        runtime_adapter
    }
}

impl Drop for RuntimeAdapter {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child_process {
            child
                .kill()
                .expect("Failed to kill Node.js data source runtime");
        }
        self.child_process = None
    }
}

impl RuntimeAdapterTrait for RuntimeAdapter {
    fn start(&mut self) {
        info!(self.logger, "Prepare runtime");

        // Copy runtime sources into the temporary directory
        debug!(self.logger, "Copy runtime source files");
        for filename in ["package.json", "index.js", "db.js"].into_iter() {
            fs::copy(
                Path::new(self.config.runtime_source_dir.as_str()).join(filename),
                Path::new(self.config.temp_dir.as_str()).join(filename),
            ).expect(format!("Failed to copy data source runtime file: {}", filename).as_str());
        }

        // Run `npm install` in the temporary directory
        debug!(self.logger, "Install NPM dependencies");
        Command::new("npm")
            .arg("install")
            .current_dir(self.config.temp_dir.as_str())
            .output()
            .expect("Failed to run `npm install`");

        // Create a channel to receive JSON updates from the adapter server and
        // start an HTTP server for the runtime to send events to
        debug!(self.logger, "Start adapter server");
        let (json_sender, json_receiver) = channel(10);
        self.runtime.spawn(server::start_server(
            self.logger.clone(),
            json_sender,
            "127.0.0.1:7500",
        ));

        // Spawn the Node.js runtime process; Node needs to be installed on the machine
        // or this will fail
        debug!(self.logger, "Start the Node.js data source runtime");
        let mut child = Command::new("node")
            .current_dir(self.config.temp_dir.as_str())
            .arg("index.js")
            .arg(self.config.data_source_definition.as_str())
            .arg(self.config.temp_dir.as_str())
            .arg("http://127.0.0.1:7500/")
            .spawn()
            .expect("Failed to start Node.js date source runtime");

        // Process forwrd incoming runtime events
        let receiver_logger = self.logger.clone();
        let event_sink = self.event_sink
            .lock()
            .unwrap()
            .clone()
            .expect("Node.js runtime adapter started without being connected");
        self.runtime
            .spawn(json_receiver.for_each(move |json: serde_json::Value| {
                info!(receiver_logger, "Handle runtime event"; "json" => format!("{}", json));

                let event_data = json.as_object()
                    .expect("Runtime event data is not an object");

                // Extract entity name
                let entity = event_data
                    .get("entity")
                    .expect("Runtime event data lacks an \"entity\" field")
                    .as_str()
                    .expect("Runtime event: \"entity\" event is not a string");

                // Extract operation name
                let operation = event_data
                    .get("operation")
                    .expect("Runtime event data lacks an \"operation\" field")
                    .as_str()
                    .expect("Runtime event: \"operation\" is not a string");

                // Extract entity data
                let data = event_data
                    .get("data")
                    .expect("Runtime event data lacks a \"data\" field");

                // Deserialize entity
                let entity_data = serde_json::from_value::<Entity>(data.clone())
                    .expect("Runtime event: Failed to deserialize entity data");

                let event = match operation {
                    "add" => RuntimeAdapterEvent::EntityAdded(
                        "memefactory".to_string(),
                        StoreKey {
                            entity: entity.to_string(),
                            id: data.get("id")
                                .expect("Runtime event: \"data\" lacks an \"id\" field")
                                .as_str()
                                .expect("Runtime event: entity \"id\" is not a string")
                                .to_string(),
                        },
                        entity_data,
                    ),
                    _ => unimplemented!(),
                };

                event_sink
                    .clone()
                    .send(event)
                    .map_err(|e| {
                        panic!("Failed to forward runtime adapter event");
                    })
                    .and_then(|_| Ok(()))
            }));
    }

    fn stop(&mut self) {
        info!(self.logger, "Stop");
    }

    fn event_stream(&mut self) -> Result<Receiver<RuntimeAdapterEvent>, StreamError> {
        info!(self.logger, "Setting up event stream");

        // If possible, create a new channel for streaming runtime adapter events
        let mut event_sink = self.event_sink.lock().unwrap();
        match *event_sink {
            Some(_) => Err(StreamError::AlreadyCreated),
            None => {
                let (sink, stream) = channel(100);
                *event_sink = Some(sink);
                Ok(stream)
            }
        }
    }
}
