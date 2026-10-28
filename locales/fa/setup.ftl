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
setup-dialog-rpc-optional-note = اختیاری: برای نگه‌داشتن اندپوینت‌های RPC تنظیم‌شده، فیلد را خالی بگذارید.
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
setup-error-restart-timeout = { -brand } هنوز دوباره متصل نشده است.

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
setup-verify-save-skipped = ذخیره نشد. مشکل بالا را برطرف کنید، سپس دوباره بررسی کنید.
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
setup-error-gateway-failed = ذخیره تنظیم ارسال تراکنش‌ها ممکن نشد.
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
    .placeholder = رشته Base58 یا آرایه JSON مانند ⁨[1,2,3,...]⁩
setup-dialog-submit = اعتبارسنجی و اتصال
setup-dialog-working = در حال انجام…
setup-dialog-validating = در حال اعتبارسنجی…
setup-dialog-saving = در حال ذخیره…
setup-dialog-restarting = در حال راه‌اندازی مجدد…
setup-dialog-saved = راه‌اندازی ذخیره شد — در حال راه‌اندازی مجدد { -brand } در حالت کامل…
setup-dialog-error-validation = اعتبارسنجی ناموفق بود.
setup-dialog-error-incomplete = تکمیل راه‌اندازی ممکن نشد.
setup-dialog-error-restart-helper = ابزار راه‌اندازی مجدد خودکار در دسترس نیست. کمی بعد داشبورد را دوباره بارگذاری کنید.
setup-dialog-error-unexpected = خطای غیرمنتظره.
setup-dialog-replace-title = جایگزینی کیف پول معاملاتی
setup-dialog-replace-subtitle = با کیف پول دیگری معامله کنید. کلید خصوصی آن روی همین دستگاه رمزگذاری می‌شود و هرگز از آن خارج نمی‌شود.
setup-dialog-replace-submit = اعتبارسنجی و جایگزینی
setup-dialog-replace-saved = جایگزینی آماده شد — در حال راه‌اندازی مجدد { -brand } با کیف پول جدید…
setup-dialog-replace-confirm-intro = جایگزینی کیف پول معاملاتی:
setup-dialog-replace-confirm-stop = معامله‌گر و کپی‌ترید را متوقف می‌کند. پس از راه‌اندازی مجدد دوباره روشنشان کنید.
setup-dialog-replace-confirm-backup = پوزیشن‌ها، تراکنش‌ها و تاریخچهٔ کیف پول فعلی را به یک پوشهٔ پشتیبان منتقل می‌کند.
setup-dialog-replace-confirm-keep = کلید کیف پول فعلی در کیف پول‌های دیگر می‌ماند و همچنان می‌توانید از آن خروجی بگیرید.
setup-dialog-replace-confirm-funds = هیچ دارایی جابه‌جا نمی‌شود: توکن‌ها و { -sol } در کیف پول فعلی می‌مانند.
setup-dialog-replace-confirm-rpc = اندپوینت‌های RPC تنظیم‌شده را با مواردی که وارد می‌کنید جایگزین می‌کند؛ اگر فیلد خالی بماند، اندپوینت‌های فعلی می‌مانند.
setup-dialog-replace-confirm-restart = { -brand } با کیف پول جدید دوباره راه‌اندازی می‌شود.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = پیشرفت راه‌اندازی
setup-wizard-wallet-security-note = پیش از ذخیره رمزگذاری می‌شود.
setup-wizard-rpc-title = اندپوینت‌های RPC
setup-wizard-rpc-input =
    .placeholder = هر خط یک نشانی HTTPS
setup-wizard-rpc-guidance = از یک ارائه‌دهندهٔ قابل اعتماد mainnet استفاده کنید؛ نقاط پایانی عمومی پشتیبانی نمی‌شوند.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = پیشنهادی
setup-wizard-account-title = حساب { -brand }
setup-wizard-account-optional = اختیاری
setup-wizard-account-loading = در حال بررسی وضعیت حساب…
setup-wizard-verify-title = تأیید و ذخیره
setup-wizard-verify-list =
    .aria-label = وضعیت تأیید راه‌اندازی
setup-wizard-verify-wallet = کیف پول
setup-wizard-verify-save = پیکربندی امن
setup-wizard-complete-title = راه‌اندازی ذخیره شد
setup-wizard-reconnect = تلاش دوباره اتصال
setup-wizard-reload = بارگذاری مجدد داشبورد
setup-wizard-error-title = راه‌اندازی نیاز به توجه دارد
setup-wizard-explore = کاوش داشبورد
setup-wizard-continue = ادامه

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = کیف پول
setup-wizard-step-connection = اتصال
setup-wizard-step-verify = بررسی
setup-wizard-step-ready = آماده
setup-wallet-title = کیف پول معاملاتی خود را راه‌اندازی کنید
setup-wallet-description = کیف پولی که { -brand } با آن معامله می‌کند. کلید آن روی همین رایانه رمزنگاری می‌شود و هرگز از آن خارج نمی‌شود.
setup-wallet-source =
    .aria-label = منبع کیف پول
setup-wallet-create-title = ساخت کیف پول جدید
setup-wallet-create-detail = { -brand } یک کیف پول جدید می‌سازد. شما کلید آن را ذخیره می‌کنید و سپس به آن { -sol } می‌فرستید.
setup-wallet-import-title = وارد کردن کیف پول
setup-wallet-import-detail = از کلید خصوصی‌ای که دارید استفاده کنید (base58 یا آرایهٔ JSON با 64 عدد).
setup-wallet-creating = در حال ساخت کیف پول…
setup-wallet-create-failed = ساخت کیف پول ممکن نشد. دوباره تلاش کنید.
setup-backup-title = کلید خصوصی خود را ذخیره کنید
setup-backup-description = هر کس این کلید را داشته باشد کیف پول را در اختیار دارد. { -brand } نمی‌تواند آن را بازیابی کند.
setup-backup-address = نشانی
setup-backup-key = کلید خصوصی
setup-backup-copy-address = کپی نشانی
setup-backup-download = دانلود
setup-backup-copied = کپی شد. پس از ذخیره، کلیپ‌بورد را پاک کنید.
setup-backup-machine-warning = نسخهٔ رمزنگاری‌شده‌ای که { -brand } نگه می‌دارد فقط روی همین رایانه باز می‌شود. اگر رایانه گم یا جایگزین شود، فقط کلیدی که اکنون ذخیره می‌کنید کیف پول را بازمی‌گرداند. آن را آفلاین نگه دارید، مثلاً روی کاغذ یا در یک مدیر گذرواژه.
setup-backup-ack-saved = کلید خصوصی را در جایی امن، بیرون از این رایانه، ذخیره کردم.
setup-backup-ack-control = می‌دانم که هر کس این کلید را داشته باشد می‌تواند دارایی این کیف پول را بردارد.
setup-backup-ack-recovery = می‌دانم که { -brand } نمی‌تواند این کلید را بازیابی کند. بدون آن، گم شدن رایانه یعنی از دست رفتن دارایی.
setup-backup-create-another = ساخت یکی دیگر
setup-backup-replace-confirm = این کلید کنار گذاشته شود و کیف پول جدیدی ساخته شود؟ فقط اگر هنوز چیزی به آن نفرستاده‌اید این کار را بکنید.
setup-connection-title = { -brand } چگونه به { -solana } وصل شود؟
setup-connection-description = بعداً می‌توانید این را در تنظیمات ← اتصال تغییر دهید.
setup-connection-choice =
    .aria-label = اتصال
setup-connection-service-title = سرویس { -brand }
setup-connection-service-badge = بدون نیاز به RPC
setup-connection-service-detail = از حساب رایگان { -brand } شما استفاده می‌کند.
setup-connection-service-prices = قیمت‌ها هر چند ثانیه یک بار به‌روز می‌شوند
setup-connection-service-features = کپی‌تریدینگ، پایش کیف پول‌ها، موجودی کیف پول‌های دیگر، تاریخچه قدیمی‌تر، سوآپ مستقیم در استخر و قیمت زندهٔ استخرها به RPC خودتان نیاز دارند
setup-connection-service-pause = اگر از حساب خارج شوید یا screenerbot.io در دسترس نباشد، معامله متوقف می‌شود
setup-connection-service-privacy = کلید خصوصی شما هرگز از این رایانه خارج نمی‌شود: سوآپ‌ها همین‌جا امضا می‌شوند و screenerbot.io فقط با آدرس عمومی کیف پول شما موجودی‌ها و تراکنش‌هایش را می‌خواند
setup-connection-own-title = RPC خودتان
setup-connection-own-badge = سریع‌ترین
setup-connection-own-detail = قیمت‌ها حدود دو بار در ثانیه و همهٔ قابلیت‌ها. به یک نقطهٔ اتصال RPC از یک ارائه‌دهنده نیاز دارد.
setup-connection-signin-required = برای استفاده از سرویس { -brand } وارد شوید یا یک حساب رایگان بسازید.
setup-connection-reauthorize = برای اجازه دادن به سرویس { -brand } برای خواندن کیف پول، دوباره وارد شوید.
setup-connection-sending = ارسال تراکنش‌ها از طریق { -brand }
setup-connection-sending-hint = هنگام ورود به حساب در دسترس است. وقتی ممکن نباشد، از RPC شما استفاده می‌شود.
setup-wizard-account-required = لازم
setup-wizard-verify-connection = اتصال
setup-verify-service-checking = در حال بررسی سرویس { -brand }
setup-verify-service-ready = سرویس { -brand } آماده است
setup-verify-service-busy = سرویس { -brand } اکنون شلوغ است. کمی بعد دوباره تلاش کنید یا از RPC خودتان استفاده کنید.
setup-verify-service-daily-limit = حساب شما به سقف امروز سرویس { -brand } رسیده است. پس از ساعت 00:00 UTC دوباره تلاش کنید یا از RPC خودتان استفاده کنید.
setup-verify-service-unreachable = screenerbot.io در دسترس نیست. اتصال اینترنت خود را بررسی کنید.
setup-verify-service-version = سرویس { -brand } دیگر از این نسخه پشتیبانی نمی‌کند. برای ادامه، { -brand } را به‌روزرسانی کنید.
setup-verify-service-signin = برای استفاده از سرویس { -brand } وارد شوید.
setup-verify-save-done-service-detail = کلید خصوصی رمزنگاری شد؛ سرویس { -brand } انتخاب شد.
setup-action-try-again = تلاش دوباره
setup-action-use-own-rpc = استفاده از RPC خودم
setup-action-sign-in-again = ورود دوباره
setup-ready-title = { -brand } آماده است
setup-fund-title = کیف پول معاملاتی را شارژ کنید
setup-fund-waiting = در انتظار { -sol }…
setup-fund-received = { $amount } { -sol } دریافت شد
setup-fund-balance-unavailable = در حال خواندن موجودی… خودش به‌روز می‌شود.
setup-fund-open-dashboard = باز کردن داشبورد
