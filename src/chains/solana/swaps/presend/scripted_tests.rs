// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A scripted [`SwapNode`] that answers the send and settle steps from a
//! fixed script and records every transaction it was handed.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::SwapNode;
use crate::chains::solana::rpc::types::SimulationOutcome;
use crate::chains::solana::settlement::FinalizedTip;
use crate::chains::solana::solana_sdk::{signature::Signature, transaction::VersionedTransaction};
use crate::chains::solana::solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionConfirmationStatus, TransactionStatus,
};
use crate::rpc::RpcError;

/// Answers in script order; the last answer of each queue repeats once the
/// queue is down to it.
pub(crate) struct ScriptedNode {
    simulation: Result<SimulationOutcome, RpcError>,
    sends: Mutex<VecDeque<SendAnswer>>,
    statuses: Mutex<VecDeque<Result<Option<TransactionStatus>, crate::Error>>>,
    block_height: u64,
    /// The slot the finalized tip is read at.
    tip_slot: u64,
    /// The slot every status read answers at.
    read_slot: u64,
    /// The execution error of the transaction the chain holds, null when it
    /// succeeded; `None` when the chain holds none.
    executed: Option<serde_json::Value>,
    sent: Mutex<Vec<VersionedTransaction>>,
    accepted: Mutex<usize>,
}

impl ScriptedNode {
    /// A node that simulates cleanly at `units`, answers each send with the
    /// next of `sends`, each status read with the next of `statuses`, reports
    /// `block_height` as its finalized tip, answers status reads from that same
    /// slot, and holds no transaction for any signature.
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
            tip_slot: 0,
            read_slot: 0,
            executed: None,
            sent: Mutex::new(Vec::new()),
            accepted: Mutex::new(0),
        }
    }

    /// The same node, answering status reads from `read_slot` while its
    /// finalized tip was read at `tip_slot`.
    pub(crate) fn reading_at(mut self, read_slot: u64, tip_slot: u64) -> Self {
        self.read_slot = read_slot;
        self.tip_slot = tip_slot;
        self
    }

    /// The same node, holding an executed transaction for every signature
    /// looked up, failed with `err` unless it is null.
    pub(crate) fn holding(mut self, err: serde_json::Value) -> Self {
        self.executed = Some(err);
        self
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

    /// How many sends the node answered with acceptance.
    pub(crate) fn accepted(&self) -> usize {
        *self.accepted.lock().unwrap()
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

/// A transaction the chain executed, failed with `err` unless it is null.
fn executed(err: serde_json::Value) -> EncodedConfirmedTransactionWithStatusMeta {
    serde_json::from_value(serde_json::json!({
        "slot": 1,
        "transaction": {
            "signatures": ["signature"],
            "message": {
                "accountKeys": [],
                "recentBlockhash": "11111111111111111111111111111111",
                "instructions": []
            }
        },
        "meta": {
            "err": err,
            "status": { "Ok": null },
            "fee": 5000,
            "preBalances": [],
            "postBalances": []
        },
        "blockTime": null
    }))
    .expect("a transaction response decodes")
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
    /// The request never comes back.
    Hangs,
    /// The node accepted the transaction after this long.
    AcceptedAfter(std::time::Duration),
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
        let answer = Self::next(&self.sends);
        if let SendAnswer::AcceptedAfter(delay) = answer {
            tokio::time::sleep(delay).await;
        }
        match answer {
            SendAnswer::Accepted | SendAnswer::AcceptedAfter(_) => {
                *self.accepted.lock().unwrap() += 1;
                Ok(transaction.signatures[0])
            }
            SendAnswer::TimedOut => Err(crate::Error::Rpc(RpcError::Network {
                message: "operation timed out".to_owned(),
                is_timeout: true,
            })),
            SendAnswer::Fails(error) => Err(error),
            SendAnswer::Hangs => std::future::pending().await,
        }
    }

    async fn status(
        &self,
        _signature: &Signature,
    ) -> crate::Result<(u64, Option<TransactionStatus>)> {
        Self::next(&self.statuses).map(|status| (self.read_slot, status))
    }

    async fn finalized_tip(&self) -> crate::Result<FinalizedTip> {
        Ok(FinalizedTip {
            slot: self.tip_slot,
            block_height: self.block_height,
        })
    }

    async fn transaction(
        &self,
        _signature: &Signature,
    ) -> crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>> {
        Ok(self.executed.clone().map(executed))
    }
}
