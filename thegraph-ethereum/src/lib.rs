extern crate ethereum_types;
extern crate futures;
extern crate parity;
extern crate thegraph;
extern crate tokio_core;

mod watcher;

pub use self::watcher::EthereumWatcher;

#[cfg(test)]
mod tests {
    use parity;
    use std::sync::{Arc, Mutex};
    #[test]
    fn new_ethereum_watcher() {
        let args: &[&str] = &[];
        let config = parity::Configuration::parse_cli(&args).unwrap();
        let execution_action = parity::start(config, move |_| {}, move || {}).unwrap();
        let parity_client = match execution_action {
            parity::ExecutionAction::Running(parity_client) => Some(parity_client),
            _ => None,
        }.unwrap();
    }
}
