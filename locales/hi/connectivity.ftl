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
