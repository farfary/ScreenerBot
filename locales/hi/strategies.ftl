strategies-filter-all = सभी
strategies-filter-entry = एंट्री
strategies-filter-exit = एग्ज़िट
strategies-type-entry = एंट्री
strategies-type-exit = एग्ज़िट
strategies-list-empty-title = अभी कोई स्ट्रैटेजी नहीं
strategies-list-empty-hint = अपनी पहली स्ट्रैटेजी बनाएं
strategies-new = नई स्ट्रैटेजी
strategies-import =
    .title = स्ट्रैटेजी इम्पोर्ट करें
    .aria-label = स्ट्रैटेजी इम्पोर्ट करें
strategies-item-enable =
    .title = चालू करें
strategies-item-disable =
    .title = बंद करें

strategies-new-name = नई स्ट्रैटेजी

strategies-editor-name =
    .placeholder = स्ट्रैटेजी का नाम
strategies-editor-dirty =
    .title = बिना सहेजे बदलाव
strategies-action-validate = सत्यापित करें
strategies-editor-empty = संपादित करने के लिए कोई स्ट्रैटेजी चुनें, या नई बनाएं
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
strategies-summary-period-seconds = लुकबैक अवधि: { $amount } सेकंड
strategies-summary-period-minutes = लुकबैक अवधि: { $amount } मिनट
strategies-summary-period-hours = लुकबैक अवधि: { $amount } घंटे

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

strategies-catalog-search =
    .placeholder = शर्तें खोजें...
    .aria-label = शर्तें खोजें
strategies-catalog-search-clear =
    .aria-label = खोज साफ़ करें
strategies-catalog-no-description = कोई विवरण उपलब्ध नहीं

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = सक्रिय
strategies-editor-timeframe = डिफ़ॉल्ट टाइमफ़्रेम
    .title = उन शर्तों के लिए कैंडल टाइमफ़्रेम जो अपना टाइमफ़्रेम तय नहीं करतीं
strategies-editor-priority = प्राथमिकता
    .title = 1 से 100 तक मूल्यांकन क्रम; छोटी संख्याओं का मूल्यांकन पहले होता है
strategies-editor-description = विवरण
strategies-editor-description-field =
    .placeholder = यह स्ट्रैटेजी क्या करती है
    .aria-label = स्ट्रैटेजी का विवरण
strategies-action-export = एक्सपोर्ट

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
strategies-data-candles = कैंडल
strategies-data-pool = पूल
strategies-data-position = पोज़िशन
strategies-data-market = बाज़ार गतिविधि

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
strategies-toast-valid = स्ट्रैटेजी मान्य है
strategies-toast-invalid = स्ट्रैटेजी में त्रुटियां हैं
strategies-toast-validation-failed = सत्यापन विफल
strategies-toast-item-enabled = स्ट्रैटेजी चालू
strategies-toast-item-disabled = स्ट्रैटेजी बंद
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
    .description = खास कैंडल पैटर्न पहचानें: बड़ी बॉडी, छोटी बॉडी (डोजी), लंबी विक
strategies-condition-candle-size-param-pattern = पैटर्न प्रकार
    .description = पहचाना जाने वाला कैंडल पैटर्न
strategies-condition-candle-size-param-pattern-option-large-body = बड़ी बॉडी (तेज़ चाल)
strategies-condition-candle-size-param-pattern-option-small-body = छोटी बॉडी (डोजी/अनिर्णय)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = लंबी ऊपरी विक (रिजेक्शन)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = लंबी निचली विक (सपोर्ट)
strategies-condition-candle-size-param-threshold = साइज़ सीमा %
    .description = पैटर्न पहचान के लिए प्रतिशत सीमा

strategies-condition-consecutive-candles = लगातार कैंडल
    .description = न्यूनतम साइज़ फ़िल्टर के साथ लगातार हरी (तेज़ी) या लाल (मंदी) कैंडल पहचानें
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
    .description = जांचें कि पोज़िशन कितने समय से होल्ड है (एग्ज़िट स्ट्रैटेजी के लिए - समय-आधारित एग्ज़िट)
strategies-condition-position-holding-time-param-hours = समय सीमा (घंटे)
    .description = पोज़िशन खुलने के बाद से घंटों में अवधि
strategies-condition-position-holding-time-param-comparison = तुलना
    .description = पोज़िशन की आयु की सीमा से तुलना कैसे हो
strategies-condition-position-holding-time-param-comparison-option-greater-than = से पुरानी (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = कम से कम (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = से नई ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = अधिकतम (≤)

strategies-condition-price-breakout = प्राइस ब्रेकआउट
    .description = प्राइस का रेज़िस्टेंस (अवधि का उच्च) से ऊपर या सपोर्ट (अवधि का निम्न) से नीचे टूटना पहचानें
strategies-condition-price-breakout-param-lookback = लुकबैक अवधि
    .description = सपोर्ट/रेज़िस्टेंस स्तर खोजने के लिए कैंडल की संख्या
strategies-condition-price-breakout-param-direction = ब्रेकआउट दिशा
    .description = ब्रेकआउट की दिशा
strategies-condition-price-breakout-param-direction-option-upward = ऊपर की ओर (रेज़िस्टेंस ब्रेक)
strategies-condition-price-breakout-param-direction-option-downward = नीचे की ओर (सपोर्ट ब्रेक)
strategies-condition-price-breakout-param-confirmation = पुष्टि %
    .description = ब्रेकआउट की पुष्टि के लिए स्तर से कितना आगे निकलना है (झूठे सिग्नल से बचाता है)

strategies-condition-price-change-percent = प्राइस बदलाव %
    .description = जांचें कि किसी समय अवधि में प्राइस प्रतिशत सीमा से बदला या नहीं
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
    .description = जांचें कि प्राइस अपने सिंपल मूविंग एवरेज से ऊपर, नीचे या रेंज के भीतर है
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
    .description = औसत वॉल्यूम की तुलना में वॉल्यूम स्पाइक पहचानें (बढ़ती दिलचस्पी का संकेत)
strategies-condition-volume-spike-param-lookback = लुकबैक अवधि
    .description = औसत वॉल्यूम निकालने के लिए कैंडल की संख्या
strategies-condition-volume-spike-param-multiplier = वॉल्यूम गुणक
    .description = औसत से कितने गुना ऊपर (जैसे, 2.0 = औसत का 200%)

strategies-condition-rsi = RSI (रिलेटिव स्ट्रेंथ इंडेक्स)
    .description = तब ट्रिगर होता है जब कैंडल क्लोज़ का RSI, वाइल्डर की विधि से स्मूद किया हुआ, किसी सीमा से नीचे या ऊपर हो। केवल कैंडल पढ़ता है।
strategies-condition-rsi-param-period = RSI अवधि
    .description = कितनी कैंडल पर RSI निकाला जाता है; शर्त को इस मान से एक कैंडल अधिक चाहिए
strategies-condition-rsi-param-comparison = तुलना
    .description = RSI सीमा से नीचे होना चाहिए या ऊपर
strategies-condition-rsi-param-comparison-option-below = नीचे (ओवरसोल्ड)
strategies-condition-rsi-param-comparison-option-above = ऊपर (ओवरबॉट)
strategies-condition-rsi-param-threshold = RSI सीमा
    .description = 1 से 99 के बीच RSI स्तर (ओवरसोल्ड के लिए 30 और ओवरबॉट के लिए 70 आम हैं)

strategies-condition-ma-cross = मूविंग एवरेज क्रॉस
    .description = तब ट्रिगर होता है जब तेज़ मूविंग एवरेज पिछली कुछ कैंडल में धीमे एवरेज को काट दे। केवल कैंडल पढ़ता है।
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
    .description = तब ट्रिगर होता है जब खुली पोज़िशन का अवास्तविक मुनाफ़ा, पूल प्राइस को एंट्री प्राइस से तुलना कर मापा गया, किसी प्रतिशत से ऊपर या नीचे हो। केवल एग्ज़िट रणनीतियों के लिए।
strategies-condition-position-profit-param-comparison = तुलना
    .description = मुनाफ़ा सीमा के बराबर या उससे अधिक होना चाहिए, या बराबर या उससे कम
strategies-condition-position-profit-param-comparison-option-above = कम से कम (≥)
strategies-condition-position-profit-param-comparison-option-below = अधिकतम (≤)
strategies-condition-position-profit-param-threshold-pct = मुनाफ़ा सीमा %
    .description = एंट्री प्राइस के प्रतिशत के रूप में अवास्तविक मुनाफ़ा; नकारात्मक मान घाटा हैं

strategies-condition-drawdown-from-peak = पीक से गिरावट
    .description = तब ट्रिगर होता है जब पूल प्राइस एंट्री के बाद देखे गए सबसे ऊँचे प्राइस से किसी प्रतिशत गिर जाए, बशर्ते वह पीक एंट्री प्राइस से पर्याप्त ऊपर रहा हो। केवल पूल प्राइस पढ़ता है। केवल एग्ज़िट रणनीतियों के लिए।
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
strategies-error-field-peak-price = पीक प्राइस
strategies-error-field-slow-period = धीमी अवधि
strategies-error-field-window = विंडो
strategies-error-field-within-candles = क्रॉस की कैंडल संख्या

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

strategies-error-indicator-consecutive-candles = लगातार कैंडल
strategies-error-indicator-moving-average = मूविंग एवरेज
strategies-error-indicator-price-breakout = प्राइस ब्रेकआउट
strategies-error-indicator-price-change-lookback = प्राइस बदलाव लुकबैक
strategies-error-indicator-volume-spike = वॉल्यूम स्पाइक
strategies-error-indicator-moving-average-cross = मूविंग एवरेज क्रॉस
strategies-error-indicator-rsi = RSI

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
