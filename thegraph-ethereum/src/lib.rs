extern crate ethereum_types;
extern crate futures;
extern crate parity;
extern crate thegraph;
extern crate tokio_core;
extern crate jsonrpc_core;
extern crate serde;
extern crate serde_json;
extern crate serde_derive;

mod watcher;

pub use self::watcher::EthereumWatcher;

#[cfg(test)]
mod tests {
    use parity;
    use std::sync::{Arc, Mutex};
    use watcher::EthereumWatcher;
    #[test]
    fn new_ethereum_watcher() {
        let args: &[&str] = &[];
        let config = parity::Configuration::parse_cli(&["parity", "--chain=kovan"]).unwrap();
        // let config = parity::Configuration::parse_cli(&args).unwrap();
        let execution_action = parity::start(config, move |_| {}, move || {}).unwrap();
        let parity_client = match execution_action {
            parity::ExecutionAction::Running(parity_client) => Some(parity_client),
            _ => None,
        }.unwrap();

        let mut ethereum_watcher = EthereumWatcher::new(&parity_client);
        let block_number = ethereum_watcher.block_number().unwrap();
        // Blocknumber: "{\"jsonrpc\":\"2.0\",\"result\":\"0x74ac63\",\"id\":1}"
        println!("Blocknumber: {:?}", block_number);
    }
}
