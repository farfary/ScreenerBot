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
