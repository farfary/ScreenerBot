# Assistant page shell and chat widget. Model output (chat replies, tool
# results, decision reasoning) is data and is never part of this catalog.

# Provider names shown by the Assistant page. Ids are the `Provider::as_str`
# values in src/apis/llm/mod.rs.
assistant-provider-openai = { -openai }
assistant-provider-anthropic = { -anthropic }
assistant-provider-groq = { -groq }
assistant-provider-deepseek = { -deepseek }
assistant-provider-gemini = { -google-gemini }
assistant-provider-ollama = { -ollama }
assistant-provider-together = { -together-ai }
assistant-provider-openrouter = { -openrouter }
assistant-provider-mistral = { -mistral-ai }
assistant-provider-select = انتخاب ارائه‌دهنده...

# Verdicts recorded for a model evaluation. `allow` is the alias the overview
# feed may carry; the rest are the ids written by src/llm_analysis/engine.rs.
assistant-decision-allow = مجاز
assistant-decision-pass = تأیید
assistant-decision-reject = رد
assistant-decision-buy = خرید
assistant-decision-sell = فروش
assistant-decision-hold = نگهداری

# Risk levels of `RiskLevel` in src/llm_analysis/types.rs.
assistant-risk-low = کم
assistant-risk-medium = متوسط
assistant-risk-high = زیاد
assistant-risk-critical = بحرانی

# assistant.html: page shell.
assistant-overview-title = نمای کلی دستیار
assistant-overview-description = وضعیت زنده LLM و تحلیل، معیارهای عملکرد و آخرین ارزیابی‌های مدل.
assistant-features-title = قابلیت‌های مدل
assistant-metric-total-evaluations = کل ارزیابی‌ها
assistant-metric-cache-hit-rate = نرخ برخورد کش
assistant-metric-avg-latency = میانگین تأخیر
assistant-metric-active-providers = ارائه‌دهندگان فعال
assistant-decisions-title = تصمیم‌های اخیر
assistant-decisions-subtitle = آخرین تصمیم‌های تحلیل مدل

assistant-providers-title = ارائه‌دهندگان LLM
assistant-providers-description = ارائه‌دهندگان مشترک LLM را که تحلیل و دستیار از آن‌ها استفاده می‌کنند پیکربندی کنید

assistant-master-title = تنظیمات اصلی
assistant-master-description = کلید اصلی مشترک LLM و انتخاب ارائه‌دهنده
assistant-setting-enabled-description = فعال‌سازی تحلیل هوشمند توکن با ارائه‌دهندگان LLM
assistant-setting-default-provider-label = ارائه‌دهنده پیش‌فرض
assistant-setting-default-provider-description = تنها ارائه‌دهنده‌ای که تحلیل، دستیار و وظایف زمان‌بندی‌شده از آن استفاده می‌کنند

assistant-filtering-title = فیلترینگ
assistant-filtering-description = فیلتر توکن با دستیار پیش از تصمیم‌های ورود
assistant-setting-filtering-enabled-label = فیلتر توکن با دستیار
assistant-setting-filtering-enabled-description = ارزیابی توکن‌ها با دستیار پیش از اجازه معاملات ورود
assistant-setting-min-confidence-label = حداقل اطمینان
assistant-setting-min-confidence-description = توکن‌ها برای عبور باید دست‌کم این سطح اطمینان را کسب کنند (0 تا 100)
assistant-setting-fallback-pass-label = عبور در صورت خطا
assistant-setting-fallback-pass-description = در صورت ناموفق بودن یا پایان مهلت ارزیابی دستیار، ورود به توکن مجاز باشد

assistant-trading-title = معاملات
assistant-trading-description = تحلیل دستیار در طول عملیات معاملاتی
assistant-setting-entry-analysis-label = تحلیل ورود
assistant-setting-entry-analysis-description = اجرای تحلیل دستیار پیش از باز کردن پوزیشن
assistant-setting-exit-analysis-label = تحلیل خروج
assistant-setting-exit-analysis-description = اجرای تحلیل دستیار پیش از بستن پوزیشن
assistant-setting-trailing-stop-label = تحلیل حد ضرر متحرک
assistant-setting-trailing-stop-description = اجرای تحلیل دستیار هنگام فعال شدن حد ضرر متحرک

assistant-blacklist-title = فهرست سیاه خودکار
assistant-blacklist-description = مسدودسازی خودکار توکن‌ها بر اساس امتیاز دستیار
assistant-setting-auto-blacklist-label = فهرست سیاه خودکار
assistant-setting-auto-blacklist-description = افزودن خودکار توکن‌هایی با امتیاز بسیار پایین دستیار به فهرست سیاه
assistant-setting-blacklist-threshold-label = آستانه فهرست سیاه
assistant-setting-blacklist-threshold-description = توکن‌هایی که امتیازشان پایین‌تر از این سطح باشد خودکار به فهرست سیاه اضافه می‌شوند (0 تا 100)

assistant-performance-title = عملکرد
assistant-performance-description = تنظیمات کش و هم‌زمانی
assistant-setting-cache-ttl-label = TTL کش
assistant-setting-cache-ttl-description = مدت نگهداری نتایج دستیار در کش (60 تا 3600 ثانیه)
assistant-setting-max-evaluations-label = حداکثر ارزیابی
assistant-setting-max-evaluations-description = حداکثر تعداد ارزیابی‌های هم‌زمان دستیار (1 تا 20)

assistant-cache-title = مدیریت کش
assistant-cache-description = مشاهده و پاک کردن ارزیابی‌های ذخیره‌شده دستیار
assistant-cache-size-label = اندازه کش:
assistant-cache-memory-label = مصرف حافظه:
assistant-cache-clear = پاک کردن کش

assistant-testing-title = محیط آزمایش دستیار
assistant-testing-subtitle = تحلیل دستیار را روی هر توکن آزمایش کنید
assistant-testing-mint-label = آدرس مینت
assistant-testing-mint-input =
    .placeholder = آدرس مینت توکن سولانا را وارد کنید...
assistant-testing-priority-label = اولویت
assistant-testing-priority-low = کم
assistant-testing-priority-medium = متوسط
assistant-testing-priority-high = زیاد
assistant-testing-evaluate = ارزیابی
assistant-testing-results = نتایج

assistant-instructions-title = دستورالعمل‌های سفارشی
assistant-instructions-description = پرامپت‌های سفارشی اضافه کنید تا در ارزیابی‌های دستیار درج شوند
assistant-instructions-new = دستورالعمل جدید
assistant-instructions-category-all = همه دسته‌ها
assistant-instructions-category-filtering = فیلترینگ
assistant-instructions-category-trading = معاملات
assistant-instructions-category-analysis = تحلیل
assistant-instructions-category-general = عمومی
assistant-instructions-status-all = همه وضعیت‌ها
assistant-instructions-status-active = فقط فعال
assistant-instructions-status-inactive = فقط غیرفعال
assistant-instructions-search =
    .placeholder = جست‌وجوی دستورالعمل‌ها...
assistant-instructions-empty = هنوز دستورالعمل سفارشی وجود ندارد
assistant-instructions-empty-add = افزودن اولین دستورالعمل
assistant-templates-title = قالب‌های دستورالعمل
assistant-templates-description = قالب‌های آماده دستورالعمل که می‌توانید اضافه کنید

assistant-automation-title = خودکارسازی
assistant-automation-description = وظایف دستیار را برای اجرای خودکار در فواصل مشخص یا زمان‌های معین زمان‌بندی کنید
assistant-automation-new =
    .aria-label = ایجاد وظیفه خودکار جدید
assistant-automation-new-label = وظیفه جدید
assistant-automation-stat-total = کل وظایف
assistant-automation-stat-active = فعال
assistant-automation-stat-runs = کل اجراها
assistant-automation-stat-success-rate = نرخ موفقیت
assistant-automation-empty = هنوز وظیفه زمان‌بندی‌شده‌ای وجود ندارد
assistant-automation-empty-subtitle = برای شروع، اولین وظیفه خودکار دستیار را ایجاد کنید
assistant-automation-empty-add = ایجاد اولین وظیفه
    .aria-label = ایجاد اولین وظیفه خودکار
assistant-automation-runs-title = اجراهای اخیر

assistant-history-title = تاریخچه تصمیم‌ها
assistant-history-subtitle = ارزیابی‌های اخیر دستیار

# assistant.js: page tabs, overview, settings and history.
assistant-tab-chat = گفتگو
assistant-tab-overview = نمای کلی
assistant-tab-providers = ارائه‌دهندگان
assistant-tab-instructions = دستورالعمل‌ها
assistant-tab-automation = خودکارسازی
assistant-tab-history = تاریخچه
assistant-tab-testing = آزمایش
assistant-tab-settings = تنظیمات
assistant-toggle-on = فعال
assistant-toggle-off = غیرفعال
assistant-status-active = دستیار فعال است
assistant-status-disabled = دستیار غیرفعال است
assistant-status-load-failed = بارگیری وضعیت قابلیت‌های مدل ناموفق بود
assistant-toggle-enabled-title = دستیار فعال شد
assistant-toggle-enabled-message = قابلیت‌های مبتنی بر مدل اکنون فعال هستند
assistant-toggle-disabled-title = دستیار غیرفعال شد
assistant-toggle-disabled-message = قابلیت‌های مبتنی بر مدل غیرفعال هستند
assistant-toggle-failed = به‌روزرسانی وضعیت قابلیت‌های مدل ناموفق بود
assistant-decisions-empty = تصمیم اخیری وجود ندارد
assistant-decision-latency =
    .title = تأخیر
assistant-decision-confidence =
    .title = اطمینان
assistant-config-load-failed = بارگیری پیکربندی تحلیل ناموفق بود
assistant-cache-clear-message = آیا مطمئنید که می‌خواهید کش تحلیل را پاک کنید؟ همه تصمیم‌های ذخیره‌شده مدل حذف می‌شوند.
assistant-cache-cleared-title = کش پاک شد
assistant-cache-cleared-message = کش تحلیل خالی است
assistant-cache-clear-failed = پاک کردن کش ناموفق بود
assistant-config-saved-title = ذخیره شد
assistant-config-saved-message = پیکربندی با موفقیت ذخیره شد
assistant-config-save-failed = ذخیره پیکربندی ناموفق بود
assistant-history-load-failed = بارگیری تاریخچه ناموفق بود
assistant-history-empty = هنوز درخواست تحلیل LLM ثبت نشده
assistant-history-column-token = توکن
assistant-history-column-decision = تصمیم
assistant-history-column-confidence = اطمینان
assistant-history-column-risk = ریسک
assistant-history-column-reasoning = استدلال
assistant-history-column-model = مدل
assistant-history-column-latency = تأخیر
assistant-history-column-when = زمان
assistant-history-previous = قبلی
assistant-history-page = صفحه { $page } از { $total }
assistant-history-cached = ذخیره‌شده در کش

# chat_widget.js: sessions sidebar and header.
assistant-chat-sessions-title = گفتگوها
assistant-chat-sidebar-new =
    .title = گفتگوی جدید
    .aria-label = ایجاد گفتگوی جدید
assistant-chat-search =
    .placeholder = جست‌وجوی گفتگوها...
    .aria-label = جست‌وجوی گفتگوها
assistant-chat-history-close =
    .aria-label = بستن تاریخچه گفتگو
assistant-chat-history-open =
    .title = تاریخچه گفتگو
    .aria-label = باز کردن تاریخچه گفتگو
assistant-chat-header-new =
    .title = گفتگوی جدید
    .aria-label = شروع گفتگوی جدید
assistant-chat-delete =
    .title = حذف
    .aria-label = حذف گفتگو
assistant-chat-close =
    .title = بستن
    .aria-label = بستن دستیار
assistant-chat-title-new = گفتگوی جدید
assistant-chat-sessions-empty = هنوز گفتگویی وجود ندارد
assistant-chat-sessions-empty-search = گفتگوی مطابقی پیدا نشد
assistant-chat-sessions-new = گفتگوی جدید
assistant-chat-group-today = امروز
assistant-chat-group-yesterday = دیروز
assistant-chat-group-week = 7 روز گذشته
assistant-chat-group-older = قدیمی‌تر

# chat_widget.js: empty state and quick prompts. The prompt texts are sent as the
# user's message.
assistant-chat-empty-kicker = دستیار
assistant-chat-empty-title = امروز چطور می‌توانم کمکتان کنم؟
assistant-chat-empty-subtitle = پورتفوی خود را بررسی کنید، یک توکن را بکاوید یا فعالیت‌های اخیر معاملاتی را بهتر بشناسید.
assistant-chat-prompt-positions-label = بررسی پوزیشن‌های باز
assistant-chat-prompt-positions-text = موجودی فعلی کیف پول و پوزیشن‌های باز من چیست؟
assistant-chat-prompt-token-label = تحلیل یک توکن
assistant-chat-prompt-token-text = امنیت و ریسک‌های این توکن را تحلیل کن:
assistant-chat-prompt-activity-label = توضیح فعالیت‌های اخیر
assistant-chat-prompt-activity-text = فعالیت‌های اخیر معاملاتی من و نتایج مهم آن‌ها را توضیح بده

# chat_widget.js: input area.
assistant-chat-input =
    .placeholder = پیام به دستیار...
    .aria-label = ورودی پیام
assistant-chat-hint-key-enter = Enter
assistant-chat-hint-key-shift = Shift
assistant-chat-hint-send = برای ارسال
assistant-chat-hint-newline = برای خط جدید
assistant-chat-send =
    .title = ارسال پیام
    .aria-label = ارسال پیام
assistant-chat-send-empty =
    .aria-label = برای ارسال، پیامی بنویسید
assistant-chat-stop =
    .title = توقف پاسخ (Esc)
    .aria-label = توقف پاسخ
assistant-chat-cancelled-title = لغو شد
assistant-chat-cancelled-message = درخواست لغو شد
assistant-chat-message-too-long-title = پیام بیش از حد طولانی است
assistant-chat-message-too-long-message = لطفاً پیام خود را به کمتر از { $limit } کاراکتر کوتاه کنید
assistant-chat-start-failed = شروع گفتگو ناموفق بود
assistant-chat-send-failed = دستیار نتوانست پاسخ را کامل کند. پیام شما برای ارسال مجدد آماده است.

# chat_widget.js: messages and tool calls.
assistant-chat-role-user = شما
assistant-chat-role-assistant = دستیار
assistant-chat-message-actions =
    .aria-label = اقدام‌های پیام
assistant-chat-message-copy =
    .title = کپی
    .aria-label = کپی پیام
assistant-chat-message-regenerate =
    .title = تولید مجدد
    .aria-label = تولید مجدد پاسخ
assistant-chat-copied-message = پیام
assistant-chat-copy-failed = کپی پیام ناموفق بود
assistant-chat-regenerate-none = پیامی برای تولید مجدد وجود ندارد
assistant-chat-regenerate-reload = پیش از تولید مجدد این پاسخ، گفتگو را دوباره بارگیری کنید
assistant-chat-regenerate-done-title = تولید مجدد شد
assistant-chat-regenerate-done-message = پاسخ با موفقیت دوباره تولید شد
assistant-chat-regenerate-failed = تولید مجدد ناموفق بود
assistant-chat-regenerate-failed-toast = تولید مجدد پاسخ ناموفق بود

# Status of a tool call; ids are the `ToolCallStatus` variants in
# src/assistant/chat/types.rs, lowercased and hyphenated.
assistant-chat-tool-status-executed = اجراشده
assistant-chat-tool-status-failed = ناموفق
assistant-chat-tool-status-denied = ردشده
assistant-chat-tool-status-pending-confirmation = در انتظار تأیید
assistant-chat-tool-status-pending = در انتظار
assistant-chat-tool-section-input = ورودی:
assistant-chat-tool-section-output = خروجی:
assistant-chat-tool-section-error = خطا:

# chat_widget.js: progress while a response is generated.
assistant-chat-typing =
    .aria-label = دستیار در حال فکر کردن است
assistant-chat-progress-preparing = در حال آماده‌سازی پاسخ
assistant-chat-progress-planning = در حال برنامه‌ریزی پاسخ
assistant-chat-progress-reviewing = در حال بررسی نتایج ابزارها
assistant-chat-progress-using-tools = در حال استفاده از ابزارها
assistant-chat-progress-running = در حال اجرا
assistant-chat-progress-failed = ناموفق
assistant-chat-progress-complete = کامل شد
assistant-chat-error-unknown = خطای ناشناخته
assistant-chat-stream-http = خطای API: { $status }
assistant-chat-stream-unavailable = جریان پیشرفت دستیار در دسترس نیست
assistant-chat-stream-failed = درخواست دستیار ناموفق بود
assistant-chat-stream-incomplete = پاسخ دستیار پیش از تکمیل پایان یافت

# chat_widget.js: tool approval and sessions.
assistant-chat-tool-review = بررسی ورودی
assistant-chat-tool-deny = رد
assistant-chat-tool-allow = اجازه
assistant-chat-tool-unknown = ابزار ناشناخته
assistant-chat-tool-default-description = اجرای این ابزار به تأیید شما نیاز دارد.
assistant-chat-tool-executed = ابزار اجرا شد
assistant-chat-tool-cancelled = اجرای ابزار لغو شد
assistant-chat-tool-confirm-failed = تأیید اجرای ابزار ناموفق بود
assistant-chat-sessions-load-failed = بارگیری گفتگوها ناموفق بود
assistant-chat-delete-title = حذف گفتگو
assistant-chat-delete-message = آیا مطمئنید که می‌خواهید این گفتگو را حذف کنید؟ این عمل قابل بازگشت نیست.
assistant-chat-delete-done = گفتگو حذف شد
assistant-chat-delete-failed = حذف گفتگو ناموفق بود

# Agent tool labels. Ids are the tool names registered by
# `create_tool_registry` in src/agent_control/tools/mod.rs, with hyphens.
assistant-tool-analyze-token = تحلیل توکن
assistant-tool-get-market-data = دریافت داده بازار
assistant-tool-check-security = بررسی امنیت
assistant-tool-get-positions = دریافت پوزیشن‌ها
assistant-tool-get-position = دریافت پوزیشن
assistant-tool-get-balance = دریافت موجودی
assistant-tool-get-pnl = دریافت سود و زیان
assistant-tool-buy-token = خرید توکن
assistant-tool-add-to-position = افزودن به پوزیشن
assistant-tool-sell-token = فروش توکن
assistant-tool-close-position = بستن پوزیشن
assistant-tool-get-config = دریافت پیکربندی
assistant-tool-describe-config = شرح پیکربندی
assistant-tool-update-config = به‌روزرسانی پیکربندی
assistant-tool-get-status = دریافت وضعیت
assistant-tool-get-events = دریافت رویدادها
assistant-tool-force-stop = توقف اجباری
assistant-tool-clear-force-stop = لغو توقف اجباری
assistant-tool-get-trader-status = دریافت وضعیت معامله‌گر
assistant-tool-get-trader-stats = دریافت آمار معامله‌گر
assistant-tool-set-trader-enabled = فعال/غیرفعال‌سازی معامله‌گر
assistant-tool-set-trader-monitor = تنظیم پایش معامله‌گر
assistant-tool-manage-loss-limit = مدیریت محدودیت ضرر
assistant-tool-list-trader-templates = فهرست قالب‌های معامله‌گر
assistant-tool-apply-trader-template = اعمال قالب معامله‌گر
assistant-tool-get-copy-trading-overview = دریافت نمای کلی کپی‌تریدینگ
assistant-tool-get-copy-task = دریافت وظیفه کپی
assistant-tool-get-copy-activity = دریافت فعالیت کپی
assistant-tool-create-copy-task = ایجاد وظیفه کپی
assistant-tool-update-copy-task = به‌روزرسانی وظیفه کپی
assistant-tool-delete-copy-task = حذف وظیفه کپی
assistant-tool-set-copy-task-mode = تنظیم حالت وظیفه کپی
assistant-tool-get-copy-insights = دریافت بینش‌های کپی
assistant-tool-get-copy-wallet-profile = دریافت پروفایل کیف پول کپی
assistant-tool-clone-copy-task = کپی‌برداری از وظیفه کپی
assistant-tool-reset-copy-paper-book = بازنشانی دفتر آزمایشی کپی
assistant-tool-close-copy-paper-holding = بستن دارایی آزمایشی کپی

# Built-in instruction templates (src/llm_analysis/database.rs
# `get_builtin_templates`). The template body is model input and stays in Rust.
assistant-template-liquidity-guard-name = نگهبان نقدینگی
assistant-template-liquidity-guard-description = توکن‌های دارای نقدینگی کم را رد می‌کند، چون ریسک اسلیپیج را بالا می‌برد و خروج را دشوار می‌کند.
assistant-template-holder-distribution-name = بررسی توزیع هولدرها
assistant-template-holder-distribution-description = توکن‌هایی را که بزرگ‌ترین هولدرهایشان سهم زیادی از عرضه را کنترل می‌کنند علامت‌گذاری می‌کند.
assistant-template-honeypot-detection-name = تشخیص هانی‌پات
assistant-template-honeypot-detection-description = توکن‌هایی با اختیار فریز یا مینت فعال یا محدودیت‌های غیرعادی انتقال را رد می‌کند.
assistant-template-momentum-filter-name = فیلتر مومنتوم
assistant-template-momentum-filter-description = توکن‌هایی با مومنتوم مثبت قیمت را که با افزایش حجم تأیید شده باشد ترجیح می‌دهد.
assistant-template-new-token-caution-name = احتیاط در توکن‌های جدید
assistant-template-new-token-caution-description = برای توکن‌هایی که کمتر از یک روز از عمرشان می‌گذرد اطمینان بالاتری می‌طلبد.
assistant-template-whale-activity-name = پایش فعالیت نهنگ‌ها
assistant-template-whale-activity-description = جابه‌جایی‌های بزرگ هولدرها و فعالیت غیرعادی دیپلویر یا کیف پول‌های اولیه را زیر نظر می‌گیرد.

# Template tags; ids are the `tags` of a built-in template.
assistant-template-tag-activity = فعالیت
assistant-template-tag-age = عمر
assistant-template-tag-authority = اختیار
assistant-template-tag-caution = احتیاط
assistant-template-tag-distribution = توزیع
assistant-template-tag-holders = هولدرها
assistant-template-tag-honeypot = هانی‌پات
assistant-template-tag-large-holders = هولدرهای بزرگ
assistant-template-tag-liquidity = نقدینگی
assistant-template-tag-momentum = مومنتوم
assistant-template-tag-new-tokens = توکن‌های جدید
assistant-template-tag-price-action = رفتار قیمت
assistant-template-tag-risk-management = مدیریت ریسک
assistant-template-tag-rug-risk = ریسک راگ‌پول
assistant-template-tag-safety = ایمنی
assistant-template-tag-security = امنیت
assistant-template-tag-volume = حجم
assistant-template-tag-whales = نهنگ‌ها

# Shared by the Assistant tab dialogs.
assistant-modal-close =
    .aria-label = بستن

# providers_tab.js: provider list.
assistant-providers-load-failed = بارگیری ارائه‌دهندگان ناموفق بود
assistant-providers-select-default =
    .title = تنظیم به‌عنوان پیش‌فرض
assistant-providers-use-default =
    .aria-label = استفاده از { $name } به‌عنوان ارائه‌دهنده پیش‌فرض
assistant-providers-model-none = پیکربندی نشده
assistant-providers-status-ready = آماده
assistant-providers-status-not-set-up = راه‌اندازی نشده
assistant-providers-status-default = پیش‌فرض
assistant-providers-test = آزمایش
assistant-providers-configure = پیکربندی
assistant-providers-default-set-title = ارائه‌دهنده پیش‌فرض تنظیم شد
assistant-providers-default-set-message = { $name } اکنون ارائه‌دهنده پیش‌فرض است
assistant-providers-default-set-failed = تنظیم ارائه‌دهنده پیش‌فرض ناموفق بود
assistant-providers-testing-title = آزمایش ارائه‌دهنده
assistant-providers-testing-message = در حال آزمایش { $name }...
assistant-providers-test-http = HTTP { $status }
assistant-providers-test-success-title = اتصال موفق بود
assistant-providers-test-success-message = { $name } به‌درستی کار می‌کند
assistant-providers-test-failed = آزمایش ناموفق بود

# providers_tab.js: configuration dialog.
assistant-providers-config-title = پیکربندی { $name }
assistant-providers-api-key = کلید API
assistant-providers-key-saved = کلید ذخیره شده
assistant-providers-key-missing = کلیدی تنظیم نشده
assistant-providers-api-key-update =
    .placeholder = برای به‌روزرسانی، کلید جدید را وارد کنید...
assistant-providers-api-key-enter =
    .placeholder = کلید API را وارد کنید...
assistant-providers-key-toggle =
    .title = نمایش/پنهان کردن
assistant-providers-key-help-saved = برای حفظ کلید فعلی خالی بگذارید یا برای به‌روزرسانی کلید جدید وارد کنید
assistant-providers-key-help-new = کلید API شما به‌صورت ایمن ذخیره می‌شود و هرگز به اشتراک گذاشته نمی‌شود
assistant-providers-model = مدل
assistant-providers-model-input =
    .placeholder = مثلاً gpt-4, claude-3-opus...
assistant-providers-model-help = مدلی که برای درخواست‌های تحلیل دستیار استفاده می‌شود
assistant-providers-enable = فعال‌سازی این ارائه‌دهنده
assistant-providers-enable-help = در صورت فعال بودن، این ارائه‌دهنده برای تحلیل دستیار در دسترس خواهد بود
assistant-providers-connection-test = آزمایش اتصال
assistant-providers-test-connection = آزمایش اتصال
assistant-providers-testing = در حال آزمایش...
assistant-providers-save = ذخیره پیکربندی
assistant-providers-saving = در حال ذخیره...
assistant-providers-missing-key-title = کلید API وجود ندارد
assistant-providers-missing-key-test = لطفاً ابتدا کلید API را وارد کنید
assistant-providers-missing-key-enable = برای فعال‌سازی این ارائه‌دهنده، کلید API را وارد کنید
assistant-providers-missing-model-title = مدل وجود ندارد
assistant-providers-missing-model-message = لطفاً نام مدل را وارد کنید
assistant-providers-test-save-failed = ذخیره پیکربندی برای آزمایش ناموفق بود
assistant-providers-test-connected = اتصال موفق بود!
assistant-providers-detail-model = مدل:
assistant-providers-detail-none = ندارد
assistant-providers-detail-latency = تأخیر:
assistant-providers-detail-tokens = توکن‌ها:
assistant-providers-saved-title = ارائه‌دهنده ذخیره شد
assistant-providers-saved-message = پیکربندی { $name } ذخیره شد
assistant-providers-save-failed = ذخیره پیکربندی ارائه‌دهنده ناموفق بود

# instructions_tab.js: list, templates and dialogs.
assistant-instructions-load-failed = بارگیری دستورالعمل‌ها ناموفق بود
assistant-instructions-priority = اولویت: { $position }
assistant-instructions-actions =
    .aria-label = اقدام‌های دستورالعمل
assistant-instructions-hint-filtering = دستورالعمل‌هایی برای تصمیم‌های فیلتر توکن - به تحلیل LLM کمک می‌کند تشخیص دهد کدام توکن‌ها را کنار بگذارد
assistant-instructions-hint-trading = دستورالعمل‌هایی برای تحلیل ورود/خروج - تصمیم‌های معاملاتی امتیازدهی‌شده توسط مدل را هدایت می‌کند
assistant-instructions-hint-analysis = رهنمودهای عمومی تحلیل بازار برای تصمیم‌های امتیازدهی‌شده توسط مدل
assistant-instructions-hint-general = دستورالعمل‌های دیگر برای رفتارهای مبتنی بر مدل
assistant-instructions-char-count =
    { $count ->
        [one] { $amount } کاراکتر
       *[other] { $amount } کاراکتر
    }
assistant-instructions-reordered-title = ترتیب تغییر کرد
assistant-instructions-reordered-message = ترتیب دستورالعمل‌ها با موفقیت تغییر کرد
assistant-instructions-reorder-failed = تغییر ترتیب دستورالعمل‌ها ناموفق بود
assistant-templates-empty = قالبی موجود نیست
assistant-templates-preview-title = پیش‌نمایش قالب: { $name }
assistant-templates-preview-content = محتوا:
assistant-templates-customize-add = سفارشی‌سازی و افزودن
assistant-templates-customize-title = سفارشی‌سازی قالب
assistant-instructions-field-name = نام
assistant-instructions-field-category = دسته
assistant-instructions-field-content = محتوا
assistant-instructions-name-input =
    .placeholder = مثلاً نگهبان نقدینگی
assistant-instructions-content-input =
    .placeholder = دستورالعمل خود را وارد کنید...
assistant-instructions-create = ایجاد
assistant-instructions-create-title = ایجاد دستورالعمل
assistant-instructions-missing-title = فیلدهای ناقص
assistant-instructions-missing-message = نام و محتوا الزامی است
assistant-instructions-created-title = ایجاد شد
assistant-instructions-created-message = دستورالعمل با موفقیت ایجاد شد
assistant-instructions-created-from-template = دستورالعمل از قالب ایجاد شد: { $name }
assistant-instructions-create-failed = ایجاد دستورالعمل ناموفق بود
assistant-instructions-create-from-template-failed = ایجاد دستورالعمل از قالب ناموفق بود
assistant-instructions-toggle-failed = تغییر وضعیت دستورالعمل ناموفق بود
assistant-instructions-edit-title = ویرایش دستورالعمل
assistant-instructions-preview = پیش‌نمایش
assistant-instructions-save-changes = ذخیره تغییرات
assistant-instructions-untitled = بدون عنوان
assistant-instructions-load-item-failed = بارگیری داده‌های دستورالعمل ناموفق بود
assistant-instructions-updated-title = به‌روزرسانی شد
assistant-instructions-updated-message = دستورالعمل با موفقیت به‌روزرسانی شد
assistant-instructions-update-failed = به‌روزرسانی دستورالعمل ناموفق بود
assistant-instructions-delete-title = حذف دستورالعمل
assistant-instructions-delete-message = آیا مطمئنید که می‌خواهید این دستورالعمل را حذف کنید؟
assistant-instructions-deleted-title = حذف شد
assistant-instructions-deleted-message = دستورالعمل با موفقیت حذف شد
assistant-instructions-delete-failed = حذف دستورالعمل ناموفق بود
assistant-instructions-copy-name = { $name } (کپی)
assistant-instructions-duplicated-title = تکثیر شد
assistant-instructions-duplicated-message = دستورالعمل با موفقیت تکثیر شد
assistant-instructions-duplicate-failed = تکثیر دستورالعمل ناموفق بود

# automation_tab.js: task list, runs and dialogs. Schedule type, permission and
# run status ids are the `as_str` values of `ScheduleType`, `TaskToolPermissions`
# and `RunStatus` in src/assistant/scheduled/types.rs.
assistant-automation-schedule-type-interval = بازه‌ای
assistant-automation-schedule-type-daily = روزانه
assistant-automation-schedule-type-weekly = هفتگی
assistant-automation-permission-read-only = فقط خواندنی
assistant-automation-permission-full = دسترسی کامل
assistant-automation-permission-option-read-only = فقط خواندنی (ایمن)
assistant-automation-permission-option-full = دسترسی کامل (امکان معامله)
assistant-automation-run-status-running = در حال اجرا
assistant-automation-run-status-success = موفق
assistant-automation-run-status-failed = ناموفق
assistant-automation-run-status-timeout = پایان مهلت
assistant-automation-run-status-skipped = ردشده
assistant-automation-hint-interval = فاصله بر حسب ثانیه (مثلاً 300 = هر 5 دقیقه)
assistant-automation-hint-daily = زمان با قالب HH:MM به UTC (مثلاً 14:00)
assistant-automation-hint-weekly = روزها و زمان: mon,wed,fri:09:00
assistant-automation-schedule-every = هر { $span }
assistant-automation-schedule-daily = روزانه ساعت { $time } UTC
assistant-automation-schedule-weekly = { $days } ساعت { $time } UTC
assistant-automation-schedule-day-separator = { "، " }
assistant-automation-task-active = فعال
assistant-automation-task-paused = متوقف
assistant-automation-never = هرگز
assistant-automation-last-run = آخرین: { $when }
assistant-automation-next-run = بعدی: { $when }
assistant-automation-run-now =
    .title = اجرا در همین لحظه
    .aria-label = اجرا در همین لحظه
assistant-automation-actions =
    .aria-label = اقدام‌های خودکارسازی
assistant-automation-runs-count =
    { $count ->
        [one] { $amount } اجرا
       *[other] { $amount } اجرا
    }
assistant-automation-runs-empty = هنوز اجرایی وجود ندارد
assistant-automation-task-runs-empty = هنوز اجرایی برای این وظیفه وجود ندارد
assistant-automation-task-fallback = وظیفه #{ $id }
assistant-automation-task-generic = وظیفه
assistant-automation-create-title = ایجاد وظیفه خودکار
assistant-automation-edit-title = ویرایش وظیفه
assistant-automation-field-name = نام وظیفه
assistant-automation-name-input =
    .placeholder = مثلاً پایش پورتفوی
assistant-automation-field-instruction = دستورالعمل
assistant-automation-instruction-input =
    .placeholder = دستیار چه کاری انجام دهد؟ مثلاً پوزیشن‌های باز را از نظر نشانه‌های برگشت بررسی کن و یافته‌ها را گزارش بده.
assistant-automation-field-schedule-type = نوع زمان‌بندی
assistant-automation-field-schedule-value = مقدار زمان‌بندی
assistant-automation-field-permissions = مجوزهای ابزار
assistant-automation-field-timeout = مهلت (ثانیه)
assistant-automation-notify-telegram = اعلان از طریق { -telegram }
assistant-automation-notify-success = اعلان در صورت موفقیت
assistant-automation-notify-failure = اعلان در صورت شکست
assistant-automation-create-task = ایجاد وظیفه
assistant-automation-save-changes = ذخیره تغییرات
assistant-automation-validation-title = اعتبارسنجی
assistant-automation-validation-required = لطفاً همه فیلدهای الزامی را پر کنید
assistant-automation-validation-interval = فاصله باید دست‌کم 60 ثانیه باشد
assistant-automation-validation-daily = زمان‌بندی روزانه باید با قالب HH:MM باشد
assistant-automation-validation-weekly = زمان‌بندی هفتگی باید با این قالب باشد: mon,wed,fri:09:00
assistant-automation-created = وظیفه ایجاد شد
assistant-automation-create-failed = ایجاد وظیفه ناموفق بود
assistant-automation-updated = وظیفه به‌روزرسانی شد
assistant-automation-update-failed = به‌روزرسانی وظیفه ناموفق بود
assistant-automation-toggle-failed = تغییر وضعیت وظیفه ناموفق بود
assistant-automation-triggered = وظیفه اجرا شد
assistant-automation-trigger-failed = اجرای وظیفه ناموفق بود
assistant-automation-delete-title = حذف وظیفه
assistant-automation-delete-message = آیا مطمئنید که می‌خواهید این وظیفه خودکار را حذف کنید؟ این عمل قابل بازگشت نیست.
assistant-automation-deleted = وظیفه حذف شد
assistant-automation-delete-failed = حذف وظیفه ناموفق بود
assistant-automation-view-runs = مشاهده اجراها
assistant-automation-runs-load-failed = بارگیری اجراها ناموفق بود
assistant-automation-runs-history-title = تاریخچه اجرا — { $task }
assistant-automation-run-load-failed = بارگیری جزئیات اجرا ناموفق بود
assistant-automation-run-details-title = جزئیات اجرا
assistant-automation-run-task = وظیفه
assistant-automation-run-status = وضعیت
assistant-automation-run-started = شروع
assistant-automation-run-duration = مدت
assistant-automation-run-provider = ارائه‌دهنده
assistant-automation-run-tokens = توکن‌ها
assistant-automation-run-tools-title = فراخوانی ابزارها ({ $amount })
assistant-automation-run-response = پاسخ دستیار
