# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = داده‌های { -brand } فعال است
    .detail = کندل‌های مشترک، رجیستری استخرها، گزارش‌های امنیتی و هویت توکن‌ها از screenerbot.io ارائه می‌شود.

account-data-access-disabled = داده‌های { -brand } خاموش است
    .detail = منبع { -brand } در تنظیمات شما خاموش شده است، بنابراین داده فقط از ارائه‌دهندگان عمومی دریافت می‌شود.

account-data-access-offline = داده‌های { -brand } آفلاین است
    .detail = اتصال شبکه برقرار نیست. با بازگشت اتصال، دریافت داده خودکار از سر گرفته می‌شود.

account-data-access-signed-out = داده‌های { -brand } به حساب کاربری نیاز دارد
    .detail = نمودارها، استخرها، گزارش‌های امنیتی و هویت توکن‌ها به‌جای آن از ارائه‌دهندگان عمومی دریافت می‌شوند. این منابع کندتر هستند، محدودیت نرخ دارند و تاریخچه کمتری ارائه می‌دهند. ورود رایگان است و هیچ چیز دیگری را در نحوه کار { -brand } تغییر نمی‌دهد.

account-data-access-reauthorization-required = داده‌های { -brand } به ورود مجدد شما نیاز دارد
    .detail = این دستگاه پیش از وجود داده‌های { -brand } مجاز شده بود. برای بازگرداندن آن دوباره وارد شوید — تا آن زمان از ارائه‌دهندگان عمومی استفاده می‌شود.

account-data-access-version-unsupported = داده‌های { -brand } به نسخه جدیدتر نیاز دارد
    .detail = این نسخه دیگر پشتیبانی نمی‌شود. برای استفاده دوباره از داده‌های { -brand }، به نسخه { $minimum } یا جدیدتر به‌روزرسانی کنید؛ تا آن زمان از ارائه‌دهندگان عمومی استفاده می‌شود.

account-data-access-unreachable = داده‌های { -brand } پاسخ نمی‌دهد
    .detail = سرویس پاسخ نداد. از ارائه‌دهندگان عمومی استفاده می‌شود و { -brand } به تلاش مجدد ادامه می‌دهد.

account-data-access-unknown = داده‌های { -brand } هنوز بررسی نشده است
    .detail = { -brand } در این نشست هنوز به داده مشترک نیاز پیدا نکرده است.

## Account panel (ui/account/panel.js), shared by Setup and Settings

# Features a signed-in account carries.
account-scope-data-read = داده بازار { -brand }
account-scope-rpc-submit = ارسال رایگان تراکنش امضاشده
account-scope-vote = رأی‌دهی به توکن‌ها
account-scope-referral-read = درآمد معرفی
account-scope-account-read = جزئیات حساب

account-panel-request-failed = انجام نشد. لطفاً دوباره تلاش کنید.
account-panel-checking = در حال بررسی وضعیت حساب…
account-panel-status-unavailable = وضعیت حساب در دسترس نیست.
account-panel-browser-notice = ورود را در مرورگر خود کامل کنید و سپس به اینجا برگردید. این پنل به‌روز می‌شود.
account-panel-browser-timeout = ورود از طریق مرورگر کامل نشد. می‌توانید دوباره شروع کنید.
account-panel-unavailable = قابلیت‌های حساب در حال حاضر در دسترس نیستند. راه‌اندازی را بدون ورود ادامه دهید.
account-panel-retry-status = تلاش مجدد برای وضعیت حساب
account-panel-signed-in-fallback = وارد شده‌اید
account-panel-features =
    .aria-label = قابلیت‌های حساب
account-panel-sign-out = خروج
account-panel-signing-out = در حال خروج…
account-panel-sign-in = ورود
account-panel-signing-in = در حال ورود…
account-panel-sign-in-wallet = ورود با کیف پول
account-panel-opening-browser = در حال باز کردن مرورگر…
account-panel-continue-browser = ادامه در مرورگر
account-panel-sign-in-email = ورود با ایمیل
account-panel-new-to = تازه با { -brand } آشنا شده‌اید؟
account-panel-create-account = ساخت حساب
account-panel-unlocks-title = همراه با حساب کاربری
account-panel-back-to-options = بازگشت به گزینه‌های ورود
account-panel-email-label = ایمیل
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = رمز عبور
account-panel-password-input =
    .placeholder = رمز عبور شما
account-panel-need-account = حساب کاربری لازم دارید یا رمز عبور را فراموش کرده‌اید؟
account-panel-open-website = باز کردن screenerbot.io
