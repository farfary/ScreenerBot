# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = کلید خصوصی کیف پول را وارد کنید.
setup-wallet-json-recognized = قالب کلید JSON با 64 بایت شناسایی شد.
setup-wallet-json-invalid = از آرایه JSON شامل دقیقاً 64 مقدار بایت (0–255) استفاده کنید.
setup-wallet-format-invalid = از کلید خصوصی base58 یا آرایه JSON با 64 بایت استفاده کنید.
setup-wallet-base58-recognized = قالب کلید Base58 شناسایی شد.

## RPC endpoint validation

setup-rpc-required = دست‌کم یک اندپوینت RPC وارد کنید.
setup-rpc-too-many = بیش از 10 اندپوینت RPC استفاده نکنید.
setup-rpc-url-invalid = هر اندپوینت باید یک نشانی HTTPS معتبر باشد.
setup-rpc-url-credentials = نشانی RPC نمی‌تواند نام کاربری یا رمز عبور داشته باشد.
setup-rpc-url-fragment = نشانی RPC نمی‌تواند بخش fragment داشته باشد.
setup-rpc-public-endpoint = RPC عمومی Solana از بررسی دوره‌ای پیوسته پشتیبانی نمی‌کند.
setup-rpc-private-host = اندپوینت‌های RPC نمی‌توانند از میزبان‌های محلی یا شبکه خصوصی استفاده کنند.
setup-rpc-duplicate = اندپوینت‌های RPC تکراری را حذف کنید.
setup-rpc-ready =
    { $count ->
        [one] { $count } اندپوینت HTTPS آماده آزمایش است.
       *[other] { $count } اندپوینت HTTPS آماده آزمایش است.
    }

## Verification results

setup-wallet-verified = کیف پول تأیید شد
setup-wallet-unverified = تأیید کیف پول ممکن نشد
setup-wallet-address-detail = آدرس { $address }
setup-wallet-format-hint = قالب کلید خصوصی را بررسی کنید.
setup-rpc-none-working = هیچ RPC فعالی برای mainnet نیست
setup-rpc-health-failed = هیچ اندپوینتی از بررسی‌های سلامت mainnet عبور نکرد.
setup-rpc-partial = { $working } فعال؛ { $failed } در دسترس نیست
setup-rpc-verified =
    { $count ->
        [one] { $count } اندپوینت mainnet تأیید شد
       *[other] { $count } اندپوینت mainnet تأیید شد
    }
setup-rpc-fastest = سریع‌ترین: { $url } ({ $latency } ms).
setup-error-request-failed = درخواست ناموفق بود ({ $status })
setup-error-restart-timeout = راه‌اندازی ذخیره شد اما { -brand } هنوز دوباره متصل نشده است.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = در حال خواندن کلید خصوصی
setup-verify-wallet-parsing-detail = بررسی کلید و استخراج آدرس عمومی آن.
setup-verify-wallet-waiting = در انتظار اعتبارسنجی
setup-verify-rpc-testing = در حال آزمایش mainnet Solana
setup-verify-rpc-testing-detail =
    { $count ->
        [one] در حال بررسی { $count } اندپوینت.
       *[other] در حال بررسی { $count } اندپوینت.
    }
setup-verify-rpc-waiting = در انتظار آزمایش اندپوینت‌ها
setup-verify-save-waiting = در انتظار ذخیره
setup-verify-save-running = در حال رمزگذاری و ذخیره
setup-verify-save-running-detail = در حال نوشتن پیکربندی تأییدشده روی این دستگاه.
setup-verify-save-done = پیکربندی ذخیره شد
setup-verify-save-done-detail = کلید خصوصی رمزگذاری و اندپوینت‌های RPC فعال ذخیره شدند.
setup-verify-save-failed = ذخیره راه‌اندازی ممکن نشد
setup-verify-save-skipped = ذخیره نشد
setup-verify-request-failed = درخواست تأیید ناموفق بود
setup-verify-summary-checking = در حال بررسی کیف پول و اتصال‌های mainnet Solana.
setup-verify-summary-running = در حال تأیید همان اطلاعاتی که وارد کرده‌اید.
setup-verify-summary-saving = اطلاعات تأیید شد. در حال ذخیره امن.
setup-verify-summary-failed = مشکل را بررسی کنید و دوباره تأیید کنید.

## Errors

setup-error-credentials-failed = تأیید اطلاعات ناموفق بود.
setup-error-save-failed = ذخیره راه‌اندازی ممکن نشد.
setup-error-verify-failed = تأیید ناموفق بود.
setup-error-explore-failed = راه‌اندازی حالت کاوش ممکن نشد.
setup-error-gateway-failed = ذخیره ترجیح درگاه ممکن نشد.
setup-action-review-credentials = بررسی اطلاعات

## Completion

setup-explore-opening = در حال باز کردن حالت کاوش…
setup-complete-restarting = در حال راه‌اندازی مجدد { -brand } با پیکربندی تأییدشده شما.
setup-complete-finishing = در حال تکمیل راه‌اندازی مجدد…
setup-complete-ready = { -brand } آماده است. در حال باز کردن داشبورد…
setup-complete-stored = پیکربندی تأییدشده شما به‌صورت امن روی این دستگاه ذخیره شده است.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = نمایش کلید خصوصی
setup-wallet-hide-key = پنهان کردن کلید خصوصی
setup-wallet-copy =
    .aria-label = کپی آدرس کیف پول
    .title = کپی آدرس کیف پول
setup-wallet-copy-done =
    .aria-label = آدرس کیف پول کپی شد
    .title = کپی شد
setup-wallet-copy-failed =
    .aria-label = کپی آدرس کیف پول ممکن نشد
    .title = کپی ناموفق بود

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = راه‌اندازی کیف پول و RPC
setup-dialog-subtitle = برای فعال شدن معاملات و داده‌های زنده روی زنجیره، کیف پول Solana و یک اندپوینت RPC ویژه را متصل کنید. کلید خصوصی شما روی همین دستگاه رمزگذاری می‌شود و هرگز از آن خارج نمی‌شود.
setup-dialog-close =
    .title = بستن
    .aria-label = بستن
setup-dialog-wallet-label = کلید خصوصی کیف پول
setup-dialog-wallet-input =
    .placeholder = رشته Base58 یا آرایه JSON مانند [1,2,3,...]
setup-dialog-rpc-label = اندپوینت(های) RPC
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (هر خط یکی)
setup-dialog-rpc-hint = استفاده از یک ارائه‌دهنده ویژه ({ -helius }، { -quicknode }، { -alchemy }) قویاً توصیه می‌شود — RPC عمومی Solana محدودیت نرخ دارد و ممکن است کار نکند.
setup-dialog-submit = اعتبارسنجی و اتصال
setup-dialog-working = در حال انجام…
setup-dialog-validating = در حال اعتبارسنجی…
setup-dialog-saving = در حال ذخیره…
setup-dialog-restarting = در حال راه‌اندازی مجدد…
setup-dialog-saved = راه‌اندازی ذخیره شد — در حال راه‌اندازی مجدد { -brand } در حالت کامل…
setup-dialog-error-missing-fields = هم کلید خصوصی کیف پول و هم دست‌کم یک نشانی RPC وارد کنید.
setup-dialog-error-validation = اعتبارسنجی ناموفق بود.
setup-dialog-error-incomplete = تکمیل راه‌اندازی ممکن نشد.
setup-dialog-error-restart-helper = ابزار راه‌اندازی مجدد خودکار در دسترس نیست. کمی بعد داشبورد را دوباره بارگذاری کنید.
setup-dialog-error-unexpected = خطای غیرمنتظره.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = پیشرفت راه‌اندازی
setup-wizard-step-credentials = اطلاعات
setup-wizard-step-verification = تأیید
setup-wizard-step-complete = تکمیل
setup-wizard-credentials-title = پیکربندی اطلاعات
setup-wizard-credentials-description = یک کیف پول محلی و اندپوینت‌های RPC قابل اعتماد برای mainnet Solana را متصل کنید.
setup-wizard-wallet-toggle =
    .title = نمایش کلید خصوصی
    .aria-label = نمایش کلید خصوصی
setup-wizard-wallet-security-note = پیش از ذخیره رمزگذاری می‌شود.
setup-wizard-rpc-title = اندپوینت‌های RPC
setup-wizard-rpc-input =
    .placeholder = هر خط یک نشانی HTTPS
setup-wizard-rpc-guidance = برای بررسی دوره‌ای پیوسته، RPC قابل اعتماد mainnet توصیه می‌شود.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = پیشنهادی
setup-wizard-gateway-title = ارسال رایگان تراکنش
setup-wizard-gateway-hint = پس از ورود به حساب در دسترس است. RPC شما به‌عنوان جایگزین در دسترس می‌ماند.
setup-wizard-account-title = حساب { -brand }
setup-wizard-account-optional = اختیاری
setup-wizard-account-loading = در حال بررسی وضعیت حساب…
setup-wizard-verify-title = تأیید و ذخیره
setup-wizard-verify-list =
    .aria-label = وضعیت تأیید راه‌اندازی
setup-wizard-verify-wallet = کیف پول
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = پیکربندی امن
setup-wizard-complete-title = راه‌اندازی ذخیره شد
setup-wizard-reconnect = تلاش دوباره اتصال
setup-wizard-reload = بارگذاری مجدد داشبورد
setup-wizard-error-title = راه‌اندازی نیاز به توجه دارد
setup-wizard-explore = کاوش داشبورد
setup-wizard-continue = ادامه
