# Trader page labels.

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = آمار
trader-tab-strategies = استراتژی‌ها
trader-tab-stop-loss = حد ضرر
trader-tab-trailing-stop = حد ضرر متحرک
trader-tab-take-profit = حد سود
trader-tab-time-rules = خروج زمانی
trader-tab-dca = DCA
trader-tab-settings = تنظیمات

## Feature status badges and their messages

trader-feature-coming-soon = به‌زودی
    .message = این قابلیت به‌زودی عرضه می‌شود و هنوز در دسترس نیست.
trader-feature-beta = بتا
trader-feature-disabled = غیرفعال
    .message = این قابلیت در حال حاضر غیرفعال است.

## Toolbar, notices and trading controls

trader-status-title = معامله‌گر خودکار
trader-status-running = در حال اجرا
trader-status-stopped = متوقف
trader-status-setup-required = نیازمند راه‌اندازی
trader-setup-action = تکمیل راه‌اندازی
trader-toggle-start-failed = راه‌اندازی معامله‌گر ناموفق بود
trader-toggle-stop-failed = توقف معامله‌گر ناموفق بود
trader-halt-title = معاملات متوقف شده است
trader-halt-reason-default = توقف اجباری دستی
trader-halt-resume = ادامه
trader-monitor-entry = پایش ورود
trader-monitor-exit = پایش خروج
trader-loss-limit-title = محدودیت ضرر دوره‌ای
trader-loss-limit-resume = ازسرگیری معاملات
trader-loss-limit-reset = بازنشانی دوره
trader-loss-limit-off = غیرفعال
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = بازنشانی تا { $hours } { $minutes } دیگر
trader-force-stop = توقف اجباری همه چیز
trader-force-stop-hint = همهٔ مانیتورها، وظایف کپی و معاملات را یک‌جا متوقف می‌کند.
trader-force-stop-idle = چیزی در حال اجرا نیست که توقف اجباری آن را متوقف کند.
trader-run-state-force-stopped = توقف اجباری
trader-run-state-waiting = در انتظار سرویس‌ها
trader-run-state-idle = پایش‌ها خاموش
trader-entries-label = ورودهای جدید
trader-entries-open = باز
trader-entries-monitor-off = پایش ورود خاموش
trader-entries-no-live-strategy = استراتژی ورود زنده‌ای وجود ندارد
trader-entries-no-paper-strategy = استراتژی ورود آزمایشی‌ای وجود ندارد
trader-action-start = شروع معامله‌گر خودکار
trader-action-stop = توقف معامله‌گر خودکار
trader-notice-loss-limit = حد زیان رسید
trader-notice-loss-limit-message = ورودهای زنده جدید تا بازنشانی دوره متوقف شده‌اند. خروج‌ها ادامه دارند.
trader-notice-stats-failed = آمار بارگیری نشد
trader-controls-group =
    .aria-label = کنترل‌های معامله
trader-monitor-dormant = همراه با معامله‌گر خودکار شروع می‌شود

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = توقف اجباری معاملات
    .message = این کار همه عملیات معاملاتی را فوراً متوقف می‌کند. ادامه می‌دهید؟
    .confirm = توقف معاملات
trader-loss-limit-resume-confirm = ازسرگیری پس از محدودیت ضرر
    .message = محدودیت ضرر دوره‌ای ورودهای جدید را متوقف کرده است. با ازسرگیری، معامله‌گر پیش از بازنشانی دوره دوباره می‌تواند پوزیشن باز کند. ادامه می‌دهید؟
trader-loss-limit-reset-confirm = بازنشانی دوره محدودیت ضرر
    .message = این کار ضرر انباشته‌شده در دوره فعلی را پاک می‌کند و دوره جدیدی آغاز می‌کند. ادامه می‌دهید؟

## Toasts

trader-toast-control-failed = کنترل معامله‌گر خودکار ناموفق بود
trader-toast-force-stop-on = توقف اجباری فعال شد
trader-toast-force-stop-failed = فعال‌سازی توقف اجباری ممکن نشد
trader-toast-force-stop-cleared = توقف اجباری برداشته شد
trader-toast-resume-failed = ازسرگیری معاملات ممکن نشد
trader-toast-loss-limit-reset-failed = بازنشانی محدودیت ضرر ممکن نشد
trader-toast-entry-monitor-failed = تغییر وضعیت پایش ورود ممکن نشد
trader-toast-exit-monitor-failed = تغییر وضعیت پایش خروج ممکن نشد
trader-toast-load-failed = بارگذاری ناموفق بود
    .message = بارگذاری پیکربندی معامله‌گر ناموفق بود
trader-toast-saved = پیکربندی ذخیره شد
    .message = تنظیمات معامله‌گر با موفقیت اعمال شد
trader-toast-save-failed = ذخیره ناموفق بود
    .message = ذخیره پیکربندی معامله‌گر ناموفق بود
trader-toast-feature-enabled = قابلیت فعال شد
trader-toast-feature-disabled = قابلیت غیرفعال شد
trader-toast-feature-applied = تنظیم معامله‌گر خودکار اعمال شد

## Stats: book, window, exposure and figures

trader-stats-window =
    .aria-label = بازه آمار
trader-stats-book =
    .aria-label = دفتر معاملات
trader-exposure-cash = نقد
trader-exposure-open = باز
trader-exposure-unrealized = تحقق‌نیافته
trader-exposure-open-title = پوزیشن‌هایی که یک جایگاه می‌گیرند، از سقف پوزیشن‌های باز
trader-exposure-in-use = در حال استفاده
trader-exposure-in-use-title = { -sol } صرف‌شده در پوزیشن‌های باز
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
        [one] { $amount } پوزیشن باز هنوز قیمت زنده ندارد
       *[other] { $amount } پوزیشن باز هنوز قیمت زنده ندارند
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = دوره آزمایشی از { $date } · شروع با { $amount }
trader-paper-reset = بازنشانی دوره آزمایشی
trader-paper-reset-title = دوره آزمایشی بازنشانی شود؟
trader-paper-reset-run-message = هر پوزیشن آزمایشی باز، از جمله دارایی‌های آزمایشی وظیفه‌های کپی، با قیمت فعلی‌اش بسته می‌شود و دوره‌ای تازه با موجودی اولیه تنظیم‌شده آغاز می‌شود. دوره‌های قبلی در تاریخچه می‌مانند.
trader-paper-reset-done = دوره آزمایشی تازه‌ای آغاز شد
trader-paper-reset-failed = بازنشانی دوره آزمایشی ممکن نشد
trader-realized-title = عملکرد تحقق‌یافته
trader-metric-net-pnl = سود و زیان خالص
trader-metric-win-rate = نرخ برد
trader-metric-profit-factor = ضریب سود
trader-metric-max-drawdown = بیشترین افت سرمایه
trader-metric-avg-win-loss = میانگین برد / باخت
trader-metric-closed-trades = معاملات بسته‌شده
trader-metric-median-hold = میانه مدت نگهداری
trader-metric-fees = کارمزد شبکه
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = سود { $won } · زیان { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } برد
       *[other] { $amount } برد
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } باخت
       *[other] { $amount } باخت
    }
# $amount is a formatted SOL amount.
trader-stats-expected = { $amount } انتظار در هر معامله
trader-stats-profit-factor-basis = مجموع سود ÷ مجموع زیان
trader-stats-drawdown-basis = عمیق‌ترین افت تحقق‌یافته از اوج تا کف
trader-stats-excluded =
    { $count ->
        [one] { $amount } معامله بسته‌شده کنار گذاشته شد: قیمت تمام‌شده‌اش کامل نیست، پس سود و زیانش نامعلوم است.
       *[other] { $amount } معامله بسته‌شده کنار گذاشته شد: قیمت تمام‌شده‌اش کامل نیست، پس سود و زیانش نامعلوم است.
    }
# $ratio is a formatted ratio.
trader-stats-payoff = نسبت بازده { $ratio }
trader-stats-streak-win =
    { $count ->
        [one] آخرین معامله برد
       *[other] { $amount } برد پیاپی
    }
trader-stats-streak-loss =
    { $count ->
        [one] آخرین معامله باخت
       *[other] { $amount } باخت پیاپی
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = بردها { $wins } · باخت‌ها { $losses }
trader-stats-fees-basis = در سود و زیان خالص لحاظ شده است
trader-stats-loading = در حال بارگیری آمار...
trader-open-strategies = باز کردن استراتژی‌ها

## Stats: P&L chart, trade outcomes and extremes

trader-daily-title = سود و زیان روزانه
trader-daily-subtitle = { -sol } تحقق‌یافته در هر روز، همراه با مجموع تجمعی
trader-daily-chart = سود و زیان تحقق‌یافته روزانه به { -sol }
trader-weekly-title = سود و زیان هفتگی
trader-weekly-subtitle = { -sol } تحقق‌یافته در هر هفته ISO، همراه با مجموع تجمعی
trader-weekly-chart = سود و زیان تحقق‌یافته هفتگی به { -sol }
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
        [one] { $date } · { $pnl } · { $amount } معامله
       *[other] { $date } · { $pnl } · { $amount } معامله
    }
trader-weekly-bar =
    { $count ->
        [one] هفته { $date } · { $pnl } · { $amount } معامله
       *[other] هفته { $date } · { $pnl } · { $amount } معامله
    }
trader-extreme-best = بهترین معامله
trader-extreme-worst = بدترین معامله
trader-extreme-runs = بیشترین پشت‌سرهم
trader-outcome-title = نتایج معاملات
trader-outcome-subtitle = معاملات بسته‌شده بر اساس بازده، به درصد
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = کمتر از { $value }
trader-outcome-range = { $from } تا { $to }
trader-outcome-above = { $value } یا بیشتر
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
        [one] { $range } · { $amount } معامله · { $pnl }
       *[other] { $range } · { $amount } معامله · { $pnl }
    }

## Stats: empty windows, exit reasons, entry sources and recent closes

trader-stats-empty-day = در 24 ساعت گذشته معامله بسته‌شده‌ای وجود ندارد
trader-stats-empty-all = هنوز معامله بسته‌شده‌ای وجود ندارد
trader-stats-empty-hint = معاملات بسته‌شده پس از فروش یک پوزیشن اینجا نمایش داده می‌شوند. معامله‌گر خودکار را روشن کنید یا بازه طولانی‌تری انتخاب کنید.
trader-stats-empty-days =
    { $count ->
        [one] در { $amount } روز گذشته معامله بسته‌شده‌ای وجود ندارد
       *[other] در { $amount } روز گذشته معامله بسته‌شده‌ای وجود ندارد
    }
trader-exits-title = دلایل خروج
trader-exits-subtitle = معاملات بسته‌شده چگونه پایان یافتند و هر خروج چه بازدهی داشت
trader-sources-title = منابع ورود
trader-sources-subtitle = چه چیزی معاملات بسته‌شده را باز کرد
trader-recent-title = بسته‌شدن‌های اخیر
trader-recent-subtitle = آخرین معاملات بسته‌شده در این بازه
trader-column-token = توکن
trader-column-exit = خروج
trader-column-source = منبع
trader-column-trades = معاملات
trader-column-win-rate = نرخ برد
trader-column-avg-return = میانگین بازده
trader-column-net-pnl = سود و زیان خالص
trader-column-held = مدت نگهداری
trader-column-closed = بسته‌شده
trader-column-pnl = سود و زیان
trader-column-return = بازده

## Shared example vocabulary

trader-example-how-it-works = نحوه کار
trader-step-entry = ورود
trader-step-initial-position = پوزیشن اولیه
trader-step-auto-exit = خروج خودکار
trader-step-exit = خروج
trader-step-full-exit = خروج کامل از پوزیشن
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% سود

## Stop loss

trader-stop-loss-title = حد ضرر
trader-stop-loss-subtitle = خروج خودکار از پوزیشن وقتی ضرر آن از آستانه شما فراتر رود
# $threshold is the threshold as typed.
trader-stop-loss-impact = خروج در صورت افت { $threshold }% از قیمت ورود
trader-stop-loss-hold-immediate = فوری
# $span is a formatted duration.
trader-stop-loss-hold-delay = { $span } تأخیر
trader-stop-loss-price-falls = افت قیمت
trader-stop-loss-threshold-reached = رسیدن به آستانه
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = فروش { $part }
trader-stop-loss-part-of-position = بخشی از پوزیشن
trader-stop-loss-sell-rest = فروش باقی‌مانده
trader-stop-loss-still-below = همچنان زیر حد ضرر
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = ضرر محدود به <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>نکته:</strong> حد ضرر با خروج زودهنگام، از ضررهای بزرگ‌تر جلوگیری می‌کند

## Trailing stop

trader-trailing-title = حد ضرر متحرک
trader-trailing-subtitle = محافظت خودکار از سود با دنبال کردن قیمت هنگام صعود
# $value is the activation percentage as typed.
trader-trailing-activation-impact = دنبال کردن از +{ $value }% سود شروع می‌شود
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = خروج در -{ $value }% از اوج
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = حداقل سود قفل‌شده هنگام فعال‌شدن: { $value }
trader-trailing-locked-minimum-warning = حد ضرر هنگام فعال‌شدن روی قیمت ورود یا زیر آن قرار می‌گیرد و فقط در سود می‌فروشد. برای داشتن کف، حد ضرر سربه‌سر را روشن کنید.
trader-trailing-activation = فعال‌سازی
trader-trailing-peak = اوج
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% نهایی
# $value is a formatted percentage.
trader-trailing-summary-protected = <strong>{ $value }</strong> سود حفظ شد
# $value is a formatted percentage.
trader-trailing-summary-avoided = از <strong>{ $value }</strong> ضرر نسبت به اوج جلوگیری شد
# $step is the band's position in the list, from 1.
trader-trailing-band = باند { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = سود اوج باند { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = فاصله دنبال‌کردن باند { $step }
trader-trailing-band-profit-caption = از سود اوج
trader-trailing-band-distance-caption = فاصله دنبال‌کردن
trader-trailing-add-band = افزودن باند
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = حذف باند { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = سود اوج هر باند باید بین { $min }% و { $max }% باشد
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = فاصله هر باند باید بین { $min }% و { $max }% باشد
trader-trailing-band-error-order = سود اوج هر باند باید از سطح فعال‌سازی و باند قبلی بالاتر باشد
trader-trailing-band-error-distance = فاصله هر باند باید کمتر از سود اوج آن باشد

## Break-even stop

trader-break-even-title = حد ضرر سربه‌سر
trader-break-even-subtitle = وقتی پوزیشن وارد سود شد، پیش از آنکه به ضرر برگردد همه را بفروشید
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = وقتی بهترین قیمت از آخرین خرید به { $trigger } برسد، بازگشت به { $offset } همه را می‌فروشد.

## Take profit

trader-take-profit-title = حد سود
trader-take-profit-subtitle = پوزیشن را با رسیدن سود به هر سطح، پله‌پله می‌فروشد
trader-take-profit-initial-buy = خرید اولیه
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = پله { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = سطح سود پله { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = سهم فروش پله { $step }
trader-take-profit-profit-caption = سطح سود
trader-take-profit-sell-caption = سهم فروش
trader-take-profit-add-step = افزودن پله
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = حذف پله { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = هر سطح سود باید بین { $min }% و { $max }% باشد
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = هر سهم باید بین { $min }% و { $max }% باشد
trader-take-profit-error-order = سطح سود هر پله باید از پله قبلی بالاتر باشد
# $max is the largest total share, the whole position.
trader-take-profit-error-total = پله‌ها روی هم حداکثر { $max }% می‌فروشند
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% از پوزیشن به‌عنوان باقی‌مانده باز می‌ماند
trader-take-profit-runner-none = پله آخر باقی‌مانده پوزیشن را می‌فروشد
trader-take-profit-runner-warning = با خاموش بودن حد ضرر متحرک، حد ضرر سربه‌سر، هر دو خروج زمانی و همه استراتژی‌های خروج، باقی‌مانده فقط با حد ضرر یا به‌صورت دستی بسته می‌شود
trader-take-profit-info = سهم‌ها از پوزیشن اولیه حساب می‌شوند. اگر قیمت چند سطح را یک‌باره رد کند، در هر بررسی یک پله فروخته می‌شود.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = در +{ $profit }%، فروش { $share }%
trader-take-profit-example-runner = باقی‌مانده
trader-take-profit-example-runner-detail = به قوانین خروج دیگر سپرده می‌شود
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = تا سود +{ $profit }%، <strong>{ $sold }%</strong> را می‌فروشد

## Time-based exit

trader-time-loss-title = بستن پوزیشن‌های در ضرر
trader-time-subtitle = خروج خودکار از پوزیشن‌ها پس از حداکثر مدت نگهداری، در صورتی که ضرر از آستانه فراتر رفته باشد
trader-time-flat-title = بستن پوزیشن‌های راکد
trader-time-flat-subtitle = آزادکردن سرمایهٔ پوزیشن‌هایی که نه بالا می‌روند و نه پایین
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = پوزیشنی که { $span } یا بیشتر نگه داشته شود و سود و زیانش در محدودهٔ ±{ $band } بماند، بسته می‌شود.
trader-time-unit-seconds = ثانیه
trader-time-unit-minutes = دقیقه
trader-time-unit-hours = ساعت
trader-time-unit-days = روز
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } ثانیه
       *[other] { $amount } ثانیه
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } دقیقه
       *[other] { $amount } دقیقه
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } ساعت
       *[other] { $amount } ساعت
    }
trader-duration-days =
    { $count ->
        [one] { $amount } روز
       *[other] { $amount } روز
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = خروج در صورت افت { $value }% یا بیشتر پس از پایان مدت نگهداری
trader-time-loss-hint = زیانی که خروج زمانی پس از پایان مدت نگهداری در آن می‌فروشد (40 = خروج در -40%)
# $day is the day number of the example.
trader-time-day = روز { $day }
trader-time-position-opened = پوزیشن باز شد
trader-time-limit = محدودیت زمانی
trader-time-hold-reached = مدت نگهداری به پایان رسید
trader-time-loss-met = آستانه ضرر رسید
trader-time-note = <strong>نکته:</strong> از پوزیشن‌های سودده یا با ضرر کمتر خارج نمی‌شود
trader-time-positions-title = وضعیت پوزیشن‌های فعلی
trader-time-positions-loading = در حال بارگذاری پوزیشن‌ها...
trader-time-positions-empty = پوزیشن بازی وجود ندارد
trader-time-positions-empty-message = پوزیشن‌های باز با مدت نگهداری و ROI نسبت به محدودیت زمانی اینجا نمایش داده می‌شوند.
trader-time-positions-token = توکن
trader-time-positions-hold = مدت نگهداری
trader-time-positions-roi = ROI
trader-time-positions-rule = خروج زمانی
trader-time-status-off = خروج‌های زمانی خاموش هستند
trader-time-status-none = اکنون هیچ خروج زمانی اعمال نمی‌شود
trader-time-status-not-managed = توسط معامله‌گر خودکار مدیریت نمی‌شود
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = در ضرر { $pnl }؛ بسته شدن تا { $remaining } دیگر
trader-time-status-losing-due = در ضرر { $pnl }؛ در بررسی بعدی بسته می‌شود
# $held and $remaining are formatted durations.
trader-time-status-flat = راکد از { $held } پیش؛ بسته شدن تا { $remaining } دیگر
trader-time-status-flat-due = راکد از { $held } پیش؛ در بررسی بعدی بسته می‌شود

## Dollar-cost averaging

trader-dca-title = میانگین‌گیری قیمت
trader-dca-subtitle = افزودن خودکار به پوزیشن‌های زیان‌ده برای کاهش میانگین قیمت ورود
trader-dca-measure-average = میانگین ورود
trader-dca-measure-first-entry = ورود اول
trader-dca-step = افزایش { $step }
trader-dca-drop-label = افت قیمت افزایش { $step }
trader-dca-size-label = اندازه افزایش { $step }، ضریب خرید اول
trader-dca-drop-caption = افت قیمت
trader-dca-size-caption = خرید × خرید اول
trader-dca-add-step = افزودن سطح
trader-dca-remove-step = حذف افزایش { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = کمتر از حداقل معامله { $minimum }: این افزایش ارسال نمی‌شود
trader-dca-error-drop-range = هر افت باید بین { $min }% و { $max }% باشد
trader-dca-error-size-range = هر اندازه باید بین { $min }× و { $max }× خرید اول باشد
trader-dca-error-order = با سنجش از ورود اول، هر افزایش به افتی عمیق‌تر از قبلی نیاز دارد
trader-dca-cooldown-impact = دست‌کم { $span } بین ورودهای اضافه
trader-dca-min-hold-none = نخستین افزایش می‌تواند بلافاصله پس از ورود باشد
trader-dca-min-hold-impact = نخستین افزایش دست‌کم { $span } پس از ورود صبر می‌کند
trader-dca-capacity = بیشترین مقداری که این پوزیشن می‌تواند داشته باشد: { $amount } ({ $multiple }× خرید اول)
trader-dca-info-title = اطلاعات استراتژی DCA
trader-dca-info-subtitle = نکات مهم درباره معامله با DCA
trader-dca-how-title = DCA چگونه کار می‌کند
trader-dca-how-trigger = <strong>محرک:</strong> قیمت به زیر سطح بعدی می‌رسد، سنجیده از میانگین ورود یا ورود اول
trader-dca-how-action = <strong>اقدام:</strong> افزودن { -sol } بیشتر برای کاهش قیمت تمام‌شده میانگین
trader-dca-how-repeat = <strong>نردبان:</strong> هر افزایش به سطح بعدی می‌رود؛ پس از استفاده از همه سطوح، افزایش‌ها متوقف می‌شوند
trader-dca-how-pause = <strong>توقف‌ها:</strong> تا وقتی افزایش قبلی در حال تسویه است یا محدودیت زیان ورودهای تازه را مسدود کرده، افزایشی انجام نمی‌شود
trader-dca-risk-title = هشدارهای ریسک
trader-dca-risk-exposure = <strong>افزایش مواجهه:</strong> DCA کل سرمایه در معرض ریسک هر پوزیشن را بیشتر می‌کند
trader-dca-risk-knife = <strong>چاقوی در حال سقوط:</strong> اگر روند نزولی توکن ادامه یابد، DCA کمکی نمی‌کند
trader-dca-risk-cooldown = <strong>زمان انتظار:</strong> برای جلوگیری از ورودهای DCA پیاپی از زمان انتظار استفاده کنید

## General settings

trader-sizing-title = اندازه پوزیشن
trader-sizing-subtitle = میزان سرمایه‌گذاری در هر پوزیشن را کنترل کنید
trader-timing-title = زمان‌بندی و زمان انتظار
trader-timing-subtitle = زمان‌بندی میان عملیات را کنترل کنید
trader-timing-close-cooldown = زمان انتظار پس از بستن پوزیشن
trader-timing-close-cooldown-hint = دقایق انتظار پیش از باز کردن دوباره پوزیشن روی همان توکن
trader-timing-concurrency = همروندی بررسی ورود
trader-timing-concurrency-hint = تعداد توکن‌هایی که هم‌زمان بررسی می‌شوند (بیشتر = سریع‌تر اما مصرف CPU بیشتر)
trader-timing-unit-minutes = دقیقه
trader-timing-unit-tokens = توکن

## Service mode notice

trader-service-notice-title = در حال اجرا روی سرویس { -brand }
trader-service-notice-message = قیمت‌ها هر چند ثانیه یک بار به‌روز می‌شوند، بنابراین خروج‌های سریع مانند حد ضرر متحرک نزدیک، دیرتر از حالت RPC خودتان واکنش نشان می‌دهند.
