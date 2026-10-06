# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = وضعیت
telegram-reply-balance = موجودی
telegram-reply-positions = پوزیشن‌ها
telegram-reply-pause = مکث
telegram-reply-resume = ادامه
telegram-reply-stop = توقف
telegram-reply-stats = آمار
telegram-reply-menu = منو
telegram-reply-help = راهنما

## Inline keyboard buttons.

telegram-button-positions = پوزیشن‌ها
telegram-button-balance = موجودی
telegram-button-stats = آمار
telegram-button-tokens = توکن‌ها
telegram-button-pause = مکث
telegram-button-stop = توقف
telegram-button-settings = تنظیمات
telegram-button-refresh = تازه‌سازی
telegram-button-menu = منو
telegram-button-back = بازگشت
telegram-button-back-to-menu = بازگشت به منو
telegram-button-back-to-tokens = بازگشت به توکن‌ها
telegram-button-cancel = لغو
telegram-button-close-all-positions = بستن همه پوزیشن‌ها
telegram-button-sell-percent = فروش { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = فهرست سیاه
telegram-button-blacklist-symbol = فهرست سیاه { $symbol }
telegram-button-close-position = بستن پوزیشن
telegram-button-confirm-close = تأیید بستن
telegram-button-confirm-close-all = بستن همه پوزیشن‌ها
telegram-button-confirm-sell = تأیید فروش { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = تأیید توقف اجباری
telegram-button-confirm-buy = خرید { $amount } { -sol }
telegram-button-notifications = اعلان‌ها
telegram-button-trading = معاملات
telegram-button-entry-monitor = پایش ورود
telegram-button-exit-monitor = پایش خروج
telegram-button-auto-trading = معامله خودکار
telegram-button-force-stop = توقف اجباری
telegram-button-notify-opened = بازشده
telegram-button-notify-closed = بسته‌شده
telegram-button-notify-partial = جزئی
telegram-button-notify-dca = DCA
telegram-button-notify-errors = خطاها
telegram-button-details = جزئیات
telegram-button-position = پوزیشن
telegram-button-sell-more = فروش بیشتر
telegram-button-more-dca = DCA بیشتر
telegram-button-history = تاریخچه
telegram-button-status = وضعیت
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = احراز هویت مجدد
telegram-button-previous = قبلی
telegram-button-next = بعدی
telegram-button-passed = تأییدشده
telegram-button-rejected = ردشده
telegram-button-new-24h = جدید (24h)
telegram-button-all-tokens = همه توکن‌ها
telegram-button-search-token = جست‌وجوی توکن
telegram-button-filter-stats = آمار فیلتر
telegram-button-refresh-stats = تازه‌سازی آمار
telegram-button-view-position = مشاهده پوزیشن
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    دستور ناشناخته: { $command }

    برای دیدن دستورهای موجود از /help استفاده کنید.
telegram-session-expired =
    <b>نشست منقضی شد</b>

    برای احراز هویت مجدد از /login استفاده کنید.
telegram-2fa-required =
    <b>2FA لازم است</b>

    لطفاً کد 6 رقمی برنامه احراز هویت خود را وارد کنید.
telegram-account-locked =
    <b>حساب قفل شد</b>

    تلاش‌های ناموفق بیش از حد.
    پس از { $seconds ->
        [one] { $seconds } ثانیه
       *[other] { $seconds } ثانیه
    } دوباره تلاش کنید.
telegram-code-invalid = لطفاً یک کد 6 رقمی معتبر وارد کنید.
telegram-authenticated =
    <b>احراز هویت شد!</b>

    اکنون به دستورهای ربات دسترسی دارید.
telegram-wrong-code =
    <b>کد نادرست</b>

    { $remaining ->
        [one] { $remaining } تلاش باقی مانده است.
       *[other] { $remaining } تلاش باقی مانده است.
    }
telegram-auth-required =
    <b>احراز هویت لازم است</b>

    لطفاً برای ادامه، رمز عبور خود را وارد کنید.

    <i>رمز عبور را بنویسید و ارسال کنید.</i>
telegram-login-required =
    <b>ورود لازم است</b>

    لطفاً کد 6 رقمی برنامه احراز هویت خود را وارد کنید:
telegram-session-activated =
    <b>نشست فعال شد</b>

    2FA پیکربندی نشده است. نشست شما اکنون فعال است.

    <i>نکته: برای امنیت بهتر، 2FA را در تنظیمات امنیتی فعال کنید.</i>

## Chat discovery.

telegram-discovery-hello = سلام { $name }!
telegram-discovery-default-name = کاربر
telegram-discovery-detected = <b>چت شناسایی شد!</b>
telegram-discovery-details =
    شناسه چت: <code>{ $chat_id }</code>
    نوع: { $chat_type }

    لطفاً به داشبورد { -brand } بروید و روی این چت کلیک کنید تا انتخاب شود.
telegram-chat-type-private = خصوصی
telegram-chat-type-group = گروه
telegram-chat-type-supergroup = سوپرگروه
telegram-chat-type-channel = کانال

## Menus.

telegram-menu-title =
    <b>پنل کنترل</b>

    برای مشاهده اطلاعات یا کنترل ربات، یک گزینه را انتخاب کنید.
telegram-menu-positions-empty =
    <b>پوزیشن بازی وجود ندارد</b>

    در انتظار فرصت‌های جدید...
telegram-menu-positions-title = <b>پوزیشن‌ها ({ $count })</b>
telegram-menu-positions-hint = <i>برای مدیریت یک پوزیشن روی آن بزنید.</i>
telegram-menu-settings =
    <b>تنظیمات</b>

    اعلان‌ها و پارامترهای معاملاتی را پیکربندی کنید.
telegram-settings-notifications =
    <b>تنظیمات اعلان</b>

    اعلان‌ها را روشن یا خاموش کنید:
telegram-settings-trading =
    <b>کنترل‌های معاملاتی</b>

    قابلیت‌های معاملاتی را روشن یا خاموش کنید:
telegram-pagination-expired = نشست صفحه‌بندی منقضی شد.

## Status commands.

telegram-status-state-stopped = <b>متوقف</b> (توقف اجباری فعال)
telegram-status-state-active = <b>فعال</b>
telegram-status-state-paused = <b>مکث</b>
telegram-status-on = فعال
telegram-status-off = غیرفعال
telegram-status-body =
    <b>وضعیت سیستم</b>

    <b>سیستم</b>
    وضعیت — { $state }
    مدت فعالیت — { $uptime }
    نسخه — v{ $version }

    <b>معاملات</b>
    ورودها — { $entries }
    خروج‌ها — { $exits }
    پوزیشن‌ها — { $positions }
telegram-positions-empty =
    <b>پوزیشن بازی وجود ندارد</b>

    در انتظار فرصت‌ها...
telegram-positions-title = <b>پوزیشن‌های باز ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } مورد دیگر...</i>
telegram-positions-summary =
    <b>خلاصه پورتفوی</b>
    سرمایه‌گذاری‌شده — { $invested } { -sol }
    سود و زیان خالص — { $pnl } { -sol }
telegram-balance-body =
    <b>موجودی کیف پول</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>آمار روزانه</b>

    پوزیشن‌ها — { $positions }
    سرمایه‌گذاری‌شده — { $invested } { -sol }
    سود و زیان — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } آماده است!</b>

    معاملات <b>فعال</b> است.

    برای کنترل ربات از کیبورد زیر استفاده کنید.
    برای دیدن دستورهای موجود، /help را بنویسید.
telegram-stop-already = <b>معاملات از قبل غیرفعال است</b>
telegram-stop-done =
    <b>معاملات غیرفعال شد</b>

    همه پایش‌های معاملاتی (ورود و خروج) متوقف شدند.
    برای توقف فقط ورودها از /pause استفاده کنید.
telegram-stop-failed =
    <b>غیرفعال‌سازی معاملات ناموفق بود</b>

    خطا: { $detail }
telegram-pause-done =
    <b>پایش ورود متوقف شد</b>

    پوزیشن جدیدی باز نمی‌شود.
    پایش خروج همچنان در حال اجراست.
telegram-pause-failed =
    <b>توقف ورودها ناموفق بود</b>

    خطا: { $detail }
telegram-resume-done =
    <b>پایش ورود ادامه یافت</b>

    اکنون سیگنال‌های ورود زیر نظر هستند.
telegram-resume-failed =
    <b>از سرگیری ورودها ناموفق بود</b>

    خطا: { $detail }
telegram-force-stop-confirm =
    <b>توقف اجباری</b>

    این کار فوراً همه فعالیت‌های معاملاتی را متوقف می‌کند:
    • بدون ورود جدید
    • بدون خروج (از جمله حد ضرر)
    • بدون عملیات DCA
telegram-force-stop-warning = <b>این یک اقدام اضطراری است!</b>
telegram-force-stop-question = مطمئن هستید؟
telegram-force-stop-active =
    <b>توقف اجباری فعال شد</b>

    همه معاملات متوقف شده است.

    برای لغو این وضعیت از /resume_trading استفاده کنید.
telegram-resume-trading-not-stopped =
    <b>معاملات در توقف اجباری نیست</b>

    نیازی به اقدام نیست.
telegram-resume-trading-done =
    <b>معاملات از سر گرفته شد</b>

    وضعیت توقف اجباری لغو شد.
    اکنون عملیات عادی معاملات می‌تواند ادامه یابد.

## Help.

telegram-help-title = <b>راهنمای { -brand }</b>
telegram-help-heading-dashboard = داشبورد
telegram-help-heading-market = بازار
telegram-help-heading-trading = معاملات
telegram-help-heading-safety = ایمنی
telegram-help-heading-system = سیستم
telegram-help-commands-dashboard =
    /status — وضعیت سیستم و مدت فعالیت
    /stats — عملکرد روزانه
    /balance — موجودی کیف پول
    /positions — پوزیشن‌های باز
telegram-help-commands-market =
    /tokens — اکسپلورر توکن
    /rejected — توکن‌های فیلترشده
telegram-help-commands-trading =
    /start — فعال‌سازی سیستم معاملات
    /stop — غیرفعال‌سازی سیستم معاملات
    /pause — توقف ورودهای جدید
    /resume — از سرگیری ورودهای جدید
    /menu — منوی تعاملی
telegram-help-commands-safety =
    /force_stop — <b>توقف اضطراری</b>
    /resume_trading — لغو وضعیت اضطراری
telegram-help-commands-system =
    /update — وضعیت و نصب به‌روزرسانی
    /login — احراز هویت 2FA
telegram-help-tip = <i>نکته: برای اجرای هر دستور، روی آن بزنید.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>به‌روز است</b>

    نسخه v{ $version } در حال اجراست و خودکار نصب شده است.
telegram-update-up-to-date =
    <b>به‌روز است</b>

    نسخه v{ $version } در حال اجراست.
telegram-update-check-failed =
    <b>بررسی به‌روزرسانی ناموفق بود</b>

    { $reason }
telegram-update-unreachable = دسترسی به screenerbot.io ممکن نشد.
telegram-update-installing = <b>در حال نصب v{ $version }</b>
telegram-update-restarting =
    { -brand } برای رفتن به نسخه جدید در حال راه‌اندازی مجدد است. معاملات خودکار از سر گرفته می‌شود.
telegram-update-install-failed =
    <b>نصب v{ $version } ممکن نشد</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } دانلود شد</b>

    این نسخه برنامه دسکتاپ را هم به‌روزرسانی می‌کند، بنابراین نصب‌کننده آن باید روی همان دستگاه اجرا شود. آنجا به تنظیمات ← به‌روزرسانی‌ها بروید.
telegram-update-downloading =
    <b>در حال دانلود v{ $version }</b>

    { $percent }% از { $size } MB.
telegram-update-available =
    <b>v{ $version } در دسترس است</b>

    { $how }
    حجم دانلود: { $size } MB.

    دانلود خودکار انجام می‌شود؛ پس از آماده شدن، دوباره /update را بفرستید.
telegram-update-how-core = بی‌صدا نصب می‌شود و یک راه‌اندازی مجدد کوتاه دارد.
telegram-update-how-installer = نصب‌کننده دسکتاپ باید یک بار اجرا شود.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = نامشخص
telegram-value-na = ندارد
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } ثانیه
telegram-duration-minutes = { $minutes } دقیقه
telegram-duration-minutes-seconds = { $minutes } دقیقه و { $seconds } ثانیه
telegram-duration-hours = { $hours } ساعت
telegram-duration-hours-minutes = { $hours } ساعت و { $minutes } دقیقه
telegram-duration-days = { $days } روز
telegram-duration-days-hours = { $days } روز و { $hours } ساعت
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = خطا: { $detail }
telegram-ai-reasoning =
    <b>تحلیل LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = ورود — { $price } { -sol }
telegram-row-exit = خروج — { $price } { -sol }
telegram-row-current = فعلی — { $price } { -sol }
telegram-row-invested = سرمایه‌گذاری‌شده — { $amount } { -sol }
telegram-row-received = دریافتی — { $amount } { -sol }
telegram-row-value = ارزش — { $amount } { -sol }
telegram-row-total = مجموع — { $amount } { -sol }
telegram-row-tokens = توکن‌ها — { $tokens }
telegram-row-duration = مدت — { $duration }
telegram-row-reason = دلیل — { $reason }
telegram-row-remaining = باقی‌مانده — { $percent }%
telegram-row-pnl = سود و زیان — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>پوزیشن باز شد</b>
telegram-notify-opened-size = اندازه — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = قیمت — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>پوزیشن بسته شد</b> — سود
telegram-notify-closed-title-loss = <b>پوزیشن بسته شد</b> — ضرر
telegram-notify-closed-reason-unspecified = بسته‌شده
telegram-notify-partial-title = <b>خروج جزئی</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }% فروخته شد
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = افزوده‌شده — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = میانگین — { $price } { -sol }
telegram-notify-severity-critical = <b>خطای بحرانی</b>
telegram-notify-severity-error = <b>خطا</b>
telegram-notify-severity-warning = <b>هشدار</b>
telegram-notify-severity-info = <b>اطلاعات</b>
telegram-notify-alert-title = <b>هشدار معامله</b>
telegram-notify-alert-token = توکن: <code>${ $symbol }</code>
telegram-notify-alert-mint = مینت: <code>{ $mint }</code>
telegram-notify-alert-bought = اقدام: خرید { $amount } { -sol }
telegram-notify-alert-sold = اقدام: فروش { $amount } { -sol }
telegram-notify-alert-wallet = کیف پول: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (آزمایشی)
telegram-notify-copy-task = وظیفه: { $task }
telegram-notify-scheduled-completed = <b>وظیفه زمان‌بندی‌شده تکمیل شد</b>
telegram-notify-scheduled-failed = <b>وظیفه زمان‌بندی‌شده ناموفق بود</b>
telegram-notify-scheduled-timed-out = <b>مهلت وظیفه زمان‌بندی‌شده تمام شد</b>
telegram-notify-scheduled-error = خطا: { $error }
telegram-notify-summary-title = <b>خلاصه روزانه</b> — { $date }
telegram-notify-summary-performance = <b>عملکرد</b>
telegram-notify-summary-trades = معاملات — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = نرخ برد — { $percent }%
telegram-notify-summary-pnl = سود و زیان — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = پوزیشن‌های باز — { $count }
telegram-notify-started-title = <b>{ -brand } شروع به کار کرد</b>
telegram-notify-started-version = <b>نسخه</b> — { $version }
telegram-notify-started-mode = <b>حالت</b> — { $mode }
telegram-notify-started-ready = آماده معامله!
telegram-notify-stopped-title = <b>{ -brand } متوقف شد</b>
telegram-notify-stopped-reason = <b>دلیل</b> — { $reason }
telegram-notify-stopped-goodbye = خداحافظ! { $icon }
telegram-notify-start-mode-normal = عادی
telegram-notify-stop-reason-graceful = خاموشی عادی
telegram-notify-update-available =
    <b>به‌روزرسانی v{ $version } در دسترس است</b>

    { $how }
    حجم دانلود: { $size } MB
telegram-notify-update-how-installer = این نسخه برنامه دسکتاپ را هم به‌روزرسانی می‌کند، بنابراین نصب‌کننده آن باید یک بار اجرا شود.
telegram-notify-update-ready =
    <b>به‌روزرسانی v{ $version } آماده است</b>

    { $how }
telegram-notify-update-ready-silent = برای اعمال فوری /update را بفرستید، وگرنه در اجرای بعدی { -brand } نصب می‌شود.
telegram-notify-update-ready-installer = برای اجرای نصب‌کننده، تنظیمات ← به‌روزرسانی‌ها را باز کنید.
telegram-notify-update-applying =
    <b>در حال نصب v{ $version }</b>

    بک‌اند در حال راه‌اندازی مجدد است؛ معاملات خودکار از سر گرفته می‌شود.
telegram-notify-new-tokens =
    <b>هشدار فیلترینگ</b>

    { $count ->
        [one] { $count } توکن جدید مطابق معیارهای شما پیدا شد.
       *[other] { $count } توکن جدید مطابق معیارهای شما پیدا شد.
    }
telegram-notify-crash =
    <b>ربات از کار افتاد!</b>

    <b>محل:</b> <code>{ $location }</code>
    <b>خطا:</b> <code>{ $error }</code>
telegram-notify-crash-restart = لطفاً ربات را دوباره راه‌اندازی کنید.

## Filter results page.

telegram-filter-results-title = <b>نتایج فیلتر</b> ({ $count })
telegram-filter-results-empty = <i>توکنی پیدا نشد.</i>
telegram-filter-results-page = <i>صفحه { $page } از { $total }</i>

## Position screens.

telegram-position-not-found = پوزیشن پیدا نشد
telegram-position-no-positions = پوزیشنی برای بستن وجود ندارد
telegram-position-history-empty =
    <b>تاریخچه معاملات</b>

    هنوز پوزیشن بسته‌شده‌ای وجود ندارد.
telegram-position-history-title = <b>معاملات اخیر</b>
telegram-position-history-more = <i>+{ $count } معامله دیگر...</i>
telegram-position-confirm-hint = <i>برای اجرا ظرف 30 ثانیه تأیید کنید.</i>
telegram-position-confirm-close-title = <b>پوزیشن بسته شود؟</b>
telegram-position-confirm-close-selling = فروش { $tokens } توکن
telegram-position-confirm-close-estimated = تخمینی — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>ظرف 30 ثانیه تأیید کنید</i>
telegram-position-confirm-sell =
    <b>تأیید فروش</b>

    توکن — { $symbol }
    مقدار — { $percent }%
    توکن‌ها — { $tokens }
telegram-position-confirm-dca =
    <b>تأیید خرید بیشتر</b>

    توکن — { $symbol }
    افزودن — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>همه پوزیشن‌ها بسته شوند؟</b>

    تعداد — { $count }
telegram-position-confirm-close-all-hint =
    <i>همه پوزیشن‌های باز با قیمت بازار فروخته می‌شوند.
    ظرف 30 ثانیه تأیید کنید.</i>
telegram-position-confirm-force-stop =
    <b>توقف اجباری</b>

    این کار فوراً همه معاملات را متوقف می‌کند:
    • بدون ورود جدید
    • بدون خروج
    • بدون DCA
telegram-position-confirm-force-stop-warning = <b>این یک اقدام اضطراری است.</b>
telegram-position-confirm-blacklist =
    <b>توکن به فهرست سیاه اضافه شود؟</b>

    توکن — { $symbol }
    مینت — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>پوزیشن بسته می‌شود و از ورودهای بعدی جلوگیری می‌شود.</i>
telegram-position-selling = در حال فروش { $percent }% از { $symbol }...
telegram-position-sell-done =
    <b>فروش انجام شد</b>

    توکن — { $symbol }
    فروخته‌شده — { $percent }%
    دریافتی — { $amount } { -sol }
telegram-position-sell-failed = <b>فروش ناموفق بود</b>
telegram-position-adding = در حال افزودن { $amount } { -sol } به { $symbol }...
telegram-position-dca-done =
    <b>DCA انجام شد</b>

    توکن — { $symbol }
    افزوده‌شده — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA ناموفق بود</b>
telegram-position-closing-all = در حال بستن همه پوزیشن‌ها...
telegram-position-close-all-done =
    <b>بستن همه تکمیل شد</b>

    بسته‌شده — { $closed }
    ناموفق — { $failed }
telegram-position-blacklisted =
    <b>توکن به فهرست سیاه اضافه شد</b>

    توکن — { $symbol }
    وضعیت — بسته‌شده و در فهرست سیاه

## Token screens.

telegram-token-not-found = توکن پیدا نشد
telegram-token-not-found-prefix = توکن پیدا نشد. با پیشوند طولانی‌تری جست‌وجو کنید.
telegram-token-stats-failed = دریافت آمار ناموفق بود: { $detail }
telegram-token-list-failed = دریافت توکن‌ها ناموفق بود: { $detail }
telegram-token-list-empty = توکنی در نمای <b>{ $view }</b> پیدا نشد.
telegram-token-view-passed = تأییدشده در فیلتر
telegram-token-view-rejected = ردشده
telegram-token-view-recent = تازه اضافه‌شده
telegram-token-view-all = همه توکن‌ها
telegram-token-list-title = <b>{ $name }</b> (صفحه { $page }/{ $total })
telegram-token-list-stats = نقدینگی: { $liquidity } • قیمت: { $price }
telegram-token-list-hint = <i>برای دیدن جزئیات، روی /token_ID بزنید</i>
telegram-token-explorer =
    <b>اکسپلورر بازار</b>

    <b>نمای کلی</b>
    تأییدشده در فیلتر — { $passed }
    ردشده — { $rejected }
    قیمت‌های فعال — { $priced }
    کل کشف‌شده‌ها — { $total }

    <i>برای مرور، یک دسته را انتخاب کنید:</i>
telegram-token-filter-title = <b>تحلیل فیلتر</b>
telegram-token-filter-distribution = <b>توزیع</b>
telegram-token-filter-passed = تأییدشده — { $count } ({ $percent }%)
telegram-token-filter-rejected = ردشده — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = فهرست سیاه — { $count }
telegram-token-filter-coverage = <b>پوشش</b>
telegram-token-filter-priced = دارای قیمت استخر — { $count }
telegram-token-filter-open = پوزیشن‌های باز — { $count }
telegram-token-filter-total = کل کشف‌شده‌ها — { $count }
telegram-token-filter-updated = <b>آخرین به‌روزرسانی</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>تازه‌سازی خودکار هر { $interval }</i>
telegram-token-detail-active = <b>پوزیشن فعال</b>
telegram-token-detail-price = قیمت — { $price } { -sol }
telegram-token-detail-liquidity = نقدینگی — { $value }
telegram-token-detail-volume = حجم 24h — { $value }
telegram-token-detail-change = تغییر 24h — { $value }
telegram-token-detail-risk = ارزیابی ریسک: { $score }/100
telegram-token-detail-risk-unknown = ارزیابی ریسک: نامشخص
telegram-token-detail-action = <i>اقدام را انتخاب کنید:</i>
telegram-token-search =
    <b>جست‌وجوی بازار</b>

    برای جست‌وجو، نماد یا آدرس مینت را وارد کنید:

    <i>مثال: /token_BONK یا /token_So11111</i>
telegram-token-confirm-buy =
    <b>تأیید خرید مستقیم</b>

    توکن — ${ $symbol }
    مینت — <code>{ $mint }</code>
    مقدار — { $amount } { -sol }

    <i>برای اجرا ظرف 30 ثانیه تأیید کنید.</i>
telegram-token-confirm-blacklist =
    <b>توکن به فهرست سیاه اضافه شود؟</b>

    توکن — ${ $symbol }
    مینت — <code>{ $mint }</code>

    <i>مانع از تأیید این توکن در فیلترها می‌شود.</i>
telegram-token-blacklisted =
    <b>توکن به فهرست سیاه اضافه شد</b>

    توکن — ${ $symbol }
    وضعیت — به فهرست سیاه اضافه شد
telegram-token-blacklist-failed = <b>افزودن به فهرست سیاه ناموفق بود</b>
telegram-token-buy-processing =
    <b>در حال پردازش خرید...</b>

    توکن — ${ $symbol }
    مقدار — { $amount } { -sol }
telegram-token-buy-done =
    <b>خرید موفق بود</b>

    توکن — ${ $symbol }
    مقدار — { $amount } { -sol }

    <i>جزئیات را در /positions ببینید</i>
telegram-token-buy-failed =
    <b>خرید ناموفق بود</b>

    توکن — ${ $symbol }
    خطا — { $detail }
