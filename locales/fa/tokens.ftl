# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = داده زنده بازار
tokens-result-source-unavailable = { $label } در دسترس نیست — در حال تلاش مجدد
tokens-result-source-not-listed = در { $label } فهرست نشده است
tokens-result-security-available = گزارش امنیتی موجود است
tokens-result-security-missing = گزارش { -rugcheck } موجود نیست
tokens-result-chart-available = داده نمودار موجود است
tokens-result-chart-missing = هنوز داده نمودار موجود نیست

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = بارگیری داده ممکن نشد
tokens-state-offline = به نظر می‌رسد آفلاین هستید.
tokens-state-request-failed = درخواست پس از چند تلاش ناموفق ماند.
tokens-state-waiting = در انتظار داده…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = ورود
tokens-chart-level-stop-loss = حد ضرر
tokens-chart-level-take-profit = حد سود

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = در حال بارگیری تراکنش‌ها…
tokens-transactions-empty-title = تراکنشی نیست
tokens-transactions-empty-history = تاریخچه تراکنش کیف پول برای این توکن موجود نیست.
tokens-transactions-empty-data = داده تراکنش برای این توکن موجود نیست.
tokens-transactions-error-title = بارگیری تراکنش‌ها ممکن نشد
tokens-transactions-error-message = تاریخچه تراکنش‌ها موقتاً در دسترس نیست.
tokens-transactions-activity-title = فعالیت 24h
tokens-transactions-activity-subtitle = تراکنش‌های ساعتی کیف پول
tokens-transactions-metric-total = مجموع
tokens-transactions-metric-buys = خریدها
tokens-transactions-metric-sells = فروش‌ها
tokens-transactions-recent-title = تراکنش‌های اخیر
# $count is the number of rows listed.
tokens-transactions-shown = { $count } مورد نمایش داده شد
tokens-transactions-column-time = زمان
tokens-transactions-column-type = نوع
tokens-transactions-column-price = قیمت
tokens-transactions-column-total = مجموع
tokens-transactions-chart-missing = کتابخانه نمودار موجود نیست
tokens-transactions-view-solscan = مشاهده تراکنش در { -solscan }

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = بدون پوزیشن
tokens-positions-no-token = توکنی انتخاب نشده است.
tokens-positions-empty-message = هنوز پوزیشنی برای این توکن وجود ندارد. برای باز کردن پوزیشن از «خرید» استفاده کنید.
tokens-positions-loading = در حال بارگیری پوزیشن…
tokens-positions-from-wallet-history = از تاریخچه کیف پول
tokens-positions-frozen = فریزشده — قابل فروش نیست
tokens-positions-no-cost-basis = بدون قیمت تمام‌شده
tokens-positions-history-incomplete = تاریخچه ناقص
# $count is the number of DCA buys.
tokens-positions-dca-count = DCA { $count }
# $count is the number of partial exits.
tokens-positions-exit-count = خروج‌ها { $count }
tokens-positions-fact-avg-entry = میانگین ورود
tokens-positions-fact-current = فعلی
tokens-positions-fact-tokens = توکن‌ها
tokens-positions-fact-opened = زمان باز شدن
tokens-positions-fact-exit-price = قیمت خروج
tokens-positions-fact-sol-received = { -sol } دریافتی
tokens-positions-fact-closed-reason = دلیل بسته شدن
tokens-positions-fact-target-min = حداقل هدف سود
tokens-positions-fact-target-max = حداکثر هدف سود
tokens-positions-fact-highest = بالاترین قیمت
tokens-positions-fact-lowest = پایین‌ترین قیمت
tokens-positions-section-range = اهداف و بازه
tokens-positions-section-market = بازار و دارایی‌ها
tokens-positions-kicker = پوزیشن
tokens-positions-fallback-symbol = توکن
tokens-positions-realized-pnl = سود و زیان تحقق‌یافته
tokens-positions-unrealized-pnl = سود و زیان تحقق‌نیافته
tokens-positions-size = اندازه

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = تحلیل { -rugcheck } در حال انجام است...
tokens-security-analyzing = در حال تحلیل امنیت…
tokens-security-pulse-title = نبض امنیتی
tokens-security-pending-caption = سیگنال‌های ریسک هنوز در حال جمع‌آوری هستند.
tokens-security-control-title = کنترل توکن
tokens-security-control-meta = وضعیت اختیارات
# $time is the formatted time of the last security update.
tokens-security-updated = به‌روزرسانی: { $time }
tokens-security-score-caption = امتیاز نرمال‌شده ریسک توکن از 100.
tokens-security-score-label = امتیاز
tokens-security-rugged = راگ‌شده
tokens-security-grade-analyzing = در حال تحلیل
tokens-security-grade-shielded = محافظت‌شده
tokens-security-grade-safe = امن
tokens-security-grade-caution = احتیاط
tokens-security-grade-vulnerable = آسیب‌پذیر
tokens-security-grade-unknown = نامشخص
tokens-security-metric-token-type = نوع توکن
tokens-security-metric-total-holders = کل هولدرها
tokens-security-metric-lp-providers = تأمین‌کنندگان LP
tokens-security-metric-graph-insiders = اینسایدرهای گراف
# $count is the number of insider wallets found in the holder graph.
tokens-security-insiders-detected = شناسایی شد ({ $count })
tokens-security-insiders-clean = پاک
tokens-security-authority-mint = مینت
tokens-security-authority-freeze = فریز
tokens-security-authority-immutable = غیرقابل تغییر
tokens-security-authority-mutable = قابل تغییر
tokens-security-authority-revoked = لغوشده
tokens-security-authority-active = فعال
tokens-security-holder-health-title = سلامت هولدرها
tokens-security-holders-unique = یکتا
tokens-security-creator-share = سهم سازنده
tokens-security-gauge-top-10 = 10 برتر
tokens-security-concentration-unknown = نامشخص
tokens-security-concentration-critical = بحرانی
tokens-security-concentration-high = زیاد
tokens-security-concentration-moderate = متوسط
tokens-security-concentration-healthy = سالم
tokens-security-transfer-title = مالیات انتقال
tokens-security-transfer-no-fee = بدون کارمزد
tokens-security-transfer-fee-percentage = درصد کارمزد
tokens-security-transfer-max-fee = حداکثر مقدار کارمزد
tokens-security-transfer-authority = اختیار کارمزد
# $percent is the formatted transfer fee percentage.
tokens-security-transfer-note = روی هر انتقال، کارمزدی برابر { $percent } کسر می‌شود.
tokens-security-transfer-none = کارمزد انتقالی شناسایی نشد.
tokens-security-risks-title = ریسک‌های امنیتی
tokens-security-risks-none = ریسک امنیتی شناسایی نشد.
tokens-security-risk-fallback-name = سیگنال امنیتی
tokens-security-risks-critical = { $count } بحرانی
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } هشدار
       *[other] { $count } هشدار
    }
tokens-security-risks-info = { $count } اطلاعاتی
tokens-security-risks-incidents =
    { $count ->
        [one] { $count } رخداد پیدا شد
       *[other] { $count } رخداد پیدا شد
    }
tokens-security-top-holders-title = هولدرهای برتر
# $percent is the formatted share of supply held by the top holders.
tokens-security-top-holders-concentration = تمرکز { $percent }
tokens-security-insider = اینسایدر

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = در حال بررسی داده…
tokens-overview-banner-open = باز کردن بنر توکن
tokens-overview-headline-label = معیارهای اصلی بازار
tokens-overview-price = قیمت
tokens-overview-market-cap = ارزش بازار
tokens-overview-liquidity = نقدینگی
tokens-overview-volume = حجم
tokens-overview-volume-24h = حجم 24H
tokens-overview-no-tags = بدون برچسب
tokens-overview-info-title = اطلاعات توکن
tokens-overview-profile = پروفایل منتشرشده
tokens-overview-fact-mint = مینت
tokens-overview-fact-decimals = اعشار
tokens-overview-fact-age = عمر
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = هولدرها
tokens-overview-fact-top-10 = سهم 10 هولدر برتر
tokens-overview-tags = برچسب‌ها
tokens-overview-liquidity-title = نقدینگی و بازار
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-sol = { -sol } استخر
tokens-overview-fact-pool-token = توکن استخر
tokens-overview-pool = استخر
tokens-overview-pulse-title = نبض بازار
tokens-overview-activity-title = فعالیت تراکنش
# $percent is the formatted share of buys among the 24h transactions.
tokens-overview-buy-share = خرید { $percent }
# $ratio is the formatted buy-to-sell ratio.
tokens-overview-buy-sell-ratio = { $ratio } B/S
tokens-overview-buys-24h = خرید 24H
tokens-overview-sells-24h = فروش 24H
tokens-overview-net-flow = جریان خالص
tokens-overview-total-24h = مجموع 24H
tokens-overview-average-24h = میانگین 24H
tokens-overview-spike-5m = جهش 5M
# $amount is the formatted average number of transactions per hour.
tokens-overview-rate-per-hour = { $amount }/h
# $amount is the formatted average number of transactions per minute.
tokens-overview-rate-per-minute = { $amount }/دقیقه
# $factor is the formatted ratio of the 5-minute rate to the 1-hour rate.
tokens-overview-spike-factor = { $factor }×
# Tooltip of one activity row. Counts and percentages are formatted; "—" marks a missing value.
tokens-overview-flow-counts = خرید: { $buys } ({ $buyPercent })، فروش: { $sells } ({ $sellPercent })، مجموع: { $total }
tokens-overview-flow-no-data = داده تراکنشی موجود نیست

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = بدون استخر
tokens-pools-empty-message = هیچ استخر نقدینگی‌ای برای این توکن شناسایی نشده است.
tokens-pools-unknown = نامشخص
tokens-pools-unknown-dex = DEX نامشخص
tokens-pools-total = کل استخرها
tokens-pools-liquidity = نقدینگی
tokens-pools-volume-24h = حجم 24h
tokens-pools-base-role = نقش پایه
tokens-pools-quote-role = نقش مظنه
tokens-pools-canonical-title = استخر مرجع
tokens-pools-canonical = مرجع
tokens-pools-dex = DEX
tokens-pools-summary-title = خلاصه استخرها
tokens-pools-breakdown-title = تفکیک بر اساس DEX
tokens-pools-all-title = همه استخرها
tokens-pools-updated = به‌روزرسانی
tokens-pools-role-base = پایه
tokens-pools-role-quote = مظنه
tokens-pools-role-unknown = نامشخص
tokens-pools-reserves = حساب‌های ذخیره
tokens-pools-no-reserves = حساب ذخیره‌ای نیست
tokens-pools-address-copy = کپی آدرس
tokens-pools-address-pool = استخر
    .title = کپی آدرس استخر
tokens-pools-address-base = مینت پایه
    .title = کپی مینت پایه
tokens-pools-address-quote = مینت مظنه
    .title = کپی مینت مظنه
tokens-pools-address-paired = مینت جفت
    .title = کپی مینت جفت

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = وب‌سایت رسمی یا لینک شبکه اجتماعی برای این توکن موجود نیست.
tokens-links-info-title = اطلاعات توکن
tokens-links-mint-address = آدرس مینت
tokens-links-data-source = منبع داده
tokens-links-security = امنیت
tokens-links-profile-title = پروفایل توکن
tokens-links-profile-published-title = محتوای پروفایل منتشرشده
tokens-links-profile-published-note = رسانه، توضیحات و لینک‌های رسمی، محتوای پولی پروفایل هستند که پیش از انتشار بررسی می‌شوند. این کار مالکیت یا ایمنی توکن را تأیید نمی‌کند.
tokens-links-profile-create-note = لوگو، توضیحات پروژه و لینک‌های رسمیِ بررسی‌شده را به پروفایل عمومی این توکن اضافه کنید.
tokens-links-profile-update-hint = پروفایل این توکن را در screenerbot.io به‌روزرسانی کنید
tokens-links-profile-create-hint = در screenerbot.io برای توکن پروفایل بسازید
tokens-links-profile-update = به‌روزرسانی پروفایل
tokens-links-profile-create = ساخت پروفایل
tokens-links-media-title = دارایی‌های رسانه‌ای
tokens-links-media-fallback-symbol = توکن
tokens-links-media-logo = لوگو
tokens-links-media-banner = بنر
# $symbol is the token symbol.
tokens-links-media-banner-alt = بنر { $symbol }
tokens-links-media-open = باز کردن تصویر
tokens-links-description-title = توضیحات
tokens-links-explorers-title = اکسپلوررها و تحلیل‌ها
tokens-links-websites-title = وب‌سایت‌های رسمی
tokens-links-socials-title = شبکه‌های اجتماعی
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = شبکه اجتماعی

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = نمای کلی
tokens-dialog-tab-security = امنیت
tokens-dialog-tab-positions = پوزیشن‌ها
tokens-dialog-tab-pools = استخرها
tokens-dialog-tab-links = لینک‌ها
tokens-dialog-tab-transactions = تراکنش‌ها
tokens-dialog-sections = بخش‌های جزئیات توکن
tokens-dialog-close =
    .title = بستن (ESC)
    .aria-label = بستن جزئیات توکن
tokens-dialog-unknown-symbol = نامشخص
tokens-dialog-unknown-name = توکن ناشناس
tokens-dialog-market-summary = خلاصه بازار
tokens-dialog-price-loading = در حال بارگیری قیمت
tokens-dialog-unit-sol = { -sol }
tokens-dialog-market-metrics = معیارهای بازار
tokens-dialog-metric-market-cap = ارزش بازار
tokens-dialog-metric-volume-24h = حجم 24h
# $change is the formatted 24 hour price change.
tokens-dialog-change-24h = تغییر 24 ساعته { $change }
tokens-dialog-buy = خرید
    .title = خرید این توکن
tokens-dialog-sell = فروش
    .title = فروش پوزیشن
tokens-dialog-sell-unavailable = پوزیشن بازی برای فروش وجود ندارد
tokens-dialog-details = جزئیات
tokens-dialog-sources = منابع
tokens-dialog-sources-status = وضعیت منابع داده
tokens-dialog-updated-label = به‌روزرسانی
tokens-dialog-just-now = هم‌اکنون
# $time is a relative or clock time.
tokens-dialog-updated-at = به‌روزرسانی: { $time }
tokens-dialog-updated-unavailable = زمان به‌روزرسانی در دسترس نیست
tokens-dialog-error-title = بارگیری داده توکن ممکن نشد
tokens-dialog-waiting-token = در انتظار داده توکن…
tokens-dialog-loading-overview = در حال بارگیری نمای کلی…
tokens-dialog-loading-security = در حال بارگیری امنیت…
tokens-dialog-loading-pools = در حال بارگیری استخرها…
tokens-dialog-loading-links = در حال بارگیری لینک‌ها…
tokens-dialog-chart-still-checking = هنوز داده نمودار موجود نیست — بررسی ادامه دارد…
tokens-dialog-no-data = داده‌ای موجود نیست
tokens-dialog-source-token = توکن
tokens-dialog-source-market = بازار
tokens-dialog-source-security = امنیت
tokens-dialog-source-chart = نمودار
tokens-dialog-status-pending = در انتظار
tokens-dialog-status-loading = در حال بارگیری
tokens-dialog-status-ready = آماده
tokens-dialog-status-unavailable = در دسترس نیست
tokens-dialog-status-cached = ذخیره‌شده
# $source is a data source name and $status its state, for example "Market data: Ready".
tokens-dialog-source-summary = داده { $source }: { $status }
tokens-dialog-badge-pool-price = قیمت استخر
tokens-dialog-badge-pool-price-hint = قیمت از استخر لحظه‌ای روی زنجیره
tokens-dialog-badge-api-price = قیمت API
tokens-dialog-badge-api-price-hint = قیمت از داده بازار ذخیره‌شده (API)
tokens-dialog-badge-profile = پروفایل منتشرشده
    .title = محتوای پولی پروفایل که پیش از انتشار بررسی شده است؛ ممیزی یا تأیید مالکیت نیست.
tokens-dialog-badge-low-risk-hint = ریسک کم بر اساس امتیاز فعلی { -rugcheck }؛ تأیید هویت نیست.
tokens-dialog-badge-immutable = غیرقابل تغییر
tokens-dialog-badge-mutable = قابل تغییر
tokens-dialog-badge-auth = اختیار:
tokens-dialog-badge-update-authority = اختیار به‌روزرسانی:
tokens-dialog-badge-position = پوزیشن
tokens-dialog-badge-blacklisted = فهرست سیاه

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = علاقه‌مندی‌ها
tokens-view-pool = سرویس استخر
tokens-view-no-market = بدون داده بازار
tokens-view-all = همه توکن‌ها
tokens-view-passed = تأییدشده
tokens-view-rejected = ردشده
tokens-view-blacklisted = فهرست سیاه
tokens-view-positions = پوزیشن‌ها
tokens-view-recent = اخیر
tokens-view-ohlcv = داده OHLCV

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = برای بزرگ‌نمایی کلیک کنید
# $boosts is the formatted active boost count.
tokens-boost-title = { $boosts } بار در screenerbot.io بوست شده
tokens-cell-action-add =
    .title = افزودن به پوزیشن (DCA)
    .aria-label = افزودن به پوزیشن
tokens-cell-action-sell =
    .title = فروش (کامل یا جزئی به درصد)
    .aria-label = فروش توکن
tokens-cell-action-buy =
    .title = خرید پوزیشن
    .aria-label = خرید توکن
tokens-cell-external-links =
    .title = لینک‌های خارجی
    .aria-label = لینک‌های خارجی

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = در حال بارگیری توکن‌ها…
tokens-table-loading-description = در حال آماده‌سازی نمای توکن انتخاب‌شده.
tokens-table-retry-hint = تب را عوض کنید یا دوباره تلاش کنید.
tokens-filter-all = همه

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = بارگیری علاقه‌مندی‌ها ممکن نشد
tokens-favorites-load-failed-toast = بارگیری علاقه‌مندی‌ها ممکن نشد
tokens-favorites-total = کل علاقه‌مندی‌ها
tokens-favorites-empty-title = هنوز علاقه‌مندی‌ای نیست
# $shortcut is the key combination that opens the search dialog.
tokens-favorites-empty-description = با جست‌وجو ({ $shortcut }) توکن‌ها را پیدا کنید و به علاقه‌مندی‌ها اضافه کنید.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = توکن
tokens-column-status = وضعیت
tokens-ohlcv-delete =
    .title = حذف داده OHLCV
    .aria-label = حذف داده OHLCV
tokens-ohlcv-status-active = فعال
tokens-ohlcv-status-inactive = غیرفعال
tokens-ohlcv-priority-critical = بحرانی
tokens-ohlcv-priority-high = زیاد
tokens-ohlcv-priority-medium = متوسط
tokens-ohlcv-priority-low = کم
tokens-ohlcv-column-priority = اولویت
tokens-ohlcv-column-backfill = پرکردن داده‌های گذشته
tokens-ohlcv-column-data-span = بازه داده
tokens-ohlcv-column-gaps = شکاف‌ها
tokens-ohlcv-column-pools = استخرها
tokens-ohlcv-column-last-fetch = آخرین دریافت
# $timeframe is a timeframe code such as 1h.
tokens-ohlcv-timeframe-complete = { $timeframe }: کامل
tokens-ohlcv-timeframe-pending = { $timeframe }: در انتظار
tokens-ohlcv-load-failed-title = بارگیری داده OHLCV ممکن نشد
tokens-ohlcv-load-failed-toast = بارگیری داده OHLCV ممکن نشد
tokens-ohlcv-total = کل توکن‌ها
tokens-ohlcv-active = فعال
tokens-ohlcv-db-size = حجم پایگاه‌داده
tokens-ohlcv-cleanup = پاکسازی غیرفعال‌ها
tokens-ohlcv-delete-title = حذف داده OHLCV
# $mint is the first characters of the token mint.
tokens-ohlcv-delete-message = همه داده OHLCV مربوط به { $mint }... حذف شود؟
tokens-ohlcv-delete-done =
    حذف شد: { $candles ->
        [one] { $candles } کندل
       *[other] { $candles } کندل
    }، { $pools ->
        [one] { $pools } استخر
       *[other] { $pools } استخر
    }
tokens-ohlcv-delete-failed = حذف داده OHLCV ناموفق بود
tokens-ohlcv-cleanup-title = حذف توکن‌های غیرفعال
tokens-ohlcv-cleanup-message = توکن‌های غیرفعال قدیمی‌تر از ساعت مشخص‌شده حذف شوند
tokens-ohlcv-cleanup-placeholder = ساعت...
tokens-ohlcv-cleanup-invalid = لطفاً یک عدد مثبت وارد کنید
tokens-ohlcv-cleanup-done =
    پاکسازی شد: { $count ->
        [one] { $count } توکن غیرفعال
       *[other] { $count } توکن غیرفعال
    }
tokens-ohlcv-cleanup-failed = پاکسازی داده OHLCV ناموفق بود

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = مجموع
tokens-summary-priced = دارای قیمت
tokens-summary-positions = پوزیشن‌ها
tokens-summary-blacklisted = فهرست سیاه
tokens-search-placeholder = جست‌وجو با نماد یا مینت...
tokens-table-waiting-title = هنوز در حال بارگیری توکن‌ها...
tokens-table-waiting-description = در انتظار پاسخ بک‌اند. به‌طور خودکار دوباره تلاش می‌کنیم.
tokens-load-failed-toast = بارگیری توکن‌ها ممکن نشد
tokens-row-data-missing = داده توکن پیدا نشد
tokens-column-price-sol = قیمت ({ -sol })
tokens-column-liquidity = نقدینگی
tokens-column-volume-24h = حجم 24h
tokens-column-fdv = FDV
tokens-column-market-cap = ارزش بازار
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = تراکنش 5m
tokens-column-txns-1h = تراکنش 1h
tokens-column-txns-6h = تراکنش 6h
tokens-column-txns-24h = تراکنش 24h
tokens-column-risk-score = امتیاز ریسک
tokens-column-reject-reason = دلیل رد
tokens-column-blacklist-reason = دلیل فهرست سیاه
tokens-column-updated = به‌روزرسانی
tokens-column-birth = تولد
tokens-column-first-seen = اولین مشاهده
tokens-badge-price = قیمت
tokens-badge-ohlcv = OHLCV
tokens-badge-position = پوزیشن
tokens-badge-blacklisted = فهرست سیاه
tokens-badge-blacklisted-title = توکن فهرست سیاه
# $reasons is the list of blacklist categories, reasons and details.
tokens-badge-blacklisted-reasons = فهرست سیاه: { $reasons }
tokens-links-menu-copy-mint = کپی مینت
tokens-links-copy-failed = کپی مینت ناموفق بود
tokens-lightbox-token-age = عمر توکن

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = جست‌وجوی نام، نماد یا مینت...
tokens-search-input-label = جست‌وجوی توکن‌ها
tokens-search-results-label = نتایج جست‌وجو
tokens-search-hint = نام یا نماد توکن را بنویسید یا مینت را جای‌گذاری کنید
tokens-search-no-matches = موردی پیدا نشد — عبارت دیگری امتحان کنید
tokens-search-tip-nav = پیمایش
tokens-search-tip-open = باز کردن
tokens-search-tip-close = بستن
tokens-search-failed = جست‌وجو ناموفق بود
# $message is the failure text.
tokens-search-error = خطا: { $message }
tokens-search-action-favorite =
    .title = افزودن به علاقه‌مندی‌ها
    .aria-label = افزودن به علاقه‌مندی‌ها
tokens-search-action-blacklist =
    .title = افزودن به فهرست سیاه
    .aria-label = افزودن به فهرست سیاه
tokens-search-no-mint = این توکن آدرس مینت ندارد
tokens-search-open-failed = باز کردن جزئیات توکن ناموفق بود
tokens-search-copy-failed = کپی در کلیپ‌بورد ناموفق بود
# $symbol is the token symbol, or its mint when the symbol is unknown.
tokens-search-favorite-added = { $symbol } به علاقه‌مندی‌ها اضافه شد
tokens-search-favorite-already = از قبل در علاقه‌مندی‌هاست
tokens-search-favorite-failed = افزودن به علاقه‌مندی‌ها ناموفق بود
tokens-search-blacklist-message = { $symbol } به فهرست سیاه اضافه شود؟ این توکن از معاملات کنار گذاشته می‌شود.
tokens-search-blacklist-done = { $symbol } به فهرست سیاه اضافه شد
tokens-search-blacklisted = در فهرست سیاه
tokens-search-blacklist-failed = افزودن توکن به فهرست سیاه ناموفق بود

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = بوست‌شده
tokens-featured-category-jupiter-organic = برترین‌های ارگانیک { -jupiter }
tokens-featured-category-jupiter-traded = پرمعامله‌ترین‌های { -jupiter }
tokens-featured-category-dexscreener-trending = پرطرفدارهای { -dexscreener }
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = تبلیغ‌شده توسط تیم‌هایشان
tokens-featured-security-risky = پرریسک
tokens-featured-load-failed = بارگیری موارد ویژه ناموفق بود
# $message is the failure text.
tokens-featured-network-error = خطای شبکه: { $message }
tokens-featured-title = ویژه
tokens-featured-subtitle = ابتدا توکن‌های بوست‌شده، سپس پرطرفدارهای سولانا
tokens-featured-boost = بوست یک توکن
tokens-featured-close =
    .title = بستن (ESC)
tokens-featured-loading = در حال بارگیری موارد ویژه و پرطرفدار...
tokens-featured-error-hint = اتصال را بررسی کنید یا دوباره تلاش کنید
tokens-featured-empty = در حال حاضر توکنی موجود نیست
tokens-featured-count =
    { $count ->
        [one] { $count } توکن
       *[other] { $count } توکن
    }
tokens-featured-stat-market-cap = ارزش بازار
tokens-featured-stat-liquidity = نقدینگی
tokens-featured-stat-volume = حجم 24H
tokens-featured-stat-holders = هولدرها
tokens-featured-stat-txns = تراکنش 24H
# $symbol is the token symbol.
tokens-featured-buy = خرید
    .title = خرید { $symbol }
# $score is the normalized security score out of 100, where higher is safer.
tokens-featured-security-score = امتیاز امنیتی: { $score }/100
tokens-featured-social-website = وب‌سایت
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = همه
    .title = باز کردن نمای کامل موارد ویژه
tokens-featured-row-scroll-start =
    .aria-label = نمایش توکن‌های قبلی
tokens-featured-row-scroll-end =
    .aria-label = نمایش توکن‌های بیشتر
tokens-featured-row-empty = توکن ویژه‌ای نیست
# $name and $symbol identify the token; $boosts is the formatted active boost count.
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — { $boosts } بار بوست شده

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = انتخاب استخر
tokens-pool-selector-loading = در حال بارگیری استخرها...
tokens-pool-selector-empty = استخری برای این توکن پیدا نشد
# $message is the failure text.
tokens-pool-selector-load-failed = بارگیری استخرها ناموفق بود: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] { $count } استخر پیدا شد
       *[other] { $count } استخر پیدا شد
    }
# $amount is the formatted pool liquidity in USD.
tokens-pool-selector-liquidity = نقدینگی { $amount }
    .title = نقدینگی
# $amount is the formatted 24 hour pool volume in USD.
tokens-pool-selector-volume = { $amount } 24h
    .title = حجم 24h

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = دارایی ناشناس
tokens-identity-copy-address =
    .title = کپی آدرس
    .aria-label = کپی آدرس
tokens-identity-copy-signature =
    .title = کپی امضا
    .aria-label = کپی امضا
