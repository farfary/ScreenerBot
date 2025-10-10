/// Transactions activity topic messages
use serde_json::json;

use crate::transactions::database::TransactionListRow;
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert transaction row to WebSocket envelope
pub fn transaction_to_envelope(row: &TransactionListRow, seq: u64) -> WsEnvelope {
    let payload = json!({
        "action": "new",
        "transaction": {
            "signature": row.signature,
            "timestamp": row.timestamp.to_rfc3339(),
            "slot": row.slot,
            "status": row.status,
            "success": row.success,
            "direction": row.direction,
            "type": row.transaction_type,
            "token_mint": row.token_mint,
            "token_symbol": row.token_symbol,
            "router": row.router,
            "sol_delta": row.sol_delta,
            "fee_sol": row.fee_sol,
            "fee_lamports": row.fee_lamports,
            "ata_rents": row.ata_rents,
            "instructions_count": row.instructions_count,
        }
    });

    WsEnvelope::new(Topic::TransactionsActivity, seq, payload)
}
