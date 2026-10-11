# Notification panel labels.

notifications-action-swap-buy = खरीदें
notifications-action-swap-sell = बेचें
notifications-action-position-open = खोलें
notifications-action-position-close = बंद करें
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = आंशिक एग्ज़िट
notifications-action-manual-order = मैन्युअल
notifications-action-unknown = कार्रवाई

actions-step-evaluate = मूल्यांकन
actions-step-validate = सत्यापन
actions-step-quote = कोट लिया जा रहा है
actions-step-swap = स्वैप हो रहा है
actions-step-verify = पुष्टि हो रही है
actions-step-unknown = प्रोसेसिंग
actions-step-evaluate-short = मूल्यांकन
actions-step-validate-short = जाँच
actions-step-quote-short = कोट
actions-step-swap-short = स्वैप
actions-step-verify-short = पुष्टि
actions-step-unknown-short = जारी

actions-failure-recorded = { $message }
actions-failure-unknown = अज्ञात त्रुटि
actions-failure-interrupted = ऐप रीस्टार्ट से बाधित
actions-failure-validation = सत्यापन विफल
actions-failure-quote = कोट विफल
actions-failure-swap = स्वैप विफल
actions-failure-trade = ट्रेड विफल
actions-failure-entry = एंट्री विफल
actions-failure-exit = एग्ज़िट विफल
actions-failure-dca = DCA विफल
actions-failure-verification-expired = पुष्टि की समय-सीमा समाप्त: ट्रांज़ैक्शन कभी लैंड नहीं हुआ
actions-failure-verification-gave-up = पुष्टि का प्रयास छोड़ दिया गया
actions-failure-transaction-failed = ट्रांज़ैक्शन ऑन-चेन विफल रहा
actions-failure-sell-transaction-failed = सेल ट्रांज़ैक्शन ऑन-चेन विफल रहा
actions-failure-dca-verification-failed = DCA की पुष्टि विफल

notifications-empty-all = कोई कार्रवाई नहीं
notifications-empty-active = कोई सक्रिय कार्रवाई नहीं
notifications-empty-completed = कोई पूर्ण कार्रवाई नहीं
notifications-empty-failed = कोई विफल कार्रवाई नहीं
notifications-source-auto = ऑटो
notifications-source-manual = मैन्युअल
notifications-cancelled = रद्द किया गया
notifications-load-failed = लोड करना विफल
notifications-clear-title = नोटिफ़िकेशन साफ़ करें
notifications-clear-message = इस सूची के सभी नोटिफ़िकेशन खारिज करें? वे पूर्ण/विफल इतिहास में बने रहेंगे।
notifications-clear-failed = नोटिफ़िकेशन साफ़ करना विफल
notifications-stream-lag-title = एक्शन स्ट्रीम पीछे रह गई
notifications-stream-lag-missed =
    छूटे हुए अपडेट: { $count ->
        [one] { $count }
       *[other] { $count }
    } — रीफ़्रेश हो रहा है
notifications-stream-lag-refreshing = रीफ़्रेश हो रहा है
notifications-sync-failed = कार्रवाइयाँ रीफ़्रेश नहीं की जा सकीं

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = आज
notifications-day-yesterday = कल
notifications-source-copy = कॉपी ट्रेडिंग
notifications-steps = चरण
notifications-detail-transaction = ट्रांज़ैक्शन
notifications-detail-fill-price = निष्पादन प्राइस
    .title = इस स्वैप की { -sol } राशि को टोकन मात्रा से भाग देने पर
notifications-detail-fee = नेटवर्क फ़ीस
notifications-detail-pnl = पोज़िशन का लाभ-हानि
notifications-detail-route = रूट
notifications-detail-duration = अवधि
notifications-detail-reason = कारण
notifications-view-token = टोकन देखें
notifications-view-position = पोज़िशन देखें
notifications-view-transaction = ट्रांज़ैक्शन देखें
notifications-unrecorded = पोज़िशन में दर्ज नहीं
notifications-unrecorded-detail = यह स्वैप पोज़िशन में दर्ज नहीं हुआ और हो सकता है पूरा न हुआ हो। इसका ट्रांज़ैक्शन जांचें।
