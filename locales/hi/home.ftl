# Home page.

home-portfolio-title = पोर्टफ़ोलियो मूल्य
home-portfolio-today = आज
home-stat-available = उपलब्ध { -sol }
home-stat-holdings = टोकन होल्डिंग
home-stat-open-pnl = खुली लाभ-हानि
home-stat-realized-today = आज का वास्तविक

home-holdings-token-count =
    { $count ->
        [one] { $count } टोकन
       *[other] { $count } टोकन
    }
home-holdings-with-unpriced = { $tokens } · बिना कीमत: { $count }
home-holdings-unpriced-note =
    { $count ->
        [one] { $count } होल्ड किए गए टोकन की कीमत उपलब्ध नहीं है; कुल में इसे 0 गिना जाता है
       *[other] { $count } होल्ड किए गए टोकन की कीमत उपलब्ध नहीं है; कुल में इन्हें 0 गिना जाता है
    }

home-wallet-copy =
    .title = वॉलेट एड्रेस कॉपी करें
    .aria-label = वॉलेट एड्रेस कॉपी करें
home-wallet-qr-open =
    .title = वॉलेट QR कोड दिखाएँ
    .aria-label = वॉलेट QR कोड दिखाएँ
home-wallet-qr-popover =
    .aria-label = वॉलेट QR कोड
home-wallet-qr-receive = प्राप्त करें
home-wallet-qr-assets = { -sol } और SPL टोकन
home-wallet-qr-close =
    .title = बंद करें
    .aria-label = वॉलेट QR कोड बंद करें
home-wallet-qr-preparing = QR कोड तैयार हो रहा है
home-wallet-qr-unavailable = QR कोड उपलब्ध नहीं है
home-wallet-qr-image =
    .alt = मुख्य वॉलेट एड्रेस का QR कोड

home-calendar-title = प्रदर्शन कैलेंडर
home-calendar-previous =
    .title = पिछला महीना
    .aria-label = पिछला महीना
home-calendar-next =
    .title = अगला महीना
    .aria-label = अगला महीना
home-calendar-month-pnl = महीने की लाभ-हानि
home-calendar-trades = ट्रेड
home-calendar-pop-net-pnl = शुद्ध लाभ-हानि
home-calendar-pop-win-rate = विन रेट
home-calendar-pop-win-rate-value = { $rate } · { $wins } जीत / { $losses } हार
home-calendar-pop-gross-profit = कुल लाभ
home-calendar-pop-gross-loss = कुल हानि
home-calendar-pop-end-balance = अंतिम बैलेंस

home-operations =
    .aria-label = पोर्टफ़ोलियो और मार्केट की स्थिति
home-exposure-title = पोज़िशन एक्सपोज़र
home-exposure-open = खुली
home-exposure-invested = निवेशित
home-exposure-avg-size = औसत साइज़
home-exposure-avg-hold = औसत होल्ड
home-exposure-best = सर्वश्रेष्ठ
home-exposure-worst = सबसे खराब
home-pipeline-title = मार्केट पाइपलाइन
home-pipeline-tracked = ट्रैक किए गए
home-pipeline-priced = कीमत वाले
home-pipeline-passed = फ़िल्टर पास
home-pipeline-not-passed = पास नहीं (सभी ट्रैक किए गए)
home-pipeline-blacklisted = ब्लैकलिस्टेड
home-pipeline-ohlcv = OHLCV
