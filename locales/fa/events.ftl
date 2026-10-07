# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = رویداد OHLCV: { $subtype }
events-filtering-default = رویداد فیلترینگ: { $subtype }
events-trader-default = رویداد معامله‌گر: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = وظیفه «{ $name }» تکمیل شد
events-task-failed = وظیفه «{ $name }» ناموفق بود
events-task-timed-out = مهلت وظیفه «{ $name }» به پایان رسید

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = وظیفه تکمیل شد
events-subtype-task-failed = وظیفه ناموفق بود
events-subtype-task-timed-out = مهلت وظیفه به پایان رسید

# Shown when an event carries no display text.
events-message-none = بدون پیام

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = پاک‌سازی کش OHLCV ناموفق بود
events-ohlcv-gap-cleanup-failed = پاک‌سازی رکوردهای شکاف پرشده ناموفق بود
events-ohlcv-gap-fill-failed = خطا در پر کردن شکاف برای { $mint }
events-ohlcv-backfill-scheduled = پرکردن داده‌های گذشته چندبازه‌ای برای { $mint } از طریق { $pool } زمان‌بندی شد
events-ohlcv-fetch-failed = دریافت OHLCV برای { $mint } از طریق { $pool } ناموفق بود: { $error }
events-ohlcv-gap-detection-failed = تشخیص شکاف برای { $mint } از طریق { $pool } ناموفق بود
events-ohlcv-fetch-success = { $count } نقطه OHLCV برای { $mint } ذخیره شد
events-ohlcv-retention-backfill-failed = پرکردن داده‌های گذشته برای نگهداری { $mint } از طریق { $pool } ناموفق بود
events-ohlcv-empty-fetch = دریافت OHLCV برای { $mint } از طریق { $pool } خالی بود
events-ohlcv-pool-discovery-failed = کشف استخر برای { $mint } ناموفق بود
events-ohlcv-pool-discovery-success = استخرهای { $mint } کشف شدند
events-ohlcv-process-token-error = خطا در پردازش { $mint }: { $error }
events-ohlcv-rate-limit-hit = هنگام پردازش { $mint } محدودیت نرخ فعال شد
events-ohlcv-pool-unavailable = استخر سالمی برای { $mint } در دسترس نیست؛ به تعویق افتاد
events-ohlcv-token-missing = توکن { $mint } هنگام پردازش وجود نداشت
events-monitors-stopped = پایش‌های معاملات خودکار متوقف شدند
events-monitors-starting = پایش‌های معاملات خودکار در حال راه‌اندازی هستند
events-entry-monitor-started = پایش فرصت‌های ورود شروع شد
events-exit-monitor-started = پایش خروج/پوزیشن شروع شد
events-trader-service-stopped = سرویس معامله‌گر به‌صورت ایمن متوقف شد
events-trader-service-stopping = توقف سرویس معامله‌گر آغاز شد
events-trader-service-started = سرویس معامله‌گر کاملاً راه‌اندازی شد و در حال اجراست
events-trader-auto-trading-error = معاملات خودکار با خطا مواجه شد
events-trader-trading-enabled = معاملات فعال است
events-trader-trading-disabled = معاملات در پیکربندی غیرفعال است
events-trader-service-initializing = راه‌اندازی سرویس معامله‌گر آغاز شد
events-connectivity-monitoring-stopped = پایش اتصال متوقف شد
events-connectivity-monitoring-started = پایش اتصال شروع شد (فاصله={ $seconds } ثانیه)
events-connectivity-service-initialized = سرویس اتصال با { $count } پایشگر راه‌اندازی شد
events-connectivity-critical-unhealthy = { $count } اندپوینت حیاتی ناسالم است - سیستم باید عملیات را متوقف کند
events-connectivity-endpoint-recovered = اندپوینت از { $from } به حالت سالم بازگشت
events-position-entry-not-landed = خرید { $symbol } روی زنجیره ثبت نشد؛ پوزیشن آن حذف شد
events-position-fill-after-force-close = یک معامله { $symbol } پس از بستن اجباری پوزیشن آن روی زنجیره ثبت شد؛ این معامله ثبت و پوزیشن دوباره محاسبه شد

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = سواپ
events-category-transaction = تراکنش
events-category-pool = استخر
events-category-position = پوزیشن
events-category-token = توکن
events-category-wallet = کیف پول
events-category-trader = معامله‌گر
events-category-entry = ورود
events-category-system = سیستم
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = امنیت
events-category-connectivity = اتصال
events-category-filtering = فیلترینگ
events-category-scheduled-task = وظیفه زمان‌بندی‌شده
events-category-learner = یادگیرنده
events-category-other = سایر

events-loading = در حال بارگذاری رویدادها...
events-load-failed = بارگذاری رویدادها ناموفق بود
events-load-failed-description = در انتظار پاسخ هسته پشتیبان. به‌طور خودکار دوباره تلاش می‌کنیم.
events-load-error = بارگذاری رویدادها ممکن نشد
events-search-placeholder = جستجوی رویدادها...
events-summary-total = مجموع
events-filter-category = دسته
events-filter-all-categories = همه دسته‌ها
events-filter-all-severities = همه شدت‌ها
events-col-time = زمان
events-col-category = دسته
events-col-type = نوع
events-col-severity = شدت
events-col-message = پیام
events-col-token = توکن
events-col-details = جزئیات
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count } مورد دیگر

## Event details dialog (ui/events_dialog.js)

events-dialog-title = جزئیات رویداد
events-dialog-close =
    .aria-label = بستن پنجره
events-dialog-payload = محتوا
events-dialog-copy = کپی جزئیات
events-dialog-copy-title =
    .title = کپی همه جزئیات رویداد
events-dialog-copy-done = کپی شد!
events-dialog-copy-failed = ناموفق
events-dialog-not-available = ندارد
# $category is the category label; shown when an event has no message.
events-dialog-category-event = رویداد { $category }
events-dialog-field-id = شناسه رویداد
events-dialog-field-severity = شدت
events-dialog-field-category = دسته
events-dialog-field-subtype = زیرنوع
events-dialog-field-mint = مینت توکن
events-dialog-field-reference = مرجع
events-dialog-field-time = زمان رویداد
events-dialog-field-age = قدمت
events-dialog-field-created = ایجاد
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = جزئیات رویداد
events-dialog-export-message = پیام
events-dialog-export-payload = محتوا
events-dialog-export-line = { $label }: { $value }
