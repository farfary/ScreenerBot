positions-state-reason-position-created = पोज़िशन बनाई गई

positions-status-open = खुली
positions-status-closed = बंद
positions-status-archived = आर्काइव
positions-origin-copy = कॉपी
positions-origin-manual = मैन्युअल
positions-origin-wallet = वॉलेट
positions-origin-copy-link =
    .title = इस पोज़िशन को खोलने वाला कॉपी टास्क खोलें
positions-holding-frozen = फ़्रीज़
    .title = मिंट अथॉरिटी ने यह टोकन अकाउंट फ़्रीज़ कर दिया है - बैलेंस ट्रांसफ़र या बेचा नहीं जा सकता
positions-toolbar-total = कुल
positions-toolbar-delete-all = सभी हटाएं
positions-search-placeholder = सिंबल या मिंट से खोजें...
positions-filter-origin = स्रोत
positions-filter-origin-all = सभी स्रोत
positions-filter-origin-auto = ऑटो ट्रेडर
positions-filter-origin-copy = कॉपी ट्रेडिंग
positions-delete-all-tooltip = सभी आर्काइव पोज़िशन हमेशा के लिए हटाएं

positions-column-token = टोकन
positions-column-archived-at = आर्काइव
positions-column-entry-time = एंट्री समय
positions-column-exit-time = एग्ज़िट समय
positions-column-avg-entry = औसत एंट्री ({ -sol })
positions-column-avg-exit = औसत एग्ज़िट ({ -sol })
positions-column-current-price = वर्तमान ({ -sol })
positions-column-total-invested = कुल निवेश
positions-column-proceeds = प्राप्ति
positions-column-pnl = PnL
positions-column-pnl-percent = PnL %
positions-column-size = साइज़
positions-column-dca = DCA
positions-column-exits = एग्ज़िट
positions-column-unrealized-pnl = अवास्तविक PnL
positions-column-unrealized-percent = अवास्तविक %

positions-unknown-basis = इस वॉलेट के इतिहास में कॉस्ट बेसिस नहीं है (एयरड्रॉप, USD-कोटेड फ़िल, या बिना SOL लेग वाला स्वैप)
positions-unknown-history = यह राउंड ऑन-चेन बैलेंस से मेल नहीं खाता
positions-dca-count =
    { $count ->
        [one] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } एग्ज़िट
       *[other] { $count } एग्ज़िट
    }

positions-action-add =
    .title = पोज़िशन में ऐड करें (DCA)
    .aria-label = पोज़िशन में ऐड करें
positions-action-sell =
    .title = बेचें (पूरी या % आंशिक)
    .aria-label = पोज़िशन बेचें
positions-action-sell-frozen = मिंट अथॉरिटी ने फ़्रीज़ किया है - यह होल्डिंग बेची नहीं जा सकती
positions-action-remove =
    .title = हटाएं (आर्काइव या डिलीट)
    .aria-label = पोज़िशन हटाएं
positions-action-restore =
    .title = खुली/बंद में वापस लाएं
    .aria-label = पोज़िशन वापस लाएं
positions-action-delete =
    .title = हमेशा के लिए हटाएं
    .aria-label = हमेशा के लिए हटाएं
positions-action-in-progress = प्रगति में…

positions-caption-buying = खरीदी जा रही है
# $step is the label of the current action step.
positions-caption-buying-step = खरीदी जा रही है · { $step }
positions-caption-selling = बेची जा रही है
positions-caption-selling-step = बेची जा रही है · { $step }
positions-caption-closing = बंद हो रही है
positions-caption-failed = विफल
positions-caption-failed-detail = विफल · { $error }
positions-step-adding = ऐड हो रहा है
positions-pending-buying = खरीदी जा रही है…
positions-pending-buy-failed = खरीद विफल

positions-load-failed = पोज़िशन रीफ़्रेश नहीं हो सकीं
positions-toast-not-found = पोज़िशन डेटा नहीं मिला
positions-toast-deleted = पोज़िशन हटा दी गई
positions-toast-archived = पोज़िशन आर्काइव की गई
positions-toast-restored = पोज़िशन वापस लाई गई
positions-action-failed = कार्रवाई विफल
positions-delete-title = पोज़िशन हमेशा के लिए हटाएं
positions-delete-message = { $symbol } को हमेशा के लिए हटाएं? इससे पोज़िशन और उसका इतिहास डेटाबेस से हट जाएगा और इसे वापस नहीं किया जा सकता। आपके ट्रांज़ैक्शन और टोकन डेटा पर कोई असर नहीं पड़ेगा।
positions-delete-confirm = हमेशा के लिए हटाएं
positions-delete-all-title = सभी आर्काइव पोज़िशन हटाएं
positions-delete-all-message =
    { $count ->
        [one] सभी { $count } आर्काइव पोज़िशन हमेशा के लिए हटाएं? इसे वापस नहीं किया जा सकता। ट्रांज़ैक्शन और टोकन डेटा पर कोई असर नहीं पड़ेगा।
       *[other] सभी { $count } आर्काइव पोज़िशन हमेशा के लिए हटाएं? इसे वापस नहीं किया जा सकता। ट्रांज़ैक्शन और टोकन डेटा पर कोई असर नहीं पड़ेगा।
    }
positions-delete-all-message-empty = सभी आर्काइव पोज़िशन हमेशा के लिए हटाएं? इसे वापस नहीं किया जा सकता।
positions-delete-all-confirm = सभी हटाएं
positions-delete-all-done =
    { $count ->
        [one] { $count } आर्काइव पोज़िशन हटाई गई
       *[other] { $count } आर्काइव पोज़िशन हटाई गईं
    }
positions-delete-all-failed = आर्काइव पोज़िशन हटाने में विफल

positions-remove-title = पोज़िशन हटाएं
positions-remove-open-warning = <strong>यह पोज़िशन अभी खुली है।</strong> बॉट यह टोकन होल्ड कर रहा है। इसे हटाने से ट्रेड स्लॉट खाली हो जाता है और ट्रैकिंग बंद हो जाती है — लेकिन यह टोकन <strong>नहीं</strong> बेचता। अपना { -sol } वापस चाहिए तो पहले बेचें।
positions-remove-modes =
    .aria-label = हटाने का मोड
positions-remove-archive = आर्काइव करें
positions-remove-recommended = अनुशंसित
positions-remove-archive-description = इसे आर्काइव टैब में छिपाएं। कभी भी वापस लाया जा सकता है — कुछ बेचा नहीं जाता और सभी ट्रेड रिकॉर्ड में रहते हैं।
positions-remove-delete = हमेशा के लिए हटाएं
positions-remove-delete-description = इस पोज़िशन और उसके पूरे इतिहास को डेटाबेस से मिटाएं।
positions-remove-danger = इससे पोज़िशन और उसका इतिहास हमेशा के लिए हट जाएगा। <strong>इसे वापस नहीं किया जा सकता।</strong> आपके ट्रांज़ैक्शन और टोकन डेटा पर कोई असर नहीं पड़ेगा।
positions-remove-confirm-archive = पोज़िशन आर्काइव करें

positions-management-changed = पोज़िशन मैनेजमेंट { $mode } पर सेट किया गया
positions-details-load-failed = पोज़िशन विवरण लोड करने में विफल
positions-details-mint-label = मिंट एड्रेस
positions-details-management-failed = पोज़िशन मैनेजमेंट अपडेट करने में विफल
positions-details-favorite-add =
    .title = पसंदीदा में जोड़ें
    .aria-label = पसंदीदा में जोड़ें
positions-details-favorite-remove =
    .title = पसंदीदा से हटाएं
    .aria-label = पसंदीदा से हटाएं
positions-details-view-solscan =
    .title = { -solscan } पर देखें
    .aria-label = { -solscan } पर टोकन देखें
positions-details-close =
    .title = बंद करें (Esc)
    .aria-label = बंद करें
positions-details-chart-section =
    .aria-label = प्राइस चार्ट
positions-details-loading-chart = चार्ट लोड हो रहा है...
positions-details-activity-section =
    .aria-label = गतिविधि
positions-details-activity-title = गतिविधि
positions-details-split-handle =
    .aria-label = चार्ट और गतिविधि का आकार बदलें
positions-details-activity-pane =
    .aria-label = गतिविधि पैनल
positions-details-activity-expand =
    .title = गतिविधि बड़ी करें
    .aria-label = गतिविधि बड़ी करें
positions-details-summary-section =
    .aria-label = पोज़िशन सारांश
positions-details-loading = पोज़िशन लोड हो रही है...

positions-management-auto-trader = ऑटो ट्रेडर
positions-management-user-only = केवल यूज़र
positions-management-copy-task = कॉपी टास्क
positions-management-hybrid = हाइब्रिड
positions-pane-show-chart = चार्ट दिखाएं
positions-pane-show-activity = गतिविधि दिखाएं
positions-pane-restore-activity = गतिविधि वापस लाएं
positions-pane-expand-chart =
    .title = चार्ट बड़ा करें
    .aria-label = चार्ट बड़ा करें

positions-risk-low = कम जोखिम
positions-risk-medium = मध्यम जोखिम
positions-risk-high = उच्च जोखिम
positions-risk-unknown = जोखिम अज्ञात
positions-busy-buying = खरीद जारी है…
positions-busy-selling = बिक्री जारी है…
positions-busy-closing = बंद करना जारी है…
positions-header-avg-entry = औसत एंट्री
positions-header-buy-count =
    { $count ->
        [one] { $count } खरीद
       *[other] { $count } खरीद
    }
positions-header-exit-price = एग्ज़िट प्राइस
positions-header-closed-ago = बंद: { $ago }
positions-header-realized-pnl = वास्तविक लाभ-हानि
positions-header-usd-note = आज के { -sol } प्राइस पर USD
positions-header-returned = वापस मिला
positions-header-of-invested = { $amount } निवेश में से
positions-header-price = प्राइस
positions-header-last-price = अंतिम प्राइस
positions-header-pool-ago = पूल · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = अवास्तविक लाभ-हानि
positions-header-pnl-last-price = अंतिम प्राइस पर लाभ-हानि
positions-header-value = वैल्यू
positions-header-last-value = अंतिम वैल्यू
positions-header-invested = { $amount } निवेश
positions-header-origin-hint = यह पोज़िशन कैसे खोली गई
positions-header-risk-hint = { -rugcheck } स्कोर — कम स्कोर यानी ज़्यादा सुरक्षित
positions-header-frozen = फ़्रीज़
    .title = मिंट अथॉरिटी ने यह होल्डिंग फ़्रीज़ कर दी है
positions-header-managed-by = प्रबंधक
positions-header-management-select =
    .aria-label = पोज़िशन मैनेजमेंट

positions-origin-unknown = अज्ञात
positions-origin-copied-task = कॉपी · टास्क { $task }
positions-origin-manual-entry = मैन्युअल एंट्री
positions-origin-wallet-entry = वॉलेट एंट्री
positions-origin-auto-strategy = ऑटो · { $strategy }
positions-origin-auto-entry = ऑटो एंट्री

positions-pending-adding = ऐड हो रहा है
positions-pending-adding-amount = { $amount } ऐड हो रहा है
positions-pending-selling = बिक्री हो रही है
positions-pending-selling-percent = { $percent } बिक्री हो रही है
positions-pending-confirming = { $label } · पुष्टि हो रही है
    .title = सबमिट हो चुका है और ऑन-चेन पुष्टि की प्रतीक्षा में है। सत्यापन के बाद आंकड़े अपडेट होंगे।

positions-trade-add = ऐड करें
    .title = पोज़िशन में ऐड करें
positions-trade-sell = बेचें
    .title = पोज़िशन का हिस्सा बेचें
positions-trade-close = पोज़िशन बंद करें
    .title = सब बेचकर बंद करें
positions-trade-token = टोकन विवरण
    .title = टोकन विवरण खोलें

positions-favorite-token-fallback = टोकन
positions-favorite-added = { $symbol } पसंदीदा में जोड़ा गया
positions-favorite-removed = { $symbol } पसंदीदा से हटाया गया
positions-favorite-add-failed = पसंदीदा में जोड़ने में विफल
positions-favorite-remove-failed = पसंदीदा से हटाने में विफल
positions-favorite-update-failed = पसंदीदा अपडेट करने में विफल

positions-summary-position = पोज़िशन
positions-summary-price-path = प्राइस पथ
positions-summary-network-fees = नेटवर्क फ़ीस
positions-summary-risk = जोखिम
positions-summary-market = मार्केट
positions-summary-market-now = मार्केट अभी
positions-summary-links = लिंक
positions-fact-tokens-fallback = टोकन
positions-fact-bought = खरीदा
positions-fact-holding = होल्डिंग
positions-fact-sold = बेचा
positions-fact-realized = वास्तविक
positions-fact-opened = खोली गई
positions-fact-closed = बंद हुई
positions-fact-reason = कारण
positions-fact-archived = आर्काइव
positions-fact-entry = एंट्री
positions-fact-exit = एग्ज़िट
positions-fact-total = कुल
positions-fact-verified = ऑन-चेन सत्यापित
positions-fact-confirming = पुष्टि हो रही है
positions-fact-share-of-bought = खरीदे का { $percent }
positions-fact-share-of-invested = निवेश का { $percent }
positions-fact-entry-count =
    { $count ->
        [0] 1 एंट्री
        [one] 1 एंट्री + { $count } ऐड
       *[other] 1 एंट्री + { $count } ऐड
    }
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } आंशिक एग्ज़िट · { $returned } वापस
       *[other] { $count } आंशिक एग्ज़िट · { $returned } वापस
    }
positions-fact-held = होल्ड: { $age }
positions-fact-vs-entry = एंट्री के मुकाबले { $percent }
positions-fact-exit-vs-peak = एग्ज़िट बनाम पीक
positions-fact-now-vs-peak = अभी बनाम पीक
positions-fact-entry-range = एंट्री रेंज
positions-range-low = न्यूनतम
positions-range-peak = पीक
positions-range-now = अभी
positions-range-label-exit = न्यूनतम और पीक के बीच एंट्री और एग्ज़िट प्राइस
positions-range-label-now = न्यूनतम और पीक के बीच एंट्री और वर्तमान प्राइस
positions-fact-mint-authority = मिंट अथॉरिटी
positions-fact-freeze-authority = फ़्रीज़ अथॉरिटी
positions-fact-active = सक्रिय
positions-fact-pool = पूल
positions-fact-pool-liquidity = { $amount } { -sol } लिक्विडिटी
positions-fact-market-cap = मार्केट कैप
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = लिक्विडिटी
positions-fact-volume-24h = वॉल्यूम 24h
positions-fact-price-change = प्राइस बदलाव
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = होल्डर
positions-link-website = वेबसाइट
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

positions-activity-load-failed = गतिविधि लोड नहीं हो सकी
positions-activity-loading = गतिविधि लोड हो रही है...
positions-activity-empty = इस वॉलेट में इस टोकन के साथ अभी तक कुछ नहीं हुआ है
positions-activity-filter-empty = इस फ़िल्टर से कोई गतिविधि मेल नहीं खाती
positions-activity-round-count =
    { $count ->
        [one] { $count } राउंड
       *[other] { $count } राउंड
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } इवेंट
       *[other] { $count } इवेंट
    }
positions-activity-pending-count = { $count } लंबित
positions-activity-failed-count = { $count } विफल
positions-filter-all = सभी
positions-filter-trades = ट्रेड
positions-filter-buys = खरीद
positions-filter-sells = बिक्री
positions-filter-wallet = वॉलेट
positions-filter-issues = समस्याएं
positions-activity-filters =
    .aria-label = गतिविधि फ़िल्टर करें
positions-activity-totals =
    .aria-label = इस टोकन के सभी राउंड
positions-activity-realized-all = वास्तविक, सभी राउंड
positions-activity-invested = निवेश
positions-activity-returned = वापस मिला
positions-activity-opened = खोली गई { $when }
positions-activity-round-title = पोज़िशन { $index }
positions-activity-this-position = यह पोज़िशन
positions-activity-dates-unavailable = तारीखें उपलब्ध नहीं
positions-activity-wallet-title = वॉलेट ट्रांज़ैक्शन
positions-activity-outside =
    { $count ->
        [one] किसी पोज़िशन से बाहर · { $range } · { $count } इवेंट
       *[other] किसी पोज़िशन से बाहर · { $range } · { $count } इवेंट
    }
positions-details-signature-label = सिग्नेचर

positions-state-open = पोज़िशन खुली
positions-state-closing = पोज़िशन बंद हो रही है
positions-state-closed = पोज़िशन बंद
positions-state-exit-pending = पोज़िशन एग्ज़िट लंबित
positions-state-exit-failed = पोज़िशन एग्ज़िट विफल
positions-state-phantom = पोज़िशन फ़ैंटम
positions-state-reconciling = पोज़िशन मिलान में

positions-event-kind-entry = एंट्री
positions-event-kind-dca = ऐड
positions-event-kind-partial-exit = आंशिक एग्ज़िट
positions-event-kind-exit = एग्ज़िट
positions-event-kind-buy = वॉलेट खरीद
positions-event-kind-sell = वॉलेट बिक्री
positions-event-kind-transfer = ट्रांसफ़र
positions-event-kind-ata = टोकन अकाउंट
positions-event-kind-other = ट्रांज़ैक्शन
positions-event-state-pending = लंबित
positions-event-state-failed = विफल
positions-event-state-synthetic = सिंथेटिक
positions-chain-status-failed-detail = विफल: { $error }
positions-event-tokens-fallback = टोकन
positions-event-entry-submitted = { $amount } की खरीद सबमिट की गई
positions-event-entry-for = { $sol } में { $amount } खरीदे गए
positions-event-entry = { $amount } खरीदे गए
positions-event-dca-submitted = { $amount } का ऐड सबमिट किया गया
positions-event-dca-for = { $sol } में { $amount } ऐड किए गए
positions-event-dca = { $amount } ऐड किए गए
positions-event-partial-exit-submitted-percent = { $amount } का { $percent } आंशिक एग्ज़िट सबमिट किया गया
positions-event-partial-exit-submitted = { $amount } का आंशिक एग्ज़िट सबमिट किया गया
positions-event-sold-percent-for = { $sol } में { $amount } ({ $percent }) बेचे गए
positions-event-sold-percent = { $amount } ({ $percent }) बेचे गए
positions-event-sold-for = { $sol } में { $amount } बेचे गए
positions-event-sold = { $amount } बेचे गए
positions-event-exit-submitted = पूरी पोज़िशन का एग्ज़िट सबमिट किया गया
positions-event-exit-for = { $sol } में { $amount } बेचकर बंद की गई
positions-event-exit-closed = पोज़िशन बंद की गई
positions-event-wallet-bought = वॉलेट ने अन्यत्र { $amount } खरीदे
positions-event-wallet-sold = वॉलेट ने अन्यत्र { $amount } बेचे
positions-event-received = { $amount } प्राप्त हुए
positions-event-sent = { $amount } भेजे गए
positions-event-transferred = { $amount } ट्रांसफ़र किए गए
positions-event-ata = टोकन अकाउंट गतिविधि
positions-event-wallet-transaction = { $amount } से जुड़ा वॉलेट ट्रांज़ैक्शन
positions-event-price-per-token = { $price } { -sol } / टोकन
positions-event-wallet-change = वॉलेट में बदलाव { $amount }
positions-event-after-title = इस इवेंट के बाद पोज़िशन
positions-event-capital-invested = निवेशित पूंजी
positions-event-average-entry = औसत एंट्री
positions-event-transfers-title = टोकन ट्रांसफ़र
positions-event-transfer-amount = राशि
positions-event-transfer-mint = मिंट
positions-event-transfer-from = से
positions-event-transfer-to = को
positions-event-no-signature = कोई ऑन-चेन सिग्नेचर नहीं
positions-event-click-to-copy = कॉपी करने के लिए क्लिक करें
positions-event-solscan = { -solscan }
positions-event-token-amount = टोकन राशि
positions-event-trade-price = ट्रेड प्राइस
positions-event-native-amount = { -sol } राशि
positions-event-cost-basis = कॉस्ट बेसिस
positions-event-usd-value = USD वैल्यू
positions-event-network-fee = नेटवर्क फ़ीस
positions-event-router = राउटर
positions-event-slot = स्लॉट
positions-event-chain-status = चेन स्टेटस
positions-event-transaction-type = ट्रांज़ैक्शन प्रकार
positions-event-direction = दिशा
positions-event-wallet-native-change = वॉलेट { -sol } बदलाव
positions-event-instructions = इंस्ट्रक्शन
positions-event-compute-units = कंप्यूट यूनिट्स
positions-event-accounts = अकाउंट
positions-event-record-id = रिकॉर्ड ID
positions-event-time-unavailable = समय उपलब्ध नहीं
positions-event-details = विवरण
positions-event-hide-details = विवरण छिपाएं

positions-chart-type-candles = कैंडल
positions-chart-type-line = लाइन
positions-chart-type-area = एरिया
positions-chart-type-group =
    .aria-label = चार्ट प्रकार
positions-chart-overlays-group =
    .aria-label = चार्ट ओवरले
positions-chart-ema = EMA
    .title = एक्सपोनेंशियल मूविंग एवरेज, 9 और 21
positions-chart-fit = फ़िट
    .title = इस पोज़िशन की पूरी अवधि दिखाएं
positions-chart-timeframes-group =
    .aria-label = टाइमफ़्रेम
positions-chart-pane-group =
    .aria-label = चार्ट पैनल
positions-chart-unavailable = चार्ट इंजन उपलब्ध नहीं
positions-chart-collecting = चार्ट डेटा जुटाया जा रहा है…
positions-chart-no-data = इस टोकन का चार्ट डेटा अभी नहीं है
positions-chart-avg-entry = औसत एंट्री
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = औसत एंट्री
positions-chart-legend-avg-entry-off-scale = औसत एंट्री (स्केल से बाहर)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } इवेंट, जिसकी इस टाइमफ़्रेम में कैंडल नहीं है
       *[other] { $count } इवेंट, जिनकी इस टाइमफ़्रेम में कैंडल नहीं है
    }
positions-chart-level = स्तर
positions-chart-level-above = { $label } { $price } इस व्यू से ऊपर है
positions-chart-level-below = { $label } { $price } इस व्यू से नीचे है
positions-chart-scale-hint = वहां तक स्केल करने के लिए प्राइस एक्सिस खींचें
positions-chart-pnl-at-bar = बार पर लाभ-हानि
positions-chart-click-to-locate = ढूंढने के लिए क्लिक करें
