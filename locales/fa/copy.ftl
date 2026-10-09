copy-skip-not-buy-swap = فعالیت کیف پول خرید نبود
copy-skip-task-disabled = وظیفه متوقف است
copy-skip-mode-transition-required = حالت اجرا باید جداگانه تغییر کند
copy-skip-live-confirmation-required = اجرای زنده به تأیید نیاز دارد
copy-skip-unsupported-sizing-mode = حالت تعیین اندازه هنوز پشتیبانی نمی‌شود
copy-skip-self-copy = این کیف پول یکی از کیف پول‌های خود شماست
copy-skip-target-below-minimum = معامله کیف پول کمتر از حداقل است
copy-skip-target-above-maximum = معامله کیف پول بیشتر از حداکثر است
copy-skip-already-bought = این توکن قبلاً خریداری شده است (فقط یک‌بار خرید)
copy-skip-blacklisted = توکن توسط کنترل‌های ریسک مسدود شده است
copy-skip-filter-required = توکن از فیلترینگ عبور نکرد
copy-skip-budget-exhausted = بودجه وظیفه تمام شده است
copy-skip-token-cap-reached = به سقف هر توکن رسیده است
copy-skip-below-minimum-size = اندازه کپی بسیار کوچک است
copy-skip-invalid-sizing = اندازه‌گذاری وظیفه نامعتبر است
copy-skip-invalid-slippage = اسلیپیج وظیفه نامعتبر است
copy-skip-invalid-exit-policy = قوانین خروج وظیفه نامعتبر است
copy-skip-invalid-price = قیمت بازار قابل استفاده‌ای وجود ندارد
copy-skip-not-sell-swap = فعالیت کیف پول فروش نبود
copy-skip-exit-mode-disabled = فروش کیف پول نادیده گرفته شد: وظیفه طبق قوانین خودش می‌فروشد
copy-skip-force-stopped = معاملات به‌طور اجباری متوقف شده است
copy-skip-copy-position-not-found = پوزیشنی متعلق به این وظیفه نیست
copy-skip-position-user-only = پوزیشن را خودتان مدیریت می‌کنید
copy-skip-position-management-mismatch = پوزیشن دیگر از فروش‌های کپی پیروی نمی‌کند
copy-skip-latency-kill-switch = توقف خودکار: معاملات با تأخیر زیاد شناسایی شدند
copy-skip-claim-reconciled-abandoned = ارسال زنده ناتمام بدون تلاش مجدد بسته شد
copy-skip-stale-observation = پس از توقف دوباره پخش شد، برای کپی بسیار قدیمی است
copy-skip-unknown-observation-time = معامله بازپخش‌شده زمان بلاک ندارد
copy-skip-entry-blocked = ورود مسدود شد

copy-entry-block-force-stopped = معاملات به‌طور اجباری متوقف شده است
copy-entry-block-loss-limit = محدودیت ضرر ورودهای جدید را مسدود می‌کند
copy-entry-block-connectivity = سرویس‌های موردنیاز در دسترس نیستند
copy-entry-block-position-limit = به سقف پوزیشن‌های باز رسیده است
copy-entry-block-already-open = یک پوزیشن از قبل باز است
copy-entry-block-reentry-cooldown = زمان انتظار ورود مجدد توکن
copy-entry-block-open-cooldown = زمان انتظار سراسری ورود
copy-entry-block-entry-reserved = ورود دیگری در حال پردازش است
copy-entry-block-blacklisted = توکن توسط کنترل‌های ریسک مسدود شده است
copy-entry-block-check-failed = یک بررسی ایمنی کامل نشد

copy-pause-user = توسط شما متوقف شد
copy-pause-latency-kill-switch = توقف خودکار: معاملات به‌طور میانگین { $average } ثانیه دیر رسیدند (حد { $threshold } ثانیه)
copy-pause-watch-detached = توقف خودکار: کیف پول دیگر پایش نمی‌شود
copy-pause-watch-budget-exceeded = متوقف شد: این کیف پول پیش از رسیدن به وضعیت به‌روز، به سقف { $limit } امضا در هر بررسی پایش رسید
copy-pause-helius-unavailable = متوقف شد: بررسی کیف پول از طریق { -helius } ناموفق بود
copy-pause-watch-processing-failed = متوقف شد: فعالیت کیف پول قابل پردازش نبود
copy-pause-unspecified = متوقف

copy-pause-short-user = توسط شما
copy-pause-short-latency-kill-switch = بسیار کند
copy-pause-short-watch-detached = پایش قطع شد
copy-pause-short-watch-budget-exceeded = سقف پایش
copy-pause-short-helius-unavailable = ارائه‌دهنده پایش
copy-pause-short-watch-processing-failed = پردازش پایش
copy-state-paused = متوقف
copy-state-paused-reason = متوقف · { $reason }

copy-readiness-history = تاریخچه آزمایشی
copy-readiness-history-met =
    { $count ->
        [one] { $count } دور آزمایشی بسته، { $needed } لازم است
       *[other] { $count } دور آزمایشی بسته، { $needed } لازم است
    }
copy-readiness-history-short = { $count } از { $needed } دور آزمایشی بسته
copy-readiness-profit = سودآور در حالت آزمایشی
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } تحقق‌یافته در { $count } دور، { $wins } برد
       *[other] { $realized } { -sol } تحقق‌یافته در { $count } دور، { $wins } برد
    }
copy-readiness-latency = معاملات به‌موقع شناسایی شدند
copy-readiness-latency-detail = رسیدن p95: { $p95 } ثانیه، حد: { $limit } ثانیه
copy-readiness-latency-none = هنوز نمونه رسیدنی وجود ندارد
copy-readiness-priced = همه دارایی‌ها قیمت‌دار هستند
copy-readiness-priced-ok = همه دارایی‌های آزمایشی باز قیمت استخر دارند
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } دارایی باز بدون قیمت استخر
       *[other] { $count } دارایی باز بدون قیمت استخر
    }
copy-readiness-runtime = اجرای زنده در دسترس است
copy-readiness-runtime-ok = راه‌اندازی و دروازه‌های ایمنی کپی زنده را مجاز می‌کنند

copy-live-block-setup-incomplete = ابتدا راه‌اندازی کیف پول و RPC را کامل کنید
copy-live-block-force-stop = توقف اضطراری فعال است
copy-live-block-copy-trading-disabled = پردازش کپی به‌طور سراسری متوقف است
copy-live-block-unavailable = اجرای زنده در دسترس نیست

## Task state, mode and exit labels

copy-state-system-paused = متوقف‌شده سراسری
copy-state-force-stopped = توقف اجباری
copy-state-entries-blocked = ورودها مسدود
copy-state-running-live = در حال اجرا
copy-state-running-paper = در حال اجرا
copy-mode-paper = آزمایشی
copy-mode-live = زنده
copy-exit-mode-buy-only = قوانین خروج من
copy-exit-mode-mirror = تقلید فروش‌های کیف پول
copy-exit-mode-hybrid = فروش‌های کیف پول و قوانین من
copy-exit-target-sell = کیف پول فروخت
copy-exit-stop-loss = حد ضرر
copy-exit-trailing-stop = حد ضرر متحرک
copy-exit-take-profit = حد سود
copy-exit-time-override = قانون زمانی
copy-exit-manual = بسته‌شده دستی

## Shared wording

copy-request-failed = درخواست ناموفق بود
copy-keep-paused = متوقف بماند
copy-paused-suffix = · متوقف
copy-mode-paused = { $mode } · متوقف
copy-task-ref = «{ $name }» ({ $mode })
copy-metric-realized-pnl = سود و زیان تحقق‌یافته
copy-metric-unrealized-pnl = سود و زیان تحقق‌نیافته
copy-metric-win-rate = نرخ برد
copy-metric-budget-spent = بودجه مصرف‌شده
copy-metric-median-arrival = میانه رسیدن
copy-metric-open-holdings = دارایی‌های باز
copy-record-won-lost = { $won } برد · { $lost } باخت
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = اجراها
copy-kind-exits = خروج‌ها
copy-kind-skips = موارد نادیده
copy-kind-errors = خطاها
copy-field-per-trade-cap = سقف هر معامله
copy-field-per-token-cap = سقف هر توکن
copy-field-total-budget = کل بودجه
copy-field-slippage = اسلیپیج
copy-rules-wallet-sells-only = فقط فروش‌های کیف پول
copy-filter-copy-setting-required = تنظیم کپی‌تریدینگ (الزامی)
copy-filter-copy-setting-not-required = تنظیم کپی‌تریدینگ (غیرالزامی)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } دور بسته
       *[other] { $count } دور بسته
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } دارایی باز
       *[other] { $count } دارایی باز
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } دارایی قیمت‌دار · { $unpriced } بدون قیمت
       *[other] { $priced } دارایی قیمت‌دار · { $unpriced } بدون قیمت
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } دارایی بدون قیمت
       *[other] { $count } دارایی بدون قیمت
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = همه
copy-range-label =
    .aria-label = بازه تاریخ

## Page strip

copy-page-title = کپی‌تریدینگ
copy-page-beta = بتا
copy-strip-loading = در حال بارگذاری
copy-strip-unavailable = در دسترس نیست
copy-strip-setup-required = راه‌اندازی لازم است · کپی ترید به کیف پول و RPC نیاز دارد
copy-strip-pause-all = توقف همه
copy-strip-resume = ازسرگیری پردازش
copy-strip-settings = تنظیمات
copy-strip-add-wallet = افزودن کیف پول
copy-strip-paused-globally = متوقف‌شده سراسری · کپی جدیدی انجام نمی‌شود، خروج‌ها همچنان اجرا می‌شوند
copy-strip-force-stopped = توقف اجباری · چیزی کپی نمی‌شود
copy-strip-loss-limit = محدودیت ضرر · ورودهای جدید مسدود است، خروج‌ها همچنان اجرا می‌شوند
copy-strip-idle-paused =
    { $count ->
        [one] بیکار · { $count } وظیفه متوقف
       *[other] بیکار · { $count } وظیفه متوقف
    }
copy-strip-idle-empty = بیکار · هنوز وظیفه‌ای نیست
copy-strip-processing = در حال پردازش · { $paper } آزمایشی
copy-strip-processing-live = در حال پردازش · { $live } زنده · { $paper } آزمایشی
copy-figures-label =
    .aria-label = جمع کل کپی‌تریدینگ
copy-figure-marked-at-pool = ارزش‌گذاری‌شده با قیمت استخر
copy-figure-across-tasks = در همه وظایف
copy-figure-budget-lifetime = مصرف کل وظایف فعال
copy-figure-budget-none = وظیفه فعالی وجود ندارد
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } معامله
       *[other] { $count } معامله
    }
copy-figure-arrival-none = نمونه‌ای از وظایف فعال وجود ندارد

## Page frame

copy-load-failed = بارگذاری کپی‌تریدینگ ممکن نشد: { $error }
copy-resume-all-title = ازسرگیری پردازش کپی
copy-resume-all-message =
    { $count ->
        [one] { $count } وظیفه زنده وقتی کیف پولش دوباره معامله کند، سواپ واقعی ارسال می‌کند.
       *[other] { $count } وظیفه زنده وقتی کیف پول‌هایشان دوباره معامله کنند، سواپ واقعی ارسال می‌کنند.
    }
copy-toast-resumed-all = پردازش کپی از سر گرفته شد
copy-toast-paused-all = همه پردازش‌های کپی متوقف شد
copy-toast-global-failed = تغییر پردازش کپی ممکن نشد

## Onboarding

copy-onboarding-title = کیف پول‌های مورد اعتماد را کپی کنید، ابتدا در حالت آزمایشی محک بزنید
copy-onboarding-body = هر وظیفه در حالت آزمایشی شروع می‌شود: معاملات هدف با قیمت استخر، اسلیپیج و کارمزدهای شما شبیه‌سازی می‌شوند و قوانین خروج شما روی دفتر آزمایشی اجرا می‌شوند. وقتی نتایج آزمایشی یک کیف پول شایسته شد، حالت زنده را برای آن فعال کنید.
copy-onboarding-add = افزودن اولین کیف پول
copy-setup-gate-title = کپی ترید به کیف پول نیاز دارد
copy-onboarding-observe = مشاهده
copy-onboarding-observe-detail = سواپ‌های کیف پول را بدون خرج‌کردن { -sol } شناسایی کنید.
copy-onboarding-evaluate = ارزیابی
copy-onboarding-evaluate-detail = سود و زیان آزمایشی، نرخ برد، موارد نادیده، سرعت شناسایی و اسلیپیج را بخوانید.
copy-onboarding-arm = فعال‌سازی
copy-onboarding-arm-detail = از بررسی‌های آمادگی عبور کنید، سپس سواپ واقعی را فعال کنید.

## Wallet list

copy-list-label =
    .aria-label = کیف پول‌های کپی‌شده
copy-list-title = کیف پول‌ها
copy-list-compare = مقایسه
copy-list-sort-label = مرتب‌سازی کیف پول‌ها
copy-list-count = { $active } فعال · { $total } کل
copy-sort-pnl = سود و زیان
copy-sort-state = وضعیت
copy-sort-name = نام
copy-compare-label =
    .aria-label = مقایسه کیف پول‌ها

## Dialog chrome

copy-dialog-close =
    .aria-label = بستن
copy-editor-title-add = افزودن کیف پول
copy-editor-sub-add = وظایف جدید در حالت آزمایشی شروع می‌شوند
copy-arm-title = فعال‌سازی کپی زنده
copy-arm-sub = سواپ‌های واقعی از کیف پول شما
copy-arm-keep-paper = ماندن در آزمایشی
copy-arm-confirm = فعال‌سازی زنده
copy-profile-title = پروفایل کیف پول
copy-profile-sub = آنچه این ربات از کیف پول دیده است

## Settings dialog

copy-settings-title = تنظیمات کپی‌تریدینگ
copy-settings-subtitle = سیاست سراسری برای همه وظایف
copy-settings-filter-warning = با تنظیمات پیش‌فرض فیلترینگ، تقریباً همه توکن‌ها رد می‌شوند و چیزی کپی نمی‌شود. مگر اینکه فیلترهای شما توکن‌های معامله‌شده توسط کیف پول‌هایتان را عبور دهند، آن را خاموش نگه دارید.
copy-settings-unit-seconds = ثانیه
copy-settings-unit-trades = معامله
copy-settings-unit-tasks = وظیفه
copy-settings-unit-rounds = دور
copy-settings-save = ذخیره تنظیمات
copy-settings-load-failed = بارگذاری تنظیمات کپی ممکن نشد
copy-settings-saved = تنظیمات کپی‌تریدینگ ذخیره شد

## Workspace

copy-tab-overview = نمای کلی
copy-tab-holdings = دارایی‌ها
copy-tab-activity = فعالیت
copy-tab-rules = قوانین
copy-tab-execution = اجرا
copy-tabs-label = نماهای وظیفه
copy-workspace-select = برای باز کردن فضای کاری، یک کیف پول انتخاب کنید.
copy-workspace-loading = در حال بارگذاری وظیفه…
copy-workspace-load-failed = بارگذاری این وظیفه ممکن نشد: { $error }

copy-state-detail-paper = در حال اجرا در حالت آزمایشی · معاملات شبیه‌سازی می‌شوند، چیزی خرج نمی‌شود
copy-state-detail-live = در حال اجرای زنده · معاملات کیف پول با سواپ واقعی کپی می‌شوند
copy-state-detail-system-paused = در انتظار · پردازش کپی به‌طور سراسری متوقف است، خروج‌ها همچنان اجرا می‌شوند
copy-state-detail-entries-blocked = ورودها با محدودیت ضرر مسدود شده‌اند · خروج‌ها همچنان اجرا می‌شوند
copy-state-detail-force-stopped = توقف اجباری · چیزی کپی نمی‌شود

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = ازسرگیری همان حد را نگه می‌دارد، پس تا وقتی معاملات دیر می‌رسند دوباره متوقف می‌شود. جریان RPC را بررسی کنید یا حد رسیدن را در تنظیمات افزایش دهید.
copy-paused-resume-detached = ازسرگیری، کیف پول را دوباره پایش می‌کند.
copy-paused-holdings-rules =
    { $count ->
        [one] قوانین خروج آن همچنان { $count } دارایی بازش را می‌بندند.
       *[other] قوانین خروج آن همچنان { $count } دارایی بازش را می‌بندند.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] فروش‌های کیف پول همچنان { $count } دارایی باز آن را می‌بندند.
       *[other] فروش‌های کیف پول همچنان { $count } دارایی باز آن را می‌بندند.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] فروش‌های کیف پول و قوانین خروج آن همچنان { $count } دارایی بازش را می‌بندند.
       *[other] فروش‌های کیف پول و قوانین خروج آن همچنان { $count } دارایی بازش را می‌بندند.
    }

copy-watch-state-catching-up = پایش کیف پول: در حال رسیدن به وضعیت به‌روز. بررسی این کیف پول از طریق { -helius }.
copy-watch-state-watching = پایش کیف پول: در حال پایش. بررسی این کیف پول از طریق { -helius }.
copy-watch-last-check = آخرین بررسی { $ago }.
copy-watch-recovery-active = پایش کیف پول فعال است
copy-watch-recovery-catching-up = پایش کیف پول در حال رسیدن به وضعیت به‌روز است
copy-watch-recovery-still-paused = وظیفه کپی همچنان متوقف است. هر زمان آماده بودید، کپی را ازسر بگیرید.
copy-watch-recovery-title = بازیابی پایش کیف پول
copy-watch-recovery-processing-failed = فعالیت کیف پول قابل پردازش نبود. پیشرفت ذخیره‌شده حفظ شده است. پس از رفع مشکل دوباره تلاش کنید.
copy-watch-recovery-provider-failed = بررسی‌های { -helius } ناموفق بود. پیشرفت ذخیره‌شده حفظ شده است. وقتی ارائه‌دهنده در دسترس بود دوباره تلاش کنید.
copy-watch-recovery-budget-intro = فعالیت این کیف پول بیشتر از چیزی است که پایش فعلی می‌تواند بررسی کند. نحوه ادامه را انتخاب کنید.
copy-watch-approve = تلاش برای رسیدن به وضعیت به‌روز با { -helius }
copy-watch-approve-help = از پیشرفت ذخیره‌شده ادامه می‌دهد. ممکن است اعتبار { -helius } بیشتری مصرف کند و همچنان عقب بماند.
copy-watch-approve-unavailable = رسیدن به وضعیت به‌روز با { -helius } در دسترس نیست. برای ادامه بدون نادیده‌گرفتن فعالیت‌های بررسی‌نشده، یک اندپوینت RPC فعال { -helius } پیکربندی کنید.
copy-watch-no-provider = برای این پایش، ارائه‌دهنده‌ای برای رسیدن به وضعیت به‌روز پشتیبانی نمی‌شود.
copy-watch-budget-label = امضاهای بررسی‌شده در هر بررسی
copy-watch-budget-hint = یا فعالیت‌های بررسی‌نشده را نادیده بگیرید و از همین لحظه ازسر بگیرید. از { $min } تا { $max } امضا در هر بررسی انتخاب کنید؛ سقف بالاتر ممکن است تماس‌های RPC بیشتری مصرف کند.
copy-watch-ack = می‌دانم فعالیت‌های ازدست‌رفته کپی نخواهند شد.
copy-watch-toast-range = بین { $min } تا { $max } امضا در هر بررسی و با گام‌های { $step } امضایی انتخاب کنید
copy-watch-toast-ack = تأیید کنید که امضاهای پس از آخرین بررسی کامل‌شده نادیده گرفته می‌شوند
copy-watch-resumed = پایش کیف پول از همین لحظه ازسر گرفته شد؛ وظیفه کپی همچنان متوقف است
copy-watch-resume-failed = ازسرگیری پایش کیف پول ممکن نشد
copy-watch-retry-started = تلاش مجدد پایش کیف پول از پیشرفت ذخیره‌شده شروع شد؛ وظیفه کپی همچنان متوقف است
copy-watch-retry-failed = تلاش مجدد پایش کیف پول ممکن نشد
copy-watch-approve-title = اجازه رسیدن به وضعیت به‌روز با { -helius } برای این کیف پول
copy-watch-approve-message = { -helius } می‌تواند تراکنش‌های موفق Solana را از پیشرفت ذخیره‌شده و بدون نادیده‌گرفتن بازه بررسی‌نشده بررسی کند. در حال حاضر برای هر 100 تراکنش کامل بازگشتی، 10 اعتبار (با رو به بالا گرد کردن) و حداقل 10 اعتبار برای هر درخواست کسر می‌کند. یک بررسی می‌تواند چند درخواست ایجاد کند؛ مصرف و قیمت‌گذاری ارائه‌دهنده ممکن است متفاوت باشد. کپی تا زمانی که آن را جداگانه ازسر نگیرید متوقف می‌ماند.
copy-watch-approve-confirm = اجازه برای این کیف پول
copy-watch-approved = پایش کیف پول از پیشرفت ذخیره‌شده شروع شد؛ وظیفه کپی همچنان متوقف است
copy-watch-restore-failed = بازیابی پایش کیف پول ممکن نشد

copy-action-pause = توقف
copy-action-resume = ازسرگیری
copy-action-resume-copy = ازسرگیری کپی
copy-action-resume-from-now = ازسرگیری از همین لحظه
copy-action-retry-watch = تلاش مجدد پایش کیف پول
copy-action-return-paper = بازگشت به آزمایشی
copy-action-edit-rules = ویرایش قوانین
copy-action-clone = کلون
copy-action-profile = پروفایل کیف پول
copy-resume-live-title = ازسرگیری کپی زنده
copy-resume-live-message = «{ $name }» وقتی این کیف پول دوباره معامله کند، سواپ واقعی از کیف پول شما ارسال می‌کند.
copy-resume-live-confirm = ازسرگیری زنده
copy-task-resumed = وظیفه ازسر گرفته شد
copy-task-paused = وظیفه متوقف شد
copy-task-state-failed = تغییر وضعیت وظیفه ممکن نشد
copy-return-paper-message = کپی‌های جدید «{ $name }» دوباره شبیه‌سازی می‌شوند و { -sol } خرج نمی‌شود.
copy-return-paper-cancel = ماندن در زنده
copy-task-returned-paper = وظیفه به حالت آزمایشی بازگشت
copy-mode-change-failed = تغییر حالت اجرا ممکن نشد
copy-delete-title = حذف وظیفه کپی
copy-delete-message = «{ $name }» حذف شود؟ تصمیم‌ها و نتایج آزمایشی آن پاک می‌شوند و کیف پول دیگر برای این وظیفه پایش نمی‌شود.
copy-delete-confirm = حذف وظیفه
copy-delete-cancel = نگه داشتن وظیفه
copy-task-deleted = وظیفه کپی حذف شد
copy-task-delete-failed = حذف وظیفه کپی ممکن نشد

## Overview tab

copy-overview-results = نتایج
copy-analytics-load-failed = بارگذاری تحلیل‌ها ممکن نشد: { $error }
copy-analytics-loading = در حال بارگذاری تحلیل‌ها…
copy-exit-bucket =
    { $count ->
        [one] { $count } فروش · { $pnl }
       *[other] { $count } فروش · { $pnl }
    }
copy-overview-average-win = میانگین برد
copy-overview-average-loss = میانگین باخت { $amount }
copy-overview-profit-factor = ضریب سود
copy-overview-profit-factor-note = مجموع بردها ÷ مجموع باخت‌ها
copy-overview-average-hold = میانگین مدت نگهداری
copy-overview-average-hold-note = از ورود تا خروج
copy-overview-best-round = بهترین دور
copy-overview-worst-round = بدترین { $amount }
copy-overview-curve-title = سود و زیان تجمعی
copy-overview-exits-title = فروش‌ها بر اساس خروج
copy-overview-skips-title = چرا معاملات نادیده گرفته شدند
copy-book-title-live = دفتر زنده
copy-book-title-paper = دفتر آزمایشی
copy-book-all-time = کل زمان
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] خرید
       *[other] خرید
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] خروج طبق قوانین شما
       *[other] خروج طبق قوانین شما
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] فروش کیف پول
       *[other] فروش کیف پول
    }
copy-book-manual-closes = <strong>{ $count }</strong> بسته‌شده دستی
copy-book-skipped = <strong>{ $count }</strong> نادیده‌گرفته‌شده
copy-book-failed = <strong>{ $count }</strong> ناموفق
copy-book-closed = { $count } بسته‌شده
copy-book-budget-note = مصرف { $mode } از { $total } · { $remaining } باقی‌مانده
copy-check-passed = قبول
copy-check-not-passed = مردود
copy-readiness-title = پیش از رفتن به حالت زنده
copy-readiness-live-note = این وظیفه زنده معامله می‌کند. آن را از سربرگ بالا به حالت آزمایشی برگردانید.
copy-readiness-all-pass = همه بررسی‌ها قبول شده‌اند.
copy-readiness-needs-review = فعال‌سازی نیازمند بازبینی صریح مواردی است که آماده نیستند.
copy-readiness-arm = بازبینی و فعال‌سازی زنده

## Rules tab and review

copy-rules-title = قوانین در حال اجرا
copy-rules-size-ratio = { $pct } از معامله کیف پول
copy-rules-size-fixed = { $amount } برای هر کپی
copy-rules-target-any = هر اندازه
copy-rules-target-min = دست‌کم { $amount }
copy-rules-target-max = حداکثر { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = بازنویسی وظیفه · معامله‌گر { $value }
copy-rules-source-default = پیش‌فرض معامله‌گر
copy-rules-not-used = استفاده نمی‌شود: فروش‌های کیف پول تصمیم می‌گیرند
copy-rules-col-rule = قانون
copy-rules-col-applies = اعمال
copy-rules-col-source = منبع
copy-rules-budget-note = { $spent } مصرف‌شده در { $mode } · { $remaining } باقی‌مانده
copy-rules-token-copies =
    { $count ->
        [one] حدود { $count } کپی کامل از یک توکن
       *[other] حدود { $count } کپی کامل از یک توکن
    }
copy-rules-sizing = اندازه‌گذاری
copy-rules-copy-size = اندازه کپی
copy-rules-entry-filters = فیلترهای ورود
copy-rules-target-size = اندازه معامله کیف پول
copy-rules-repeat-buys = خریدهای تکراری
copy-rules-repeat-first-only = فقط اولین خرید هر توکن
copy-rules-repeat-every = هر خرید، تا سقف هر توکن
copy-rules-filter-pass = عبور از فیلترینگ
copy-rules-filter-required = الزامی
copy-rules-filter-not-required = غیرالزامی
copy-rules-filter-task-override = بازنویسی وظیفه
copy-rules-exits = خروج‌ها
copy-rules-exits-inactive = دارایی‌ها فقط وقتی کیف پول می‌فروشد فروخته می‌شوند؛ قوانین زیر در این حالت اجرا نمی‌شوند.

## Exit rules

copy-rule-status = وضعیت
copy-rule-on = روشن
copy-rule-off = خاموش
copy-rule-unit-seconds = ثانیه
copy-rule-unit-minutes = دقیقه
copy-rule-stop-loss-threshold = فروش در ضرر
copy-rule-stop-loss-min-hold = نه پیش از نگهداری
copy-rule-no-minimum = بدون حداقل
copy-rule-partial-exits = خروج‌های جزئی
copy-rule-partial-allowed = مجاز
copy-rule-partial-full-only = فقط خروج کامل
copy-rule-partial-size = اندازه خروج جزئی
copy-rule-trailing-activation = فعال‌سازی در سود
copy-rule-trailing-distance = فروش در فاصله زیر اوج
copy-rule-take-profit-target = فروش در سود
copy-rule-time-duration = بررسی پس از نگهداری
copy-rule-time-threshold = فروش وقتی سود و زیان برابر یا کمتر از
copy-preset-inherit = پیش‌فرض‌های معامله‌گر
copy-preset-conservative = محافظه‌کارانه
copy-preset-balanced = متعادل
copy-preset-aggressive = تهاجمی
copy-preset-custom = سفارشی
copy-validate-stop-loss = حد ضرر باید بیشتر از 0% و حداکثر 100% باشد.
copy-validate-partial-size = اندازه خروج جزئی باید بین 0% و 100% باشد.
copy-validate-min-hold = حداقل نگهداری باید عدد صحیح ثانیه باشد.
copy-validate-trailing-activation = فعال‌سازی حد ضرر متحرک باید بیشتر از 0% و حداکثر 100% باشد.
copy-validate-trailing-distance = فاصله حد ضرر متحرک باید بیشتر از 0% و حداکثر 100% باشد.
copy-validate-take-profit = حد سود باید بیشتر از 0% باشد.
copy-validate-time-duration = قانون زمانی به مدتی بیشتر از صفر نیاز دارد.
copy-validate-time-threshold = آستانه قانون زمانی یک ضرر است: از 0% یا عدد منفی استفاده کنید.
copy-warning-mirror = فقط فروش‌های کیف پول دارایی‌ها را می‌بندند: هیچ حد ضرری از آن‌ها محافظت نمی‌کند و توکنی که کیف پول هرگز نفروشد نگه داشته می‌شود.
copy-warning-no-rules = هیچ قانون خروجی روشن نیست و فروش‌های کیف پول نادیده گرفته می‌شوند: دارایی‌ها هرگز فروخته نمی‌شوند.
copy-warning-no-stop-loss = هیچ حد ضرری اعمال نمی‌شود: توکن در حال ریزش تا زمانی که قانون دیگری یا کیف پول بفروشد نگه داشته می‌شود.
copy-warning-stop-delay = حد ضرر پس از هر خرید { $hold } صبر می‌کند: توکنی که سریع‌تر سقوط کند خیلی پایین‌تر از { $threshold } بسته می‌شود.
copy-warning-take-profit-cost = حد سود در { $target } هزینه فروش ({ $slippage } اسلیپیج و { $fee } کارمزد سواپ) را پوشش نمی‌دهد، پس دورها را با ضرر می‌بندد.
copy-warning-trailing-distance = فاصله حد ضرر متحرک دست‌کم برابر سود فعال‌سازی آن است، پس یک حد ضرر متحرک فعال‌شده می‌تواند زیر قیمت ورود بفروشد.

## Execution tab

copy-execution-title = کیفیت اجرا
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = هر مقدار
copy-execution-limit-on =
    { $count ->
        [one] در صورت میانگین بیش از { $limit } در { $count } معامله متوقف می‌شود
       *[other] در صورت میانگین بیش از { $limit } در { $count } معامله متوقف می‌شود
    }
copy-execution-limit-off = کلید توقف خاموش
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } معامله در لحظه رخ‌دادن دیده شد
       *[other] { $count } معامله در لحظه رخ‌دادن دیده شد
    }
copy-execution-p95 = رسیدن p95
copy-execution-median-slippage = میانه اسلیپیج
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } اجرای اندازه‌گیری‌شده
       *[other] { $count } اجرای اندازه‌گیری‌شده
    }
copy-execution-worst-slippage = بدترین اسلیپیج
copy-execution-average-slippage = میانگین { $amount }
copy-execution-delay-title = تأخیر شناسایی
copy-execution-delay-note = زمان از بلاک کیف پول تا دیده‌شدن معامله توسط این ربات. بازپخش‌های پس از توقف حذف می‌شوند.
copy-execution-delay-limit = میله‌های فراتر از حد رسیدن { $limit } کهربایی هستند.
copy-execution-fastest = سریع‌ترین
copy-execution-average = میانگین
copy-execution-slowest = کندترین
copy-execution-fill-title = اجرا در مقایسه با کیف پول
copy-execution-fill-note = مثبت یعنی بدتر از کیف پول: در خرید بیشتر پرداخت شده یا در فروش تقلیدی کمتر دریافت شده. اجرای آزمایشی توکنی که قیمت استخر ندارد با قیمت معامله خود کیف پول محاسبه می‌شود، پس چیزی را نمی‌سنجد و کنار گذاشته می‌شود.
copy-execution-samples = نمونه‌ها
copy-execution-median = میانه
copy-execution-worst = بدترین
copy-execution-decisions = تصمیم‌ها در بازه

## Compare view

copy-compare-title = مقایسه کیف پول‌ها
copy-compare-back = بازگشت به کیف پول
copy-compare-load-failed = بارگذاری مقایسه ممکن نشد: { $error }
copy-compare-loading = در حال بارگذاری مقایسه…
copy-compare-empty = وظیفه‌ای برای مقایسه وجود ندارد.
copy-compare-empty-message = یک وظیفهٔ کپی اضافه کنید تا نتیجه‌اش را با بقیه مقایسه کنید.
copy-compare-curve-title = سود و زیان تحقق‌یافته تجمعی
copy-table-wallet = کیف پول
copy-table-mode = حالت
copy-table-rounds = دورها
copy-table-realized = تحقق‌یافته
copy-table-profit-factor = ضریب سود
copy-table-average-hold = میانگین نگهداری
copy-table-median-slippage = میانه اسلیپیج

## Charts

copy-chart-curve-label = سود و زیان تجمعی { $amount } { -sol }
copy-chart-compare-label = سود و زیان تجمعی به تفکیک وظیفه
copy-chart-empty-curve = هنوز دور بسته‌ای در این بازه نیست.
copy-chart-empty-bars = چیزی در این بازه ثبت نشده است.
copy-chart-empty-histogram = نمونه رسیدنی در این بازه نیست.
copy-chart-empty-compare = دور بسته‌ای برای مقایسه در این بازه نیست.
copy-chart-histogram-title = { $count } از { $total }

## Wallet profile

copy-profile-copy = کپی این کیف پول
copy-profile-copy-other = کپی با قوانین دیگر
copy-profile-loading = در حال بارگذاری پروفایل کیف پول…
copy-profile-watch-title = پایش
copy-profile-watched = پایش‌شده
copy-profile-watch-resume-hint = ازسرگیری یک وظیفه، پایش آن را دوباره شروع می‌کند
copy-profile-watch-add-hint = افزودن یک وظیفه، پایش را شروع می‌کند
copy-profile-stream = جریان
copy-profile-subscribed = مشترک شده
copy-profile-not-subscribed = مشترک نشده
copy-profile-sources =
    { $count ->
        [one] { $count } منبع
       *[other] { $count } منبع
    }
copy-profile-last-activity = آخرین فعالیت
copy-profile-last-error = آخرین خطا
copy-profile-own-wallet = این یکی از کیف پول‌های خود شماست؛ کپی آن رد می‌شود.
copy-profile-observed-title = معاملات مشاهده‌شده
copy-profile-observed-none = هنوز معامله‌ای از این کیف پول در این ربات نیست. یک وظیفه آزمایشی آن را بدون خرج‌کردن { -sol } مشاهده می‌کند.
copy-profile-swaps-seen = سواپ‌های دیده‌شده
copy-profile-swaps-seen-note = سواپ‌های متمایز کیف پول در وظایف شما
copy-profile-buys-sells = خرید / فروش
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = توکن‌های معامله‌شده
copy-profile-first-seen = اولین مشاهده
copy-profile-last-seen = آخرین مشاهده
copy-profile-tasks-title = وظایف شما روی این کیف پول
copy-table-task = وظیفه

## Arm live dialog

copy-arm-acks-left =
    { $count ->
        [one] { $count } تأییدیه باقی‌مانده برای علامت‌زدن
       *[other] { $count } تأییدیه باقی‌مانده برای علامت‌زدن
    }
copy-arm-readiness-title = آمادگی بر اساس دفتر آزمایشی
copy-arm-exposure-title = میزان مواجهه
copy-arm-per-copy = برای هر کپی
copy-arm-budget-left-value = { $left } از { $total } { -sol }
copy-arm-budget-left = بودجه زنده باقی‌مانده
copy-arm-budget-left-note = مصرف آزمایشی جداگانه شمرده می‌شود و از آن کم نمی‌کند
copy-arm-exits = خروج‌ها
copy-arm-stop-note = نه پیش از نگهداری { $hold }: سقوط سریع‌تر پایین‌تر بسته می‌شود
copy-arm-shared = این کیف پول را { $tasks } نیز کپی می‌کنند: هر وظیفه معاملات آن را با بودجه خودش کپی می‌کند.
copy-arm-unavailable = اجرای زنده اکنون در دسترس نیست؛ آخرین بررسی را ببینید.
copy-arm-ack-real-native = { -sol } واقعی: این وظیفه می‌تواند تا { $budget } { -sol } از کیف پول شما خرج کند، حداکثر { $trade } { -sol } برای هر کپی.
copy-arm-ack-fees = کپی‌های زنده کارمزد شبکه و اسلیپیج واقعی می‌پردازند؛ نتایج آزمایشی نتایج زنده را تضمین نمی‌کنند.
copy-arm-ack-unready = برخی بررسی‌های آمادگی قبول نشده‌اند. با این حال این وظیفه را فعال می‌کنم.
copy-arm-lead = «{ $name }» معاملات این کیف پول را با سواپ واقعی از کیف پول شما کپی می‌کند.
copy-arm-confirmation-missing = بارگذاری تأیید زنده ممکن نشد
copy-arm-armed = کپی زنده فعال شد
copy-arm-failed = فعال‌سازی کپی زنده ممکن نشد

## Holdings tab

copy-holdings-title = دارایی‌ها
copy-holdings-view-label = نمای دارایی‌ها
copy-holdings-view-open = باز ({ $count })
copy-holdings-view-closed = دورهای بسته ({ $count })
copy-holdings-reset = بازنشانی دفتر آزمایشی
copy-holdings-live-note = کپی‌های زنده پوزیشن‌های واقعی هستند.
copy-holdings-open-positions = پوزیشن‌های باز
copy-holdings-token-details = باز کردن جزئیات توکن
copy-holdings-opened = باز شده در { $time }
copy-holdings-no-pool-price = بدون قیمت استخر
copy-holdings-close = بستن
copy-holdings-write-off = حذف از دفتر
copy-holdings-activity = فعالیت
copy-holdings-no-exit-rule = بدون قانون خروج
copy-holdings-watch-stop = حد ضرر { $level }
copy-holdings-watch-stop-until = حد ضرر { $level } تا { $span } دیگر
copy-holdings-watch-take = حد سود { $level }
copy-holdings-watch-trail = حد ضرر متحرک { $level }
copy-holdings-watch-trail-arms = فعال‌سازی حد ضرر متحرک { $level }
copy-holdings-watch-time = زمان ≤ { $level }
copy-holdings-watch-time-until = زمان ≤ { $level } تا { $span } دیگر
copy-holdings-watch-wallet-sells = فروش‌های کیف پول
copy-holdings-empty = دارایی آزمایشی بازی وجود ندارد. خریدهای کپی‌شده از کیف پول اینجا ظاهر می‌شوند.
copy-holdings-col-token = توکن
copy-holdings-col-cost = هزینه
copy-holdings-col-entry = ورود
copy-holdings-col-mark = قیمت فعلی
copy-holdings-col-peak = اوج
copy-holdings-col-pnl = سود و زیان
copy-holdings-col-exit-rules = قوانین خروج
copy-holdings-col-held = مدت نگهداری
copy-holdings-col-actions = اقدامات
copy-holdings-col-invested = سرمایه‌گذاری‌شده
copy-holdings-col-proceeds = عواید
copy-holdings-col-exit = خروج
copy-holdings-col-closed = بسته‌شده
copy-holdings-price-note = قیمت‌ها به { -sol } برای هر توکن هستند. ورود شامل اسلیپیج و کارمزد خرید است؛ اوج و سطوح خروج نسبت به آن سنجیده می‌شوند، پس یک دارایی با اوجی پایین‌تر از ورود باز می‌شود. برای دیدن قیمت استخر، نشانگر را روی هر مورد نگه دارید.
copy-holdings-paused-rules = متوقف: کپی جدیدی انجام نمی‌شود. قوانین خروج شما همچنان این دارایی‌ها را می‌بندند.
copy-holdings-paused-mirror = متوقف: کپی جدیدی انجام نمی‌شود. فروش‌های کیف پول همچنان این دارایی‌ها را می‌بندند.
copy-holdings-paused-hybrid = متوقف: کپی جدیدی انجام نمی‌شود. فروش‌های کیف پول و قوانین خروج شما همچنان این دارایی‌ها را می‌بندند.
copy-holdings-closed-load-failed = بارگذاری دورهای بسته ممکن نشد: { $error }
copy-holdings-closed-loading = در حال بارگذاری دورهای بسته…
copy-holdings-closed-empty = هنوز دور بسته‌ای نیست.
copy-holdings-closed-latest = آخرین { $shown } دور از { $total }.
copy-holdings-close-title = بستن دارایی آزمایشی
copy-holdings-close-message = { $token } در دفتر آزمایشی با قیمت استخر ({ $price }) و اسلیپیج و کارمزدهای وظیفه فروخته می‌شود.
copy-holdings-close-confirm = بستن دارایی
copy-holdings-write-off-title = حذف دارایی آزمایشی از دفتر
copy-holdings-write-off-message = { $token } قیمت استخری برای فروش ندارد. حذف از دفتر آن را با عواید صفر می‌بندد و هزینه { $cost } آن را به‌عنوان ضرر ثبت می‌کند.
copy-holdings-keep = نگه داشتن
copy-holdings-written-off = { $token } از دفتر حذف شد
copy-holdings-closed = { $token } بسته شد
copy-holdings-written-off-detail = با عواید صفر بسته شد
copy-holdings-sold-at = فروخته شد در { $price }
copy-holdings-close-failed = بستن دارایی ممکن نشد
copy-holdings-reset-message = «{ $name }» را از نو شروع کنید: دارایی‌ها، مصرف، اجراها، خروج‌ها و موارد نادیده آزمایشی آن پاک می‌شوند. قوانین و کیف پول باقی می‌مانند.
copy-holdings-reset-cancel = نگه داشتن تاریخچه
copy-holdings-reset-done = دفتر آزمایشی بازنشانی شد
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } تصمیم پاک شد
       *[other] { $count } تصمیم پاک شد
    }
copy-holdings-reset-failed = بازنشانی دفتر آزمایشی ممکن نشد

## Activity tab

copy-activity-title = فعالیت
copy-activity-filter-label = فیلتر فعالیت
copy-filter-all = همه
copy-outcome-paper-filled = خرید آزمایشی
copy-outcome-live-submitted = خرید زنده ارسال شد
copy-outcome-live-confirmed = خرید زنده تأیید شد
copy-outcome-live-failed = خرید زنده ناموفق بود
copy-outcome-paper-sell-observed = فروش آزمایشی · کیف پول فروخت
copy-outcome-live-sell-submitted = فروش زنده ارسال شد
copy-outcome-live-sell-failed = فروش زنده ناموفق بود
copy-outcome-skipped = نادیده گرفته شد
copy-activity-decision = تصمیم
copy-activity-paper-exit = خروج آزمایشی · { $rule }
copy-activity-filled = { $input } با قیمت { $price } · کیف پول { $target } خرید
copy-activity-filled-slippage = { $input } با قیمت { $price } · کیف پول { $target } خرید · اسلیپیج { $slippage }
copy-activity-filled-unpriced = { $input } با قیمت { $price } · کیف پول { $target } خرید · با قیمت معامله کیف پول محاسبه شد، بدون قیمت استخر
copy-activity-live-sized = { $sized } · کیف پول { $target } خرید
copy-activity-sell-nothing = کیف پول { $amount } فروخت · چیزی برای فروش نگه‌داری نمی‌شد
copy-activity-written-off = با مقدار صفر از دفتر حذف شد: بدون قیمت استخر
copy-activity-sold = { $tokens } توکن به مبلغ { $proceeds } با قیمت { $price }
copy-activity-full-close = بستن کامل
copy-activity-partial-exit = خروج { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = حداقل { $amount }
copy-activity-skip-maximum = حداکثر { $value }
copy-activity-skip-stale = { $arrival } تأخیر، حد { $limit }
copy-activity-skip-latency = میانگین { $average }، حد { $limit }
copy-activity-arrival-replayed = { $span } پس از بلاک بازپخش شد
copy-activity-arrival-seen = { $span } پس از بلاک دیده شد
copy-activity-link-wallet-tx = تراکنش کیف پول
copy-activity-link-own-tx = تراکنش شما
copy-activity-only-token = فقط این توکن
copy-activity-skipped-group = نادیده‌گرفته‌شده ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } توکن · از { $since }
       *[other] { $tokens } توکن · از { $since }
    }
copy-activity-mint-filter =
    .placeholder = مینت توکن
    .aria-label = فیلتر بر اساس مینت توکن
copy-activity-clear = پاک‌کردن
copy-activity-load-failed = بارگذاری فعالیت ممکن نشد: { $error }
copy-activity-loading = در حال بارگذاری فعالیت…
copy-activity-no-match = چیزی با این فیلتر مطابقت ندارد.
copy-activity-empty = هنوز تصمیمی وجود ندارد. با معامله کیف پول، اجراها، خروج‌ها و موارد نادیده اینجا ظاهر می‌شوند.
copy-activity-load-older = بارگذاری قدیمی‌تر
copy-activity-start = ابتدای تاریخچه
copy-activity-older-failed = بارگذاری فعالیت قدیمی‌تر ممکن نشد

## Task editor

copy-step-wallet = کیف پول
copy-step-sizing = اندازه‌گذاری
copy-step-entry = فیلترهای ورود
copy-step-exits = خروج‌ها
copy-step-review = بازبینی
copy-editor-title-edit = ویرایش { $name }
copy-editor-title-clone = کلون { $name }
copy-editor-sub-edit = وظیفه { $mode } · تغییرات روی تصمیم‌های بعدی آن اعمال می‌شود
copy-editor-sub-clone = همان قوانین، دفتر آزمایشی خالی، در حالت آزمایشی شروع می‌شود
copy-editor-save-edit = ذخیره تغییرات
copy-editor-save-clone = ایجاد کلون
copy-editor-save-create = ایجاد وظیفه آزمایشی
copy-editor-clone-suffix = (کپی)
copy-editor-discard-edit = دور انداختن تغییرات
copy-editor-discard-create = دور انداختن این وظیفه
copy-editor-discard-edit-message = تغییرات شما در «{ $name }» ذخیره نشده است.
copy-editor-discard-create-message = کیف پول و قوانینی که تاکنون وارد شده ذخیره نشده است.
copy-editor-discard-confirm = دور انداختن
copy-editor-keep-editing = ادامه ویرایش
copy-editor-toast-updated = وظیفه به‌روزرسانی شد
copy-editor-toast-clone = کلون ایجاد شد
copy-editor-toast-created = وظیفه آزمایشی ایجاد شد
copy-unit-native = { -sol }
copy-editor-any = هر مقدار
copy-editor-duplicate = از قبل توسط { $tasks } کپی می‌شود. این وظیفه همان معاملات را دوباره با قوانین و بودجه خودش کپی می‌کند.
copy-editor-wallet = کیف پول
copy-editor-wallet-identity = کیف پول هر وظیفه هویت آن است. برای کپی کیف پول دیگر با این قوانین، وظیفه را کلون کنید.
copy-editor-address-label = آدرس کیف پول
copy-editor-address-placeholder = آدرس کیف پول Solana
copy-editor-address-help-clone = همان قوانین با دفتر آزمایشی خالی. برای آزمایش قوانین دیگر روی این کیف پول، آن را نگه دارید یا کیف پول دیگری وارد کنید.
copy-editor-address-help-create = کیف پولی که این وظیفه خریدهای آن (و در صورت انتخاب شما، فروش‌های آن) را کپی می‌کند.
copy-editor-name-label = نام <em>اختیاری</em>
copy-editor-name-placeholder = مثلاً چرخشی سریع
copy-editor-enabled-title = پردازش معاملات کیف پول
copy-editor-enabled-help = خاموش بودن، وظیفه را تا زمانی که ازسر بگیرید متوقف نگه می‌دارد.
copy-editor-note-live = این وظیفه زنده است: تغییرات روی کپی‌های واقعی بعدی آن اعمال می‌شود.
copy-editor-note-paper = وظایف تا زمانی که فعالشان کنید در حالت آزمایشی اجرا می‌شوند: معاملات با قیمت استخر شبیه‌سازی می‌شوند و چیزی خرج نمی‌شود.
copy-editor-copy-size = اندازه کپی
copy-editor-sizing-fixed = مقدار ثابت
copy-editor-sizing-ratio = سهمی از معامله کیف پول
copy-editor-amount-fixed = مقدار برای هر کپی
copy-editor-amount-ratio = سهم از هر معامله
copy-editor-amount-help-fixed = برای هر خرید کپی‌شده خرج می‌شود، دست‌کم { $minimum }.
copy-editor-amount-help-ratio = از خرید خود کیف پول، تا سقف هر معامله.
copy-editor-help-trade-cap = هیچ کپی منفردی بیشتر خرج نمی‌کند.
copy-editor-help-token-cap = کل مبلغ خرج‌شده روی یک توکن.
copy-editor-help-budget = هر آنچه این وظیفه در طول عمرش می‌تواند خرج کند؛ حالت آزمایشی و زنده هر کدام مصرف خود را حساب می‌کنند.
copy-editor-preview-title = هزینه یک کپی
copy-editor-preview-empty = برای دیدن هزینه یک کپی، اندازه‌گذاری را وارد کنید.
copy-editor-preview-example = کیف پول { $target } می‌خرد → شما <strong>{ $copy }</strong> کپی می‌کنید
copy-editor-preview-once = هر توکن یک کپی { $size } دریافت می‌کند، چون هر توکن یک‌بار خریده می‌شود
copy-editor-preview-token-cap =
    { $count ->
        [one] هر توکن حداکثر { $count } کپی { $size } دریافت می‌کند
       *[other] هر توکن حداکثر { $count } کپی { $size } دریافت می‌کند
    }
copy-editor-preview-summary-exact = { $perToken }؛ بودجه حدود { $count } مورد از آن را پوشش می‌دهد. کارمزد شبکه و اولویت اضافه می‌شود.
copy-editor-preview-summary-minimum = { $perToken }؛ بودجه دست‌کم { $count } مورد از آن را پوشش می‌دهد. کارمزد شبکه و اولویت اضافه می‌شود.
copy-editor-target-min = کوچک‌ترین معامله کیف پول که کپی می‌شود
copy-editor-target-min-help = خریدهای کوچک‌تر کیف پول نادیده گرفته می‌شوند. برای نداشتن حداقل خالی بگذارید.
copy-editor-target-max = بزرگ‌ترین معامله کیف پول که کپی می‌شود
copy-editor-target-max-help = خریدهای بزرگ‌تر کیف پول نادیده گرفته می‌شوند. برای نداشتن حداکثر خالی بگذارید.
copy-editor-buy-once-title = هر توکن یک‌بار خریداری شود
copy-editor-buy-once-help = فقط اولین خرید کیف پول از یک توکن کپی می‌شود؛ خریدهای بعدی آن نادیده گرفته می‌شوند.
copy-editor-filter-require = الزامی
copy-editor-filter-skip = غیرالزامی
copy-editor-filter-help = توکن پیش از کپی‌شدن باید از خط لوله فیلترینگ شما عبور کند.
copy-editor-filter-warning = با تنظیمات پیش‌فرض فیلترینگ تقریباً همه توکن‌ها رد می‌شوند، پس وظیفه‌ای که عبور را الزامی کند چیزی کپی نمی‌کند. فقط وقتی آن را الزامی کنید که فیلترهای شما توکن‌های معامله‌شده توسط این کیف پول را عبور دهند.
copy-editor-exit-both = هر دو
copy-editor-exit-help-buy-only = قوانین شما در زیر هر دارایی را می‌فروشند؛ فروش‌های کیف پول نادیده گرفته می‌شوند.
copy-editor-exit-help-hybrid = هر کدام زودتر برسد: کیف پول بفروشد، یا یکی از قوانین شما فعال شود.
copy-editor-exit-help-mirror = دارایی‌ها فقط وقتی کیف پول می‌فروشد فروخته می‌شوند. قوانین خروج شما اجرا نمی‌شوند.
copy-editor-who-sells = چه کسی می‌فروشد
copy-editor-preset = پیش‌تنظیم
copy-editor-preset-help = یک پیش‌تنظیم همه قوانین زیر را پر می‌کند؛ پس از آن هرکدام را تنظیم کنید.
copy-editor-mirror-note = تا زمانی که فروش‌های کیف پول تصمیم می‌گیرند این قوانین اجرا نمی‌شوند. اگر به { $mine } یا { $both } تغییر دهید اعمال می‌شوند.
copy-editor-rule-inherit = پیش‌فرض معامله‌گر
copy-editor-inherit-value = پیش‌فرض معامله‌گر ({ $value })
copy-editor-rule-aria = تنظیم { $rule }
copy-editor-rule-empty-uses = خالی یعنی پیش‌فرض معامله‌گر: { $value }
copy-editor-rule-follows = پیرو معامله‌گر: { $summary }
copy-editor-rule-follows-plain = پیرو تنظیم معامله‌گر.
copy-editor-rule-follows-own = روشن/خاموش پیرو معامله‌گر، با مقدارهای این وظیفه: { $summary }
copy-editor-rule-off-note = برای این وظیفه خاموش است، هر چه معامله‌گر استفاده کند.
copy-task-unnamed = وظیفه بدون نام
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = پس از ذخیره معاملات را پردازش می‌کند
copy-editor-review-paused = متوقف ذخیره می‌شود
copy-editor-error-address = یک آدرس کیف پول Solana معتبر وارد کنید.
copy-editor-error-sizing = همه مقادیر اندازه‌گذاری باید بیشتر از صفر باشند.
copy-editor-error-min-copy = هر کپی باید دست‌کم { $minimum } باشد: مقدار برای هر کپی را افزایش دهید.
copy-editor-error-min-cap = هر کپی باید دست‌کم { $minimum } باشد: سقف هر معامله را افزایش دهید.
copy-editor-error-trade-cap = سقف هر معامله نمی‌تواند از سقف هر توکن بیشتر باشد.
copy-editor-error-token-cap = سقف هر توکن نمی‌تواند از کل بودجه بیشتر باشد.
copy-editor-error-slippage = اسلیپیج باید بین { $min } و { $max } باشد.
copy-editor-error-target-limits = حدود معامله کیف پول باید صفر یا بیشتر باشند.
copy-editor-error-target-order = کوچک‌ترین معامله کیف پول نمی‌تواند از بزرگ‌ترین بیشتر باشد.

## Copy notices

copy-notice-task-unnamed = وظیفه #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = خرید کپی آزمایشی
copy-notice-title-paper-sell = فروش کپی آزمایشی
copy-notice-title-paper-closed = دارایی آزمایشی بسته شد
copy-notice-title-paper-exit = خروج آزمایشی: { $rule }
copy-notice-title-live-buy-submitted = خرید کپی زنده ارسال شد
copy-notice-title-live-buy-confirmed = خرید کپی زنده تأیید شد
copy-notice-title-live-buy-failed = خرید کپی زنده ناموفق بود
copy-notice-title-live-sell-submitted = فروش کپی زنده ارسال شد
copy-notice-title-live-sell-failed = فروش کپی زنده ناموفق بود
copy-notice-title-auto-paused = وظیفه کپی به‌طور خودکار متوقف شد
copy-notice-detail-bought = خرید به مبلغ { $amount } { -sol }
copy-notice-detail-sold = فروش به مبلغ { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% از دارایی
copy-notice-detail-full-close = بستن کامل
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = سواپ ناموفق بود
