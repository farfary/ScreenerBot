# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = نصب خودکار غیرفعال است. به‌روزرسانی آماده است و هر زمان که بخواهید اعمال می‌شود.
updates-defer-trading-active = یک پوزیشن، معامله یا عملیات ابزار در حال انجام است، بنابراین راه‌اندازی مجدد به تعویق افتاده است. به‌روزرسانی هنگام بیکار بودن برنامه به‌طور خودکار اعمال می‌شود.
updates-defer-needs-installer = این نسخه پوسته دسکتاپ را هم به‌روزرسانی می‌کند، بنابراین نصب‌کننده باید یک بار اجرا شود.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = در حال دانلود به‌روزرسانی v{ $version }...
updates-apply-started = در حال نصب به‌روزرسانی. { -brand } به‌طور خودکار دوباره راه‌اندازی و متصل می‌شود.
updates-install-opened = نصب‌کننده تأییدشده به‌روزرسانی باز شد. نصب‌کننده سیستم‌عامل را تکمیل کنید.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = نصب‌کننده باز شد
updates-installer-toast-message = { -brand } اکنون به‌صورت ایمن بسته می‌شود.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = وضعیت
updates-tab-release-notes = یادداشت‌های نسخه
updates-tab-preferences = ترجیحات
updates-tab-sections = بخش‌های به‌روزرسانی
updates-checking-installation = در حال بررسی این نصب...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = آماده بررسی به‌روزرسانی
updates-phase-idle-detail = { -brand } نسخه v{ $version } نصب شده است.
updates-phase-up-to-date-headline = شما به‌روز هستید
updates-phase-up-to-date-detail = { -brand } نسخه v{ $version } آخرین نسخه است.
updates-phase-checking-headline = در حال بررسی به‌روزرسانی
updates-phase-checking-detail = در حال جستجوی آخرین نسخه منتشرشده.
updates-phase-available-headline = نسخه { $version } در دسترس است
updates-phase-downloading-headline = در حال دانلود v{ $version }
updates-phase-verifying-headline = در حال تأیید v{ $version }
updates-phase-verifying-detail = در حال مقایسه فایل دانلودشده با چک‌سام منتشرشده.
updates-phase-ready-to-apply-headline = نسخه { $version } آماده است
updates-phase-ready-to-apply-detail = به‌روزرسانی را می‌توان اکنون با یک راه‌اندازی مجدد کوتاه نصب کرد یا در شروع بعدی به‌طور خودکار اعمال می‌شود.
updates-phase-ready-to-install-headline = نسخه { $version } آماده است
updates-phase-ready-to-install-detail = نصب‌کننده دسکتاپ برای تکمیل این به‌روزرسانی آماده است.
updates-phase-applying-headline = در حال نصب به‌روزرسانی
updates-phase-applying-detail = { -brand } در حال راه‌اندازی مجدد روی نسخه جدید است.
updates-phase-applied-headline = به v{ $version } به‌روزرسانی شد
updates-phase-applied-detail = به‌روزرسانی نصب شد. کار دیگری لازم نیست.
updates-phase-failed-headline = به‌روزرسانی کامل نشد
updates-phase-failed-detail = به‌روزرسانی را دوباره امتحان کنید.
updates-phase-check-failed-headline = بررسی به‌روزرسانی ممکن نشد
updates-phase-check-failed-detail = اتصال به سرویس انتشار ممکن نبود.
updates-status-unavailable-headline = وضعیت به‌روزرسانی در دسترس نیست
updates-phase-unrecognized-detail = وضعیت گزارش‌شده برای به‌روزرسانی شناخته نمی‌شود.
updates-status-load-failed-detail = بارگذاری وضعیت نصب ممکن نشد.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = به‌روزرسانی هسته · { $size } · راه‌اندازی مجدد کوتاه
updates-kind-full = به‌روزرسانی دسکتاپ · { $size } · نیازمند نصب‌کننده
updates-size-unknown = حجم نامشخص

updates-action-check-now = بررسی اکنون
updates-action-check-again = بررسی دوباره
updates-action-try-again = تلاش دوباره
updates-action-download = دانلود به‌روزرسانی
updates-action-restart = راه‌اندازی مجدد برای به‌روزرسانی
updates-action-open-installer = باز کردن نصب‌کننده

updates-busy-checking = در حال بررسی...
updates-busy-resuming = در حال ازسرگیری دانلود...
updates-busy-starting-download = در حال شروع دانلود...
updates-busy-restarting = در حال راه‌اندازی مجدد...
updates-busy-opening-installer = در حال باز کردن نصب‌کننده...

updates-progress-downloading = در حال دانلود به‌روزرسانی
updates-progress-verifying = در حال تأیید به‌روزرسانی
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } از { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }، { $percent }، { $transferred }

updates-detail-list-label = جزئیات نصب
updates-detail-installed-version = نسخه نصب‌شده
updates-detail-system = سیستم
updates-detail-last-checked = آخرین بررسی
updates-detail-never = هرگز
updates-detail-download-size = حجم دانلود

updates-version-installed = نصب‌شده
updates-version-available = در دسترس

updates-notes-highlights = نکات برجسته
updates-notes-empty-title = هنوز یادداشتی برای نسخه‌ها وجود ندارد
updates-notes-empty-error = بارگذاری تاریخچه نسخه‌ها ممکن نشد. اتصال خود را بررسی کنید و دوباره تلاش کنید.
updates-notes-empty-none = پس از انتشار یک نسخه، یادداشت‌های آن اینجا نمایش داده می‌شود.
updates-notes-history-notice = آنچه این نصب از قبل می‌داند نمایش داده می‌شود — بارگذاری تاریخچه نسخه‌ها ممکن نشد.
updates-release-empty = تغییری برای این نسخه فهرست نشده است.
updates-release-changes =
    { $count ->
        [one] { $count } تغییر
       *[other] { $count } تغییر
    }

updates-preferences-unavailable-title = ترجیحات به‌روزرسانی در دسترس نیست
updates-preferences-unavailable-detail = بارگذاری پیکربندی به‌روزرسانی ممکن نشد.
updates-preference-fallback-name = ترجیح به‌روزرسانی
updates-preference-save-failed = ذخیره { $preference } ممکن نشد

updates-request-failed = درخواست ناموفق بود
updates-check-request-failed = بررسی به‌روزرسانی ممکن نشد
updates-resume-failed = ازسرگیری دانلود به‌روزرسانی ممکن نشد
updates-download-failed = شروع دانلود به‌روزرسانی ممکن نشد
updates-apply-failed = نصب به‌روزرسانی ممکن نشد
updates-install-failed = باز کردن نصب‌کننده به‌روزرسانی ممکن نشد
updates-apply-confirm-title = نصب v{ $version }
updates-apply-confirm-message = { -brand } روی نسخه جدید راه‌اندازی مجدد می‌شود. معاملات چند ثانیه متوقف می‌شوند و به‌طور خودکار ادامه می‌یابند؛ پوزیشن‌های باز دست‌نخورده می‌مانند.
updates-install-confirm-title = اجرای نصب‌کننده
updates-install-confirm-message = نصب‌کننده تأییدشده باز می‌شود و { -brand } به‌صورت ایمن بسته می‌شود. نصب‌کننده را تکمیل کنید و سپس { -brand } را دوباره باز کنید.

# A release version as displayed.
updates-version-number = v{ $version }

# The Home update notice, shown while a release is in play.
updates-notice-region =
    .aria-label = وضعیت به‌روزرسانی
updates-notice-view = مشاهده به‌روزرسانی
updates-notice-whats-new = تازه‌ها
updates-notice-available-detail = تغییرات را ببینید و آن را از تنظیمات نصب کنید.
updates-notice-updated-detail = ببینید در این نسخه چه چیزهایی تغییر کرده است.
# A headless installation cannot download or install a release itself.
updates-headless-install-detail = نصب‌های بدون رابط کاربری بیرون از داشبورد به‌روزرسانی می‌شوند. در Linux، { "screenerbot-manager update" } را اجرا کنید.
