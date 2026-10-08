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
strategies-summary-period-seconds = अवधि: { $amount } सेकंड
strategies-summary-period-minutes = अवधि: { $amount } मिनट
strategies-summary-period-hours = अवधि: { $amount } घंटे

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
strategies-unit-hours = घंटे
strategies-unit-multiplier = ×

strategies-catalog-search =
    .placeholder = शर्तें खोजें...
strategies-catalog-search-clear =
    .aria-label = खोज साफ़ करें
strategies-catalog-fold-all = सभी समेटें
strategies-catalog-unfold-all = सभी फैलाएं
strategies-catalog-no-description = कोई विवरण उपलब्ध नहीं

strategies-create-title = नई स्ट्रैटेजी बनाएं
strategies-create-prompt = चुनें कि आप किस प्रकार की स्ट्रैटेजी बनाना चाहते हैं:
strategies-create-entry-name = एंट्री स्ट्रैटेजी
strategies-create-entry-description = टोकन कब खरीदना है, इसकी शर्तें तय करें
strategies-create-exit-name = एग्ज़िट स्ट्रैटेजी
strategies-create-exit-description = टोकन कब बेचना है, इसकी शर्तें तय करें

strategies-delete-title = स्ट्रैटेजी हटाएं
strategies-delete-message = स्ट्रैटेजी "{ $name }" हटाएं? यह कार्रवाई वापस नहीं की जा सकती।

strategies-toast-fix-validation = सहेजने से पहले सत्यापन त्रुटियां ठीक करें
strategies-toast-enabled = स्ट्रैटेजी चालू
    .message = "{ $name }" चालू किया गया
strategies-toast-disabled = स्ट्रैटेजी बंद
    .message = "{ $name }" बंद किया गया
strategies-toast-toggle-failed = टॉगल विफल
    .message = स्ट्रैटेजी स्टेटस अपडेट करने में विफल
strategies-toast-load-failed = लोड विफल
    .message = सर्वर से स्ट्रैटेजी लोड करने में विफल
strategies-toast-load-strategy-failed = स्ट्रैटेजी लोड करने में विफल
strategies-toast-no-strategy = कोई स्ट्रैटेजी नहीं बनी
    .message = कम से कम एक शर्त जोड़ें या पहले स्ट्रैटेजी बनाने के लिए 'नई स्ट्रैटेजी' पर क्लिक करें
strategies-toast-no-conditions-save = कोई शर्त नहीं
    .message = सहेजने से पहले स्ट्रैटेजी में कम से कम एक शर्त जोड़ें
strategies-toast-name-required = नाम ज़रूरी है
    .message = सहेजने से पहले स्ट्रैटेजी का नाम दर्ज करें
strategies-toast-saved = स्ट्रैटेजी सहेजी गई
    .message = "{ $name }" सफलतापूर्वक सहेजी गई
strategies-toast-save-failed = सहेजना विफल
    .message = स्ट्रैटेजी को डेटाबेस में सहेजने में विफल
strategies-toast-no-strategy-validate = सत्यापित करने के लिए कोई स्ट्रैटेजी नहीं
strategies-toast-no-conditions-validate = कोई शर्त नहीं
    .message = सत्यापित करने से पहले कम से कम एक शर्त जोड़ें
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
strategies-toast-condition-added = शर्त जोड़ी गई
    .message = { $name } स्ट्रैटेजी में जोड़ी गई

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
strategies-condition-price-change-percent-param-time-value = समय अवधि
    .description = लुकबैक अवधि का मान (सेकंड के लिए 1-3600, मिनट के लिए 1-1440, घंटे के लिए 1-720)
strategies-condition-price-change-percent-param-time-unit = समय इकाई
    .description = लुकबैक अवधि की समय इकाई
strategies-condition-price-change-percent-param-time-unit-option-seconds = सेकंड
strategies-condition-price-change-percent-param-time-unit-option-minutes = मिनट
strategies-condition-price-change-percent-param-time-unit-option-hours = घंटे

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

strategies-condition-param-timeframe = टाइमफ़्रेम
    .description = विश्लेषण के लिए कैंडल टाइमफ़्रेम (सेट न होने पर स्ट्रैटेजी टाइमफ़्रेम लागू होता है)
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

strategies-error-expected-boolean = बूलियन
strategies-error-expected-number = संख्या
strategies-error-expected-string = स्ट्रिंग

strategies-error-data-current-price = वर्तमान प्राइस
strategies-error-data-liquidity-data = लिक्विडिटी डेटा
strategies-error-data-market-data = मार्केट डेटा
strategies-error-data-ohlcv-data = OHLCV डेटा
strategies-error-data-position-data = पोज़िशन डेटा

strategies-error-indicator-consecutive-candles = लगातार कैंडल
strategies-error-indicator-moving-average = मूविंग एवरेज
strategies-error-indicator-price-breakout = प्राइस ब्रेकआउट
strategies-error-indicator-price-change-lookback = प्राइस बदलाव लुकबैक
strategies-error-indicator-volume-spike = वॉल्यूम स्पाइक

strategies-error-rule-branch-node-missing-conditions = ब्रांच नोड में शर्तें नहीं हैं
strategies-error-rule-branch-node-missing-operator = ब्रांच नोड में ऑपरेटर नहीं है
strategies-error-rule-branch-node-must-have-at-least-one-child = ब्रांच नोड में कम से कम एक चाइल्ड होना चाहिए
strategies-error-rule-invalid-rule-tree-structure = अमान्य रूल ट्री संरचना
strategies-error-rule-leaf-node-missing-condition = लीफ़ नोड में शर्त नहीं है
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT ऑपरेटर में ठीक एक चाइल्ड होना चाहिए
