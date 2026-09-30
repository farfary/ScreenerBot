setup-wallet-required = वॉलेट की प्राइवेट की दर्ज करें।
setup-wallet-json-recognized = 64-बाइट JSON की फ़ॉर्मैट पहचाना गया।
setup-wallet-json-invalid = ठीक 64 बाइट मान (0–255) वाला JSON ऐरे इस्तेमाल करें।
setup-wallet-format-invalid = base58 प्राइवेट की या 64-बाइट JSON ऐरे इस्तेमाल करें।
setup-wallet-base58-recognized = Base58 की फ़ॉर्मैट पहचाना गया।

setup-rpc-required = कम से कम एक RPC एंडपॉइंट दर्ज करें।
setup-rpc-too-many = 10 से ज़्यादा RPC एंडपॉइंट इस्तेमाल न करें।
setup-rpc-url-invalid = हर एंडपॉइंट एक मान्य HTTPS URL होना चाहिए।
setup-rpc-url-credentials = RPC URL में यूज़रनेम या पासवर्ड शामिल नहीं हो सकते।
setup-rpc-url-fragment = RPC URL में फ़्रैगमेंट शामिल नहीं हो सकते।
setup-rpc-public-endpoint = पब्लिक Solana RPC लगातार पोलिंग को सपोर्ट नहीं कर सकता।
setup-rpc-private-host = RPC एंडपॉइंट लोकल या प्राइवेट नेटवर्क होस्ट इस्तेमाल नहीं कर सकते।
setup-rpc-duplicate = डुप्लिकेट RPC एंडपॉइंट हटाएं।
setup-rpc-ready =
    { $count ->
        [one] { $count } HTTPS एंडपॉइंट टेस्ट के लिए तैयार।
       *[other] { $count } HTTPS एंडपॉइंट टेस्ट के लिए तैयार।
    }

setup-wallet-verified = वॉलेट सत्यापित
setup-wallet-unverified = वॉलेट सत्यापित नहीं हो सका
setup-wallet-address-detail = एड्रेस { $address }
setup-wallet-format-hint = प्राइवेट की का फ़ॉर्मैट जांचें।
setup-rpc-none-working = कोई काम करने वाला मेननेट RPC नहीं
setup-rpc-health-failed = कोई भी एंडपॉइंट मेननेट हेल्थ चेक पास नहीं कर सका।
setup-rpc-partial = { $working } काम कर रहे हैं; { $failed } उपलब्ध नहीं
setup-rpc-verified =
    { $count ->
        [one] { $count } मेननेट एंडपॉइंट सत्यापित
       *[other] { $count } मेननेट एंडपॉइंट सत्यापित
    }
setup-rpc-fastest = सबसे तेज़: { $url } ({ $latency } ms)।
setup-error-request-failed = रिक्वेस्ट विफल ({ $status })
setup-error-restart-timeout = सेटअप सहेज लिया गया है, लेकिन { -brand } अभी दोबारा कनेक्ट नहीं हुआ है।

setup-verify-wallet-parsing = प्राइवेट की पार्स हो रही है
setup-verify-wallet-parsing-detail = की जांची जा रही है और उसका पब्लिक एड्रेस निकाला जा रहा है।
setup-verify-wallet-waiting = सत्यापन का इंतज़ार है
setup-verify-rpc-testing = Solana मेननेट टेस्ट हो रहा है
setup-verify-rpc-testing-detail =
    { $count ->
        [one] { $count } एंडपॉइंट जांचा जा रहा है।
       *[other] { $count } एंडपॉइंट जांचे जा रहे हैं।
    }
setup-verify-rpc-waiting = एंडपॉइंट टेस्ट का इंतज़ार है
setup-verify-save-waiting = सहेजने का इंतज़ार है
setup-verify-save-running = एन्क्रिप्ट और सेव हो रहा है
setup-verify-save-running-detail = सत्यापित कॉन्फ़िगरेशन इस डिवाइस पर लिखा जा रहा है।
setup-verify-save-done = कॉन्फ़िगरेशन सहेजा गया
setup-verify-save-done-detail = प्राइवेट की एन्क्रिप्ट हुई; काम करने वाले RPC एंडपॉइंट सहेजे गए।
setup-verify-save-failed = सेटअप सहेजा नहीं जा सका
setup-verify-save-skipped = सहेजा नहीं गया
setup-verify-request-failed = सत्यापन रिक्वेस्ट विफल
setup-verify-summary-checking = आपके वॉलेट और Solana मेननेट कनेक्शन जांचे जा रहे हैं।
setup-verify-summary-running = आपके दर्ज किए गए क्रेडेंशियल ठीक वैसे ही सत्यापित हो रहे हैं।
setup-verify-summary-saving = क्रेडेंशियल सत्यापित। सुरक्षित रूप से सहेजा जा रहा है।
setup-verify-summary-failed = समस्या देखें, फिर दोबारा सत्यापित करें।

setup-error-credentials-failed = क्रेडेंशियल सत्यापन विफल रहा।
setup-error-save-failed = सेटअप सहेजा नहीं जा सका।
setup-error-verify-failed = सत्यापन विफल रहा।
setup-error-explore-failed = एक्सप्लोर मोड शुरू नहीं हो सका।
setup-error-gateway-failed = गेटवे प्राथमिकता सहेजी नहीं जा सकी।
setup-action-review-credentials = क्रेडेंशियल देखें

setup-explore-opening = एक्सप्लोर मोड खुल रहा है…
setup-complete-restarting = आपके सत्यापित कॉन्फ़िगरेशन के साथ { -brand } रीस्टार्ट हो रहा है।
setup-complete-finishing = रीस्टार्ट पूरा हो रहा है…
setup-complete-ready = { -brand } तैयार है। डैशबोर्ड खुल रहा है…
setup-complete-stored = आपका सत्यापित कॉन्फ़िगरेशन इस डिवाइस पर सुरक्षित रखा गया है।

setup-wallet-show-key = प्राइवेट की दिखाएं
setup-wallet-hide-key = प्राइवेट की छिपाएं
setup-wallet-copy =
    .aria-label = वॉलेट एड्रेस कॉपी करें
    .title = वॉलेट एड्रेस कॉपी करें
setup-wallet-copy-done =
    .aria-label = वॉलेट एड्रेस कॉपी हो गया
    .title = कॉपी हो गया
setup-wallet-copy-failed =
    .aria-label = वॉलेट एड्रेस कॉपी नहीं हो सका
    .title = कॉपी विफल

setup-dialog-title = वॉलेट और RPC सेट करें
setup-dialog-subtitle = ट्रेडिंग और लाइव ऑन-चेन डेटा चालू करने के लिए अपना Solana वॉलेट और एक प्रीमियम RPC एंडपॉइंट कनेक्ट करें। आपकी प्राइवेट की इस डिवाइस पर एन्क्रिप्ट रहती है और कभी बाहर नहीं जाती।
setup-dialog-close =
    .title = बंद करें
    .aria-label = बंद करें
setup-dialog-wallet-label = वॉलेट प्राइवेट की
setup-dialog-wallet-input =
    .placeholder = Base58 स्ट्रिंग या JSON ऐरे [1,2,3,...]
setup-dialog-rpc-label = RPC एंडपॉइंट
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (हर लाइन में एक)
setup-dialog-rpc-hint = प्रीमियम प्रोवाइडर ({ -helius }, { -quicknode }, { -alchemy }) की पुरज़ोर सलाह है — पब्लिक Solana RPC पर रेट लिमिट होती है और वह काम न करे, ऐसा हो सकता है।
setup-dialog-submit = सत्यापित करें और कनेक्ट करें
setup-dialog-working = काम जारी है…
setup-dialog-validating = सत्यापित हो रहा है…
setup-dialog-saving = सहेज रहे हैं…
setup-dialog-restarting = रीस्टार्ट हो रहा है…
setup-dialog-saved = सेटअप सहेजा गया — { -brand } फ़ुल मोड में रीस्टार्ट हो रहा है…
setup-dialog-error-missing-fields = वॉलेट प्राइवेट की और कम से कम एक RPC URL दोनों दर्ज करें।
setup-dialog-error-validation = सत्यापन विफल रहा।
setup-dialog-error-incomplete = सेटअप पूरा नहीं हो सका।
setup-dialog-error-restart-helper = ऑटोमैटिक रीस्टार्ट हेल्पर उपलब्ध नहीं है। थोड़ी देर में डैशबोर्ड रीलोड करें।
setup-dialog-error-unexpected = अप्रत्याशित एरर।

setup-wizard-progress =
    .aria-label = सेटअप प्रगति
setup-wizard-step-credentials = क्रेडेंशियल
setup-wizard-step-verification = सत्यापन
setup-wizard-step-complete = पूर्ण
setup-wizard-credentials-title = क्रेडेंशियल कॉन्फ़िगर करें
setup-wizard-credentials-description = एक लोकल वॉलेट और भरोसेमंद Solana मेननेट RPC एंडपॉइंट कनेक्ट करें।
setup-wizard-wallet-toggle =
    .title = प्राइवेट की दिखाएं
    .aria-label = प्राइवेट की दिखाएं
setup-wizard-wallet-security-note = सहेजने से पहले एन्क्रिप्ट की जाती है।
setup-wizard-rpc-title = RPC एंडपॉइंट
setup-wizard-rpc-input =
    .placeholder = हर लाइन में एक HTTPS URL
setup-wizard-rpc-guidance = लगातार पोलिंग के लिए भरोसेमंद मेननेट RPC की सलाह है।
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = अनुशंसित
setup-wizard-gateway-title = मुफ़्त ट्रांज़ैक्शन भेजना
setup-wizard-gateway-hint = साइन इन होने पर उपलब्ध। आपका RPC फ़ॉलबैक के रूप में उपलब्ध रहता है।
setup-wizard-account-title = { -brand } अकाउंट
setup-wizard-account-optional = वैकल्पिक
setup-wizard-account-loading = अकाउंट स्टेटस जांचा जा रहा है…
setup-wizard-verify-title = सत्यापित करें और सहेजें
setup-wizard-verify-list =
    .aria-label = सेटअप सत्यापन स्टेटस
setup-wizard-verify-wallet = वॉलेट
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = सुरक्षित कॉन्फ़िगरेशन
setup-wizard-complete-title = सेटअप सहेजा गया
setup-wizard-reconnect = कनेक्शन दोबारा आज़माएं
setup-wizard-reload = डैशबोर्ड रीलोड करें
setup-wizard-error-title = सेटअप पर ध्यान चाहिए
setup-wizard-explore = डैशबोर्ड एक्सप्लोर करें
setup-wizard-continue = जारी रखें
