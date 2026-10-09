# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = ابزارها
tools-category-wallet = کیف پول
tools-category-token = توکن
tools-category-single-token = توکن منفرد
tools-category-utilities = ابزارهای جانبی
tools-sidebar-hint = برای شروع یک ابزار انتخاب کنید
tools-placeholder-title = انتخاب ابزار
tools-placeholder-subtitle = برای شروع، یک ابزار از نوار کناری انتخاب کنید
tools-placeholder-hint-wallets = ابزارهای کیف پول به مدیریت کیف پول‌های سولانای شما کمک می‌کنند
tools-placeholder-hint-secure = همه عملیات ایمن هستند و در صورت امکان قابل بازگشت‌اند

# Status dot tooltip of a tool in the navigation. Ids are the nav `data-status` values.
tools-status-ready = آماده استفاده
tools-status-coming = به‌زودی
tools-status-beta = بتا - ممکن است باگ داشته باشد
tools-status-disabled = در حال حاضر غیرفعال
# Badge of a tool that is not available yet.
tools-status-badge-coming = به‌زودی
tools-status-badge-beta = بتا
tools-toast-coming-soon = این ابزار به‌زودی اضافه می‌شود
tools-toast-disabled = این ابزار در حال حاضر غیرفعال است
tools-setup-gate-title = این ابزار به کیف پول نیاز دارد

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = پاکسازی کیف پول
tools-tool-wallet-cleanup-summary = بستن ATAهای خالی
tools-tool-wallet-cleanup-description = بستن حساب‌های توکن (ATA) خالی برای بازپس‌گیری { -sol }
tools-tool-burn-tokens-title = برن توکن
tools-tool-burn-tokens-summary = نابودسازی دائمی توکن‌ها
tools-tool-burn-tokens-description = نابودسازی دائمی توکن‌های کیف پول شما
tools-tool-token-analyzer-title = تحلیل‌گر توکن
tools-tool-token-analyzer-summary = تحلیل عمیق توکن
tools-tool-token-analyzer-description = تحلیل عمیق هر توکن سولانا با بینش‌های چندبعدی
tools-tool-create-token-title = ساخت توکن
tools-tool-create-token-summary = استقرار توکن SPL جدید
tools-tool-create-token-description = استقرار یک توکن SPL جدید روی سولانا
tools-tool-trade-watcher-title = ناظر معاملات
tools-tool-trade-watcher-summary = پایش معاملات و اقدام خودکار
tools-tool-trade-watcher-description = پایش معاملات توکن و اجرای خودکار اقدام‌های خرید/فروش
tools-tool-token-watch-title = پایش هولدرها
tools-tool-token-watch-summary = ردیابی هولدرهای جدید توکن
tools-tool-token-watch-description = ردیابی و پایش هولدرهای جدید توکن به‌صورت لحظه‌ای
tools-tool-buy-multi-wallets-title = خرید چندگانه
tools-tool-buy-multi-wallets-summary = هماهنگ‌سازی خرید در چند کیف پول
tools-tool-buy-multi-wallets-description = اجرای سفارش‌های خرید هماهنگ در چند کیف پول با مقادیر تصادفی
tools-tool-sell-multi-wallets-title = فروش چندگانه
tools-tool-sell-multi-wallets-summary = هماهنگ‌سازی فروش در چند کیف پول
tools-tool-sell-multi-wallets-description = اجرای سفارش‌های فروش هماهنگ در چند کیف پول همراه با تجمیع { -sol }
tools-tool-wallet-consolidation-title = تجمیع کیف پول
tools-tool-wallet-consolidation-nav-title = تجمیع
tools-tool-wallet-consolidation-summary = تجمیع دارایی کیف پول‌ها
tools-tool-wallet-consolidation-description = تجمیع { -sol } و توکن‌ها از کیف پول‌های فرعی به کیف پول اصلی
tools-tool-airdrop-checker-title = بررسی ایردراپ
tools-tool-airdrop-checker-summary = بررسی ایردراپ‌های در انتظار
tools-tool-airdrop-checker-description = بررسی ایردراپ‌های در انتظار و پاداش‌های قابل دریافت
tools-tool-wallet-generator-title = تولید کیف پول
tools-tool-wallet-generator-summary = تولید جفت‌کلید جدید
tools-tool-wallet-generator-description = تولید ایمن جفت‌کلیدهای جدید سولانا

## Shared by the tools

tools-validation-mint-required = لطفاً آدرس مینت توکن را وارد کنید
tools-validation-mint-format = قالب آدرس مینت توکن نامعتبر است
tools-validation-mint-invalid = لطفاً یک آدرس مینت معتبر وارد کنید

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = جزئیات توکن
tools-create-token-name-label = نام توکن
tools-create-token-name-input =
    .placeholder = My Token
tools-create-token-symbol-label = نماد
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = اعشار
tools-create-token-supply-label = عرضه اولیه
tools-create-token-description-label = توضیحات
tools-create-token-description-input =
    .placeholder = توضیحات توکن...
tools-create-token-image-title = تصویر توکن
tools-create-token-image-drop = تصویر را اینجا رها کنید یا برای بارگذاری کلیک کنید
tools-create-token-image-hint = پیشنهادی: PNG با اندازه 512x512
tools-create-token-action-preview = پیش‌نمایش
tools-create-token-action-create = ساخت توکن

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = در حال بارگیری تنظیمات...
tools-holder-watch-saved = تنظیمات پایش هولدرها ذخیره شد
tools-holder-watch-save-failed = ذخیره تنظیمات ناموفق بود
tools-holder-watch-save-error = خطا در ذخیره تنظیمات
tools-holder-watch-settings-title = تنظیمات پایش هولدرها
tools-holder-watch-enabled-label = فعال‌سازی پایش هولدرها
tools-holder-watch-interval-label = فاصله بررسی
tools-holder-watch-interval-hint = هر چند وقت یک‌بار تعداد هولدرها بررسی شود (10 تا 3600 ثانیه)
tools-holder-watch-max-tokens-label = حداکثر توکن‌های تحت پایش
tools-holder-watch-max-tokens-hint = حداکثر تعداد توکن‌هایی که هم‌زمان پایش می‌شوند
tools-holder-watch-notify-new-label = اعلان هولدرهای جدید
tools-holder-watch-notify-drop-label = اعلان کاهش هولدرها
tools-holder-watch-min-change-label = حداقل تغییر هولدر
tools-holder-watch-min-change-hint = حداقل تغییر تعداد هولدر برای ارسال اعلان
tools-holder-watch-drop-percent-label = آستانه کاهش هولدر
tools-holder-watch-drop-percent-hint = درصد کاهش برای ارسال هشدار
tools-holder-watch-action-save = ذخیره تنظیمات
tools-holder-watch-tokens-title = توکن‌های تحت پایش
tools-holder-watch-token-input =
    .placeholder = آدرس مینت توکن را وارد کنید...
tools-holder-watch-empty = هیچ توکنی تحت پایش نیست
tools-holder-watch-empty-hint = برای شروع پایش، آدرس مینت یک توکن را در بالا اضافه کنید
tools-holder-watch-coming-soon = قابلیت پایش توکن به‌زودی اضافه می‌شود

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = تحلیل توکن
tools-analyzer-mint-input =
    .placeholder = آدرس مینت توکن را جای‌گذاری کنید...
tools-analyzer-action-analyze = تحلیل
tools-analyzer-action-analyzing = در حال تحلیل...
tools-analyzer-action-copy-report = کپی گزارش
tools-analyzer-loading = در حال تحلیل توکن...
tools-analyzer-failed = تحلیل توکن ناموفق بود
tools-analyzer-empty = برای تحلیل، آدرس مینت یک توکن را وارد کنید
tools-analyzer-empty-hint = بینش جامع درباره هر توکن سولانا دریافت کنید
tools-analyzer-tab-overview = نمای کلی
tools-analyzer-tab-security = امنیت
tools-analyzer-tab-market = بازار
tools-analyzer-tab-liquidity = نقدینگی
tools-analyzer-unknown-token = توکن ناشناس

# Token header actions.
tools-analyzer-favorite-add =
    .title = افزودن به علاقه‌مندی‌ها
    .aria-label = افزودن به علاقه‌مندی‌ها
tools-analyzer-favorite-already = از قبل در علاقه‌مندی‌هاست
# $symbol is the token symbol.
tools-analyzer-favorite-added = { $symbol } به علاقه‌مندی‌ها اضافه شد
tools-analyzer-favorite-failed = افزودن به علاقه‌مندی‌ها ناموفق بود
tools-analyzer-blacklist-add =
    .title = افزودن به فهرست سیاه
    .aria-label = افزودن به فهرست سیاه
tools-analyzer-blacklist-title = افزودن توکن به فهرست سیاه
# $symbol is the token symbol.
tools-analyzer-blacklist-message = { $symbol } به فهرست سیاه اضافه شود؟ این توکن از معاملات کنار گذاشته می‌شود.
tools-analyzer-blacklist-confirm = افزودن به فهرست سیاه
tools-analyzer-blacklisted = در فهرست سیاه
# $symbol is the token symbol.
tools-analyzer-blacklist-done = { $symbol } به فهرست سیاه اضافه شد
tools-analyzer-blacklist-failed = افزودن توکن به فهرست سیاه ناموفق بود

# Overview tab.
tools-analyzer-card-quick-stats = آمار سریع
tools-analyzer-card-market-summary = خلاصه بازار
tools-analyzer-card-token-info = اطلاعات توکن
tools-analyzer-stat-holders = هولدرها
tools-analyzer-stat-decimals = اعشار
tools-analyzer-stat-safety-score = امتیاز ایمنی
tools-analyzer-stat-pools = استخرها
tools-analyzer-stat-volume-24h = حجم 24h
tools-analyzer-stat-change-24h = تغییر 24h
tools-analyzer-stat-market-cap = ارزش بازار
tools-analyzer-stat-liquidity = نقدینگی
tools-analyzer-info-mint = آدرس مینت
tools-analyzer-info-description = توضیحات
tools-analyzer-info-supply = عرضه

# Security tab.
tools-analyzer-security-empty = داده امنیتی موجود نیست
tools-analyzer-security-empty-hint = تحلیل امنیتی برای این توکن در دسترس نیست
tools-analyzer-card-safety-score = امتیاز ایمنی
tools-analyzer-score-good = خوب
tools-analyzer-score-moderate = متوسط
tools-analyzer-score-risky = پرریسک
# $score is the formatted raw risk score.
tools-analyzer-raw-score = امتیاز خام ریسک: { $score }
tools-analyzer-card-authorities = اختیارات توکن
tools-analyzer-authority-mint = اختیار مینت
tools-analyzer-authority-freeze = اختیار فریز
tools-analyzer-authority-transfer-fee = کارمزد انتقال
tools-analyzer-authority-mutable = قابل تغییر
tools-analyzer-authority-active = فعال
tools-analyzer-authority-revoked = لغوشده
tools-analyzer-card-holder-concentration = تمرکز هولدرها
tools-analyzer-top-holders = در اختیار 10 هولدر برتر
# $count is the formatted number of risks.
tools-analyzer-risks-title = ریسک‌های امنیتی ({ $count })
tools-analyzer-risks-title-none = ریسک‌های امنیتی
tools-analyzer-risks-none = ریسک امنیتی شناسایی نشد

# Market tab.
tools-analyzer-market-empty = داده بازار موجود نیست
tools-analyzer-market-empty-hint = داده بازار برای این توکن در دسترس نیست
tools-analyzer-card-price = قیمت فعلی
tools-analyzer-card-price-changes = تغییرات قیمت
tools-analyzer-card-volume = حجم معاملات
tools-analyzer-card-transactions = تراکنش‌های 24h
tools-analyzer-card-valuation = ارزش‌گذاری
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = حجم 1h
tools-analyzer-stat-volume-6h = حجم 6h
tools-analyzer-stat-fdv = ارزش کاملاً رقیق‌شده
tools-analyzer-txn-buys = خریدها
tools-analyzer-txn-sells = فروش‌ها

# Liquidity tab.
tools-analyzer-liquidity-empty = داده نقدینگی موجود نیست
tools-analyzer-liquidity-empty-hint = استخری برای این توکن پیدا نشد
tools-analyzer-card-total-liquidity = کل نقدینگی
tools-analyzer-card-pools = استخرها
tools-analyzer-active-pools =
    { $count ->
        [one] استخر فعال
       *[other] استخر فعال
    }
tools-analyzer-card-pool-details = جزئیات استخر
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = نقدینگی ({ -sol })
tools-analyzer-pools-column-status = وضعیت
tools-analyzer-pool-primary = اصلی

# Copied report. Each line is one message; values arrive already formatted.
tools-analyzer-report-empty = تحلیلی برای کپی وجود ندارد
tools-analyzer-report-label = گزارش تحلیل
tools-analyzer-report-title = گزارش تحلیل توکن
tools-analyzer-report-token = توکن: { $symbol } ({ $name })
tools-analyzer-report-mint = مینت: { $mint }
# $sol is the price with the SOL unit.
tools-analyzer-report-price = قیمت: { $sol }
tools-analyzer-report-price-with-usd = قیمت: { $sol } ({ $usd })
tools-analyzer-report-security = امنیت:
tools-analyzer-report-safety-score = - امتیاز ایمنی: { $score }/100
tools-analyzer-report-mint-authority = - اختیار مینت: { $state }
tools-analyzer-report-freeze-authority = - اختیار فریز: { $state }
tools-analyzer-report-risks = - ریسک‌ها: { $count }
tools-analyzer-report-market = بازار:
tools-analyzer-report-volume = - حجم 24h: { $amount }
tools-analyzer-report-change = - تغییر 24h: { $amount }
tools-analyzer-report-market-cap = - ارزش بازار: { $amount }
tools-analyzer-report-liquidity = نقدینگی:
tools-analyzer-report-liquidity-total = - مجموع: { $amount }
tools-analyzer-report-pools = - استخرها: { $count }
# $time is the formatted time the analysis was fetched.
tools-analyzer-report-generated = زمان تولید: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

# Ids are the watch types. `notify` is the short badge of the notify-only type.
tools-watch-type-buy-on-sell = خرید هنگام فروش
tools-watch-type-sell-on-buy = فروش هنگام خرید
tools-watch-type-notify = اعلان
tools-watch-type-notify-only = فقط اعلان

tools-trade-watcher-setup-title = راه‌اندازی پایش
tools-trade-watcher-mint-label = آدرس مینت توکن
tools-trade-watcher-mint-input =
    .placeholder = آدرس مینت توکن را وارد کنید...
tools-trade-watcher-action-search-pools = جست‌وجوی استخرها
tools-trade-watcher-pool-label = استخر انتخاب‌شده
tools-trade-watcher-pool-none = استخری انتخاب نشده
tools-trade-watcher-pool-clear =
    .title = پاک کردن استخر
# $dex is the DEX name, $base and $quote are the pair symbols.
tools-trade-watcher-pool-selected = استخر انتخاب‌شده: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = نوع پایش
tools-trade-watcher-type-hint = خرید هنگام فروش: با فروش دیگران، به‌طور خودکار خرید انجام می‌شود. فروش هنگام خرید: با خرید دیگران، به‌طور خودکار فروش انجام می‌شود.
tools-trade-watcher-trigger-label = مقدار محرک
tools-trade-watcher-trigger-hint = حداقل اندازه معامله به { -sol } برای اجرای اقدام
tools-trade-watcher-action-amount-label = مقدار اقدام
tools-trade-watcher-action-amount-hint = مقدار خرید/فروش هنگام فعال شدن محرک
tools-trade-watcher-slippage-label = اسلیپیج
tools-trade-watcher-slippage-hint = حداکثر اسلیپیج قابل‌قبول برای معاملات
tools-trade-watcher-active-title = پایش‌های فعال
tools-trade-watcher-empty = پایش فعالی وجود ندارد
tools-trade-watcher-empty-hint = یک پایش را در بالا تنظیم کنید و برای شروع، روی «شروع پایش» کلیک کنید
tools-trade-watcher-action-start = شروع پایش
tools-trade-watcher-action-starting = در حال شروع...
tools-trade-watcher-action-stop-all = توقف همه
tools-trade-watcher-action-stopping = در حال توقف...
# $token is the token symbol or the start of its mint address.
tools-trade-watcher-started = پایش برای { $token } شروع شد...
tools-trade-watcher-start-failed = شروع پایش ناموفق بود
tools-trade-watcher-stopped = پایش متوقف شد
tools-trade-watcher-stop-failed = توقف پایش ناموفق بود
tools-trade-watcher-stopped-all = همه پایش‌ها متوقف شدند
tools-trade-watcher-stop-all-failed = توقف پایش‌ها ناموفق بود
tools-trade-watcher-load-failed = بارگیری پایش‌ها ناموفق بود
tools-trade-watcher-column-token = توکن
tools-trade-watcher-column-type = نوع
tools-trade-watcher-column-trigger = محرک ({ -sol })
tools-trade-watcher-column-action = اقدام ({ -sol })
tools-trade-watcher-column-triggered = فعال‌شده
tools-trade-watcher-stop-watch =
    .title = توقف پایش

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = برن { -sol } ممکن نیست
tools-burn-failure-open-position = برن توکن‌های پوزیشن باز ممکن نیست
tools-burn-failure-account-not-found = حساب توکن پیدا نشد
tools-burn-failure-zero-balance = موجودی توکن از قبل صفر است
tools-burn-failure-transaction = تراکنش ناموفق بود
tools-burn-warning-open-position = برن توکن‌های پوزیشن باز ممکن نیست
tools-burn-warning-closed-position = باقی‌مانده از پوزیشن بسته
# $amount is the token value in SOL with six decimals.
tools-burn-warning-worth = ارزش تقریبی { $amount } { -sol }
# $needed and $have are SOL amounts with four decimals.
tools-multi-buy-warning-insufficient = موجودی کافی نیست. به { $needed } { -sol } نیاز است، موجودی: { $have } { -sol }
# $needed and $limit are SOL amounts with four decimals.
tools-multi-buy-warning-over-limit = مجموع { -sol } موردنیاز ({ $needed }) از سقف ({ $limit }) بیشتر است
tools-multi-sell-warning-no-wallets = کیف پول ثانویه‌ای پیدا نشد
tools-multi-sell-warning-no-balance = هیچ کیف پولی موجودی توکن ندارد
tools-multi-op-buy-failed = خرید ناموفق بود
tools-multi-op-sell-failed = فروش ناموفق بود
tools-multi-op-transfer-failed = انتقال ناموفق بود
tools-multi-op-balance-failed = دریافت موجودی ناموفق بود
tools-multi-op-mint-invalid = آدرس مینت نامعتبر است
tools-multi-buy-session-failed = خرید چندگانه ناموفق بود
tools-multi-sell-session-failed = فروش چندگانه ناموفق بود
tools-multi-session-aborted = عملیات توسط کاربر متوقف شد

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = اسکن کیف پول
tools-wallet-action-scanning = در حال اسکن...
# $reason is the technical cause of the failure.
tools-wallet-scan-failed = اسکن ناموفق بود: { $reason }
# $amount is an amount with its unit.
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] انتخاب‌شده: { $count } کیف پول
       *[other] انتخاب‌شده: { $count } کیف پول
    }
tools-wallet-transfer-failed = انتقال ناموفق بود: { $reason }
tools-wallet-cleanup-failed = پاکسازی ناموفق بود: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = نتایج اسکن
tools-wallet-cleanup-stat-empty = ATAهای خالی
tools-wallet-cleanup-stat-reclaimable = { -sol } قابل بازپس‌گیری
tools-wallet-cleanup-stat-failed = ناموفق (ذخیره‌شده)
tools-wallet-cleanup-prompt = برای یافتن ATAهای خالی، روی «اسکن کیف پول» کلیک کنید
tools-wallet-cleanup-prompt-hint = همه حساب‌های توکن کیف پول شما بررسی می‌شود
tools-wallet-cleanup-action-cleanup = پاکسازی همه
tools-wallet-cleanup-action-cleaning = در حال پاکسازی...
tools-wallet-cleanup-scanning = در حال اسکن کیف پول...
# $amount is the reclaimable rent with its unit.
tools-wallet-cleanup-found =
    { $count ->
        [one] { $count } ATA خالی با ارزش تقریبی { $amount } پیدا شد
       *[other] { $count } ATA خالی با ارزش تقریبی { $amount } پیدا شد
    }
tools-wallet-cleanup-clean = ATA خالی پیدا نشد - کیف پول تمیز است!
tools-wallet-cleanup-scan-failed = اسکن ATAها ناموفق بود
tools-wallet-cleanup-done =
    { $count ->
        [one] { $count } ATA پاکسازی شد
       *[other] { $count } ATA پاکسازی شد
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = برن توکن
tools-burn-info-title = برن چیست؟
tools-burn-info-body = برن، توکن‌ها را برای همیشه نابود می‌کند و بازیابی آن‌ها ممکن نیست. پس از برن، «پاکسازی کیف پول» را اجرا کنید تا ATAهای خالی بسته شوند و به ازای هر توکن حدود 0.002 { -sol } رنت بازپس گرفته شود.
tools-burn-stat-total = کل توکن‌ها
tools-burn-stat-selected = انتخاب‌شده
tools-burn-stat-rent = رنت قابل بازپس‌گیری
tools-burn-prompt = برای یافتن توکن‌ها، روی «اسکن کیف پول» کلیک کنید
tools-burn-scanning = در حال اسکن کیف پول برای یافتن توکن‌ها...
tools-burn-scan-failed = اسکن توکن‌ها ناموفق بود
tools-burn-empty = توکنی در کیف پول پیدا نشد
# $count is the number of selected tokens.
tools-burn-action-burn = برن انتخاب‌شده‌ها ({ $count })
tools-burn-action-burning = در حال برن...
tools-burn-cannot-burn = قابل برن نیست
tools-burn-no-value = بدون ارزش

# Category titles and descriptions. Ids are the token categories of the scan.
tools-burn-category-open-position = پوزیشن‌های باز
tools-burn-category-has-value = دارای ارزش
tools-burn-category-closed-position = پوزیشن‌های بسته
tools-burn-category-zero-liquidity = نقدینگی صفر
tools-burn-category-hint-open-position = برن توکن‌های پوزیشن باز ممکن نیست
tools-burn-category-hint-has-value = به‌جای برن، فروش را در نظر بگیرید
tools-burn-category-hint-closed-position = باقی‌مانده از معاملات بسته‌شده
tools-burn-category-hint-zero-liquidity = برن امن است - ارزش بازاری ندارد

tools-burn-confirm-title = تأیید برن
tools-burn-confirm-message =
    { $count ->
        [one] آیا مطمئنید که می‌خواهید <strong>{ $count }</strong> توکن را برن کنید؟
       *[other] آیا مطمئنید که می‌خواهید <strong>{ $count }</strong> توکن را برن کنید؟
    }
# $amount is the estimated value with its unit.
tools-burn-confirm-value = مجموع ارزش تخمینی: <strong>{ $amount }</strong>
tools-burn-confirm-continue = ادامه
tools-burn-final-title = هشدار نهایی
tools-burn-final-headline = این عمل غیرقابل بازگشت است!
tools-burn-final-message =
    { $count ->
        [one] { $count } توکن زیر برای همیشه نابود می‌شود و به هیچ وجه قابل بازیابی نیست.
       *[other] { $count } توکن زیر برای همیشه نابود می‌شوند و به هیچ وجه قابل بازیابی نیستند.
    }
tools-burn-final-confirm = بله، برن شود
# $successful and $total count tokens; $amount is the reclaimable rent with its unit.
tools-burn-toast-burned =
    { $total ->
        [one] { $successful } از { $total } توکن برن شد. برای بازپس‌گیری حدود { $amount }، «پاکسازی کیف پول» را اجرا کنید
       *[other] { $successful } از { $total } توکن برن شد. برای بازپس‌گیری حدود { $amount }، «پاکسازی کیف پول» را اجرا کنید
    }
tools-burn-toast-failed =
    { $count ->
        [one] برن { $count } توکن ناموفق بود
       *[other] برن { $count } توکن ناموفق بود
    }
tools-burn-failed = برن ناموفق بود: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] { $count } توکن برن نشد
       *[other] { $count } توکن برن نشد
    }
tools-burn-failure-unknown = دلیلی گزارش نشد

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = درباره
tools-airdrop-about-body = ایردراپ‌های در انتظار، پاداش‌های قابل دریافت و سهمیه‌های دریافت‌نشده را در پروتکل‌های پرکاربرد سولانا بررسی کنید.
tools-airdrop-list-title = ایردراپ‌های موجود
tools-airdrop-prompt = برای اسکن دریافتی‌های موجود، روی «بررسی ایردراپ» کلیک کنید
tools-airdrop-action-check = بررسی ایردراپ
tools-airdrop-action-claim-all = دریافت همه

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = گزینه‌های تولید
tools-generator-warning-title = کلیدهای خصوصی خود را ایمن نگه دارید!
tools-generator-warning-body = جفت‌کلیدها به‌صورت محلی ساخته می‌شوند و هرگز ارسال نمی‌شوند. حتماً از کلیدهای خود در جای امنی نسخه پشتیبان تهیه کنید.
tools-generator-count-label = تعداد کیف پول
tools-generator-vanity-label = آدرس اختصاصی (شروع با کاراکترهای مشخص)
tools-generator-prefix-label = پیشوند
tools-generator-prefix-input =
    .placeholder = مثلاً SOL
tools-generator-prefix-hint = پیشوندهای طولانی‌تر به‌صورت نمایی زمان بیشتری برای تولید نیاز دارند
tools-generator-list-title = کیف پول‌های تولیدشده
tools-generator-empty = هنوز کیف پولی تولید نشده
tools-generator-action-generate = تولید
tools-generator-action-generating = در حال تولید...
tools-generator-count-invalid = لطفاً عددی بین 1 و 10 وارد کنید
tools-generator-no-keypairs = جفت‌کلیدی برنگشت
tools-generator-generated =
    { $count ->
        [one] { $count } کیف پول تولید شد
       *[other] { $count } کیف پول تولید شد
    }
tools-generator-failed = تولید کیف پول ناموفق بود: { $reason }
tools-generator-copy-public-key =
    .title = کپی کلید عمومی
tools-generator-copy-private-key =
    .title = کپی کلید خصوصی
tools-generator-remove =
    .title = حذف از فهرست
tools-generator-reveal =
    .title = نمایش کلید خصوصی
tools-generator-public-key-label = کلید عمومی:
tools-generator-private-key-label = کلید خصوصی:
# Names the copied value in the shared copied toast.
tools-generator-public-key-name = کلید عمومی
tools-generator-private-key-copied = کلید خصوصی کپی شد
tools-generator-private-key-warning = هر کسی که این کلید را داشته باشد، کیف پول را در اختیار دارد
tools-generator-export-empty = کیف پولی برای خروجی گرفتن وجود ندارد
tools-generator-exported = خروجی کیف پول‌ها ذخیره شد - آن را ایمن نگه دارید

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = خلاصه
tools-consolidation-stat-wallets = کیف پول‌های فرعی
tools-consolidation-stat-native = مجموع { -sol }
tools-consolidation-stat-tokens = انواع توکن
tools-consolidation-stat-rent = رنت قابل بازپس‌گیری
tools-consolidation-wallets-title = کیف پول‌ها
tools-consolidation-loading-wallets = در حال بارگیری کیف پول‌ها...
tools-consolidation-loading-data = در حال بارگیری داده‌های کیف پول...
tools-consolidation-action-transfer-native = انتقال { -sol }
tools-consolidation-action-transfer-tokens = انتقال همه توکن‌ها
tools-consolidation-action-cleanup = پاکسازی ATAها
tools-consolidation-action-transferring = در حال انتقال...
tools-consolidation-column-name = نام
tools-consolidation-column-native = موجودی ({ -sol })
tools-consolidation-column-tokens = توکن‌ها
tools-consolidation-column-atas = ATAهای خالی
tools-consolidation-empty = کیف پول فرعی پیدا نشد
tools-consolidation-empty-hint = برای شروع، با «خرید چندگانه» کیف پول فرعی بسازید
tools-consolidation-load-failed = بارگیری ناموفق بود: { $reason }
tools-consolidation-select-prompt = کیف پول‌ها را برای تجمیع انتخاب کنید
# $amount is the selected balance with its unit.
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } توکن
       *[other] { $tokens } توکن
    } | { $atas ->
        [one] { $atas } ATA خالی
       *[other] { $atas } ATA خالی
    }
# $amount is the transferred balance with its unit.
tools-consolidation-transferred-native = { $amount } به کیف پول اصلی منتقل شد
tools-consolidation-transferred-tokens =
    { $count ->
        [one] { $count } توکن به کیف پول اصلی منتقل شد
       *[other] { $count } توکن به کیف پول اصلی منتقل شد
    }
# $amount is the reclaimed rent with its unit.
tools-consolidation-cleaned =
    { $count ->
        [one] { $count } ATA بسته شد و { $amount } بازپس گرفته شد
       *[other] { $count } ATA بسته شد و { $amount } بازپس گرفته شد
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = توکن
tools-multi-mint-label = آدرس مینت توکن
tools-multi-mint-input =
    .placeholder = آدرس مینت توکن را جای‌گذاری کنید...
tools-multi-execution-title = تنظیمات اجرا
tools-multi-delay-min-label = حداقل تأخیر
tools-unit-native = { -sol }
tools-unit-seconds = ثانیه
tools-unit-ms = ms
tools-multi-delay-max-label = حداکثر تأخیر
tools-multi-concurrency-label = هم‌زمانی
tools-multi-concurrency-sequential = { $count } (ترتیبی)
tools-multi-concurrency-parallel = { $count } موازی
tools-multi-slippage-label = اسلیپیج
tools-multi-router-label = روتر
tools-multi-router-auto = خودکار (بهترین مسیر)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = استخر مستقیم
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = پیشرفت
tools-multi-progress-preparing = در حال آماده‌سازی...
# $label is the session state, $completed and $total count wallet operations.
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = کیف پول
tools-multi-column-route = مسیر
tools-multi-column-status = وضعیت
tools-multi-op-completed = تکمیل‌شده
tools-multi-op-failed = ناموفق
tools-multi-action-stop = توقف
tools-multi-action-loading = در حال بارگیری...
# $reason is the technical cause of the failure.
tools-multi-start-failed = شروع ناموفق بود: { $reason }

# Session states. Ids are the states of a multi-wallet session.
tools-multi-state-pending = در انتظار
tools-multi-state-funding = در حال تأمین موجودی
tools-multi-state-executing = در حال اجرا
tools-multi-state-consolidating = در حال تجمیع
tools-multi-state-completed = تکمیل‌شده
tools-multi-state-failed = ناموفق
tools-multi-state-aborted = متوقف‌شده

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = توکنی که می‌خواهید در چند کیف پول بخرید
tools-multi-buy-wallets-title = تنظیمات کیف پول
tools-multi-buy-wallet-count-label = تعداد کیف پول
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } کیف پول
       *[other] { $count } کیف پول
    }
tools-multi-buy-wallet-count-hint = تعداد کیف پول‌های فرعی مورد استفاده
tools-multi-buy-buffer-label = ذخیره { -sol } برای هر کیف پول
tools-multi-buy-buffer-hint = برای کارمزدها کنار گذاشته می‌شود (حداقل 0.015 { -sol })
tools-multi-buy-amounts-title = تنظیمات مقدار
tools-multi-buy-min-label = حداقل { -sol } برای هر کیف پول
tools-multi-buy-min-hint = حداقل مقدار خرید
tools-multi-buy-max-label = حداکثر { -sol } برای هر کیف پول
tools-multi-buy-max-hint = حداکثر مقدار خرید
tools-multi-buy-limit-label = سقف کل { -sol } (اختیاری)
tools-multi-buy-limit-hint = حداکثر مجموع هزینه
tools-multi-buy-preview-title = پیش‌نمایش
tools-multi-buy-preview-create = کیف پول‌های قابل ساخت
tools-multi-buy-preview-amount = مقدار برای هر کیف پول
# $min and $max are amounts with their unit.
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = مجموع { -sol } موردنیاز
tools-multi-buy-preview-balance = موجودی اصلی
tools-multi-buy-action-preview = پیش‌نمایش
tools-multi-buy-action-start = شروع خرید چندگانه
tools-multi-buy-executing = در حال اجرای خریدها...
tools-multi-buy-column-spent = خرج‌شده ({ -sol })
tools-multi-buy-column-tokens = توکن‌ها
tools-multi-buy-preview-failed = پیش‌نمایش ناموفق بود: { $reason }
tools-multi-buy-started = خرید چندگانه شروع شد
tools-multi-buy-stopped = خرید چندگانه متوقف شد
tools-multi-buy-completed = خرید چندگانه تکمیل شد! { $successful } از { $total } موفق

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = برای اسکن کیف پول‌های دارای توکن، آدرس آن را وارد کنید
tools-multi-sell-action-scan = اسکن
tools-multi-sell-settings-title = تنظیمات فروش
tools-multi-sell-percent-label = درصد فروش
tools-multi-sell-percent-hint = درصد توکن برای فروش از هر کیف پول
tools-multi-sell-min-fee-label = حداقل { -sol } برای کارمزد
tools-multi-sell-min-fee-hint = حداقل { -sol } موردنیاز برای کارمزد تراکنش
tools-multi-sell-topup-label = شارژ خودکار در صورت نیاز
tools-multi-sell-topup-hint = اگر موجودی کیف پول فرعی کافی نباشد، { -sol } از کیف پول اصلی منتقل می‌شود
tools-multi-sell-post-title = اقدام‌های پس از فروش
tools-multi-sell-consolidate-label = تجمیع { -sol } در کیف پول اصلی
tools-multi-sell-consolidate-hint = انتقال همه { -sol } کیف پول‌های فرعی به کیف پول اصلی
tools-multi-sell-close-atas-label = بستن ATAهای توکن پس از فروش
tools-multi-sell-close-atas-hint = بازپس‌گیری حدود 0.002 { -sol } به ازای هر ATA
tools-multi-sell-wallets-title = کیف پول‌های دارای توکن
tools-multi-sell-empty = هیچ کیف پول فرعی این توکن را ندارد
tools-multi-sell-column-tokens = توکن‌ها
tools-multi-sell-column-native = موجودی ({ -sol })
tools-multi-sell-column-topup = نیازمند شارژ
tools-multi-sell-none-selected = کیف پولی انتخاب نشده
tools-multi-sell-select-required = لطفاً دست‌کم یک کیف پول انتخاب کنید
tools-multi-sell-action-start = شروع فروش چندگانه
tools-multi-sell-executing = در حال اجرای فروش‌ها...
tools-multi-sell-column-sold = توکن‌های فروخته‌شده
tools-multi-sell-column-received = دریافتی ({ -sol })
tools-multi-sell-started = فروش چندگانه شروع شد
tools-multi-sell-stopped = فروش چندگانه متوقف شد
# $amount is the received amount with its unit.
tools-multi-sell-completed = فروش چندگانه تکمیل شد! { $amount } دریافت شد

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = علاقه‌مندی‌ها
tools-favorites-saved = علاقه‌مندی‌های ذخیره‌شده
tools-favorites-save-current = ذخیره مورد فعلی
tools-favorites-empty = هنوز علاقه‌مندی‌ای ذخیره نشده
tools-favorites-no-label = بدون برچسب
# $count is how many times the favorite was used.
tools-favorites-uses = { $count } بار
tools-favorites-remove = حذف
# $name is the favorite's label or symbol.
tools-favorites-loaded = علاقه‌مندی بارگیری شد: { $name }
tools-favorites-default-name = پیکربندی
tools-favorites-mint-required = ابتدا آدرس مینت توکن را وارد کنید
tools-favorites-add-title = افزودن علاقه‌مندی
tools-favorites-add-message = برای این علاقه‌مندی یک برچسب وارد کنید
tools-favorites-add-placeholder = برچسب (اختیاری)...
tools-favorites-saved-toast = در علاقه‌مندی‌ها ذخیره شد
tools-favorites-save-failed = ذخیره علاقه‌مندی ناموفق بود
tools-favorites-remove-title = حذف علاقه‌مندی
tools-favorites-remove-message = این علاقه‌مندی حذف شود؟
tools-favorites-removed-toast = علاقه‌مندی حذف شد
tools-favorites-remove-failed = حذف علاقه‌مندی ناموفق بود
