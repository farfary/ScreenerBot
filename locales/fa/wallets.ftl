# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = ساخته‌شده
wallets-type-imported = واردشده
wallets-type-migrated = منتقل‌شده

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = توقف‌شده توسط شما
wallets-watch-disabled-signature-budget = متوقف شد: پیش از رسیدن به آخرین وضعیت، سقف بررسی { $limit } امضا پر شد
wallets-watch-disabled-unknown = متوقف شد: دلیل ایمنی ذخیره‌شده برای پایش قابل خواندن نبود
wallets-watch-disabled-helius-unavailable = متوقف شد: ارائه‌دهنده ویژه فعالیت بالا در دسترس نیست؛ نشانگر پیشرفت حفظ شد
wallets-watch-disabled-processing-failed = متوقف شد: پردازش فعالیت کیف پول ممکن نشد؛ نشانگر پیشرفت حفظ شد

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = ارائه‌دهنده ویژه فعالیت بالا در دسترس نیست؛ پایش متوقف شد
wallets-watch-error-provider-repeated-failure = بررسی‌های { -helius } بارها ناموفق بود؛ پایش متوقف شد
wallets-watch-error-processing-repeated-failure = پردازش فعالیت کیف پول بارها ناموفق بود؛ پایش متوقف شد
wallets-watch-error-position-unreadable = پایش کیف پول نتوانست موقعیت ذخیره‌شده خود را بخواند؛ تلاش دوباره
wallets-watch-error-provider-check-failed = بررسی ارائه‌دهنده ویژه فعالیت بالا ناموفق بود؛ تلاش دوباره
wallets-watch-error-decode-failed = رمزگشایی تراکنش فعالیت بالا ممکن نشد؛ نشانگر پیشرفت حفظ شد
wallets-watch-error-processing-failed = پردازش فعالیت کیف پول ممکن نشد؛ تلاش دوباره
wallets-watch-error-position-save-failed = پایش کیف پول نتوانست موقعیت خود را ذخیره کند؛ تلاش دوباره

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = توقف‌شده توسط شما.
wallets-watch-reason-signature-budget = فعالیت این کیف پول بیشتر از حدی است که پایش فعلی می‌تواند بررسی کند.
wallets-watch-reason-helius-unavailable = بررسی‌های { -helius } ناموفق بود. پیشرفت ذخیره‌شده حفظ شده است.
wallets-watch-reason-processing-failed = پردازش فعالیت کیف پول ممکن نشد. پیشرفت ذخیره‌شده حفظ شده است.

# Vocabulary shared by the wallet tables and dialogs.
wallets-field-address = آدرس
wallets-field-name = نام کیف پول
wallets-field-notes = یادداشت
wallets-field-private-key = کلید خصوصی
wallets-address-copy = کپی آدرس
wallets-modal-close =
    .aria-label = بستن پنجره
wallets-this-wallet = این کیف پول
wallets-summary-sol = { -sol }
wallets-copied-address = آدرس
wallets-copied-mint = آدرس مینت
wallets-copied-private-key = کلید خصوصی

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = کیف پول اصلی
wallets-tab-secondaries = کیف پول‌های ثانویه
wallets-tab-archive = بایگانی
wallets-tab-watched = پایش‌شده
wallets-refresh-failed = به‌روزرسانی کیف پول‌ها ممکن نشد
wallets-action-failed = ناموفق
wallets-toast-failed = ناموفق: { $reason }
wallets-create-busy = در حال ساخت...
wallets-create-fallback = ساخت ناموفق بود
wallets-create-done = کیف پول «{ $name }» ساخته شد!
wallets-import-busy = در حال وارد کردن...
wallets-import-failed = وارد کردن ناموفق بود
wallets-import-done = کیف پول «{ $name }» وارد شد!
wallets-archive-busy = در حال بایگانی...
wallets-archive-confirm-text = آیا از بایگانی <strong>{ $name }</strong> مطمئن هستید؟
wallets-archive-done = کیف پول بایگانی شد
wallets-restore-done = کیف پول بازگردانده شد
wallets-export-busy = در حال رمزگشایی...
wallets-export-revealed = کلید نمایش داده شد - با احتیاط نگهداری کنید
wallets-delete-busy = در حال حذف...
wallets-delete-confirm-text = آیا از حذف <strong>{ $name }</strong> مطمئن هستید؟
wallets-delete-done = کیف پول برای همیشه حذف شد

# wallets.html: Add Wallet dialog.
wallets-add-title = افزودن کیف پول
wallets-add-tab-create = ساخت جدید
wallets-add-tab-import = وارد کردن موجود
wallets-create-name-input =
    .placeholder = مثلاً کیف پول معاملات
wallets-create-name-hint = نامی ساده برای شناسایی این کیف پول
wallets-create-notes-input =
    .placeholder = توضیح یا هدف (اختیاری)...
wallets-create-submit = ساخت کیف پول
wallets-import-warning-title = هشدار امنیتی
wallets-import-warning-body = کلید خصوصی را فقط از منابع معتبر وارد کنید. کلید شما رمزگذاری می‌شود و به‌صورت امن روی همین دستگاه ذخیره خواهد شد.
wallets-import-name-input =
    .placeholder = مثلاً کیف پول من
wallets-import-key-input =
    .placeholder = رشته Base58 یا آرایه JSON مانند ⁨[1,2,3,...]⁩
wallets-import-key-toggle =
    .aria-label = نمایش یا پنهان کردن کلید خصوصی
wallets-import-key-hint = کلید رمزگذاری‌شده با Base58 یا قالب آرایه بایت پشتیبانی می‌شود
wallets-import-notes-input =
    .placeholder = توضیح (اختیاری)...
wallets-import-submit = وارد کردن کیف پول

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = پایش کیف پول
wallets-watch-add-address = آدرس کیف پول
wallets-watch-add-address-input =
    .placeholder = آدرس Solana
wallets-watch-add-address-hint = فعالیت روی زنجیره این کیف پول را ثبت می‌کند و هشدار معاملات را طبق تنظیمات { -telegram } شما ارسال می‌کند.
wallets-watch-add-label = برچسب
wallets-watch-add-label-input =
    .placeholder = نام (اختیاری)
wallets-watch-add-submit = افزودن پایش

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = گزینه‌های پایش کیف پول
wallets-watch-budget-title-restore = بازگرداندن پایش کیف پول
wallets-watch-budget-close =
    .aria-label = بستن
wallets-watch-budget-label-signatures = امضاهای بررسی‌شده در هر بررسی
wallets-watch-budget-label-transactions = تراکنش‌های کامل موفق بررسی‌شده در هر بررسی
wallets-watch-budget-hint-signatures = سقف فعلی: { $limit }. بین 500 تا 5,000 امضا در هر بررسی و با گام‌های 100 تایی انتخاب کنید.
wallets-watch-budget-hint-transactions = سقف فعلی: { $limit }. بین 500 تا 5,000 تراکنش موفق در هر بررسی و با گام‌های 100 تایی انتخاب کنید.
wallets-watch-budget-error-range = مقداری بین 500 تا 5,000 رکورد در هر بررسی و با گام‌های 100 تایی انتخاب کنید.
wallets-watch-budget-error-ack = تأیید کنید که امضاهای پس از آخرین بررسی کامل‌شده نادیده گرفته می‌شوند.
wallets-watch-budget-save-failed = ذخیره سقف پایش ممکن نشد.
wallets-watch-budget-save = ذخیره سقف
wallets-watch-budget-resume = ادامه از اکنون
wallets-watch-budget-resume-notice = این کیف پول پیش از رسیدن به آخرین وضعیت به سقف بررسی خود رسید. «ادامه از اکنون» از آخرین فعالیت کیف پول شروع می‌کند؛ فعالیت‌های پس از آخرین بررسی کامل‌شده کپی نمی‌شوند.
wallets-watch-budget-resume-tasks = وظیفه‌های کپی تا زمانی که هرکدام را در کپی‌تریدینگ از سر نگیرید متوقف می‌مانند.
wallets-watch-budget-resume-ack = می‌دانم فعالیت‌های ازدست‌رفته کپی نمی‌شوند.
wallets-watch-budget-resumed = پایش از آخرین وضعیت کیف پول ادامه یافت
wallets-watch-budget-updated = سقف پایش کیف پول به‌روزرسانی شد
wallets-watch-helius-allow = در صورت نیاز، رسیدن به آخرین وضعیت با { -helius } مجاز باشد
wallets-watch-helius-try = تلاش برای رسیدن به آخرین وضعیت با { -helius }
wallets-watch-helius-stop = توقف رسیدن به آخرین وضعیت با { -helius } برای این کیف پول
wallets-watch-helius-description-approved = رسیدن به آخرین وضعیت با { -helius } برای این کیف پول مجاز است. با خاموش کردن آن، بررسی‌ها به حالت استاندارد برمی‌گردند که ممکن است در کیف پول پرفعالیت عقب بماند.
wallets-watch-helius-description-available = { -helius } می‌تواند تراکنش‌های موفق Solana را از موقعیت ذخیره‌شده و بدون رد کردن بازه بررسی‌نشده بررسی کند. ممکن است اعتبار بیشتری از ارائه‌دهنده مصرف کند و همچنان عقب بماند.
wallets-watch-helius-description-unavailable = رسیدن به آخرین وضعیت با { -helius } در دسترس نیست. برای استفاده از آن، یک اندپوینت RPC فعال { -helius } را پیکربندی کنید.
wallets-watch-helius-description-unsupported = هیچ ارائه‌دهنده‌ای برای رسیدن به آخرین وضعیت در این پایش پشتیبانی نمی‌شود. اگر پایش به سقف خود برسد، «ادامه از اکنون» در دسترس است.
wallets-watch-helius-allow-title = مجاز کردن رسیدن به آخرین وضعیت با { -helius } برای این کیف پول
wallets-watch-helius-allow-message = { -helius } می‌تواند تراکنش‌های موفق Solana را از موقعیت ذخیره‌شده و بدون رد کردن بازه بررسی‌نشده بررسی کند. در حال حاضر برای هر 100 تراکنش کامل بازگشتی، با رو به بالا گرد کردن، 10 اعتبار کسر می‌کند و حداقل هزینه هر درخواست 10 اعتبار است. یک بررسی می‌تواند چند درخواست ایجاد کند؛ مصرف و قیمت‌گذاری ارائه‌دهنده ممکن است تغییر کند. وظیفه‌های کپی تا زمان از سر گرفتن جداگانه متوقف می‌مانند.
wallets-watch-helius-allow-confirm = مجاز کردن برای این کیف پول
wallets-watch-helius-stop-message = این کیف پول به بررسی‌های استاندارد برمی‌گردد. ممکن است کیف پول پرفعالیت دوباره به سقف پایش برسد و متوقف شود. کیف پول‌های دیگر و پیکربندی RPC شما در { -helius } تغییری نمی‌کنند.
wallets-watch-helius-stop-confirm = توقف برای این کیف پول
wallets-watch-helius-stop-keep = مجاز بماند
wallets-watch-helius-restored = پایش از پیشرفت ذخیره‌شده بازگردانده شد؛ وظیفه‌های کپی متوقف می‌مانند
wallets-watch-helius-allowed = رسیدن به آخرین وضعیت با { -helius } در صورت نیاز برای این کیف پول مجاز شد
wallets-watch-helius-stopped = رسیدن به آخرین وضعیت با { -helius } برای این کیف پول متوقف شد
wallets-watch-helius-update-failed = به‌روزرسانی تنظیم رسیدن به آخرین وضعیت کیف پول ممکن نشد

# wallets.html: Export Private Key dialog.
wallets-export-title = خروجی گرفتن از کلید خصوصی
wallets-export-warning-title = هشدار امنیتی مهم
wallets-export-warning-body = کلید خصوصی خود را هرگز با کسی به اشتراک نگذارید. هر کسی که به این کلید دسترسی داشته باشد می‌تواند همه دارایی‌های این کیف پول را بردارد.
wallets-export-key-label = کلید خصوصی (Base58)
wallets-export-copy =
    .title = کپی در کلیپ‌بورد
    .aria-label = کپی در کلیپ‌بورد
wallets-export-reveal = نمایش کلید

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = بایگانی کیف پول
wallets-archive-note = کیف پول‌های بایگانی‌شده در هیچ عملیاتی استفاده نمی‌شوند اما هر زمان قابل بازگردانی هستند.
wallets-archive-confirm = بله، بایگانی شود
wallets-delete-title = حذف کیف پول
wallets-delete-warning-title = این کار قابل بازگشت نیست!
wallets-delete-warning-body = با حذف این کیف پول، خود کیف پول و کلید خصوصی رمزگذاری‌شده آن برای همیشه از این دستگاه پاک می‌شود.
wallets-delete-confirm = بله، حذف شود

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = وارد کردن کیف پول‌ها
wallets-bulk-import-submit = وارد کردن کیف پول‌ها
wallets-bulk-step-upload = بارگذاری فایل
wallets-bulk-step-map = نگاشت ستون‌ها
wallets-bulk-step-results = نتایج
wallets-bulk-import-file-warning-body = فقط فایل‌های منابع معتبر را وارد کنید. کلیدهای خصوصی رمزگذاری می‌شوند و به‌صورت امن روی همین دستگاه ذخیره خواهند شد.
wallets-bulk-drop-title = فایل خود را اینجا رها کنید
wallets-bulk-drop-subtitle = یا برای انتخاب کلیک کنید
wallets-bulk-drop-formats = CSV و Excel (‎.xlsx، ‎.xls) پشتیبانی می‌شود
wallets-bulk-file-remove =
    .aria-label = حذف فایل
wallets-bulk-map-subtitle = ستون‌های فایل را با فیلدهای کیف پول تطبیق دهید
wallets-bulk-preview-title = پیش‌نمایش (5 ردیف اول)
wallets-bulk-summary-valid = <strong>{ $count }</strong> معتبر
wallets-bulk-summary-invalid = <strong>{ $count }</strong> نامعتبر
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> تکراری
       *[other] <strong>{ $count }</strong> تکراری
    }
wallets-bulk-done = انجام شد
wallets-bulk-file-invalid = نوع فایل نامعتبر است. لطفاً از فایل CSV یا Excel استفاده کنید.
wallets-bulk-preview-busy = در حال پردازش...
wallets-bulk-preview-fallback = پردازش فایل ناموفق بود
wallets-bulk-preview-failed = پردازش فایل ناموفق بود: { $reason }
wallets-bulk-column-select = -- انتخاب ستون --
wallets-bulk-preview-empty = هیچ ردیف داده‌ای در فایل پیدا نشد
wallets-bulk-preview-status = وضعیت
wallets-bulk-status-valid = معتبر
wallets-bulk-status-duplicate = تکراری
wallets-bulk-status-invalid = نامعتبر
wallets-bulk-import-busy = در حال وارد کردن...
wallets-bulk-import-toast =
    { $count ->
        [one] { $count } کیف پول وارد شد
       *[other] { $count } کیف پول وارد شد
    }
wallets-bulk-import-error = وارد کردن ناموفق بود: { $reason }
wallets-bulk-result-success-title = وارد کردن موفق بود
wallets-bulk-result-success-detail =
    { $count ->
        [one] همه { $count } کیف پول با موفقیت وارد شد
       *[other] همه { $count } کیف پول با موفقیت وارد شد
    }
wallets-bulk-result-partial-title = موفقیت جزئی
wallets-bulk-result-partial-detail = وارد شده: { $imported }، ناموفق: { $failed }
wallets-bulk-result-failed-title = وارد کردن ناموفق بود
wallets-bulk-result-failed-detail =
    { $count ->
        [one] وارد کردن همه { $count } کیف پول ناموفق بود
       *[other] وارد کردن همه { $count } کیف پول ناموفق بود
    }
wallets-bulk-result-imported = وارد شده
wallets-bulk-result-failed = ناموفق

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = خروجی گرفتن از کیف پول‌ها
wallets-bulk-export-format = قالب
wallets-bulk-export-format-csv = CSV (‎.csv)
wallets-bulk-export-format-xlsx = Excel (‎.xlsx)
wallets-bulk-export-include-archived = شامل کیف پول‌های بایگانی‌شده
wallets-bulk-export-safe-title = خروجی امن
wallets-bulk-export-safe-body = فقط آدرس کیف پول‌ها و فراداده صادر می‌شود. کلید خصوصی شامل نمی‌شود.
wallets-bulk-export-safe-submit = خروجی آدرس‌ها
wallets-bulk-export-or = یا
wallets-bulk-export-danger-title = خروجی پرخطر
wallets-bulk-export-danger-body = کلیدهای خصوصی هم در خروجی گنجانده می‌شوند. هر کسی که این فایل را داشته باشد می‌تواند دارایی‌های شما را بردارد.
wallets-bulk-export-danger-submit = خروجی همراه با کلیدهای خصوصی
wallets-bulk-export-busy = در حال خروجی گرفتن...
wallets-bulk-export-done = کیف پول‌ها در { $filename } ذخیره شدند
wallets-bulk-export-fallback = خروجی گرفتن ناموفق بود
wallets-bulk-export-error = خروجی گرفتن ناموفق بود: { $reason }
wallets-bulk-confirm-title = تأیید خروجی پرخطر
wallets-bulk-confirm-warning =
    { $count ->
        [one] در آستانه صدور <strong>{ $count }</strong> کلید خصوصی هستید. این کار بسیار خطرناک است!
       *[other] در آستانه صدور <strong>{ $count }</strong> کلید خصوصی هستید. این کار بسیار خطرناک است!
    }
wallets-bulk-confirm-risk-steal = هر کسی که این فایل را داشته باشد می‌تواند همه دارایی‌ها را بردارد
wallets-bulk-confirm-risk-share = این فایل را هرگز با کسی به اشتراک نگذارید
wallets-bulk-confirm-risk-delete = فایل را بلافاصله پس از استفاده حذف کنید
wallets-bulk-confirm-prompt = برای تأیید، عبارت زیر را تایپ کنید
wallets-bulk-confirm-submit = خروجی کلیدها

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = توکن
wallets-holdings-col-balance = موجودی
wallets-holdings-col-value = ارزش ({ -sol })
wallets-holdings-col-type = نوع
wallets-holdings-col-decimals = اعشار
wallets-holdings-col-mint = مینت
wallets-holdings-empty-title = هیچ توکنی نگهداری نمی‌شود
wallets-holdings-empty-message = توکن‌هایی که این کیف پول نگه می‌دارد اینجا نمایش داده می‌شوند.
wallets-holdings-no-main = کیف پول اصلی وجود ندارد
wallets-holdings-main-tag = اصلی
wallets-holdings-main-title = کیف پول اصلی
wallets-holdings-tokens = توکن‌ها
wallets-holdings-last-used = آخرین استفاده
wallets-holdings-never = هرگز
wallets-holdings-search =
    .placeholder = جستجو بر اساس نماد یا مینت...
wallets-holdings-export = خروجی کلید
wallets-holdings-export-tooltip = خروجی گرفتن از کلید خصوصی این کیف پول
wallets-list-col-name = نام
wallets-list-col-balance = موجودی ({ -sol })
wallets-list-col-type = نوع
wallets-list-col-created = تاریخ ساخت
wallets-list-col-actions = عملیات
wallets-list-action-export = خروجی کلید خصوصی
wallets-list-action-archive = بایگانی کیف پول
wallets-list-action-restore = بازگرداندن کیف پول
wallets-list-action-delete = حذف دائمی
wallets-list-count = کیف پول‌ها
wallets-list-search =
    .placeholder = جستجو بر اساس نام یا آدرس...
wallets-list-loading-title = در حال بارگذاری کیف پول‌ها…
wallets-list-loading-description = در حال آماده‌سازی نمای کیف پول انتخاب‌شده.
wallets-secondaries-empty-title = کیف پول ثانویه‌ای وجود ندارد
wallets-secondaries-empty-message = برای سازمان‌دهی فعالیت‌های معاملاتی میان چند حساب، کیف پول‌های بیشتری بسازید.
wallets-secondaries-add = افزودن کیف پول
wallets-archive-empty-title = کیف پول بایگانی‌شده‌ای وجود ندارد
wallets-archive-empty-message = کیف پول‌هایی که بایگانی کنید برای مراجعه بعدی در اینجا به‌صورت امن نگهداری می‌شوند.

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = کیف پول
wallets-watched-col-status = وضعیت
wallets-watched-col-progress = پیشرفت ذخیره‌شده
wallets-watched-col-last-check = آخرین بررسی
wallets-watched-unlabelled = کیف پول بدون برچسب
wallets-watched-generic-name = کیف پول
wallets-watched-not-synced = هنوز همگام نشده
wallets-watched-not-checked = هنوز بررسی نشده
wallets-watched-action-copy = کپی معامله
    .title = باز کردن این کیف پول در کپی‌تریدینگ
wallets-watched-action-restore = بازگرداندن پایش
wallets-watched-action-options = گزینه‌های پایش
wallets-watched-action-retry = تلاش دوباره پایش
wallets-watched-action-pause = توقف
wallets-watched-action-enable = فعال‌سازی
wallets-watched-action-remove =
    .title = حذف
    .aria-label = حذف { $name }
wallets-watch-state-paused = متوقف
wallets-watch-state-catching-up = در حال رسیدن به آخرین وضعیت
wallets-watch-state-watching = در حال پایش
wallets-watch-state-streaming = جریانی
wallets-watch-state-polling = بررسی دوره‌ای
wallets-watched-detail-helius = بررسی این کیف پول از طریق { -helius } انجام می‌شود.
wallets-watched-empty-title = آدرسی پایش نمی‌شود
wallets-watched-empty-message = برای ثبت فعالیت روی زنجیره یک کیف پول عمومی از «پایش کیف پول» استفاده کنید.
wallets-watched-count = پایش‌شده
wallets-watched-search =
    .placeholder = جستجوی کیف پول‌های پایش‌شده...
wallets-watched-add = پایش کیف پول
wallets-watched-refresh = به‌روزرسانی کیف پول‌های پایش‌شده
wallets-watched-loading-title = در حال بارگذاری کیف پول‌های پایش‌شده...
wallets-watched-loading-description = در حال دریافت اهداف پایش.
wallets-watched-load-error-title = بارگذاری آدرس‌های پایش‌شده ممکن نشد
wallets-watched-load-error-description = برای تلاش دوباره، به‌روزرسانی را بزنید.
wallets-watched-address-invalid = یک آدرس کیف پول معتبر Solana وارد کنید.
wallets-watched-added = پایش کیف پول اضافه شد
wallets-watched-duplicate = این کیف پول از قبل پایش می‌شود.
wallets-watched-add-failed = افزودن پایش کیف پول ممکن نشد.
wallets-watched-retried = پایش کیف پول با نشانگر ذخیره‌شده بازگردانده شد
wallets-watched-paused = پایش کیف پول متوقف شد
wallets-watched-enabled = پایش کیف پول فعال شد
wallets-watched-removed = پایش کیف پول حذف شد
wallets-watched-update-failed = به‌روزرسانی پایش کیف پول ممکن نشد
