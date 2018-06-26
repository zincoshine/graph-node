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
    use ethereum_types::{Address, H256};
    use std::error::Error;
    use std::result::Result;
    use std::sync::Arc;
    use std::time::Duration;
    use thegraph::components::ethereum::{BlockNumberRange, EthereumEventSubscription};
    use tokio_core::reactor::Core;
    use web3;
    use web3::api::{FilterStream, Web3};
    use web3::futures::future;
    use web3::futures::stream::{iter_ok, IterResult};
    use web3::futures::{Future, Stream};
    use web3::transports;
    use web3::types::Bytes;
    // use futures::stream::Stream;

    #[test]
    fn new_ethereum_ipc_adapter() {
        let mut core = Core::new().unwrap();
        let result = transports::ipc::Ipc::new(&"/Users/brandon/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"[..])
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
        let tranport_result = transports::ipc::Ipc::with_event_loop(
            &"/Users/brandon/Library/Application Support/io.parity.ethereum/jsonrpc.ipc"[..],
            &core.handle(),
        );
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
                from: Some(7781365),
                to: Some(7781365),
            },
        };
        let work = adapter
            .event_filter(subscription)
            .map(|base_filter| {
                let past_logs_stream = base_filter
                    .logs()
                    .map(|logs_vec| iter_ok::<_, web3::error::Error>(logs_vec))
                    .flatten_stream();
                let future_logs_stream = base_filter.stream(Duration::from_millis(2000));
                past_logs_stream.chain(future_logs_stream)
            })
            .flatten_stream()
            .for_each(|log_event| {
                println!("{:?}", log_event);
                Result::Ok(())
            });

        // let stream = base_filter.stream(Duration::from_millis(20));
        // .flatten_stream();
        // .for_each(|log_event| {
        //     println!("{:?}", log_event);
        //     future::ok::<(), web3::error::Error>(());
        // });

        // let logs_vector_call = base_filter.logs();
        // let logs_vector = core.run(logs_vector_call).unwrap();
        // println!("logs_vector {:?}", logs_vector);
        // let map = logs_vector.iter().for_each(|log| println!("{:?}", log));
        core.run(work);
    }
}
