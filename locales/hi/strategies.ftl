strategies-type-entry = एंट्री
strategies-type-exit = एग्ज़िट
strategies-list-empty-title = अभी कोई स्ट्रैटेजी नहीं
strategies-list-empty-hint = अपनी पहली स्ट्रैटेजी बनाएं
strategies-new = नई स्ट्रैटेजी
strategies-import =
    .title = स्ट्रैटेजी इम्पोर्ट करें
    .aria-label = स्ट्रैटेजी इम्पोर्ट करें

strategies-new-name = नई स्ट्रैटेजी

strategies-editor-name =
    .placeholder = स्ट्रैटेजी का नाम
strategies-editor-dirty =
    .title = बिना सहेजे बदलाव
strategies-action-validate = सत्यापित करें
strategies-conditions-empty-title = अभी कोई शर्त नहीं
strategies-conditions-empty-hint = बनाना शुरू करने के लिए "{ strategies-add-condition }" का उपयोग करें
strategies-add-condition = शर्त जोड़ें
strategies-modal-close =
    .aria-label = बंद करें
strategies-card-move-up =
    .title = ऊपर ले जाएं
strategies-card-move-down =
    .title = नीचे ले जाएं
strategies-card-duplicate =
    .title = डुप्लिकेट करें
strategies-card-delete =
    .title = हटाएं
# $name is the condition name.
strategies-card-delete-confirm = शर्त हटाएं
    .message = इस स्ट्रैटेजी से "{ $name }" हटाएं?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = कोई पैरामीटर नहीं
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = स्ट्रैटेजी की सेटिंग ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } सेकंड
strategies-value-duration-minutes = { $amount } मिनट
strategies-value-duration-hours = { $amount } घंटे
strategies-value-duration-days = { $amount } दिन
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = आपका स्थानीय समय: { $time }
strategies-hint-price-impact = इस आकार का ट्रेड कीमत को लगभग { $amount }% हिलाता है

strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } घंटा
       *[other] { $amount } घंटे
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } कैंडल
       *[other] { $amount } कैंडल
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = घं
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

strategies-catalog-search =
    .placeholder = शर्तें खोजें...
    .aria-label = शर्तें खोजें
strategies-catalog-search-clear =
    .aria-label = खोज साफ़ करें
strategies-catalog-no-description = कोई विवरण उपलब्ध नहीं

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = चालू
strategies-editor-timeframe = डिफ़ॉल्ट टाइमफ़्रेम
    .title = उन शर्तों के लिए कैंडल टाइमफ़्रेम जो अपना टाइमफ़्रेम तय नहीं करतीं
strategies-editor-order = क्रम
    .title = 1 से 100 तक मूल्यांकन क्रम; छोटी संख्याओं का मूल्यांकन पहले होता है
strategies-editor-description = विवरण
strategies-editor-description-field =
    .placeholder = यह स्ट्रैटेजी क्या करती है
    .aria-label = स्ट्रैटेजी का विवरण
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = सभी एंट्री स्ट्रैटेजी
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = प्रबंधन
strategies-editor-entry-binding-hint = चुनी गई एंट्री स्ट्रैटेजी से खुली पोज़िशन संभालती है। कोई न चुनी हो तो मैनुअल और कॉपी की गई पोज़िशन समेत हर पोज़िशन संभालती है।
strategies-editor-entry-binding-empty = अभी कोई एंट्री स्ट्रैटेजी नहीं है
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (हटाई गई)
strategies-action-export = एक्सपोर्ट
strategies-action-test = किसी टोकन पर जाँचें
# $token is the tested token's symbol or address.
strategies-test-title = { $token } पर जाँच
# $token is the tested token's symbol or address.
strategies-test-running = { $token } पर जाँच जारी…
strategies-test-verdict-pass = पूरी होती है
strategies-test-verdict-fail = पूरी नहीं होती
strategies-test-verdict-unknown = अज्ञात

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = पूरी होने वाली शर्तें: { $mode }
strategies-group-match = पूरी होनी चाहिए: { $mode }
strategies-match-all = सभी
strategies-match-any = कम से कम एक
strategies-match-mode =
    .aria-label = सभी या कम से कम एक शर्त
strategies-invert = उलटें
    .title = उलटें: यह मेल न खाने पर सही होता है
strategies-chip-off = बंद
strategies-chip-inverted = उलटा
strategies-row-switch =
    .title = सहेजे रखने के लिए बंद करें, पर मूल्यांकन न हो
    .aria-label = इस स्ट्रैटेजी द्वारा मूल्यांकित
strategies-row-expand =
    .aria-label = पैरामीटर दिखाएं या छिपाएं
strategies-add-group = ग्रुप जोड़ें
# $count is the number of conditions in the group.
strategies-group-delete-confirm = ग्रुप हटाएं
    .message =
        { $count ->
            [one] इस ग्रुप और इसकी शर्त को हटाएं?
           *[other] इस ग्रुप और इसकी { $count } शर्तों को हटाएं?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] सुधारने के लिए { $count } समस्या: { $first }
       *[other] सुधारने के लिए { $count } समस्याएं: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = कम से कम एक चालू शर्त जोड़ें
strategies-problem-group-empty = ग्रुप में कम से कम एक चालू शर्त होनी चाहिए
strategies-problem-not-applicable = इस शर्त का उपयोग इस प्रकार की स्ट्रैटेजी में नहीं किया जा सकता
strategies-problem-unknown-condition = अज्ञात शर्त प्रकार: { $type }
strategies-problem-too-deep = नियम अधिकतम { $max } स्तर तक नेस्ट हो सकते हैं; उलटा ग्रुप एक अतिरिक्त स्तर गिना जाता है
strategies-problem-required = कोई संख्या दर्ज करें
strategies-problem-range = { $min } और { $max } के बीच होना चाहिए
strategies-problem-invalid-option = दिए गए विकल्पों में से एक चुनें

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = सभी
strategies-catalog-categories =
    .aria-label = श्रेणियां
strategies-catalog-empty = आपकी खोज से कोई शर्त मेल नहीं खाती
strategies-catalog-exit-only = केवल एग्ज़िट स्ट्रैटेजी के लिए
strategies-catalog-entry-only = केवल एंट्री स्ट्रैटेजी के लिए
strategies-catalog-candles-note = { $description } केवल कैंडल पढ़ता है।
strategies-data-candles = कैंडल
strategies-data-pool = पूल
strategies-data-position = पोज़िशन
strategies-data-market = बाज़ार गतिविधि
strategies-data-clock = दिन का समय

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = ट्रेड
strategies-value-trades =
    { $count ->
        [one] { $amount } ट्रेड
       *[other] { $amount } ट्रेड
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = बहुत अधिक स्तर
    .message = नियम अधिकतम { $max } स्तर तक नेस्ट हो सकते हैं
strategies-toast-priority-invalid = अमान्य प्राथमिकता
    .message = 1 से 100 के बीच कोई पूर्ण संख्या दर्ज करें
strategies-toast-exported = स्ट्रैटेजी एक्सपोर्ट की गई
# $name is the strategy name.
strategies-unsaved-confirm = बिना सहेजे बदलाव छोड़ें?
    .message = "{ $name }" में बदलाव सहेजे नहीं गए हैं।
strategies-unsaved-discard = छोड़ें
strategies-import-error-json = फ़ाइल मान्य JSON नहीं है
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = फ़ाइल में प्राथमिकता { $min } से { $max } तक की पूर्ण संख्या होनी चाहिए
strategies-import-error-shape = फ़ाइल में कोई स्ट्रैटेजी नहीं है
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = फ़ाइल ऐसी शर्त का उपयोग करती है जो इस संस्करण में नहीं है: { $type }
strategies-import-error-not-applicable = { $name } का उपयोग इस प्रकार की स्ट्रैटेजी में नहीं किया जा सकता

strategies-create-title = नई स्ट्रैटेजी बनाएं
strategies-create-prompt = चुनें कि आप किस प्रकार की स्ट्रैटेजी बनाना चाहते हैं:
strategies-create-entry-name = एंट्री स्ट्रैटेजी
strategies-create-entry-description = टोकन कब खरीदना है, इसकी शर्तें तय करें
strategies-create-exit-name = एग्ज़िट स्ट्रैटेजी
strategies-create-exit-description = टोकन कब बेचना है, इसकी शर्तें तय करें

strategies-delete-title = स्ट्रैटेजी हटाएं
strategies-delete-message = स्ट्रैटेजी "{ $name }" हटाएं? यह कार्रवाई वापस नहीं की जा सकती।

strategies-toast-fix-validation = सहेजने से पहले सत्यापन त्रुटियां ठीक करें
strategies-toast-load-failed = लोड विफल
    .message = सर्वर से स्ट्रैटेजी लोड करने में विफल
strategies-toast-load-strategy-failed = स्ट्रैटेजी लोड करने में विफल
strategies-toast-no-strategy = कोई स्ट्रैटेजी नहीं बनी
    .message = कम से कम एक शर्त जोड़ें या पहले स्ट्रैटेजी बनाने के लिए 'नई स्ट्रैटेजी' पर क्लिक करें
strategies-toast-name-required = नाम ज़रूरी है
    .message = सहेजने से पहले स्ट्रैटेजी का नाम दर्ज करें
strategies-toast-saved = स्ट्रैटेजी सहेजी गई
    .message = "{ $name }" सफलतापूर्वक सहेजी गई
strategies-toast-save-failed = सहेजना विफल
    .message = स्ट्रैटेजी को डेटाबेस में सहेजने में विफल
strategies-toast-no-strategy-validate = सत्यापित करने के लिए कोई स्ट्रैटेजी नहीं
strategies-toast-no-strategy-test = जाँचने के लिए कोई रणनीति नहीं
strategies-toast-test-save-first = जाँच से पहले रणनीति सहेजें: जाँच सहेजे गए नियम पढ़ती है
strategies-toast-test-failed = जाँच विफल
    .message = रणनीति की जाँच नहीं हो सकी
strategies-toast-valid = स्ट्रैटेजी मान्य है
strategies-toast-invalid = स्ट्रैटेजी में त्रुटियां हैं
strategies-toast-validation-failed = सत्यापन विफल
strategies-toast-item-toggle-failed = स्ट्रैटेजी टॉगल करने में विफल
strategies-toast-deleted = स्ट्रैटेजी हटाई गई
    .message = "{ $name }" सफलतापूर्वक हटाई गई
strategies-toast-delete-failed = हटाना विफल
    .message = डेटाबेस से स्ट्रैटेजी हटाने में विफल
strategies-toast-imported = स्ट्रैटेजी इम्पोर्ट हुई
strategies-toast-import-failed = स्ट्रैटेजी इम्पोर्ट करने में विफल
strategies-toast-unknown-condition = अज्ञात शर्त
    .message = शर्त का प्रकार नहीं मिला
strategies-toast-create-first = पहले स्ट्रैटेजी बनाएं
    .message = शर्तें जोड़ने से पहले स्ट्रैटेजी बनाने के लिए 'नई स्ट्रैटेजी' पर क्लिक करें

strategies-condition-candle-size = कैंडल साइज़ पैटर्न
    .description = खास कैंडल पैटर्न पहचानें: बड़ी बॉडी, छोटी बॉडी (डोजी), लंबी विक।
strategies-condition-candle-size-param-pattern = पैटर्न प्रकार
    .description = पहचाना जाने वाला कैंडल पैटर्न
strategies-condition-candle-size-param-pattern-option-large-body = बड़ी बॉडी (तेज़ चाल)
strategies-condition-candle-size-param-pattern-option-small-body = छोटी बॉडी (डोजी/अनिर्णय)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = लंबी ऊपरी विक (रिजेक्शन)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = लंबी निचली विक (सपोर्ट)
strategies-condition-candle-size-param-threshold = साइज़ सीमा %
    .description = पैटर्न पहचान के लिए प्रतिशत सीमा

strategies-condition-consecutive-candles = लगातार कैंडल
    .description = न्यूनतम साइज़ फ़िल्टर के साथ लगातार हरी (तेज़ी) या लाल (मंदी) कैंडल पहचानें।
strategies-condition-consecutive-candles-param-count = कैंडल संख्या
    .description = ज़रूरी लगातार कैंडल की संख्या
strategies-condition-consecutive-candles-param-direction = कैंडल दिशा
    .description = लगातार कैंडल का रंग/दिशा
strategies-condition-consecutive-candles-param-direction-option-green = हरी (तेज़ी)
strategies-condition-consecutive-candles-param-direction-option-red = लाल (मंदी)
strategies-condition-consecutive-candles-param-minimum-change = न्यूनतम बदलाव %
    .description = हर कैंडल के लिए न्यूनतम % बदलाव (शोर हटाता है)

strategies-condition-liquidity-level = पूल लिक्विडिटी स्तर
    .description = { -sol } में पूल लिक्विडिटी जांचें (एंट्री: पर्याप्त लिक्विडिटी सुनिश्चित करें, एग्ज़िट: लिक्विडिटी निकासी पहचानें)
strategies-condition-liquidity-level-param-threshold = लिक्विडिटी सीमा ({ -sol })
    .description = { -sol } में पूल लिक्विडिटी स्तर
strategies-condition-liquidity-level-param-comparison = तुलना
    .description = पूल लिक्विडिटी की सीमा से तुलना कैसे हो
strategies-condition-liquidity-level-param-comparison-option-greater-than = से अधिक (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = अधिक या बराबर (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = से कम ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = कम या बराबर (≤)

strategies-condition-position-holding-time = पोज़िशन होल्डिंग समय
    .description = जांचें कि पोज़िशन कितने समय से होल्ड है
strategies-condition-position-holding-time-param-hours = समय सीमा (घंटे)
    .description = पोज़िशन खुलने के बाद से घंटों में अवधि
strategies-condition-position-holding-time-param-comparison = तुलना
    .description = पोज़िशन की आयु की सीमा से तुलना कैसे हो
strategies-condition-position-holding-time-param-comparison-option-greater-than = से पुरानी (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = कम से कम (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = से नई ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = अधिकतम (≤)

strategies-condition-price-breakout = प्राइस ब्रेकआउट
    .description = प्राइस का रेज़िस्टेंस (अवधि का उच्च) से ऊपर या सपोर्ट (अवधि का निम्न) से नीचे टूटना पहचानें।
strategies-condition-price-breakout-param-lookback = लुकबैक अवधि
    .description = सपोर्ट/रेज़िस्टेंस स्तर खोजने के लिए कैंडल की संख्या
strategies-condition-price-breakout-param-direction = ब्रेकआउट दिशा
    .description = ब्रेकआउट की दिशा
strategies-condition-price-breakout-param-direction-option-upward = ऊपर की ओर (रेज़िस्टेंस ब्रेक)
strategies-condition-price-breakout-param-direction-option-downward = नीचे की ओर (सपोर्ट ब्रेक)
strategies-condition-price-breakout-param-confirmation = पुष्टि %
    .description = ब्रेकआउट की पुष्टि के लिए स्तर से कितना आगे निकलना है (झूठे सिग्नल से बचाता है)

strategies-condition-price-change-percent = प्राइस बदलाव %
    .description = जांचें कि किसी समय अवधि में प्राइस प्रतिशत सीमा से बदला या नहीं।
strategies-condition-price-change-percent-param-percentage = बदलाव सीमा %
    .description = ट्रिगर के लिए प्राइस का प्रतिशत बदलाव (0.1-1000%)
strategies-condition-price-change-percent-param-direction = दिशा
    .description = प्राइस की चाल की दिशा
strategies-condition-price-change-percent-param-direction-option-above = बढ़त (+%)
strategies-condition-price-change-percent-param-direction-option-below = गिरावट (-%)
strategies-condition-price-change-percent-param-direction-option-within = रेंज के भीतर (±%)
strategies-condition-price-change-percent-param-time-value = लुकबैक अवधि
    .description = बदलाव कितना पीछे से मापा जाता है, फ़ील्ड के अंदर चुनी गई इकाई में (1-3600 से, 1-1440 मि, 1-720 घं)
strategies-condition-price-change-percent-param-time-unit = समय इकाई
    .description = लुकबैक अवधि की समय इकाई
strategies-condition-price-change-percent-param-time-unit-option-seconds = से
strategies-condition-price-change-percent-param-time-unit-option-minutes = मि
strategies-condition-price-change-percent-param-time-unit-option-hours = घं

strategies-condition-price-to-ma = प्राइस बनाम मूविंग एवरेज
    .description = जांचें कि प्राइस अपने सिंपल मूविंग एवरेज से ऊपर, नीचे या रेंज के भीतर है।
strategies-condition-price-to-ma-param-period = MA अवधि
    .description = मूविंग एवरेज की गणना के लिए कैंडल की संख्या
strategies-condition-price-to-ma-param-position = स्थिति
    .description = MA के सापेक्ष प्राइस की स्थिति
strategies-condition-price-to-ma-param-position-option-above = MA से ऊपर
strategies-condition-price-to-ma-param-position-option-below = MA से नीचे
strategies-condition-price-to-ma-param-position-option-within = रेंज के भीतर
strategies-condition-price-to-ma-param-distance = दूरी %
    .description = MA से न्यूनतम दूरी (ऊपर/नीचे के लिए) या अधिकतम रेंज (रेंज के भीतर के लिए)

strategies-condition-volume-spike = वॉल्यूम स्पाइक
    .description = औसत वॉल्यूम की तुलना में वॉल्यूम स्पाइक पहचानें (बढ़ती दिलचस्पी का संकेत)।
strategies-condition-volume-spike-param-lookback = लुकबैक अवधि
    .description = औसत वॉल्यूम निकालने के लिए कैंडल की संख्या
strategies-condition-volume-spike-param-multiplier = वॉल्यूम गुणक
    .description = औसत से कितने गुना ऊपर (जैसे, 2.0 = औसत का 200%)

strategies-condition-rsi = RSI (रिलेटिव स्ट्रेंथ इंडेक्स)
    .description = तब ट्रिगर होता है जब बंद कैंडलों पर वाइल्डर स्मूदिंग वाला RSI किसी सीमा से ऊपर या नीचे हो।
strategies-condition-rsi-param-period = RSI अवधि
    .description = कितनी कैंडल पर RSI निकाला जाता है; शर्त को इस मान से एक कैंडल अधिक चाहिए
strategies-condition-rsi-param-comparison = तुलना
    .description = RSI सीमा से नीचे होना चाहिए या ऊपर
strategies-condition-rsi-param-comparison-option-below = नीचे (ओवरसोल्ड)
strategies-condition-rsi-param-comparison-option-above = ऊपर (ओवरबॉट)
strategies-condition-rsi-param-threshold = RSI सीमा
    .description = 1 से 99 के बीच RSI स्तर (ओवरसोल्ड के लिए 30 और ओवरबॉट के लिए 70 आम हैं)

strategies-condition-ma-cross = मूविंग एवरेज क्रॉस
    .description = तब ट्रिगर होता है जब तेज़ मूविंग एवरेज पिछली कुछ कैंडल में धीमे एवरेज को काट दे।
strategies-condition-ma-cross-param-ma-type = एवरेज का प्रकार
    .description = एक्सपोनेंशियल एवरेज हाल की कैंडल को अधिक वज़न देते हैं; सिंपल एवरेज सभी कैंडल को बराबर वज़न देते हैं
strategies-condition-ma-cross-param-ma-type-option-ema = एक्सपोनेंशियल (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = सिंपल (SMA)
strategies-condition-ma-cross-param-fast-period = तेज़ अवधि
    .description = तेज़ एवरेज में कैंडल की संख्या
strategies-condition-ma-cross-param-slow-period = धीमी अवधि
    .description = धीमे एवरेज में कैंडल की संख्या; तेज़ अवधि से बड़ी होनी चाहिए
strategies-condition-ma-cross-param-direction = क्रॉस की दिशा
    .description = तेज़ एवरेज धीमे एवरेज को किस दिशा में काटता है
strategies-condition-ma-cross-param-direction-option-bullish = तेज़ी (तेज़ ऊपर काटे)
strategies-condition-ma-cross-param-direction-option-bearish = मंदी (तेज़ नीचे काटे)
strategies-condition-ma-cross-param-within-candles = क्रॉस इतनी कैंडल में
    .description = क्रॉस कितनी पिछली कैंडल में हो सकता है; 1 का मतलब केवल सबसे ताज़ा कैंडल

strategies-condition-position-profit = पोज़िशन का मुनाफ़ा
    .description = तब ट्रिगर होता है जब खुली पोज़िशन का अवास्तविक मुनाफ़ा एंट्री प्राइस के किसी प्रतिशत से ऊपर या नीचे हो, जिसे पूल प्राइस पर मापा जाता है।
strategies-condition-position-profit-param-comparison = तुलना
    .description = मुनाफ़ा सीमा के बराबर या उससे अधिक होना चाहिए, या बराबर या उससे कम
strategies-condition-position-profit-param-comparison-option-above = कम से कम (≥)
strategies-condition-position-profit-param-comparison-option-below = अधिकतम (≤)
strategies-condition-position-profit-param-threshold-pct = मुनाफ़ा सीमा %
    .description = एंट्री प्राइस के प्रतिशत के रूप में अवास्तविक मुनाफ़ा; नकारात्मक मान घाटा हैं

strategies-condition-drawdown-from-peak = पीक से गिरावट
    .description = तब ट्रिगर होता है जब पूल प्राइस अंतिम खरीद के बाद देखे गए सबसे ऊँचे प्राइस से किसी प्रतिशत गिर जाए, बशर्ते वह पीक एंट्री प्राइस से पर्याप्त ऊपर रहा हो।
strategies-condition-drawdown-from-peak-param-drop-pct = पीक से गिरावट %
    .description = मौजूदा प्राइस पीक से कितना नीचे होना चाहिए
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = मुनाफ़े के बाद सक्रिय %
    .description = शर्त ट्रिगर हो सके, इससे पहले पीक एंट्री प्राइस से कम से कम इतना ऊपर रहा होना चाहिए; 0 का मतलब हमेशा सक्रिय

strategies-condition-buy-pressure = खरीद दबाव
    .description = तब ट्रिगर होता है जब किसी समय विंडो के ट्रांज़ैक्शन में खरीद का हिस्सा कम से कम तय प्रतिशत हो और विंडो में पर्याप्त ट्रांज़ैक्शन हों। टोकन की खरीद और बिक्री की गिनती पढ़ता है।
strategies-condition-buy-pressure-param-window = समय विंडो
    .description = वह अवधि जिसे खरीद और बिक्री की गिनती कवर करती है
strategies-condition-buy-pressure-param-window-option-5m = 5 मिनट
strategies-condition-buy-pressure-param-window-option-1h = 1 घंटा
strategies-condition-buy-pressure-param-window-option-6h = 6 घंटे
strategies-condition-buy-pressure-param-window-option-24h = 24 घंटे
strategies-condition-buy-pressure-param-min-buy-share-pct = न्यूनतम खरीद हिस्सा %
    .description = विंडो के कुल ट्रांज़ैक्शन में खरीद का प्रतिशत
strategies-condition-buy-pressure-param-min-transactions = न्यूनतम ट्रांज़ैक्शन
    .description = कम ट्रांज़ैक्शन वाली विंडो शर्त को कभी ट्रिगर नहीं करतीं

strategies-condition-transaction-velocity = लेनदेन वेग
    .description = तब सक्रिय होती है जब हाल की विंडो की लेनदेन दर, उसे शामिल करने वाली लंबी विंडो की दर के किसी गुणक के कम से कम या अधिकतम बराबर हो। टोकन की खरीद और बिक्री गिनती पढ़ती है।
strategies-condition-transaction-velocity-param-window = विंडो
    .description = हाल की विंडो और वह लंबी विंडो जिससे उसकी तुलना होती है
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 मिनट बनाम 1 घंटा
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 घंटा बनाम 6 घंटे
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 घंटे बनाम 24 घंटे
strategies-condition-transaction-velocity-param-comparison = तुलना
    .description = हाल की दर अनुपात से कम से कम हो या अधिकतम
strategies-condition-transaction-velocity-param-comparison-option-above = कम से कम (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = अधिकतम (≤)
strategies-condition-transaction-velocity-param-ratio = दर अनुपात
    .description = हाल की विंडो में प्रति सेकंड लेनदेन को लंबी विंडो के प्रति सेकंड लेनदेन से भाग
strategies-condition-transaction-velocity-param-min-transactions = न्यूनतम लेनदेन
    .description = कम लेनदेन वाली हाल की विंडो कभी शर्त सक्रिय नहीं करतीं

strategies-condition-pullback-from-high = हाई से गिरावट
    .description = तब सक्रिय होती है जब लाइव कीमत हाल की कैंडलों के सर्वोच्च हाई से नीचे न्यूनतम और अधिकतम गिरावट के बीच हो।
strategies-condition-pullback-from-high-param-lookback = लुकबैक अवधि
    .description = कैंडलों की संख्या जिनसे सर्वोच्च हाई लिया जाता है
strategies-condition-pullback-from-high-param-min-drop-pct = न्यूनतम गिरावट %
    .description = कीमत हाई से कम से कम इतनी नीचे होनी चाहिए
strategies-condition-pullback-from-high-param-max-drop-pct = अधिकतम गिरावट %
    .description = कीमत हाई से अधिकतम इतनी नीचे होनी चाहिए; न्यूनतम गिरावट से बड़ी होनी चाहिए

strategies-condition-profit-giveback = मुनाफ़ा वापसी
    .description = तब सक्रिय होती है जब पोज़िशन ने पिछली खरीद के बाद अपने सबसे अच्छे अप्राप्त लाभ का एक हिस्सा लौटा दिया हो, जब वह लाभ एक स्तर तक पहुँच चुका हो।
strategies-condition-profit-giveback-param-arm-after-profit-pct = मुनाफ़े के बाद सक्रिय %
    .description = शर्त सक्रिय होने से पहले सबसे अच्छा लाभ प्रवेश कीमत के इस प्रतिशत तक पहुँचना चाहिए
strategies-condition-profit-giveback-param-giveback-pct = वापसी %
    .description = सबसे अच्छे लाभ का लौटाया गया हिस्सा; 100% का अर्थ है कीमत प्रवेश कीमत पर लौट आई

strategies-condition-token-age = टोकन की उम्र
    .description = तब सक्रिय होती है जब टोकन के ऑन-चेन बनने के बाद का समय किसी अवधि से अधिक या कम हो। जिस टोकन के बनने का समय अज्ञात है, उसके लिए शर्त अनिर्णीत रहती है।
strategies-condition-token-age-param-comparison = तुलना
    .description = टोकन अवधि से पुराना हो या नया
strategies-condition-token-age-param-comparison-option-older-than = इससे पुराना (≥)
strategies-condition-token-age-param-comparison-option-younger-than = इससे नया ({ "<" })
strategies-condition-token-age-param-time-value = उम्र
    .description = टोकन बनने के बाद का समय, फ़ील्ड में चुनी इकाई में (1-10000)
strategies-condition-token-age-param-time-unit = समय इकाई
    .description = उम्र की इकाई
strategies-condition-token-age-param-time-unit-option-minutes = मिनट
strategies-condition-token-age-param-time-unit-option-hours = घंटे
strategies-condition-token-age-param-time-unit-option-days = दिन

strategies-condition-vwap = VWAP
    .description = तब सक्रिय होती है जब लाइव कीमत हाल की कैंडलों की वॉल्यूम-भारित औसत कीमत से कम से कम एक दूरी ऊपर या नीचे हो।
strategies-condition-vwap-param-lookback = लुकबैक अवधि
    .description = वॉल्यूम-भारित औसत में शामिल कैंडलों की संख्या
strategies-condition-vwap-param-position = स्थिति
    .description = VWAP के सापेक्ष कीमत की स्थिति
strategies-condition-vwap-param-position-option-above = VWAP से ऊपर
strategies-condition-vwap-param-position-option-below = VWAP से नीचे
strategies-condition-vwap-param-distance-pct = दूरी %
    .description = VWAP से न्यूनतम दूरी; 0 का अर्थ है चुनी हुई ओर कोई भी दूरी

strategies-condition-volatility-range = अस्थिरता रेंज
    .description = तब सक्रिय होती है जब हाल की कैंडलों का औसत ट्रू रेंज, लाइव कीमत के प्रतिशत के रूप में, किसी सीमा से ऊपर या नीचे हो। एक समूह में ऐसी दो शर्तें एक रेंज बनाती हैं।
strategies-condition-volatility-range-param-period = ATR अवधि
    .description = कैंडलों की संख्या जिन पर औसत ट्रू रेंज स्मूद किया जाता है; शर्त को एक कैंडल और चाहिए
strategies-condition-volatility-range-param-comparison = तुलना
    .description = रेंज सीमा से ऊपर हो या नीचे
strategies-condition-volatility-range-param-comparison-option-above = ऊपर
strategies-condition-volatility-range-param-comparison-option-below = नीचे
strategies-condition-volatility-range-param-threshold-pct = रेंज सीमा %
    .description = लाइव कीमत के प्रतिशत के रूप में औसत ट्रू रेंज

strategies-condition-bollinger-bands = बोलिंजर बैंड
    .description = तब सक्रिय होती है जब लाइव कीमत निचले बैंड से नीचे या ऊपरी बैंड से ऊपर हो, या जब बैंड सिकुड़ जाएँ। बैंड औसत क्लोज़ के चारों ओर कुछ मानक विचलन पर होते हैं।
strategies-condition-bollinger-bands-param-period = अवधि
    .description = औसत और मानक विचलन में शामिल कैंडलों की संख्या
strategies-condition-bollinger-bands-param-std-dev = मानक विचलन
    .description = हर बैंड औसत से कितने मानक विचलन दूर है
strategies-condition-bollinger-bands-param-signal = सिग्नल
    .description = बैंड को क्या दिखाना चाहिए
strategies-condition-bollinger-bands-param-signal-option-below-lower = निचले बैंड से नीचे
strategies-condition-bollinger-bands-param-signal-option-above-upper = ऊपरी बैंड से ऊपर
strategies-condition-bollinger-bands-param-signal-option-squeeze = स्क्वीज़
strategies-condition-bollinger-bands-param-squeeze-width-pct = स्क्वीज़ चौड़ाई %
    .description = स्क्वीज़ के लिए: बैंडों के बीच अधिकतम दूरी, औसत के प्रतिशत के रूप में

strategies-condition-liquidity-to-size = आकार के मुकाबले लिक्विडिटी
    .description = तब सक्रिय होती है जब पूल के रिज़र्व दाँव पर लगे आकार के किसी गुणक के कम से कम या उससे कम हों: प्रवेश रणनीति में ट्रेड आकार, निकास रणनीति में पोज़िशन का शेष मूल्य।
strategies-condition-liquidity-to-size-param-comparison = तुलना
    .description = रिज़र्व गुणक के कम से कम हों या उससे कम
strategies-condition-liquidity-to-size-param-comparison-option-at-least = कम से कम (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = इससे कम ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = लिक्विडिटी गुणक
    .description = पूल रिज़र्व को दाँव पर लगे आकार से भाग; N का गुणक कीमत को लगभग 1/(N+1) हिलाता है

strategies-condition-position-adds = पोज़िशन में जोड़
    .description = तब सक्रिय होती है जब पहले प्रवेश के बाद जोड़ी गई खरीदों, या आंशिक बिक्रियों, की संख्या किसी संख्या के कम से कम या अधिकतम हो।
strategies-condition-position-adds-param-subject = गिनती का विषय
    .description = कौन सी क्रियाएँ गिनी जाती हैं
strategies-condition-position-adds-param-subject-option-adds = जोड़ी गई खरीदें
strategies-condition-position-adds-param-subject-option-partial-sells = आंशिक बिक्रियाँ
strategies-condition-position-adds-param-comparison = तुलना
    .description = गिनती संख्या के कम से कम हो या अधिकतम
strategies-condition-position-adds-param-comparison-option-at-least = कम से कम (≥)
strategies-condition-position-adds-param-comparison-option-at-most = अधिकतम (≤)
strategies-condition-position-adds-param-count = गिनती
    .description = क्रियाओं की संख्या

strategies-condition-time-of-day = दिन का समय
    .description = तब सक्रिय होती है जब वर्तमान घंटा UTC घंटों की किसी विंडो में हो। समाप्ति के बाद का आरंभ आधी रात के पार जाता है।
strategies-condition-time-of-day-param-start-hour = आरंभ घंटा (UTC)
    .description = विंडो का पहला घंटा, 0 से 23
strategies-condition-time-of-day-param-end-hour = समाप्ति घंटा (UTC)
    .description = वह घंटा जिस पर विंडो समाप्त होती है, 1 से 24; वह घंटा स्वयं विंडो से बाहर है

strategies-condition-liquidity-change-since-entry = प्रवेश के बाद लिक्विडिटी बदलाव
    .description = तब सक्रिय होती है जब पोज़िशन खुलने के बाद से पूल के रिज़र्व कम से कम किसी प्रतिशत से गिरे या बढ़े हों। जिस पोज़िशन की प्रवेश लिक्विडिटी दर्ज नहीं है, उसके लिए शर्त अनिर्णीत रहती है।
strategies-condition-liquidity-change-since-entry-param-comparison = तुलना
    .description = रिज़र्व गिरे हों या बढ़े हों
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = इतना गिरा
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = इतना बढ़ा
strategies-condition-liquidity-change-since-entry-param-change-pct = बदलाव %
    .description = प्रवेश के बाद पूल रिज़र्व में बदलाव

strategies-condition-param-timeframe = टाइमफ़्रेम
    .description = कैंडल आकार: शर्त जिस हर कैंडल को पढ़ती है उसकी अवधि (सेट न होने पर स्ट्रैटेजी टाइमफ़्रेम)
strategies-condition-timeframe-option-1m = 1 मिनट
strategies-condition-timeframe-option-5m = 5 मिनट
strategies-condition-timeframe-option-15m = 15 मिनट
strategies-condition-timeframe-option-1h = 1 घंटा
strategies-condition-timeframe-option-4h = 4 घंटे
strategies-condition-timeframe-option-12h = 12 घंटे
strategies-condition-timeframe-option-1d = 1 दिन

strategies-condition-category-price-analysis = प्राइस विश्लेषण
strategies-condition-category-candle-patterns = कैंडल पैटर्न
strategies-condition-category-technical-indicators = टेक्निकल इंडिकेटर
strategies-condition-category-market-context = मार्केट संदर्भ
strategies-condition-category-position-performance = पोज़िशन और प्रदर्शन
strategies-condition-category-volume-analysis = वॉल्यूम विश्लेषण

strategies-error-missing-parameter = { $field } पैरामीटर गायब है
strategies-error-parameter-type = { $field } पैरामीटर { $expected } होना चाहिए
strategies-error-invalid-value = "{ $value }" मान्य { $field } नहीं है
strategies-error-missing-data = { $data } उपलब्ध नहीं है
strategies-error-no-candle-data = { $timeframe } टाइमफ़्रेम के लिए कैंडल डेटा नहीं है
strategies-error-insufficient-history = { $indicator } के लिए पर्याप्त इतिहास नहीं: { $available } सेकंड उपलब्ध, { $required } सेकंड चाहिए
strategies-error-insufficient-candles = { $indicator } के लिए पर्याप्त कैंडल नहीं: उपलब्ध { $available }, ज़रूरी { $required }
strategies-error-stale-candle-data = { $timeframe } कैंडल डेटा पुराना है: इसकी आयु { $age } सेकंड, { $max } सेकंड से अधिक है
strategies-error-stale-market-data = { $window } ट्रांज़ैक्शन की गिनती पुरानी है: इसकी आयु { $age } सेकंड, { $max } सेकंड से अधिक है
strategies-error-unusable-data = { $field } का मान "{ $value }" उपयोग नहीं किया जा सकता
strategies-error-invalid-rule-tree = अमान्य रूल ट्री: { $reason }
strategies-error-evaluation-timeout = स्ट्रैटेजी मूल्यांकन { $timeout } ms बाद टाइमआउट हुआ
strategies-error-condition-not-applicable = { $condition } को { $strategy } प्रकार की रणनीतियों में इस्तेमाल नहीं किया जा सकता
strategies-error-invalid-rules = नियम पढ़े नहीं जा सके: { $reason }

strategies-error-field-average-volume = औसत वॉल्यूम
strategies-error-field-candle-open = कैंडल ओपन
strategies-error-field-comparison = तुलना
strategies-error-field-condition-type = शर्त प्रकार
strategies-error-field-confirmation = पुष्टि
strategies-error-field-count = संख्या
strategies-error-field-current-price = वर्तमान प्राइस
strategies-error-field-direction = दिशा
strategies-error-field-distance = दूरी
strategies-error-field-hours = घंटे
strategies-error-field-lookback = लुकबैक
strategies-error-field-minimum-change = न्यूनतम बदलाव
strategies-error-field-multiplier = गुणक
strategies-error-field-pattern = पैटर्न
strategies-error-field-percentage = प्रतिशत
strategies-error-field-period = अवधि
strategies-error-field-position = स्थिति
strategies-error-field-threshold = सीमा
strategies-error-field-time-unit = समय इकाई
strategies-error-field-time-value = समय मान
strategies-error-field-timeframe = टाइमफ़्रेम
strategies-error-field-arm-after-profit = मुनाफ़े के बाद सक्रिय
strategies-error-field-drop-percentage = गिरावट प्रतिशत
strategies-error-field-entry-price = एंट्री प्राइस
strategies-error-field-fast-period = तेज़ अवधि
strategies-error-field-minimum-buy-share = न्यूनतम खरीद हिस्सा
strategies-error-field-minimum-transactions = न्यूनतम ट्रांज़ैक्शन
strategies-error-field-moving-average-type = एवरेज का प्रकार
strategies-error-field-moving-average = मूविंग एवरेज
strategies-error-field-reference-candle-close = रेफ़रेंस कैंडल क्लोज़
strategies-error-field-peak-price = पीक प्राइस
strategies-error-field-slow-period = धीमी अवधि
strategies-error-field-window = विंडो
strategies-error-field-within-candles = क्रॉस की कैंडल संख्या
strategies-error-field-change = बदलाव
strategies-error-field-end-hour = समाप्ति घंटा
strategies-error-field-entry-liquidity = प्रवेश लिक्विडिटी
strategies-error-field-giveback = वापसी
strategies-error-field-highest-high = सर्वोच्च हाई
strategies-error-field-long-window-transactions = लंबी विंडो के लेनदेन
strategies-error-field-maximum-drop = अधिकतम गिरावट
strategies-error-field-minimum-drop = न्यूनतम गिरावट
strategies-error-field-multiple = गुणक
strategies-error-field-pool-liquidity = पूल लिक्विडिटी
strategies-error-field-ratio = अनुपात
strategies-error-field-remaining-position-value = पोज़िशन का शेष मूल्य
strategies-error-field-signal = सिग्नल
strategies-error-field-squeeze-width = स्क्वीज़ चौड़ाई
strategies-error-field-standard-deviations = मानक विचलन
strategies-error-field-start-hour = आरंभ घंटा
strategies-error-field-subject = गिनती का विषय
strategies-error-field-token-creation-time = टोकन बनने का समय
strategies-error-field-total-volume = कुल वॉल्यूम
strategies-error-field-trade-size = ट्रेड आकार
strategies-error-field-vwap = VWAP

strategies-error-expected-boolean = बूलियन
strategies-error-expected-number = संख्या
strategies-error-expected-string = स्ट्रिंग

strategies-error-data-current-price = वर्तमान प्राइस
strategies-error-data-liquidity-data = लिक्विडिटी डेटा
strategies-error-data-market-data = मार्केट डेटा
strategies-error-data-ohlcv-data = OHLCV डेटा
strategies-error-data-position-data = पोज़िशन डेटा
strategies-error-data-peak-price = पीक प्राइस
strategies-error-data-transaction-counts = ट्रांज़ैक्शन की गिनती
strategies-error-data-entry-liquidity = प्रवेश लिक्विडिटी
strategies-error-data-entry-pool-liquidity = एंट्री पूल की लिक्विडिटी
strategies-error-data-real-pool-reserves = पूल की वास्तविक लिक्विडिटी
strategies-error-data-remaining-position-value = पोज़िशन का शेष मूल्य
strategies-error-data-token-creation-time = टोकन बनने का समय
strategies-error-data-trade-size = ट्रेड आकार

strategies-error-indicator-consecutive-candles = लगातार कैंडल
strategies-error-indicator-moving-average = मूविंग एवरेज
strategies-error-indicator-price-breakout = प्राइस ब्रेकआउट
strategies-error-indicator-price-change-lookback = प्राइस बदलाव लुकबैक
strategies-error-indicator-volume-spike = वॉल्यूम स्पाइक
strategies-error-indicator-moving-average-cross = मूविंग एवरेज क्रॉस
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = बोलिंजर बैंड
strategies-error-indicator-pullback-from-high = हाई से गिरावट
strategies-error-indicator-volatility-range = अस्थिरता रेंज
strategies-error-indicator-vwap = VWAP

strategies-error-rule-branch-node-missing-conditions = ब्रांच नोड में शर्तें नहीं हैं
strategies-error-rule-branch-node-missing-operator = ब्रांच नोड में ऑपरेटर नहीं है
strategies-error-rule-branch-node-must-have-at-least-one-child = ब्रांच नोड में कम से कम एक चाइल्ड होना चाहिए
strategies-error-rule-invalid-rule-tree-structure = अमान्य रूल ट्री संरचना
strategies-error-rule-leaf-node-missing-condition = लीफ़ नोड में शर्त नहीं है
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT ऑपरेटर में ठीक एक चाइल्ड होना चाहिए
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = हर समूह में कम से कम एक शर्त चालू होनी चाहिए
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT ऑपरेटर का चाइल्ड बंद नहीं हो सकता
strategies-error-rule-rules-nest-deeper-than-four-levels = नियम चार स्तर से अधिक गहरे नेस्ट नहीं हो सकते
strategies-error-rule-the-root-group-cannot-be-switched-off = रूट समूह को बंद नहीं किया जा सकता

strategies-readiness-history-since-met =
    { $count ->
        [one] { $changed } को नियम बदलने के बाद से { $count } बंद पेपर राउंड, { $needed } ज़रूरी
       *[other] { $changed } को नियम बदलने के बाद से { $count } बंद पेपर राउंड, { $needed } ज़रूरी
    }
strategies-readiness-history-since-short = { $changed } को नियम बदलने के बाद से { $needed } में से { $count } बंद पेपर राउंड
strategies-readiness-priced = हर पोज़िशन की कीमत उपलब्ध
strategies-readiness-priced-ok = हर खुली पेपर पोज़िशन का पूल प्राइस है
strategies-readiness-priced-missing =
    { $count ->
        [one] { $count } खुली पेपर पोज़िशन का पूल प्राइस नहीं है
       *[other] { $count } खुली पेपर पोज़िशन का पूल प्राइस नहीं है
    }
strategies-readiness-runtime = लाइव ट्रेडिंग उपलब्ध
strategies-readiness-runtime-ok = सेटअप और सुरक्षा गेट लाइव ट्रेड की अनुमति देते हैं

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = सभी स्ट्रैटेजी
strategies-view-kind = स्ट्रैटेजी का प्रकार
strategies-unnamed = बिना नाम की स्ट्रैटेजी
strategies-col-on = चालू
strategies-col-strategy = स्ट्रैटेजी
strategies-col-order = क्रम
    .title = एक मोड की चालू एंट्री स्ट्रैटेजी के बीच मूल्यांकन क्रम; पहले सिग्नल देने वाली स्ट्रैटेजी टोकन लेती है
strategies-col-mode = मोड
strategies-col-status = स्थिति
strategies-col-open = खुली
strategies-col-watching = निगरानी में
strategies-col-rounds = राउंड
strategies-col-closed = बंद
strategies-col-win-rate = जीत दर
strategies-col-realized = प्राप्त ({ -sol })
strategies-col-manages = प्रबंधित करती है
strategies-col-last-signal = आख़िरी सिग्नल
strategies-status-running = चल रही है
strategies-status-not-running = नहीं चल रही
strategies-status-off = बंद
strategies-status-setup-required = सेटअप ज़रूरी
strategies-status-live-paused = लाइव एंट्री रुकी हुई
strategies-status-paper-cash = पेपर कैश कम
strategies-status-slots-full = स्लॉट भरे हुए
strategies-status-entry-monitor-off = एंट्री मॉनिटर बंद
strategies-status-exit-monitor-off = एग्ज़िट मॉनिटर बंद
strategies-notice-setup-required = पेपर ट्रेडिंग सेटअप के बाद शुरू होती है
strategies-notice-force-stopped = आपातकालीन रोक लगी है
strategies-notice-trader-off = Auto Trader बंद है
strategies-notice-waiting = मुख्य सेवाओं की प्रतीक्षा
strategies-notice-monitors-off = एंट्री और एग्ज़िट मॉनिटर बंद हैं
strategies-notice-start = Auto Trader शुरू करें
# $closed and $needed count closed paper rounds; $changed is a formatted date.
strategies-readiness-progress = पेपर राउंड { $closed }/{ $needed }
strategies-readiness-ready = लाइव के लिए तैयार
strategies-readiness-since = नियम बदले { $changed }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
        [one] { $book }: { $count } पोज़िशन
       *[other] { $book }: { $count } पोज़िशन
    }
strategies-manages-all = सभी एंट्री स्ट्रैटेजी
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
        [one] { $first } और { $more } अन्य
       *[other] { $first } और { $more } अन्य
    }
strategies-exit-before = पहले जाँचे जाते हैं
strategies-exit-hint = ये एग्ज़िट नियम हर पोज़िशन पर किसी भी एग्ज़िट स्ट्रैटेजी से पहले जाँचे जाते हैं
strategies-action-edit = { $name } संपादित करें
strategies-action-delete = { $name } हटाएँ
strategies-roster-stale = आख़िरी अपडेट

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = लाइव “{ $name }” के नियम बदलें
    .message = नए नियम अगली जाँच से असली { -sol } से ट्रेड करते हैं, और इस स्ट्रैटेजी का पेपर प्रमाण इसी सेव से फिर शुरू होता है।
strategies-live-rules-save = सेव करें और लाइव ट्रेड करें
strategies-toast-evidence-restarted = स्ट्रैटेजी सहेजी गई
    .message = “{ $name }” के नियम बदल गए; इसका पेपर प्रमाण अब से फिर शुरू होता है

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = लाइव सक्रिय करें
strategies-mode-return-paper = पेपर पर लौटें
strategies-mode-arm-title = “{ $name }” को लाइव सक्रिय करें
strategies-mode-arm-lead = “{ $name }” की नई एंट्री आपके ट्रेडिंग वॉलेट से असली स्वैप के ज़रिए होंगी।
strategies-mode-arm-lead-exit = सिग्नल मिलने पर “{ $name }” आपके ट्रेडिंग वॉलेट से असली स्वैप द्वारा लाइव पोज़िशन बेचेगी।
strategies-mode-arm-readiness = पेपर बुक से रेडीनेस
strategies-mode-arm-open-paper = खुली पेपर पोज़िशन पेपर ही रहती हैं और पेपर पर ही बंद होती हैं।
strategies-mode-arm-ack-real-native = नई एंट्री आपके ट्रेडिंग वॉलेट से असली { -sol } खर्च करती हैं।
strategies-mode-arm-ack-real-native-exit = इसकी प्रबंधित लाइव पोज़िशन असली स्वैप से बेची जाती हैं।
strategies-mode-arm-ack-fees = लाइव स्वैप में असली नेटवर्क फ़ीस और स्लिपेज लगता है; पेपर नतीजे लाइव नतीजों की गारंटी नहीं हैं।
strategies-mode-arm-ack-unready = कुछ रेडीनेस जाँच पास नहीं हुई हैं। फिर भी इस स्ट्रैटेजी को सक्रिय करें।
strategies-mode-arm-unavailable = लाइव ट्रेडिंग अभी उपलब्ध नहीं है; आखिरी जाँच देखें।
strategies-mode-return-title = “{ $name }” को पेपर पर लौटाएँ
strategies-mode-return-message = नई एंट्री बिना { -sol } खर्च किए पेपर बुक में खुलती हैं। खुली लाइव पोज़िशन लाइव ही रहती हैं और लाइव ही बंद होती हैं।
strategies-mode-return-message-exit = यह लाइव पोज़िशन का मूल्यांकन बंद कर देती है; उनके एग्ज़िट नियम बने रहते हैं।
strategies-mode-readiness-failed = स्ट्रैटेजी की रेडीनेस नहीं पढ़ी जा सकी
strategies-mode-toast-armed = { $name } अब लाइव ट्रेड करती है
strategies-mode-toast-paper = { $name } अब पेपर पर ट्रेड करती है
strategies-mode-toast-failed = ट्रेडिंग मोड नहीं बदला जा सका
