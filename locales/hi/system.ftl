system-result-config-differs = मेमोरी में मौजूद कॉन्फ़िगरेशन डिस्क वाले संस्करण से अलग है
system-result-config-matches = मेमोरी में मौजूद कॉन्फ़िगरेशन डिस्क वाले संस्करण से मेल खाता है

system-result-config-imported =
    सफलतापूर्वक इम्पोर्ट हुए: { $count ->
        [one] { $count } सेक्शन
       *[other] { $count } सेक्शन
    }
system-result-config-imported-with-warnings =
    इम्पोर्ट हुए: { $count ->
        [one] { $count } सेक्शन
       *[other] { $count } सेक्शन
    }, साथ में { $warnings ->
        [one] { $warnings } चेतावनी
       *[other] { $warnings } चेतावनी
    }: { $details }

system-config-search =
    .placeholder = सेटिंग्स खोजें...
system-config-export-title =
    .title = कॉन्फ़िगरेशन को फ़ाइल में एक्सपोर्ट करें
system-config-import-title =
    .title = फ़ाइल से कॉन्फ़िगरेशन इम्पोर्ट करें
system-config-reload = डिस्क से रीलोड करें
system-config-reset-defaults = डिफ़ॉल्ट पर रीसेट करें
system-config-select-section = कोई कॉन्फ़िगरेशन सेक्शन चुनें
system-config-select-section-details = विवरण देखने के लिए कोई कॉन्फ़िगरेशन सेक्शन चुनें।
system-config-no-metadata = <code>{ $section }</code> के लिए कोई मेटाडेटा नहीं
system-config-technical-settings = टेक्निकल सेटिंग्स
system-config-expand-title = हर सेक्शन और हर नेस्टेड सब-कॉन्फ़िग फैलाएं
system-config-collapse-title = हर सेक्शन और हर नेस्टेड सब-कॉन्फ़िग समेटें
system-config-toolbar-no-changes = सेक्शन में कोई बदलाव नहीं
system-config-toolbar-section-changes =
    { $count ->
        [one] सेक्शन में <strong>{ $count }</strong> बदलाव
       *[other] सेक्शन में <strong>{ $count }</strong> बदलाव
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] कुल <strong>{ $count }</strong> बदलाव
       *[other] कुल <strong>{ $count }</strong> बदलाव
    }

system-config-loading = कॉन्फ़िगरेशन लोड हो रहा है…
system-config-refreshing = कॉन्फ़िगरेशन रीफ़्रेश हो रहा है…
system-config-saving-title = बदलाव सहेजे जा रहे हैं…
system-config-saving-detail = कॉन्फ़िगरेशन अपडेट हो रहा है
system-config-validation-issues = <strong>सत्यापन समस्याएं मिलीं।</strong> कृपया हाइलाइट किए गए फ़ील्ड देखें।

system-config-save-changes = बदलाव सहेजें
system-config-saving = सहेज रहे हैं…
system-config-compare = डिस्क से तुलना करें
system-config-revert-section = सेक्शन वापस लाएं
system-config-summary-critical = { $count } क्रिटिकल
system-config-summary-performance = { $count } परफ़ॉर्मेंस
system-config-summary-pending =
    { $count ->
        [one] { $count } लंबित बदलाव
       *[other] { $count } लंबित बदलाव
    }
system-config-summary-none = कोई मेटाडेटा सारांश नहीं
system-config-fields-count =
    { $count ->
        [one] { $count } फ़ील्ड
       *[other] { $count } फ़ील्ड
    }
system-config-chip-pending = { $fields } · { $pending } लंबित
system-config-chip-visible = { $fields } में से { $visible }

system-config-field-unit = इकाई: { $unit }
system-config-field-default = डिफ़ॉल्ट: { $value }
system-config-field-reset = डिफ़ॉल्ट पर रीसेट करें
system-config-array-invalid-title = अमान्य ऐरे प्रविष्टि
system-config-json-invalid-title = अमान्य JSON
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
        [one] पंक्ति { $lines } मान्य पूर्णांक होनी चाहिए।
       *[other] पंक्तियां { $lines } मान्य पूर्णांक होनी चाहिए।
    }
system-config-array-invalid-number =
    { $count ->
        [one] पंक्ति { $lines } मान्य संख्या होनी चाहिए।
       *[other] पंक्तियां { $lines } मान्य संख्या होनी चाहिए।
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] पंक्ति { $lines } मान्य बूलियन होनी चाहिए।
       *[other] पंक्तियां { $lines } मान्य बूलियन होनी चाहिए।
    }
system-config-array-invalid-value =
    { $count ->
        [one] पंक्ति { $lines } मान्य मान होनी चाहिए।
       *[other] पंक्तियां { $lines } मान्य मान होनी चाहिए।
    }

system-config-telegram-actions = कार्रवाइयां
system-config-telegram-test-title = कनेक्शन टेस्ट करें
system-config-telegram-test-description = यह जांचने के लिए टेस्ट संदेश भेजें कि आपका { -telegram } कॉन्फ़िगरेशन काम कर रहा है
system-config-telegram-send-test = टेस्ट संदेश भेजें
system-config-telegram-sending = भेज रहे हैं...
system-config-telegram-configure-token-title = पहले बॉट टोकन कॉन्फ़िगर करें
system-config-telegram-configure-token-status = टेस्टिंग चालू करने के लिए ऊपर बॉट टोकन कॉन्फ़िगर करें
system-config-telegram-test-sent-status = टेस्ट संदेश सफलतापूर्वक भेजा गया! अपना { -telegram } देखें।
system-config-telegram-test-sent = { -telegram } टेस्ट संदेश भेजा गया
system-config-telegram-test-failed = टेस्ट संदेश भेजने में विफल
system-config-telegram-auth-title = बॉट ऑथेंटिकेशन
system-config-telegram-totp-title = टू-फ़ैक्टर ऑथेंटिकेशन (TOTP)
system-config-telegram-totp-configured = कॉन्फ़िगर है
system-config-telegram-totp-not-configured = कॉन्फ़िगर नहीं है
system-config-telegram-totp-active = टू-फ़ैक्टर ऑथेंटिकेशन सक्रिय है। समाप्त हुए { -telegram } सेशन के लिए आपके ऑथेंटिकेटर ऐप का TOTP कोड ज़रूरी है।
system-config-telegram-totp-inactive = { -telegram } कमांड सुरक्षित रखने के लिए सिक्योरिटी सेटिंग्स में टू-फ़ैक्टर ऑथेंटिकेशन चालू करें।
system-config-telegram-totp-note = TOTP डैशबोर्ड लॉकस्क्रीन के साथ साझा है। इसे सिक्योरिटी सेटिंग्स में कॉन्फ़िगर करें।
system-config-telegram-require-2fa = कमांड के लिए 2FA अनिवार्य करें
system-config-telegram-save-rejected = सहेजना अस्वीकृत ({ $status })
system-config-telegram-save-failed = { -telegram } सेटिंग सहेजी नहीं जा सकी

system-config-saved = कॉन्फ़िगरेशन सहेजा गया
system-config-save-failed = कॉन्फ़िगरेशन सहेजा नहीं जा सका
system-config-reloaded = कॉन्फ़िगरेशन डिस्क से रीलोड हुआ
system-config-reload-failed = कॉन्फ़िगरेशन रीलोड नहीं हो सका
system-config-diff-title = कॉन्फ़िगरेशन अंतर
system-config-diff-console = ब्राउज़र कंसोल में लिखा गया
system-config-diff-failed = अंतर की गणना नहीं हो सकी
system-config-reset-title = कॉन्फ़िगरेशन रीसेट करें
system-config-reset-message =
    इससे पूरा कॉन्फ़िगरेशन एम्बेडेड डिफ़ॉल्ट मानों पर रीसेट हो जाएगा। सभी मौजूदा सेटिंग्स खो जाएंगी।

    यह कार्रवाई वापस नहीं की जा सकती।
system-config-reset-done-title = कॉन्फ़िगरेशन रीसेट हुआ
system-config-reset-done-message = सभी सेटिंग्स डिफ़ॉल्ट मानों पर लौट गईं
system-config-reset-failed = कॉन्फ़िगरेशन रीसेट नहीं हो सका
system-config-load-failed = कॉन्फ़िगरेशन लोड नहीं हो सका
system-config-metadata-failed = कॉन्फ़िगरेशन मेटाडेटा लोड नहीं हो सका

system-config-dialog-close =
    .aria-label = बंद करें
system-config-select-none = कोई नहीं चुनें
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } बदलाव
       *[other] { $count } बदलाव
    }
system-config-sections-count =
    { $count ->
        [one] { $count } सेक्शन
       *[other] { $count } सेक्शन
    }

system-config-section-hint-chains = ब्लॉकचेन सक्षम करना, RPC एंडपॉइंट और स्वैप रूटिंग
system-config-section-hint-trader = ट्रेडिंग नियम और ऑटोमेशन
system-config-section-hint-positions = पोज़िशन मैनेजमेंट सेटिंग्स
system-config-section-hint-filtering = टोकन फ़िल्टरिंग नियम और सीमाएं
system-config-section-hint-tokens = टोकन डिस्कवरी और डेटा स्रोत
system-config-section-hint-events = इवेंट रिकॉर्डिंग सेटिंग्स
system-config-section-hint-services = बैकग्राउंड सर्विस सेटिंग्स
system-config-section-hint-monitoring = सिस्टम मॉनिटरिंग कॉन्फ़िगरेशन
system-config-section-hint-ohlcv = कैंडलस्टिक डेटा सेटिंग्स
system-config-section-hint-gui = डैशबोर्ड और UI सेटिंग्स
system-config-section-hint-telegram = { -telegram } बॉट कॉन्फ़िगरेशन

system-config-export-dialog-title = कॉन्फ़िगरेशन एक्सपोर्ट करें
system-config-export-intro = चुनें कि कौन से कॉन्फ़िगरेशन सेक्शन एक्सपोर्ट करने हैं। एक्सपोर्ट की गई फ़ाइल बाद में सेटिंग्स बहाल करने या साझा करने के लिए इम्पोर्ट की जा सकती है।
system-config-export-sections = सेक्शन
system-config-export-timestamp = एक्सपोर्ट टाइमस्टैम्प शामिल करें
system-config-sections-selected =
    { $count ->
        [one] { $count } सेक्शन चुना गया
       *[other] { $count } सेक्शन चुने गए
    }
system-config-exporting = एक्सपोर्ट हो रहा है...
system-config-export-invalid-response = सर्वर से अमान्य प्रतिक्रिया
system-config-exported-title = कॉन्फ़िगरेशन एक्सपोर्ट हुआ
system-config-exported-message =
    { $count ->
        [one] { $count } सेक्शन एक्सपोर्ट हुआ
       *[other] { $count } सेक्शन एक्सपोर्ट हुए
    }
system-config-export-failed-title = एक्सपोर्ट विफल
system-config-export-failed = कॉन्फ़िगरेशन एक्सपोर्ट करने में विफल

system-config-import-dialog-title = कॉन्फ़िगरेशन इम्पोर्ट करें
system-config-import-upload-intro = पहले से एक्सपोर्ट की गई कॉन्फ़िगरेशन फ़ाइल अपलोड करें। आप प्रीव्यू देखकर चुन सकेंगे कि कौन से सेक्शन इम्पोर्ट करने हैं।
system-config-import-dropzone-title = कॉन्फ़िग फ़ाइल यहां छोड़ें
system-config-import-dropzone-hint = या ब्राउज़ करने के लिए क्लिक करें
system-config-import-analyzing = कॉन्फ़िगरेशन का विश्लेषण हो रहा है...
system-config-import-preview = प्रीव्यू
system-config-import-preview-intro = नीचे दिए कॉन्फ़िगरेशन सेक्शन देखें। चुनें कि कौन से सेक्शन इम्पोर्ट करने हैं।
system-config-import-sections = फ़ाइल के सेक्शन
system-config-import-select-valid = सभी मान्य चुनें
system-config-import-merge-label = मौजूदा के साथ मर्ज करें
system-config-import-merge-hint = केवल फ़ाइल में मौजूद फ़ील्ड अपडेट होंगे। अनचेक होने पर पूरे सेक्शन बदल दिए जाते हैं।
system-config-import-save-label = डिस्क पर सहेजें
system-config-import-save-hint = इम्पोर्ट के बाद बदलाव config.toml में सहेजें
system-config-import-selected = चुने हुए इम्पोर्ट करें
system-config-import-warnings =
    { $count ->
        [one] { $count } चेतावनी
       *[other] { $count } चेतावनी
    }
system-config-import-warning-unknown-section = अज्ञात सेक्शन "{ $section }" को अनदेखा किया जाएगा
system-config-import-warning-sensitive-field = { $field } इम्पोर्ट करने से ऑथेंटिकेशन सेटिंग्स ओवरराइट हो सकती हैं
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = फ़ाइल में नहीं है
system-config-import-status-invalid = अमान्य कॉन्फ़िगरेशन
system-config-import-status-unchanged = कोई बदलाव नहीं
system-config-import-not-included = फ़ाइल में शामिल नहीं
system-config-import-show-changes = बदलाव दिखाएं
system-config-import-hide-changes = बदलाव छिपाएं
system-config-import-value-current = मौजूदा मान
system-config-import-value-new = नया मान
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } और बदलाव
       *[other] +{ $count } और बदलाव
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } आइटम
       *[other] { $count } आइटम
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } की
       *[other] { $count } की
    }{ "}" }
system-config-importing = इम्पोर्ट हो रहा है...
system-config-import-failed = इम्पोर्ट विफल
system-config-import-invalid-file-title = अमान्य फ़ाइल
system-config-import-invalid-file = कॉन्फ़िगरेशन फ़ाइल पार्स करने में विफल
system-config-imported-title = कॉन्फ़िगरेशन इम्पोर्ट हुआ
system-config-imported-message =
    { $count ->
        [one] { $count } सेक्शन इम्पोर्ट हुआ
       *[other] { $count } सेक्शन इम्पोर्ट हुए
    }
system-config-import-failed-title = इम्पोर्ट विफल
system-config-import-failed-message = कॉन्फ़िगरेशन इम्पोर्ट करने में विफल
