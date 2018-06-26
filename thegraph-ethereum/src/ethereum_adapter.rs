use web3::types::Log;
use web3::api::CreateFilter;
use web3::types::{Filter, FilterBuilder, BlockNumber};
use ethereum_types::{Address, H256};
use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::{channel, Receiver};
use tokio_core::reactor::Handle;
use web3;
use web3::api::Web3;
use web3::helpers::CallResult;
use web3::transports;
use web3::types::{U256, Bytes};
use std::time::Duration;

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

impl<T: web3::Transport> EthereumAdapterTrait for EthereumAdapter<T> {
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
    ) -> Receiver<EthereumEvent> {
        // let create_filter = self.event_filter();
        // let base_filter = create_filter.wait().unwrap();
        // let call_result = base_filter.logs();
        // let logs = call_result.wait().unwrap();
        let (sender, receiver) = channel(100);
        receiver
    }

    fn unsubscribe_from_event(&mut self, unique_id: String) -> bool {
        false
    }
}
