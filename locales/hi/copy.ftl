# Copy trading messages.

copy-skip-not-buy-swap = वॉलेट की गतिविधि खरीद नहीं थी
copy-skip-task-disabled = टास्क रुका हुआ है
copy-skip-mode-transition-required = एक्ज़ीक्यूशन मोड अलग से बदलना होगा
copy-skip-live-confirmation-required = लाइव एक्ज़ीक्यूशन के लिए पुष्टि चाहिए
copy-skip-unsupported-sizing-mode = साइज़िंग मोड अभी समर्थित नहीं है
copy-skip-self-copy = यह वॉलेट आपके अपने वॉलेट में से एक है
copy-skip-target-below-minimum = वॉलेट का ट्रेड न्यूनतम से कम है
copy-skip-target-above-maximum = वॉलेट का ट्रेड अधिकतम से ज़्यादा है
copy-skip-already-bought = यह टोकन पहले खरीदा जा चुका है (एक बार खरीदें)
copy-skip-blacklisted = टोकन रिस्क कंट्रोल द्वारा ब्लॉक है
copy-skip-filter-required = टोकन फ़िल्टरिंग पास नहीं कर सका
copy-skip-budget-exhausted = टास्क का बजट खत्म हो गया है
copy-skip-token-cap-reached = प्रति-टोकन सीमा पूरी हो गई
copy-skip-below-minimum-size = कॉपी साइज़ बहुत छोटा है
copy-skip-invalid-sizing = टास्क की साइज़िंग अमान्य है
copy-skip-invalid-slippage = टास्क का स्लिपेज अमान्य है
copy-skip-invalid-exit-policy = टास्क के एग्ज़िट नियम अमान्य हैं
copy-skip-invalid-price = कोई उपयोगी मार्केट प्राइस नहीं
copy-skip-not-sell-swap = वॉलेट की गतिविधि सेल नहीं थी
copy-skip-exit-mode-disabled = वॉलेट का सेल अनदेखा किया गया: टास्क अपने नियमों से बेचता है
copy-skip-force-stopped = ट्रेडिंग ज़बरन रोकी गई है
copy-skip-copy-position-not-found = इस टास्क की कोई पोज़िशन नहीं मिली
copy-skip-position-user-only = पोज़िशन का प्रबंधन आप करते हैं
copy-skip-position-management-mismatch = पोज़िशन अब कॉपी सेल को फ़ॉलो नहीं करती
copy-skip-latency-kill-switch = ऑटो-पॉज़: ट्रेड बहुत देर से पहचाने गए
copy-skip-claim-reconciled-abandoned = बाधित लाइव सबमिशन बिना रीट्राई बंद किया गया
copy-skip-stale-observation = डाउनटाइम के बाद रीप्ले हुआ, कॉपी करने के लिए बहुत पुराना
copy-skip-unknown-observation-time = रीप्ले किए गए ट्रेड का कोई ब्लॉक टाइम नहीं है
copy-skip-entry-blocked = एंट्री ब्लॉक

copy-entry-block-force-stopped = ट्रेडिंग ज़बरन रोकी गई है
copy-entry-block-loss-limit = लॉस लिमिट नई एंट्री रोकती है
copy-entry-block-connectivity = ज़रूरी सर्विसेज़ उपलब्ध नहीं हैं
copy-entry-block-position-limit = खुली पोज़िशन की सीमा पूरी हो गई
copy-entry-block-already-open = एक पोज़िशन पहले से खुली है
copy-entry-block-reentry-cooldown = टोकन री-एंट्री कूलडाउन
copy-entry-block-open-cooldown = ग्लोबल एंट्री कूलडाउन
copy-entry-block-entry-reserved = दूसरी एंट्री प्रोसेस हो रही है
copy-entry-block-blacklisted = टोकन रिस्क कंट्रोल द्वारा ब्लॉक है
copy-entry-block-check-failed = एक सुरक्षा जाँच पूरी नहीं हो सकी

copy-pause-user = आपके द्वारा रोका गया
copy-pause-latency-kill-switch = ऑटो-पॉज़: ट्रेड औसतन { $average } सेकंड देर से पहुँचे (सीमा { $threshold } सेकंड)
copy-pause-watch-detached = ऑटो-पॉज़: वॉलेट अब वॉच नहीं हो रहा
copy-pause-watch-budget-exceeded = रोका गया: इस वॉलेट ने पिछड़ी गतिविधि पकड़ने से पहले वॉच जाँच की { $limit } सिग्नेचर की सीमा छू ली
copy-pause-helius-unavailable = रोका गया: { -helius } वॉलेट जाँच विफल रही
copy-pause-watch-processing-failed = रोका गया: वॉलेट गतिविधि प्रोसेस नहीं हो सकी
copy-pause-unspecified = रोका गया

copy-pause-short-user = आपके द्वारा
copy-pause-short-latency-kill-switch = बहुत धीमा
copy-pause-short-watch-detached = वॉच टूटा
copy-pause-short-watch-budget-exceeded = वॉच सीमा
copy-pause-short-helius-unavailable = वॉच प्रोवाइडर
copy-pause-short-watch-processing-failed = वॉच प्रोसेसिंग
copy-state-paused = रोका गया
copy-state-paused-reason = रोका गया · { $reason }

copy-readiness-history = पेपर हिस्ट्री
copy-readiness-history-met =
    { $count ->
        [one] { $count } बंद पेपर राउंड, { $needed } ज़रूरी
       *[other] { $count } बंद पेपर राउंड, { $needed } ज़रूरी
    }
copy-readiness-history-short = { $needed } में से { $count } बंद पेपर राउंड
copy-readiness-profit = पेपर में लाभदायक
copy-readiness-profit-detail =
    { $count ->
        [one] { $count } राउंड में { $realized } { -sol } वास्तविक, { $wins } जीत
       *[other] { $count } राउंड में { $realized } { -sol } वास्तविक, { $wins } जीत
    }
copy-readiness-latency = ट्रेड समय पर पहचाने गए
copy-readiness-latency-detail = p95 आगमन { $p95 } सेकंड, सीमा { $limit } सेकंड
copy-readiness-latency-none = अभी कोई आगमन सैंपल नहीं
copy-readiness-priced = हर होल्डिंग की कीमत उपलब्ध
copy-readiness-priced-ok = हर खुली पेपर होल्डिंग का पूल प्राइस है
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } खुली होल्डिंग का पूल प्राइस नहीं है
       *[other] { $count } खुली होल्डिंग का पूल प्राइस नहीं है
    }
copy-readiness-runtime = लाइव एक्ज़ीक्यूशन उपलब्ध
copy-readiness-runtime-ok = सेटअप और सुरक्षा गेट लाइव कॉपी की अनुमति देते हैं

copy-live-block-setup-incomplete = पहले वॉलेट और RPC सेटअप पूरा करें
copy-live-block-force-stop = इमरजेंसी स्टॉप चालू है
copy-live-block-copy-trading-disabled = कॉपी प्रोसेसिंग ग्लोबली रुकी हुई है
copy-live-block-unavailable = लाइव एक्ज़ीक्यूशन उपलब्ध नहीं है

copy-state-system-paused = ग्लोबली रोका गया
copy-state-force-stopped = ज़बरन रोका गया
copy-state-entries-blocked = एंट्री ब्लॉक
copy-state-running-live = चल रहा है
copy-state-running-paper = चल रहा है
copy-mode-paper = पेपर
copy-mode-live = लाइव
copy-exit-mode-buy-only = मेरे एग्ज़िट नियम
copy-exit-mode-mirror = वॉलेट के सेल की नकल
copy-exit-mode-hybrid = वॉलेट के सेल और मेरे नियम
copy-exit-target-sell = वॉलेट ने बेचा
copy-exit-stop-loss = स्टॉप लॉस
copy-exit-trailing-stop = ट्रेलिंग स्टॉप
copy-exit-take-profit = टेक प्रॉफ़िट
copy-exit-time-override = टाइम नियम
copy-exit-manual = हाथ से बंद किया

copy-request-failed = अनुरोध विफल
copy-keep-paused = रुका रहने दें
copy-paused-suffix = · रोका गया
copy-mode-paused = { $mode } · रोका गया
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = वास्तविक लाभ-हानि
copy-metric-unrealized-pnl = अवास्तविक लाभ-हानि
copy-metric-win-rate = विन रेट
copy-metric-budget-spent = खर्च हुआ बजट
copy-metric-median-arrival = औसत (मीडियन) आगमन
copy-metric-open-holdings = खुली होल्डिंग
copy-record-won-lost = { $won } जीत · { $lost } हार
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = फ़िल
copy-kind-exits = एग्ज़िट
copy-kind-skips = स्किप
copy-kind-errors = त्रुटियाँ
copy-field-per-trade-cap = प्रति-ट्रेड सीमा
copy-field-per-token-cap = प्रति-टोकन सीमा
copy-field-total-budget = कुल बजट
copy-field-slippage = स्लिपेज
copy-rules-wallet-sells-only = केवल वॉलेट के सेल
copy-filter-copy-setting-required = कॉपी सेटिंग (ज़रूरी)
copy-filter-copy-setting-not-required = कॉपी सेटिंग (ज़रूरी नहीं)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } बंद राउंड
       *[other] { $count } बंद राउंड
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } खुली होल्डिंग
       *[other] { $count } खुली होल्डिंग
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } कीमत वाली होल्डिंग · { $unpriced } बिना कीमत
       *[other] { $priced } कीमत वाली होल्डिंग · { $unpriced } बिना कीमत
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } बिना कीमत की होल्डिंग
       *[other] { $count } बिना कीमत की होल्डिंग
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = सभी
copy-range-label =
    .aria-label = तारीख़ सीमा

copy-page-title = कॉपी ट्रेडिंग
copy-page-beta = बीटा
copy-strip-loading = लोड हो रहा है
copy-strip-unavailable = उपलब्ध नहीं
copy-strip-setup-required = सेटअप ज़रूरी · कॉपी ट्रेडिंग के लिए वॉलेट और RPC चाहिए
copy-strip-pause-all = सभी रोकें
copy-strip-resume = प्रोसेसिंग फिर शुरू करें
copy-strip-settings = सेटिंग्स
copy-strip-add-wallet = वॉलेट जोड़ें
copy-strip-paused-globally = ग्लोबली रोका गया · कोई नई कॉपी नहीं, एग्ज़िट फिर भी चलते हैं
copy-strip-force-stopped = ज़बरन रोका गया · कुछ भी कॉपी नहीं होता
copy-strip-loss-limit = लॉस लिमिट · नई एंट्री ब्लॉक, एग्ज़िट फिर भी चलते हैं
copy-strip-idle-paused =
    { $count ->
        [one] निष्क्रिय · { $count } टास्क रुका
       *[other] निष्क्रिय · { $count } टास्क रुके
    }
copy-strip-idle-empty = निष्क्रिय · अभी कोई टास्क नहीं
copy-strip-processing = प्रोसेसिंग · पेपर { $paper }
copy-strip-processing-live = प्रोसेसिंग · लाइव { $live } · पेपर { $paper }
copy-figures-label =
    .aria-label = कॉपी ट्रेडिंग कुल
copy-figure-marked-at-pool = पूल प्राइस पर मार्क किया गया
copy-figure-across-tasks = सभी टास्क में
copy-figure-budget-lifetime = चालू टास्क का कुल खर्च
copy-figure-budget-none = कोई चालू टास्क नहीं
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } ट्रेड
       *[other] { $count } ट्रेड
    }
copy-figure-arrival-none = चालू टास्क से कोई सैंपल नहीं

copy-load-failed = कॉपी ट्रेडिंग लोड नहीं हो सकी: { $error }
copy-resume-all-title = कॉपी प्रोसेसिंग फिर शुरू करें
copy-resume-all-message =
    { $count ->
        [one] { $count } लाइव टास्क अपने वॉलेट के दोबारा ट्रेड करने पर असली स्वैप सबमिट करेगा।
       *[other] { $count } लाइव टास्क अपने वॉलेट के दोबारा ट्रेड करने पर असली स्वैप सबमिट करेंगे।
    }
copy-toast-resumed-all = कॉपी प्रोसेसिंग फिर शुरू हुई
copy-toast-paused-all = सारी कॉपी प्रोसेसिंग रोकी गई
copy-toast-global-failed = कॉपी प्रोसेसिंग बदली नहीं जा सकी

copy-onboarding-title = भरोसेमंद वॉलेट कॉपी करें, पहले उन्हें पेपर में परखें
copy-onboarding-body = हर टास्क पेपर से शुरू होता है: टारगेट के ट्रेड आपके स्लिपेज और फ़ीस के साथ पूल प्राइस पर सिमुलेट होते हैं, और आपके एग्ज़िट नियम पेपर बुक पर चलते हैं। किसी वॉलेट के पेपर नतीजे भरोसा जीत लें तभी उसके लिए लाइव सक्रिय करें।
copy-onboarding-add = अपना पहला वॉलेट जोड़ें
copy-setup-gate-title = कॉपी ट्रेडिंग के लिए वॉलेट ज़रूरी है
copy-onboarding-observe = निरीक्षण
copy-onboarding-observe-detail = { -sol } खर्च किए बिना वॉलेट के स्वैप पहचानें।
copy-onboarding-evaluate = मूल्यांकन
copy-onboarding-evaluate-detail = पेपर लाभ-हानि, विन रेट, स्किप, पहचान की गति और स्लिपेज देखें।
copy-onboarding-arm = सक्रिय करें
copy-onboarding-arm-detail = रेडीनेस जाँच पास करें, फिर असली स्वैप चालू करें।

copy-list-label =
    .aria-label = कॉपी किए गए वॉलेट
copy-list-title = वॉलेट
copy-list-compare = तुलना
copy-list-sort-label = वॉलेट क्रमबद्ध करें
copy-list-count = { $active } सक्रिय · कुल { $total }
copy-sort-pnl = लाभ-हानि
copy-sort-state = स्थिति
copy-sort-name = नाम
copy-compare-label =
    .aria-label = वॉलेट की तुलना

copy-dialog-close =
    .aria-label = बंद करें
copy-editor-title-add = वॉलेट जोड़ें
copy-editor-sub-add = नए टास्क पेपर से शुरू होते हैं
copy-arm-title = लाइव कॉपी सक्रिय करें
copy-arm-sub = आपके वॉलेट से असली स्वैप
copy-arm-keep-paper = पेपर में रहने दें
copy-arm-confirm = लाइव सक्रिय करें
copy-profile-title = वॉलेट प्रोफ़ाइल
copy-profile-sub = इस बॉट ने वॉलेट के बारे में जो देखा है

copy-settings-title = कॉपी ट्रेडिंग सेटिंग्स
copy-settings-subtitle = हर टास्क के लिए ग्लोबल नीति
copy-settings-filter-warning = डिफ़ॉल्ट फ़िल्टरिंग सेटअप के साथ यह लगभग हर टोकन को अस्वीकार कर देता है, इसलिए कुछ भी कॉपी नहीं होता। जब तक आपके फ़िल्टर वे टोकन पास न करें जिन्हें आपके वॉलेट ट्रेड करते हैं, इसे बंद रखें।
copy-settings-unit-seconds = से
copy-settings-unit-trades = ट्रेड
copy-settings-unit-tasks = कार्य
copy-settings-unit-rounds = राउंड
copy-settings-save = सेटिंग्स सहेजें
copy-settings-load-failed = कॉपी सेटिंग्स लोड नहीं हो सकीं
copy-settings-saved = कॉपी ट्रेडिंग सेटिंग्स सहेजी गईं

copy-tab-overview = अवलोकन
copy-tab-holdings = होल्डिंग
copy-tab-activity = गतिविधि
copy-tab-rules = नियम
copy-tab-execution = एक्ज़ीक्यूशन
copy-tabs-label = टास्क व्यू
copy-workspace-select = वर्कस्पेस खोलने के लिए कोई वॉलेट चुनें।
copy-workspace-loading = टास्क लोड हो रहा है…
copy-workspace-load-failed = यह टास्क लोड नहीं हो सका: { $error }

copy-state-detail-paper = पेपर में चल रहा है · ट्रेड सिमुलेट होते हैं, कुछ खर्च नहीं होता
copy-state-detail-live = लाइव चल रहा है · वॉलेट के ट्रेड असली स्वैप से कॉपी होते हैं
copy-state-detail-system-paused = प्रतीक्षा · कॉपी प्रोसेसिंग ग्लोबली रुकी है, एग्ज़िट फिर भी चलते हैं
copy-state-detail-entries-blocked = लॉस लिमिट ने एंट्री रोकी हैं · एग्ज़िट फिर भी चलते हैं
copy-state-detail-force-stopped = ज़बरन रोका गया · कुछ भी कॉपी नहीं होता

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = फिर शुरू करने पर वही सीमा रहती है, इसलिए ट्रेड देर से आते रहे तो यह फिर रुक जाएगा। RPC स्ट्रीम जाँचें या सेटिंग्स में आगमन सीमा बढ़ाएँ।
copy-paused-resume-detached = फिर शुरू करने पर वॉलेट दोबारा वॉच होगा।
copy-paused-holdings-rules =
    { $count ->
        [one] इसके एग्ज़िट नियम अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
       *[other] इसके एग्ज़िट नियम अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] वॉलेट के सेल अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
       *[other] वॉलेट के सेल अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] वॉलेट के सेल और इसके एग्ज़िट नियम अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
       *[other] वॉलेट के सेल और इसके एग्ज़िट नियम अब भी इसकी { $count } खुली होल्डिंग बंद करते हैं।
    }

copy-watch-state-catching-up = वॉलेट वॉच: पिछड़ी गतिविधि पकड़ी जा रही है। इस वॉलेट की जाँच { -helius } से हो रही है।
copy-watch-state-watching = वॉलेट वॉच: वॉच जारी है। इस वॉलेट की जाँच { -helius } से हो रही है।
copy-watch-last-check = आखिरी जाँच: { $ago }।
copy-watch-recovery-active = वॉलेट वॉच सक्रिय है
copy-watch-recovery-catching-up = वॉलेट वॉच पिछड़ी गतिविधि पकड़ रहा है
copy-watch-recovery-still-paused = कॉपी टास्क अब भी रुका है। तैयार होने पर कॉपी फिर शुरू करें।
copy-watch-recovery-title = वॉलेट वॉच बहाल करें
copy-watch-recovery-processing-failed = वॉलेट गतिविधि प्रोसेस नहीं हो सकी। सहेजी गई प्रगति सुरक्षित है। समस्या सुलझने के बाद फिर कोशिश करें।
copy-watch-recovery-provider-failed = { -helius } जाँच विफल रही। सहेजी गई प्रगति सुरक्षित है। प्रोवाइडर उपलब्ध होने पर फिर कोशिश करें।
copy-watch-recovery-budget-intro = इस वॉलेट की गतिविधि इसके मौजूदा वॉच की जाँच क्षमता से ज़्यादा है। आगे कैसे बढ़ना है, चुनें।
copy-watch-approve = { -helius } से पिछड़ी गतिविधि पकड़ने की कोशिश करें
copy-watch-approve-help = सहेजी गई प्रगति से आगे बढ़ता है। { -helius } के ज़्यादा क्रेडिट लग सकते हैं और फिर भी पिछड़ना संभव है।
copy-watch-approve-unavailable = { -helius } से पिछड़ी गतिविधि पकड़ना उपलब्ध नहीं है। अनजाँची गतिविधि छोड़े बिना आगे बढ़ने के लिए चालू { -helius } RPC एंडपॉइंट कॉन्फ़िगर करें।
copy-watch-no-provider = इस वॉच के लिए कोई कैच-अप प्रोवाइडर समर्थित नहीं है।
copy-watch-budget-label = प्रति जाँच जाँचे जाने वाले सिग्नेचर
copy-watch-budget-hint = या अनजाँची गतिविधि छोड़कर अभी से फिर शुरू करें। प्रति जाँच { $min }–{ $max } सिग्नेचर चुनें; ऊँची सीमा में ज़्यादा RPC कॉल लग सकती हैं।
copy-watch-ack = मुझे पता है कि छूटी हुई गतिविधि कॉपी नहीं होगी।
copy-watch-toast-range = प्रति पोल { $min } से { $max } सिग्नेचर चुनें, { $step }-सिग्नेचर के चरणों में
copy-watch-toast-ack = पुष्टि करें कि आखिरी पूरी हुई जाँच के बाद के सिग्नेचर छोड़ दिए जाएँगे
copy-watch-resumed = वॉलेट वॉच अभी से फिर शुरू हुआ; कॉपी टास्क रुका रहेगा
copy-watch-resume-failed = वॉलेट वॉच फिर शुरू नहीं हो सका
copy-watch-retry-started = वॉलेट वॉच रीट्राई सहेजी गई प्रगति से शुरू हुआ; कॉपी टास्क रुका रहेगा
copy-watch-retry-failed = वॉलेट वॉच रीट्राई नहीं हो सका
copy-watch-approve-title = इस वॉलेट के लिए { -helius } कैच-अप की अनुमति दें
copy-watch-approve-message = { -helius } अनजाँचे अंतराल को छोड़े बिना, सहेजी गई प्रगति से सफल Solana ट्रांज़ैक्शन जाँच सकता है। यह अभी हर 100 पूर्ण ट्रांज़ैक्शन लौटने पर 10 क्रेडिट लेता है, ऊपर की ओर पूर्णांकित, और प्रति अनुरोध न्यूनतम 10 क्रेडिट। एक जाँच में कई अनुरोध हो सकते हैं; उपयोग और प्रोवाइडर की कीमतें अलग हो सकती हैं। कॉपी तब तक रुकी रहती है जब तक आप उसे अलग से फिर शुरू न करें।
copy-watch-approve-confirm = इस वॉलेट के लिए अनुमति दें
copy-watch-approved = वॉलेट वॉच सहेजी गई प्रगति से शुरू हुआ; कॉपी टास्क रुका रहेगा
copy-watch-restore-failed = वॉलेट वॉच बहाल नहीं हो सका

copy-action-pause = रोकें
copy-action-resume = फिर शुरू करें
copy-action-resume-copy = कॉपी फिर शुरू करें
copy-action-resume-from-now = अभी से फिर शुरू करें
copy-action-retry-watch = वॉलेट वॉच फिर आज़माएँ
copy-action-return-paper = पेपर पर लौटें
copy-action-edit-rules = नियम संपादित करें
copy-action-clone = क्लोन करें
copy-action-profile = वॉलेट प्रोफ़ाइल
copy-resume-live-title = लाइव कॉपी फिर शुरू करें
copy-resume-live-message = जब यह वॉलेट फिर ट्रेड करेगा, टास्क “{ $name }” आपके वॉलेट से असली स्वैप सबमिट करेगा।
copy-resume-live-confirm = लाइव फिर शुरू करें
copy-task-resumed = टास्क फिर शुरू हुआ
copy-task-paused = टास्क रोका गया
copy-task-state-failed = टास्क की स्थिति बदली नहीं जा सकी
copy-return-paper-message = टास्क “{ $name }” की नई कॉपी फिर सिमुलेट होंगी, { -sol } खर्च किए बिना।
copy-return-paper-cancel = लाइव रहने दें
copy-task-returned-paper = टास्क पेपर पर लौटा
copy-mode-change-failed = एक्ज़ीक्यूशन मोड बदला नहीं जा सका
copy-delete-title = कॉपी टास्क हटाएँ
copy-delete-message = टास्क “{ $name }” हटाएँ? इसके निर्णय और पेपर नतीजे हटा दिए जाएँगे और इस टास्क के लिए वॉलेट वॉच नहीं होगा।
copy-delete-confirm = टास्क हटाएँ
copy-delete-cancel = टास्क रखें
copy-task-deleted = कॉपी टास्क हटाया गया
copy-task-delete-failed = कॉपी टास्क हटाया नहीं जा सका

copy-overview-results = नतीजे
copy-analytics-load-failed = एनालिटिक्स लोड नहीं हो सका: { $error }
copy-analytics-loading = एनालिटिक्स लोड हो रहा है…
copy-exit-bucket =
    { $count ->
        [one] { $count } सेल · { $pnl }
       *[other] { $count } सेल · { $pnl }
    }
copy-overview-average-win = औसत जीत
copy-overview-average-loss = औसत घाटा { $amount }
copy-overview-profit-factor = प्रॉफ़िट फ़ैक्टर
copy-overview-profit-factor-note = कुल जीत ÷ कुल घाटे
copy-overview-average-hold = औसत होल्ड
copy-overview-average-hold-note = एंट्री से एग्ज़िट तक
copy-overview-best-round = सर्वश्रेष्ठ राउंड
copy-overview-worst-round = सबसे खराब { $amount }
copy-overview-curve-title = संचयी लाभ-हानि
copy-overview-exits-title = एग्ज़िट के अनुसार सेल
copy-overview-skips-title = ट्रेड क्यों स्किप हुए
copy-book-title-live = लाइव बुक
copy-book-title-paper = पेपर बुक
copy-book-all-time = सर्वकालिक
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] खरीद
       *[other] खरीद
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] एग्ज़िट आपके नियमों से
       *[other] एग्ज़िट आपके नियमों से
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] वॉलेट सेल
       *[other] वॉलेट सेल
    }
copy-book-manual-closes = <strong>{ $count }</strong> हाथ से बंद
copy-book-skipped = <strong>{ $count }</strong> स्किप
copy-book-failed = <strong>{ $count }</strong> विफल
copy-book-closed = { $count } बंद
copy-book-budget-note = { $mode } खर्च { $total } में से · { $remaining } शेष
copy-check-passed = पास
copy-check-not-passed = पास नहीं
copy-readiness-title = लाइव जाने से पहले
copy-readiness-live-note = यह टास्क लाइव ट्रेड करता है। ऊपर हेडर से इसे पेपर पर लौटाएँ।
copy-readiness-all-pass = हर जाँच पास है।
copy-readiness-needs-review = सक्रिय करने के लिए जो तैयार नहीं है उसकी स्पष्ट समीक्षा ज़रूरी है।
copy-readiness-arm = समीक्षा करें और लाइव सक्रिय करें

copy-rules-title = लागू नियम
copy-rules-size-ratio = वॉलेट के ट्रेड का { $pct }
copy-rules-size-fixed = प्रति कॉपी { $amount }
copy-rules-target-any = कोई भी साइज़
copy-rules-target-min = कम से कम { $amount }
copy-rules-target-max = अधिकतम { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = टास्क ओवरराइड · ट्रेडर { $value }
copy-rules-source-default = ट्रेडर डिफ़ॉल्ट
copy-rules-not-used = उपयोग नहीं: वॉलेट के सेल तय करते हैं
copy-rules-col-rule = नियम
copy-rules-col-applies = लागू
copy-rules-col-source = स्रोत
copy-rules-budget-note = { $mode } में { $spent } खर्च · { $remaining } शेष
copy-rules-token-copies =
    { $count ->
        [one] एक टोकन की लगभग { $count } पूरी कॉपी
       *[other] एक टोकन की लगभग { $count } पूरी कॉपी
    }
copy-rules-sizing = साइज़िंग
copy-rules-copy-size = कॉपी साइज़
copy-rules-entry-filters = एंट्री फ़िल्टर
copy-rules-target-size = वॉलेट ट्रेड साइज़
copy-rules-repeat-buys = दोहराई गई खरीद
copy-rules-repeat-first-only = हर टोकन की केवल पहली खरीद
copy-rules-repeat-every = हर खरीद, प्रति-टोकन सीमा तक
copy-rules-filter-pass = फ़िल्टरिंग पास
copy-rules-filter-required = ज़रूरी
copy-rules-filter-not-required = ज़रूरी नहीं
copy-rules-filter-task-override = टास्क ओवरराइड
copy-rules-exits = एग्ज़िट
copy-rules-exits-inactive = होल्डिंग केवल तब बिकती है जब वॉलेट बेचता है; नीचे के नियम इस मोड में नहीं चलते।

copy-rule-status = स्थिति
copy-rule-on = चालू
copy-rule-off = बंद
copy-rule-unit-seconds = से
copy-rule-unit-minutes = मि
copy-rule-stop-loss-threshold = इतने घाटे पर बेचता है
copy-rule-stop-loss-min-hold = इतना होल्ड करने से पहले नहीं
copy-rule-no-minimum = कोई न्यूनतम नहीं
copy-rule-partial-exits = आंशिक एग्ज़िट
copy-rule-partial-allowed = अनुमत
copy-rule-partial-full-only = केवल पूरा एग्ज़िट
copy-rule-partial-size = आंशिक एग्ज़िट साइज़
copy-rule-trailing-activation = इतने लाभ पर सक्रिय होता है
copy-rule-trailing-distance = पीक से इतना नीचे बेचता है
copy-rule-take-profit-target = इतने लाभ पर बेचता है
copy-rule-time-duration = इतना होल्ड करने के बाद जाँचता है
copy-rule-time-threshold = लाभ-हानि इसके बराबर या नीचे होने पर बेचता है
copy-preset-inherit = ट्रेडर डिफ़ॉल्ट
copy-preset-conservative = रूढ़िवादी
copy-preset-balanced = संतुलित
copy-preset-aggressive = आक्रामक
copy-preset-custom = कस्टम
copy-validate-stop-loss = स्टॉप लॉस 0% से ऊपर और अधिकतम 100% होना चाहिए।
copy-validate-partial-size = आंशिक एग्ज़िट साइज़ 0% और 100% के बीच होना चाहिए।
copy-validate-min-hold = न्यूनतम होल्ड सेकंड की पूर्ण संख्या होनी चाहिए।
copy-validate-trailing-activation = ट्रेलिंग सक्रियण 0% से ऊपर और अधिकतम 100% होना चाहिए।
copy-validate-trailing-distance = ट्रेलिंग दूरी 0% से ऊपर और अधिकतम 100% होनी चाहिए।
copy-validate-take-profit = टेक प्रॉफ़िट 0% से ऊपर होना चाहिए।
copy-validate-time-duration = टाइम नियम के लिए शून्य से ऊपर की अवधि चाहिए।
copy-validate-time-threshold = टाइम नियम की सीमा एक घाटा है: 0% या ऋणात्मक संख्या इस्तेमाल करें।
copy-warning-mirror = केवल वॉलेट के सेल होल्डिंग बंद करते हैं: कोई स्टॉप लॉस उनकी रक्षा नहीं करता, और जिस टोकन को वॉलेट कभी नहीं बेचता वह होल्ड ही रहता है।
copy-warning-no-rules = कोई एग्ज़िट नियम चालू नहीं है और वॉलेट के सेल अनदेखे किए जाते हैं: होल्डिंग कभी नहीं बिकती।
copy-warning-no-stop-loss = कोई स्टॉप लॉस लागू नहीं: गिरता टोकन तब तक होल्ड रहता है जब तक कोई दूसरा नियम या वॉलेट न बेचे।
copy-warning-stop-delay = स्टॉप लॉस हर खरीद के बाद { $hold } इंतज़ार करता है: जो टोकन इससे तेज़ गिरे वह { $threshold } से काफ़ी नीचे बंद होगा।
copy-warning-take-profit-cost = { $target } पर टेक प्रॉफ़िट बेचने की लागत ({ $slippage } स्लिपेज और { $fee } स्वैप फ़ीस) नहीं निकालता, इसलिए राउंड घाटे में बंद होते हैं।
copy-warning-trailing-distance = ट्रेलिंग दूरी अपने सक्रियण लाभ के बराबर या उससे ज़्यादा है, इसलिए सक्रिय ट्रेल एंट्री से नीचे बेच सकता है।

copy-execution-title = एक्ज़ीक्यूशन गुणवत्ता
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = कोई भी
copy-execution-limit-on =
    { $count ->
        [one] { $count } ट्रेड पर औसतन { $limit } से ऊपर जाने पर रुकता है
       *[other] { $count } ट्रेड पर औसतन { $limit } से ऊपर जाने पर रुकता है
    }
copy-execution-limit-off = किल स्विच बंद
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } ट्रेड जैसे हुआ वैसे ही देखा गया
       *[other] { $count } ट्रेड जैसे हुए वैसे ही देखे गए
    }
copy-execution-p95 = p95 आगमन
copy-execution-median-slippage = मीडियन स्लिपेज
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } मापा गया फ़िल
       *[other] { $count } मापे गए फ़िल
    }
copy-execution-worst-slippage = सबसे खराब स्लिपेज
copy-execution-average-slippage = औसत { $amount }
copy-execution-delay-title = पहचान में देरी
copy-execution-delay-note = वॉलेट के ब्लॉक से लेकर इस बॉट के ट्रेड देखने तक का समय। डाउनटाइम के बाद के रीप्ले शामिल नहीं हैं।
copy-execution-delay-limit = { $limit } आगमन सीमा से आगे की बार एम्बर रंग की होती हैं।
copy-execution-fastest = सबसे तेज़
copy-execution-average = औसत
copy-execution-slowest = सबसे धीमा
copy-execution-fill-title = वॉलेट के मुकाबले फ़िल
copy-execution-fill-note = धनात्मक का मतलब वॉलेट से बदतर: खरीद में ज़्यादा चुकाया, मिरर किए सेल में कम मिला। जिस टोकन का पूल प्राइस नहीं है, उसका पेपर फ़िल वॉलेट के अपने ट्रेड की कीमत पर आँका जाता है, इसलिए वह कुछ नहीं मापता और छोड़ दिया जाता है।
copy-execution-samples = सैंपल
copy-execution-median = मीडियन
copy-execution-worst = सबसे खराब
copy-execution-decisions = सीमा में निर्णय

copy-compare-title = वॉलेट की तुलना
copy-compare-back = वॉलेट पर वापस
copy-compare-load-failed = तुलना लोड नहीं हो सकी: { $error }
copy-compare-loading = तुलना लोड हो रही है…
copy-compare-empty = तुलना के लिए कोई टास्क नहीं।
copy-compare-empty-message = इसके परिणामों की दूसरों से तुलना करने के लिए एक कॉपी कार्य जोड़ें।
copy-compare-curve-title = संचयी वास्तविक लाभ-हानि
copy-table-wallet = वॉलेट
copy-table-mode = मोड
copy-table-rounds = राउंड
copy-table-realized = वास्तविक
copy-table-profit-factor = प्रॉफ़िट फ़ैक्टर
copy-table-average-hold = औसत होल्ड
copy-table-median-slippage = मीडियन स्लिपेज

copy-chart-curve-label = संचयी लाभ-हानि { $amount } { -sol }
copy-chart-compare-label = टास्क के अनुसार संचयी लाभ-हानि
copy-chart-empty-curve = इस सीमा में अभी कोई बंद राउंड नहीं।
copy-chart-empty-bars = इस सीमा में कुछ दर्ज नहीं हुआ।
copy-chart-empty-histogram = इस सीमा में कोई आगमन सैंपल नहीं।
copy-chart-empty-compare = इस सीमा में तुलना के लिए कोई बंद राउंड नहीं।
copy-chart-histogram-title = { $total } में से { $count }

copy-profile-copy = इस वॉलेट को कॉपी करें
copy-profile-copy-other = अन्य नियमों से कॉपी करें
copy-profile-loading = वॉलेट प्रोफ़ाइल लोड हो रही है…
copy-profile-watch-title = वॉच
copy-profile-watched = वॉच हो रहा है
copy-profile-watch-resume-hint = टास्क फिर शुरू करने पर यह दोबारा वॉच होता है
copy-profile-watch-add-hint = टास्क जोड़ने पर यह वॉच होना शुरू होता है
copy-profile-stream = स्ट्रीम
copy-profile-subscribed = सब्सक्राइब्ड
copy-profile-not-subscribed = सब्सक्राइब्ड नहीं
copy-profile-sources =
    { $count ->
        [one] { $count } स्रोत
       *[other] { $count } स्रोत
    }
copy-profile-last-activity = आखिरी गतिविधि
copy-profile-last-error = आखिरी त्रुटि
copy-profile-own-wallet = यह आपका अपना वॉलेट है; इसे कॉपी करना अस्वीकृत है।
copy-profile-observed-title = देखे गए ट्रेड
copy-profile-observed-none = इस बॉट में अभी इस वॉलेट का कोई ट्रेड नहीं। पेपर टास्क { -sol } खर्च किए बिना इसे देखता है।
copy-profile-swaps-seen = देखे गए स्वैप
copy-profile-swaps-seen-note = आपके टास्क में अलग-अलग वॉलेट स्वैप
copy-profile-buys-sells = खरीद / सेल
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = ट्रेड किए गए टोकन
copy-profile-first-seen = पहली बार देखा
copy-profile-last-seen = आखिरी बार देखा
copy-profile-tasks-title = इस वॉलेट पर आपके टास्क
copy-table-task = टास्क

copy-arm-acks-left =
    { $count ->
        [one] टिक करने के लिए { $count } स्वीकृति बाकी
       *[other] टिक करने के लिए { $count } स्वीकृति बाकी
    }
copy-arm-readiness-title = पेपर बुक से रेडीनेस
copy-arm-exposure-title = एक्सपोज़र
copy-arm-per-copy = प्रति कॉपी
copy-arm-budget-left-value = { $total } { -sol } में से { $left }
copy-arm-budget-left = बचा लाइव बजट
copy-arm-budget-left-note = पेपर खर्च अलग गिना जाता है और इसे इस्तेमाल नहीं करता
copy-arm-exits = एग्ज़िट
copy-arm-stop-note = { $hold } होल्ड से पहले नहीं: तेज़ गिरावट पर और नीचे बंद होता है
copy-arm-shared = इस वॉलेट को कॉपी करने वाले अन्य टास्क: { $tasks }। हर टास्क अपने बजट पर इसके ट्रेड कॉपी करता है।
copy-arm-unavailable = लाइव एक्ज़ीक्यूशन अभी उपलब्ध नहीं है; आखिरी जाँच देखें।
copy-arm-ack-real-native = असली { -sol }: यह टास्क आपके वॉलेट से अधिकतम { $budget } { -sol } खर्च कर सकता है, प्रति कॉपी अधिकतम { $trade } { -sol }।
copy-arm-ack-fees = लाइव कॉपी में असली नेटवर्क फ़ीस और स्लिपेज लगता है; पेपर नतीजे लाइव नतीजों की गारंटी नहीं हैं।
copy-arm-ack-unready = कुछ रेडीनेस जाँच पास नहीं हुई हैं। फिर भी इस टास्क को सक्रिय करें।
copy-arm-lead = टास्क “{ $name }” इस वॉलेट के ट्रेड आपके वॉलेट से असली स्वैप के ज़रिए कॉपी करेगा।
copy-arm-confirmation-missing = लाइव पुष्टि लोड नहीं हो सकी
copy-arm-armed = लाइव कॉपी सक्रिय हुई
copy-arm-failed = लाइव कॉपी सक्रिय नहीं हो सकी

copy-holdings-title = होल्डिंग
copy-holdings-view-label = होल्डिंग व्यू
copy-holdings-view-open = खुली ({ $count })
copy-holdings-view-closed = बंद राउंड ({ $count })
copy-holdings-reset = पेपर बुक रीसेट करें
copy-holdings-live-note = लाइव कॉपी असली पोज़िशन होती हैं।
copy-holdings-open-positions = खुली पोज़िशन
copy-holdings-token-details = टोकन विवरण खोलें
copy-holdings-opened = खुली: { $time }
copy-holdings-no-pool-price = पूल प्राइस नहीं
copy-holdings-close = बंद करें
copy-holdings-write-off = राइट-ऑफ़ करें
copy-holdings-activity = गतिविधि
copy-holdings-no-exit-rule = कोई एग्ज़िट नियम नहीं
copy-holdings-watch-stop = स्टॉप { $level }
copy-holdings-watch-stop-until = { $span } में स्टॉप { $level }
copy-holdings-watch-take = टेक { $level }
copy-holdings-watch-trail = ट्रेल { $level }
copy-holdings-watch-trail-arms = ट्रेल सक्रिय { $level }
copy-holdings-watch-time = समय ≤ { $level }
copy-holdings-watch-time-until = { $span } में समय ≤ { $level }
copy-holdings-watch-wallet-sells = वॉलेट सेल
copy-holdings-empty = कोई खुली पेपर होल्डिंग नहीं। वॉलेट से कॉपी की गई खरीद यहाँ दिखती हैं।
copy-holdings-col-token = टोकन
copy-holdings-col-cost = लागत
copy-holdings-col-entry = एंट्री
copy-holdings-col-mark = मार्क
copy-holdings-col-peak = पीक
copy-holdings-col-pnl = लाभ-हानि
copy-holdings-col-exit-rules = एग्ज़िट नियम
copy-holdings-col-held = होल्ड
copy-holdings-col-actions = कार्रवाइयाँ
copy-holdings-col-invested = निवेशित
copy-holdings-col-proceeds = प्राप्ति
copy-holdings-col-exit = एग्ज़िट
copy-holdings-col-closed = बंद
copy-holdings-price-note = कीमतें प्रति टोकन { -sol } में हैं। एंट्री में खरीद का स्लिपेज और फ़ीस शामिल है; पीक और एग्ज़िट स्तर इसी के सापेक्ष हैं, इसलिए होल्डिंग का पीक खुलते समय एंट्री से नीचे होता है। किसी पर होवर करके उसका पूल प्राइस देखें।
copy-holdings-paused-rules = रुका हुआ: कोई नई कॉपी नहीं। आपके एग्ज़िट नियम अब भी इन होल्डिंग को बंद करते हैं।
copy-holdings-paused-mirror = रुका हुआ: कोई नई कॉपी नहीं। वॉलेट के सेल अब भी इन होल्डिंग को बंद करते हैं।
copy-holdings-paused-hybrid = रुका हुआ: कोई नई कॉपी नहीं। वॉलेट के सेल और आपके एग्ज़िट नियम अब भी इन होल्डिंग को बंद करते हैं।
copy-holdings-closed-load-failed = बंद राउंड लोड नहीं हो सके: { $error }
copy-holdings-closed-loading = बंद राउंड लोड हो रहे हैं…
copy-holdings-closed-empty = अभी कोई बंद राउंड नहीं।
copy-holdings-closed-latest = कुल { $total } राउंड में से नवीनतम { $shown }।
copy-holdings-close-title = पेपर होल्डिंग बंद करें
copy-holdings-close-message = पेपर बुक में { $token } को पूल प्राइस ({ $price }) पर, टास्क के स्लिपेज और फ़ीस के साथ बेचें।
copy-holdings-close-confirm = होल्डिंग बंद करें
copy-holdings-write-off-title = पेपर होल्डिंग राइट-ऑफ़ करें
copy-holdings-write-off-message = बेचने के लिए { $token } का कोई पूल प्राइस नहीं है। राइट-ऑफ़ करने पर यह शून्य पर बंद होता है और इसकी { $cost } लागत घाटे के रूप में दर्ज होती है।
copy-holdings-keep = रखें
copy-holdings-written-off = राइट-ऑफ़ किया गया: { $token }
copy-holdings-closed = बंद किया गया: { $token }
copy-holdings-written-off-detail = शून्य प्राप्ति पर बंद
copy-holdings-sold-at = { $price } पर बेचा
copy-holdings-close-failed = होल्डिंग बंद नहीं हो सकी
copy-holdings-reset-message = टास्क “{ $name }” को नए सिरे से शुरू करें: इसकी पेपर होल्डिंग, खर्च, फ़िल, एग्ज़िट और स्किप हटा दिए जाएँगे। नियम और वॉलेट बने रहेंगे।
copy-holdings-reset-cancel = हिस्ट्री रखें
copy-holdings-reset-done = पेपर बुक रीसेट हुई
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } निर्णय हटाया गया
       *[other] { $count } निर्णय हटाए गए
    }
copy-holdings-reset-failed = पेपर बुक रीसेट नहीं हो सकी

copy-activity-title = गतिविधि
copy-activity-filter-label = गतिविधि फ़िल्टर
copy-filter-all = सभी
copy-outcome-paper-filled = पेपर खरीद
copy-outcome-live-submitted = लाइव खरीद सबमिट
copy-outcome-live-confirmed = लाइव खरीद कन्फ़र्म
copy-outcome-live-failed = लाइव खरीद विफल
copy-outcome-paper-sell-observed = पेपर सेल · वॉलेट ने बेचा
copy-outcome-live-sell-submitted = लाइव सेल सबमिट
copy-outcome-live-sell-failed = लाइव सेल विफल
copy-outcome-skipped = स्किप
copy-activity-decision = निर्णय
copy-activity-paper-exit = पेपर एग्ज़िट · { $rule }
copy-activity-filled = { $input } { $price } पर · वॉलेट ने { $target } खरीदा
copy-activity-filled-slippage = { $input } { $price } पर · वॉलेट ने { $target } खरीदा · स्लिपेज { $slippage }
copy-activity-filled-unpriced = { $input } { $price } पर · वॉलेट ने { $target } खरीदा · वॉलेट के ट्रेड की कीमत पर आँका गया, पूल प्राइस नहीं
copy-activity-live-sized = { $sized } · वॉलेट ने { $target } खरीदा
copy-activity-sell-nothing = वॉलेट ने { $amount } बेचा · बेचने को कुछ होल्ड नहीं था
copy-activity-written-off = शून्य पर राइट-ऑफ़: पूल प्राइस नहीं
copy-activity-sold = { $tokens } टोकन के बदले { $proceeds }, { $price } पर
copy-activity-full-close = पूरा क्लोज़
copy-activity-partial-exit = { $pct } एग्ज़िट
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = न्यूनतम { $amount }
copy-activity-skip-maximum = अधिकतम { $value }
copy-activity-skip-stale = { $arrival } देरी, सीमा { $limit }
copy-activity-skip-latency = औसत { $average }, सीमा { $limit }
copy-activity-arrival-replayed = ब्लॉक के { $span } बाद रीप्ले हुआ
copy-activity-arrival-seen = ब्लॉक के { $span } बाद देखा गया
copy-activity-link-wallet-tx = वॉलेट tx
copy-activity-link-own-tx = आपका tx
copy-activity-only-token = केवल यह टोकन
copy-activity-skipped-group = स्किप ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } टोकन · { $since } से
       *[other] { $tokens } टोकन · { $since } से
    }
copy-activity-mint-filter =
    .placeholder = टोकन मिंट
    .aria-label = टोकन मिंट से फ़िल्टर करें
copy-activity-clear = साफ़ करें
copy-activity-load-failed = गतिविधि लोड नहीं हो सकी: { $error }
copy-activity-loading = गतिविधि लोड हो रही है…
copy-activity-no-match = इस फ़िल्टर से कुछ मेल नहीं खाता।
copy-activity-empty = अभी कोई निर्णय नहीं। वॉलेट के ट्रेड करने पर फ़िल, एग्ज़िट और स्किप यहाँ दिखते हैं।
copy-activity-load-older = पुराना लोड करें
copy-activity-start = हिस्ट्री की शुरुआत
copy-activity-older-failed = पुरानी गतिविधि लोड नहीं हो सकी

copy-step-wallet = वॉलेट
copy-step-sizing = साइज़िंग
copy-step-entry = एंट्री फ़िल्टर
copy-step-exits = एग्ज़िट
copy-step-review = समीक्षा
copy-editor-title-edit = संपादित करें: { $name }
copy-editor-title-clone = क्लोन करें: { $name }
copy-editor-sub-edit = { $mode } टास्क · बदलाव इसके अगले निर्णयों पर लागू होते हैं
copy-editor-sub-clone = वही नियम, खाली पेपर बुक, पेपर से शुरू
copy-editor-save-edit = बदलाव सहेजें
copy-editor-save-clone = क्लोन बनाएँ
copy-editor-save-create = पेपर टास्क बनाएँ
copy-editor-clone-suffix = (कॉपी)
copy-editor-discard-edit = बदलाव छोड़ें
copy-editor-discard-create = यह टास्क छोड़ें
copy-editor-discard-edit-message = “{ $name }” में आपके बदलाव सहेजे नहीं गए हैं।
copy-editor-discard-create-message = अब तक दर्ज वॉलेट और नियम सहेजे नहीं गए हैं।
copy-editor-discard-confirm = छोड़ें
copy-editor-keep-editing = संपादन जारी रखें
copy-editor-toast-updated = टास्क अपडेट हुआ
copy-editor-toast-clone = क्लोन बनाया गया
copy-editor-toast-created = पेपर टास्क बनाया गया
copy-unit-native = { -sol }
copy-editor-any = कोई भी
copy-editor-duplicate = इसे पहले से कॉपी करने वाले टास्क: { $tasks }। यह टास्क अपने नियमों और बजट के साथ वही ट्रेड फिर कॉपी करता है।
copy-editor-wallet = वॉलेट
copy-editor-wallet-identity = टास्क का वॉलेट ही उसकी पहचान है। इन नियमों से दूसरा वॉलेट कॉपी करने के लिए टास्क क्लोन करें।
copy-editor-address-label = वॉलेट एड्रेस
copy-editor-address-placeholder = Solana वॉलेट एड्रेस
copy-editor-address-help-clone = वही नियम, खाली पेपर बुक के साथ। दूसरे नियम आज़माने के लिए यही वॉलेट रखें, या कोई दूसरा वॉलेट दर्ज करें।
copy-editor-address-help-create = वह वॉलेट जिसकी खरीद (और, आप चाहें तो सेल) यह टास्क कॉपी करता है।
copy-editor-name-label = नाम <em>वैकल्पिक</em>
copy-editor-name-placeholder = जैसे फ़ास्ट रोटेटर
copy-editor-enabled-title = वॉलेट के ट्रेड प्रोसेस करें
copy-editor-enabled-help = बंद रखने पर टास्क तब तक रुका रहता है जब तक आप उसे फिर शुरू न करें।
copy-editor-note-live = यह टास्क लाइव है: बदलाव इसकी अगली असली कॉपी पर लागू होते हैं।
copy-editor-note-paper = टास्क तब तक पेपर में चलते हैं जब तक आप उन्हें सक्रिय न करें: ट्रेड पूल प्राइस पर सिमुलेट होते हैं और कुछ खर्च नहीं होता।
copy-editor-copy-size = कॉपी साइज़
copy-editor-sizing-fixed = निश्चित राशि
copy-editor-sizing-ratio = वॉलेट के ट्रेड का हिस्सा
copy-editor-amount-fixed = प्रति कॉपी राशि
copy-editor-amount-ratio = हर ट्रेड का हिस्सा
copy-editor-amount-help-fixed = हर कॉपी की गई खरीद पर खर्च, कम से कम { $minimum }।
copy-editor-amount-help-ratio = वॉलेट की अपनी खरीद का, प्रति-ट्रेड सीमा तक।
copy-editor-help-trade-cap = कोई भी एक कॉपी इससे ज़्यादा खर्च नहीं करती।
copy-editor-help-token-cap = एक टोकन पर कुल खर्च।
copy-editor-help-budget = यह टास्क अपने जीवनकाल में जितना खर्च कर सकता है; पेपर और लाइव अपना-अपना खर्च गिनते हैं।
copy-editor-preview-title = एक कॉपी की लागत
copy-editor-preview-empty = कॉपी की लागत देखने के लिए साइज़िंग दर्ज करें।
copy-editor-preview-example = वॉलेट { $target } खरीदता है → आप <strong>{ $copy }</strong> कॉपी करते हैं
copy-editor-preview-once = हर टोकन एक ही बार खरीदा जाता है, इसलिए एक टोकन पर { $size } की एक कॉपी
copy-editor-preview-token-cap =
    { $count ->
        [one] एक टोकन पर अधिकतम { $count } कॉपी, हर एक { $size } की
       *[other] एक टोकन पर अधिकतम { $count } कॉपी, हर एक { $size } की
    }
copy-editor-preview-summary-exact = { $perToken }; बजट लगभग { $count } कॉपी के लिए काफ़ी है। नेटवर्क और प्रायोरिटी फ़ीस अतिरिक्त लगती है।
copy-editor-preview-summary-minimum = { $perToken }; बजट कम से कम { $count } कॉपी के लिए काफ़ी है। नेटवर्क और प्रायोरिटी फ़ीस अतिरिक्त लगती है।
copy-editor-target-min = कॉपी होने वाला सबसे छोटा वॉलेट ट्रेड
copy-editor-target-min-help = वॉलेट की छोटी खरीद अनदेखी करें। कोई न्यूनतम न चाहिए तो खाली छोड़ें।
copy-editor-target-max = कॉपी होने वाला सबसे बड़ा वॉलेट ट्रेड
copy-editor-target-max-help = वॉलेट की बड़ी खरीद अनदेखी करें। कोई अधिकतम न चाहिए तो खाली छोड़ें।
copy-editor-buy-once-title = हर टोकन एक बार खरीदें
copy-editor-buy-once-help = केवल वॉलेट की किसी टोकन की पहली खरीद कॉपी करें; उसकी बाद की खरीद स्किप होती हैं।
copy-editor-filter-require = ज़रूरी करें
copy-editor-filter-skip = ज़रूरी न करें
copy-editor-filter-help = कॉपी करने से पहले टोकन को आपकी फ़िल्टरिंग पाइपलाइन पास करना ज़रूरी करें।
copy-editor-filter-warning = डिफ़ॉल्ट फ़िल्टरिंग सेटअप में लगभग हर टोकन फ़ेल होता है, इसलिए पास ज़रूरी करने वाला टास्क कुछ भी कॉपी नहीं करता। इसे केवल तब ज़रूरी करें जब आपके फ़िल्टर वे टोकन पास करें जिन्हें यह वॉलेट ट्रेड करता है।
copy-editor-exit-both = दोनों
copy-editor-exit-help-buy-only = नीचे के आपके नियम हर होल्डिंग बेचते हैं; वॉलेट के सेल अनदेखे किए जाते हैं।
copy-editor-exit-help-hybrid = जो पहले हो: वॉलेट बेचे, या आपका कोई नियम ट्रिगर हो।
copy-editor-exit-help-mirror = होल्डिंग केवल तब बिकती है जब वॉलेट बेचता है। आपके एग्ज़िट नियम नहीं चलते।
copy-editor-who-sells = कौन बेचता है
copy-editor-preset = प्रीसेट
copy-editor-preset-help = प्रीसेट नीचे का हर नियम भर देता है; बाद में उनमें से किसी को भी बदलें।
copy-editor-mirror-note = जब तक वॉलेट के सेल तय करते हैं, ये नियम नहीं चलते। { $mine } या { $both } चुनने पर ये लागू होते हैं।
copy-editor-rule-inherit = ट्रेडर डिफ़ॉल्ट
copy-editor-inherit-value = ट्रेडर डिफ़ॉल्ट ({ $value })
copy-editor-rule-aria = { $rule } सेटिंग
copy-editor-rule-empty-uses = खाली छोड़ने पर ट्रेडर डिफ़ॉल्ट लगता है: { $value }
copy-editor-rule-follows = ट्रेडर का अनुसरण: { $summary }
copy-editor-rule-follows-plain = ट्रेडर की सेटिंग का अनुसरण।
copy-editor-rule-follows-own = चालू/बंद ट्रेडर के अनुसार, मान इस कार्य के: { $summary }
copy-editor-rule-off-note = इस टास्क के लिए बंद, ट्रेडर चाहे जो इस्तेमाल करे।
copy-task-unnamed = बिना नाम का टास्क
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = सहेजने के बाद ट्रेड प्रोसेस करता है
copy-editor-review-paused = रुका हुआ सहेजा जाएगा
copy-editor-error-address = मान्य Solana वॉलेट एड्रेस दर्ज करें।
copy-editor-error-sizing = हर साइज़िंग मान शून्य से ऊपर होना चाहिए।
copy-editor-error-min-copy = कॉपी कम से कम { $minimum } की होनी चाहिए: प्रति कॉपी राशि बढ़ाएँ।
copy-editor-error-min-cap = कॉपी कम से कम { $minimum } की होनी चाहिए: प्रति-ट्रेड सीमा बढ़ाएँ।
copy-editor-error-trade-cap = प्रति-ट्रेड सीमा प्रति-टोकन सीमा से ज़्यादा नहीं हो सकती।
copy-editor-error-token-cap = प्रति-टोकन सीमा कुल बजट से ज़्यादा नहीं हो सकती।
copy-editor-error-slippage = स्लिपेज { $min } और { $max } के बीच होना चाहिए।
copy-editor-error-target-limits = वॉलेट ट्रेड सीमाएँ शून्य या उससे ज़्यादा होनी चाहिए।
copy-editor-error-target-order = सबसे छोटा वॉलेट ट्रेड सबसे बड़े से ज़्यादा नहीं हो सकता।

copy-notice-task-unnamed = टास्क #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = पेपर कॉपी खरीद
copy-notice-title-paper-sell = पेपर कॉपी सेल
copy-notice-title-paper-closed = पेपर होल्डिंग बंद
copy-notice-title-paper-exit = पेपर एग्ज़िट: { $rule }
copy-notice-title-live-buy-submitted = लाइव कॉपी खरीद सबमिट
copy-notice-title-live-buy-confirmed = लाइव कॉपी खरीद कन्फ़र्म
copy-notice-title-live-buy-failed = लाइव कॉपी खरीद विफल
copy-notice-title-live-sell-submitted = लाइव कॉपी सेल सबमिट
copy-notice-title-live-sell-failed = लाइव कॉपी सेल विफल
copy-notice-title-auto-paused = कॉपी टास्क ऑटो-पॉज़
copy-notice-detail-bought = { $amount } { -sol } में खरीदा
copy-notice-detail-sold = { $amount } { -sol } में बेचा
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = होल्डिंग का { $percent }%
copy-notice-detail-full-close = पूरा क्लोज़
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = स्वैप विफल
