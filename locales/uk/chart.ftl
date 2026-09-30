chart-data = Дані
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = L
chart-ohlc-close = C
chart-loading = Завантаження даних графіка...
chart-waiting = Очікування даних графіка...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = Вихід { $index }
chart-candles = Свічки

chart-status-none = Даних графіка ще немає
chart-status-ready = Дані готові
chart-status-partial = Збирання історії…
chart-status-collecting = Отримання даних…
chart-status-aria = Дані графіка: { $summary }
chart-status-last-candle = Остання нова свічка
chart-status-checked = перевірено { $ago }
chart-status-checking = перевірка…
chart-status-not-checked = не перевірено
chart-status-updated = оновлено { $ago }
chart-status-no-candles = свічок ще немає
chart-status-column-timeframe = ТФ
chart-status-column-new = Нові
chart-status-total-monitoring =
    { $count ->
        [one] { $count } свічка · моніторинг
        [few] { $count } свічки · моніторинг
        [many] { $count } свічок · моніторинг
       *[other] { $count } свічки · моніторинг
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } свічка · очікування
        [few] { $count } свічки · очікування
        [many] { $count } свічок · очікування
       *[other] { $count } свічки · очікування
    }
