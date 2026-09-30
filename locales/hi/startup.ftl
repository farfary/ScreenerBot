startup-wallet-mismatch-title = वॉलेट बदल गया है
startup-wallet-mismatch-detail =
    आपके कॉन्फ़िगरेशन का वॉलेट इस कंप्यूटर के लोकल इतिहास में दर्ज वॉलेट से मेल नहीं खाता।

    वर्तमान वॉलेट: { $current }
    पिछला वॉलेट: { $stored }

    प्रभावित लोकल डेटा: { $systems }

    ऐसा आमतौर पर किसी दूसरी प्राइवेट की इंपोर्ट करने या दूसरा कॉन्फ़िगरेशन रीस्टोर करने के बाद होता है। ट्रेडिंग, पोज़िशन और इतिहास पिछले वॉलेट के हैं और नया वॉलेट सुरक्षित रूप से शुरू करने से पहले इन्हें साफ़ करना ज़रूरी है।
startup-wallet-mismatch-systems-default = ट्रांज़ैक्शन, पोज़िशन, वॉलेट इतिहास
startup-wallet-mismatch-remedy =
    जारी रखने के लिए पिछले वॉलेट का लोकल इतिहास साफ़ करें (आपके डेटाबेस का पहले अपने आप बैकअप लिया जाता है):

      - ऐप में: नीचे "{ $action }" चुनें।
      - टर्मिनल से: यह चलाएँ  screenerbot --clean-wallet-data

    ऑन-चेन फ़ंड पर कोई असर नहीं पड़ता; केवल इस कंप्यूटर का लोकल ट्रेड/पोज़िशन इतिहास रीसेट होता है। बैकअप यहाँ लिखे जाते हैं:
      { $path }
startup-recovery-reset-wallet = वॉलेट डेटा रीसेट करें और रीस्टार्ट करें

startup-port-in-use-title = नेटवर्क पोर्ट व्यस्त है
startup-port-in-use-detail = डैशबोर्ड पोर्ट { $address } पहले से उपयोग में है।
startup-port-in-use-remedy = कोई दूसरा प्रोग्राम उस पोर्ट का उपयोग कर रहा है जो { -brand } को चाहिए। उस प्रोग्राम को बंद करें, या सेटिंग्स में वेबसर्वर पोर्ट बदलें, फिर { -brand } दोबारा शुरू करें।

startup-lock-held-title = { -brand } पहले से चल रहा है
startup-lock-held-detail = इस कंप्यूटर पर { -brand } की दूसरी कॉपी पहले से चल रही है, इसलिए दूसरी शुरू नहीं हो सकती।
startup-lock-held-remedy = पहले से खुली विंडो पर जाएँ। यदि कोई विंडो न दिखे, तो बैकग्राउंड में चल रही किसी भी { -brand } प्रोसेस को बंद करके दोबारा प्रयास करें। रीबूट के बाद भी समस्या बनी रहे तो लॉक फ़ाइल पुरानी हो सकती है और उसे डेटा फ़ोल्डर से हटाया जा सकता है (.screenerbot.lock)।

startup-config-invalid-title = कॉन्फ़िगरेशन पढ़ा नहीं जा सका
startup-config-parse-detail = config.toml पार्स नहीं हो सका: { $detail }
startup-config-load-parse-detail = कॉन्फ़िग लोड करने में विफल: config.toml पार्स नहीं हो सका: { $detail }
startup-config-parse-remedy = आपकी कॉन्फ़िगरेशन फ़ाइल पढ़ी नहीं जा सकी। डेटा फ़ोल्डर से बैकअप रीस्टोर करें, या कॉन्फ़िगरेशन को डिफ़ॉल्ट पर रीसेट करके अपना वॉलेट और RPC दोबारा सेट करें।
startup-config-load-parse-remedy = कोई मान्य कॉन्फ़िगरेशन रीस्टोर करें या सेटअप दोबारा पूरा करें।
startup-option-invalid-title = स्टार्टअप विकल्प अमान्य है
startup-option-invalid-remedy = कोई कमांड-लाइन विकल्प अमान्य है। उस विकल्प के बिना { -brand } शुरू करें, या उसे सुधारकर दोबारा प्रयास करें।

startup-generic-title = { -brand } शुरू नहीं हो सका
startup-generic-remedy = विवरण के लिए लॉग फ़ाइल देखें, फिर ऐप रीस्टार्ट करें। समस्या बनी रहे तो t.me/screenerbotio_support पर सपोर्ट से संपर्क करें।
startup-generic-detail = { $error }
startup-failure-directories = ज़रूरी डायरेक्टरी बनाने में विफल: { $error }
startup-failure-config-load = कॉन्फ़िग लोड करने में विफल: { $error }
startup-failure-actions-init = एक्शन डेटाबेस इनिशियलाइज़ करने में विफल: { $error }
startup-failure-actions-sync = डेटाबेस से एक्शन सिंक करने में विफल: { $error }
startup-failure-strategy-init = स्ट्रैटेजी सिस्टम इनिशियलाइज़ करने में विफल: { $error }
startup-failure-analysis-init = विश्लेषण इंजन इनिशियलाइज़ करने में विफल: { $error }
startup-failure-assistant-init = असिस्टेंट चैट इंजन इनिशियलाइज़ करने में विफल: { $error }
startup-failure-wallets-init = वॉलेट इनिशियलाइज़ करने में विफल: { $error }
startup-failure-wallet-validation = वॉलेट की संगति सत्यापित करने में विफल: { $error }
