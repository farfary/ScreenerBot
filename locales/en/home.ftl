# Home page.

# Source: templates/pages/home.html, scripts/pages/home.js, scripts/pages/home/portfolio_calendar.js

## Portfolio overview

home-portfolio-title = Portfolio value
home-portfolio-today = today
home-stat-available = Available { -sol }
home-stat-holdings = Token holdings
home-stat-open-pnl = Open P&L
home-stat-realized-today = Realized today

# $count is the number of held tokens.
home-holdings-token-count =
    { $count ->
        [one] { $count } token
       *[other] { $count } tokens
    }
# $tokens is the held-token count text, $count the number of tokens with no price.
home-holdings-with-unpriced = { $tokens } · { $count } unpriced
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } held token has no price available and counts as 0 in the total
       *[other] { $count } held tokens have no price available and count as 0 in the total
    }

## Wallet address and QR code

home-wallet-copy =
    .title = Copy wallet address
    .aria-label = Copy wallet address
home-wallet-qr-open =
    .title = Show wallet QR code
    .aria-label = Show wallet QR code
home-wallet-qr-popover =
    .aria-label = Wallet QR code
home-wallet-qr-receive = Receive
home-wallet-qr-assets = { -sol } and SPL tokens
home-wallet-qr-close =
    .title = Close
    .aria-label = Close wallet QR code
home-wallet-qr-preparing = Preparing QR code
home-wallet-qr-unavailable = QR code unavailable
home-wallet-qr-image =
    .alt = QR code for the main wallet address

## Performance calendar

home-calendar-title = Performance calendar
home-calendar-previous =
    .title = Previous month
    .aria-label = Previous month
home-calendar-next =
    .title = Next month
    .aria-label = Next month
home-calendar-month-pnl = Month P&L
home-calendar-trades = Trades
home-calendar-pop-net-pnl = Net P&L
home-calendar-pop-win-rate = Win rate
# $rate is the formatted win percentage, $wins and $losses are counts.
home-calendar-pop-win-rate-value = { $rate } · { $wins }W / { $losses }L
home-calendar-pop-gross-profit = Gross profit
home-calendar-pop-gross-loss = Gross loss
home-calendar-pop-end-balance = End balance

## Position exposure and market pipeline

home-operations =
    .aria-label = Portfolio and market status
home-exposure-title = Position exposure
home-exposure-open = open
home-exposure-invested = Invested
home-exposure-avg-size = Average size
home-exposure-avg-hold = Average hold
home-exposure-best = Best
home-exposure-worst = Worst
home-pipeline-title = Market pipeline
home-pipeline-tracked = Tracked
home-pipeline-priced = Priced
home-pipeline-passed = Passed filters
home-pipeline-rejected = Rejected
home-pipeline-blacklisted = Blacklisted
home-pipeline-ohlcv = OHLCV
