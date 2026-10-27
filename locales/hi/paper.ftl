paper-readiness-history = पेपर हिस्ट्री
paper-readiness-history-met =
    { $count ->
        [one] { $count } बंद पेपर राउंड, { $needed } ज़रूरी
       *[other] { $count } बंद पेपर राउंड, { $needed } ज़रूरी
    }
paper-readiness-history-short = { $needed } में से { $count } बंद पेपर राउंड
paper-readiness-profit = पेपर में लाभदायक
paper-readiness-profit-detail =
    { $count ->
        [one] { $count } राउंड में { $realized } { -sol } वास्तविक, { $wins } जीत
       *[other] { $count } राउंड में { $realized } { -sol } वास्तविक, { $wins } जीत
    }
paper-readiness-exits = एग्ज़िट भर रहे हैं
paper-readiness-exits-ok = हाल के पेपर एग्ज़िट भरे गए
paper-readiness-exits-failing = लगातार बिना भरे पेपर एग्ज़िट: { $count }
paper-live-block-setup-incomplete = पहले वॉलेट और RPC सेटअप पूरा करें
paper-live-block-force-stop = इमरजेंसी स्टॉप चालू है
