# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = مُنشأة
wallets-type-imported = مستوردة
wallets-type-migrated = منقولة

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = أوقفتها مؤقتًا
wallets-watch-disabled-unknown = متوقفة مؤقتًا: تعذّرت قراءة سبب أمان المراقبة المحفوظ
wallets-watch-disabled-helius-unavailable = متوقفة مؤقتًا: مزوّد النشاط المرتفع غير متاح، وتم حفظ المؤشر
wallets-watch-disabled-processing-failed = متوقفة مؤقتًا: تعذّرت معالجة نشاط المحفظة، وتم حفظ المؤشر

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = مزوّد النشاط المرتفع غير متاح، وتوقفت المراقبة مؤقتًا
wallets-watch-error-provider-repeated-failure = تكرر فشل فحوصات { -helius }، وتوقفت المراقبة مؤقتًا
wallets-watch-error-processing-repeated-failure = تكرر فشل معالجة نشاط المحفظة، وتوقفت المراقبة مؤقتًا
wallets-watch-error-position-unreadable = تعذّرت على مراقبة المحفظة قراءة موضعها المحفوظ، وتجري إعادة المحاولة
wallets-watch-error-provider-check-failed = فشل فحص مزوّد النشاط المرتفع، وتجري إعادة المحاولة
wallets-watch-error-decode-failed = تعذّر فك ترميز معاملة النشاط المرتفع، وتم الاحتفاظ بالمؤشر
wallets-watch-error-processing-failed = تعذّرت معالجة نشاط المحفظة، وتجري إعادة المحاولة
wallets-watch-error-position-save-failed = تعذّر على مراقبة المحفظة حفظ موضعها، وتجري إعادة المحاولة

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = أوقفتها مؤقتًا.
wallets-watch-reason-helius-unavailable = فشلت فحوصات { -helius }. التقدم المحفوظ باقٍ.
wallets-watch-reason-processing-failed = تعذّرت معالجة نشاط المحفظة. التقدم المحفوظ باقٍ.
wallets-watch-reason-short-user = بواسطتك
wallets-watch-reason-short-helius-unavailable = مزوّد المراقبة
wallets-watch-reason-short-processing-failed = معالجة المراقبة
wallets-watch-state-reason = { $state } · { $reason }

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = إغلاق النافذة
wallets-summary-native = { -sol }
wallets-copied-private-key = المفتاح الخاص

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = المحفظة الرئيسية
wallets-tab-other = محافظ أخرى
wallets-tab-watched = المراقبة
wallets-refresh-failed = تعذّر تحديث المحافظ
wallets-action-failed = فشل
wallets-toast-failed = فشل: { $reason }
wallets-export-busy = جارٍ فك التشفير...
wallets-export-revealed = تم إظهار المفتاح، تعامل معه بحذر

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = مراقبة محفظة
wallets-watch-add-address = عنوان المحفظة
wallets-watch-add-address-input =
    .placeholder = عنوان Solana
wallets-watch-add-address-hint = يسجل نشاط المحفظة على السلسلة ويرسل تنبيهات الصفقات عبر إعدادات { -telegram } لديك.
wallets-watch-add-label = التسمية
wallets-watch-add-label-input =
    .placeholder = اسم اختياري
wallets-watch-add-submit = إضافة مراقبة

# wallets.html and watched.js: watch options dialog.
wallets-watch-options-title = خيارات مراقبة المحفظة
wallets-watch-options-close =
    .aria-label = إغلاق
wallets-watch-helius-allow = السماح باللحاق عبر { -helius } عند الحاجة
wallets-watch-helius-stop = إيقاف اللحاق عبر { -helius } لهذه المحفظة
wallets-watch-helius-description-approved = اللحاق عبر { -helius } مسموح لهذه المحفظة. إيقافه يعيد الفحوصات القياسية، وقد تتأخر في محفظة كثيرة النشاط.
wallets-watch-helius-description-available = يمكن لـ { -helius } فحص معاملات Solana الناجحة من الموضع المحفوظ دون تخطي الفترة غير المفحوصة. قد يستهلك رصيدًا أكبر من المزوّد وقد يتأخر أيضًا.
wallets-watch-helius-description-unavailable = اللحاق عبر { -helius } غير متاح. اضبط نقطة اتصال RPC مفعّلة لـ { -helius } لاستخدامه.
wallets-watch-helius-allow-title = السماح باللحاق عبر { -helius } لهذه المحفظة
wallets-watch-helius-allow-message = يمكن لـ { -helius } فحص معاملات Solana الناجحة من الموضع المحفوظ دون تخطي الفترة غير المفحوصة. يحتسب حاليًا 10 أرصدة لكل 100 معاملة كاملة مُعادة مع التقريب للأعلى، وبحد أدنى 10 أرصدة لكل طلب. قد ينفّذ الفحص الواحد عدة طلبات، وقد يختلف الاستهلاك وتسعير المزوّد. تبقى مهام النسخ متوقفة مؤقتًا حتى تُستأنف بشكل منفصل.
wallets-watch-helius-allow-confirm = السماح لهذه المحفظة
wallets-watch-helius-stop-message = ستعود هذه المحفظة إلى الفحوصات القياسية. قد تبلغ المحفظة كثيرة النشاط حد المراقبة وتتوقف مجددًا. لا تتغير المحافظ الأخرى ولا إعدادات { -helius } RPC لديك.
wallets-watch-helius-stop-confirm = الإيقاف لهذه المحفظة
wallets-watch-helius-stop-keep = إبقاء السماح
wallets-watch-helius-allowed = تم السماح باللحاق عبر { -helius } لهذه المحفظة عند الحاجة
wallets-watch-helius-stopped = تم إيقاف اللحاق عبر { -helius } لهذه المحفظة
wallets-watch-helius-update-failed = تعذّر تحديث إعداد اللحاق للمحفظة

# wallets.html: Export Private Key dialog.
wallets-export-title = تصدير المفتاح الخاص
wallets-export-warning-title = تحذير أمني حرج
wallets-export-warning-body = لا تشارك مفتاحك الخاص مع أي شخص. يستطيع أي شخص يملك هذا المفتاح سرقة كل الأموال من هذه المحفظة.
wallets-export-key-label = المفتاح الخاص (Base58)
wallets-export-copy =
    .title = نسخ إلى الحافظة
    .aria-label = نسخ إلى الحافظة
wallets-export-reveal = إظهار المفتاح

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = الرمز
wallets-holdings-col-balance = الرصيد
wallets-holdings-col-value = القيمة ({ -sol })
wallets-holdings-col-type = النوع
wallets-holdings-col-decimals = الخانات العشرية
wallets-holdings-empty-title = لا توجد حيازات رموز
wallets-holdings-unreadable-title = الحيازات غير معروفة
wallets-holdings-empty-message = ستظهر هنا الرموز التي تحتفظ بها هذه المحفظة.
wallets-funding-title = موّل محفظة التداول الخاصة بك
wallets-funding-message = لا تحتوي هذه المحفظة على أي { -sol }. أرسل { -sol } إلى هذا العنوان من منصة تداول أو محفظة أخرى؛ كل عملية شراء تنفق مبلغها مع رسوم الشبكة.
wallets-holdings-no-main = لا توجد محفظة رئيسية
wallets-holdings-main-title = المحفظة الرئيسية
wallets-holdings-tokens = الرموز
wallets-holdings-last-used = آخر استخدام
wallets-holdings-never = أبدًا
wallets-holdings-search =
    .placeholder = البحث بالرمز أو عنوان الإصدار...
wallets-holdings-export = تصدير المفتاح
wallets-holdings-export-tooltip = تصدير المفتاح الخاص لهذه المحفظة
wallets-holdings-replace-tooltip = تداول بمحفظة أخرى؛ يُحتفظ بمفتاح هذه المحفظة وسجلها
wallets-list-col-name = الاسم
wallets-list-col-balance = الرصيد ({ -sol })
wallets-list-col-type = النوع
wallets-list-col-created = الإنشاء
wallets-list-col-replaced = الاستبدال
wallets-list-replaced-on = في { $date }
wallets-replace-abandoned-title = أُلغي استبدال المحفظة
wallets-replace-abandoned-open-positions = كانت لدى محفظة التداول مراكز مفتوحة ({ $count }) عند إعادة التشغيل، لذا أُلغي الاستبدال. لم تتغير محفظة التداول؛ ويُحتفظ بالمفتاح الجديد ضمن محافظ أخرى.
wallets-replace-abandoned-positions-unreadable = تعذّرت قراءة مخزن المراكز عند إعادة التشغيل، لذا أُلغي الاستبدال. لم تتغير محفظة التداول؛ ويُحتفظ بالمفتاح الجديد ضمن محافظ أخرى.
wallets-replace-abandoned-vault-changed = لم تعد خزنة المحافظ مطابقة للاستبدال المُجهَّز عند إعادة التشغيل، لذا أُلغي الاستبدال. لم تتغير محفظة التداول؛ ويُحتفظ بالمفتاح الجديد ضمن محافظ أخرى.
wallets-list-col-actions = الإجراءات
wallets-list-action-export = تصدير المفتاح الخاص
wallets-list-count = المحافظ
wallets-list-search =
    .placeholder = البحث بالاسم أو العنوان...
wallets-list-loading-title = جارٍ تحميل المحافظ…
wallets-list-loading-description = جارٍ تجهيز عرض المحفظة المحددة.
wallets-list-kept-empty-message = تظهر هنا محافظ التداول المستبدلة والمحافظ من الإصدارات السابقة لتتمكن من تصدير مفاتيحها.
wallets-list-previous-trading-wallet = محفظة التداول السابقة
wallets-other-empty-title = لا توجد محافظ أخرى

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = المحفظة
wallets-watched-col-status = الحالة
wallets-watched-col-progress = التقدم المحفوظ
wallets-watched-col-last-check = آخر فحص
wallets-watched-unlabelled = محفظة بلا تسمية
wallets-watched-generic-name = محفظة
wallets-watched-not-synced = لم تتم المزامنة بعد
wallets-watched-not-checked = لم يتم الفحص بعد
wallets-watched-action-copy = نسخ الصفقات
    .title = فتح هذه المحفظة في نسخ التداول
wallets-watched-action-options = خيارات المراقبة
wallets-watched-action-retry = إعادة محاولة المراقبة
wallets-watched-action-pause = إيقاف مؤقت
wallets-watched-action-enable = تفعيل
wallets-watched-action-remove =
    .title = إزالة
    .aria-label = إزالة { $name }
wallets-watch-state-paused = متوقفة مؤقتًا
wallets-watch-state-catching-up = تلحق بالتحديثات
wallets-watch-state-watching = تحت المراقبة
wallets-watch-state-streaming = بث مباشر
wallets-watch-state-polling = استعلام دوري
wallets-watched-detail-helius = يتم الفحص عبر { -helius } لهذه المحفظة.
wallets-watched-detail-skipped = تم التقدّم إلى الأحدث عند { $time } للبقاء محدّثًا؛ لم يُقرأ النشاط الأقدم.
wallets-watched-detail-no-stream = لا يوجد بث مباشر من نقاط RPC لديك، لذا تُفحص هذه المحفظة دوريًا ويصل نشاطها متأخرًا. نقطة { -helius } أو { -drpc } تبثّه فورًا.
wallets-watched-empty-title = لا توجد عناوين مراقبة
wallets-watched-empty-message = استخدم «مراقبة محفظة» لتسجيل نشاط محفظة عامة على السلسلة.
wallets-watched-count = المراقبة
wallets-watched-search =
    .placeholder = البحث في المحافظ المراقبة...
wallets-watched-add = مراقبة محفظة
wallets-watched-refresh = تحديث المحافظ المراقبة
wallets-watched-loading-title = جارٍ تحميل المحافظ المراقبة...
wallets-watched-loading-description = جارٍ جلب أهداف المراقبة.
wallets-watched-load-error-title = تعذّر تحميل عناوين المراقبة
wallets-watched-load-error-description = استخدم التحديث للمحاولة مرة أخرى.
wallets-watched-address-invalid = أدخل عنوان محفظة Solana صالحًا.
wallets-watched-added = تمت إضافة مراقبة المحفظة
wallets-watched-duplicate = هذه المحفظة قيد المراقبة بالفعل.
wallets-watched-add-failed = تعذّرت إضافة مراقبة المحفظة.
wallets-watched-retried = تمت استعادة مراقبة المحفظة بمؤشرها المحفوظ
wallets-watched-paused = تم إيقاف مراقبة المحفظة مؤقتًا
wallets-watched-enabled = تم تفعيل مراقبة المحفظة
wallets-watched-removed = تمت إزالة مراقبة المحفظة
wallets-watched-update-failed = تعذّر تحديث مراقبة المحفظة
wallets-access-gate-title = المحافظ تتطلب الإعداد
