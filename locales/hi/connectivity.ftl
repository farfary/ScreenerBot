# Connectivity endpoint health text. Probe ids come from ProbeFailure in
# src/connectivity/types.rs; the technical values are not translated.
connectivity-probe-client-setup-failed = क्लाइंट बनाने में विफल: { $detail }
connectivity-probe-http-status = HTTP { $status }
connectivity-probe-timeout = { $seconds } सेकंड बाद टाइमआउट
connectivity-probe-request-failed = अनुरोध विफल: { $detail }
connectivity-probe-rpc-none = कोई RPC प्रोवाइडर कॉन्फ़िगर नहीं है
connectivity-probe-rpc-partial = RPC प्रोवाइडर स्वस्थ: { $healthy }/{ $total }। अस्वस्थ: { $unhealthy }
connectivity-probe-rpc-all-unreachable = सभी RPC प्रोवाइडर पहुँच से बाहर (कुल: { $total }): { $unhealthy }
connectivity-probe-internet-dns-failed = DNS जाँच विफल रही, लेकिन HTTP काम कर रहा है: { $detail }
connectivity-probe-internet-all-failed = DNS और HTTP जाँच विफल रहीं। DNS: { $dns }। HTTP: { $http }
connectivity-probe-unknown = अज्ञात त्रुटि

connectivity-endpoint-not-checked = अभी जाँचा नहीं गया

connectivity-service-checking = { -brand } सेवा से जुड़ रहे हैं
    .detail = कनेक्शन की पुष्टि होते ही ट्रेडिंग शुरू होगी।
connectivity-service-ready = { -brand } सेवा जुड़ी है
    .detail = वॉलेट रीड और ट्रेड screenerbot.io से होकर जाते हैं।
connectivity-service-signed-out = ट्रेडिंग रुकी है · साइन आउट
    .detail = फिर से शुरू करने के लिए साइन इन करें। ट्रेडिंग रुकी रहने तक खुली पोज़िशन नहीं बेची जातीं।
connectivity-service-reauthorize = ट्रेडिंग रुकी है · फिर से साइन इन ज़रूरी
    .detail = इस डिवाइस का साइन-इन अभी वॉलेट रीड की अनुमति नहीं देता। फिर से शुरू करने के लिए दोबारा साइन इन करें।
connectivity-service-busy = ट्रेडिंग रुकी है · सेवा व्यस्त, दोबारा कोशिश
    .detail = ट्रेडिंग अपने आप फिर शुरू होगी। रुकावट से बचने के लिए अपना RPC इस्तेमाल करें।
connectivity-service-daily-limit = ट्रेडिंग रुकी है · दैनिक सीमा पूरी
    .detail = आपके खाते ने { -brand } सेवा की आज की सीमा पूरी कर ली है। ट्रेडिंग 00:00 UTC पर फिर शुरू होगी, या अपने RPC से तुरंत।
connectivity-service-unreachable = ट्रेडिंग रुकी है · screenerbot.io तक नहीं पहुँच पा रहे
    .detail = कनेक्शन लौटने पर ट्रेडिंग फिर शुरू होगी। अपना इंटरनेट कनेक्शन जाँचें।
connectivity-service-version = ट्रेडिंग रुकी है · अपडेट ज़रूरी
    .detail = यह संस्करण अब { -brand } सेवा में समर्थित नहीं है। { $minimum } या नए संस्करण में अपडेट करें।
