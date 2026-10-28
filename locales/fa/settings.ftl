## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } دقیقه
       *[other] { $count } دقیقه
    }
settings-duration-hours =
    { $count ->
        [one] { $count } ساعت
       *[other] { $count } ساعت
    }

## settings_dialog.js

settings-dialog-title = تنظیمات
settings-dialog-close =
    .title = بستن (ESC)
    .aria-label = بستن تنظیمات
settings-dialog-save = ذخیره تغییرات
settings-dialog-saving = در حال ذخیره...
settings-dialog-saved = ذخیره شد
settings-dialog-save-success = تنظیمات با موفقیت ذخیره شد
settings-dialog-save-failed = ذخیره تنظیمات ناموفق بود
settings-dialog-unsaved-title = تغییرات ذخیره‌نشده
settings-dialog-unsaved-message = تغییرات تنظیمات هنوز ذخیره نشده‌اند.
settings-dialog-unsaved-discard = کنار گذاشتن
settings-dialog-unsaved-keep = ادامه ویرایش
settings-dialog-update-attention = به‌روزرسانی نیاز به توجه دارد
settings-dialog-tab-interface = رابط کاربری
settings-dialog-tab-navigation = پیمایش
settings-dialog-tab-startup = راه‌اندازی
settings-dialog-tab-hints = راهنماها
settings-dialog-tab-data = داده
settings-dialog-tab-security = امنیت
settings-dialog-tab-wallet = کیف پول
settings-dialog-tab-account = حساب کاربری
settings-dialog-tab-connection = اتصال
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = اتصال‌های عامل
settings-dialog-tab-updates = به‌روزرسانی‌ها
settings-dialog-tab-licenses = مجوزها
settings-dialog-tab-about = درباره
settings-dialog-link-privacy = سیاست حریم خصوصی
settings-dialog-link-terms = شرایط استفاده

## settings_dialog.js: Startup tab

settings-startup-section-title = رفتار هنگام راه‌اندازی
settings-startup-auto-start-label = شروع خودکار معامله‌گر
settings-startup-auto-start-hint = معامله‌گر خودکار هنگام اجرای برنامه به‌طور خودکار شروع شود
settings-startup-coming-soon = به‌زودی
settings-startup-default-page-label = صفحه پیش‌فرض
settings-startup-default-page-hint = صفحه‌ای که هنگام باز کردن برنامه نمایش داده می‌شود
settings-startup-notifications-label = نمایش اعلان‌های پس‌زمینه
settings-startup-notifications-hint = نمایش اعلان برای رویدادهای پس‌زمینه

## settings_dialog.js: About tab

settings-about-tagline = موتور معاملاتی بومی Solana
settings-about-link-github = { -github }
settings-about-link-docs = مستندات
settings-about-link-telegram = { -telegram }
settings-about-link-website = وب‌سایت
settings-about-credits = ساخته‌شده برای معامله‌گران Solana
settings-about-copyright = © { $year } { -brand }. همه حقوق محفوظ است.

## interface_tab.js

settings-interface-section-appearance = ظاهر
settings-interface-theme-label = پوسته
settings-interface-theme-hint = طرح رنگی دلخواه خود را انتخاب کنید
settings-interface-theme-dark = تیره
settings-interface-theme-light = روشن
settings-interface-language-label = زبان
settings-interface-language-hint = زبان نمایش داشبورد
settings-interface-logo-shape-label = شکل لوگوی توکن
settings-interface-logo-shape-hint = «دایره» همه لوگوها را برش می‌دهد؛ «طبیعی» شکل اصلی هر طرح را حفظ می‌کند
settings-interface-logo-shape-circle = دایره
settings-interface-logo-shape-natural = طبیعی
settings-interface-animations-label = فعال‌سازی انیمیشن‌ها
settings-interface-animations-hint = گذارها و جلوه‌های روان
settings-interface-compact-label = حالت فشرده
settings-interface-compact-hint = کاهش فاصله‌ها برای نمایش محتوای بیشتر
settings-interface-section-data = داده و نمایش
settings-interface-refresh-label = فاصله تازه‌سازی
settings-interface-refresh-hint = هر چند وقت یک‌بار داده‌ها تازه شوند
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } ثانیه
       *[other] { $count } ثانیه
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } دقیقه
       *[other] { $count } دقیقه
    }
settings-interface-ticker-label = نمایش نوار تیکر
settings-interface-ticker-hint = تیکر شاخص‌های زنده در سربرگ
settings-interface-page-size-label = اندازه صفحه جدول
settings-interface-page-size-hint = تعداد پیش‌فرض ردیف در هر صفحه جدول
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } ردیف
       *[other] { $count } ردیف
    }
settings-interface-auto-expand-label = باز شدن خودکار دسته‌ها
settings-interface-auto-expand-hint = دسته‌های پیکربندی به‌طور پیش‌فرض باز باشند
settings-interface-hints-label = نمایش راهنماهای زمینه‌ای
settings-interface-hints-hint = نمایش آیکون‌های راهنما برای توضیح قابلیت‌های داشبورد
settings-interface-featured-label = نمایش ردیف ویژه
settings-interface-featured-hint = نمایش ردیف توکن‌های ویژه در صفحه‌های خانه و توکن‌ها
settings-interface-section-sound = جلوه‌های صوتی
settings-interface-sounds-label = فعال‌سازی صداها
settings-interface-sounds-hint = نشانه‌های صوتی برای پیمایش، تغییر وضعیت و نتایج

## security_tab.js

settings-security-loading = در حال بارگذاری تنظیمات امنیتی...
settings-security-load-failed = بارگذاری تنظیمات امنیتی ناموفق بود

settings-security-type-pin4 = پین 4 رقمی
settings-security-type-pin6 = پین 6 رقمی
settings-security-type-text = گذرواژه متنی
settings-security-type-unset = تنظیم نشده

settings-security-lockscreen-title = قفل صفحه داشبورد
settings-security-lockscreen-description = از داشبورد خود با پین یا گذرواژه محافظت کنید. قفل صفحه در زمان لازم ظاهر می‌شود و برای ادامه به احراز هویت نیاز دارد.
settings-security-enable-label = فعال‌سازی قفل صفحه
settings-security-enable-hint = محافظت از داشبورد با احراز هویت گذرواژه
settings-security-password-status-label = وضعیت گذرواژه
settings-security-password-current = فعلی: { $type }
settings-security-password-none = گذرواژه‌ای تنظیم نشده است
settings-security-change = تغییر
settings-security-remove = حذف
settings-security-set-password = تنظیم گذرواژه
settings-security-auto-lock-label = قفل خودکار پس از بی‌فعالیتی
settings-security-auto-lock-hint = قفل خودکار پس از مدتی بی‌فعالیتی
settings-security-auto-lock-never = هرگز
settings-security-lock-blur-label = قفل هنگام خروج پنجره از تمرکز
settings-security-lock-blur-hint = هنگام رفتن به برنامه دیگر، قفل خودکار فعال شود
settings-security-quick-actions-title = اقدامات سریع
settings-security-lock-now-label = قفل فوری داشبورد
settings-security-lock-now-hint = داشبورد بلافاصله قفل شود
settings-security-needs-password = برای استفاده از این، ابتدا گذرواژه تنظیم کنید
settings-security-needs-lockscreen = برای استفاده از این، ابتدا قفل صفحه را فعال کنید
settings-security-lock-now = قفل کردن
settings-security-lock-not-ready = قفل‌کردن ممکن نیست - قفل صفحه آماده نیست
settings-security-setting-save-failed = ذخیره تنظیم امنیتی ممکن نشد

## security_tab.js: two-factor authentication

settings-security-2fa-title = احراز هویت دومرحله‌ای
settings-security-2fa-description = با یک برنامه احراز هویت (Google Authenticator، Authy و غیره) لایه امنیتی بیشتری اضافه کنید
settings-security-2fa-status-label = وضعیت 2FA
settings-security-2fa-status-enabled = احراز هویت دومرحله‌ای فعال است
settings-security-2fa-status-none = پیکربندی نشده
settings-security-2fa-disable = غیرفعال‌سازی 2FA
settings-security-2fa-enable = فعال‌سازی 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = بستن
settings-security-password-set-title = تنظیم گذرواژه
settings-security-password-change-title = تغییر گذرواژه
settings-security-password-current-label = گذرواژه فعلی
settings-security-password-current-input =
    .placeholder = گذرواژه فعلی را وارد کنید
settings-security-password-type-label = نوع گذرواژه
settings-security-password-new-label = گذرواژه جدید
settings-security-password-new-input =
    .placeholder = گذرواژه جدید را وارد کنید
settings-security-password-confirm-label = تأیید گذرواژه
settings-security-password-confirm-input =
    .placeholder = گذرواژه را تأیید کنید
settings-security-password-update = به‌روزرسانی گذرواژه
settings-security-placeholder-pin4 = پین 4 رقمی را وارد کنید
settings-security-placeholder-pin6 = پین 6 رقمی را وارد کنید
settings-security-placeholder-text = گذرواژه را وارد کنید
settings-security-password-required = لطفاً گذرواژه را وارد کنید
settings-security-password-mismatch = گذرواژه‌ها مطابقت ندارند
settings-security-pin4-invalid = پین باید دقیقاً 4 رقم باشد
settings-security-pin6-invalid = پین باید دقیقاً 6 رقم باشد
settings-security-text-too-short = گذرواژه باید دست‌کم 4 نویسه باشد
settings-security-password-saved = گذرواژه ذخیره شد
settings-security-password-save-failed = ذخیره گذرواژه ناموفق بود
settings-security-password-save-failed-detail = ذخیره گذرواژه ناموفق بود: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = حذف گذرواژه
settings-security-remove-description = برای حذف محافظت قفل صفحه، گذرواژه فعلی خود را وارد کنید.
settings-security-remove-confirm = حذف گذرواژه
settings-security-current-required = لطفاً گذرواژه فعلی خود را وارد کنید
settings-security-password-removed = گذرواژه حذف شد
settings-security-password-remove-failed = حذف گذرواژه ناموفق بود
settings-security-password-remove-failed-detail = حذف گذرواژه ناموفق بود: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = فعال‌سازی احراز هویت دومرحله‌ای
settings-security-2fa-password-prompt = برای ادامه گذرواژه خود را وارد کنید:
settings-security-2fa-password-input =
    .placeholder = گذرواژه را وارد کنید
settings-security-2fa-continue = ادامه
settings-security-2fa-manual-code = کد ورود دستی:
settings-security-2fa-qr =
    .alt = کد QR برای TOTP
settings-security-2fa-code-prompt = کد 6 رقمی برنامه احراز هویت خود را وارد کنید:
settings-security-2fa-verify-enable = تأیید و فعال‌سازی
settings-security-2fa-password-required = لطفاً گذرواژه خود را وارد کنید
settings-security-2fa-setup-failed = راه‌اندازی 2FA ناموفق بود
settings-security-2fa-code-invalid-length = لطفاً یک کد 6 رقمی وارد کنید
settings-security-2fa-code-invalid = کد نامعتبر است
settings-security-2fa-enabled = احراز هویت دومرحله‌ای فعال شد
settings-security-2fa-verify-failed = تأیید کد ناموفق بود
settings-security-2fa-disable-title = غیرفعال‌سازی احراز هویت دومرحله‌ای
settings-security-2fa-disable-prompt = برای غیرفعال‌سازی 2FA گذرواژه خود را وارد کنید:
settings-security-2fa-disable-failed = غیرفعال‌سازی 2FA ناموفق بود
settings-security-2fa-disabled = احراز هویت دومرحله‌ای غیرفعال شد

## agent_connections_tab.js

settings-agent-category-analysis = تحلیل
settings-agent-category-portfolio = پورتفوی
settings-agent-category-trading = معاملات
settings-agent-category-config = پیکربندی
settings-agent-category-system = سیستم
settings-agent-category-analysis-description = تحلیل توکن، داده‌های بازار و بررسی‌های امنیتی.
settings-agent-category-portfolio-description = پوزیشن‌های باز، موجودی‌ها و سود و زیان.
settings-agent-category-trading-description = خرید، فروش و بستن پوزیشن‌ها با پول واقعی.
settings-agent-category-config-description = همه تنظیمات ربات، از جمله اندپوینت‌های RPC. هرگز کلیدهای کیف پول.
settings-agent-category-system-description = وضعیت، رویدادها و توقف اضطراری.
settings-agent-category-analysis-inline = تحلیل
settings-agent-category-portfolio-inline = پورتفوی
settings-agent-category-trading-inline = معاملات
settings-agent-category-config-inline = پیکربندی
settings-agent-category-system-inline = سیستم

settings-agent-level-allow = مجاز
settings-agent-level-ask-user = پرسش
settings-agent-level-deny = خاموش
settings-agent-level-allow-hint = بلافاصله اجرا می‌شود.
settings-agent-level-ask-user-hint = منتظر تأیید شما در برنامه می‌ماند.
settings-agent-level-deny-hint = رد می‌شود و از دید عامل پنهان است.

settings-agent-preset-full = دسترسی کامل
settings-agent-preset-ask = ابتدا پرسش
settings-agent-preset-read = فقط خواندن
settings-agent-preset-full-description = همه‌چیز بدون پرسش اجرا می‌شود. کلیدهای کیف پول همچنان غیرقابل دسترسی می‌مانند.
settings-agent-preset-ask-description = هر اقدام منتظر تأیید شما در برنامه می‌ماند.
settings-agent-preset-read-description = فقط خواندن تحلیل و پورتفوی. هیچ چیزی قابل تغییر نیست.
settings-agent-preset-custom = سفارشی
settings-agent-preset-group =
    .aria-label = پیش‌تنظیم دسترسی
settings-agent-permission-group = دسترسی { $category }

settings-agent-summary-asks-only = محدود — پرسش برای { $asking }
settings-agent-summary-off-only = محدود — بدون { $off }
settings-agent-summary-asks-and-off = محدود — پرسش برای { $asking }؛ بدون { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP عمومی با stdio

settings-agent-note-placeholder = مسیر /absolute/path/to/screenerbot را با مسیر مطلق فایل اجرایی { -brand } خود جایگزین کنید — برنامه در حال اجرا نتوانست مسیر فایل اجرایی خود را در این سیستم مشخص کند.
settings-agent-note-data-dir = اگر { -brand } را با پوشه داده غیرپیش‌فرض اجرا می‌کنید، SCREENERBOT_DATA_DIR را نیز در کلاینت روی همان مسیر تنظیم کنید (یک پرچم -e / --env دیگر، یا یک ورودی env).
settings-agent-note-codex-run = دستور را اجرا کنید، یا بلوک TOML را به ~/.codex/config.toml ($CODEX_HOME/config.toml) اضافه کنید. سپس { -codex } را دوباره راه‌اندازی کنید.
settings-agent-note-codex-get = `codex mcp get screenerbot` رمز را در خروجی خود پنهان می‌کند.
settings-agent-note-claude-code = { -claude } Code: دستور را اجرا کنید و سپس { -claude } Code را دوباره راه‌اندازی کنید. `claude mcp get screenerbot` محیط پیکربندی‌شده را همراه با رمز نمایش می‌دهد.
settings-agent-note-claude-desktop = { -claude } Desktop: JSON را در claude_desktop_config.json زیر `mcpServers` ادغام کنید و برنامه را دوباره راه‌اندازی کنید.
settings-agent-note-openclaw = دستور را اجرا کنید، سپس با `openclaw mcp doctor screenerbot --probe` بررسی کنید که سرور stdio ذخیره‌شده راه‌اندازی می‌شود و ابزارها را ارائه می‌دهد.
settings-agent-note-hermes = این را در فایل پیکربندی { -hermes } زیر `mcp_servers` اضافه کنید و سپس { -hermes } را دوباره راه‌اندازی کنید.
settings-agent-note-generic = هر کلاینت MCP که stdio را پشتیبانی کند: این دستور را با همین آرگومان‌ها و محیط اجرا کنید، در هر جایی که کلاینت فهرست سرورهایش را نگه می‌دارد.
settings-agent-block-codex-command = { -codex } CLI — دستور ترمینال
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (جایگزین)
settings-agent-block-claude-command = { -claude } Code — دستور ترمینال
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — دستور ترمینال
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = کلاینت MCP عمومی با stdio

settings-agent-name-required = برای این اتصال یک نام وارد کنید.
settings-agent-name-too-long = نام باید حداکثر { $max } نویسه باشد.
settings-agent-name-control-characters = نام نباید شامل نویسه‌های کنترلی باشد.

settings-agent-title = اتصال‌های عامل
settings-agent-description = { -claude }، { -codex }، { -hermes }، { -openclaw } یا هر کلاینت MCP با stdio را متصل کنید. { -brand } باید در حال اجرا بماند. هر اتصال دسترسی‌های مخصوص خود را دارد: به‌طور پیش‌فرض دسترسی کامل، و هر زمان که بخواهید قابل محدودسازی برای هر اتصال. هیچ اتصالی هرگز نمی‌تواند کلید کیف پول شما را بخواند یا تغییر دهد.
settings-agent-name-label = نام اتصال
settings-agent-name-hint = در فهرست زیر نمایش داده می‌شود تا اتصال‌ها را از هم تشخیص دهید.
settings-agent-name-input =
    .placeholder = عامل کدنویسی لپ‌تاپ
settings-agent-client-label = کلاینت
settings-agent-client-hint = راهنمای راه‌اندازی نمایش‌داده‌شده پس از ساخت اتصال را انتخاب می‌کند.
settings-agent-permissions-label = دسترسی‌ها
settings-agent-permissions-hint = اتصال جدید می‌تواند همه کار را انجام دهد. هر دسته را همین حالا یا بعداً از فهرست زیر محدود کنید — در هر صورت کلیدهای کیف پول هرگز در دسترس نیستند.
settings-agent-create = ایجاد اتصال
settings-agent-issued-group =
    .aria-label = اعتبارنامه اتصال جدید
settings-agent-issued-warning = رمز را همین حالا کپی کنید. فقط یک‌بار نمایش داده می‌شود و دوباره قابل بازیابی نیست — اگر آن را گم کردید، اتصال را لغو و دوباره ایجاد کنید. { -brand } فقط یک تأییدکننده یک‌طرفه نگه می‌دارد؛ کلاینت MCP شما متن اصلی را در پیکربندی خودش ذخیره می‌کند.
settings-agent-issued-client-id = شناسه کلاینت
settings-agent-issued-secret = رمز یک‌بار مصرف
settings-agent-setup-for = راه‌اندازی برای
settings-agent-done = انجام شد
settings-agent-list-title = اتصال‌ها
settings-agent-loading = در حال بارگذاری اتصال‌ها...
settings-agent-active-count = { $count } فعال
settings-agent-empty = هنوز اتصالی وجود ندارد. برای جفت‌کردن یک کلاینت، در بالا یکی ایجاد کنید.
settings-agent-empty-active = اتصال فعالی وجود ندارد.
settings-agent-revoked-title = اتصال‌های لغوشده
settings-agent-created = ایجاد: { $time }
settings-agent-last-used = آخرین استفاده: { $time }
settings-agent-never-used = هرگز استفاده نشده
settings-agent-permissions-edit = دسترسی‌ها
settings-agent-revoke = لغو
settings-agent-permissions-save = ذخیره دسترسی‌ها

settings-agent-load-failed = بارگذاری اتصال‌های عامل ناموفق بود
settings-agent-list-failed = بارگذاری اتصال‌ها ممکن نشد
settings-agent-create-failed = ایجاد اتصال ممکن نشد.
settings-agent-unreachable-create = برای ایجاد اتصال، دسترسی به { -brand } ممکن نشد.
settings-agent-permissions-update-failed = به‌روزرسانی دسترسی‌ها ممکن نشد
settings-agent-permissions-updated = دسترسی‌ها به‌روزرسانی شد
settings-agent-permissions-updated-detail = از درخواست بعدی اتصال اعمال می‌شود.
settings-agent-unreachable-save = برای ذخیره، دسترسی به { -brand } ممکن نشد
settings-agent-revoke-title = لغو اتصال
settings-agent-revoke-message = «{ $label }» لغو شود؟ کلاینت از درخواست بعدی خود از کار می‌افتد و قابل بازگردانی نیست.
settings-agent-revoke-fallback-name = این اتصال
settings-agent-revoke-failed = لغو اتصال ممکن نشد
settings-agent-unreachable-revoke = برای لغو، دسترسی به { -brand } ممکن نشد

## telegram_tab.js

settings-telegram-loading = در حال بارگذاری تنظیمات { -telegram }...
settings-telegram-load-failed = بارگذاری تنظیمات { -telegram } ناموفق بود
settings-telegram-unknown = نامشخص
settings-telegram-session-active = فعال: { $duration }
settings-telegram-sessions-empty = نشست فعالی وجود ندارد
settings-telegram-session-revoke = لغو

settings-telegram-connection-title = اتصال
settings-telegram-connection-description = ربات { -telegram } خود را متصل کنید تا اعلان‌ها را دریافت کنید و { -brand } را از راه دور کنترل کنید.
settings-telegram-enable-label = فعال‌سازی { -telegram }
settings-telegram-enable-hint = فعال‌سازی یکپارچگی ربات { -telegram }
settings-telegram-token-label = توکن ربات
settings-telegram-token-saved = توکن ذخیره شد
settings-telegram-token-help = آن را از @BotFather در { -telegram } دریافت کنید
settings-telegram-token-input-saved =
    .placeholder = توکن ذخیره شد (برای تغییر، توکن جدید وارد کنید)
settings-telegram-token-input =
    .placeholder = توکن ربات را وارد کنید
settings-telegram-token-toggle =
    .title = نمایش/پنهان‌کردن
settings-telegram-chat-label = شناسه چت
settings-telegram-chat-connected = متصل به چت:
settings-telegram-chat-discover-hint = شناسه چت را به‌طور خودکار پیدا کنید
settings-telegram-chat-change =
    .title = تغییر
settings-telegram-chat-discover = یافتن شناسه چت
settings-telegram-discovery-step-add = ربات خود را به یک گروه { -telegram } اضافه کنید، یا یک چت مستقیم با آن شروع کنید
settings-telegram-discovery-step-privacy = برای گروه‌ها: @BotFather ← /mybots ← [ربات شما] ← Bot Settings ← Group Privacy را بررسی کنید
settings-telegram-discovery-privacy = <strong>حالت حریم خصوصی غیرفعال:</strong> ربات همه پیام‌های گروه را دریافت می‌کند<br/><strong>حالت حریم خصوصی فعال:</strong> ربات فقط پیام‌هایی را دریافت می‌کند که با @ به آن اشاره شده باشد
settings-telegram-discovery-step-send = یک پیام دلخواه بفرستید (یا اگر حالت حریم خصوصی فعال است، به ربات خود با @ اشاره کنید)
settings-telegram-discovery-listening = در انتظار پیام...
settings-telegram-discovery-select = انتخاب
settings-telegram-chat-id-label = شناسه:
settings-telegram-language-label = زبان پیام‌ها
settings-telegram-language-hint = زبان پیام‌ها و دکمه‌های ربات { -telegram }
settings-telegram-language-follow-app = پیروی از زبان برنامه
settings-telegram-test-label = آزمایش اتصال
settings-telegram-test-hint = برای تأیید پیکربندی، یک پیام آزمایشی ارسال کنید
settings-telegram-test-send = ارسال آزمایشی
settings-telegram-test-sending = در حال ارسال...
settings-telegram-restart-label = برای اعمال، راه‌اندازی مجدد کنید
settings-telegram-restart-hint = { -telegram } تا راه‌اندازی مجدد { -brand } تنظیمات قبلی خود را نگه می‌دارد
settings-telegram-restart-action = راه‌اندازی مجدد
settings-telegram-needs-token = ابتدا توکن ربات را اضافه کنید
settings-telegram-needs-chat = ابتدا یک گفتگو را متصل کنید
settings-telegram-needs-enabled = ابتدا { -telegram } را روشن کنید

settings-telegram-chat-type-private = خصوصی
settings-telegram-chat-type-group = گروه
settings-telegram-chat-type-supergroup = سوپرگروه
settings-telegram-chat-type-channel = کانال

settings-telegram-auth-title = احراز هویت دستورها
settings-telegram-auth-description = دستورهای { -telegram } از همان 2FA قفل صفحه داشبورد استفاده می‌کنند.
settings-telegram-auth-protected = محافظت‌شده
settings-telegram-auth-disabled = غیرفعال
settings-telegram-auth-not-configured = پیکربندی نشده
settings-telegram-auth-error = خطا
settings-telegram-auth-protected-note = دستورها با 2FA قفل صفحه محافظت می‌شوند. وقتی نشست‌ها منقضی شوند، کاربران باید کد برنامه احراز هویت خود را با دستور <code>/login</code> ارائه دهند.
settings-telegram-auth-disabled-note = 2FA قفل صفحه پیکربندی شده اما برای { -telegram } غیرفعال است. برای محافظت از دستورهای { -telegram }، گزینه «الزام 2FA برای دستورها» را در بالا فعال کنید.
settings-telegram-auth-missing-note = 2FA قفل صفحه پیکربندی نشده است. بدون 2FA، نشست‌های منقضی‌شده بدون تأیید دوباره فعال می‌شوند.
settings-telegram-auth-managed-in = 2FA در این بخش مدیریت می‌شود:
settings-telegram-auth-configure-in = 2FA را در این بخش پیکربندی کنید:
settings-telegram-auth-configure-suffix = تا برای دستورهای { -telegram } تأیید الزامی شود.
settings-telegram-security-link = تنظیمات امنیتی
settings-telegram-timeout-title = مهلت نشست
settings-telegram-timeout-description = مدتی که یک نشست احراز هویت‌شده فعال می‌ماند
settings-telegram-sessions-title = نشست‌های فعال

settings-telegram-notifications-title = تنظیمات اعلان
settings-telegram-notifications-description = انتخاب کنید کدام رویدادها اعلان { -telegram } ایجاد کنند.
settings-telegram-notify-opened-label = باز شدن پوزیشن
settings-telegram-notify-opened-hint = اعلان هنگام باز شدن پوزیشن جدید
settings-telegram-notify-closed-label = بسته شدن پوزیشن
settings-telegram-notify-closed-hint = اعلان هنگام بسته شدن پوزیشن
settings-telegram-notify-partial-label = خروج جزئی
settings-telegram-notify-partial-hint = اعلان برای خروج‌های جزئی از پوزیشن
settings-telegram-notify-dca-label = اجرای DCA
settings-telegram-notify-dca-hint = اعلان هنگام اجرای سفارش‌های DCA
settings-telegram-notify-errors-label = خطاها
settings-telegram-notify-errors-hint = اعلان برای خطاها و شکست‌ها
settings-telegram-notify-startup-label = شروع/توقف
settings-telegram-notify-startup-hint = اعلان هنگام شروع یا توقف ربات
settings-telegram-notify-filtering-label = هشدارهای فیلترینگ
settings-telegram-notify-filtering-hint = اعلان هنگام عبور توکن‌های جدید از معیارهای فیلترینگ
settings-telegram-notify-trades-label = هشدارهای معامله
settings-telegram-notify-trades-hint = اعلان برای معاملات مهم توکن‌های تحت پایش
settings-telegram-notify-daily-label = خلاصه روزانه
settings-telegram-notify-daily-hint = دریافت خلاصه فعالیت معاملاتی و سود و زیان روزانه

settings-telegram-features-title = قابلیت‌ها
settings-telegram-features-description = پیکربندی قابلیت‌های ربات { -telegram }.
settings-telegram-commands-label = فعال‌سازی دستورها
settings-telegram-commands-hint = امکان کنترل ربات با دستورهای { -telegram }
settings-telegram-require-2fa-label = الزام 2FA برای دستورها
settings-telegram-require-2fa-hint = وقتی نشست‌ها منقضی شوند، برای فعال‌سازی دوباره کد 2FA لازم است. از 2FA قفل صفحه استفاده می‌کند.
settings-telegram-inline-label = دکمه‌های اقدام درون‌خطی
settings-telegram-inline-hint = نمایش دکمه‌های اقدام در پیام‌های اعلان

settings-telegram-setting-save-failed = ذخیره تنظیم { -telegram } ممکن نشد
settings-telegram-discovery-start-failed = شروع یافتن ممکن نشد
settings-telegram-chat-selected = چت انتخاب شد
settings-telegram-chat-selected-restart = گفتگو انتخاب شد. { -telegram } پس از راه‌اندازی دوباره { -brand } به آن ارسال می‌کند
settings-telegram-chat-select-failed = انتخاب چت ممکن نشد
settings-telegram-test-sent = پیام آزمایشی ارسال شد
settings-telegram-test-failed = پیام آزمایشی ناموفق بود
settings-telegram-session-revoked = نشست لغو شد
settings-telegram-session-revoke-failed = لغو نشست ممکن نشد

## licenses_tab.js

settings-licenses-title = مجوزهای متن‌باز
settings-licenses-subtitle = { -brand } با نرم‌افزارهای متن‌باز زیر ساخته شده است
settings-licenses-footer = متن کامل مجوزها در مخزن پروژه و در کد منبع هر وابستگی در دسترس است.
settings-licenses-category-framework = چارچوب برنامه
settings-licenses-category-solana = بلاکچین Solana
settings-licenses-category-data = داده و ذخیره‌سازی
settings-licenses-category-networking = شبکه
settings-licenses-category-cryptography = رمزنگاری و کدگذاری
settings-licenses-category-assets = دارایی‌های رابط کاربری
settings-licenses-desc-electron = چارچوب برنامه دسکتاپ
settings-licenses-desc-tokio = اجرای ناهمگام برای Rust
settings-licenses-desc-axum = چارچوب وب‌سرور
settings-licenses-desc-tower = انتزاع‌های سرویس
settings-licenses-desc-hyper = پیاده‌سازی HTTP
settings-licenses-desc-solana-sdk = هسته SDK سولانا
settings-licenses-desc-solana-client = کلاینت RPC
settings-licenses-desc-solana-program = کتابخانه برنامه
settings-licenses-desc-spl-token = برنامه SPL Token
settings-licenses-desc-spl-token-2022 = افزونه‌های Token-2022
settings-licenses-desc-spl-associated-token-account = حساب‌های توکن وابسته
settings-licenses-desc-sqlite = موتور پایگاه داده توکار
settings-licenses-desc-rusqlite = اتصال SQLite برای Rust
settings-licenses-desc-r2d2 = مجموعه اتصال‌های پایگاه داده
settings-licenses-desc-serde = چارچوب سریال‌سازی
settings-licenses-desc-toml = تجزیه پیکربندی
settings-licenses-desc-reqwest = کلاینت HTTP
settings-licenses-desc-tokio-tungstenite = کلاینت WebSocket
settings-licenses-desc-rustls = پیاده‌سازی TLS
settings-licenses-desc-blake3 = تابع هش
settings-licenses-desc-sha-2 = هش SHA-256/512
settings-licenses-desc-bs58 = کدگذاری Base58
settings-licenses-desc-base64 = کدگذاری Base64
settings-licenses-desc-lucide-icons = کتابخانه فونت آیکون
settings-licenses-desc-inter = فونت رابط کاربری
settings-licenses-desc-jetbrains-mono = فونت تک‌فاصله
settings-licenses-desc-orbitron = فونت نمایشی
settings-licenses-desc-vazirmatn = فونت عربی و فارسی
settings-licenses-desc-noto-sans-devanagari = فونت دوناگری
settings-licenses-desc-noto-sans-sc = فونت چینی ساده‌شده
settings-licenses-desc-pretendard = فونت کره‌ای
settings-licenses-desc-pretendard-jp = فونت ژاپنی

## hints_tab.js

settings-hints-title = راهنماهای زمینه‌ای
settings-hints-description = راهنماهای زمینه‌ای همان آیکون‌های راهنما هستند که قابلیت‌های داشبورد را توضیح می‌دهند. همه راهنماها را در زیر مرور کنید و هر راهنمایی را که با «دیگر نمایش نده» پنهان کرده‌اید، یکی‌یکی یا یکجا بازگردانید.
settings-hints-hidden-label = راهنماهای پنهان
settings-hints-hidden-summary = { $hidden } از { $total } راهنما در حال حاضر پنهان است.
settings-hints-restore-all = بازگردانی همه راهنماها
settings-hints-toggle-shown =
    .title = نمایش این راهنما
settings-hints-toggle-shown-title = نمایش داده می‌شود
settings-hints-toggle-hidden-title = پنهان — برای نمایش روشن کنید
settings-hints-restore-title = بازگردانی همه راهنماها
settings-hints-restore-message = همه راهنماهای زمینه‌ای، از جمله همه راهنماهای پنهان‌شده، دوباره نمایش داده شوند؟
settings-hints-restore-confirm = بازگردانی همه
settings-hints-restored = همه راهنماها بازگردانده شد

## account_tab.js

settings-account-title = حساب { -brand }
settings-account-description = رایگان. با RPC خودتان اختیاری است: { -brand } بدون حساب هم با ارائه‌دهندگان عمومی معامله، کشف و رسم نمودار می‌کند. سرویس { -brand } به حساب نیاز دارد. پنل زیر نشان می‌دهد ورود به حساب چه چیزهایی اضافه می‌کند.
settings-account-description-service = رایگان و برای سرویس { -brand } لازم: خواندن کیف پول و سوآپ‌هایی که امضا می‌کنید با این حساب از screenerbot.io می‌گذرند. کلید خصوصی شما هرگز از این رایانه خارج نمی‌شود.
settings-account-data-title = داده { -brand }
settings-account-data-description = { -brand } در screenerbot.io یک سرویس مشترک دادهٔ بازار برای نمودارها، دادهٔ استخرها، گزارش‌های امنیتی و جزئیات توکن‌ها اجرا می‌کند تا نصب شما به‌تنهایی با محدودیت نرخ ارائه‌دهندگان عمومی روبه‌رو نشود. این سرویس برای محدود کردن سوءاستفاده به یک حساب رایگان نیاز دارد.
settings-account-data-fallback = وقتی در دسترس نباشد، { -brand } به‌طور خودکار به ارائه‌دهندگان عمومی برمی‌گردد. هیچ چیز متوقف نمی‌شود؛ نمودارها کندتر پر می‌شوند و تاریخچه کمتری دارند.
settings-account-data-fallback-service = در سرویس { -brand }، خواندن کیف پول و معاملات هم از screenerbot.io می‌گذرند: تا وقتی در دسترس نباشد، معامله متوقف می‌ماند و با برگشتن اتصال ادامه پیدا می‌کند.
settings-account-manage-title = مدیریت حساب
settings-account-manage-description = گذرواژه، نشانی ایمیل، دستگاه‌های متصل و پرداخت‌های ارجاع شما در وب‌سایت مدیریت می‌شود. لغو یک دستگاه در آنجا آن را از همه‌جا، از جمله همین دستگاه، خارج می‌کند.
settings-account-open-dashboard = باز کردن داشبورد شما

## navigation_tab.js

settings-navigation-title = تب‌های پیمایش
settings-navigation-hint = برای تغییر ترتیب، موارد را بکشید. نمایش را با کلید تغییر دهید.
settings-navigation-section-layout = چیدمان
settings-navigation-overflow-label = زبانه‌هایی که جا نمی‌شوند
settings-navigation-overflow-hint = ردیف زبانه‌ها را افقی پیمایش کنید، یا زبانه‌هایی را که جا نمی‌شوند در منوی «بیشتر» در انتهای آن جمع کنید.
settings-navigation-overflow-scroll = پیمایش
settings-navigation-overflow-menu = منوی «بیشتر»
settings-navigation-drag-handle =
    .title = برای تغییر ترتیب بکشید
settings-navigation-defaults-failed = بارگذاری پیمایش پیش‌فرض ممکن نشد
settings-navigation-reset = پیمایش به حالت پیش‌فرض بازنشانی شد

## data_tab.js

settings-data-usage-title = فضای ذخیره‌سازی
settings-data-usage-description = فضای دیسکی که داده‌های { -brand } به تفکیک نوع اشغال می‌کنند. داده‌های بازار قابل‌بازسازی به‌طور خودکار پاک‌سازی می‌شوند تا در محدوده فضای ذخیره‌سازی بمانند؛ کلیدهای کیف پول، سوابق معاملات و تنظیمات هرگز حذف نمی‌شوند.
settings-data-usage-loading = در حال اندازه‌گیری فضای ذخیره‌سازی...
settings-data-usage-load-failed = اندازه‌گیری فضای ذخیره‌سازی ممکن نشد
settings-data-usage-total = استفاده‌شده
settings-data-usage-of-limit = { $used } از { $limit }
settings-data-usage-free-disk = فضای آزاد دیسک
settings-data-usage-free-disk-value = { $free } از { $total }
settings-data-usage-free-disk-unknown = نامشخص
settings-data-category-wallet-keys = کلیدهای کیف پول
settings-data-category-trading-records = سوابق معاملات
settings-data-category-wallet-history = تاریخچه کیف پول
settings-data-category-market-data = داده‌های بازار
settings-data-category-settings = تنظیمات
settings-data-category-recovery-copies = نسخه‌های بازیابی
settings-data-category-updates = به‌روزرسانی‌ها
settings-data-category-logs = لاگ‌ها
settings-data-category-leftovers = فایل‌های باقی‌مانده
settings-data-last-cleanup-label = آخرین پاک‌سازی
settings-data-last-cleanup-never = هنوز اجرا نشده
settings-data-last-cleanup-value = { $when }، { $freed } آزاد شد
settings-data-last-cleanup-failures-label = مشکلات پاک‌سازی
settings-data-last-cleanup-failures =
    { $count ->
        [one] { $count } مرحله ناموفق بود؛ در دور بعد دوباره امتحان می‌شود
       *[other] { $count } مرحله ناموفق بود؛ در دور بعد دوباره امتحان می‌شوند
    }
settings-data-limit-label = محدودیت فضای ذخیره‌سازی
settings-data-limit-hint = فضای دیسکی که داده‌های { -brand } می‌توانند پیش از پاک‌سازی داده‌های بازار قابل‌بازسازی اشغال کنند. حالت خودکار خود را با این دیسک تطبیق می‌دهد، تا سقف 5 GB.
settings-data-limit-automatic = خودکار ({ $size })
settings-data-limit-save-failed = ذخیره محدودیت فضای ذخیره‌سازی ممکن نشد
settings-data-cleanup-now-label = پاک‌سازی همین حالا
settings-data-cleanup-now-hint = لاگ‌ها و دانلودهای قدیمی را حذف می‌کند، داده‌های بازاری را که دیگر استفاده نمی‌شوند پاک می‌کند و پایگاه‌های داده را فشرده می‌کند. به پوزیشن‌های باز و توکن‌های نگه‌داشته‌شده و تحت نظر هرگز دست زده نمی‌شود.
settings-data-cleanup-now-action = پاک‌سازی
settings-data-cleanup-now-running = در حال پاک‌سازی...
settings-data-cleanup-now-failed = شروع پاک‌سازی ممکن نشد
settings-data-directory-label = پوشه داده
settings-data-directory-copied = پوشه داده
settings-data-config-path-copied = مسیر پیکربندی
settings-data-path-unavailable = در دسترس نیست
settings-data-path-copy-title = برای کپی مسیر کلیک کنید
settings-data-path-copy-failed = کپی مسیر ناموفق بود

settings-data-config-title = مدیریت پیکربندی
settings-data-config-description = پیکربندی ربات خود را خروجی بگیرید، وارد کنید و مدیریت کنید. پیش از تغییرات بزرگ نسخه پشتیبان نگه دارید.
settings-data-config-export = خروجی پیکربندی
settings-data-config-import = ورود پیکربندی
settings-data-config-reset = بازگشت به پیش‌فرض
settings-data-config-location-label = محل پیکربندی
settings-data-config-fetch-failed = دریافت پیکربندی ناموفق بود
settings-data-config-exported = پیکربندی خروجی گرفته شد
settings-data-config-export-failed = خروجی گرفتن از پیکربندی ناموفق بود: { $message }
settings-data-config-import-title = ورود پیکربندی
settings-data-config-import-message = این پیکربندی وارد شود؟ تنظیمات فعلی بازنویسی می‌شوند. اعتبارنامه‌های کیف پول حفظ می‌شوند.
settings-data-config-imported = پیکربندی با موفقیت وارد شد. برخی تغییرات ممکن است به راه‌اندازی مجدد نیاز داشته باشند.
settings-data-config-import-failed = ورود پیکربندی ناموفق بود: { $message }
settings-data-config-reset-title = بازنشانی پیکربندی
settings-data-config-reset-message = همه تنظیمات به پیش‌فرض بازنشانی شوند؟ اعتبارنامه‌های کیف پول حفظ می‌شوند، اما همه تنظیمات دیگر بازنشانی می‌شوند.
settings-data-config-reset-done = پیکربندی به پیش‌فرض بازنشانی شد
settings-data-config-reset-failed = بازنشانی پیکربندی ناموفق بود: { $message }
settings-data-unknown-error = خطای ناشناخته

settings-data-cleanup-title = پاک‌سازی داده
settings-data-cleanup-description = با حذف داده‌های قدیمی یا بلااستفاده، فضای دیسک آزاد کنید. این اقدام‌ها قابل بازگشت نیستند.

settings-data-cache-clear-label = پاک‌کردن همه کش OHLCV
settings-data-cache-clear-hint = همه داده‌های کندل کش‌شده را پاک می‌کند و هر توکن پایش‌شده را از ابتدا دوباره دریافت می‌کند. اگر نمودارها نادرست به نظر می‌رسند یا پس از به‌روزرسانی منطق داده، از آن استفاده کنید.
settings-data-cache-clear = پاک‌کردن کش OHLCV
settings-data-cache-clearing = در حال پاک‌کردن...
settings-data-cache-confirm-title = پاک‌کردن همه کش OHLCV
settings-data-cache-confirm-message = همه داده‌های کندل کش‌شده برای همه توکن‌ها پاک شود؟ توکن‌های پایش‌شده تاریخچه خود را از ابتدا دوباره دریافت می‌کنند. این اقدام قابل بازگشت نیست.
settings-data-candles-count =
    { $count ->
        [one] { $count } کندل
       *[other] { $count } کندل
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } توکن
       *[other] { $count } توکن
    }
settings-data-cache-cleared = پاک شد: { $candles } در { $tokens }؛ در حال دریافت دوباره
settings-data-cache-clear-failed = پاک‌کردن کش OHLCV ناموفق بود
settings-data-cache-clear-failed-detail = پاک‌کردن کش OHLCV ناموفق بود: { $message }

settings-data-ui-cache-label = کش وضعیت رابط کاربری
settings-data-ui-cache-hint = ترجیحات ذخیره‌شده جدول، وضعیت فیلترها و تنظیمات نمایش را پاک کنید.
settings-data-ui-cache-clear = پاک‌کردن کش رابط کاربری
settings-data-ui-cache-confirm-title = پاک‌کردن وضعیت رابط کاربری
settings-data-ui-cache-confirm-message = همه ترجیحات ذخیره‌شده رابط کاربری پاک شود؟ ستون‌های جدول، فیلترها و تنظیمات نمایش بازنشانی می‌شوند.
settings-data-ui-cache-cleared =
    پاک شد: { $count ->
        [one] { $count } تنظیم کش‌شده رابط کاربری
       *[other] { $count } تنظیم کش‌شده رابط کاربری
    }

settings-data-folder-label = باز کردن پوشه داده
settings-data-folder-hint = پوشه حاوی همه داده‌های { -brand } را در مدیر فایل خود باز کنید.
settings-data-folder-open = باز کردن پوشه
settings-data-folder-open-failed = باز کردن پوشه داده ممکن نشد

## Wallet tab (scripts/ui/settings/wallet_tab.js)

settings-wallet-title = کیف پول معاملاتی
settings-wallet-description = کیف پولی که { -brand } هر معامله را با آن امضا می‌کند. کلید خصوصی آن روی همین دستگاه رمزگذاری می‌شود و هرگز از آن خارج نمی‌شود.
settings-wallet-address = آدرس
settings-wallet-balance = موجودی
settings-wallet-unread = کیف پول معاملاتی خوانده نشد.
settings-wallet-open = باز کردن کیف پول‌ها
settings-wallet-key-title = کلید خصوصی
settings-wallet-key-description = از این کیف پول نسخه پشتیبان بگیرید یا آن را در اپ کیف پول دیگری باز کنید. هر کسی که کلید را داشته باشد، همه دارایی این کیف پول را در اختیار دارد.
settings-wallet-key-reveal = نمایش کلید خصوصی
settings-wallet-replace-title = معامله با کیف پول دیگر
settings-wallet-replace-description = کیف پول تازه‌ای بسازید یا یکی را وارد کنید. { -brand } با آن دوباره راه‌اندازی می‌شود؛ کیف پول فعلی دارایی‌اش را نگه می‌دارد و در کیف پول‌های دیگر می‌ماند.
settings-wallet-replace-action = جایگزینی کیف پول

## Connection tab (scripts/ui/settings/connection_tab.js)

settings-connection-title = اتصال
settings-connection-description = { -brand } چگونه به { -solana } دسترسی پیدا می‌کند.
settings-connection-choice =
    .aria-label = اتصال
settings-connection-active = فعال
settings-connection-service-title = سرویس { -brand }
settings-connection-service-detail = قیمت‌ها هر چند ثانیه یک بار به‌روز می‌شوند. کپی‌تریدینگ، پایش کیف پول‌ها، موجودی کیف پول‌های دیگر، تاریخچه قدیمی‌تر، سوآپ مستقیم در استخر و قیمت زندهٔ استخرها به RPC خودتان نیاز دارند.
settings-connection-own-title = RPC خودتان
settings-connection-own-detail = قیمت‌ها حدود دو بار در ثانیه و همهٔ قابلیت‌ها.
settings-connection-apply = اعمال و راه‌اندازی دوباره
settings-connection-to-service-title = به سرویس { -brand } تغییر کند؟
settings-connection-to-service-message = کپی‌تریدینگ، پایش کیف پول‌ها، موجودی کیف پول‌های دیگر، تاریخچه قدیمی‌تر، سوآپ مستقیم در استخر و قیمت زندهٔ استخرها خاموش می‌شوند؛ تنظیمات آن‌ها حفظ می‌شود. هر زمان از حساب خارج باشید معامله متوقف می‌شود. { -brand } برای تغییر دوباره راه‌اندازی می‌شود.
settings-connection-to-own-title = به RPC خودتان تغییر کند؟
settings-connection-to-own-message = { -brand } نقاط اتصال شما را بررسی می‌کند و سپس برای تغییر دوباره راه‌اندازی می‌شود. همهٔ قابلیت‌ها دوباره روشن می‌شوند.
settings-connection-signed-in-as = متصل · واردشده با نام { $name }
settings-connection-sending-title = ارسال تراکنش‌ها
settings-connection-sending-label = ارسال تراکنش‌ها از طریق { -brand }
settings-connection-sending-own-hint = هنگامی که وارد حساب هستید، سوآپ‌هایی که روی این رایانه امضا می‌کنید از طریق screenerbot.io پخش می‌شوند. وقتی ممکن نباشد، از RPC شما استفاده می‌شود.
settings-connection-sending-service-hint = با سرویس { -brand } همیشه روشن است: سوآپ‌هایی که روی این رایانه امضا می‌کنید از همین راه به { -solana } می‌رسند.
settings-connection-switch-failed = تغییر اتصال ممکن نشد.
