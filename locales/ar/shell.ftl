# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = فتح الصفحة الرئيسية للوحة التحكم
    .title = الصفحة الرئيسية للوحة التحكم
shell-bot-card =
    .aria-label = جارٍ تحميل حالة المتداول الآلي
shell-bot-label = آلي
shell-bot-status-loading = جارٍ التحميل
shell-bot-today = اليوم
shell-explore-control =
    .aria-label = وضع الاستكشاف. اربط محفظة ونقطة اتصال RPC لتفعيل كل الميزات
    .title = اربط محفظة ونقطة اتصال RPC لتفعيل التداول والأرصدة وبيانات السلسلة الحية
shell-explore-title = وضع الاستكشاف
shell-explore-detail = المحفظة وRPC غير متصلين
shell-explore-action = إكمال الإعداد
shell-wallet-card =
    .aria-label = قيمة المحفظة؛ فتح المراكز
    .title = قيمة المحفظة ({ -sol } + الرموز) · فتح المراكز
shell-wallet-worth-label = القيمة
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = الرموز
shell-sol-price-card =
    .aria-label = سعر { -sol } بالدولار، فتح المخطط
    .title = سعر { -sol } · انقر لعرض المخطط
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = نسخ التداول؛ فتح نسخ التداول
    .title = نسخ التداول · فتح نسخ التداول
shell-copy-label = نسخ التداول
shell-actions-more =
    .aria-label = مزيد من إجراءات الترويسة
    .title = مزيد من الإجراءات
shell-actions-group =
    .aria-label = إجراءات الترويسة
shell-action-search =
    .aria-label = البحث عن الرموز
    .title = البحث عن الرموز (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = الرموز المميّزة
    .title = الرموز المميّزة
shell-action-notifications =
    .aria-label = الإجراءات والإشعارات
    .title = الإجراءات والإشعارات
shell-action-restart =
    .aria-label = إعادة تشغيل التطبيق
    .title = إعادة تشغيل التطبيق
shell-action-theme =
    .aria-label = تبديل السمة
    .title = تبديل السمة
shell-action-settings =
    .aria-label = الإعدادات
    .title = الإعدادات

## Ticker

shell-ticker-monitoring-segment =
    .title = الرموز التي تراقبها خدمة مجمعات السيولة
shell-ticker-monitoring = تحت المراقبة:
shell-ticker-filtering-segment =
    .title = الرموز التي اجتازت معايير الترشيح أو رُفضت
shell-ticker-passed = الناجحة:
shell-ticker-rejected = المرفوضة:
shell-ticker-pnl-segment =
    .title = الأرباح والخسائر المحققة اليوم
shell-ticker-pnl = أرباح وخسائر اليوم:
shell-ticker-rpc-segment =
    .title = استدعاءات RPC في الدقيقة ونسبة النجاح
shell-ticker-rpc = RPC:
shell-ticker-rpc-per-minute = /د
shell-ticker-services-segment =
    .title = حالة سلامة خدمات الخلفية
shell-ticker-services-loading = الخدمات: <strong>جارٍ التحميل</strong>

## Notification drawer

shell-notification-title = الإجراءات
shell-notification-mark-all-read =
    .title = تحديد الكل كمقروء
shell-notification-clear-all =
    .title = مسح الكل
shell-notification-close =
    .aria-label = إغلاق
shell-notification-tab-all = الكل
shell-notification-tab-active = النشطة
shell-notification-tab-done = المكتملة
shell-notification-tab-failed = الفاشلة
shell-notification-filter-type-all = كل الأنواع
shell-notification-filter-type-buy = شراء
shell-notification-filter-type-sell = بيع
shell-notification-filter-type-open = فتح
shell-notification-filter-type-close = إغلاق
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = جزئي
shell-notification-filter-state-all = كل الحالات
shell-notification-filter-state-in-progress = قيد التنفيذ
shell-notification-filter-state-completed = مكتمل
shell-notification-filter-state-failed = فشل
shell-notification-filter-state-cancelled = ملغى
shell-notification-list =
    .aria-label = الإشعارات
shell-notification-empty = لا توجد إجراءات بعد
shell-notification-loading-more = جارٍ تحميل المزيد...
shell-notification-back-to-top =
    .title = العودة إلى الأعلى

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = التشغيل
shell-status-bar-memory = الذاكرة
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/دقيقة
shell-status-bar-trading = التداول
shell-status-bar-positions = المراكز
shell-status-bar-tokens = الرموز

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = جارٍ تشغيل { -brand }
shell-splash-waiting = بانتظار استجابة النواة المحلية.
shell-splash-failed = تعذّر تشغيل { -brand }
shell-splash-failed-detail = تحقق من ملف السجل، ثم أعد تشغيل التطبيق.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = النواة متصلة
shell-connection-waiting = بانتظار النواة…
shell-connection-retry-now = إعادة المحاولة الآن
shell-connection-overlay-detail = تعذّر الوصول إلى النواة. التداول متوقف مؤقتًا، وسيعود تلقائيًا.
shell-connection-restored = تمت استعادة الاتصال بالنواة

# Source: scripts/core/header.js
shell-trader-control-failed = فشل التحكم بالمتداول
shell-notification-button-unread = الإجراءات والإشعارات، غير المقروءة: { $count }
shell-restart-confirm-title = إعادة تشغيل البوت
shell-restart-confirm-message =
    هل تريد بالتأكيد إعادة تشغيل البوت؟

    سيؤدي ذلك إلى:
    • إيقاف كل الخدمات
    • إعادة تشغيل العملية
    • الاستغراق نحو 10-15 ثانية

    ستتم مقاطعة كل العمليات النشطة.
shell-restart-confirm-action = إعادة التشغيل
shell-restart-progress = جارٍ إعادة تشغيل البوت
shell-restart-failed = فشلت إعادة التشغيل
shell-restart-failed-status = فشلت إعادة التشغيل: { $status }
shell-restart-helper-unavailable = مساعد إعادة التشغيل التلقائي غير متاح. أعد تحميل لوحة التحكم بعد قليل.

# Source: scripts/core/router.js
shell-page-title-fallback = لوحة التحكم
shell-page-load-failed = فشل تحميل الصفحة
shell-page-offline-detail = تعذّر الوصول إلى النواة حاليًا. ستُحمَّل هذه الصفحة تلقائيًا عند عودة الاتصال.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = استكشاف
shell-bot-state-halted = متوقف
shell-bot-state-off = معطّل
shell-bot-state-waiting = بالانتظار
shell-bot-state-idle = خامل
shell-bot-state-entry-paused = الدخول متوقف مؤقتًا
shell-bot-state-running = قيد التشغيل
shell-bot-control-explore = المتداول الآلي غير متاح في وضع الاستكشاف. افتح إعداد المحفظة وRPC.
shell-bot-control-halted = الإيقاف الطارئ نشط. افتح عناصر التحكم بالمتداول الآلي.
shell-bot-control-off = المتداول الآلي معطّل. انقر لتفعيله.
shell-bot-control-waiting = المتداول الآلي مفعّل ويتنظر خدمات النواة. انقر لتعطيله.
shell-bot-control-idle = المتداول الآلي مفعّل، لكن المراقبين معطّلان. افتح عناصر التحكم بالمتداول الآلي.
shell-bot-control-entry-paused = أوقفت حماية الخسارة الدخولات مؤقتًا، ويمكن أن تستمر عمليات الخروج. افتح عناصر التحكم بالمتداول الآلي.
shell-bot-control-running = المتداول الآلي قيد التشغيل. انقر لتعطيله.

## Wallet and copy cards

shell-wallet-card-summary = قيمة المحفظة: { $equity } { -sol } (النقد { $balance } { -sol }، الرموز { $tokens }); فتح المراكز
shell-copy-running-live = مباشر: { $count }
shell-copy-running-paper = تجريبي: { $count }
shell-copy-value-paused = متوقف مؤقتًا
shell-copy-value-idle = خامل
shell-copy-sub-active = { $active } من { $total } نشطة

## Ticker services state

shell-ticker-services-healthy = الخدمات: <strong>سليمة</strong>
shell-ticker-services-issues =
    { $count ->
        [zero] الخدمات: <strong>{ $count } مشكلة</strong>
        [one] الخدمات: <strong>{ $count } مشكلة</strong>
        [two] الخدمات: <strong>{ $count } مشكلتان</strong>
        [few] الخدمات: <strong>{ $count } مشكلات</strong>
        [many] الخدمات: <strong>{ $count } مشكلة</strong>
       *[other] الخدمات: <strong>{ $count } مشكلة</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = طلب من وكيل
shell-agent-request-client-fallback = وكيل مقترن
shell-agent-request-message = يريد { $client } تشغيل «{ $tool }» في { -brand }. ينتهي هذا الطلب { $expiry }.
shell-agent-request-message-arguments = يريد { $client } تشغيل «{ $tool }» في { -brand }. المعطيات: { $summary }. ينتهي هذا الطلب { $expiry }.
shell-agent-request-expires-minutes = خلال { $minutes } د
shell-agent-request-expires-seconds = خلال { $seconds } ث
shell-agent-request-approve = موافقة
shell-agent-request-deny = رفض

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = تم نسخ { $label }
shell-toast-copy-failed = فشل النسخ
shell-toast-still-running = لا يزال قيد التشغيل، تحقق من مركز الإشعارات
shell-toast-dismiss =
    .aria-label = إغلاق
shell-confirm-title = تأكيد الإجراء
shell-confirm-message = هل أنت متأكد؟
shell-address-open-solscan = — فتح في { -solscan }
shell-address-copy = نسخ العنوان

# Source: scripts/core/global_chat.js
shell-assistant-label = المساعد
shell-assistant-dialog =
    .aria-label = المساعد

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = نشط
shell-status-bar-trading-inactive = غير نشط

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = تم إلغاء { $title }
shell-action-swap-buy-live = جارٍ الشراء
shell-action-swap-buy-done = تم الشراء
shell-action-swap-buy-failed = فشل الشراء
shell-action-swap-sell-live = جارٍ البيع
shell-action-swap-sell-done = تم البيع
shell-action-swap-sell-failed = فشل البيع
shell-action-position-open-live = جارٍ فتح المركز
shell-action-position-open-done = تم الفتح
shell-action-position-open-failed = فشل الفتح
shell-action-position-close-live = جارٍ إغلاق المركز
shell-action-position-close-done = تم الإغلاق
shell-action-position-close-failed = فشل الإغلاق
shell-action-position-dca-live = جارٍ الإضافة إلى المركز
shell-action-position-dca-done = تمت الإضافة إلى
shell-action-position-dca-failed = فشلت الإضافة
shell-action-partial-exit-live = خروج جزئي
shell-action-partial-exit-done = خروج جزئي
shell-action-partial-exit-failed = فشل الخروج الجزئي
shell-action-manual-order-live = جارٍ تنفيذ الأمر
shell-action-manual-order-done = تم تنفيذ الأمر
shell-action-manual-order-failed = فشل الأمر
shell-action-trade-live = صفقة
shell-action-trade-done = تمت الصفقة
shell-action-trade-failed = فشلت الصفقة

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = هل تريد إغلاق { -brand }؟
shell-exit-description = اختر طريقة إغلاق التطبيق
shell-exit-minimize = تصغير إلى علبة النظام
shell-exit-minimize-detail = الاستمرار في العمل في الخلفية
shell-exit-quit = الخروج من التطبيق
shell-exit-quit-detail = الإغلاق الكامل وإيقاف كل الخدمات

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = حفظ الصورة
shell-lightbox-close =
    .title = إغلاق (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = فاتح
shell-theme-dark = داكن
shell-theme-switch-to-light = التبديل إلى السمة الفاتحة
shell-theme-switch-to-dark = التبديل إلى السمة الداكنة
