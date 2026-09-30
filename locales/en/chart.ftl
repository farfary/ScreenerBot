# Chart vocabulary shared by the position details and token details charts.

chart-data = Data
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = L
chart-ohlc-close = C
chart-loading = Loading chart data...
chart-waiting = Waiting for chart data...
# $index is the 1-based number of the add or exit.
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Exit { $index }
chart-candles = Candles

# Per-timeframe data status chip and tooltip (scripts/ui/chart_data.js)
# State ids are computed in renderOhlcvStatus.

chart-status-none = No chart data yet
chart-status-ready = Data ready
chart-status-partial = Collecting history…
chart-status-collecting = Fetching data…
# $summary is the state label.
chart-status-aria = Chart data: { $summary }
chart-status-last-candle = Last new candle
# $ago is the formatted time since the last check or update.
chart-status-checked = checked { $ago }
chart-status-checking = checking…
chart-status-not-checked = not checked
chart-status-updated = updated { $ago }
chart-status-no-candles = no candles yet
chart-status-column-timeframe = TF
chart-status-column-new = New
chart-status-total-monitoring =
    { $count ->
        [one] { $count } candle · monitoring
       *[other] { $count } candles · monitoring
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } candle · idle
       *[other] { $count } candles · idle
    }
