services-health-component-unavailable = { $component } कॉम्पोनेंट उपलब्ध नहीं है
services-health-unavailable = हेल्थ स्थिति उपलब्ध नहीं है
services-health-pools-not-running = पूल सर्विस नहीं चल रही
services-health-events-db-uninitialized = इवेंट डेटाबेस इनिशियलाइज़ नहीं हुआ है
services-health-sol-price-not-running = SOL प्राइस सर्विस नहीं चल रही
services-health-sol-price-stale = SOL प्राइस डेटा पुराना है ({ $seconds } सेकंड पुराना)
services-health-sol-price-no-data = अभी तक SOL प्राइस डेटा उपलब्ध नहीं है
services-health-telegram-discovery = डिस्कवरी मोड
services-health-telegram-disconnected = डिस्कनेक्टेड
services-health-wallet-watch-polling-only = डिटेक्शन केवल पोलिंग पर चल रहा है
services-health-assistant-tasks-disabled = कॉन्फ़िग में बंद है
services-health-connectivity-critical-unhealthy = महत्वपूर्ण एंडपॉइंट अस्वस्थ हैं: { $endpoints }
services-health-filtering-snapshot-stale = फ़िल्टरिंग स्नैपशॉट { $seconds } सेकंड पुराना है

services-status-healthy = स्वस्थ
services-status-starting = शुरू हो रहा है
services-status-degraded = डिग्रेडेड
services-status-unhealthy = अस्वस्थ
services-status-stopping = रुक रहा है
services-status-disabled = बंद
services-status-unknown = अज्ञात

services-name-account = अकाउंट
services-name-assistant-scheduled-tasks = असिस्टेंट शेड्यूल्ड टास्क
services-name-ata-cleanup = टोकन अकाउंट क्लीनअप
services-name-connectivity = कनेक्टिविटी
services-name-copy-trading = कॉपी ट्रेडिंग
services-name-events = इवेंट
services-name-filtering = फ़िल्टरिंग
services-name-llm-analysis = LLM विश्लेषण
services-name-ohlcv = OHLCV
services-name-pool-pricing = पूल मूल्य निर्धारण
services-name-pools = पूल
services-name-positions = पोज़िशन
services-name-referral = रेफ़रल
services-name-rpc-stats = RPC आँकड़े
services-name-sol-price = { -sol } कीमत
services-name-telegram = { -telegram }
services-name-tokens = टोकन
services-name-trader = ट्रेडर
services-name-transactions = ट्रांज़ैक्शन
services-name-update-check = अपडेट जाँच
services-name-wallet = वॉलेट
services-name-wallet-watch = वॉलेट वॉच
services-name-webserver = वेब सर्वर

services-loading = सर्विसेज़ लोड हो रही हैं...
services-load-failed = सर्विसेज़ लोड करने में विफल
services-load-failed-description = बैकएंड के जवाब की प्रतीक्षा है। हम अपने आप दोबारा प्रयास करेंगे।
services-refresh-failed = सर्विसेज़ रिफ़्रेश नहीं हो सकीं
services-search-placeholder = सर्विसेज़ खोजें...
services-summary-total = कुल
services-summary-alerts = अलर्ट
services-summary-alerts-tooltip = { $degraded } डिग्रेडेड / { $unhealthy } अस्वस्थ
services-filter-status = स्थिति
services-filter-all-statuses = सभी स्थितियाँ
services-filter-all-services = सभी सर्विसेज़
services-filter-enabled-only = केवल चालू
services-filter-disabled-only = केवल बंद
services-col-service = सर्विस
services-col-health = हेल्थ
services-col-priority = प्राथमिकता
services-col-uptime = अपटाइम
services-col-activity = गतिविधि
services-col-last-cycle = पिछला साइकिल
services-col-avg-cycle = औसत साइकिल
services-col-avg-poll = औसत पोल
services-col-cycle-rate = साइकिल/सेकंड
services-col-tasks = टास्क
services-col-ops = ऑप्स/सेकंड
services-col-errors = त्रुटियाँ
services-col-dependencies = डिपेंडेंसी
services-activity-busy = { $percent } व्यस्त
services-activity-polls =
    { $count ->
        [one] { $count } पोल
       *[other] { $count } पोल
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } टास्क
       *[other] { $count } टास्क
    }
    पिछला: { $last }
    औसत: { $avg }
    पोल: { $poll }
    निष्क्रिय: { $idle }
    कुल पोल: { $polls }
services-tasks-none = कोई इंस्ट्रूमेंटेड टास्क नहीं

# Empty table (scripts/pages/services.js)
services-empty = कोई सर्विस नहीं चल रही
    .message = बॉट द्वारा शुरू किए जाने के बाद सर्विसेज़ यहाँ दिखती हैं।
