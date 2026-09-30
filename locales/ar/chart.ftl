chart-data = البيانات
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = L
chart-ohlc-close = C
chart-loading = جارٍ تحميل بيانات المخطط...
chart-waiting = في انتظار بيانات المخطط...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = خروج { $index }
chart-candles = الشموع

chart-status-none = لا توجد بيانات مخطط بعد
chart-status-ready = البيانات جاهزة
chart-status-partial = جارٍ جمع السجل…
chart-status-collecting = جارٍ جلب البيانات…
chart-status-aria = بيانات المخطط: { $summary }
chart-status-last-candle = آخر شمعة جديدة
chart-status-checked = تم الفحص { $ago }
chart-status-checking = جارٍ الفحص…
chart-status-not-checked = لم يُفحص
chart-status-updated = تم التحديث { $ago }
chart-status-no-candles = لا توجد شموع بعد
chart-status-column-timeframe = الإطار
chart-status-column-new = جديد
chart-status-total-monitoring =
    { $count ->
        [zero] { $count } شمعة · قيد المراقبة
        [one] { $count } شمعة · قيد المراقبة
        [two] { $count } شمعتان · قيد المراقبة
        [few] { $count } شموع · قيد المراقبة
        [many] { $count } شمعة · قيد المراقبة
       *[other] { $count } شمعة · قيد المراقبة
    }
chart-status-total-idle =
    { $count ->
        [zero] { $count } شمعة · خامل
        [one] { $count } شمعة · خامل
        [two] { $count } شمعتان · خامل
        [few] { $count } شموع · خامل
        [many] { $count } شمعة · خامل
       *[other] { $count } شمعة · خامل
    }
