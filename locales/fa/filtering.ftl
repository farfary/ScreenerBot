# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = اعشار در پایگاه داده موجود نیست
filtering-reject-token-too-new = توکن بیش از حد جدید است
filtering-reject-cooldown-filtered = رد شده به‌دلیل زمان انتظار
filtering-reject-dex-data-missing = داده { -dexscreener } موجود نیست
filtering-reject-gecko-data-missing = داده { -geckoterminal } موجود نیست
filtering-reject-rug-data-missing = داده { -rugcheck } موجود نیست
filtering-reject-onchain-numeric-symbol = نماد فقط عددی (کلاهبرداری)
filtering-reject-onchain-empty-symbol = نماد خالی (کلاهبرداری)
filtering-reject-onchain-suspicious-symbol = نماد مشکوک (کلاهبرداری)
filtering-reject-onchain-known-scam-authority = اختیار شناخته‌شده کلاهبرداری
filtering-reject-onchain-immutable-with-freeze = تغییرناپذیر + اختیار فریز (کلاهبرداری)
filtering-reject-onchain-high-risk-score = امتیاز ریسک بالا روی زنجیره
filtering-reject-dex-empty-name = نام خالی
filtering-reject-dex-empty-symbol = نماد خالی
filtering-reject-dex-empty-logo = نشانی لوگو خالی
filtering-reject-dex-empty-website = نشانی وب‌سایت خالی
filtering-reject-dex-txn-5m = تراکنش 5m کم
filtering-reject-dex-txn-1h = تراکنش 1h کم
filtering-reject-dex-zero-liq = نقدینگی صفر
filtering-reject-dex-liq-low = نقدینگی بیش از حد کم
filtering-reject-dex-liq-high = نقدینگی بیش از حد زیاد
filtering-reject-dex-mcap-low = ارزش بازار بیش از حد کم
filtering-reject-dex-mcap-high = ارزش بازار بیش از حد زیاد
filtering-reject-dex-vol-low = حجم بیش از حد کم
filtering-reject-dex-vol-missing = حجم موجود نیست
filtering-reject-dex-fdv-low = FDV بیش از حد کم
filtering-reject-dex-fdv-high = FDV بیش از حد زیاد
filtering-reject-dex-vol5m-low = حجم 5m بیش از حد کم
filtering-reject-dex-vol5m-missing = حجم 5m موجود نیست
filtering-reject-dex-vol1h-low = حجم 1h بیش از حد کم
filtering-reject-dex-vol1h-missing = حجم 1h موجود نیست
filtering-reject-dex-vol6h-low = حجم 6h بیش از حد کم
filtering-reject-dex-vol6h-missing = حجم 6h موجود نیست
filtering-reject-dex-price-change-5m-low = تغییر قیمت 5m بیش از حد کم
filtering-reject-dex-price-change-5m-high = تغییر قیمت 5m بیش از حد زیاد
filtering-reject-dex-price-change-low = تغییر قیمت بیش از حد کم
filtering-reject-dex-price-change-high = تغییر قیمت بیش از حد زیاد
filtering-reject-dex-price-change-6h-low = تغییر قیمت 6h بیش از حد کم
filtering-reject-dex-price-change-6h-high = تغییر قیمت 6h بیش از حد زیاد
filtering-reject-dex-price-change-24h-low = تغییر قیمت 24h بیش از حد کم
filtering-reject-dex-price-change-24h-high = تغییر قیمت 24h بیش از حد زیاد
filtering-reject-gecko-liq-low = نقدینگی بیش از حد کم
filtering-reject-gecko-liq-high = نقدینگی بیش از حد زیاد
filtering-reject-gecko-mcap-low = ارزش بازار بیش از حد کم
filtering-reject-gecko-mcap-high = ارزش بازار بیش از حد زیاد
filtering-reject-gecko-vol5m-low = حجم 5m بیش از حد کم
filtering-reject-gecko-vol5m-missing = حجم 5m موجود نیست
filtering-reject-gecko-vol1h-low = حجم 1h بیش از حد کم
filtering-reject-gecko-vol1h-missing = حجم 1h موجود نیست
filtering-reject-gecko-vol24h-low = حجم 24h بیش از حد کم
filtering-reject-gecko-vol24h-missing = حجم 24h موجود نیست
filtering-reject-gecko-price-change-5m-low = تغییر قیمت 5m بیش از حد کم
filtering-reject-gecko-price-change-5m-high = تغییر قیمت 5m بیش از حد زیاد
filtering-reject-gecko-price-change-1h-low = تغییر قیمت 1h بیش از حد کم
filtering-reject-gecko-price-change-1h-high = تغییر قیمت 1h بیش از حد زیاد
filtering-reject-gecko-price-change-24h-low = تغییر قیمت 24h بیش از حد کم
filtering-reject-gecko-price-change-24h-high = تغییر قیمت 24h بیش از حد زیاد
filtering-reject-gecko-pool-count-low = تعداد استخر بیش از حد کم
filtering-reject-gecko-pool-count-high = تعداد استخر بیش از حد زیاد
filtering-reject-gecko-pool-count-missing = تعداد استخر موجود نیست
filtering-reject-gecko-reserve-low = ذخیره بیش از حد کم
filtering-reject-gecko-reserve-missing = ذخیره موجود نیست
filtering-reject-rug-rugged = توکن راگ‌شده
filtering-reject-rug-score = امتیاز ریسک بیش از حد بالا
filtering-reject-rug-level-danger = سطح ریسک خطرناک
filtering-reject-rug-mint-authority = اختیار مینت وجود دارد
filtering-reject-rug-freeze-authority = اختیار فریز وجود دارد
filtering-reject-rug-top-holder = درصد بزرگ‌ترین هولدر بیش از حد زیاد
filtering-reject-rug-top3-holders = درصد 3 هولدر برتر بیش از حد زیاد
filtering-reject-rug-min-holders = تعداد هولدرها کافی نیست
filtering-reject-rug-insider-count = تعداد هولدرهای اینسایدر بیش از حد زیاد
filtering-reject-rug-insider-pct = درصد اینسایدرها بیش از حد زیاد
filtering-reject-rug-creator-pct = موجودی سازنده بیش از حد زیاد
filtering-reject-rug-transfer-fee-present = کارمزد انتقال وجود دارد
filtering-reject-rug-transfer-fee-high = کارمزد انتقال بیش از حد زیاد
filtering-reject-rug-graph-insiders = اینسایدرهای گراف بیش از حد زیاد
filtering-reject-rug-lp-providers-low = تعداد تأمین‌کنندگان LP بیش از حد کم
filtering-reject-rug-lp-providers-missing = تأمین‌کنندگان LP موجود نیست
filtering-reject-rug-lp-lock-low = قفل LP بیش از حد کم
filtering-reject-rug-lp-lock-missing = قفل LP موجود نیست
filtering-reject-llm-analysis-rejected = تحلیل LLM ردشده: { $reason } (اطمینان { $confidence }%، { $provider })
filtering-reject-llm-analysis-rejected-generic = تحلیل LLM ردشده
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV موجود نیست
filtering-reject-dex-price-change-5m-missing = تغییر قیمت 5m موجود نیست
filtering-reject-dex-price-change-missing = تغییر قیمت موجود نیست
filtering-reject-dex-price-change-6h-missing = تغییر قیمت 6h موجود نیست
filtering-reject-dex-price-change-24h-missing = تغییر قیمت 24h موجود نیست
filtering-reject-gecko-liq-missing = نقدینگی موجود نیست
filtering-reject-gecko-mcap-missing = ارزش بازار موجود نیست
filtering-reject-gecko-price-change-5m-missing = تغییر قیمت 5m موجود نیست
filtering-reject-gecko-price-change-1h-missing = تغییر قیمت 1h موجود نیست
filtering-reject-gecko-price-change-24h-missing = تغییر قیمت 24h موجود نیست
filtering-reject-rug-transfer-fee-missing = داده کارمزد انتقال موجود نیست

# Rejection categories used to group reasons.
filtering-reject-category-security = مشکلات امنیتی
filtering-reject-category-distribution = توزیع هولدرها
filtering-reject-category-liquidity-lock = مشکلات قفل LP
filtering-reject-category-fees = کارمزدهای انتقال
filtering-reject-category-liquidity = نقدینگی
filtering-reject-category-volume = حجم معاملات
filtering-reject-category-market-cap = ارزش بازار/FDV
filtering-reject-category-price-action = حرکت قیمت
filtering-reject-category-activity = فعالیت معاملاتی
filtering-reject-category-data-quality = داده‌های ناموجود
filtering-reject-category-timing = فیلترهای زمانی
filtering-reject-category-market = داده‌های بازار
filtering-reject-category-other = سایر

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = وضعیت
filtering-tab-analytics = تحلیل‌ها
filtering-tab-explorer = اکسپلورر
filtering-source-core = هسته
filtering-source-onchain = روی زنجیره
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = تحلیل LLM

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = همه
filtering-range-all-time = تمام دوران
filtering-range-custom = سفارشی
filtering-range-now = اکنون
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } ← { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = در حال ذخیره تغییرات...
filtering-footer-refreshing = در حال به‌روزرسانی اسنپ‌شات...
filtering-footer-unsaved = تغییرات ذخیره‌نشده در انتظار است
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = آخرین ذخیره: { $time }
filtering-footer-in-sync = پیکربندی همگام است

## Info bar and status metrics

filtering-info-total = مجموع:
filtering-info-priced = دارای قیمت:
filtering-info-passed = تأییدشده:
filtering-info-positions = پوزیشن‌ها:
filtering-info-blacklisted = در فهرست سیاه:
filtering-info-cache = کش:
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = در حال ساخت…
filtering-refresh-never = هرگز

filtering-status-loading = در حال بارگذاری آمار...
filtering-status-total = کل توکن‌ها
filtering-status-total-detail = در کش فیلترینگ
filtering-status-total-detail-building = اسنپ‌شات در حال ساخت است — شمارش‌ها در به‌روزرسانی بعدی می‌آیند
filtering-status-priced = دارای قیمت
filtering-status-priced-detail = { $share } دارای قیمت هستند
filtering-status-passed = عبور از فیلترها
filtering-status-passed-detail = { $share } تأیید شدند
filtering-status-positions = پوزیشن‌های باز
filtering-status-positions-detail = معاملات فعال
filtering-status-blacklisted = در فهرست سیاه
filtering-status-blacklisted-detail = توکن‌های علامت‌گذاری‌شده
filtering-status-ohlcv = دارای OHLCV
filtering-status-ohlcv-detail = داده‌های تاریخی
filtering-status-refresh = آخرین به‌روزرسانی
filtering-status-refresh-building = اولین اسنپ‌شات در حال ساخت است
filtering-status-refresh-none = هنوز به‌روزرسانی انجام نشده
filtering-status-no-rejections = داده‌ای از ردشدن‌ها موجود نیست

## Analytics

filtering-analytics-loading = در حال بارگذاری تحلیل‌ها برای { $range }…
filtering-analytics-scanned = کل بررسی‌شده
# $time is a relative time such as "5m ago".
filtering-analytics-updated = به‌روزرسانی: { $time }
filtering-analytics-passed = توکن‌های تأییدشده
filtering-analytics-pass-rate = نرخ تأیید <strong>{ $share }</strong>
filtering-analytics-rejected = توکن‌های ردشده
filtering-analytics-rejection-rate = نرخ رد <strong>{ $share }</strong>
filtering-analytics-by-category = رد شدن بر اساس دسته
filtering-analytics-by-source = رد شدن بر اساس منبع
filtering-analytics-no-category = داده‌ای برای دسته‌ها نیست
filtering-analytics-no-source = داده‌ای برای منبع‌ها نیست
filtering-analytics-top-reasons = پرتکرارترین دلایل رد شدن
filtering-analytics-no-data = داده‌ای موجود نیست
filtering-analytics-column-reason = دلیل
filtering-analytics-column-category = دسته
filtering-analytics-column-count = تعداد
filtering-analytics-column-share = %
filtering-analytics-column-impact = تأثیر
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [one] { $amount } توکن
       *[other] { $amount } توکن
    }

## Explorer

filtering-explorer-top-reasons = دلایل برتر
filtering-explorer-recent = ردشدن‌های اخیر
filtering-explorer-none = داده‌ای نیست
filtering-explorer-none-recent = مورد اخیری نیست
filtering-explorer-search =
    .placeholder = جستجوی دلایل...
filtering-explorer-overview = نمای کلی
filtering-explorer-no-match = دلیل مطابقی پیدا نشد
filtering-explorer-column-token = توکن
filtering-explorer-column-source = منبع
filtering-explorer-column-time = زمان
filtering-explorer-page = صفحه { $page }
filtering-explorer-no-results = نتیجه‌ای نیست
filtering-explorer-empty = توکنی پیدا نشد
filtering-explorer-empty-filtered = توکنی مطابق فیلتر پیدا نشد
filtering-explorer-load-failed = بارگذاری توکن‌ها ناموفق بود

## Configuration panels

filtering-config-loading = در حال بارگذاری پیکربندی…
# $query is the text typed in the filter box.
filtering-config-no-match = پارامتری مطابق «{ $query }» پیدا نشد
filtering-config-no-parameters = این منبع پارامتری ارائه نمی‌دهد
# $source is the source name.
filtering-source-off = فیلترینگ { $source } خاموش است — این پارامترها ارزیابی نمی‌شوند.
filtering-toolbar-filter =
    .placeholder = فیلتر پارامترها
    .aria-label = فیلتر پارامترها
filtering-toolbar-clear =
    .aria-label = پاک کردن فیلتر
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [one] { $amount } پارامتر
       *[other] { $amount } پارامتر
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [one] { $visible } از { $total } پارامتر
       *[other] { $visible } از { $total } پارامتر
    }
filtering-group-enable =
    .aria-label = فعال‌سازی بررسی‌های { $group }
filtering-field-min = حداقل
filtering-field-max = حداکثر
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = حداقل { $label }
filtering-field-max-aria =
    .aria-label = حداکثر { $label }
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = بازگشت به پیش‌فرض ({ $default })
    .aria-label = بازگرداندن { $label } به پیش‌فرض

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = پیکربندی ذخیره شد
    .message = تنظیمات فیلترینگ ذخیره و اسنپ‌شات به‌روزرسانی شد
filtering-toast-save-failed = ذخیره ناموفق بود
    .message = ذخیره پیکربندی فیلترینگ ناموفق بود
filtering-toast-reset = تغییرات بازنشانی شد
    .message = پیکربندی به آخرین وضعیت ذخیره‌شده بازگردانده شد
filtering-toast-refresh-failed = به‌روزرسانی ناموفق بود
    .message = به‌روزرسانی اسنپ‌شات فیلترینگ ناموفق بود
filtering-toast-exported = پیکربندی صادر شد
    .message = تنظیمات فیلترینگ در فایل ذخیره شد
filtering-toast-imported = پیکربندی وارد شد
    .message = تنظیمات فیلترینگ از فایل بارگذاری شد
filtering-toast-import-failed = وارد کردن ناموفق بود
    .message = وارد کردن پیکربندی ناموفق بود - قالب فایل نامعتبر است
filtering-toast-load-failed = بارگذاری ناموفق بود
    .message = بارگذاری پیکربندی فیلترینگ ناموفق بود
filtering-toast-range-missing = لطفاً تاریخ شروع و پایان را هر دو انتخاب کنید
filtering-toast-range-order = زمان شروع باید پیش از زمان پایان باشد
filtering-toast-range-future = زمان پایان نمی‌تواند در آینده باشد
