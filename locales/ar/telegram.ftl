# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = الحالة
telegram-reply-balance = الرصيد
telegram-reply-positions = المراكز
telegram-reply-pause = إيقاف مؤقت
telegram-reply-resume = استئناف
telegram-reply-stop = إيقاف
telegram-reply-stats = الإحصاءات
telegram-reply-menu = القائمة
telegram-reply-help = المساعدة

## Inline keyboard buttons.

telegram-button-positions = المراكز
telegram-button-balance = الرصيد
telegram-button-stats = الإحصاءات
telegram-button-tokens = الرموز
telegram-button-pause = إيقاف مؤقت
telegram-button-stop = إيقاف
telegram-button-settings = الإعدادات
telegram-button-refresh = تحديث
telegram-button-menu = القائمة
telegram-button-back = رجوع
telegram-button-back-to-menu = العودة إلى القائمة
telegram-button-back-to-tokens = العودة إلى الرموز
telegram-button-cancel = إلغاء
telegram-button-close-all-positions = إغلاق جميع المراكز
telegram-button-sell-percent = بيع { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = القائمة السوداء
telegram-button-blacklist-symbol = إضافة { $symbol } إلى القائمة السوداء
telegram-button-close-position = إغلاق المركز
telegram-button-confirm-close = تأكيد الإغلاق
telegram-button-confirm-close-all = إغلاق كل المراكز
telegram-button-confirm-sell = تأكيد بيع { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = تأكيد الإيقاف القسري
telegram-button-confirm-buy = شراء { $amount } { -sol }
telegram-button-notifications = الإشعارات
telegram-button-trading = التداول
telegram-button-entry-monitor = مراقب الدخول
telegram-button-exit-monitor = مراقب الخروج
telegram-button-auto-trading = التداول الآلي
telegram-button-force-stop = إيقاف قسري
telegram-button-notify-opened = الفتح
telegram-button-notify-closed = الإغلاق
telegram-button-notify-partial = الجزئي
telegram-button-notify-dca = DCA
telegram-button-notify-errors = الأخطاء
telegram-button-details = التفاصيل
telegram-button-position = المركز
telegram-button-sell-more = بيع المزيد
telegram-button-more-dca = مزيد من DCA
telegram-button-history = السجل
telegram-button-status = الحالة
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = إعادة المصادقة
telegram-button-previous = السابق
telegram-button-next = التالي
telegram-button-passed = الناجحة
telegram-button-rejected = المرفوضة
telegram-button-new-24h = الجديدة (24h)
telegram-button-all-tokens = جميع الرموز
telegram-button-search-token = البحث عن رمز
telegram-button-filter-stats = إحصاءات الترشيح
telegram-button-refresh-stats = تحديث الإحصاءات
telegram-button-view-position = عرض المركز
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    أمر غير معروف: { $command }

    استخدم /help لعرض الأوامر المتاحة.
telegram-session-expired =
    <b>انتهت الجلسة</b>

    استخدم /login لإعادة المصادقة.
telegram-2fa-required =
    <b>المصادقة الثنائية مطلوبة</b>

    يرجى إدخال رمز المصادقة المكوَّن من 6 أرقام.
telegram-account-locked =
    <b>الحساب مقفل</b>

    محاولات فاشلة كثيرة جدًا.
    حاول مرة أخرى بعد { $seconds ->
        [zero] { $seconds } ثانية.
        [one] { $seconds } ثانية.
        [two] { $seconds } ثانيتين.
        [few] { $seconds } ثوانٍ.
        [many] { $seconds } ثانية.
       *[other] { $seconds } ثانية.
    }
telegram-code-invalid = يرجى إدخال رمز صالح مكوَّن من 6 أرقام.
telegram-authenticated =
    <b>تمت المصادقة!</b>

    يمكنك الآن استخدام أوامر البوت.
telegram-wrong-code =
    <b>الرمز خاطئ</b>

    المحاولات المتبقية: { $remaining }
telegram-auth-required =
    <b>المصادقة مطلوبة</b>

    يرجى إدخال كلمة المرور للمتابعة.

    <i>اكتب كلمة المرور وأرسلها.</i>
telegram-login-required =
    <b>تسجيل الدخول مطلوب</b>

    يرجى إدخال رمز المصادقة المكوَّن من 6 أرقام:
telegram-session-activated =
    <b>تم تفعيل الجلسة</b>

    المصادقة الثنائية غير مهيأة. جلستك نشطة الآن.

    <i>نصيحة: فعّل المصادقة الثنائية من إعدادات الأمان لمزيد من الحماية.</i>

## Chat discovery.

telegram-discovery-hello = مرحبًا { $name }!
telegram-discovery-default-name = مستخدم
telegram-discovery-detected = <b>تم اكتشاف الدردشة!</b>
telegram-discovery-details =
    معرّف الدردشة: <code>{ $chat_id }</code>
    النوع: { $chat_type }

    يرجى الانتقال إلى لوحة تحكم { -brand } والنقر على هذه الدردشة لاختيارها.
telegram-chat-type-private = خاصة
telegram-chat-type-group = مجموعة
telegram-chat-type-supergroup = مجموعة خارقة
telegram-chat-type-channel = قناة

## Menus.

telegram-menu-title =
    <b>لوحة التحكم</b>

    اختر خيارًا لعرض المعلومات أو التحكم في البوت.
telegram-menu-positions-empty =
    <b>لا توجد مراكز مفتوحة</b>

    بانتظار فرص جديدة...
telegram-menu-positions-title = <b>المراكز ({ $count })</b>
telegram-menu-positions-hint = <i>اضغط على مركز لإدارته.</i>
telegram-menu-settings =
    <b>الإعدادات</b>

    اضبط الإشعارات ومعاملات التداول.
telegram-settings-notifications =
    <b>إعدادات الإشعارات</b>

    فعّل الإشعارات أو عطّلها:
telegram-settings-trading =
    <b>ضوابط التداول</b>

    فعّل ميزات التداول أو عطّلها:
telegram-pagination-expired = انتهت جلسة التنقل بين الصفحات.

## Status commands.

telegram-status-state-stopped = <b>متوقف</b> (الإيقاف القسري نشط)
telegram-status-state-active = <b>نشط</b>
telegram-status-state-paused = <b>متوقف مؤقتًا</b>
telegram-status-on = مفعّل
telegram-status-off = معطّل
telegram-status-body =
    <b>حالة النظام</b>

    <b>النظام</b>
    الحالة — { $state }
    مدة التشغيل — { $uptime }
    الإصدار — v{ $version }

    <b>التداول</b>
    الدخول — { $entries }
    الخروج — { $exits }
    المراكز — { $positions }
telegram-positions-empty =
    <b>لا توجد مراكز مفتوحة</b>

    بانتظار الفرص...
telegram-positions-title = <b>المراكز المفتوحة ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } أخرى...</i>
telegram-positions-summary =
    <b>ملخص المحفظة الاستثمارية</b>
    المستثمر — { $invested } { -sol }
    صافي الأرباح والخسائر (P{ "&amp;" }L) — { $pnl } { -sol }
telegram-balance-body =
    <b>رصيد المحفظة</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>الإحصاءات اليومية</b>

    المراكز — { $positions }
    المستثمر — { $invested } { -sol }
    الأرباح والخسائر (P{ "&amp;" }L) — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } جاهز!</b>

    التداول <b>مفعّل</b>.

    استخدم لوحة المفاتيح أدناه للتحكم في البوت.
    اكتب /help لعرض الأوامر المتاحة.
telegram-stop-already = <b>التداول معطّل بالفعل</b>
telegram-stop-done =
    <b>تم تعطيل التداول</b>

    تم إيقاف جميع مراقبات التداول (الدخول { "&amp;" } الخروج).
    استخدم /pause لإيقاف الدخول فقط.
telegram-stop-failed =
    <b>تعذّر تعطيل التداول</b>

    الخطأ: { $detail }
telegram-pause-done =
    <b>تم إيقاف مراقب الدخول مؤقتًا</b>

    لن تُفتح مراكز جديدة.
    يواصل مراقب الخروج عمله.
telegram-pause-failed =
    <b>تعذّر إيقاف الدخول مؤقتًا</b>

    الخطأ: { $detail }
telegram-resume-done =
    <b>تم استئناف مراقب الدخول</b>

    يراقب الآن إشارات الدخول.
telegram-resume-failed =
    <b>تعذّر استئناف الدخول</b>

    الخطأ: { $detail }
telegram-force-stop-confirm =
    <b>إيقاف قسري</b>

    سيؤدي هذا إلى إيقاف كل نشاط التداول فورًا:
    • لا دخول جديد
    • لا خروج (بما في ذلك وقف الخسارة)
    • لا عمليات DCA
telegram-force-stop-warning = <b>هذا إجراء طارئ!</b>
telegram-force-stop-question = هل أنت متأكد؟
telegram-force-stop-active =
    <b>تم تفعيل الإيقاف القسري</b>

    تم إيقاف كل التداول.

    استخدم /resume_trading لإلغاء هذه الحالة.
telegram-resume-trading-not-stopped =
    <b>التداول غير متوقف قسريًا</b>

    لا حاجة لأي إجراء.
telegram-resume-trading-done =
    <b>تم استئناف التداول</b>

    تم إلغاء حالة الإيقاف القسري.
    يمكن الآن استئناف عمليات التداول الطبيعية.

## Help.

telegram-help-title = <b>مساعدة { -brand }</b>
telegram-help-heading-dashboard = لوحة التحكم
telegram-help-heading-market = السوق
telegram-help-heading-trading = التداول
telegram-help-heading-safety = الأمان
telegram-help-heading-system = النظام
telegram-help-commands-dashboard =
    /status — حالة النظام { "&amp;" } مدة التشغيل
    /stats — الأداء اليومي
    /balance — رصيد المحفظة
    /positions — المراكز المفتوحة
telegram-help-commands-market =
    /tokens — مستكشف الرموز
    /rejected — الرموز المرشَّحة
telegram-help-commands-trading =
    /start — تفعيل نظام التداول
    /stop — تعطيل نظام التداول
    /pause — إيقاف الدخول الجديد مؤقتًا
    /resume — استئناف الدخول الجديد
    /menu — القائمة التفاعلية
telegram-help-commands-safety =
    /force_stop — <b>إيقاف طارئ</b>
    /resume_trading — إلغاء حالة الطوارئ
telegram-help-commands-system =
    /update — حالة التحديث { "&amp;" } التثبيت
    /login — المصادقة الثنائية
telegram-help-tip = <i>نصيحة: اضغط على أمر لتشغيله.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>محدَّث</b>

    الإصدار الحالي v{ $version }، مثبّت تلقائيًا.
telegram-update-up-to-date =
    <b>محدَّث</b>

    الإصدار الحالي v{ $version }.
telegram-update-check-failed =
    <b>فشل التحقق من التحديثات</b>

    { $reason }
telegram-update-unreachable = تعذّر الوصول إلى screenerbot.io.
telegram-update-installing = <b>جارٍ تثبيت v{ $version }</b>
telegram-update-restarting =
    يُعاد تشغيل { -brand } على الإصدار الجديد. يُستأنف التداول تلقائيًا.
telegram-update-install-failed =
    <b>تعذّر تثبيت v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>تم تنزيل v{ $version }</b>

    يحدّث هذا الإصدار تطبيق سطح المكتب أيضًا، لذا يجب تشغيل مثبّته على الجهاز. افتح الإعدادات ← التحديثات هناك.
telegram-update-downloading =
    <b>جارٍ تنزيل v{ $version }</b>

    { $percent }% من { $size } MB.
telegram-update-available =
    <b>v{ $version } متاح</b>

    { $how }
    حجم التنزيل: { $size } MB.

    يُنزَّل من تلقاء نفسه؛ أرسل /update مرة أخرى عندما يصبح جاهزًا.
telegram-update-how-core = يُثبَّت بصمت مع إعادة تشغيل قصيرة.
telegram-update-how-installer = يتطلب تشغيل مثبّت سطح المكتب مرة واحدة.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = غير معروف
telegram-value-na = غير متاح
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }ث
telegram-duration-minutes = { $minutes }د
telegram-duration-minutes-seconds = { $minutes }د { $seconds }ث
telegram-duration-hours = { $hours }س
telegram-duration-hours-minutes = { $hours }س { $minutes }د
telegram-duration-days = { $days }ي
telegram-duration-days-hours = { $days }ي { $hours }س
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = الخطأ: { $detail }
telegram-ai-reasoning =
    <b>تحليل LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = الدخول — { $price } { -sol }
telegram-row-exit = الخروج — { $price } { -sol }
telegram-row-current = الحالي — { $price } { -sol }
telegram-row-invested = المستثمر — { $amount } { -sol }
telegram-row-received = المستلم — { $amount } { -sol }
telegram-row-value = القيمة — { $amount } { -sol }
telegram-row-total = الإجمالي — { $amount } { -sol }
telegram-row-tokens = الرموز — { $tokens }
telegram-row-duration = المدة — { $duration }
telegram-row-reason = السبب — { $reason }
telegram-row-remaining = المتبقي — { $percent }%
telegram-row-pnl = الأرباح والخسائر (P{ "&amp;" }L) — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>تم فتح مركز</b>
telegram-notify-opened-size = الحجم — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = السعر — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>تم إغلاق المركز</b> — ربح
telegram-notify-closed-title-loss = <b>تم إغلاق المركز</b> — خسارة
telegram-notify-closed-reason-unspecified = مغلق
telegram-notify-partial-title = <b>خروج جزئي</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — تم بيع { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = المضاف — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = المتوسط — { $price } { -sol }
telegram-notify-severity-critical = <b>خطأ حرج</b>
telegram-notify-severity-error = <b>خطأ</b>
telegram-notify-severity-warning = <b>تحذير</b>
telegram-notify-severity-info = <b>معلومة</b>
telegram-notify-alert-title = <b>تنبيه صفقة</b>
telegram-notify-alert-token = الرمز: <code>${ $symbol }</code>
telegram-notify-alert-mint = الإصدار: <code>{ $mint }</code>
telegram-notify-alert-bought = الإجراء: شراء { $amount } { -sol }
telegram-notify-alert-sold = الإجراء: بيع { $amount } { -sol }
telegram-notify-alert-wallet = المحفظة: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (تجريبي)
telegram-notify-copy-task = المهمة: { $task }
telegram-notify-scheduled-completed = <b>اكتملت المهمة المجدولة</b>
telegram-notify-scheduled-failed = <b>فشلت المهمة المجدولة</b>
telegram-notify-scheduled-timed-out = <b>انتهت مهلة المهمة المجدولة</b>
telegram-notify-scheduled-error = الخطأ: { $error }
telegram-notify-summary-title = <b>الملخص اليومي</b> — { $date }
telegram-notify-summary-performance = <b>الأداء</b>
telegram-notify-summary-trades = الصفقات — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = نسبة الربح — { $percent }%
telegram-notify-summary-pnl = الأرباح والخسائر (P{ "&amp;" }L) — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = المراكز المفتوحة — { $count }
telegram-notify-started-title = <b>تم تشغيل { -brand }</b>
telegram-notify-started-version = <b>الإصدار</b> — { $version }
telegram-notify-started-mode = <b>الوضع</b> — { $mode }
telegram-notify-started-ready = جاهز للتداول!
telegram-notify-stopped-title = <b>تم إيقاف { -brand }</b>
telegram-notify-stopped-reason = <b>السبب</b> — { $reason }
telegram-notify-stopped-goodbye = إلى اللقاء! { $icon }
telegram-notify-start-mode-normal = عادي
telegram-notify-stop-reason-graceful = إيقاف تشغيل سلس
telegram-notify-update-available =
    <b>التحديث v{ $version } متاح</b>

    { $how }
    حجم التنزيل: { $size } MB
telegram-notify-update-how-installer = يحدّث هذا الإصدار تطبيق سطح المكتب أيضًا، لذا يجب تشغيل مثبّته مرة واحدة.
telegram-notify-update-ready =
    <b>التحديث v{ $version } جاهز</b>

    { $how }
telegram-notify-update-ready-silent = أرسل /update لتطبيقه الآن، أو يُثبَّت عند تشغيل { -brand } في المرة التالية.
telegram-notify-update-ready-installer = افتح الإعدادات ← التحديثات لتشغيل المثبّت.
telegram-notify-update-applying =
    <b>جارٍ تثبيت v{ $version }</b>

    تُعاد تشغيل الواجهة الخلفية؛ يُستأنف التداول تلقائيًا.
telegram-notify-new-tokens =
    <b>تنبيه الترشيح</b>

    الرموز الجديدة المطابقة لمعاييرك: { $count }
telegram-notify-crash =
    <b>تعطّل البوت!</b>

    <b>الموقع:</b> <code>{ $location }</code>
    <b>الخطأ:</b> <code>{ $error }</code>
telegram-notify-crash-restart = يرجى إعادة تشغيل البوت.

## Filter results page.

telegram-filter-results-title = <b>نتائج الترشيح</b> ({ $count })
telegram-filter-results-empty = <i>لم يتم العثور على رموز.</i>
telegram-filter-results-page = <i>الصفحة { $page } من { $total }</i>

## Position screens.

telegram-position-not-found = لم يتم العثور على المركز
telegram-position-no-positions = لا توجد مراكز لإغلاقها
telegram-position-history-empty =
    <b>سجل الصفقات</b>

    لا توجد مراكز مغلقة بعد.
telegram-position-history-title = <b>الصفقات الأخيرة</b>
telegram-position-history-more = <i>+{ $count } صفقات أخرى...</i>
telegram-position-confirm-hint = <i>أكّد خلال 30ث للتنفيذ.</i>
telegram-position-confirm-close-title = <b>إغلاق المركز؟</b>
telegram-position-confirm-close-selling = بيع { $tokens } من الرموز
telegram-position-confirm-close-estimated = التقدير — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>أكّد خلال 30 ثانية</i>
telegram-position-confirm-sell =
    <b>تأكيد البيع</b>

    الرمز — { $symbol }
    المبلغ — { $percent }%
    الرموز — { $tokens }
telegram-position-confirm-dca =
    <b>تأكيد شراء المزيد</b>

    الرمز — { $symbol }
    الإضافة — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>إغلاق جميع المراكز؟</b>

    العدد — { $count }
telegram-position-confirm-close-all-hint =
    <i>سيؤدي هذا إلى بيع جميع المراكز المفتوحة بسعر السوق.
    أكّد خلال 30ث.</i>
telegram-position-confirm-force-stop =
    <b>إيقاف قسري</b>

    سيؤدي هذا إلى إيقاف كل التداول فورًا:
    • لا دخول جديد
    • لا خروج
    • لا DCA
telegram-position-confirm-force-stop-warning = <b>هذا إجراء طارئ.</b>
telegram-position-confirm-blacklist =
    <b>إضافة الرمز إلى القائمة السوداء؟</b>

    الرمز — { $symbol }
    الإصدار — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>سيؤدي هذا إلى إغلاق المركز ومنع الدخول مستقبلًا.</i>
telegram-position-selling = جارٍ بيع { $percent }% من { $symbol }...
telegram-position-sell-done =
    <b>تم تنفيذ البيع</b>

    الرمز — { $symbol }
    المباع — { $percent }%
    المستلم — { $amount } { -sol }
telegram-position-sell-failed = <b>فشل البيع</b>
telegram-position-adding = جارٍ إضافة { $amount } { -sol } إلى { $symbol }...
telegram-position-dca-done =
    <b>تم تنفيذ DCA</b>

    الرمز — { $symbol }
    المضاف — { $amount } { -sol }
telegram-position-dca-failed = <b>فشل DCA</b>
telegram-position-closing-all = جارٍ إغلاق جميع المراكز...
telegram-position-close-all-done =
    <b>اكتمل إغلاق الكل</b>

    المغلقة — { $closed }
    الفاشلة — { $failed }
telegram-position-blacklisted =
    <b>تمت إضافة الرمز إلى القائمة السوداء</b>

    الرمز — { $symbol }
    الحالة — مغلق { "&amp;" } في القائمة السوداء

## Token screens.

telegram-token-not-found = لم يتم العثور على الرمز
telegram-token-not-found-prefix = لم يتم العثور على الرمز. جرّب البحث ببادئة أطول.
telegram-token-stats-failed = تعذّر جلب الإحصاءات: { $detail }
telegram-token-list-failed = تعذّر جلب الرموز: { $detail }
telegram-token-list-empty = لم يتم العثور على رموز في عرض <b>{ $view }</b>.
telegram-token-view-passed = الناجحة في الترشيح
telegram-token-view-rejected = المرفوضة
telegram-token-view-recent = المضافة حديثًا
telegram-token-view-all = جميع الرموز
telegram-token-list-title = <b>{ $name }</b> (الصفحة { $page }/{ $total })
telegram-token-list-stats = السيولة: { $liquidity } • السعر: { $price }
telegram-token-list-hint = <i>اضغط /token_ID لعرض التفاصيل</i>
telegram-token-explorer =
    <b>مستكشف السوق</b>

    <b>نظرة عامة</b>
    الناجحة في الترشيح — { $passed }
    المرفوضة — { $rejected }
    الأسعار النشطة — { $priced }
    إجمالي المكتشفة — { $total }

    <i>اختر فئة للتصفح:</i>
telegram-token-filter-title = <b>تحليل الترشيح</b>
telegram-token-filter-distribution = <b>التوزيع</b>
telegram-token-filter-passed = الناجحة — { $count } ({ $percent }%)
telegram-token-filter-rejected = المرفوضة — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = في القائمة السوداء — { $count }
telegram-token-filter-coverage = <b>التغطية</b>
telegram-token-filter-priced = بسعر مجمع السيولة — { $count }
telegram-token-filter-open = المراكز المفتوحة — { $count }
telegram-token-filter-total = إجمالي المكتشفة — { $count }
telegram-token-filter-updated = <b>آخر تحديث</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>يُحدَّث تلقائيًا كل { $interval }</i>
telegram-token-detail-active = <b>مركز نشط</b>
telegram-token-detail-price = السعر — { $price } { -sol }
telegram-token-detail-liquidity = السيولة — { $value }
telegram-token-detail-volume = حجم التداول (24h) — { $value }
telegram-token-detail-change = التغيّر (24h) — { $value }
telegram-token-detail-risk = تقييم المخاطر: { $score }/100
telegram-token-detail-risk-unknown = تقييم المخاطر: غير معروف
telegram-token-detail-action = <i>اختر إجراءً:</i>
telegram-token-search =
    <b>البحث في السوق</b>

    أدخل الرمز المختصر أو عنوان الإصدار للبحث:

    <i>مثال: /token_BONK أو /token_So11111</i>
telegram-token-confirm-buy =
    <b>تأكيد الشراء المباشر</b>

    الرمز — ${ $symbol }
    الإصدار — <code>{ $mint }</code>
    المبلغ — { $amount } { -sol }

    <i>أكّد خلال 30ث للتنفيذ.</i>
telegram-token-confirm-blacklist =
    <b>إضافة الرمز إلى القائمة السوداء؟</b>

    الرمز — ${ $symbol }
    الإصدار — <code>{ $mint }</code>

    <i>سيمنع هذا الرمز من استيفاء المرشحات.</i>
telegram-token-blacklisted =
    <b>تمت إضافة الرمز إلى القائمة السوداء</b>

    الرمز — ${ $symbol }
    الحالة — أُضيف إلى القائمة السوداء
telegram-token-blacklist-failed = <b>فشلت الإضافة إلى القائمة السوداء</b>
telegram-token-buy-processing =
    <b>جارٍ معالجة الشراء...</b>

    الرمز — ${ $symbol }
    المبلغ — { $amount } { -sol }
telegram-token-buy-done =
    <b>نجح الشراء</b>

    الرمز — ${ $symbol }
    المبلغ — { $amount } { -sol }

    <i>اعرض التفاصيل في /positions</i>
telegram-token-buy-failed =
    <b>فشل الشراء</b>

    الرمز — ${ $symbol }
    الخطأ — { $detail }
