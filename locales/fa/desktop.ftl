# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = تأیید

## Splash and loading status.

desktop-splash-starting = در حال راه‌اندازی { -brand }
desktop-splash-restarting = در حال راه‌اندازی مجدد { -brand }
desktop-splash-recovering = در حال بازیابی
desktop-splash-opening-dashboard = در حال باز کردن داشبورد
desktop-splash-checking-dependencies = در حال بررسی وابستگی‌ها
desktop-splash-installing-dependencies = در حال نصب وابستگی‌های سیستم
desktop-splash-installing-dependencies-detail = { -brand } برای اجرا به Microsoft Visual C++ Redistributable نیاز دارد.
desktop-splash-resetting-wallet = در حال بازنشانی داده‌های کیف پول
desktop-splash-resetting-wallet-detail = داده‌های کیف پول موجود پیش از پاک شدن پشتیبان‌گیری می‌شوند.
desktop-splash-updating = در حال به‌روزرسانی به v{ $version }
desktop-splash-updating-detail = تنظیمات و داده‌های شما دقیقاً همان‌طور که هست باقی می‌مانند.
desktop-splash-restoring = در حال بازگرداندن v{ $version }
desktop-splash-restoring-detail = به‌روزرسانی v{ $failed } اجرا نشد، بنابراین نسخه قبلی جایگزین می‌شود.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = راه‌اندازی { -brand } ممکن نشد
desktop-boot-detail-fallback = هسته پشتیبان به‌طور غیرمنتظره متوقف شد.
desktop-boot-remedy-label = راه‌حل
desktop-boot-log-file-label = فایل لاگ:
desktop-boot-action-reset-wallet = بازنشانی داده‌های کیف پول و راه‌اندازی مجدد
desktop-boot-action-working = در حال انجام...
desktop-boot-action-open-logs = باز کردن پوشه لاگ‌ها
desktop-boot-action-copy = کپی جزئیات
desktop-boot-action-copied = کپی شد
desktop-boot-action-quit = خروج
desktop-boot-subtitle-wallet-mismatch = کیف پول متفاوتی شناسایی شد
desktop-boot-subtitle-port-in-use = یک پورت شبکه موردنیاز مشغول است
desktop-boot-subtitle-lock-held = { -brand } از قبل در حال اجراست
desktop-boot-subtitle-config-invalid = مشکل در پیکربندی
desktop-boot-subtitle-directory-setup = مشکل در فضای ذخیره‌سازی
desktop-boot-subtitle-generic = خطای راه‌اندازی

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = راه‌اندازی { -brand } ممکن نشد
desktop-boot-error-remedy = برای دیدن علت مشکل، پوشه لاگ‌ها را باز کنید و سپس برنامه را دوباره راه‌اندازی کنید. اگر مشکل ادامه داشت، با پشتیبانی از طریق t.me/screenerbotio_support تماس بگیرید.
desktop-boot-error-default = هسته پشتیبان پیش از آماده شدن داشبورد به‌طور غیرمنتظره متوقف شد.
desktop-boot-error-restore-failed = هسته پشتیبان به‌روزرسانی‌شده با خطا مواجه شد و نسخه قبلی بازگردانی نشد ({ $error }).
desktop-boot-error-spawn-failed = اجرای برنامه هسته پشتیبان ممکن نشد ({ $error }).
desktop-boot-error-spawn-missing = اجرای برنامه هسته پشتیبان ممکن نشد. ممکن است وجود نداشته باشد یا نرم‌افزار امنیتی آن را مسدود کرده باشد.
desktop-boot-error-exited-running = هسته پشتیبان در حین اجرای داشبورد متوقف شد (کد خروج { $code }).
desktop-boot-error-exited-early = هسته پشتیبان پیش از آماده شدن داشبورد متوقف شد (کد خروج { $code }).
desktop-boot-error-dashboard-load = بارگذاری داشبورد ناموفق بود ({ $description }، { $code }).
desktop-boot-error-renderer-gone = رندرکننده داشبورد متوقف شد ({ $reason }).
desktop-boot-error-unresponsive = داشبورد پاسخگو نبود.
desktop-boot-error-url-failed = بارگذاری نشانی داشبورد ممکن نشد ({ $error }).
desktop-boot-error-relaunch-setup = اجرای دوباره هسته پشتیبان پس از راه‌اندازی ممکن نشد.
desktop-boot-error-relaunch-recovery = اجرای دوباره هسته پشتیبان برای بازیابی ممکن نشد.
desktop-boot-error-restart-offline = هسته پشتیبان پس از راه‌اندازی مجدد دوباره آنلاین نشد.
desktop-boot-error-recovery-offline = بازیابی تمام شد اما هسته پشتیبان آماده نشد.
desktop-boot-error-start-timeout = هسته پشتیبان در زمان مقرر راه‌اندازی نشد. این مشکل ممکن است در اولین اجرای کند یا هنگام مسدود شدن اتصال توسط برنامه‌ای دیگر رخ دهد.

## System tray.

desktop-tray-tooltip = { -brand } - ربات معاملاتی Solana
desktop-tray-show = نمایش { -brand }
desktop-tray-open-dashboard = باز کردن داشبورد
desktop-tray-quit = خروج از { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = باز کردن پوشه داده‌ها
desktop-menu-open-logs-folder = باز کردن پوشه لاگ‌ها
desktop-menu-documentation = مستندات
desktop-menu-telegram-support = پشتیبانی { -telegram }
desktop-menu-check-updates = بررسی به‌روزرسانی...

## Application menu.

desktop-menu-file = فایل
desktop-menu-edit = ویرایش
desktop-menu-view = نمایش
desktop-menu-window = پنجره
desktop-menu-help = راهنما
desktop-menu-reset-zoom = بازنشانی بزرگ‌نمایی
desktop-menu-zoom-in = بزرگ‌نمایی
desktop-menu-zoom-out = کوچک‌نمایی
desktop-menu-keyboard-shortcuts = میان‌برهای صفحه‌کلید
desktop-menu-telegram-channel = کانال { -telegram }
desktop-menu-telegram-community = انجمن { -telegram }
desktop-menu-follow-x = دنبال کردن در { -x } ({ -twitter })
desktop-menu-visit-website = بازدید از وب‌سایت
desktop-menu-about = درباره { -brand }

## About dialog.

desktop-about-title = درباره { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    نسخه { $version }

    ربات پیشرفته مدیریت کیف پول و معاملات خودکار Solana.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = میان‌برهای صفحه‌کلید
desktop-shortcuts-message = میان‌برهای صفحه‌کلید { -brand }
desktop-shortcuts-body-mac =
    میان‌برهای صفحه‌کلید:

    کنترل پنجره:
      Cmd+M          کوچک کردن
      Cmd+W          بستن پنجره
      Cmd+Q          خروج
      Cmd+Ctrl+F     تمام‌صفحه

    بزرگ‌نمایی:
      Cmd++          بزرگ‌نمایی
      Cmd+-          کوچک‌نمایی
      Cmd+0          بازنشانی بزرگ‌نمایی

    پیمایش:
      Cmd+R          بارگذاری مجدد داشبورد
      Cmd+Shift+D    باز کردن پوشه داده‌ها

    سایر:
      F1             باز کردن مستندات
      Cmd+Alt+I      ابزار توسعه‌دهنده
desktop-shortcuts-body-other =
    میان‌برهای صفحه‌کلید:

    کنترل پنجره:
      Alt+F4         خروج
      F11            تمام‌صفحه

    بزرگ‌نمایی:
      Ctrl++         بزرگ‌نمایی
      Ctrl+-         کوچک‌نمایی
      Ctrl+0         بازنشانی بزرگ‌نمایی

    پیمایش:
      Ctrl+R         بارگذاری مجدد داشبورد
      Ctrl+Shift+D   باز کردن پوشه داده‌ها

    سایر:
      F1             باز کردن مستندات
      Ctrl+Shift+I   ابزار توسعه‌دهنده

## Close confirmation (Windows and Linux).

desktop-close-title = بستن { -brand }
desktop-close-message = چه کاری می‌خواهید انجام دهید؟
desktop-close-detail = { -brand } می‌تواند در پس‌زمینه به اجرا ادامه دهد. ربات معامله‌گر در حالت کوچک‌شده در سینی سیستم نیز به پایش و معامله ادامه می‌دهد.
desktop-close-minimize = کوچک کردن به سینی سیستم
desktop-close-quit = خروج کامل
desktop-close-cancel = لغو

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = وابستگی ناموجود
desktop-vcredist-missing-message = Visual C++ Redistributable نصب نشده است
desktop-vcredist-missing-detail = { -brand } برای اجرا به Microsoft Visual C++ Redistributable نیاز دارد. آیا می‌خواهید اکنون آن را نصب کنید؟
desktop-vcredist-install = نصب و رفع مشکل
desktop-vcredist-exit = خروج
desktop-vcredist-not-found-title = نصب‌کننده پیدا نشد
desktop-vcredist-not-found-message = { $name } به‌درستی پیدا نشد.
desktop-vcredist-done-title = نصب کامل شد
desktop-vcredist-done-message = وابستگی‌ها با موفقیت نصب شدند.
desktop-vcredist-done-detail = { -brand } اکنون اجرا می‌شود.
desktop-vcredist-failed-title = نصب ناموفق بود
desktop-vcredist-failed-message = لطفاً Visual C++ Redistributable را به‌صورت دستی نصب کنید.
