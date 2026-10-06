# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = Buy
transactions-type-sell = Sell
transactions-type-swap = Swap
transactions-type-sol-transfer = SOL transfer
transactions-type-token-transfer = Token transfer
transactions-type-transfer = Transfer
transactions-type-dust = Dust
transactions-type-spam = Spam
transactions-type-ata-create = Account opened
transactions-type-ata-close = Rent reclaimed
transactions-type-ata = Token account
transactions-type-liquidity-add = Add liquidity
transactions-type-liquidity-remove = Remove liquidity
transactions-type-nft = NFT
transactions-type-program = Program call
transactions-type-compute = Compute
transactions-type-failed = Failed
transactions-type-unknown = Unclassified

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Spam airdrop ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = All Types
transactions-filter-transfer = Transfers
transactions-filter-ata = Rent & accounts
transactions-filter-liquidity = Liquidity
transactions-filter-program = Program calls

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = Incoming
transactions-direction-outgoing = Outgoing
transactions-direction-internal = Internal
transactions-direction-unknown = Unclassified

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = Pending
transactions-status-confirmed = Confirmed
transactions-status-finalized = Finalized
transactions-status-failed = Failed
transactions-status-success = Success
transactions-status-unknown = Unknown

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = Creation
transactions-ata-operation-closure = Closure

## Transactions page (pages/transactions.js)

transactions-toolbar-title = Transaction history
transactions-search =
    .placeholder = Search signatures…
    .aria-label = Search transaction signatures
transactions-load-failed = Could not refresh transactions
transactions-summary-total = Total
transactions-summary-estimate = Estimate
transactions-summary-success = Success
transactions-summary-failed = Failed
transactions-filter-wallet = Wallet
transactions-filter-type = Type
transactions-filter-direction = Direction
transactions-filter-status = Status
transactions-filter-all-directions = All Directions
transactions-filter-all-statuses = All Statuses
transactions-wallet-main = Main wallet
transactions-col-time = Time
transactions-col-signature = Signature
transactions-col-type = Type
transactions-col-direction = Direction
transactions-col-status = Status
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Fees ({ -sol })
transactions-col-token = Token
transactions-col-router = Router
transactions-col-instructions = Instr.

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = Copy signature
transactions-dialog-close =
    .title = Close (ESC)
transactions-dialog-tabs-label = Transaction details sections
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Fee:
transactions-dialog-loading = Loading...
transactions-dialog-loading-details = Loading transaction details...
transactions-dialog-load-failed = Failed to load transaction details
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = Failed to load transaction details: { $reason }
transactions-dialog-not-found = Transaction not found
transactions-dialog-tab-overview = Overview
transactions-dialog-tab-balances = Balances
transactions-dialog-tab-instructions = Instructions
transactions-dialog-tab-logs = Logs
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Raw
transactions-dialog-unknown = Unknown
transactions-dialog-unknown-asset = Unknown asset
transactions-dialog-unavailable = Unavailable

## Transaction details dialog: overview

transactions-dialog-failed-title = Transaction failed
transactions-dialog-no-program-error = No program error was provided.
transactions-dialog-story-title = What happened
# $router is the routing program name.
transactions-dialog-router-via = via { $router }
transactions-dialog-flow-paid = Paid
transactions-dialog-flow-received = Received
transactions-dialog-flow-from = From
transactions-dialog-flow-to = To
transactions-dialog-flow-amount = Amount
transactions-dialog-net-wallet-change = Net wallet change:
transactions-dialog-processed = Processed on Solana
transactions-dialog-execution-title = Execution
transactions-dialog-metric-execution-price = Execution price
transactions-dialog-metric-effective-received = Effective received
transactions-dialog-metric-effective-spent = Effective spent
transactions-dialog-metric-network-fee = Network fee
transactions-dialog-metric-estimated-pnl = Estimated P&L
transactions-dialog-metric-net-native-change = Net { -sol } change
transactions-dialog-route-title = Route and assets
transactions-dialog-route-router = Router
transactions-dialog-route-input-asset = Input asset
transactions-dialog-route-output-asset = Output asset
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Program
transactions-dialog-tech-title = Technical details
transactions-dialog-tech-summary = Signature, slot and resources
transactions-dialog-tech-signature = Signature
transactions-dialog-tech-timestamp = Timestamp
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Exact fee
transactions-dialog-tech-accounts = Accounts
transactions-dialog-tech-instructions = Instructions
transactions-dialog-tech-compute-units = Compute units
transactions-dialog-tech-token-decimals = Token decimals

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = { -sol } Balance Changes
transactions-dialog-balances-native-empty = No { -sol } balance changes
transactions-dialog-balances-token-title = Token Balance Changes
transactions-dialog-balances-token-empty = No token balance changes
transactions-dialog-balances-net-native = Net { -sol } Change
transactions-dialog-balances-fee = Transaction Fee
transactions-dialog-col-account = Account
transactions-dialog-col-token = Token
transactions-dialog-col-mint = Mint Address
transactions-dialog-col-pre-balance = Pre Balance
transactions-dialog-col-post-balance = Post Balance
transactions-dialog-col-change = Change
transactions-dialog-col-type = Type
transactions-dialog-col-rent = Rent ({ -sol })
transactions-dialog-instructions-empty = No instructions found
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } instruction
       *[other] { $count } instructions
    }
transactions-dialog-instruction-program-id = Program ID
transactions-dialog-instruction-accounts = Accounts ({ $count })
transactions-dialog-instruction-data = Data
transactions-dialog-logs-empty = No logs available
transactions-dialog-logs-filter = Filter logs...
transactions-dialog-logs-no-match = No matching logs
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } log
       *[other] { $count } logs
    }
transactions-dialog-ata-empty = No ATA operations in this transaction
transactions-dialog-ata-summary-title = ATA Analysis Summary
transactions-dialog-ata-creations = Creations
transactions-dialog-ata-closures = Closures
transactions-dialog-ata-rent-spent = Rent Spent
transactions-dialog-ata-rent-recovered = Rent Recovered
transactions-dialog-ata-net-rent = Net Rent Impact
transactions-dialog-ata-operations-title = ATA Operations ({ $count })
transactions-dialog-raw-copy = Copy JSON
transactions-dialog-raw-empty = No raw data available
