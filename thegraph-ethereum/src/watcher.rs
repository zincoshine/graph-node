use ethereum_types::{Address, H256};
use futures::future;
use futures::prelude::*;
use futures::sync::mpsc::{channel, Receiver};
use parity;
use std::sync::{Arc, Mutex};
use jsonrpc_core::types::response::Response;
use serde;
use serde_json;
use serde_json::{Error};


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

    pub fn block_number(&mut self) -> Result<Response, Error> {
        let req_number = r#"{
    		"jsonrpc": "2.0",
    		"method": "eth_blockNumber",
    		"params": [],
    		"id": 1
    	}"#;
        let rpcres = self.parity_client.rpc_query_sync(req_number).unwrap();
        serde_json::from_str(&rpcres)
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
