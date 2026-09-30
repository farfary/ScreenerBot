chart-data = Dados
chart-ohlc-open = A
chart-ohlc-high = M
chart-ohlc-low = Mí
chart-ohlc-close = F
chart-loading = Carregando dados do gráfico...
chart-waiting = Aguardando dados do gráfico...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Saída { $index }
chart-candles = Candles

chart-status-none = Ainda sem dados do gráfico
chart-status-ready = Dados prontos
chart-status-partial = Coletando histórico…
chart-status-collecting = Buscando dados…
chart-status-aria = Dados do gráfico: { $summary }
chart-status-last-candle = Último candle novo
chart-status-checked = verificado { $ago }
chart-status-checking = verificando…
chart-status-not-checked = não verificado
chart-status-updated = atualizado { $ago }
chart-status-no-candles = ainda sem candles
chart-status-column-timeframe = TF
chart-status-column-new = Novos
chart-status-total-monitoring =
    { $count ->
        [one] { $count } candle · monitorando
        [many] { $count } candles · monitorando
       *[other] { $count } candles · monitorando
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } candle · inativo
        [many] { $count } candles · inativo
       *[other] { $count } candles · inativo
    }
