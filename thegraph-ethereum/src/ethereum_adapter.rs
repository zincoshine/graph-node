use futures::stream::iter_ok;
use ethereum_types::{Address, H256};
use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tokio_core::reactor::Handle;
use web3;
use web3::api::CreateFilter;
use web3::api::Web3;
use web3::helpers::CallResult;
use web3::transports;
use web3::types::Log;
use web3::types::{BlockNumber, Filter, FilterBuilder};
use web3::types::{Bytes, U256};
use web3::error::{Error as Web3Error};

use thegraph::components::ethereum::{EthereumAdapter as EthereumAdapterTrait, *};

pub struct EthereumAdapterConfig<T: web3::Transport> {
    pub transport: T,
}

pub struct EthereumAdapter<T: web3::Transport> {
    eth_client: Web3<T>,
    runtime: Handle,
}

// pub fn new_ipc_ethereum_adapter(
//     config: EthereumAdapterConfig,
// ) -> EthereumAdapter<transports::ipc::Ipc> {
//     let (eloop, transport) = transports::ipc::Ipc::new(config.path).unwrap();
//     EthereumAdapter {
//         eth_client: Web3::new(transport),
//         eloop: eloop,
//     }
// }

impl<T: web3::Transport> EthereumAdapter<T> {
    pub fn new(config: EthereumAdapterConfig<T>, runtime: Handle) -> Self {
        EthereumAdapter {
            eth_client: Web3::new(config.transport),
            runtime: runtime,
        }
    }

    pub fn block_number(&self) -> CallResult<U256, T::Out> {
        self.eth_client.eth().block_number()
    }

    pub fn sha3(&self, data: &str) -> CallResult<H256, T::Out> {
        self.eth_client.web3().sha3(Bytes::from(data))
    }

    pub fn event_filter(&self, subscription: EthereumEventSubscription) -> CreateFilter<T, Log> {
        let filter_builder = FilterBuilder::default();
        let eth_filter: Filter = filter_builder
            .from_block(BlockNumber::Number(subscription.range.from.unwrap()))
            // .to_block(BlockNumber::Number(subscription.range.to.unwrap()))
            .to_block(BlockNumber::Latest)
            .topics(Some(vec!(subscription.event_signature)), None, None, None)
            .build();
        self.eth_client.eth_filter().create_logs_filter(eth_filter)
    }

    // pub fn event_filter(&self) -> CreateFilter<T, Log> {
    //     let filter_builder = FilterBuilder::default();
    //     let topic = H256::from("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");
    //     let eth_filter: Filter = filter_builder
    //         .from_block(BlockNumber::Number(7768000))
    //         .to_block(BlockNumber::Latest)
    //         .topics(Some(vec!(topic)), None, None, None)
    //         .build();
    //     self.eth_client.eth_filter().create_logs_filter(eth_filter)
    // }
}

impl<T: 'static +  web3::Transport> EthereumAdapterTrait for EthereumAdapter<T> {
    fn contract_state(
        &mut self,
        request: EthereumContractStateRequest,
    ) -> Result<EthereumContractState, EthereumContractStateError> {
        Ok(EthereumContractState {
            address: Address::new(),
            block_hash: H256::new(),
            data: Vec::new(),
        })
    }

    fn subscribe_to_event(
        &mut self,
        subscription: EthereumEventSubscription,
    ) -> Box<Stream<Item=EthereumEvent,Error=Web3Error>> {
        Box::new(
            self.event_filter(subscription)
            .map(|base_filter| {
                let past_logs_stream = base_filter
                    .logs()
                    .map(|logs_vec| iter_ok::<_, web3::error::Error>(logs_vec))
                    .flatten_stream();
                let future_logs_stream = base_filter.stream(Duration::from_millis(2000));
                past_logs_stream.chain(future_logs_stream)
            })
            .flatten_stream()
            .map(|log| {
                EthereumEvent {
                    address: log.address,
                    event_signature: log.topics[0],
                    block_hash: log.block_hash.unwrap()
                }
            })
        )
    }

    fn unsubscribe_from_event(&mut self, unique_id: String) -> bool {
        false
    }
}
