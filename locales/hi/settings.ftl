# The Settings dialog. Each section is named after the script that owns it.

settings-duration-minutes =
    { $count ->
        [one] { $count } मिनट
       *[other] { $count } मिनट
    }
settings-duration-hours =
    { $count ->
        [one] { $count } घंटा
       *[other] { $count } घंटे
    }

settings-dialog-title = सेटिंग्स
settings-dialog-close =
    .title = बंद करें (ESC)
    .aria-label = सेटिंग्स बंद करें
settings-dialog-save = बदलाव सहेजें
settings-dialog-saving = सहेजा जा रहा है...
settings-dialog-saved = सहेजा गया
settings-dialog-save-success = सेटिंग्स सफलतापूर्वक सहेजी गईं
settings-dialog-save-failed = सेटिंग्स सहेजना विफल
settings-dialog-update-attention = अपडेट पर ध्यान देना ज़रूरी है
settings-dialog-tab-interface = इंटरफ़ेस
settings-dialog-tab-navigation = नेविगेशन
settings-dialog-tab-startup = स्टार्टअप
settings-dialog-tab-hints = हिंट्स
settings-dialog-tab-data = डेटा
settings-dialog-tab-security = सुरक्षा
settings-dialog-tab-account = अकाउंट
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = एजेंट कनेक्शन
settings-dialog-tab-updates = अपडेट
settings-dialog-tab-licenses = लाइसेंस
settings-dialog-tab-about = परिचय
settings-dialog-link-privacy = गोपनीयता नीति
settings-dialog-link-terms = सेवा की शर्तें

settings-startup-section-title = स्टार्टअप व्यवहार
settings-startup-auto-start-label = ट्रेडर ऑटो-स्टार्ट
settings-startup-auto-start-hint = लॉन्च होते ही ट्रेडर अपने आप शुरू करें
settings-startup-coming-soon = जल्द आ रहा है
settings-startup-default-page-label = डिफ़ॉल्ट पेज
settings-startup-default-page-hint = ऐप खोलने पर दिखने वाला पेज
settings-startup-page-dashboard = डैशबोर्ड
settings-startup-page-tokens = टोकन
settings-startup-page-positions = पोज़िशन
settings-startup-page-wallet = वॉलेट
settings-startup-page-config = कॉन्फ़िग
settings-startup-notifications-label = बैकग्राउंड नोटिफ़िकेशन दिखाएँ
settings-startup-notifications-hint = बैकग्राउंड इवेंट के नोटिफ़िकेशन दिखाएँ

settings-about-tagline = नेटिव Solana ट्रेडिंग इंजन
settings-about-link-github = { -github }
settings-about-link-docs = दस्तावेज़ीकरण
settings-about-link-telegram = { -telegram }
settings-about-link-website = वेबसाइट
settings-about-credits = Solana ट्रेडर्स के लिए बनाया गया
settings-about-copyright = © { $year } { -brand }। सर्वाधिकार सुरक्षित।

settings-interface-section-appearance = रूप-रंग
settings-interface-theme-label = थीम
settings-interface-theme-hint = अपनी पसंदीदा रंग योजना चुनें
settings-interface-theme-dark = डार्क
settings-interface-theme-light = लाइट
settings-interface-language-label = भाषा
settings-interface-language-hint = डैशबोर्ड की प्रदर्शन भाषा
settings-interface-logo-shape-label = टोकन लोगो का आकार
settings-interface-logo-shape-hint = गोल हर लोगो को गोलाकार काटता है; प्राकृतिक हर आर्टवर्क की अपनी आकृति बनाए रखता है
settings-interface-logo-shape-circle = गोल
settings-interface-logo-shape-natural = प्राकृतिक
settings-interface-animations-label = एनिमेशन चालू करें
settings-interface-animations-hint = सहज ट्रांज़िशन और इफ़ेक्ट
settings-interface-compact-label = कॉम्पैक्ट मोड
settings-interface-compact-hint = ज़्यादा सामग्री के लिए पैडिंग घटाएँ
settings-interface-section-data = डेटा और डिस्प्ले
settings-interface-refresh-label = रीफ़्रेश अंतराल
settings-interface-refresh-hint = डेटा कितनी बार रीफ़्रेश हो
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } सेकंड
       *[other] { $count } सेकंड
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } मिनट
       *[other] { $count } मिनट
    }
settings-interface-ticker-label = टिकर बार दिखाएँ
settings-interface-ticker-hint = हेडर में लाइव मेट्रिक्स टिकर
settings-interface-page-size-label = टेबल पेज साइज़
settings-interface-page-size-hint = प्रति टेबल पेज डिफ़ॉल्ट पंक्तियाँ
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } पंक्ति
       *[other] { $count } पंक्तियाँ
    }
settings-interface-auto-expand-label = श्रेणियाँ अपने आप खोलें
settings-interface-auto-expand-hint = कॉन्फ़िग श्रेणियाँ डिफ़ॉल्ट रूप से खोलें
settings-interface-hints-label = संदर्भ हिंट दिखाएँ
settings-interface-hints-hint = डैशबोर्ड की सुविधाएँ समझाने वाले हेल्प आइकन दिखाएँ
settings-interface-featured-label = फ़ीचर्ड पंक्ति दिखाएँ
settings-interface-featured-hint = होम और टोकन पेज पर फ़ीचर्ड टोकन की पंक्ति दिखाएँ
settings-interface-section-sound = साउंड इफ़ेक्ट
settings-interface-sounds-label = साउंड चालू करें
settings-interface-sounds-hint = नेविगेशन, स्थिति बदलाव और नतीजों के लिए स्पर्श-आधारित संकेत

settings-security-loading = सुरक्षा सेटिंग्स लोड हो रही हैं...
settings-security-load-failed = सुरक्षा सेटिंग्स लोड करना विफल

settings-security-type-pin4 = 4 अंकों का PIN
settings-security-type-pin6 = 6 अंकों का PIN
settings-security-type-text = टेक्स्ट पासवर्ड
settings-security-type-unset = सेट नहीं है

settings-security-lockscreen-title = डैशबोर्ड लॉकस्क्रीन
settings-security-lockscreen-description = अपने डैशबोर्ड को PIN या पासवर्ड से सुरक्षित करें। ट्रिगर होने पर लॉकस्क्रीन दिखेगी और आगे बढ़ने के लिए प्रमाणीकरण ज़रूरी होगा।
settings-security-enable-label = लॉकस्क्रीन चालू करें
settings-security-enable-hint = पासवर्ड प्रमाणीकरण से अपने डैशबोर्ड को सुरक्षित करें
settings-security-password-status-label = पासवर्ड की स्थिति
settings-security-password-current = वर्तमान: { $type }
settings-security-password-none = कोई पासवर्ड सेट नहीं है
settings-security-change = बदलें
settings-security-remove = हटाएँ
settings-security-set-password = पासवर्ड सेट करें
settings-security-auto-lock-label = निष्क्रियता के बाद ऑटो-लॉक
settings-security-auto-lock-hint = कुछ समय गतिविधि न होने पर अपने आप लॉक करें
settings-security-auto-lock-never = कभी नहीं
settings-security-lock-blur-label = विंडो फ़ोकस खोने पर लॉक करें
settings-security-lock-blur-hint = दूसरे ऐप पर जाते ही अपने आप लॉक करें
settings-security-quick-actions-title = क्विक एक्शन
settings-security-lock-now-label = डैशबोर्ड अभी लॉक करें
settings-security-lock-now-hint = डैशबोर्ड को तुरंत लॉक करें
settings-security-lock-now = अभी लॉक करें
settings-security-lock-not-ready = लॉक नहीं हो सकता - लॉकस्क्रीन तैयार नहीं है
settings-security-setting-save-failed = सुरक्षा सेटिंग सहेजी नहीं जा सकी

settings-security-2fa-title = टू-फ़ैक्टर ऑथेंटिकेशन
settings-security-2fa-description = ऑथेंटिकेटर ऐप (Google Authenticator, Authy आदि) से सुरक्षा की एक अतिरिक्त परत जोड़ें
settings-security-2fa-status-label = 2FA की स्थिति
settings-security-2fa-status-enabled = टू-फ़ैक्टर ऑथेंटिकेशन चालू है
settings-security-2fa-status-none = कॉन्फ़िगर नहीं है
settings-security-2fa-disable = 2FA बंद करें
settings-security-2fa-enable = 2FA चालू करें

settings-security-modal-close =
    .aria-label = बंद करें
settings-security-password-set-title = पासवर्ड सेट करें
settings-security-password-change-title = पासवर्ड बदलें
settings-security-password-current-label = वर्तमान पासवर्ड
settings-security-password-current-input =
    .placeholder = वर्तमान पासवर्ड दर्ज करें
settings-security-password-type-label = पासवर्ड का प्रकार
settings-security-password-new-label = नया पासवर्ड
settings-security-password-new-input =
    .placeholder = नया पासवर्ड दर्ज करें
settings-security-password-confirm-label = पासवर्ड की पुष्टि करें
settings-security-password-confirm-input =
    .placeholder = पासवर्ड की पुष्टि करें
settings-security-password-update = पासवर्ड अपडेट करें
settings-security-placeholder-pin4 = 4 अंकों का PIN दर्ज करें
settings-security-placeholder-pin6 = 6 अंकों का PIN दर्ज करें
settings-security-placeholder-text = पासवर्ड दर्ज करें
settings-security-password-required = कृपया पासवर्ड दर्ज करें
settings-security-password-mismatch = पासवर्ड मेल नहीं खाते
settings-security-pin4-invalid = PIN ठीक 4 अंकों का होना चाहिए
settings-security-pin6-invalid = PIN ठीक 6 अंकों का होना चाहिए
settings-security-text-too-short = पासवर्ड कम से कम 4 अक्षरों का होना चाहिए
settings-security-password-saved = पासवर्ड सहेजा गया
settings-security-password-save-failed = पासवर्ड सहेजना विफल
settings-security-password-save-failed-detail = पासवर्ड सहेजना विफल: { $message }

settings-security-remove-title = पासवर्ड हटाएँ
settings-security-remove-description = लॉकस्क्रीन सुरक्षा हटाने के लिए अपना वर्तमान पासवर्ड दर्ज करें।
settings-security-remove-confirm = पासवर्ड हटाएँ
settings-security-current-required = कृपया अपना वर्तमान पासवर्ड दर्ज करें
settings-security-password-removed = पासवर्ड हटाया गया
settings-security-password-remove-failed = पासवर्ड हटाना विफल
settings-security-password-remove-failed-detail = पासवर्ड हटाना विफल: { $message }

settings-security-2fa-enable-title = टू-फ़ैक्टर ऑथेंटिकेशन चालू करें
settings-security-2fa-password-prompt = जारी रखने के लिए अपना पासवर्ड दर्ज करें:
settings-security-2fa-password-input =
    .placeholder = पासवर्ड दर्ज करें
settings-security-2fa-continue = जारी रखें
settings-security-2fa-manual-code = मैन्युअल एंट्री कोड:
settings-security-2fa-qr =
    .alt = TOTP QR कोड
settings-security-2fa-code-prompt = अपने ऑथेंटिकेटर ऐप से 6 अंकों का कोड दर्ज करें:
settings-security-2fa-verify-enable = सत्यापित करें और चालू करें
settings-security-2fa-password-required = कृपया अपना पासवर्ड दर्ज करें
settings-security-2fa-setup-failed = 2FA सेटअप विफल
settings-security-2fa-code-invalid-length = कृपया 6 अंकों का कोड दर्ज करें
settings-security-2fa-code-invalid = अमान्य कोड
settings-security-2fa-enabled = टू-फ़ैक्टर ऑथेंटिकेशन चालू हो गया
settings-security-2fa-verify-failed = कोड सत्यापित करना विफल
settings-security-2fa-disable-title = टू-फ़ैक्टर ऑथेंटिकेशन बंद करें
settings-security-2fa-disable-prompt = 2FA बंद करने के लिए अपना पासवर्ड दर्ज करें:
settings-security-2fa-disable-failed = 2FA बंद करना विफल
settings-security-2fa-disabled = टू-फ़ैक्टर ऑथेंटिकेशन बंद हो गया

settings-agent-category-analysis = विश्लेषण
settings-agent-category-portfolio = पोर्टफ़ोलियो
settings-agent-category-trading = ट्रेडिंग
settings-agent-category-config = कॉन्फ़िगरेशन
settings-agent-category-system = सिस्टम
settings-agent-category-analysis-description = टोकन विश्लेषण, मार्केट डेटा और सुरक्षा जाँच।
settings-agent-category-portfolio-description = खुली पोज़िशन, बैलेंस और लाभ-हानि।
settings-agent-category-trading-description = असली फ़ंड से पोज़िशन खरीदना, बेचना और बंद करना।
settings-agent-category-config-description = RPC एंडपॉइंट सहित बॉट की हर सेटिंग। वॉलेट कीज़ कभी नहीं।
settings-agent-category-system-description = स्टेटस, इवेंट और इमरजेंसी स्टॉप।
settings-agent-category-analysis-inline = विश्लेषण
settings-agent-category-portfolio-inline = पोर्टफ़ोलियो
settings-agent-category-trading-inline = ट्रेडिंग
settings-agent-category-config-inline = कॉन्फ़िगरेशन
settings-agent-category-system-inline = सिस्टम

settings-agent-level-allow = अनुमति दें
settings-agent-level-ask-user = पूछें
settings-agent-level-deny = बंद
settings-agent-level-allow-hint = तुरंत चलता है।
settings-agent-level-ask-user-hint = ऐप में आपकी मंज़ूरी की प्रतीक्षा करता है।
settings-agent-level-deny-hint = अस्वीकृत, और एजेंट से छिपा रहता है।

settings-agent-preset-full = पूर्ण एक्सेस
settings-agent-preset-ask = पहले पूछें
settings-agent-preset-read = केवल पढ़ना
settings-agent-preset-full-description = सब कुछ बिना पूछे चलता है। वॉलेट कीज़ पहुँच से बाहर रहती हैं।
settings-agent-preset-ask-description = हर कार्रवाई ऐप में आपकी मंज़ूरी का इंतज़ार करती है।
settings-agent-preset-read-description = विश्लेषण और पोर्टफ़ोलियो पढ़ना। कुछ भी बदला नहीं जा सकता।
settings-agent-preset-custom = कस्टम
settings-agent-preset-group =
    .aria-label = अनुमति प्रीसेट
settings-agent-permission-group = अनुमति: { $category }

settings-agent-summary-asks-only = सीमित — इनके लिए पूछता है: { $asking }
settings-agent-summary-off-only = सीमित — इनके लिए बंद: { $off }
settings-agent-summary-asks-and-off = सीमित — पूछता है: { $asking }; बंद: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = जेनेरिक stdio MCP

settings-agent-note-placeholder = /absolute/path/to/screenerbot को अपने { -brand } बाइनरी के पूर्ण पथ से बदलें — चालू ऐप इस सिस्टम पर अपनी एक्ज़ीक्यूटेबल का पथ नहीं बता सका।
settings-agent-note-data-dir = यदि आप { -brand } को गैर-डिफ़ॉल्ट डेटा डायरेक्टरी के साथ चलाते हैं, तो क्लाइंट पर भी SCREENERBOT_DATA_DIR को उसी पथ पर सेट करें (एक और -e / --env फ़्लैग, या env एंट्री)।
settings-agent-note-codex-run = कमांड चलाएँ, या TOML ब्लॉक को ~/.codex/config.toml ($CODEX_HOME/config.toml) में जोड़ें। इसके बाद { -codex } रीस्टार्ट करें।
settings-agent-note-codex-get = `codex mcp get screenerbot` अपने आउटपुट में सीक्रेट को मास्क कर देता है।
settings-agent-note-claude-code = { -claude } Code: कमांड चलाएँ, फिर { -claude } Code रीस्टार्ट करें। `claude mcp get screenerbot` सीक्रेट सहित कॉन्फ़िगर किया गया एनवायरनमेंट प्रिंट करेगा।
settings-agent-note-claude-desktop = { -claude } Desktop: JSON को claude_desktop_config.json में `mcpServers` के अंतर्गत मर्ज करें और ऐप रीस्टार्ट करें।
settings-agent-note-openclaw = कमांड चलाएँ, फिर `openclaw mcp doctor screenerbot --probe` से जाँचें कि सहेजा गया stdio सर्वर शुरू होता है और टूल्स उपलब्ध कराता है।
settings-agent-note-hermes = इसे { -hermes } की कॉन्फ़िगरेशन फ़ाइल में `mcp_servers` के अंतर्गत जोड़ें, फिर { -hermes } रीस्टार्ट करें।
settings-agent-note-generic = कोई भी MCP क्लाइंट जो stdio समझता है: क्लाइंट अपनी सर्वर सूची जहाँ भी रखता हो, वहाँ इन आर्ग्युमेंट और एनवायरनमेंट के साथ यह कमांड चलाएँ।
settings-agent-block-codex-command = { -codex } CLI — टर्मिनल कमांड
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (विकल्प)
settings-agent-block-claude-command = { -claude } Code — टर्मिनल कमांड
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — टर्मिनल कमांड
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = जेनेरिक stdio MCP क्लाइंट

settings-agent-name-required = इस कनेक्शन के लिए नाम दर्ज करें।
settings-agent-name-too-long = नाम अधिकतम { $max } अक्षरों का होना चाहिए।
settings-agent-name-control-characters = नाम में कंट्रोल कैरेक्टर नहीं होने चाहिए।

settings-agent-title = एजेंट कनेक्शन
settings-agent-description = { -claude }, { -codex }, { -hermes }, { -openclaw } या कोई भी stdio MCP क्लाइंट कनेक्ट करें। { -brand } का चालू रहना ज़रूरी है। हर कनेक्शन की अपनी अनुमतियाँ होती हैं: डिफ़ॉल्ट रूप से पूर्ण एक्सेस, और जब चाहें हर कनेक्शन के लिए सीमित। कोई भी कनेक्शन आपकी वॉलेट की को कभी पढ़ या बदल नहीं सकता।
settings-agent-name-label = कनेक्शन का नाम
settings-agent-name-hint = नीचे सूची में दिखता है ताकि आप कनेक्शनों में अंतर कर सकें।
settings-agent-name-input =
    .placeholder = लैपटॉप कोडिंग एजेंट
settings-agent-client-label = क्लाइंट
settings-agent-client-hint = कनेक्शन बनने के बाद दिखने वाला सेटअप चुनता है।
settings-agent-permissions-label = अनुमतियाँ
settings-agent-permissions-hint = नया कनेक्शन सब कुछ कर सकता है। किसी भी श्रेणी को अभी या बाद में नीचे की सूची से सीमित करें — वॉलेट कीज़ दोनों स्थितियों में पहुँच से बाहर रहती हैं।
settings-agent-create = कनेक्शन बनाएँ
settings-agent-issued-group =
    .aria-label = नए कनेक्शन का क्रेडेंशियल
settings-agent-issued-warning = सीक्रेट अभी कॉपी कर लें। यह केवल एक बार दिखता है और दोबारा नहीं मिल सकता — खो जाए तो कनेक्शन रद्द करके फिर से बनाएँ। { -brand } केवल एक-तरफ़ा वेरिफ़ायर रखता है; आपका MCP क्लाइंट प्लेनटेक्स्ट को अपनी कॉन्फ़िगरेशन में रखता है।
settings-agent-issued-client-id = क्लाइंट ID
settings-agent-issued-secret = एक-बार का सीक्रेट
settings-agent-setup-for = सेटअप:
settings-agent-done = पूर्ण
settings-agent-list-title = कनेक्शन
settings-agent-loading = कनेक्शन लोड हो रहे हैं...
settings-agent-active-count = { $count } सक्रिय
settings-agent-empty = अभी कोई कनेक्शन नहीं। क्लाइंट जोड़ने के लिए ऊपर एक बनाएँ।
settings-agent-empty-active = कोई सक्रिय कनेक्शन नहीं।
settings-agent-revoked-title = रद्द किए गए कनेक्शन
settings-agent-created = बनाया गया: { $time }
settings-agent-last-used = आखिरी उपयोग: { $time }
settings-agent-never-used = कभी उपयोग नहीं हुआ
settings-agent-permissions-edit = अनुमतियाँ
settings-agent-revoke = रद्द करें
settings-agent-permissions-save = अनुमतियाँ सहेजें

settings-agent-load-failed = एजेंट कनेक्शन लोड करना विफल
settings-agent-list-failed = कनेक्शन लोड नहीं हो सके
settings-agent-create-failed = कनेक्शन नहीं बनाया जा सका।
settings-agent-unreachable-create = कनेक्शन बनाने के लिए { -brand } तक पहुँचा नहीं जा सका।
settings-agent-permissions-update-failed = अनुमतियाँ अपडेट नहीं की जा सकीं
settings-agent-permissions-updated = अनुमतियाँ अपडेट की गईं
settings-agent-permissions-updated-detail = कनेक्शन के अगले अनुरोध से लागू होंगी।
settings-agent-unreachable-save = सहेजने के लिए { -brand } तक पहुँचा नहीं जा सका
settings-agent-revoke-title = कनेक्शन रद्द करें
settings-agent-revoke-message = "{ $label }" रद्द करें? क्लाइंट अपने अगले अनुरोध पर काम करना बंद कर देगा और इसे वापस नहीं लाया जा सकता।
settings-agent-revoke-fallback-name = यह कनेक्शन
settings-agent-revoke-failed = कनेक्शन रद्द नहीं किया जा सका
settings-agent-unreachable-revoke = रद्द करने के लिए { -brand } तक पहुँचा नहीं जा सका

settings-telegram-loading = { -telegram } सेटिंग्स लोड हो रही हैं...
settings-telegram-load-failed = { -telegram } सेटिंग्स लोड करना विफल
settings-telegram-unknown = अज्ञात
settings-telegram-session-active = सक्रिय: { $duration }
settings-telegram-sessions-empty = कोई सक्रिय सेशन नहीं
settings-telegram-session-revoke = रद्द करें

settings-telegram-connection-title = कनेक्शन
settings-telegram-connection-description = नोटिफ़िकेशन पाने और { -brand } को दूर से नियंत्रित करने के लिए अपना { -telegram } बॉट कनेक्ट करें।
settings-telegram-enable-label = { -telegram } चालू करें
settings-telegram-enable-hint = { -telegram } बॉट इंटीग्रेशन चालू करें
settings-telegram-token-label = बॉट टोकन
settings-telegram-token-saved = टोकन सहेजा गया
settings-telegram-token-help = इसे { -telegram } पर @BotFather से प्राप्त करें
settings-telegram-token-input-saved =
    .placeholder = टोकन सहेजा गया (बदलने के लिए नया दर्ज करें)
settings-telegram-token-input =
    .placeholder = बॉट टोकन दर्ज करें
settings-telegram-token-toggle =
    .title = दिखाएँ/छिपाएँ
settings-telegram-chat-label = चैट ID
settings-telegram-chat-connected = इस चैट से जुड़ा है:
settings-telegram-chat-discover-hint = अपनी चैट ID अपने आप खोजें
settings-telegram-chat-change =
    .title = बदलें
settings-telegram-chat-discover = चैट ID खोजें
settings-telegram-discovery-step-add = अपने बॉट को { -telegram } ग्रुप में जोड़ें, या उसके साथ सीधी चैट शुरू करें
settings-telegram-discovery-step-privacy = ग्रुप के लिए: @BotFather → /mybots → [आपका बॉट] → Bot Settings → Group Privacy जाँचें
settings-telegram-discovery-privacy = <strong>प्राइवेसी मोड बंद:</strong> बॉट को ग्रुप के सभी संदेश मिलते हैं<br/><strong>प्राइवेसी मोड चालू:</strong> बॉट को केवल @mention होने पर संदेश मिलते हैं
settings-telegram-discovery-step-send = कोई भी संदेश भेजें (या प्राइवेसी मोड चालू होने पर अपने बॉट को @mention करें)
settings-telegram-discovery-listening = संदेशों की प्रतीक्षा है...
settings-telegram-discovery-select = चुनें
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = संदेश की भाषा
settings-telegram-language-hint = { -telegram } बॉट के संदेशों और बटनों की भाषा
settings-telegram-language-follow-app = ऐप की भाषा अपनाएँ
settings-telegram-test-label = कनेक्शन टेस्ट करें
settings-telegram-test-hint = कॉन्फ़िगरेशन जाँचने के लिए टेस्ट संदेश भेजें
settings-telegram-test-send = टेस्ट भेजें
settings-telegram-test-sending = भेजा जा रहा है...

settings-telegram-chat-type-private = निजी
settings-telegram-chat-type-group = ग्रुप
settings-telegram-chat-type-supergroup = सुपरग्रुप
settings-telegram-chat-type-channel = चैनल

settings-telegram-auth-title = कमांड प्रमाणीकरण
settings-telegram-auth-description = { -telegram } कमांड वही 2FA इस्तेमाल करते हैं जो डैशबोर्ड लॉकस्क्रीन में है।
settings-telegram-auth-protected = सुरक्षित
settings-telegram-auth-disabled = बंद
settings-telegram-auth-not-configured = कॉन्फ़िगर नहीं है
settings-telegram-auth-error = त्रुटि
settings-telegram-auth-protected-note = कमांड लॉकस्क्रीन 2FA से सुरक्षित हैं। सेशन समाप्त होने पर उपयोगकर्ताओं को <code>/login</code> कमांड से अपना ऑथेंटिकेटर कोड देना होगा।
settings-telegram-auth-disabled-note = लॉकस्क्रीन 2FA कॉन्फ़िगर है, पर { -telegram } के लिए बंद है। { -telegram } कमांड सुरक्षित करने के लिए ऊपर "कमांड के लिए 2FA आवश्यक करें" चालू करें।
settings-telegram-auth-missing-note = लॉकस्क्रीन 2FA कॉन्फ़िगर नहीं है। 2FA के बिना, समाप्त सेशन बिना सत्यापन के अपने आप फिर सक्रिय हो जाएँगे।
settings-telegram-auth-managed-in = 2FA का प्रबंधन यहाँ होता है:
settings-telegram-auth-configure-in = 2FA यहाँ कॉन्फ़िगर करें:
settings-telegram-auth-configure-suffix = ताकि { -telegram } कमांड के लिए सत्यापन आवश्यक हो।
settings-telegram-security-link = सुरक्षा सेटिंग्स
settings-telegram-timeout-title = सेशन टाइमआउट
settings-telegram-timeout-description = प्रमाणित सेशन कितनी देर सक्रिय रहता है
settings-telegram-sessions-title = सक्रिय सेशन

settings-telegram-notifications-title = नोटिफ़िकेशन सेटिंग्स
settings-telegram-notifications-description = चुनें कि कौन-से इवेंट { -telegram } नोटिफ़िकेशन भेजें।
settings-telegram-notify-opened-label = पोज़िशन खुली
settings-telegram-notify-opened-hint = नई पोज़िशन खुलने पर सूचित करें
settings-telegram-notify-closed-label = पोज़िशन बंद हुई
settings-telegram-notify-closed-hint = पोज़िशन बंद होने पर सूचित करें
settings-telegram-notify-partial-label = आंशिक एग्ज़िट
settings-telegram-notify-partial-hint = पोज़िशन के आंशिक एग्ज़िट पर सूचित करें
settings-telegram-notify-dca-label = DCA निष्पादित
settings-telegram-notify-dca-hint = DCA ऑर्डर निष्पादित होने पर सूचित करें
settings-telegram-notify-errors-label = त्रुटियाँ
settings-telegram-notify-errors-hint = त्रुटियों और विफलताओं पर सूचित करें
settings-telegram-notify-startup-label = स्टार्टअप/शटडाउन
settings-telegram-notify-startup-hint = बॉट शुरू या बंद होने पर सूचित करें
settings-telegram-notify-filtering-label = फ़िल्टरिंग अलर्ट
settings-telegram-notify-filtering-hint = नए टोकन फ़िल्टरिंग मानदंड पास करने पर सूचित करें
settings-telegram-notify-trades-label = ट्रेड अलर्ट
settings-telegram-notify-trades-hint = वॉच किए गए टोकन के बड़े ट्रेड पर सूचित करें
settings-telegram-notify-daily-label = दैनिक सारांश
settings-telegram-notify-daily-hint = दैनिक ट्रेडिंग गतिविधि और लाभ-हानि का सारांश पाएँ

settings-telegram-features-title = सुविधाएँ
settings-telegram-features-description = { -telegram } बॉट की क्षमताएँ कॉन्फ़िगर करें।
settings-telegram-commands-label = कमांड चालू करें
settings-telegram-commands-hint = { -telegram } कमांड से बॉट को नियंत्रित करने दें
settings-telegram-require-2fa-label = कमांड के लिए 2FA आवश्यक करें
settings-telegram-require-2fa-hint = सेशन समाप्त होने पर फिर सक्रिय करने के लिए 2FA कोड आवश्यक करें। लॉकस्क्रीन 2FA का उपयोग होता है।
settings-telegram-inline-label = इनलाइन एक्शन बटन
settings-telegram-inline-hint = नोटिफ़िकेशन संदेशों में एक्शन बटन दिखाएँ

settings-telegram-setting-save-failed = { -telegram } सेटिंग सहेजी नहीं जा सकी
settings-telegram-discovery-start-failed = खोज शुरू नहीं हो सकी
settings-telegram-chat-selected = चैट चुनी गई
settings-telegram-chat-select-failed = चैट चुनी नहीं जा सकी
settings-telegram-test-sent = टेस्ट संदेश भेजा गया
settings-telegram-test-failed = टेस्ट संदेश विफल
settings-telegram-session-revoked = सेशन रद्द किया गया
settings-telegram-session-revoke-failed = सेशन रद्द नहीं हो सका

settings-licenses-title = ओपन सोर्स लाइसेंस
settings-licenses-subtitle = { -brand } निम्नलिखित ओपन सोर्स सॉफ़्टवेयर से बना है
settings-licenses-footer = पूरे लाइसेंस टेक्स्ट प्रोजेक्ट रिपॉज़िटरी और हर डिपेंडेंसी के सोर्स कोड में उपलब्ध हैं।
settings-licenses-category-framework = एप्लिकेशन फ़्रेमवर्क
settings-licenses-category-solana = Solana ब्लॉकचेन
settings-licenses-category-data = डेटा और स्टोरेज
settings-licenses-category-networking = नेटवर्किंग
settings-licenses-category-cryptography = क्रिप्टोग्राफ़ी और एन्कोडिंग
settings-licenses-category-assets = UI एसेट्स
settings-licenses-desc-electron = डेस्कटॉप एप्लिकेशन फ़्रेमवर्क
settings-licenses-desc-tokio = Rust के लिए एसिंक रनटाइम
settings-licenses-desc-axum = वेब सर्वर फ़्रेमवर्क
settings-licenses-desc-tower = सर्विस एब्स्ट्रैक्शन
settings-licenses-desc-hyper = HTTP इम्प्लीमेंटेशन
settings-licenses-desc-solana-sdk = Solana SDK कोर
settings-licenses-desc-solana-client = RPC क्लाइंट
settings-licenses-desc-solana-program = प्रोग्राम लाइब्रेरी
settings-licenses-desc-spl-token = SPL Token प्रोग्राम
settings-licenses-desc-spl-token-2022 = Token-2022 एक्सटेंशन
settings-licenses-desc-spl-associated-token-account = एसोसिएटेड टोकन अकाउंट
settings-licenses-desc-sqlite = एम्बेडेड डेटाबेस इंजन
settings-licenses-desc-rusqlite = SQLite के Rust बाइंडिंग
settings-licenses-desc-r2d2 = डेटाबेस कनेक्शन पूल
settings-licenses-desc-serde = सीरियलाइज़ेशन फ़्रेमवर्क
settings-licenses-desc-toml = कॉन्फ़िगरेशन पार्सिंग
settings-licenses-desc-reqwest = HTTP क्लाइंट
settings-licenses-desc-tokio-tungstenite = WebSocket क्लाइंट
settings-licenses-desc-rustls = TLS इम्प्लीमेंटेशन
settings-licenses-desc-blake3 = हैश फ़ंक्शन
settings-licenses-desc-sha-2 = SHA-256/512 हैशिंग
settings-licenses-desc-bs58 = Base58 एन्कोडिंग
settings-licenses-desc-base64 = Base64 एन्कोडिंग
settings-licenses-desc-lucide-icons = आइकन फ़ॉन्ट लाइब्रेरी
settings-licenses-desc-inter = इंटरफ़ेस फ़ॉन्ट
settings-licenses-desc-jetbrains-mono = मोनोस्पेस फ़ॉन्ट
settings-licenses-desc-orbitron = डिस्प्ले फ़ॉन्ट
settings-licenses-desc-vazirmatn = अरबी और फ़ारसी लिपि फ़ॉन्ट
settings-licenses-desc-noto-sans-devanagari = देवनागरी लिपि फ़ॉन्ट
settings-licenses-desc-noto-sans-sc = सरलीकृत चीनी फ़ॉन्ट
settings-licenses-desc-pretendard = कोरियाई फ़ॉन्ट
settings-licenses-desc-pretendard-jp = जापानी फ़ॉन्ट

settings-hints-title = संदर्भ हिंट
settings-hints-description = संदर्भ हिंट वे हेल्प आइकन हैं जो डैशबोर्ड की सुविधाएँ समझाते हैं। नीचे हर हिंट देखें और "दोबारा न दिखाएँ" से छिपाए गए हिंट वापस लाएँ — एक-एक करके या सभी एक साथ।
settings-hints-hidden-label = छिपे हुए हिंट
settings-hints-hidden-summary = कुल { $total } हिंट में से { $hidden } फ़िलहाल छिपे हैं।
settings-hints-restore-all = सभी हिंट वापस लाएँ
settings-hints-toggle-shown =
    .title = यह हिंट दिखाएँ
settings-hints-toggle-shown-title = दिख रहा है
settings-hints-toggle-hidden-title = छिपा है — दिखाने के लिए चालू करें
settings-hints-restore-title = सभी हिंट वापस लाएँ
settings-hints-restore-message = आपके छिपाए हुए सहित सभी संदर्भ हिंट फिर से दिखाएँ?
settings-hints-restore-confirm = सभी वापस लाएँ
settings-hints-restored = सभी हिंट वापस लाए गए

settings-account-title = { -brand } अकाउंट
settings-account-description = मुफ़्त और वैकल्पिक। { -brand } बिना अकाउंट के भी ट्रेड, डिस्कवरी और चार्टिंग करता है — बस सार्वजनिक प्रोवाइडर्स के ज़रिए। नीचे का पैनल बताता है कि साइन इन करने से क्या जुड़ता है।
settings-account-data-title = { -brand } डेटा
settings-account-data-description = हम screenerbot.io पर एक साझा मार्केट-डेटा सेवा चलाते हैं: सात टाइमफ़्रेम में पूल की हुई कैंडल, एक रिज़ॉल्व्ड पूल रजिस्ट्री, कैश की गई सुरक्षा रिपोर्ट और सामान्यीकृत टोकन पहचान। यह इसलिए है ताकि हर इंस्टॉल पर सार्वजनिक प्रोवाइडर्स की रेट लिमिट अलग-अलग न लगे, और इसके उपयोग के लिए अकाउंट चाहिए ताकि साझा लागत का हिसाब किसी के नाम रहे।
settings-account-data-fallback = यह उपलब्ध न हो तो { -brand } अपने आप सार्वजनिक प्रोवाइडर्स पर लौट जाता है। कुछ नहीं रुकता; चार्ट धीमे भरते हैं और उनमें कम हिस्ट्री होती है।
settings-account-gateway-title = ट्रांज़ैक्शन भेजना
settings-account-gateway-description = साइन इन होने पर { -brand } आपके स्वैप आपके अपने RPC के बजाय screenerbot.io के ज़रिए ब्रॉडकास्ट कर सकता है। आपका बॉट हर ट्रांज़ैक्शन इसी मशीन पर बनाता और साइन करता है — सर्वर केवल उसे रिले करता है, और साइन किए गए ट्रांज़ैक्शन को उसका सिग्नेचर अमान्य किए बिना बदल नहीं सकता।
settings-account-gateway-label = ट्रांज़ैक्शन भेजने के लिए { -brand } RPC का उपयोग करें
settings-account-gateway-hint = केवल सबमिशन के लिए। प्राइस डेटा हमेशा आपके अपने RPC से आता है — पूल पोलिंग साझा एंडपॉइंट के लिए बहुत भारी है, इसलिए उसे वहाँ कभी नहीं भेजा जाता।
settings-account-manage-title = अपना अकाउंट प्रबंधित करना
settings-account-manage-description = आपका पासवर्ड, ईमेल एड्रेस, कनेक्टेड डिवाइस और रेफ़रल पेआउट वेबसाइट पर प्रबंधित होते हैं। वहाँ किसी डिवाइस को रद्द करने पर वह हर जगह साइन आउट हो जाता है, इस डिवाइस सहित।
settings-account-open-dashboard = अपना डैशबोर्ड खोलें

settings-navigation-title = नेविगेशन टैब
settings-navigation-hint = क्रम बदलने के लिए आइटम खींचें। स्विच से दृश्यता बदलें।
settings-navigation-section-layout = लेआउट
settings-navigation-overflow-label = जो टैब फिट नहीं होते
settings-navigation-overflow-hint = टैब पंक्ति को बगल में स्क्रॉल करें, या जो टैब फिट नहीं होते उन्हें अंत में "और" मेनू में रखें।
settings-navigation-overflow-scroll = स्क्रॉल
settings-navigation-overflow-menu = "और" मेनू
settings-navigation-note = बदलाव सहेजने के बाद लागू होते हैं। नेविगेशन बार में अपडेट देखने के लिए पेज रीफ़्रेश करें।
settings-navigation-drag-handle =
    .title = क्रम बदलने के लिए खींचें
settings-navigation-defaults-failed = डिफ़ॉल्ट नेविगेशन लोड नहीं हो सका
settings-navigation-reset = नेविगेशन डिफ़ॉल्ट पर रीसेट हुआ

settings-data-storage-title = डेटाबेस स्टोरेज
settings-data-storage-description = आपके ट्रेडिंग डेटा, पोज़िशन और ऐतिहासिक जानकारी को रखने वाले सभी डेटाबेस का अवलोकन।
settings-data-stats-loading = डेटाबेस आँकड़े लोड हो रहे हैं...
settings-data-stats-load-failed = डेटाबेस आँकड़े लोड करना विफल
settings-data-total-storage = कुल डेटाबेस स्टोरेज
settings-data-db-tokens = टोकन
settings-data-db-transactions = ट्रांज़ैक्शन
settings-data-db-positions = पोज़िशन
settings-data-db-events = इवेंट्स
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = वॉलेट
settings-data-db-pools = पूल
settings-data-db-strategies = स्ट्रैटेजी
settings-data-db-actions = कार्रवाइयां
settings-data-directory-label = डेटा डायरेक्टरी
settings-data-directory-copied = डेटा डायरेक्टरी
settings-data-config-path-copied = कॉन्फ़िग पथ
settings-data-path-unavailable = उपलब्ध नहीं
settings-data-path-copy-title = पथ कॉपी करने के लिए क्लिक करें
settings-data-path-copy-failed = पथ कॉपी करना विफल

settings-data-config-title = कॉन्फ़िगरेशन प्रबंधन
settings-data-config-description = अपने बॉट का कॉन्फ़िगरेशन एक्सपोर्ट, इंपोर्ट और प्रबंधित करें। बड़े बदलाव से पहले बैकअप रखें।
settings-data-config-export = कॉन्फ़िग एक्सपोर्ट करें
settings-data-config-import = कॉन्फ़िग इंपोर्ट करें
settings-data-config-reset = डिफ़ॉल्ट पर रीसेट करें
settings-data-config-location-label = कॉन्फ़िग की जगह
settings-data-config-fetch-failed = कॉन्फ़िग प्राप्त करना विफल
settings-data-config-exported = कॉन्फ़िगरेशन एक्सपोर्ट हुआ
settings-data-config-export-failed = कॉन्फ़िग एक्सपोर्ट करना विफल: { $message }
settings-data-config-import-title = कॉन्फ़िगरेशन इंपोर्ट करें
settings-data-config-import-message = यह कॉन्फ़िगरेशन इंपोर्ट करें? वर्तमान सेटिंग्स ओवरराइट हो जाएँगी। वॉलेट क्रेडेंशियल सुरक्षित रहेंगे।
settings-data-config-imported = कॉन्फ़िगरेशन सफलतापूर्वक इंपोर्ट हुआ। कुछ बदलावों के लिए रीस्टार्ट ज़रूरी हो सकता है।
settings-data-config-import-failed = कॉन्फ़िग इंपोर्ट करना विफल: { $message }
settings-data-config-reset-title = कॉन्फ़िगरेशन रीसेट करें
settings-data-config-reset-message = सभी सेटिंग्स डिफ़ॉल्ट पर रीसेट करें? आपके वॉलेट क्रेडेंशियल सुरक्षित रहेंगे, पर बाकी सभी सेटिंग्स रीसेट हो जाएँगी।
settings-data-config-reset-done = कॉन्फ़िगरेशन डिफ़ॉल्ट पर रीसेट हुआ
settings-data-config-reset-failed = कॉन्फ़िग रीसेट करना विफल: { $message }
settings-data-unknown-error = अज्ञात त्रुटि

settings-data-cleanup-title = डेटा क्लीनअप
settings-data-cleanup-description = पुराना या अनुपयोगी डेटा हटाकर डिस्क स्पेस खाली करें। ये कार्रवाइयाँ पूर्ववत नहीं की जा सकतीं।
settings-data-ohlcv-cleanup-label = OHLCV डेटा क्लीनअप
settings-data-ohlcv-cleanup-hint = जो टोकन तय समय से सक्रिय नहीं रहे, उनका कैंडलस्टिक डेटा हटाएँ।
settings-data-cleanup-hours-unit = घं
settings-data-cleanup-ohlcv = OHLCV क्लीनअप
settings-data-cleanup-running = साफ़ हो रहा है...
settings-data-cleanup-hours-invalid = घंटों का मान अमान्य है
settings-data-cleanup-confirm-title = OHLCV डेटा हटाएँ
settings-data-cleanup-confirm-message =
    { $hours ->
        [one] { $hours } घंटे
       *[other] { $hours } घंटे
    } से अधिक समय से निष्क्रिय टोकन का OHLCV डेटा हटाएँ?
settings-data-cleanup-done =
    साफ़ किए गए निष्क्रिय टोकन: { $count ->
        [one] { $count }
       *[other] { $count }
    }
settings-data-cleanup-failed = क्लीनअप विफल
settings-data-cleanup-failed-detail = क्लीनअप विफल: { $message }

settings-data-cache-clear-label = पूरा OHLCV कैश साफ़ करें
settings-data-cache-clear-hint = सारा कैश किया कैंडलस्टिक डेटा मिटाएँ और हर मॉनिटर किए गए टोकन को शुरू से दोबारा लाएँ। चार्ट गलत दिखें या डेटा लॉजिक अपडेट के बाद इस्तेमाल करें।
settings-data-cache-clear = OHLCV कैश साफ़ करें
settings-data-cache-clearing = साफ़ हो रहा है...
settings-data-cache-confirm-title = पूरा OHLCV कैश साफ़ करें
settings-data-cache-confirm-message = हर टोकन का सारा कैश किया कैंडलस्टिक डेटा मिटाएँ? मॉनिटर किए गए टोकन अपनी हिस्ट्री शुरू से दोबारा लाएँगे। इसे पूर्ववत नहीं किया जा सकता।
settings-data-candles-count =
    { $count ->
        [one] { $count } कैंडल
       *[other] { $count } कैंडल
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } टोकन
       *[other] { $count } टोकन
    }
settings-data-cache-cleared = साफ़ किया गया: { $candles }, { $tokens } में; दोबारा लाया जा रहा है
settings-data-cache-clear-failed = OHLCV कैश साफ़ करना विफल
settings-data-cache-clear-failed-detail = OHLCV कैश साफ़ करना विफल: { $message }

settings-data-ui-cache-label = UI स्टेट कैश
settings-data-ui-cache-hint = सहेजी गई टेबल प्राथमिकताएँ, फ़िल्टर स्थितियाँ और व्यू सेटिंग्स साफ़ करें।
settings-data-ui-cache-clear = UI कैश साफ़ करें
settings-data-ui-cache-confirm-title = UI स्टेट साफ़ करें
settings-data-ui-cache-confirm-message = सभी सहेजी गई UI प्राथमिकताएँ साफ़ करें? इससे टेबल कॉलम, फ़िल्टर और व्यू सेटिंग्स रीसेट हो जाएँगी।
settings-data-ui-cache-cleared =
    कैश की गई UI सेटिंग्स साफ़ हुईं: { $count ->
        [one] { $count }
       *[other] { $count }
    }

settings-data-folder-label = डेटा फ़ोल्डर खोलें
settings-data-folder-hint = { -brand } का सारा डेटा रखने वाला फ़ोल्डर अपने फ़ाइल मैनेजर में खोलें।
settings-data-folder-open = फ़ोल्डर खोलें
settings-data-folder-open-failed = डेटा फ़ोल्डर नहीं खोला जा सका
