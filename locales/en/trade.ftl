# Trade dialog messages.

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = Couldn't fetch a quote — check your connection and try again

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = Strategy Signal
trade-reason-manual-entry = Manual Entry
trade-reason-force-buy = Force Buy
trade-reason-copy-buy = Copy Buy
trade-reason-dca-scheduled = DCA Scheduled
trade-reason-take-profit = Take Profit
trade-reason-stop-loss = Stop Loss
trade-reason-trailing-stop = Trailing Stop
trade-reason-time-override = Time Override
trade-reason-strategy-exit = Strategy Exit
trade-reason-llm-analysis-exit = LLM Analysis Exit
trade-reason-manual-exit = Manual Exit
trade-reason-risk-management = Risk Management
trade-reason-blacklisted = Blacklisted
trade-reason-force-sell = Force Sell
trade-reason-copy-sell = Copy Sell
trade-reason-closed-externally = Closed Externally
trade-reason-wallet-history = Wallet History
trade-reason-exit-retry-pending = Exit Retry Pending
trade-reason-synthetic-exit-permanent-failure = Synthetic Exit Permanent Failure
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (pending verification)
# $note is the operator text of a force close.
trade-reason-force-closed = Force closed: { $note }
