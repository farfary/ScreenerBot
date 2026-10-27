# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = حد ضرر
trader-exit-type-take-profit = حد سود
trader-exit-type-roi = حد سود
trader-exit-type-roi-exit = حد سود
trader-exit-type-trailing-stop = حد ضرر متحرک
trader-exit-type-time-override = خروج زمانی
trader-exit-type-time-rule = قانون زمانی
trader-exit-type-manual = دستی
trader-exit-type-manual-close = دستی
trader-exit-type-dca = DCA
trader-exit-type-unknown = نامشخص

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = آمار
trader-tab-strategy-control = کنترل استراتژی
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

## Status bar and trading controls

trader-status-title = معامله‌گر خودکار
trader-status-loading = در حال بارگذاری...
trader-status-running = در حال اجرا
trader-status-stopped = متوقف
trader-status-setup-required = نیازمند راه‌اندازی
trader-status-unavailable = برای استفاده از معامله‌گر خودکار، راه‌اندازی کیف پول و RPC را کامل کنید
trader-setup-action = تکمیل راه‌اندازی
trader-toggle-on = فعال
trader-toggle-off = غیرفعال
trader-toggle-start-failed = راه‌اندازی معامله‌گر ناموفق بود
trader-toggle-stop-failed = توقف معامله‌گر ناموفق بود
trader-controls-title = کنترل معاملات
trader-halt-title = معاملات متوقف شده است
trader-halt-reason-default = توقف اجباری دستی
trader-halt-resume = ادامه
trader-monitor-entry = پایش ورود
trader-monitor-exit = پایش خروج
trader-monitor-master-off = معامله‌گر خودکار غیرفعال است
trader-loss-limit-title = محدودیت ضرر دوره‌ای
trader-loss-limit-resume = ازسرگیری معاملات
trader-loss-limit-reset = بازنشانی دوره
trader-loss-limit-off = غیرفعال
trader-loss-limit-none = محدودیت ضرر دوره‌ای تنظیم نشده است
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = بازنشانی تا { $hours } { $minutes } دیگر
trader-loss-limit-reached = به سقف رسیده است
trader-force-stop = توقف اجباری همه چیز
trader-force-stop-hint = همهٔ مانیتورها، وظایف کپی و معاملات را یک‌جا متوقف می‌کند.
trader-force-stop-idle = چیزی در حال اجرا نیست که توقف اجباری آن را متوقف کند.

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
trader-toast-strategy-enabled = استراتژی فعال شد
    .message = استراتژی فعال است
trader-toast-strategy-disabled = استراتژی غیرفعال شد
    .message = استراتژی غیرفعال است
trader-toast-strategy-failed = به‌روزرسانی ناموفق بود
    .message = به‌روزرسانی وضعیت استراتژی ناموفق بود

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = بازه آمار
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = عملکرد تحقق‌یافته
trader-metric-net-pnl = سود و زیان خالص
trader-metric-win-rate = نرخ برد
trader-metric-profit-factor = ضریب سود
trader-metric-max-drawdown = بیشترین افت سرمایه
trader-metric-capital = سرمایه درگیر
trader-metric-avg-win-loss = میانگین برد / باخت
trader-metric-closed-trades = معاملات بسته‌شده
trader-metric-median-hold = میانه مدت نگهداری
trader-stats-empty = در این بازه معامله بسته‌شده‌ای وجود ندارد
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
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [one] { $used } از { $max } جایگاه پوزیشن استفاده شده
       *[other] { $used } از { $max } جایگاه پوزیشن استفاده شده
    }
trader-stats-avg-basis = میانگین نتیجه معامله‌های برنده در برابر بازنده
trader-stats-closed =
    { $count ->
        [one] { $amount } پوزیشن بسته شد
       *[other] { $amount } پوزیشن بسته شد
    }
# $span is a formatted duration.
trader-stats-hold-average = میانگین { $span }
trader-stats-excluded =
    { $count ->
        [one] { $amount } دور بسته‌شده کنار گذاشته شد — قیمت تمام‌شده کامل نیست و سود و زیان دقیقی قابل محاسبه نیست.
       *[other] { $amount } دور بسته‌شده کنار گذاشته شد — قیمت تمام‌شده کامل نیست و سود و زیان دقیقی قابل محاسبه نیست.
    }

## Stats: daily P&L and extremes

trader-daily-title = سود و زیان روزانه
trader-daily-subtitle = { -sol } تحقق‌یافته در هر روز، همراه با مجموع تجمعی
trader-daily-loading = در حال بارگذاری سود و زیان روزانه...
trader-daily-chart = سود و زیان تحقق‌یافته روزانه به { -sol }
trader-extreme-best = بهترین معامله
trader-extreme-worst = بدترین معامله

## Stats: exit breakdown

trader-exit-title = تفکیک استراتژی‌های خروج
trader-exit-subtitle = نحوه بسته شدن پوزیشن‌ها و بازده هر خروج
trader-exit-loading = در حال بارگذاری داده‌های خروج...
trader-exit-empty-day = در 24 ساعت گذشته معامله بسته‌شده‌ای وجود ندارد
trader-exit-empty-days =
    { $count ->
        [one] در { $amount } روز گذشته معامله بسته‌شده‌ای وجود ندارد
       *[other] در { $amount } روز گذشته معامله بسته‌شده‌ای وجود ندارد
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [one] { $amount } معامله · { $share } از خروج‌ها
       *[other] { $amount } معامله · { $share } از خروج‌ها
    }
# $value is a formatted average percentage.
trader-exit-average = میانگین { $value }

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
# Shown before the configured duration loads.
trader-time-conversion-default = 168 ساعت = 7 روز
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

## Strategy control

trader-strategy-entry-title = استراتژی‌های ورود
trader-strategy-entry-subtitle = سیگنال‌هایی که می‌توانند پوزیشن جدید باز کنند.
trader-strategy-exit-title = استراتژی‌های خروج
trader-strategy-exit-subtitle = سیگنال‌هایی که می‌توانند پوزیشن باز را ببندند یا از آن محافظت کنند.
trader-strategy-active-unknown = -- فعال
trader-strategy-active = { $enabled } از { $total } فعال
trader-strategy-loading = در حال بارگذاری استراتژی‌ها...
trader-strategy-load-failed = بارگذاری استراتژی‌ها ممکن نشد
trader-strategy-empty = استراتژی‌ای تعریف نشده است
trader-strategy-unnamed = استراتژی بدون نام
trader-strategy-priority-auto = خودکار
trader-strategy-priority = اولویت { $priority }

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
trader-dca-min-hold-impact =
    { $minutes ->
        [0] نخستین افزایش می‌تواند بلافاصله پس از ورود باشد
       *[other] نخستین افزایش دست‌کم { $span } پس از ورود صبر می‌کند
    }
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
