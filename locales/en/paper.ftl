# Paper trading messages shared by copy trading and strategies.

# Shared live-readiness checks: a title and a detail per check. Ids come from
# src/trader/paper/readiness.rs. $realized is a signed SOL figure.
paper-readiness-history = Paper history
paper-readiness-history-met =
    { $count ->
        [one] { $count } closed paper round, { $needed } needed
       *[other] { $count } closed paper rounds, { $needed } needed
    }
paper-readiness-history-short = { $count } of { $needed } closed paper rounds
paper-readiness-profit = Profitable in paper
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realized over { $count } round, { $wins } won
       *[other] { $realized } { -sol } realized over { $count } rounds, { $wins } won
    }
paper-readiness-exits = Exits fill
paper-readiness-exits-ok = Recent paper exits found a fill
paper-readiness-exits-failing = Paper exits in a row without a fill: { $count }

# Why live trading is unavailable, shared by copy trading and strategies.
paper-live-block-setup-incomplete = Finish wallet and RPC setup first
paper-live-block-force-stop = The emergency stop is engaged
