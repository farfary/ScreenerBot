# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

# $component is a code identifier and is not translated.
services-health-component-unavailable = مؤلفه { $component } در دسترس نیست
services-health-unavailable = وضعیت سلامت در دسترس نیست
services-health-pools-not-running = سرویس استخر در حال اجرا نیست
services-health-events-db-uninitialized = پایگاه‌داده رویدادها راه‌اندازی نشده است
services-health-sol-price-not-running = سرویس قیمت SOL در حال اجرا نیست
services-health-sol-price-stale = داده قیمت SOL کهنه است ({ $seconds } ثانیه قدمت)
services-health-sol-price-no-data = هنوز داده قیمت SOL موجود نیست
services-health-telegram-discovery = حالت کشف
services-health-telegram-disconnected = قطع‌شده
services-health-wallet-watch-polling-only = تشخیص فقط با بررسی دوره‌ای در حال اجراست
services-health-assistant-tasks-disabled = در پیکربندی غیرفعال شده است
# $endpoints is the debug-formatted list of unhealthy critical endpoints.
services-health-connectivity-critical-unhealthy = اندپوینت‌های حیاتی ناسالم هستند: { $endpoints }
# $seconds is the age of the filtering snapshot.
services-health-filtering-snapshot-stale = اسنپ‌شات فیلترینگ { $seconds } ثانیه قدمت دارد

## Services page (pages/services.js)

# Health status tags from ServiceHealth in src/services/health.rs.
services-status-healthy = سالم
services-status-starting = در حال شروع
services-status-degraded = کاهش‌یافته
services-status-unhealthy = ناسالم
services-status-stopping = در حال توقف
services-status-disabled = غیرفعال
services-status-unknown = نامشخص

# Service ids from Service::name() in src/services/implementations/*.rs.
services-name-account = حساب کاربری
services-name-assistant-scheduled-tasks = وظایف زمان‌بندی‌شده دستیار
services-name-ata-cleanup = پاکسازی حساب‌های توکن
services-name-connectivity = اتصال‌پذیری
services-name-copy-trading = کپی‌تریدینگ
services-name-events = رویدادها
services-name-filtering = فیلترینگ
services-name-llm-analysis = تحلیل LLM
services-name-ohlcv = OHLCV
services-name-pool-analyzer = تحلیل‌گر استخر
services-name-pool-calculator = محاسبه‌گر استخر
services-name-pool-discovery = کشف استخر
services-name-pool-fetcher = دریافت‌کننده استخر
services-name-pools = استخرها
services-name-positions = پوزیشن‌ها
services-name-referral = معرفی
services-name-rpc-stats = آمار RPC
services-name-sol-price = قیمت { -sol }
services-name-telegram = { -telegram }
services-name-tokens = توکن‌ها
services-name-trader = معامله‌گر
services-name-transactions = تراکنش‌ها
services-name-update-check = بررسی به‌روزرسانی
services-name-wallet = کیف پول
services-name-wallet-watch = پایش کیف پول
services-name-webserver = وب‌سرور

services-loading = در حال بارگیری سرویس‌ها...
services-load-failed = بارگیری سرویس‌ها ناموفق بود
services-load-failed-description = در انتظار پاسخ بک‌اند. به‌طور خودکار دوباره تلاش می‌کنیم.
services-refresh-failed = تازه‌سازی سرویس‌ها ممکن نشد
services-search-placeholder = جست‌وجوی سرویس‌ها...
services-summary-total = مجموع
services-summary-alerts = هشدارها
# $degraded and $unhealthy are formatted counts.
services-summary-alerts-tooltip = { $degraded } کاهش‌یافته / { $unhealthy } ناسالم
services-filter-status = وضعیت
services-filter-all-statuses = همه وضعیت‌ها
services-filter-all-services = همه سرویس‌ها
services-filter-enabled-only = فقط فعال‌ها
services-filter-disabled-only = فقط غیرفعال‌ها
services-col-service = سرویس
services-col-health = سلامت
services-col-priority = اولویت
services-col-uptime = مدت فعالیت
services-col-activity = فعالیت
services-col-last-cycle = آخرین چرخه
services-col-avg-cycle = میانگین چرخه
services-col-avg-poll = میانگین بررسی
services-col-cycle-rate = نرخ چرخه
services-col-tasks = وظایف
services-col-ops = عملیات/ثانیه
services-col-errors = خطاها
services-col-dependencies = وابستگی‌ها
services-dependencies-none = ندارد
# $percent is a formatted percentage.
services-activity-busy = { $percent } مشغول
services-activity-polls =
    { $count ->
        [one] { $count } بررسی
       *[other] { $count } بررسی
    }
# The durations are formatted values; the message spans several lines.
services-tasks-tooltip =
    { $count ->
        [one] { $count } وظیفه
       *[other] { $count } وظیفه
    }
    آخرین: { $last }
    میانگین: { $avg }
    بررسی: { $poll }
    بیکار: { $idle }
    کل بررسی‌ها: { $polls }
services-tasks-none = وظیفه ابزارگذاری‌شده‌ای نیست
