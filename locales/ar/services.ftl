# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

# $component is a code identifier and is not translated.
services-health-component-unavailable = المكوّن { $component } غير متاح
services-health-unavailable = حالة الصحة غير متاحة
services-health-pools-not-running = خدمة مجمعات السيولة لا تعمل
services-health-events-db-uninitialized = قاعدة بيانات الأحداث غير مهيأة
services-health-sol-price-not-running = خدمة سعر SOL لا تعمل
services-health-sol-price-stale = بيانات سعر SOL قديمة (عمرها { $seconds }ث)
services-health-sol-price-no-data = لا تتوفر بيانات سعر SOL بعد
services-health-telegram-discovery = وضع الاكتشاف
services-health-telegram-disconnected = غير متصل
services-health-wallet-watch-polling-only = الرصد يعمل بالاستعلام الدوري فقط
services-health-assistant-tasks-disabled = معطّلة في الإعدادات
# $endpoints is the debug-formatted list of unhealthy critical endpoints.
services-health-connectivity-critical-unhealthy = نقاط الاتصال الحرجة غير سليمة: { $endpoints }
# $seconds is the age of the filtering snapshot.
services-health-filtering-snapshot-stale = عمر لقطة الترشيح { $seconds }ث

## Services page (pages/services.js)

# Health status tags from ServiceHealth in src/services/health.rs.
services-status-healthy = سليمة
services-status-starting = جارٍ البدء
services-status-degraded = متدهورة
services-status-unhealthy = غير سليمة
services-status-stopping = جارٍ الإيقاف
services-status-disabled = معطّلة
services-status-unknown = غير معروفة

# Service ids from Service::name() in src/services/implementations/*.rs.
services-name-account = الحساب
services-name-assistant-scheduled-tasks = مهام المساعد المجدولة
services-name-ata-cleanup = تنظيف حسابات الرموز
services-name-connectivity = الاتصال
services-name-copy-trading = نسخ التداول
services-name-events = الأحداث
services-name-filtering = الترشيح
services-name-llm-analysis = تحليل LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = تسعير مجمعات السيولة
services-name-pools = مجمعات السيولة
services-name-positions = المراكز
services-name-referral = الإحالة
services-name-rpc-stats = إحصاءات RPC
services-name-sol-price = سعر { -sol }
services-name-telegram = { -telegram }
services-name-tokens = الرموز
services-name-trader = المتداول الآلي
services-name-transactions = المعاملات
services-name-update-check = التحقق من التحديثات
services-name-wallet = المحفظة
services-name-wallet-watch = مراقبة المحافظ
services-name-webserver = خادم الويب

services-loading = جارٍ تحميل الخدمات...
services-load-failed = تعذّر تحميل الخدمات
services-load-failed-description = بانتظار استجابة الخلفية. ستتم إعادة المحاولة تلقائيًا.
services-refresh-failed = تعذّر تحديث الخدمات
services-search-placeholder = البحث في الخدمات...
services-summary-total = الإجمالي
services-summary-alerts = التنبيهات
# $degraded and $unhealthy are formatted counts.
services-summary-alerts-tooltip = متدهورة: { $degraded } / غير سليمة: { $unhealthy }
services-filter-status = الحالة
services-filter-all-statuses = جميع الحالات
services-filter-all-services = جميع الخدمات
services-filter-enabled-only = المفعّلة فقط
services-filter-disabled-only = المعطّلة فقط
services-col-service = الخدمة
services-col-health = الصحة
services-col-priority = الأولوية
services-col-uptime = مدة التشغيل
services-col-activity = النشاط
services-col-last-cycle = آخر دورة
services-col-avg-cycle = متوسط الدورة
services-col-avg-poll = متوسط الاستعلام
services-col-cycle-rate = دورات/ث
services-col-tasks = المهام
services-col-ops = عمليات/ث
services-col-errors = الأخطاء
services-col-dependencies = التبعيات
# $percent is a formatted percentage.
services-activity-busy = مشغولة { $percent }
services-activity-polls =
    { $count ->
        [zero] { $count } استعلام
        [one] { $count } استعلام
        [two] { $count } استعلامان
        [few] { $count } استعلامات
        [many] { $count } استعلامًا
       *[other] { $count } استعلام
    }
# The durations are formatted values; the message spans several lines.
services-tasks-tooltip =
    المهام: { $count }
    الأخير: { $last }
    المتوسط: { $avg }
    الاستعلام: { $poll }
    الخمول: { $idle }
    إجمالي الاستعلامات: { $polls }
services-tasks-none = لا توجد مهام مرصودة

# Empty table (scripts/pages/services.js)
services-empty = لا توجد خدمات قيد التشغيل
    .message = تظهر الخدمات هنا بعد أن يشغّلها البوت.
