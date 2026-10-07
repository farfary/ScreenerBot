# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = حدث OHLCV: { $subtype }
events-filtering-default = حدث الترشيح: { $subtype }
events-trader-default = حدث المتداول: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = اكتملت المهمة «{ $name }»
events-task-failed = فشلت المهمة «{ $name }»
events-task-timed-out = انتهت مهلة المهمة «{ $name }»

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = اكتملت المهمة
events-subtype-task-failed = فشلت المهمة
events-subtype-task-timed-out = انتهت مهلة المهمة

# Shown when an event carries no display text.
events-message-none = لا توجد رسالة

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = فشل تنظيف ذاكرة OHLCV المؤقتة
events-ohlcv-gap-cleanup-failed = فشل تنظيف سجلات الفجوات المملوءة
events-ohlcv-gap-fill-failed = خطأ في ملء الفجوة لـ { $mint }
events-ohlcv-backfill-scheduled = تمت جدولة ملء البيانات السابقة متعدد الأطر الزمنية لـ { $mint } عبر { $pool }
events-ohlcv-fetch-failed = فشل جلب OHLCV لـ { $mint } عبر { $pool }: { $error }
events-ohlcv-gap-detection-failed = فشل اكتشاف الفجوات لـ { $mint } عبر { $pool }
events-ohlcv-fetch-success = نقاط OHLCV المخزَّنة لـ { $mint }: { $count }
events-ohlcv-retention-backfill-failed = فشل ملء بيانات الاحتفاظ لـ { $mint } عبر { $pool }
events-ohlcv-empty-fetch = جلب OHLCV فارغ لـ { $mint } عبر { $pool }
events-ohlcv-pool-discovery-failed = فشل اكتشاف مجمعات السيولة لـ { $mint }
events-ohlcv-pool-discovery-success = تم اكتشاف مجمعات السيولة لـ { $mint }
events-ohlcv-process-token-error = خطأ في معالجة { $mint }: { $error }
events-ohlcv-rate-limit-hit = تم بلوغ حد المعدل أثناء معالجة { $mint }
events-ohlcv-pool-unavailable = لا توجد مجمعات سيولة سليمة لـ { $mint }، وتم التأجيل
events-ohlcv-token-missing = كان الرمز { $mint } مفقودًا أثناء المعالجة
events-monitors-stopped = تم إيقاف مراقبي التداول الآلي
events-monitors-starting = جارٍ تشغيل مراقبي التداول الآلي
events-entry-monitor-started = تم تشغيل مراقب فرص الدخول
events-exit-monitor-started = تم تشغيل مراقب الخروج/المراكز
events-trader-service-stopped = تم إيقاف خدمة المتداول بسلاسة
events-trader-service-stopping = بدأ إيقاف خدمة المتداول
events-trader-service-started = تمت تهيئة خدمة المتداول بالكامل وهي قيد التشغيل
events-trader-auto-trading-error = واجه التداول الآلي خطأ
events-trader-trading-enabled = التداول مفعّل ونشط
events-trader-trading-disabled = التداول معطّل في الإعدادات
events-trader-service-initializing = بدأت تهيئة خدمة المتداول
events-connectivity-monitoring-stopped = تم إيقاف مراقبة الاتصال
events-connectivity-monitoring-started = بدأت مراقبة الاتصال (الفاصل={ $seconds }ث)
events-connectivity-service-initialized = تمت تهيئة خدمة الاتصال، عدد المراقبين: { $count }
events-connectivity-critical-unhealthy = نقاط الاتصال الحرجة غير السليمة: { $count } - ينبغي أن يوقف النظام العمليات مؤقتًا
events-connectivity-endpoint-recovered = تعافت نقطة الاتصال من { $from } إلى سليمة
events-position-entry-not-landed = لم تصل عملية شراء { $symbol } إلى السلسلة؛ تمت إزالة مركزها
events-position-fill-after-force-close = وصلت صفقة على { $symbol } إلى السلسلة بعد الإغلاق القسري لمركزها؛ تم تسجيلها وإعادة احتساب المركز
events-position-swap-unbooked = صفقة على { $symbol } مؤكدة على السلسلة لم تُسجَّل في مركزها بعد؛ يُعاد التحقق منها حتى تُسجَّل

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = مبادلة
events-category-transaction = معاملة
events-category-pool = مجمع سيولة
events-category-position = مركز
events-category-token = رمز
events-category-wallet = محفظة
events-category-trader = المتداول
events-category-entry = دخول
events-category-system = النظام
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = الأمان
events-category-connectivity = الاتصال
events-category-filtering = الترشيح
events-category-scheduled-task = مهمة مجدولة
events-category-learner = المتعلّم
events-category-other = أخرى

events-loading = جارٍ تحميل الأحداث...
events-load-failed = فشل تحميل الأحداث
events-load-failed-description = بانتظار استجابة الخادم الخلفي. ستتم إعادة المحاولة تلقائيًا.
events-load-error = تعذّر تحميل الأحداث
events-search-placeholder = البحث في الأحداث...
events-summary-total = الإجمالي
events-filter-category = الفئة
events-filter-all-categories = كل الفئات
events-filter-all-severities = كل مستويات الخطورة
events-col-time = الوقت
events-col-category = الفئة
events-col-type = النوع
events-col-severity = الخطورة
events-col-message = الرسالة
events-col-token = الرمز
events-col-details = التفاصيل
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count } أخرى

## Event details dialog (ui/events_dialog.js)

events-dialog-title = تفاصيل الحدث
events-dialog-close =
    .aria-label = إغلاق النافذة
events-dialog-payload = الحمولة
events-dialog-copy = نسخ التفاصيل
events-dialog-copy-title =
    .title = نسخ كل تفاصيل الحدث
events-dialog-copy-done = تم النسخ!
events-dialog-copy-failed = فشل
events-dialog-not-available = غير متاح
# $category is the category label; shown when an event has no message.
events-dialog-category-event = حدث { $category }
events-dialog-field-id = معرّف الحدث
events-dialog-field-severity = الخطورة
events-dialog-field-category = الفئة
events-dialog-field-subtype = النوع الفرعي
events-dialog-field-mint = عنوان إصدار الرمز
events-dialog-field-reference = المرجع
events-dialog-field-time = وقت الحدث
events-dialog-field-age = العمر
events-dialog-field-created = وقت الإنشاء
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = تفاصيل الحدث
events-dialog-export-message = الرسالة
events-dialog-export-payload = الحمولة
events-dialog-export-line = { $label }: { $value }
