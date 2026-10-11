# Home page.

# Source: templates/pages/home.html, scripts/pages/home.js, scripts/pages/home/portfolio_calendar.js

## Portfolio overview

home-portfolio-title = Portfolio value
# Label before the day's worth change.
home-portfolio-change-label = Today
# Range label under the worth trend; the day starts at 00:00 UTC.
home-trend-range = Since 00:00 UTC
# Tooltip of the day change when the start of the day and now priced different holdings, so the two worths do not compare.
home-portfolio-change-incomparable =
    .title = Some token prices were missing at the start of the day or are missing now, so today's change isn't shown.
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
home-calendar-win-days = Win days
home-calendar-loss-days = Loss days
home-calendar-best-day = Best day
# Column header of the weekly P&L totals.
home-calendar-week = Week
# Accessible name of a traded day: $date is the formatted date, $pnl its signed P&L.
home-calendar-day-label = { $date }, { $pnl }
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
home-exposure-value = Current value
home-exposure-position = Position
# Hold time of the only open position.
home-exposure-held = Held
home-exposure-empty = No open positions
home-exposure-avg-size = Average size
home-exposure-avg-hold = Average hold
home-exposure-best = Best
home-exposure-worst = Worst
home-pipeline-title = Market pipeline
home-pipeline-tracked = Tracked
# Tokens with market data, which the filters evaluate.
home-pipeline-monitored = Monitored
home-pipeline-passed = Passed filters
home-pipeline-rejected = Rejected
home-pipeline-blacklisted = Blacklisted
# Tokens the pool service holds a live price for now.
home-pipeline-live-price = Live price
home-pipeline-ohlcv = OHLCV
