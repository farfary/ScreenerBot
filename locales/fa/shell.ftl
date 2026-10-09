# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = باز کردن صفحه اصلی داشبورد
    .title = صفحه اصلی داشبورد
shell-bot-card =
    .aria-label = در حال بارگذاری وضعیت معامله‌گر خودکار
shell-bot-label = خودکار
shell-bot-status-loading = در حال بارگذاری
shell-bot-today = امروز
shell-explore-control =
    .aria-label = حالت کاوش. برای فعال شدن همه قابلیت‌ها، کیف پول و اندپوینت RPC را متصل کنید
    .title = برای فعال شدن معاملات، موجودی‌ها و داده‌های زنده روی زنجیره، کیف پول و اندپوینت RPC را متصل کنید
shell-explore-title = حالت کاوش
shell-explore-detail = کیف پول و RPC متصل نیست
shell-explore-action = تکمیل راه‌اندازی
shell-setup-gate-detail = حالت کاوش بدون کیف پول و RPC اجرا می‌شود. برای اتصال آن‌ها راه‌اندازی را کامل کنید.
shell-wallet-card =
    .aria-label = ارزش کیف پول؛ باز کردن پوزیشن‌ها
    .title = ارزش کیف پول ({ -sol } + توکن‌ها) · باز کردن پوزیشن‌ها
shell-wallet-worth-label = ارزش
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = توکن
shell-sol-price-card =
    .aria-label = قیمت { -sol } به دلار — باز کردن نمودار
    .title = قیمت { -sol } · برای نمودار کلیک کنید
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = کپی‌تریدینگ؛ باز کردن کپی‌تریدینگ
    .title = کپی‌تریدینگ · باز کردن کپی‌تریدینگ
shell-copy-label = کپی
shell-actions-more =
    .aria-label = اقدام‌های بیشتر سربرگ
    .title = اقدام‌های بیشتر
shell-actions-group =
    .aria-label = اقدام‌های سربرگ
shell-action-search =
    .aria-label = جستجوی توکن‌ها
    .title = جستجوی توکن‌ها (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = توکن‌های ویژه
    .title = توکن‌های ویژه
shell-action-notifications =
    .aria-label = اقدام‌ها و اعلان‌ها
    .title = اقدام‌ها و اعلان‌ها
shell-action-restart =
    .aria-label = راه‌اندازی مجدد برنامه
    .title = راه‌اندازی مجدد برنامه
shell-action-theme =
    .aria-label = تغییر پوسته
    .title = تغییر پوسته
shell-action-settings =
    .aria-label = تنظیمات
    .title = تنظیمات
shell-tabs-scroll-start =
    .aria-label = نمایش تب‌های قبلی
    .title = نمایش تب‌های قبلی
shell-tabs-scroll-end =
    .aria-label = نمایش تب‌های بیشتر
    .title = نمایش تب‌های بیشتر
shell-nav-more = بیشتر
shell-ticker-scroll-start =
    .aria-label = نمایش معیارهای قبلی
    .title = نمایش معیارهای قبلی
shell-ticker-scroll-end =
    .aria-label = نمایش معیارهای بیشتر
    .title = نمایش معیارهای بیشتر

## Ticker

shell-ticker-monitoring-segment =
    .title = توکن‌هایی که سرویس استخر پایش می‌کند
shell-ticker-monitoring = در حال پایش:
shell-ticker-filtering-segment =
    .title = توکن‌هایی که معیارهای فیلترینگ را گذراندند یا رد شدند
shell-ticker-passed = تأییدشده:
shell-ticker-rejected = ردشده:
shell-ticker-pnl-segment =
    .title = سود و زیان تحقق‌یافته امروز
shell-ticker-pnl = سود و زیان امروز:
shell-ticker-rpc-segment =
    .title = تعداد فراخوانی‌های RPC در دقیقه و نرخ موفقیت
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/دقیقه
shell-ticker-services-segment =
    .title = وضعیت سلامت سرویس‌های پس‌زمینه
shell-ticker-services-loading = سرویس‌ها: <strong>در حال بارگذاری</strong>

## Notification drawer

shell-notification-title = اقدام‌ها
shell-notification-mark-all-read =
    .title = علامت‌گذاری همه به‌عنوان خوانده‌شده
shell-notification-clear-all =
    .title = پاک کردن همه
shell-notification-close =
    .aria-label = بستن
shell-notification-tab-all = همه
shell-notification-tab-active = فعال
shell-notification-tab-done = انجام‌شده
shell-notification-tab-failed = ناموفق
shell-notification-filter-type-all = همه انواع
shell-notification-filter-type-buy = خرید
shell-notification-filter-type-sell = فروش
shell-notification-filter-type-open = باز کردن
shell-notification-filter-type-close = بستن
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = جزئی
shell-notification-filter-state-all = همه وضعیت‌ها
shell-notification-filter-state-in-progress = در حال انجام
shell-notification-filter-state-completed = تکمیل‌شده
shell-notification-filter-state-failed = ناموفق
shell-notification-filter-state-cancelled = لغوشده
shell-notification-list =
    .aria-label = اعلان‌ها
shell-notification-empty = هنوز اقدامی ثبت نشده است
shell-notification-loading-more = در حال بارگذاری موارد بیشتر...
shell-notification-back-to-top =
    .title = بازگشت به بالا

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = فعالیت
shell-status-bar-memory = حافظه
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/دقیقه
shell-status-bar-trading = معاملات
shell-status-bar-positions = پوزیشن
shell-status-bar-tokens = توکن‌ها

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = در حال راه‌اندازی { -brand }
shell-splash-waiting = در انتظار پاسخ هسته محلی.
shell-splash-failed = راه‌اندازی { -brand } ممکن نشد
shell-splash-failed-detail = فایل لاگ را بررسی کنید و سپس برنامه را دوباره راه‌اندازی کنید.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = هسته متصل است
shell-connection-waiting = در انتظار هسته…
shell-connection-retry-now = تلاش دوباره
shell-connection-overlay-detail = هسته در دسترس نیست. معاملات متوقف شده است؛ با برقراری اتصال به‌طور خودکار ادامه می‌یابد.
shell-connection-restored = اتصال به هسته برقرار شد

# Source: scripts/core/header.js
shell-trader-control-failed = کنترل معامله‌گر ناموفق بود
shell-notification-button-unread = اقدام‌ها و اعلان‌ها، { $count } خوانده‌نشده
shell-restart-confirm-title = راه‌اندازی مجدد ربات
shell-restart-confirm-message =
    آیا از راه‌اندازی مجدد ربات مطمئن هستید؟

    این کار:
    • همه سرویس‌ها را متوقف می‌کند
    • فرایند را دوباره اجرا می‌کند
    • حدود 10 تا 15 ثانیه طول می‌کشد

    همه عملیات فعال قطع خواهد شد.
shell-restart-confirm-action = راه‌اندازی مجدد
shell-restart-progress = در حال راه‌اندازی مجدد ربات
shell-restart-failed = راه‌اندازی مجدد ناموفق بود
shell-restart-failed-status = راه‌اندازی مجدد ناموفق بود: { $status }
shell-restart-helper-unavailable = ابزار راه‌اندازی مجدد خودکار در دسترس نیست. کمی بعد داشبورد را دوباره بارگذاری کنید.

# Source: scripts/core/router.js
shell-page-title-fallback = داشبورد
shell-page-load-failed = بارگذاری صفحه ناموفق بود
shell-page-offline-detail = هسته در حال حاضر در دسترس نیست. با برقراری اتصال، این صفحه به‌طور خودکار بارگذاری می‌شود.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = کاوش
shell-bot-state-halted = متوقف
shell-bot-state-off = خاموش
shell-bot-state-waiting = در انتظار
shell-bot-state-idle = بیکار
shell-bot-state-entry-paused = ورود متوقف
shell-bot-state-running = در حال اجرا
shell-bot-control-explore = معامله‌گر خودکار در حالت کاوش در دسترس نیست. راه‌اندازی کیف پول و RPC را باز کنید.
shell-bot-control-halted = توقف اضطراری فعال است. کنترل‌های معامله‌گر خودکار را باز کنید.
shell-bot-control-off = معامله‌گر خودکار خاموش است. برای فعال‌سازی کلیک کنید.
shell-bot-control-waiting = معامله‌گر خودکار فعال است و منتظر سرویس‌های هسته است. برای غیرفعال‌سازی کلیک کنید.
shell-bot-control-idle = معامله‌گر خودکار فعال است اما هر دو پایش خاموش‌اند. کنترل‌های معامله‌گر خودکار را باز کنید.
shell-bot-control-entry-paused = محافظت در برابر ضرر ورودها را متوقف کرده است؛ خروج‌ها می‌توانند ادامه یابند. کنترل‌های معامله‌گر خودکار را باز کنید.
shell-bot-control-running = معامله‌گر خودکار در حال اجراست. برای غیرفعال‌سازی کلیک کنید.

## Wallet and copy cards

shell-wallet-card-summary = ارزش کیف پول: { $equity } { -sol } ({ $balance } { -sol } نقد، { $tokens } توکن)؛ باز کردن پوزیشن‌ها
shell-copy-running-live = { $count } زنده
shell-copy-running-paper = { $count } آزمایشی
shell-copy-value-paused = متوقف
shell-copy-value-idle = بیکار
shell-copy-sub-active = { $active } از { $total } فعال

## Ticker services state

shell-ticker-services-healthy = سرویس‌ها: <strong>سالم</strong>
shell-ticker-services-issues =
    { $count ->
        [one] سرویس‌ها: <strong>{ $count } مشکل</strong>
       *[other] سرویس‌ها: <strong>{ $count } مشکل</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = درخواست عامل
shell-agent-request-client-fallback = یک عامل جفت‌شده
shell-agent-request-message = { $client } می‌خواهد «{ $tool }» را در { -brand } اجرا کند. این درخواست { $expiry }.
shell-agent-request-message-arguments = { $client } می‌خواهد «{ $tool }» را در { -brand } اجرا کند. آرگومان‌ها: { $summary }. این درخواست { $expiry }.
shell-agent-request-expires-minutes = { $minutes } دقیقه دیگر منقضی می‌شود
shell-agent-request-expires-seconds = { $seconds } ثانیه دیگر منقضی می‌شود
shell-agent-request-approve = تأیید
shell-agent-request-deny = رد

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } کپی شد
shell-toast-copy-failed = کپی ناموفق بود
shell-toast-still-running = هنوز در حال اجراست — مرکز اعلان‌ها را بررسی کنید
shell-toast-dismiss =
    .aria-label = بستن
shell-confirm-title = تأیید عملیات
shell-confirm-message = آیا مطمئن هستید؟

# Source: scripts/core/global_chat.js
shell-assistant-label = دستیار
shell-assistant-dialog =
    .aria-label = دستیار

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = فعال
shell-status-bar-trading-inactive = غیرفعال

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } لغو شد
shell-action-swap-buy-live = در حال خرید
shell-action-swap-buy-done = خریداری شد
shell-action-swap-buy-failed = خرید ناموفق بود
shell-action-swap-sell-live = در حال فروش
shell-action-swap-sell-done = فروخته شد
shell-action-swap-sell-failed = فروش ناموفق بود
shell-action-position-open-live = در حال باز کردن پوزیشن
shell-action-position-open-done = باز شد
shell-action-position-open-failed = باز کردن ناموفق بود
shell-action-position-close-live = در حال بستن پوزیشن
shell-action-position-close-done = بسته شد
shell-action-position-close-failed = بستن ناموفق بود
shell-action-position-dca-live = در حال افزودن به پوزیشن
shell-action-position-dca-done = افزوده شد به
shell-action-position-dca-failed = افزودن ناموفق بود
shell-action-partial-exit-live = خروج جزئی
shell-action-partial-exit-done = خروج جزئی
shell-action-partial-exit-failed = خروج جزئی ناموفق بود
shell-action-manual-order-live = در حال ثبت سفارش
shell-action-manual-order-done = سفارش ثبت شد
shell-action-manual-order-failed = ثبت سفارش ناموفق بود
shell-action-trade-live = معامله
shell-action-trade-done = معامله انجام شد
shell-action-trade-failed = معامله ناموفق بود
shell-action-via-router = { $action } از طریق { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = با دور زدن { $venue }
shell-action-cost-guard-avoiding-cost = با دور زدن { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = با دور زدن یک پلتفرم
shell-action-cost-guard-avoiding-unnamed-cost = با دور زدن یک پلتفرم · { $cost }
shell-action-cost-guard-avoided = { $outcome } · از پرداخت اجاره { $cost } در { $venue } جلوگیری شد
shell-action-cost-guard-avoided-unnamed = { $outcome } · از پرداخت اجاره پلتفرم به مبلغ { $cost } جلوگیری شد
shell-action-exit-full = خروج کامل
shell-action-exit-percent = خروج { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = { -brand } بسته شود؟
shell-exit-description = نحوه بستن برنامه را انتخاب کنید
shell-exit-minimize = کوچک کردن به سینی سیستم
shell-exit-minimize-detail = ادامه اجرا در پس‌زمینه
shell-exit-quit = خروج از برنامه
shell-exit-quit-detail = بستن کامل و توقف همه سرویس‌ها

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = ذخیره تصویر
shell-lightbox-close =
    .title = بستن (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = روشن
shell-theme-dark = تیره
shell-theme-switch-to-light = تغییر به پوسته روشن
shell-theme-switch-to-dark = تغییر به پوسته تیره
