// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A scripted [`SwapNode`] that answers the send and settle steps from a
//! fixed script and records every transaction it was handed.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::SwapNode;
use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::solana_sdk::{signature::Signature, transaction::VersionedTransaction};
use crate::chains::solana::solana_transaction_status::{
    TransactionConfirmationStatus, TransactionStatus,
};
use crate::rpc::RpcError;

/// Answers in script order; the last answer of each queue repeats once the
/// queue is down to it.
pub(crate) struct ScriptedNode {
    simulation: Result<SimulationOutcome, RpcError>,
    sends: Mutex<VecDeque<SendAnswer>>,
    statuses: Mutex<VecDeque<Result<Option<TransactionStatus>, crate::Error>>>,
    block_height: u64,
    sent: Mutex<Vec<VersionedTransaction>>,
}

impl ScriptedNode {
    /// A node that simulates cleanly at `units`, answers each send with the
    /// next of `sends`, each status read with the next of `statuses`, and
    /// reports `block_height`.
    pub(crate) fn new(
        units: u64,
        sends: Vec<SendAnswer>,
        statuses: Vec<Result<Option<TransactionStatus>, crate::Error>>,
        block_height: u64,
    ) -> Self {
        Self {
            simulation: Ok(SimulationOutcome {
                err: None,
                logs: Vec::new(),
                units_consumed: Some(units),
                inner_instructions: Vec::new(),
            }),
            sends: Mutex::new(sends.into()),
            statuses: Mutex::new(statuses.into()),
            block_height,
            sent: Mutex::new(Vec::new()),
        }
    }

    /// The same node, unable to answer a simulation.
    pub(crate) fn without_simulation(mut self, error: RpcError) -> Self {
        self.simulation = Err(error);
        self
    }

    /// Every transaction handed to `send`, first send and re-broadcasts alike.
    pub(crate) fn sent(&self) -> Vec<VersionedTransaction> {
        self.sent.lock().unwrap().clone()
    }

    fn next<T: Clone>(queue: &Mutex<VecDeque<T>>) -> T {
        let mut queue = queue.lock().unwrap();
        if queue.len() > 1 {
            queue.pop_front().unwrap()
        } else {
            queue.front().cloned().expect("the script has an answer")
        }
    }
}

/// A status at `level`, carrying `err` when the chain reported one.
pub(crate) fn status(
    level: TransactionConfirmationStatus,
    err: Option<crate::chains::solana::solana_sdk::transaction::TransactionError>,
) -> TransactionStatus {
    TransactionStatus {
        slot: 1,
        confirmations: None,
        status: match &err {
            Some(err) => Err(err.clone()),
            None => Ok(()),
        },
        err,
        confirmation_status: Some(level),
    }
}

/// How the scripted node answers one send.
#[derive(Debug, Clone)]
pub(crate) enum SendAnswer {
    /// The node accepted the transaction and returned its signature.
    Accepted,
    /// The request left the machine and timed out without an answer.
    TimedOut,
    /// The send failed with this error.
    Fails(crate::Error),
}

impl SwapNode for ScriptedNode {
    async fn simulate(
        &self,
        _transaction: &VersionedTransaction,
    ) -> crate::Result<SimulationOutcome> {
        self.simulation.clone().map_err(crate::Error::Rpc)
    }

    async fn send(&self, transaction: &VersionedTransaction) -> crate::Result<Signature> {
        self.sent.lock().unwrap().push(transaction.clone());
        match Self::next(&self.sends) {
            SendAnswer::Accepted => Ok(transaction.signatures[0]),
            SendAnswer::TimedOut => Err(crate::Error::Rpc(RpcError::Network {
                message: "operation timed out".to_owned(),
                is_timeout: true,
            })),
            SendAnswer::Fails(error) => Err(error),
        }
    }

    async fn status(&self, _signature: &Signature) -> crate::Result<Option<TransactionStatus>> {
        Self::next(&self.statuses)
    }

    async fn block_height(&self) -> crate::Result<u64> {
        Ok(self.block_height)
    }
}
