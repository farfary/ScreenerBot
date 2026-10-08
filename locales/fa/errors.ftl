# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

# Framing for an operation message followed by its technical cause.
errors-with-details = { $message }: { $details }

# Configuration
errors-config-save-failed = ذخیره پیکربندی ناموفق بود

# Authentication
errors-auth-current-password-incorrect = رمز عبور فعلی نادرست است
errors-auth-current-password-required = برای تغییر رمز عبور، وارد کردن رمز عبور فعلی الزامی است
errors-auth-password-too-short = رمز عبور باید دست‌کم 4 کاراکتر باشد
errors-auth-password-too-long = رمز عبور باید حداکثر 128 کاراکتر باشد
errors-auth-hash-failed = هش کردن رمز عبور ناموفق بود
errors-auth-not-enabled = احراز هویت فعال نیست
errors-auth-no-password = هنوز رمز عبوری تنظیم نشده است
errors-auth-password-incorrect = رمز عبور نادرست است
errors-auth-required = احراز هویت لازم است. برای دسترسی به این اندپوینت وارد شوید.
errors-auth-totp-invalid = کد 2FA نامعتبر یا منقضی است
errors-auth-totp-verify-failed = تأیید کد 2FA ناموفق بود
errors-auth-totp-password-required = پیش از فعال‌سازی 2FA باید رمز عبور تنظیم شود
errors-auth-totp-uri-failed = تولید URI مربوط به TOTP ناموفق بود
errors-auth-totp-qr-failed = تولید کد QR ناموفق بود
errors-auth-totp-secret-required = کلید مخفی الزامی است
errors-auth-totp-save-failed = ذخیره پیکربندی TOTP ناموفق بود
errors-auth-totp-code-invalid = کد تأیید نامعتبر است. کد را بررسی کنید و دوباره تلاش کنید.
errors-auth-totp-code-verify-failed = تأیید کد ناموفق بود

# Request security
errors-security-invalid-local-request = درخواست باید از داشبورد محلی ارسال شده باشد
errors-security-invalid-token = توکن امنیتی نامعتبر است
errors-security-token-required = توکن امنیتی لازم است. این اندپوینت فقط از داخل { -brand } در دسترس است.

# Lockscreen
errors-lockscreen-no-password-set = رمزی تنظیم نشده است
errors-lockscreen-invalid-type = نوع رمز نامعتبر است. باید 'pin4'، 'pin6' یا 'text' باشد
errors-lockscreen-invalid-format = رمز با نوع انتخاب‌شده مطابقت ندارد
errors-lockscreen-no-current-password = در حال حاضر رمزی تنظیم نشده است
errors-lockscreen-password-incorrect = رمز نادرست است
errors-lockscreen-enable-needs-password = بدون تنظیم رمز، فعال‌سازی صفحه قفل ممکن نیست

# Account
errors-account-signin-failed = ورود ناموفق بود
# The reason is the account server's own wording, shown exactly as sent.
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = ورود در دسترس نیست
errors-account-browser-open-failed = باز کردن مرورگر ممکن نشد. مرورگر پیش‌فرض خود را باز کنید و دوباره تلاش کنید.
errors-account-signup-open-failed = باز کردن صفحه ثبت‌نام ممکن نشد. screenerbot.io/signup را در مرورگر خود باز کنید.
errors-account-credentials-required = ایمیل و رمز عبور خود را وارد کنید.
errors-account-signout-failed = خروج ناموفق بود
errors-account-gateway-update-failed = به‌روزرسانی تنظیم گیت‌وی ممکن نشد

# Localization
errors-i18n-locale-not-registered = زبان ثبت نشده است
errors-i18n-catalog-encode-failed = کدگذاری کاتالوگ ممکن نشد

# System
errors-system-paths-init-failed = ساخت پوشه‌های برنامه ممکن نشد
errors-system-open-data-failed = باز کردن پوشه داده ممکن نشد
errors-system-url-empty = URL نمی‌تواند خالی باشد
errors-system-open-url-failed = باز کردن URL ممکن نشد

# Initialization
errors-initialization-required = پیش از دسترسی به این اندپوینت، راه‌اندازی اولیه ربات لازم است. فرایند راه‌اندازی را از طریق رابط وب کامل کنید.
errors-initialization-onboarding-update-failed = به‌روزرسانی وضعیت راه‌اندازی اولیه ناموفق بود
errors-initialization-validation-required = پیش از ذخیره تنظیمات، تأیید اعتبارنامه‌ها لازم است
errors-initialization-encrypt-failed = رمزگذاری کلید خصوصی ناموفق بود

# Dashboard state
errors-ui-state-save-failed = ذخیره وضعیت ناموفق بود
errors-ui-state-clear-failed = پاک کردن وضعیت ناموفق بود

# Agent control
errors-agent-config-failed = پیکربندی کنترل عامل ناموفق بود
errors-agent-invalid-parameters = پارامترهای نامعتبر
errors-agent-wallet-key-material = مواد کلید کیف پول برای عامل قابل دسترسی نیست
errors-agent-store-failed = ذخیره‌ساز کنترل عامل ناموفق بود
errors-agent-invalid-pairing-request = درخواست جفت‌سازی نامعتبر است
errors-agent-pairing-rejected = اعتبارنامه جفت‌سازی رد شد
errors-agent-disabled = کنترل عامل غیرفعال است
errors-agent-approval-not-pending = تأیید دیگر در انتظار نیست
errors-agent-approval-not-found = تأیید پیدا نشد
errors-agent-bridge-task-failed = وظیفه پل کنترل عامل ناموفق بود
errors-agent-task-failed = وظیفه کنترل عامل ناموفق بود
errors-agent-pairing-not-found = جفت‌سازی فعالی با این شناسه وجود ندارد
errors-agent-permissions-update-failed = به‌روزرسانی مجوزها ناموفق بود

# Connectivity
errors-connectivity-endpoint-not-found = اندپوینت '{ $endpoint }' پیدا نشد یا پایش نمی‌شود

# Copy trading
errors-copy-task-not-found = وظیفه کپی پیدا نشد
errors-copy-holding-not-found = هیچ دارایی آزمایشی بازی در این توکن وجود ندارد
errors-copy-live-confirmation-required = فعال‌سازی زنده کپی‌تریدینگ به تأیید صریح نیاز دارد
errors-copy-task-invalid = وظیفه کپی نامعتبر است
errors-copy-request-rejected = درخواست کپی‌تریدینگ رد شد
errors-copy-task-limit = به حداکثر تعداد وظایف کپی فعال رسیده‌اید
errors-copy-watch-rejected = پایش هدف کپی ممکن نشد
errors-copy-live-unavailable = کپی‌تریدینگ زنده در دسترس نیست
errors-copy-task-live = پیش از حذف، وظیفه زنده را متوقف کنید
errors-copy-task-owns-positions = وظیفه کپی هنوز پوزیشن باز دارد
errors-copy-request-failed = درخواست کپی‌تریدینگ ناموفق بود

# Strategies
errors-strategies-not-found = استراتژی پیدا نشد
errors-strategies-invalid-type = نوع استراتژی نامعتبر است. باید ENTRY یا EXIT باشد
errors-strategies-list-failed = دریافت استراتژی‌ها ناموفق بود
errors-strategies-get-failed = دریافت استراتژی ناموفق بود
errors-strategies-serialize-rules-failed = سریال‌سازی قوانین ناموفق بود
errors-strategies-invalid-rules-json = JSON قوانین نامعتبر است
errors-strategies-already-exists = استراتژی با شناسه '{ $id }' از قبل وجود دارد
errors-strategies-validation-failed = اعتبارسنجی استراتژی ناموفق بود
errors-strategies-create-failed = ساخت استراتژی ناموفق بود
errors-strategies-update-failed = به‌روزرسانی استراتژی ناموفق بود
errors-strategies-update-enabled-failed = به‌روزرسانی وضعیت فعال بودن استراتژی ناموفق بود
errors-strategies-delete-failed = حذف استراتژی ناموفق بود
errors-strategies-deploy-failed = استقرار استراتژی ناموفق بود
errors-strategies-no-performance = داده عملکردی برای این استراتژی موجود نیست
errors-strategies-performance-failed = دریافت آمار عملکرد ناموفق بود
errors-strategies-schemas-failed = دریافت طرحواره‌های شرط ناموفق بود
errors-strategies-evaluation-failed = ارزیابی استراتژی ناموفق بود

# Transactions
errors-transactions-own-wallet-unavailable = کیف پول اصلی پیکربندی نشده است
errors-transactions-invalid-subject = موضوع تراکنش آدرس معتبر سولانا نیست
errors-transactions-subject-not-watched = موضوع تراکنش یک کیف پول تحت پایش نیست
errors-transactions-watch-store-unavailable = کیف پول‌های تحت پایش در دسترس نیستند

# Wallet
errors-wallet-unavailable = کیف پول اصلی در دسترس نیست
errors-wallet-changed = کیف پول اصلی تغییر کرد؛ صفحه را تازه‌سازی کنید و دوباره تلاش کنید
errors-wallet-qr-failed = تولید کد QR کیف پول ممکن نشد

# Updates
errors-updates-none-available = به‌روزرسانی‌ای برای دانلود وجود ندارد
errors-updates-version-changed = به‌روزرسانی موجود تغییر کرد؛ دوباره به‌روزرسانی‌ها را بررسی کنید
errors-updates-check-failed = بررسی به‌روزرسانی ناموفق بود
errors-updates-download-failed = شروع دانلود به‌روزرسانی ممکن نشد
errors-updates-history-unavailable = تاریخچه نسخه‌ها در دسترس نیست
errors-updates-apply-failed = اعمال به‌روزرسانی ممکن نشد
errors-updates-install-failed = باز کردن نصب‌کننده به‌روزرسانی ممکن نشد

# Telegram
errors-telegram-settings-update-failed = به‌روزرسانی تنظیمات ناموفق بود
errors-telegram-disabled = { -telegram } فعال نیست
errors-telegram-not-configured = توکن ربات یا شناسه چت پیکربندی نشده است
errors-telegram-send-failed = ارسال پیام ناموفق بود
errors-telegram-notifier-failed = ساخت اعلان‌رسان ناموفق بود
errors-telegram-token-required = ابتدا باید توکن ربات پیکربندی شود
errors-telegram-discovery-failed = شروع کشف ناموفق بود
errors-telegram-chat-select-failed = انتخاب چت ناموفق بود

# Assistant chat
errors-chat-message-empty = پیام نمی‌تواند خالی باشد
errors-chat-message-too-long = پیام از حداکثر طول 10,000 کاراکتر بیشتر است
errors-chat-database-unavailable = پایگاه‌داده گفتگو راه‌اندازی نشده است
errors-chat-session-not-found = گفتگوی { $id } پیدا نشد
errors-chat-session-validate-failed = اعتبارسنجی گفتگو ناموفق بود
errors-chat-engine-unavailable = موتور گفتگو راه‌اندازی نشده است
errors-chat-process-failed = پردازش پیام گفتگو ناموفق بود
errors-chat-stream-serialize-failed = سریال‌سازی رویداد گفتگو ناموفق بود
errors-chat-sessions-list-failed = دریافت فهرست گفتگوها ناموفق بود
errors-chat-session-create-failed = ساخت گفتگو ناموفق بود
errors-chat-session-get-failed = دریافت گفتگو ناموفق بود
errors-chat-messages-get-failed = دریافت پیام‌های گفتگو ناموفق بود
errors-chat-session-delete-failed = حذف گفتگو ناموفق بود
errors-chat-messages-load-failed = دریافت پیام‌ها ناموفق بود
errors-chat-summarize-empty = خلاصه‌سازی گفتگوی خالی ممکن نیست
errors-chat-provider-invalid = ارائه‌دهنده نامعتبر: { $provider }
errors-chat-summary-save-failed = ذخیره خلاصه ناموفق بود
errors-chat-title-empty-session = تولید عنوان برای گفتگوی خالی ممکن نیست
errors-chat-no-user-message = پیامی از کاربر در گفتگو پیدا نشد
errors-chat-title-save-failed = به‌روزرسانی عنوان گفتگو ناموفق بود
errors-chat-confirmation-save-failed = ذخیره پاسخ تأیید ناموفق بود
errors-chat-confirmation-failed = پردازش تأیید ناموفق بود
errors-chat-summary-failed = تولید خلاصه ناموفق بود

# Assistant automation
errors-automation-database-unavailable = پایگاه‌داده راه‌اندازی نشده است
errors-automation-tasks-list-failed = دریافت فهرست وظایف ناموفق بود
errors-automation-name-empty = نام وظیفه نمی‌تواند خالی باشد
errors-automation-instruction-empty = دستورالعمل وظیفه نمی‌تواند خالی باشد
errors-automation-schedule-type-invalid = schedule_type نامعتبر است. باید یکی از این‌ها باشد: interval، daily یا weekly
errors-automation-schedule-value-invalid = schedule_value نامعتبر است
errors-automation-task-create-failed = ساخت وظیفه ناموفق بود
errors-automation-task-not-found = وظیفه پیدا نشد
errors-automation-task-get-failed = دریافت وظیفه ناموفق بود
errors-automation-schedule-invalid = زمان‌بندی نامعتبر است
errors-automation-tool-permissions-invalid = tool_permissions باید 'full' یا 'readonly' باشد
errors-automation-priority-invalid = priority باید 'low'، 'medium' یا 'high' باشد
errors-automation-task-update-failed = به‌روزرسانی وظیفه ناموفق بود
errors-automation-task-running-delete = تا زمانی که وظیفه در حال اجراست، حذف آن ممکن نیست
errors-automation-task-delete-failed = حذف وظیفه ناموفق بود
errors-automation-task-toggle-failed = تغییر وضعیت وظیفه ناموفق بود
errors-automation-task-disabled = اجرای وظیفه غیرفعال ممکن نیست
errors-automation-task-already-running = وظیفه از قبل در حال اجراست
errors-automation-runs-list-failed = دریافت فهرست اجراها ناموفق بود
errors-automation-recent-runs-failed = دریافت فهرست اجراهای اخیر ناموفق بود
errors-automation-run-not-found = اجرا پیدا نشد
errors-automation-run-get-failed = دریافت اجرا ناموفق بود
errors-automation-stats-failed = دریافت آمار ناموفق بود

# LLM providers
errors-llm-config-update-failed = به‌روزرسانی پیکربندی LLM ناموفق بود
errors-llm-provider-unknown = ارائه‌دهنده ناشناخته: { $provider }
errors-llm-manager-unavailable = مدیر LLM راه‌اندازی نشده است
errors-llm-provider-disabled = ارائه‌دهنده '{ $provider }' پیکربندی نشده یا غیرفعال است
errors-llm-provider-config-update-failed = به‌روزرسانی پیکربندی ارائه‌دهنده ناموفق بود
errors-llm-provider-test-failed = آزمایش ارائه‌دهنده ناموفق بود
# The reason is the provider's own wording, shown exactly as sent.
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = به‌روزرسانی پیکربندی تحلیل ناموفق بود
errors-llm-analysis-unavailable = موتور تحلیل راه‌اندازی نشده است
errors-llm-analysis-disabled = قابلیت‌های LLM غیرفعال هستند. ابتدا [llm] را فعال کنید.
errors-llm-analysis-priority-invalid = اولویت نامعتبر: '{ $priority }'. از 'high'، 'medium' یا 'low' استفاده کنید.
errors-llm-analysis-evaluation-failed = تحلیل مدل ناموفق بود
errors-llm-analysis-instructions-list-failed = دریافت فهرست دستورالعمل‌ها ناموفق بود
errors-llm-analysis-instruction-not-found = دستورالعمل { $id } پیدا نشد
errors-llm-analysis-instruction-get-failed = دریافت دستورالعمل ناموفق بود
errors-llm-analysis-instruction-created-retrieve-failed = بازیابی دستورالعمل ساخته‌شده ناموفق بود
errors-llm-analysis-instruction-create-failed = ساخت دستورالعمل ناموفق بود
errors-llm-analysis-instruction-updated-retrieve-failed = بازیابی دستورالعمل به‌روزشده ناموفق بود
errors-llm-analysis-instruction-update-failed = به‌روزرسانی دستورالعمل ناموفق بود
errors-llm-analysis-instruction-delete-failed = حذف دستورالعمل ناموفق بود
errors-llm-analysis-instructions-reorder-failed = تغییر ترتیب دستورالعمل‌ها ناموفق بود
errors-llm-analysis-decisions-list-failed = دریافت تاریخچه تصمیم‌ها ناموفق بود
errors-llm-analysis-decision-not-found = تصمیم { $id } پیدا نشد
errors-llm-analysis-decision-get-failed = دریافت تصمیم ناموفق بود

# Wallets
errors-wallets-list-failed = دریافت فهرست کیف پول‌ها ناموفق بود
errors-wallets-name-empty = نام کیف پول نمی‌تواند خالی باشد
errors-wallets-create-failed = ساخت کیف پول ناموفق بود
errors-wallets-key-empty = کلید خصوصی نمی‌تواند خالی باشد
errors-wallets-already-exists = کیف پول از قبل وجود دارد
errors-wallets-key-invalid = قالب کلید خصوصی نامعتبر است
errors-wallets-import-failed = درون‌ریزی کیف پول ناموفق بود
errors-wallets-summary-failed = دریافت خلاصه کیف پول‌ها ناموفق بود
errors-wallets-no-main-wallet = کیف پول اصلی پیکربندی نشده است
errors-wallets-main-get-failed = دریافت کیف پول اصلی ناموفق بود
errors-wallets-not-found = کیف پول پیدا نشد
errors-wallets-get-failed = دریافت کیف پول ناموفق بود
errors-wallets-update-failed = به‌روزرسانی کیف پول ناموفق بود
errors-wallets-delete-failed = حذف کیف پول ناموفق بود
errors-wallets-export-failed = خروجی گرفتن از کیف پول ناموفق بود
errors-wallets-set-main-failed = تنظیم کیف پول اصلی ناموفق بود
errors-wallets-archive-failed = بایگانی کیف پول ناموفق بود
errors-wallets-restore-failed = بازیابی کیف پول ناموفق بود
errors-wallets-export-format-unsupported = در حال حاضر فقط قالب CSV پشتیبانی می‌شود
# The confirmation is the exact phrase the request must carry.
errors-wallets-export-confirmation-required = باید با ارائه این عبارت تأیید کنید: «{ $confirmation }»
errors-wallets-export-no-ids = شناسه کیف پولی ارائه نشده است
errors-wallets-export-bulk-failed = خروجی گرفتن از کیف پول‌ها ناموفق بود
errors-wallets-export-no-match = کیف پولی مطابق شناسه‌های ارائه‌شده پیدا نشد
errors-wallets-import-file-too-large = حجم فایل از حداکثر { $megabytes } MB بیشتر است
errors-wallets-import-read-failed = خواندن فایل بارگذاری‌شده ناموفق بود
errors-wallets-import-no-file = فایلی بارگذاری نشده است. از فیلد 'file' در فرم چندبخشی استفاده کنید
errors-wallets-import-encoding-invalid = فایل CSV باید با کدگذاری UTF-8 باشد
errors-wallets-import-csv-parse-failed = تجزیه فایل CSV ناموفق بود
errors-wallets-import-excel-parse-failed = تجزیه فایل Excel ناموفق بود
errors-wallets-import-format-unsupported = قالب فایل پشتیبانی نمی‌شود. از .csv، .xlsx یا .xls استفاده کنید
errors-wallets-import-file-empty = فایل هیچ ردیف داده‌ای ندارد
errors-wallets-import-existing-check-failed = بررسی کیف پول‌های موجود ناموفق بود
errors-wallets-import-mapping-invalid = ستون‌های الزامی وجود ندارند: { $columns }
errors-wallets-import-session-not-found = نشست درون‌ریزی پیدا نشد یا منقضی شده است. فایل را دوباره بارگذاری کنید
errors-wallets-import-no-valid-rows = ردیف معتبری برای درون‌ریزی وجود ندارد

# Wallet watching
errors-wallet-watch-list-failed = دریافت فهرست اهداف پایش ناموفق بود
errors-wallet-watch-address-empty = آدرس نمی‌تواند خالی باشد
errors-wallet-watch-add-failed = افزودن هدف پایش ناموفق بود
errors-wallet-watch-remove-failed = حذف هدف پایش ناموفق بود
errors-wallet-watch-update-failed = به‌روزرسانی هدف پایش ناموفق بود
errors-wallet-watch-budget-failed = به‌روزرسانی بودجه پایش ممکن نشد
errors-wallet-watch-resume-failed = از سرگیری پایش ممکن نشد
errors-wallet-watch-approval-failed = به‌روزرسانی تأیید { -helius } ممکن نشد
errors-wallet-watch-status-failed = دریافت وضعیت پایش ناموفق بود

# Tools
errors-tools-wallet-failed = دریافت کیف پول ناموفق بود
errors-tools-wallet-address-failed = دریافت آدرس کیف پول ناموفق بود
errors-tools-accounts-scan-failed = اسکن حساب‌ها ناموفق بود
errors-tools-token-accounts-scan-failed = اسکن حساب‌های توکن ناموفق بود
errors-tools-token-accounts-get-failed = دریافت حساب‌های توکن ناموفق بود
errors-tools-cleanup-failed = پاکسازی ناموفق بود
errors-tools-cache-clear-failed = پاک کردن کش ناموفق بود
errors-tools-no-tokens = توکنی برای برن انتخاب نشده است
errors-tools-burn-failed = برن توکن‌ها ناموفق بود
errors-tools-favorites-list-failed = دریافت علاقه‌مندی‌ها ناموفق بود
errors-tools-favorite-type-invalid = نوع ابزار نامعتبر است. باید یکی از این‌ها باشد: { $types }
errors-tools-favorite-add-failed = افزودن علاقه‌مندی ناموفق بود
errors-tools-favorite-not-found = علاقه‌مندی پیدا نشد
errors-tools-favorite-update-failed = به‌روزرسانی علاقه‌مندی ناموفق بود
errors-tools-favorite-delete-failed = حذف علاقه‌مندی ناموفق بود
errors-tools-favorite-use-failed = به‌روزرسانی شمارنده استفاده ناموفق بود
errors-tools-pool-search-failed = جست‌وجوی استخر برای توکن { $mint } ناموفق بود
errors-tools-watched-list-failed = دریافت فهرست توکن‌های تحت پایش ناموفق بود
errors-tools-watched-add-failed = افزودن توکن تحت پایش ناموفق بود
errors-tools-watched-delete-failed = حذف توکن تحت پایش ناموفق بود
errors-tools-mint-invalid = آدرس مینت توکن نامعتبر است
errors-tools-wallets-get-failed = دریافت کیف پول‌ها ناموفق بود
errors-tools-balance-failed = دریافت موجودی کیف پول ناموفق بود
errors-tools-session-active = عملیات چندکیف‌پولی دیگری در حال انجام است
# The reason is the tool configuration check's own wording, shown exactly as produced.
errors-tools-config-invalid = پیکربندی ابزار نامعتبر است: { $reason }
errors-tools-config-rejected = پیکربندی ابزار نامعتبر است
errors-tools-consolidate-failed = تجمیع کیف پول‌ها ناموفق بود
errors-tools-ata-cleanup-failed = پاکسازی ATAها ناموفق بود
errors-tools-routers-unavailable = روترهای سواپ هنوز آماده نیستند
errors-tools-router-disabled-chain-settings = { $router } در تنظیمات > زنجیره‌ها غیرفعال است
errors-tools-router-unknown = روتر سواپ ناشناخته '{ $router }'
errors-tools-session-type-mismatch = نوع نشست { $actual } است، نه { $expected }
errors-tools-session-not-found = نشست پیدا نشد
errors-tools-session-complete = نشست از قبل کامل شده است

# Configuration import and reload
errors-config-reload-failed = بارگذاری مجدد پیکربندی ناموفق بود
errors-config-reset-failed = بازنشانی پیکربندی ناموفق بود
errors-config-disk-parse-failed = تجزیه پیکربندی دیسک ناموفق بود
errors-config-disk-read-failed = خواندن پیکربندی دیسک ناموفق بود
errors-config-update-failed = به‌روزرسانی پیکربندی ناموفق بود
errors-config-import-not-object = پیکربندی باید یک شیء JSON باشد
errors-config-import-no-sections = بخش معتبری برای درون‌ریزی پیدا نشد
errors-config-import-validation-failed = اعتبارسنجی پیکربندی ناموفق بود. هیچ تغییری اعمال نشد.
errors-config-import-commit-failed = اعمال تغییرات پیکربندی ناموفق بود
errors-config-import-failed = درون‌ریزی پیکربندی ناموفق بود

# Filtering
errors-filtering-analytics-failed = دریافت تحلیل‌ها ناموفق بود
errors-filtering-refresh-failed = بازسازی اسنپ‌شات فیلترینگ ناموفق بود
errors-filtering-rejection-stats-failed = دریافت آمار ردشدن‌ها ناموفق بود
errors-filtering-rejected-tokens-failed = دریافت توکن‌های ردشده ناموفق بود
errors-filtering-csv-header-failed = نوشتن سرستون CSV ناموفق بود
errors-filtering-csv-record-failed = نوشتن رکورد CSV ناموفق بود
errors-filtering-csv-finalize-failed = نهایی‌سازی CSV ناموفق بود
errors-filtering-export-response-failed = ساخت پاسخ ناموفق بود

# OHLCV
errors-ohlcv-fetch-failed = دریافت داده OHLCV ناموفق بود
errors-ohlcv-pools-failed = دریافت استخرها ناموفق بود
errors-ohlcv-gaps-failed = دریافت شکاف‌ها ناموفق بود
errors-ohlcv-refresh-failed = تازه‌سازی ناموفق بود
errors-ohlcv-monitor-start-failed = شروع پایش ناموفق بود
errors-ohlcv-monitor-stop-failed = توقف پایش ناموفق بود
errors-ohlcv-activity-failed = ثبت فعالیت ناموفق بود
errors-ohlcv-list-failed = دریافت فهرست توکن‌های OHLCV ناموفق بود
errors-ohlcv-delete-failed = حذف داده توکن ناموفق بود
errors-ohlcv-clear-failed = پاک کردن کش OHLCV ناموفق بود
errors-ohlcv-cleanup-failed = پاکسازی توکن‌های غیرفعال ناموفق بود

# Trader and manual trading
errors-trade-already-running = معامله‌گر از قبل در حال اجراست
errors-trade-already-stopped = معامله‌گر از قبل متوقف است
errors-trade-config-update-failed = به‌روزرسانی پیکربندی معامله‌گر ناموفق بود
errors-trade-trader-unavailable = پیش از استفاده از معامله‌گر خودکار، راه‌اندازی کیف پول و RPC را کامل کنید
errors-trade-force-stop-active = توقف اضطراری فعال است؛ ابتدا آن را لغو کنید
errors-trade-template-not-found = قالب معامله‌گری با نام { $template } وجود ندارد
errors-trade-manual-force-stopped = تا زمانی که توقف اضطراری فعال است، معامله دستی غیرفعال است
errors-trade-core-services-not-ready = سرویس‌های اصلی برای معامله آماده نیستند: { $pending }
errors-trade-mint-invalid = آدرس مینت توکن { $mint } نامعتبر است
errors-trade-blacklisted = توکن { $mint } در فهرست سیاه است
errors-trade-slippage-invalid = اسلیپیج { $slippage }% باید در بازه ⁨(0, { $maximum }]⁩ باشد
errors-trade-percentage-invalid = درصد فروش { $percentage } باید در بازه ⁨(0, 100]⁩ باشد
errors-trade-record-failed = ثبت معامله دستی ممکن نشد
errors-trade-task-cancelled = معامله دستی پیش از پاسخ متوقف شد چون برنامه در حال بسته شدن است؛ نتیجه آن در پوزیشن ثبت است
errors-trade-no-open-position = پوزیشن بازی برای توکن { $mint } وجود ندارد
errors-trade-size-invalid = اندازه معامله { $amount } { -sol } نامعتبر است
errors-trade-management-invalid = مدیریت پوزیشن { $management } نامعتبر است
errors-trade-strategy-evaluation-failed = ارزیابی استراتژی برای توکن { $mint } ناموفق بود
errors-trade-token-data-missing = داده توکن { $mint } در دسترس نیست
errors-trade-endpoints-unhealthy = اندپوینت سالمی در دسترس نیست
errors-trade-dependency-failed = وابستگی { $dependency } ناموفق بود
errors-trade-storage-failed = درخواست معامله تکمیل نشد
errors-trade-manual-failed = معامله دستی ناموفق بود
# The reason is the trader's own wording for a refused trade, shown exactly as produced.
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = هیچ مسیر سواپی نتوانست تراکنشی به‌اندازهٔ کافی کوچک برای ارسال بسازد
    .hint = بهترین مسیر به حساب‌های بیشتری از ظرفیت یک تراکنش نیاز داشت، بنابراین چیزی ارسال نشد و هزینه‌ای هم نشد. چند لحظه دیگر دوباره تلاش کنید تا مسیر دیگری پیدا شود، یا روتر سواپ دیگری را فعال کنید.
errors-trade-wallet-not-configured = کیف پول پیکربندی نشده است
errors-trade-amount-sol-invalid = amount_sol برای خرید الزامی و باید مثبت باشد
errors-trade-no-tokens-in-wallet = توکنی برای این پوزیشن در کیف پول پیدا نشد. موجودی توکن 0 است؛ پوزیشن را نمی‌توان با سواپ بست.
errors-trade-percentage-range = درصد باید در بازه ⁨(0, 100]⁩ باشد
errors-trade-amount-tokens-invalid = amount_tokens باید مثبت باشد
errors-trade-sell-amount-zero = مقدار محاسبه‌شده برای فروش صفر است

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = مسیریابی سواپ هنوز آماده نیست
    .hint = سرویس سواپ هنوز در حال شروع است. منتظر آماده شدن سرویس‌ها بمانید و دوباره تلاش کنید.
errors-trade-quote-no-routers-enabled = هیچ ارائه‌دهنده سواپی فعال نیست
    .hint = دست‌کم یک روتر سواپ را در تنظیمات معامله‌گر فعال کنید و دوباره تلاش کنید.
errors-trade-quote-not-tradable = این توکن اکنون قابل معامله نیست
    .hint = نقدینگی یا مسیر سواپی در دسترس نیست. ممکن است توکن هنوز عرضه نشده، رها شده یا استخر نداشته باشد. بعداً دوباره تلاش کنید یا توکن دیگری انتخاب کنید.
errors-trade-quote-no-route = مسیر سواپی در دسترس نیست
    .hint = هیچ ارائه‌دهنده‌ای نتوانست این معامله را با مقدار درخواستی مسیریابی کند. مقدار کمتری را امتحان کنید یا کمی بعد دوباره تلاش کنید.
errors-trade-quote-rate-limited = ارائه‌دهندگان سواپ ما را محدود کرده‌اند
    .hint = ارائه‌دهندگان سواپ درخواست‌ها را کند کرده‌اند. چند ثانیه صبر کنید و دوباره تلاش کنید.
errors-trade-quote-timeout = مهلت درخواست قیمت پیشنهادی تمام شد
    .hint = ارائه‌دهندگان سواپ به‌موقع پاسخ ندادند. اتصال خود را بررسی کنید و دوباره تلاش کنید.
errors-trade-quote-router-rejected = قیمت پیشنهادی پذیرفته نشد
    .hint = یکی از ارائه‌دهندگان قیمتی برگرداند که از بررسی‌های ایمنی ما نگذشت و کنار گذاشته شد. برای دریافت قیمت تازه دوباره تلاش کنید.
errors-trade-quote-not-offered-exact-out = هیچ روتر سواپ فعالی برای مقدار خروجی دقیق قیمت نمی‌دهد
    .hint = روترهای سواپ فعال، معامله را فقط بر اساس مبلغ پرداختی قیمت‌گذاری می‌کنند. مبلغ پرداختی را وارد کنید یا روتر سواپ دیگری را فعال کنید.
errors-trade-quote-not-offered-unsupported-venue = هیچ روتر سواپ فعالی در استخر این توکن معامله نمی‌کند
    .hint = این توکن در صرافی‌ای معامله می‌شود که روترهای سواپ فعال هنوز از آن پشتیبانی نمی‌کنند. روتر سواپ دیگری را فعال کنید و دوباره تلاش کنید.
errors-trade-quote-unavailable = دریافت قیمت پیشنهادی ممکن نشد
    .hint = ارائه‌دهندگان سواپ نتوانستند این معامله را قیمت‌گذاری کنند. کمی بعد دوباره تلاش کنید.

# Positions
errors-positions-not-found = پوزیشن پیدا نشد
errors-positions-already-closed = پوزیشن از قبل بسته شده است
errors-positions-force-close-failed = بستن اجباری پوزیشن ناموفق بود
errors-positions-already-archived = پوزیشن از قبل بایگانی شده است
errors-positions-unverified-entry-archive = خرید این پوزیشن هنوز تأیید نشده است. پس از تأیید آن را بایگانی کنید.
errors-positions-not-archived = پوزیشن بایگانی نشده است
errors-positions-archive-failed = بایگانی پوزیشن ناموفق بود
errors-positions-unarchive-failed = خروج پوزیشن از بایگانی ناموفق بود
errors-positions-management-invalid = مدیریت متعلق به کپی به پوزیشنی با منشأ کپی نیاز دارد
errors-positions-management-failed = به‌روزرسانی مدیریت پوزیشن ناموفق بود
errors-positions-delete-failed = حذف پوزیشن ناموفق بود
errors-positions-bulk-delete-failed = حذف پوزیشن‌های بایگانی‌شده ناموفق بود
errors-positions-detail-failed = بارگیری جزئیات پوزیشن ناموفق بود
errors-positions-resolve-failed = تعیین وضعیت پوزیشن ناموفق بود
errors-positions-wrapped-sol-activity = Wrapped SOL فعالیت توکنی ندارد

# Tokens
errors-tokens-database-unavailable = پایگاه‌داده توکن در دسترس نیست
errors-tokens-blacklist-failed = افزودن توکن به فهرست سیاه ناموفق بود
errors-tokens-blacklist-internal = خطای داخلی هنگام افزودن به فهرست سیاه
errors-tokens-unblacklist-failed = حذف از فهرست سیاه ناموفق بود
errors-tokens-unblacklist-internal = خطای داخلی هنگام حذف از فهرست سیاه
errors-tokens-blacklist-status-failed = بررسی وضعیت فهرست سیاه ناموفق بود
errors-tokens-blacklist-status-internal = خطای داخلی هنگام بررسی وضعیت فهرست سیاه
errors-tokens-favorites-fetch-failed = دریافت علاقه‌مندی‌ها ناموفق بود
errors-tokens-favorite-add-failed = افزودن علاقه‌مندی ناموفق بود
errors-tokens-favorite-remove-failed = حذف علاقه‌مندی ناموفق بود
errors-tokens-favorite-update-failed = به‌روزرسانی علاقه‌مندی ناموفق بود
errors-tokens-detail-not-found = توکن در پایگاه‌داده یا منابع خارجی پیدا نشد
errors-tokens-fetch-failed = دریافت توکن ناموفق بود
errors-tokens-refresh-all-failed = همه منابع داده ناموفق بودند
errors-tokens-refresh-failed = تازه‌سازی توکن ناموفق بود
errors-tokens-search-query-required = عبارت جست‌وجوی 'q' الزامی است
errors-tokens-search-failed = جست‌وجوی توکن ناموفق بود

# Actions and services
errors-actions-not-found = اقدام { $id } پیدا نشد
errors-services-not-found = سرویس '{ $name }' پیدا نشد
