extern crate ethereum_types;
extern crate futures;
extern crate serde_json;
extern crate thegraph;
extern crate tokio_core;
extern crate web3;

mod ethereum_adapter;

pub use self::ethereum_adapter::EthereumAdapter;
pub use self::ethereum_adapter::EthereumAdapterConfig;
pub use web3::transports;

#[cfg(test)]
mod tests {
    use ethereum_adapter::{EthereumAdapter, EthereumAdapterConfig};
    use tokio_core::reactor::Core;
    use web3::futures::Future;
    use web3::transports;

    #[test]
    fn new_ethereum_adapter() {
        let mut core = Core::new().unwrap();
        let result = transports::ipc::Ipc::new(&"insert_ipc_path_here"[..])
            // eloop needs to be threaded through, because if the handle is dropped
            // then rust-web3 futures will stop working.
            .map(|(eloop, transport)| {
                (
                    eloop,
                    EthereumAdapter::new(
                        EthereumAdapterConfig {
                            transport: transport,
                        },
                        core.handle(),
                    ),
                )
            })
            .and_then(|(eloop, eth_adapter)| {
                (eth_adapter.block_number().wait().map(|res| (eloop, res)))
            });
    }
}
