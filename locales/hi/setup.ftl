setup-wallet-required = वॉलेट की प्राइवेट की दर्ज करें।
setup-wallet-json-recognized = 64-बाइट JSON की फ़ॉर्मैट पहचाना गया।
setup-wallet-json-invalid = ठीक 64 बाइट मान (0–255) वाला JSON ऐरे इस्तेमाल करें।
setup-wallet-format-invalid = base58 प्राइवेट की या 64-बाइट JSON ऐरे इस्तेमाल करें।
setup-wallet-base58-recognized = Base58 की फ़ॉर्मैट पहचाना गया।

setup-rpc-required = कम से कम एक RPC एंडपॉइंट दर्ज करें।
setup-dialog-rpc-optional-note = वैकल्पिक: पहले से कॉन्फ़िगर किए गए RPC एंडपॉइंट रखने के लिए फ़ील्ड खाली छोड़ें।
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
setup-error-restart-timeout = { -brand } अभी दोबारा कनेक्ट नहीं हुआ है।

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
setup-verify-save-skipped = सहेजा नहीं गया। ऊपर की समस्या ठीक करें, फिर दोबारा सत्यापित करें।
setup-verify-request-failed = सत्यापन रिक्वेस्ट विफल
setup-verify-summary-checking = आपके वॉलेट और Solana मेननेट कनेक्शन जांचे जा रहे हैं।
setup-verify-summary-running = आपके दर्ज किए गए क्रेडेंशियल ठीक वैसे ही सत्यापित हो रहे हैं।
setup-verify-summary-saving = क्रेडेंशियल सत्यापित। सुरक्षित रूप से सहेजा जा रहा है।
setup-verify-summary-failed = समस्या देखें, फिर दोबारा सत्यापित करें।

setup-error-credentials-failed = क्रेडेंशियल सत्यापन विफल रहा।
setup-error-save-failed = सेटअप सहेजा नहीं जा सका।
setup-error-verify-failed = सत्यापन विफल रहा।
setup-error-explore-failed = एक्सप्लोर मोड शुरू नहीं हो सका।
setup-error-gateway-failed = ट्रांज़ैक्शन भेजने की सेटिंग सहेजी नहीं जा सकी।
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
setup-dialog-submit = सत्यापित करें और कनेक्ट करें
setup-dialog-working = काम जारी है…
setup-dialog-validating = सत्यापित हो रहा है…
setup-dialog-saving = सहेज रहे हैं…
setup-dialog-restarting = रीस्टार्ट हो रहा है…
setup-dialog-saved = सेटअप सहेजा गया — { -brand } फ़ुल मोड में रीस्टार्ट हो रहा है…
setup-dialog-error-validation = सत्यापन विफल रहा।
setup-dialog-error-incomplete = सेटअप पूरा नहीं हो सका।
setup-dialog-error-restart-helper = ऑटोमैटिक रीस्टार्ट हेल्पर उपलब्ध नहीं है। थोड़ी देर में डैशबोर्ड रीलोड करें।
setup-dialog-error-unexpected = अप्रत्याशित एरर।
setup-dialog-replace-title = ट्रेडिंग वॉलेट बदलें
setup-dialog-replace-subtitle = किसी दूसरे वॉलेट से ट्रेड करें। उसकी प्राइवेट की इसी डिवाइस पर एन्क्रिप्ट होती है और कभी बाहर नहीं जाती।
setup-dialog-replace-submit = सत्यापित करें और बदलें
setup-dialog-replace-saved = बदलाव तैयार — { -brand } नए वॉलेट के साथ रीस्टार्ट हो रहा है…
setup-dialog-replace-confirm-intro = ट्रेडिंग वॉलेट बदलने पर:
setup-dialog-replace-confirm-stop = ट्रेडर और कॉपी ट्रेडिंग रुक जाते हैं। रीस्टार्ट के बाद उन्हें फिर से चालू करें।
setup-dialog-replace-confirm-backup = मौजूदा वॉलेट की पोज़िशन, ट्रांज़ैक्शन और हिस्ट्री एक बैकअप फ़ोल्डर में चली जाती हैं।
setup-dialog-replace-confirm-keep = मौजूदा वॉलेट की की अन्य वॉलेट में रहती है, जहाँ से आप उसे अब भी एक्सपोर्ट कर सकते हैं।
setup-dialog-replace-confirm-funds = कोई फ़ंड नहीं हिलता: टोकन और { -sol } मौजूदा वॉलेट में ही रहते हैं।
setup-dialog-replace-confirm-rpc = कॉन्फ़िगर किए गए RPC एंडपॉइंट को आपके दर्ज किए गए एंडपॉइंट से बदलता है; फ़ील्ड खाली रहने पर मौजूदा एंडपॉइंट बने रहते हैं।
setup-dialog-replace-confirm-restart = { -brand } नए वॉलेट के साथ रीस्टार्ट होता है।

setup-wizard-progress =
    .aria-label = सेटअप प्रगति
setup-wizard-wallet-security-note = सहेजने से पहले एन्क्रिप्ट की जाती है।
setup-wizard-rpc-title = RPC एंडपॉइंट
setup-wizard-rpc-input =
    .placeholder = हर लाइन में एक HTTPS URL
setup-wizard-rpc-guidance = भरोसेमंद मेननेट प्रोवाइडर इस्तेमाल करें; सार्वजनिक एंडपॉइंट समर्थित नहीं हैं।
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = अनुशंसित
setup-wizard-account-title = { -brand } अकाउंट
setup-wizard-account-optional = वैकल्पिक
setup-wizard-account-loading = अकाउंट स्टेटस जांचा जा रहा है…
setup-wizard-verify-title = सत्यापित करें और सहेजें
setup-wizard-verify-list =
    .aria-label = सेटअप सत्यापन स्टेटस
setup-wizard-verify-wallet = वॉलेट
setup-wizard-verify-save = सुरक्षित कॉन्फ़िगरेशन
setup-wizard-complete-title = सेटअप सहेजा गया
setup-wizard-reconnect = कनेक्शन दोबारा आज़माएं
setup-wizard-reload = डैशबोर्ड रीलोड करें
setup-wizard-error-title = सेटअप पर ध्यान चाहिए
setup-wizard-explore = डैशबोर्ड एक्सप्लोर करें
setup-wizard-continue = जारी रखें

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = वॉलेट
setup-wizard-step-connection = कनेक्शन
setup-wizard-step-verify = जाँच
setup-wizard-step-ready = तैयार
setup-wallet-title = अपना ट्रेडिंग वॉलेट सेट करें
setup-wallet-description = वह वॉलेट जिससे { -brand } ट्रेड करता है। इसकी की इसी कंप्यूटर पर एन्क्रिप्ट होती है और कभी बाहर नहीं जाती।
setup-wallet-source =
    .aria-label = वॉलेट का स्रोत
setup-wallet-create-title = नया वॉलेट बनाएँ
setup-wallet-create-detail = { -brand } एक नया वॉलेट बनाता है। आप उसकी की सहेजते हैं, फिर उसमें { -sol } भेजते हैं।
setup-wallet-import-title = वॉलेट इम्पोर्ट करें
setup-wallet-import-detail = अपनी मौजूदा प्राइवेट की इस्तेमाल करें (base58 या 64 संख्याओं वाली JSON array)।
setup-wallet-creating = आपका वॉलेट बन रहा है…
setup-wallet-create-failed = वॉलेट नहीं बन सका। फिर से कोशिश करें।
setup-backup-title = अपनी प्राइवेट की सहेजें
setup-backup-description = जिसके पास यह की है, वही वॉलेट नियंत्रित करता है। { -brand } इसे वापस नहीं ला सकता।
setup-backup-address = पता
setup-backup-key = प्राइवेट की
setup-backup-copy-address = पता कॉपी करें
setup-backup-download = डाउनलोड करें
setup-backup-copied = कॉपी हो गया। सहेजने के बाद अपना क्लिपबोर्ड साफ़ करें।
setup-backup-machine-warning = { -brand } जो एन्क्रिप्टेड कॉपी रखता है, वह केवल इसी कंप्यूटर पर खुलती है। कंप्यूटर खो जाए या बदल जाए, तो सिर्फ़ वही की वॉलेट वापस ला सकती है जिसे आप अभी सहेजते हैं। इसे ऑफ़लाइन रखें, जैसे कागज़ पर या पासवर्ड मैनेजर में।
setup-backup-ack-saved = मैंने प्राइवेट की को इस कंप्यूटर के बाहर किसी सुरक्षित जगह सहेज लिया है।
setup-backup-ack-control = मैं समझता हूँ कि इस की वाला कोई भी व्यक्ति इस वॉलेट के फ़ंड ले सकता है।
setup-backup-ack-recovery = मैं समझता हूँ कि { -brand } यह की वापस नहीं ला सकता। इसके बिना, कंप्यूटर खोने का मतलब फ़ंड खोना है।
setup-backup-create-another = दूसरा बनाएँ
setup-backup-replace-confirm = यह की छोड़कर नया वॉलेट बनाएँ? ऐसा तभी करें जब आपने इसमें अभी तक कुछ नहीं भेजा हो।
setup-connection-title = { -brand } { -solana } से कैसे जुड़े?
setup-connection-description = आप इसे बाद में सेटिंग्स → कनेक्शन में बदल सकते हैं।
setup-connection-choice =
    .aria-label = कनेक्शन
setup-connection-service-title = { -brand } सेवा
setup-connection-service-badge = RPC की ज़रूरत नहीं
setup-connection-service-detail = आपका मुफ़्त { -brand } अकाउंट इस्तेमाल होता है।
setup-connection-service-prices = कीमतें हर कुछ सेकंड में ताज़ा होती हैं
setup-connection-service-features = कॉपी ट्रेडिंग, वॉलेट वॉच, दूसरे वॉलेट के बैलेंस, पुराना इतिहास, डायरेक्ट पूल स्वैप और लाइव पूल कीमतों के लिए आपका अपना RPC चाहिए
setup-connection-service-pause = साइन आउट करने या screenerbot.io तक न पहुँच पाने पर ट्रेडिंग रुक जाती है
setup-connection-service-privacy = आपकी प्राइवेट की कभी इस कंप्यूटर से बाहर नहीं जाती: स्वैप यहीं साइन होते हैं, और screenerbot.io बैलेंस और ट्रांज़ैक्शन पढ़ने के लिए सिर्फ़ आपके वॉलेट का सार्वजनिक पता इस्तेमाल करता है
setup-connection-own-title = आपका अपना RPC
setup-connection-own-badge = सबसे तेज़
setup-connection-own-detail = कीमतें लगभग हर सेकंड दो बार और सभी फ़ीचर। किसी प्रोवाइडर का RPC एंडपॉइंट चाहिए।
setup-connection-signin-required = { -brand } सेवा इस्तेमाल करने के लिए साइन इन करें या मुफ़्त अकाउंट बनाएँ।
setup-connection-reauthorize = { -brand } सेवा को वॉलेट रीड की अनुमति देने के लिए फिर से साइन इन करें।
setup-connection-sending = ट्रांज़ैक्शन { -brand } के ज़रिए भेजें
setup-connection-sending-hint = साइन इन होने पर उपलब्ध। जब यह संभव न हो, आपका RPC इस्तेमाल होता है।
setup-wizard-account-required = ज़रूरी
setup-wizard-verify-connection = कनेक्शन
setup-verify-service-checking = { -brand } सेवा जाँची जा रही है
setup-verify-service-ready = { -brand } सेवा तैयार है
setup-verify-service-busy = { -brand } सेवा अभी व्यस्त है। थोड़ी देर में फिर कोशिश करें, या अपना RPC इस्तेमाल करें।
setup-verify-service-daily-limit = आपके अकाउंट ने { -brand } सेवा की आज की सीमा पूरी कर ली है। 00:00 UTC के बाद फिर कोशिश करें, या अपना RPC इस्तेमाल करें।
setup-verify-service-unreachable = screenerbot.io तक नहीं पहुँचा जा सकता। अपना इंटरनेट कनेक्शन जाँचें।
setup-verify-service-version = { -brand } सेवा अब इस वर्ज़न को सपोर्ट नहीं करती। जारी रखने के लिए { -brand } अपडेट करें।
setup-verify-service-signin = { -brand } सेवा इस्तेमाल करने के लिए साइन इन करें।
setup-verify-save-done-service-detail = प्राइवेट की एन्क्रिप्ट हो गई; { -brand } सेवा चुनी गई।
setup-action-try-again = फिर कोशिश करें
setup-action-use-own-rpc = मेरा अपना RPC इस्तेमाल करें
setup-action-sign-in-again = फिर से साइन इन करें
setup-ready-title = { -brand } तैयार है
setup-fund-title = अपने ट्रेडिंग वॉलेट में फ़ंड डालें
setup-fund-waiting = { -sol } का इंतज़ार…
setup-fund-received = { $amount } { -sol } मिले
setup-fund-balance-unavailable = बैलेंस पढ़ा जा रहा है… यह अपने आप अपडेट होगा।
setup-fund-open-dashboard = डैशबोर्ड खोलें
