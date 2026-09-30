chart-data = Данные
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = L
chart-ohlc-close = C
chart-loading = Загрузка данных графика...
chart-waiting = Ожидание данных графика...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Выход { $index }
chart-candles = Свечи

## Per-timeframe status

chart-status-none = Данных графика пока нет
chart-status-ready = Данные готовы
chart-status-partial = Сбор истории…
chart-status-collecting = Загрузка данных…
chart-status-aria = Данные графика: { $summary }
chart-status-last-candle = Последняя новая свеча
chart-status-checked = проверено { $ago }
chart-status-checking = проверка…
chart-status-not-checked = не проверялось
chart-status-updated = обновлено { $ago }
chart-status-no-candles = свечей пока нет
chart-status-column-timeframe = ТФ
chart-status-column-new = Новые
chart-status-total-monitoring =
    { $count ->
        [one] { $count } свеча · мониторинг
        [few] { $count } свечи · мониторинг
        [many] { $count } свечей · мониторинг
       *[other] { $count } свечи · мониторинг
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } свеча · простой
        [few] { $count } свечи · простой
        [many] { $count } свечей · простой
       *[other] { $count } свечи · простой
    }
