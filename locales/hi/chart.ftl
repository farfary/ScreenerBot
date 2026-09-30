# Chart vocabulary shared by the position details and token details charts.

chart-data = डेटा
chart-ohlc-open = O
chart-ohlc-high = H
chart-ohlc-low = L
chart-ohlc-close = C
chart-loading = चार्ट डेटा लोड हो रहा है...
chart-waiting = चार्ट डेटा की प्रतीक्षा है...
chart-marker-dca = DCA { $index }
chart-marker-exit-numbered = एग्ज़िट { $index }
chart-candles = कैंडल

chart-status-none = अभी कोई चार्ट डेटा नहीं
chart-status-ready = डेटा तैयार
chart-status-partial = हिस्ट्री जुटाई जा रही है…
chart-status-collecting = डेटा लाया जा रहा है…
chart-status-aria = चार्ट डेटा: { $summary }
chart-status-last-candle = आखिरी नई कैंडल
chart-status-checked = जाँच: { $ago }
chart-status-checking = जाँच हो रही है…
chart-status-not-checked = जाँच नहीं हुई
chart-status-updated = अपडेट: { $ago }
chart-status-no-candles = अभी कोई कैंडल नहीं
chart-status-column-timeframe = TF
chart-status-column-new = नई
chart-status-total-monitoring =
    { $count ->
        [one] { $count } कैंडल · निगरानी जारी
       *[other] { $count } कैंडल · निगरानी जारी
    }
chart-status-total-idle =
    { $count ->
        [one] { $count } कैंडल · निष्क्रिय
       *[other] { $count } कैंडल · निष्क्रिय
    }
