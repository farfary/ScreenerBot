/// Rent-exempt minimum for ATA (~0.00089088 SOL)
const ATA_RENT_EXEMPT: f64 = 0.00089088;

/// Minimum SOL balance to operate a wallet (for transaction fees)
const MIN_SOL_FOR_OPERATIONS: f64 = 0.005;

pub(super) fn sol_topup_needed(sol_balance: f64) -> (bool, f64) {
    if sol_balance < MIN_SOL_FOR_OPERATIONS {
        (true, MIN_SOL_FOR_OPERATIONS - sol_balance)
    } else {
        (false, 0.0)
    }
}

pub(super) fn reclaimable_ata_rent(empty_ata_count: u32) -> f64 {
    empty_ata_count as f64 * ATA_RENT_EXEMPT
}

