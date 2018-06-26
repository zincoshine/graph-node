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
    use web3::futures::{Stream, Future};
    use web3::futures::future::ok;
    use web3::futures::stream::{iter_ok, IterResult};
    use web3::transports;
    use web3::api::Web3;
    use web3::types::Bytes;
    use ethereum_types::{Address, H256};
    use thegraph::components::ethereum::{EthereumEventSubscription, BlockNumberRange};
    use std::time::Duration;
    use web3;
    // use futures::stream::Stream;

    #[test]
    fn new_ethereum_ipc_adapter() {
        let mut core = Core::new().unwrap();
        let result = transports::ipc::Ipc::new(&"/Users/aklempner/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"[..])
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
                (eth_adapter.block_number().wait().map(|res| (println!("ipc block number {:?}", res))))
            });
    }

    #[test]
    fn new_ethereum_rpc_adapter() {
        let mut core = Core::new().unwrap();
        let result = transports::http::Http::new(&"http://127.0.0.1:8545"[..])
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
                (eth_adapter.block_number().wait().map(|res| (println!("rpc block number {:?}", res))))
            });
    }

    #[test]
    fn event_logs() {
        let mut core = Core::new().unwrap();
        let tranport_result = transports::ipc::Ipc::with_event_loop(&"/Users/aklempner/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"[..], &core.handle());
        let transport = tranport_result.unwrap();
        let adapter = EthereumAdapter::new(
            EthereumAdapterConfig {
                transport: transport,
            },
            core.handle(),
        );
        let transfer_topic_work = adapter.sha3("Transfer(address,address,uint256)");
        let transfer_topic = core.run(transfer_topic_work).unwrap();
        println!("transfer topic {:?}", transfer_topic);
        let subscription = EthereumEventSubscription {
            subscription_id: String::from("1"),
            address: Address::zero(),
            event_signature: transfer_topic,
            range: BlockNumberRange {
                from: Some(7773000),
                to: Some(7768100)
            },
        };
        let work = adapter.event_filter(subscription);
        let base_filter = core.run(work).unwrap();
        let logs_vector_call = base_filter.logs();
        let logs_vector = core.run(logs_vector_call).unwrap();
        println!("logs_vector {:?}", logs_vector);
        let map = logs_vector.iter().for_each(|log| {println!("{:?}", log)});
    }
}
