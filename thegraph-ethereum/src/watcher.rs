use ethereum_types::{Address, H256};
use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::{channel, Receiver};
use parity;
use std::sync::{Arc, Mutex};

use thegraph::components::ethereum::{EthereumWatcher as EthereumWatcherTrait, *};

pub struct EthereumWatcher<'a> {
    parity_client: &'a parity::RunningClient,
}

impl<'a> EthereumWatcher<'a> {
    pub fn new(parity_client: &'a parity::RunningClient) -> Self {
        EthereumWatcher {
            parity_client: parity_client,
        }
    }
}

impl<'a> EthereumWatcherTrait for EthereumWatcher<'a> {
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
        let (sender, receiver) = channel(100);
        receiver
    }

    fn unsubscribe_from_event(&mut self, unique_id: String) -> bool {
        false
    }
}
