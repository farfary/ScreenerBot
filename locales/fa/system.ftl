# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = پیکربندی درون حافظه با نسخه روی دیسک تفاوت دارد
system-result-config-matches = پیکربندی درون حافظه با نسخه روی دیسک یکسان است

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    { $count ->
        [one] { $count } بخش
       *[other] { $count } بخش
    } با موفقیت وارد شد
system-result-config-imported-with-warnings =
    { $count ->
        [one] { $count } بخش
       *[other] { $count } بخش
    } با { $warnings ->
        [one] { $warnings } هشدار
       *[other] { $warnings } هشدار
    } وارد شد: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = جستجوی تنظیمات...
system-config-export-title =
    .title = خروجی گرفتن از پیکربندی در فایل
system-config-import-title =
    .title = وارد کردن پیکربندی از فایل
system-config-reload = بارگذاری مجدد از دیسک
system-config-reset-defaults = بازنشانی به پیش‌فرض
system-config-select-section = یک بخش پیکربندی را انتخاب کنید
system-config-select-section-details = برای مشاهده جزئیات، یک بخش پیکربندی را انتخاب کنید.
system-config-no-metadata = فراداده‌ای برای <code>{ $section }</code> وجود ندارد
system-config-technical-settings = تنظیمات فنی
system-config-expand-title = باز کردن همه بخش‌ها و همه زیرپیکربندی‌های تودرتو
system-config-collapse-title = بستن همه بخش‌ها و همه زیرپیکربندی‌های تودرتو
system-config-toolbar-no-changes = بدون تغییر در بخش
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> تغییر در بخش
       *[other] <strong>{ $count }</strong> تغییر در بخش
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> تغییر در مجموع
       *[other] <strong>{ $count }</strong> تغییر در مجموع
    }

## Config page: state banner

system-config-loading = در حال بارگذاری پیکربندی…
system-config-refreshing = در حال به‌روزرسانی پیکربندی…
system-config-saving-title = در حال ذخیره تغییرات…
system-config-saving-detail = در حال به‌روزرسانی پیکربندی
system-config-validation-issues = <strong>مشکلات اعتبارسنجی شناسایی شد.</strong> لطفاً فیلدهای مشخص‌شده را بررسی کنید.

## Config page: section header and category chips

system-config-save-changes = ذخیره تغییرات
system-config-saving = در حال ذخیره…
system-config-compare = مقایسه با دیسک
system-config-revert-section = بازگردانی بخش
system-config-summary-critical = { $count } حیاتی
system-config-summary-performance = { $count } عملکردی
system-config-summary-pending =
    { $count ->
        [one] { $count } تغییر در انتظار
       *[other] { $count } تغییر در انتظار
    }
system-config-summary-none = خلاصه فراداده‌ای وجود ندارد
system-config-fields-count =
    { $count ->
        [one] { $count } فیلد
       *[other] { $count } فیلد
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · { $pending } در انتظار
system-config-chip-visible = { $visible } از { $fields }

## Config page: field rows

system-config-field-unit = واحد: { $unit }
system-config-field-default = پیش‌فرض: { $value }
system-config-field-reset = بازگشت به پیش‌فرض
system-config-array-invalid-title = مورد نامعتبر در آرایه
system-config-json-invalid-title = JSON نامعتبر
system-config-list-separator = { "، " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [one] خط { $lines } باید یک عدد صحیح معتبر باشد.
       *[other] خط‌های { $lines } باید عدد صحیح معتبر باشند.
    }
system-config-array-invalid-number =
    { $count ->
        [one] خط { $lines } باید یک عدد معتبر باشد.
       *[other] خط‌های { $lines } باید عدد معتبر باشند.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] خط { $lines } باید یک مقدار منطقی معتبر باشد.
       *[other] خط‌های { $lines } باید مقدار منطقی معتبر باشند.
    }
system-config-array-invalid-value =
    { $count ->
        [one] خط { $lines } باید یک مقدار معتبر باشد.
       *[other] خط‌های { $lines } باید مقدار معتبر باشند.
    }

## Config page: Telegram actions

system-config-telegram-actions = عملیات
system-config-telegram-test-title = آزمایش اتصال
system-config-telegram-test-description = یک پیام آزمایشی بفرستید تا از درستی پیکربندی { -telegram } مطمئن شوید
system-config-telegram-send-test = ارسال پیام آزمایشی
system-config-telegram-sending = در حال ارسال...
system-config-telegram-configure-token-title = ابتدا توکن ربات را تنظیم کنید
system-config-telegram-configure-token-status = برای فعال شدن آزمایش، توکن ربات را در بالا تنظیم کنید
system-config-telegram-test-sent-status = پیام آزمایشی با موفقیت ارسال شد! { -telegram } خود را بررسی کنید.
system-config-telegram-test-sent = پیام آزمایشی { -telegram } ارسال شد
system-config-telegram-test-failed = ارسال پیام آزمایشی ناموفق بود
system-config-telegram-auth-title = احراز هویت ربات
system-config-telegram-totp-title = احراز هویت دومرحله‌ای (TOTP)
system-config-telegram-totp-configured = تنظیم‌شده
system-config-telegram-totp-not-configured = تنظیم‌نشده
system-config-telegram-totp-active = احراز هویت دومرحله‌ای فعال است. نشست‌های منقضی‌شده { -telegram } به کد TOTP از برنامه احراز هویت شما نیاز دارند.
system-config-telegram-totp-inactive = برای محافظت از دستورهای { -telegram }، احراز هویت دومرحله‌ای را در تنظیمات امنیت فعال کنید.
system-config-telegram-totp-note = TOTP با قفل صفحه داشبورد مشترک است. آن را در تنظیمات امنیت پیکربندی کنید.
system-config-telegram-require-2fa = الزام 2FA برای دستورها
# $status is the HTTP status code.
system-config-telegram-save-rejected = ذخیره رد شد ({ $status })
system-config-telegram-save-failed = ذخیره تنظیم { -telegram } ممکن نشد

## Config page: operations

system-config-saved = پیکربندی ذخیره شد
system-config-save-failed = ذخیره پیکربندی ممکن نشد
system-config-reloaded = پیکربندی از دیسک دوباره بارگذاری شد
system-config-reload-failed = بارگذاری مجدد پیکربندی ممکن نشد
system-config-diff-title = تفاوت پیکربندی
system-config-diff-console = در کنسول مرورگر نوشته شد
system-config-diff-failed = محاسبه تفاوت ممکن نشد
system-config-reset-title = بازنشانی پیکربندی
system-config-reset-message =
    این کار کل پیکربندی را به مقادیر پیش‌فرض داخلی بازنشانی می‌کند. همه تنظیمات فعلی از بین می‌روند.

    این کار قابل بازگشت نیست.
system-config-reset-done-title = پیکربندی بازنشانی شد
system-config-reset-done-message = همه تنظیمات به مقادیر پیش‌فرض بازگردانده شد
system-config-reset-failed = بازنشانی پیکربندی ممکن نشد
system-config-load-failed = بارگذاری پیکربندی ممکن نشد
system-config-metadata-failed = بارگذاری فراداده پیکربندی ممکن نشد

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = بستن
system-config-select-none = لغو انتخاب همه
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } تغییر
       *[other] { $count } تغییر
    }
system-config-sections-count =
    { $count ->
        [one] { $count } بخش
       *[other] { $count } بخش
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-rpc = اندپوینت‌های RPC و تنظیمات اتصال
system-config-section-hint-trader = قوانین معاملات و خودکارسازی
system-config-section-hint-positions = تنظیمات مدیریت پوزیشن
system-config-section-hint-filtering = قوانین و آستانه‌های فیلتر توکن
system-config-section-hint-swaps = تنظیمات اجرای سواپ
system-config-section-hint-tokens = کشف توکن و منابع داده
system-config-section-hint-sol-price = پیکربندی سرویس قیمت { -sol }
system-config-section-hint-events = تنظیمات ثبت رویدادها
system-config-section-hint-services = تنظیمات سرویس‌های پس‌زمینه
system-config-section-hint-monitoring = پیکربندی پایش سیستم
system-config-section-hint-ohlcv = تنظیمات داده‌های کندل
system-config-section-hint-gui = تنظیمات داشبورد و رابط کاربری
system-config-section-hint-telegram = پیکربندی ربات { -telegram }

## Export dialog

system-config-export-dialog-title = خروجی گرفتن از پیکربندی
system-config-export-intro = بخش‌هایی از پیکربندی را که می‌خواهید صادر شوند انتخاب کنید. فایل خروجی را بعداً می‌توان برای بازیابی یا اشتراک‌گذاری تنظیمات وارد کرد.
system-config-export-sections = بخش‌ها
system-config-export-timestamp = درج زمان خروجی
system-config-sections-selected =
    { $count ->
        [one] { $count } بخش انتخاب شد
       *[other] { $count } بخش انتخاب شد
    }
system-config-exporting = در حال خروجی گرفتن...
system-config-export-invalid-response = پاسخ نامعتبر از سرور
system-config-exported-title = پیکربندی صادر شد
system-config-exported-message =
    { $count ->
        [one] { $count } بخش صادر شد
       *[other] { $count } بخش صادر شد
    }
system-config-export-failed-title = خروجی گرفتن ناموفق بود
system-config-export-failed = خروجی گرفتن از پیکربندی ناموفق بود

## Import dialog

system-config-import-dialog-title = وارد کردن پیکربندی
system-config-import-upload-intro = فایل پیکربندی که قبلاً صادر کرده‌اید را بارگذاری کنید. می‌توانید پیش‌نمایش ببینید و بخش‌های موردنظر برای وارد شدن را انتخاب کنید.
system-config-import-dropzone-title = فایل پیکربندی را اینجا رها کنید
system-config-import-dropzone-hint = یا برای انتخاب کلیک کنید
system-config-import-analyzing = در حال تحلیل پیکربندی...
system-config-import-preview = پیش‌نمایش
system-config-import-preview-intro = بخش‌های پیکربندی زیر را بررسی کنید. بخش‌هایی را که باید وارد شوند انتخاب کنید.
system-config-import-sections = بخش‌های فایل
system-config-import-select-valid = انتخاب همه بخش‌های معتبر
system-config-import-merge-label = ادغام با موجود
system-config-import-merge-hint = فقط فیلدهای موجود در فایل به‌روزرسانی می‌شوند. بدون تیک = جایگزینی کل بخش‌ها.
system-config-import-save-label = ذخیره روی دیسک
system-config-import-save-hint = ماندگار کردن تغییرات در config.toml پس از وارد کردن
system-config-import-selected = وارد کردن موارد انتخاب‌شده
system-config-import-warnings =
    { $count ->
        [one] { $count } هشدار
       *[other] { $count } هشدار
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = بخش ناشناخته «{ $section }» نادیده گرفته می‌شود
system-config-import-warning-sensitive-field = وارد کردن { $field } ممکن است تنظیمات احراز هویت را بازنویسی کند
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = در فایل نیست
system-config-import-status-invalid = پیکربندی نامعتبر
system-config-import-status-unchanged = بدون تغییر
system-config-import-not-included = در فایل گنجانده نشده است
system-config-import-show-changes = نمایش تغییرات
system-config-import-hide-changes = پنهان کردن تغییرات
system-config-import-value-current = مقدار فعلی
system-config-import-value-new = مقدار جدید
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } تغییر دیگر
       *[other] +{ $count } تغییر دیگر
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } مورد
       *[other] { $count } مورد
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } کلید
       *[other] { $count } کلید
    }{ "}" }
system-config-importing = در حال وارد کردن...
system-config-import-failed = وارد کردن ناموفق بود
system-config-import-invalid-file-title = فایل نامعتبر
system-config-import-invalid-file = خواندن فایل پیکربندی ناموفق بود
system-config-imported-title = پیکربندی وارد شد
system-config-imported-message =
    { $count ->
        [one] { $count } بخش وارد شد
       *[other] { $count } بخش وارد شد
    }
system-config-import-failed-title = وارد کردن ناموفق بود
system-config-import-failed-message = وارد کردن پیکربندی ناموفق بود
