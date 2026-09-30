# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = کیف پول تغییر کرده است
startup-wallet-mismatch-detail =
    کیف پول موجود در پیکربندی شما با کیف پول ثبت‌شده در تاریخچه محلی این رایانه مطابقت ندارد.

    کیف پول فعلی: { $current }
    کیف پول قبلی: { $stored }

    داده‌های محلی تحت تأثیر: { $systems }

    این مشکل معمولاً پس از درون‌ریزی یک کلید خصوصی متفاوت یا بازیابی پیکربندی دیگری رخ می‌دهد. معاملات، پوزیشن‌ها و تاریخچه به کیف پول قبلی تعلق دارند و برای راه‌اندازی ایمن کیف پول جدید باید پاک شوند.
startup-wallet-mismatch-systems-default = تراکنش‌ها، پوزیشن‌ها، تاریخچه کیف پول
startup-wallet-mismatch-remedy =
    برای ادامه، تاریخچه محلی کیف پول قبلی را پاک کنید (ابتدا از پایگاه‌داده‌های شما خودکار نسخه پشتیبان تهیه می‌شود):

      - در برنامه: گزینه «{ $action }» را در پایین انتخاب کنید.
      - از ترمینال: دستور  screenerbot --clean-wallet-data  را اجرا کنید

    دارایی‌های روی زنجیره تحت تأثیر قرار نمی‌گیرند؛ فقط تاریخچه محلی معاملات و پوزیشن‌های این رایانه بازنشانی می‌شود. نسخه‌های پشتیبان در این مسیر ذخیره می‌شوند:
      { $path }
startup-recovery-reset-wallet = بازنشانی داده‌های کیف پول و راه‌اندازی مجدد

## Port in use.

startup-port-in-use-title = پورت شبکه اشغال است
startup-port-in-use-detail = پورت داشبورد { $address } از قبل در حال استفاده است.
startup-port-in-use-remedy = برنامه دیگری از پورتی که { -brand } نیاز دارد استفاده می‌کند. آن برنامه را ببندید یا پورت وب‌سرور را در تنظیمات تغییر دهید و سپس { -brand } را دوباره اجرا کنید.

## Another instance is running.

startup-lock-held-title = { -brand } از قبل در حال اجراست
startup-lock-held-detail = نسخه دیگری از { -brand } روی این رایانه در حال اجراست، بنابراین نسخه دوم نمی‌تواند شروع شود.
startup-lock-held-remedy = به پنجره‌ای که از قبل باز است بروید. اگر پنجره‌ای نمی‌بینید، هر فرایند پس‌زمینه { -brand } را ببندید و دوباره تلاش کنید. اگر پس از راه‌اندازی مجدد سیستم هم مشکل ادامه داشت، ممکن است فایل قفل منقضی باشد و می‌توانید آن را از پوشه داده حذف کنید (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = خواندن پیکربندی ممکن نشد
startup-config-parse-detail = تجزیه config.toml ممکن نشد: { $detail }
startup-config-load-parse-detail = بارگیری پیکربندی ناموفق بود: تجزیه config.toml ممکن نشد: { $detail }
startup-config-parse-remedy = فایل پیکربندی شما قابل خواندن نیست. یک نسخه پشتیبان را از پوشه داده بازیابی کنید یا پیکربندی را به حالت پیش‌فرض بازنشانی کنید و کیف پول و RPC را دوباره تنظیم کنید.
startup-config-load-parse-remedy = یک پیکربندی معتبر را بازیابی کنید یا راه‌اندازی را دوباره کامل کنید.
startup-option-invalid-title = گزینه راه‌اندازی نامعتبر
startup-option-invalid-remedy = یکی از گزینه‌های خط فرمان نامعتبر است. { -brand } را بدون آن گزینه اجرا کنید یا آن را اصلاح کنید و دوباره تلاش کنید.

## Generic failures.

startup-generic-title = { -brand } شروع نشد
startup-generic-remedy = برای جزئیات، فایل لاگ را بررسی کنید و سپس برنامه را دوباره اجرا کنید. اگر مشکل ادامه داشت، با پشتیبانی به آدرس t.me/screenerbotio_support تماس بگیرید.
startup-generic-detail = { $error }
startup-failure-directories = ساخت پوشه‌های موردنیاز ناموفق بود: { $error }
startup-failure-config-load = بارگیری پیکربندی ناموفق بود: { $error }
startup-failure-actions-init = راه‌اندازی پایگاه‌داده اقدام‌ها ناموفق بود: { $error }
startup-failure-actions-sync = همگام‌سازی اقدام‌ها از پایگاه‌داده ناموفق بود: { $error }
startup-failure-strategy-init = راه‌اندازی سیستم استراتژی ناموفق بود: { $error }
startup-failure-analysis-init = راه‌اندازی موتور تحلیل ناموفق بود: { $error }
startup-failure-assistant-init = راه‌اندازی موتور گفتگوی دستیار ناموفق بود: { $error }
startup-failure-wallets-init = راه‌اندازی کیف پول‌ها ناموفق بود: { $error }
startup-failure-wallet-validation = اعتبارسنجی سازگاری کیف پول ناموفق بود: { $error }
