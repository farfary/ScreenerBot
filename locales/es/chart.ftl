chart-data = Datos
chart-ohlc-open = A
chart-ohlc-high = M
chart-ohlc-low = m
chart-ohlc-close = C
chart-loading = Cargando datos del gráfico...
chart-waiting = Esperando datos del gráfico...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Salida { $index }
chart-candles = Velas

chart-status-none = Aún no hay datos del gráfico
chart-status-ready = Datos listos
chart-status-partial = Recopilando historial…
chart-status-collecting = Obteniendo datos…
chart-status-aria = Datos del gráfico: { $summary }
chart-status-last-candle = Última vela nueva
chart-status-checked = comprobado { $ago }
chart-status-checking = comprobando…
chart-status-not-checked = sin comprobar
chart-status-updated = actualizado { $ago }
chart-status-no-candles = aún sin velas
chart-status-column-timeframe = Marco
chart-status-column-new = Nuevas
chart-status-total-monitoring =
    { $count ->
        [one] { $count } vela · en seguimiento
        [many] { $count } velas · en seguimiento
       *[other] { $count } velas · en seguimiento
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } vela · inactivo
        [many] { $count } velas · inactivo
       *[other] { $count } velas · inactivo
    }
