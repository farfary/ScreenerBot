telegram-reply-status = स्थिति
telegram-reply-balance = बैलेंस
telegram-reply-positions = पोज़िशन
telegram-reply-pause = रोकें
telegram-reply-resume = फिर शुरू करें
telegram-reply-stop = बंद करें
telegram-reply-stats = आँकड़े
telegram-reply-menu = मेनू
telegram-reply-help = सहायता

telegram-button-positions = पोज़िशन
telegram-button-balance = बैलेंस
telegram-button-stats = आँकड़े
telegram-button-tokens = टोकन
telegram-button-pause = रोकें
telegram-button-stop = बंद करें
telegram-button-settings = सेटिंग्स
telegram-button-refresh = रिफ़्रेश
telegram-button-menu = मेनू
telegram-button-back = वापस
telegram-button-back-to-menu = मेनू पर वापस
telegram-button-back-to-tokens = टोकन पर वापस
telegram-button-cancel = रद्द करें
telegram-button-close-all-positions = सभी पोज़िशन बंद करें
telegram-button-sell-percent = { $percent }% बेचें
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = ब्लैकलिस्ट
telegram-button-blacklist-symbol = { $symbol } ब्लैकलिस्ट करें
telegram-button-close-position = पोज़िशन बंद करें
telegram-button-confirm-close = बंद करने की पुष्टि करें
telegram-button-confirm-close-all = सभी पोज़िशन बंद करें
telegram-button-confirm-sell = { $percent }% बिक्री की पुष्टि करें
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = ज़बरन रोक की पुष्टि करें
telegram-button-confirm-buy = { $amount } { -sol } खरीदें
telegram-button-notifications = नोटिफ़िकेशन
telegram-button-trading = ट्रेडिंग
telegram-button-entry-monitor = एंट्री मॉनिटर
telegram-button-exit-monitor = एग्ज़िट मॉनिटर
telegram-button-auto-trading = ऑटो ट्रेडिंग
telegram-button-force-stop = ज़बरन रोक
telegram-button-notify-opened = खुली
telegram-button-notify-closed = बंद
telegram-button-notify-partial = आंशिक
telegram-button-notify-dca = DCA
telegram-button-notify-errors = त्रुटियाँ
telegram-button-details = विवरण
telegram-button-position = पोज़िशन
telegram-button-sell-more = और बेचें
telegram-button-more-dca = और DCA
telegram-button-history = इतिहास
telegram-button-status = स्थिति
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = दोबारा ऑथेंटिकेट करें
telegram-button-previous = पिछला
telegram-button-next = अगला
telegram-button-passed = पास
telegram-button-rejected = अस्वीकृत
telegram-button-new-24h = नए (24h)
telegram-button-all-tokens = सभी टोकन
telegram-button-search-token = टोकन खोजें
telegram-button-filter-stats = फ़िल्टर आँकड़े
telegram-button-refresh-stats = आँकड़े रिफ़्रेश करें
telegram-button-view-position = पोज़िशन देखें
telegram-button-buy-amount = { $amount } { -sol }

telegram-unknown-command =
    अज्ञात कमांड: { $command }

    उपलब्ध कमांड देखने के लिए /help का उपयोग करें।
telegram-session-expired =
    <b>सेशन समाप्त हो गया</b>

    दोबारा ऑथेंटिकेट करने के लिए /login का उपयोग करें।
telegram-2fa-required =
    <b>2FA आवश्यक है</b>

    कृपया अपना 6 अंकों का ऑथेंटिकेटर कोड दर्ज करें।
telegram-account-locked =
    <b>अकाउंट लॉक हो गया</b>

    बहुत अधिक असफल प्रयास हुए।
    { $seconds ->
        [one] { $seconds } सेकंड बाद दोबारा प्रयास करें।
       *[other] { $seconds } सेकंड बाद दोबारा प्रयास करें।
    }
telegram-code-invalid = कृपया मान्य 6 अंकों का कोड दर्ज करें।
telegram-authenticated =
    <b>ऑथेंटिकेट हो गया!</b>

    अब आपके पास बॉट कमांड का एक्सेस है।
telegram-wrong-code =
    <b>गलत कोड</b>

    { $remaining ->
        [one] { $remaining } प्रयास शेष है।
       *[other] { $remaining } प्रयास शेष हैं।
    }
telegram-auth-required =
    <b>ऑथेंटिकेशन आवश्यक है</b>

    जारी रखने के लिए कृपया अपना पासवर्ड दर्ज करें।

    <i>अपना पासवर्ड लिखकर भेजें।</i>
telegram-login-required =
    <b>लॉगिन आवश्यक है</b>

    कृपया अपना 6 अंकों का ऑथेंटिकेटर कोड दर्ज करें:
telegram-session-activated =
    <b>सेशन सक्रिय हो गया</b>

    2FA कॉन्फ़िगर नहीं है। आपका सेशन अब सक्रिय है।

    <i>सुझाव: बेहतर सुरक्षा के लिए सिक्योरिटी सेटिंग्स में 2FA चालू करें।</i>

telegram-discovery-hello = नमस्ते { $name }!
telegram-discovery-default-name = उपयोगकर्ता
telegram-discovery-detected = <b>चैट मिल गई!</b>
telegram-discovery-details =
    चैट ID: <code>{ $chat_id }</code>
    प्रकार: { $chat_type }

    कृपया { -brand } डैशबोर्ड पर जाएँ और इस चैट को चुनने के लिए उस पर क्लिक करें।
telegram-chat-type-private = प्राइवेट
telegram-chat-type-group = ग्रुप
telegram-chat-type-supergroup = सुपरग्रुप
telegram-chat-type-channel = चैनल

telegram-menu-title =
    <b>कंट्रोल पैनल</b>

    जानकारी देखने या बॉट को नियंत्रित करने के लिए कोई विकल्प चुनें।
telegram-menu-positions-empty =
    <b>कोई खुली पोज़िशन नहीं</b>

    नए अवसरों की प्रतीक्षा है...
telegram-menu-positions-title = <b>पोज़िशन ({ $count })</b>
telegram-menu-positions-hint = <i>प्रबंधित करने के लिए किसी पोज़िशन पर टैप करें।</i>
telegram-menu-settings =
    <b>सेटिंग्स</b>

    नोटिफ़िकेशन और ट्रेडिंग पैरामीटर कॉन्फ़िगर करें।
telegram-settings-notifications =
    <b>नोटिफ़िकेशन सेटिंग्स</b>

    नोटिफ़िकेशन चालू/बंद करें:
telegram-settings-trading =
    <b>ट्रेडिंग कंट्रोल</b>

    ट्रेडिंग फ़ीचर चालू/बंद करें:
telegram-pagination-expired = पेजिनेशन सेशन समाप्त हो गया।

telegram-status-state-stopped = <b>रुका हुआ</b> (ज़बरन रोक सक्रिय)
telegram-status-state-active = <b>सक्रिय</b>
telegram-status-state-paused = <b>रोका गया</b>
telegram-status-on = चालू
telegram-status-off = बंद
telegram-status-body =
    <b>सिस्टम स्थिति</b>

    <b>सिस्टम</b>
    स्थिति — { $state }
    अपटाइम — { $uptime }
    वर्शन — v{ $version }

    <b>ट्रेडिंग</b>
    एंट्री — { $entries }
    एग्ज़िट — { $exits }
    पोज़िशन — { $positions }
telegram-positions-empty =
    <b>कोई खुली पोज़िशन नहीं</b>

    अवसरों की प्रतीक्षा है...
telegram-positions-title = <b>खुली पोज़िशन ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } और...</i>
telegram-positions-summary =
    <b>पोर्टफ़ोलियो सारांश</b>
    निवेशित — { $invested } { -sol }
    शुद्ध लाभ-हानि (P{ "&amp;" }L) — { $pnl } { -sol }
telegram-balance-body =
    <b>वॉलेट बैलेंस</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>दैनिक आँकड़े</b>

    पोज़िशन — { $positions }
    निवेशित — { $invested } { -sol }
    लाभ-हानि (P{ "&amp;" }L) — { $pnl } { -sol }

telegram-start-ready =
    <b>{ -brand } तैयार है!</b>

    ट्रेडिंग <b>चालू</b> है।

    बॉट को नियंत्रित करने के लिए नीचे दिए कीबोर्ड का उपयोग करें।
    उपलब्ध कमांड के लिए /help लिखें।
telegram-stop-already = <b>ट्रेडिंग पहले से बंद है</b>
telegram-stop-done =
    <b>ट्रेडिंग बंद हो गई</b>

    सभी ट्रेडिंग मॉनिटर (एंट्री { "&amp;" } एग्ज़िट) रुक गए हैं।
    केवल एंट्री रोकने के लिए /pause का उपयोग करें।
telegram-stop-failed =
    <b>ट्रेडिंग बंद करने में विफल</b>

    त्रुटि: { $detail }
telegram-pause-done =
    <b>एंट्री मॉनिटर रोका गया</b>

    कोई नई पोज़िशन नहीं खोली जाएगी।
    एग्ज़िट मॉनिटर चलता रहेगा।
telegram-pause-failed =
    <b>एंट्री रोकने में विफल</b>

    त्रुटि: { $detail }
telegram-resume-done =
    <b>एंट्री मॉनिटर फिर से शुरू हुआ</b>

    अब एंट्री सिग्नल पर नज़र रखी जा रही है।
telegram-resume-failed =
    <b>एंट्री फिर से शुरू करने में विफल</b>

    त्रुटि: { $detail }
telegram-force-stop-confirm =
    <b>ज़बरन रोक</b>

    इससे सारी ट्रेडिंग गतिविधि तुरंत रुक जाएगी:
    • कोई नई एंट्री नहीं
    • कोई एग्ज़िट नहीं (स्टॉप लॉस सहित)
    • कोई DCA ऑपरेशन नहीं
telegram-force-stop-warning = <b>यह एक इमरजेंसी क्रिया है!</b>
telegram-force-stop-question = क्या आप निश्चित हैं?
telegram-force-stop-active =
    <b>ज़बरन रोक सक्रिय हो गई</b>

    सारी ट्रेडिंग रोक दी गई है।

    इस फ़्लैग को हटाने के लिए /resume_trading का उपयोग करें।
telegram-resume-trading-not-stopped =
    <b>ट्रेडिंग ज़बरन रोकी हुई नहीं है</b>

    कोई कार्रवाई आवश्यक नहीं।
telegram-resume-trading-done =
    <b>ट्रेडिंग फिर से शुरू हुई</b>

    ज़बरन रोक का फ़्लैग हटा दिया गया है।
    सामान्य ट्रेडिंग ऑपरेशन अब फिर से शुरू हो सकते हैं।

telegram-help-title = <b>{ -brand } सहायता</b>
telegram-help-heading-dashboard = डैशबोर्ड
telegram-help-heading-market = मार्केट
telegram-help-heading-trading = ट्रेडिंग
telegram-help-heading-safety = सुरक्षा
telegram-help-heading-system = सिस्टम
telegram-help-commands-dashboard =
    /status — सिस्टम स्थिति { "&amp;" } अपटाइम
    /stats — दैनिक परफ़ॉर्मेंस
    /balance — वॉलेट बैलेंस
    /positions — खुली पोज़िशन
telegram-help-commands-market =
    /tokens — टोकन एक्सप्लोरर
    /rejected — फ़िल्टर किए गए टोकन
telegram-help-commands-trading =
    /start — ट्रेडिंग सिस्टम चालू करें
    /stop — ट्रेडिंग सिस्टम बंद करें
    /pause — नई एंट्री रोकें
    /resume — नई एंट्री फिर से शुरू करें
    /menu — इंटरैक्टिव मेनू
telegram-help-commands-safety =
    /force_stop — <b>इमरजेंसी रोक</b>
    /resume_trading — इमरजेंसी स्थिति हटाएँ
telegram-help-commands-system =
    /update — अपडेट स्थिति { "&amp;" } इंस्टॉल
    /login — 2FA ऑथेंटिकेशन
telegram-help-tip = <i>सुझाव: कमांड चलाने के लिए उस पर टैप करें।</i>

telegram-update-up-to-date-auto =
    <b>अप टू डेट</b>

    v{ $version } चल रहा है, अपने आप इंस्टॉल हुआ।
telegram-update-up-to-date =
    <b>अप टू डेट</b>

    v{ $version } चल रहा है।
telegram-update-check-failed =
    <b>अपडेट की जाँच विफल रही</b>

    { $reason }
telegram-update-unreachable = screenerbot.io तक पहुँचा नहीं जा सका।
telegram-update-installing = <b>v{ $version } इंस्टॉल हो रहा है</b>
telegram-update-restarting =
    { -brand } नए वर्शन पर रीस्टार्ट हो रहा है। ट्रेडिंग अपने आप फिर से शुरू हो जाएगी।
telegram-update-install-failed =
    <b>v{ $version } इंस्टॉल नहीं हो सका</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } डाउनलोड हो गया</b>

    यह रिलीज़ डेस्कटॉप ऐप भी अपडेट करती है, इसलिए उसका इंस्टॉलर उस मशीन पर चलाना होगा। वहाँ सेटिंग्स → अपडेट खोलें।
telegram-update-downloading =
    <b>v{ $version } डाउनलोड हो रहा है</b>

    { $size } MB में से { $percent }%।
telegram-update-available =
    <b>v{ $version } उपलब्ध है</b>

    { $how }
    डाउनलोड साइज़: { $size } MB।

    यह अपने आप डाउनलोड होता है; तैयार होने पर /update दोबारा भेजें।
telegram-update-how-core = थोड़े रीस्टार्ट के साथ चुपचाप इंस्टॉल होता है।
telegram-update-how-installer = डेस्कटॉप इंस्टॉलर को एक बार चलाना ज़रूरी है।

telegram-value-unknown = अज्ञात
telegram-value-na = लागू नहीं
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } सेकंड
telegram-duration-minutes = { $minutes } मिनट
telegram-duration-minutes-seconds = { $minutes } मिनट { $seconds } सेकंड
telegram-duration-hours =
    { $hours ->
        [one] { $hours } घंटा
       *[other] { $hours } घंटे
    }
telegram-duration-hours-minutes =
    { $hours ->
        [one] { $hours } घंटा
       *[other] { $hours } घंटे
    } { $minutes } मिनट
telegram-duration-days = { $days } दिन
telegram-duration-days-hours =
    { $days } दिन { $hours ->
        [one] { $hours } घंटा
       *[other] { $hours } घंटे
    }
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = त्रुटि: { $detail }
telegram-ai-reasoning =
    <b>LLM विश्लेषण</b>
    <i>{ $reasoning }</i>

telegram-row-entry = एंट्री — { $price } { -sol }
telegram-row-exit = एग्ज़िट — { $price } { -sol }
telegram-row-current = वर्तमान — { $price } { -sol }
telegram-row-invested = निवेशित — { $amount } { -sol }
telegram-row-received = प्राप्त — { $amount } { -sol }
telegram-row-value = मूल्य — { $amount } { -sol }
telegram-row-total = कुल — { $amount } { -sol }
telegram-row-tokens = टोकन — { $tokens }
telegram-row-duration = अवधि — { $duration }
telegram-row-reason = कारण — { $reason }
telegram-row-remaining = शेष — { $percent }%
telegram-row-pnl = लाभ-हानि (P{ "&amp;" }L) — { $pnl }
telegram-row-dca = DCA — #{ $count }

telegram-notify-opened-title = <b>पोज़िशन खुली</b>
telegram-notify-opened-size = साइज़ — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = कीमत — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>पोज़िशन बंद</b> — लाभ
telegram-notify-closed-title-loss = <b>पोज़िशन बंद</b> — हानि
telegram-notify-closed-reason-unspecified = बंद
telegram-notify-partial-title = <b>आंशिक एग्ज़िट</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }% बेचा गया
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = जोड़ा गया — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = औसत — { $price } { -sol }
telegram-notify-severity-critical = <b>गंभीर त्रुटि</b>
telegram-notify-severity-error = <b>त्रुटि</b>
telegram-notify-severity-warning = <b>चेतावनी</b>
telegram-notify-severity-info = <b>जानकारी</b>
telegram-notify-alert-title = <b>ट्रेड अलर्ट</b>
telegram-notify-alert-token = टोकन: <code>${ $symbol }</code>
telegram-notify-alert-mint = मिंट: <code>{ $mint }</code>
telegram-notify-alert-bought = एक्शन: { $amount } { -sol } खरीदे
telegram-notify-alert-sold = एक्शन: { $amount } { -sol } बेचे
telegram-notify-alert-wallet = वॉलेट: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (पेपर)
telegram-notify-copy-task = टास्क: { $task }
telegram-notify-scheduled-completed = <b>शेड्यूल्ड टास्क पूरा हुआ</b>
telegram-notify-scheduled-failed = <b>शेड्यूल्ड टास्क विफल रहा</b>
telegram-notify-scheduled-timed-out = <b>शेड्यूल्ड टास्क टाइमआउट हो गया</b>
telegram-notify-scheduled-error = त्रुटि: { $error }
telegram-notify-summary-title = <b>दैनिक सारांश</b> — { $date }
telegram-notify-summary-performance = <b>परफ़ॉर्मेंस</b>
telegram-notify-summary-trades = ट्रेड — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = विन रेट — { $percent }%
telegram-notify-summary-pnl = लाभ-हानि (P{ "&amp;" }L) — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = खुली पोज़िशन — { $count }
telegram-notify-started-title = <b>{ -brand } शुरू हुआ</b>
telegram-notify-started-version = <b>वर्शन</b> — { $version }
telegram-notify-started-mode = <b>मोड</b> — { $mode }
telegram-notify-started-ready = ट्रेडिंग के लिए तैयार!
telegram-notify-stopped-title = <b>{ -brand } बंद हुआ</b>
telegram-notify-stopped-reason = <b>कारण</b> — { $reason }
telegram-notify-stopped-goodbye = अलविदा! { $icon }
telegram-notify-start-mode-normal = सामान्य
telegram-notify-stop-reason-graceful = सामान्य शटडाउन
telegram-notify-update-available =
    <b>अपडेट v{ $version } उपलब्ध है</b>

    { $how }
    डाउनलोड साइज़: { $size } MB
telegram-notify-update-how-installer = यह रिलीज़ डेस्कटॉप ऐप भी अपडेट करती है, इसलिए उसका इंस्टॉलर एक बार चलाना होगा।
telegram-notify-update-ready =
    <b>अपडेट v{ $version } तैयार है</b>

    { $how }
telegram-notify-update-ready-silent = अभी लागू करने के लिए /update भेजें, या यह { -brand } के अगली बार शुरू होने पर इंस्टॉल हो जाएगा।
telegram-notify-update-ready-installer = इंस्टॉलर चलाने के लिए सेटिंग्स → अपडेट खोलें।
telegram-notify-update-applying =
    <b>v{ $version } इंस्टॉल हो रहा है</b>

    बैकएंड रीस्टार्ट हो रहा है; ट्रेडिंग अपने आप फिर से शुरू हो जाएगी।
telegram-notify-new-tokens =
    <b>फ़िल्टरिंग अलर्ट</b>

    { $count ->
        [one] आपके मानदंड से मेल खाते { $count } नए टोकन मिले।
       *[other] आपके मानदंड से मेल खाते { $count } नए टोकन मिले।
    }
telegram-notify-crash =
    <b>बॉट क्रैश हो गया!</b>

    <b>स्थान:</b> <code>{ $location }</code>
    <b>त्रुटि:</b> <code>{ $error }</code>
telegram-notify-crash-restart = कृपया बॉट को रीस्टार्ट करें।

telegram-filter-results-title = <b>फ़िल्टर परिणाम</b> ({ $count })
telegram-filter-results-empty = <i>कोई टोकन नहीं मिला।</i>
telegram-filter-results-page = <i>पेज { $page } / { $total }</i>

telegram-position-not-found = पोज़िशन नहीं मिली
telegram-position-no-positions = बंद करने के लिए कोई पोज़िशन नहीं है
telegram-position-history-empty =
    <b>ट्रेड इतिहास</b>

    अभी तक कोई बंद पोज़िशन नहीं है।
telegram-position-history-title = <b>हाल के ट्रेड</b>
telegram-position-history-more = <i>+{ $count } और ट्रेड...</i>
telegram-position-confirm-hint = <i>निष्पादित करने के लिए 30 सेकंड के भीतर पुष्टि करें।</i>
telegram-position-confirm-close-title = <b>पोज़िशन बंद करें?</b>
telegram-position-confirm-close-selling = { $tokens } टोकन बेचे जा रहे हैं
telegram-position-confirm-close-estimated = अनुमानित — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>30 सेकंड के भीतर पुष्टि करें</i>
telegram-position-confirm-sell =
    <b>बिक्री की पुष्टि करें</b>

    टोकन — { $symbol }
    राशि — { $percent }%
    टोकन — { $tokens }
telegram-position-confirm-dca =
    <b>और खरीद की पुष्टि करें</b>

    टोकन — { $symbol }
    जोड़ें — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>सभी पोज़िशन बंद करें?</b>

    संख्या — { $count }
telegram-position-confirm-close-all-hint =
    <i>इससे सभी खुली पोज़िशन मार्केट भाव पर बिक जाएँगी।
    30 सेकंड के भीतर पुष्टि करें।</i>
telegram-position-confirm-force-stop =
    <b>ज़बरन रोक</b>

    इससे सारी ट्रेडिंग तुरंत रुक जाएगी:
    • कोई नई एंट्री नहीं
    • कोई एग्ज़िट नहीं
    • कोई DCA नहीं
telegram-position-confirm-force-stop-warning = <b>यह एक इमरजेंसी क्रिया है।</b>
telegram-position-confirm-blacklist =
    <b>टोकन ब्लैकलिस्ट करें?</b>

    टोकन — { $symbol }
    मिंट — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>इससे पोज़िशन बंद हो जाएगी और भविष्य की एंट्री रुक जाएँगी।</i>
telegram-position-selling = { $symbol } का { $percent }% बेचा जा रहा है...
telegram-position-sell-done =
    <b>बिक्री निष्पादित हुई</b>

    टोकन — { $symbol }
    बेचा गया — { $percent }%
    प्राप्त — { $amount } { -sol }
telegram-position-sell-failed = <b>बिक्री विफल रही</b>
telegram-position-adding = { $symbol } में { $amount } { -sol } जोड़े जा रहे हैं...
telegram-position-dca-done =
    <b>DCA निष्पादित हुआ</b>

    टोकन — { $symbol }
    जोड़ा गया — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA विफल रहा</b>
telegram-position-closing-all = सभी पोज़िशन बंद की जा रही हैं...
telegram-position-close-all-done =
    <b>सभी बंद करना पूरा हुआ</b>

    बंद — { $closed }
    विफल — { $failed }
telegram-position-blacklisted =
    <b>टोकन ब्लैकलिस्ट हुआ</b>

    टोकन — { $symbol }
    स्थिति — बंद { "&amp;" } ब्लैकलिस्टेड

telegram-token-not-found = टोकन नहीं मिला
telegram-token-not-found-prefix = टोकन नहीं मिला। लंबे प्रीफ़िक्स से खोजकर देखें।
telegram-token-stats-failed = आँकड़े प्राप्त करने में विफल: { $detail }
telegram-token-list-failed = टोकन प्राप्त करने में विफल: { $detail }
telegram-token-list-empty = <b>{ $view }</b> व्यू में कोई टोकन नहीं मिला।
telegram-token-view-passed = फ़िल्टर पास
telegram-token-view-rejected = अस्वीकृत
telegram-token-view-recent = हाल में जोड़े गए
telegram-token-view-all = सभी टोकन
telegram-token-list-title = <b>{ $name }</b> (पेज { $page }/{ $total })
telegram-token-list-stats = लिक्विडिटी: { $liquidity } • कीमत: { $price }
telegram-token-list-hint = <i>विवरण देखने के लिए /token_ID पर टैप करें</i>
telegram-token-explorer =
    <b>मार्केट एक्सप्लोरर</b>

    <b>ओवरव्यू</b>
    फ़िल्टर पास — { $passed }
    अस्वीकृत — { $rejected }
    सक्रिय कीमतें — { $priced }
    कुल खोजे गए — { $total }

    <i>ब्राउज़ करने के लिए कोई श्रेणी चुनें:</i>
telegram-token-filter-title = <b>फ़िल्टर विश्लेषण</b>
telegram-token-filter-distribution = <b>वितरण</b>
telegram-token-filter-passed = पास — { $count } ({ $percent }%)
telegram-token-filter-rejected = अस्वीकृत — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = ब्लैकलिस्टेड — { $count }
telegram-token-filter-coverage = <b>कवरेज</b>
telegram-token-filter-priced = पूल कीमत वाले — { $count }
telegram-token-filter-open = खुली पोज़िशन — { $count }
telegram-token-filter-total = कुल खोजे गए — { $count }
telegram-token-filter-updated = <b>अंतिम अपडेट</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>हर { $interval } में अपने आप रिफ़्रेश होता है</i>
telegram-token-detail-active = <b>सक्रिय पोज़िशन</b>
telegram-token-detail-price = कीमत — { $price } { -sol }
telegram-token-detail-liquidity = लिक्विडिटी — { $value }
telegram-token-detail-volume = 24h वॉल्यूम — { $value }
telegram-token-detail-change = 24h बदलाव — { $value }
telegram-token-detail-risk = जोखिम आकलन: { $score }/100
telegram-token-detail-risk-unknown = जोखिम आकलन: अज्ञात
telegram-token-detail-action = <i>क्रिया चुनें:</i>
telegram-token-search =
    <b>मार्केट खोजें</b>

    खोजने के लिए सिंबल या मिंट एड्रेस दर्ज करें:

    <i>उदाहरण: /token_BONK या /token_So11111</i>
telegram-token-confirm-buy =
    <b>सीधी खरीद की पुष्टि करें</b>

    टोकन — ${ $symbol }
    मिंट — <code>{ $mint }</code>
    राशि — { $amount } { -sol }

    <i>निष्पादित करने के लिए 30 सेकंड के भीतर पुष्टि करें।</i>
telegram-token-confirm-blacklist =
    <b>टोकन ब्लैकलिस्ट करें?</b>

    टोकन — ${ $symbol }
    मिंट — <code>{ $mint }</code>

    <i>इससे यह टोकन फ़िल्टर पास करने से रुक जाएगा।</i>
telegram-token-blacklisted =
    <b>टोकन ब्लैकलिस्ट हुआ</b>

    टोकन — ${ $symbol }
    स्थिति — ब्लैकलिस्ट में जोड़ा गया
telegram-token-blacklist-failed = <b>ब्लैकलिस्ट विफल रहा</b>
telegram-token-buy-processing =
    <b>खरीद प्रोसेस हो रही है...</b>

    टोकन — ${ $symbol }
    राशि — { $amount } { -sol }
telegram-token-buy-done =
    <b>खरीद सफल रही</b>

    टोकन — ${ $symbol }
    राशि — { $amount } { -sol }

    <i>विवरण /positions में देखें</i>
telegram-token-buy-failed =
    <b>खरीद विफल रही</b>

    टोकन — ${ $symbol }
    त्रुटि — { $detail }
