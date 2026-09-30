# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = بيانات { -brand } نشطة
    .detail = تُقدَّم الشموع المشتركة وسجل مجمعات السيولة وتقارير الأمان وهوية الرموز من screenerbot.io.

account-data-access-disabled = بيانات { -brand } متوقفة
    .detail = مصدر { -brand } متوقف في إعداداتك، لذا تأتي البيانات من المزوّدين العامين فقط.

account-data-access-offline = بيانات { -brand } غير متصلة
    .detail = لا يوجد اتصال بالشبكة. ستُستأنف البيانات تلقائيًا عند عودة الاتصال.

account-data-access-signed-out = بيانات { -brand } تتطلب حسابًا
    .detail = تأتي المخططات ومجمعات السيولة وتقارير الأمان وهوية الرموز من المزوّدين العامين بدلًا من ذلك. وهي أبطأ ومحدودة المعدل وأقل عمقًا في السجل التاريخي. تسجيل الدخول مجاني ولا يغيّر أي شيء آخر في طريقة عمل { -brand }.

account-data-access-reauthorization-required = بيانات { -brand } تتطلب تسجيل الدخول مرة أخرى
    .detail = تم ترخيص هذا الجهاز قبل وجود بيانات { -brand }. سجّل الدخول مرة أخرى لاستعادتها — يُستخدم المزوّدون العامون حتى ذلك الحين.

account-data-access-version-unsupported = بيانات { -brand } تتطلب إصدارًا أحدث
    .detail = لم يعد هذا الإصدار مدعومًا. حدّث إلى { $minimum } أو أحدث لاستخدام بيانات { -brand } مجددًا؛ يُستخدم المزوّدون العامون حتى ذلك الحين.

account-data-access-unreachable = بيانات { -brand } لا تستجيب
    .detail = لم تجب الخدمة. يُستخدم المزوّدون العامون، وسيواصل { -brand } إعادة المحاولة.

account-data-access-unknown = لم يتم فحص بيانات { -brand } بعد
    .detail = لم يحتج { -brand } إلى البيانات المشتركة بعد في هذه الجلسة.

## Account panel (ui/account/panel.js), shared by Setup and Settings

# Features a signed-in account carries.
account-scope-data-read = بيانات سوق { -brand }
account-scope-rpc-submit = إرسال المعاملات الموقّعة مجانًا
account-scope-vote = التصويت على الرموز
account-scope-referral-read = أرباح الإحالة
account-scope-account-read = تفاصيل الحساب

account-panel-request-failed = لم تنجح العملية. يرجى المحاولة مرة أخرى.
account-panel-checking = جارٍ فحص حالة الحساب…
account-panel-status-unavailable = حالة الحساب غير متاحة.
account-panel-browser-notice = أكمل تسجيل الدخول في متصفحك ثم عد إلى هنا. ستُحدَّث هذه اللوحة.
account-panel-browser-timeout = لم يكتمل تسجيل الدخول عبر المتصفح. يمكنك بدؤه مرة أخرى.
account-panel-unavailable = ميزات الحساب غير متاحة حاليًا. تابع الإعداد دون تسجيل الدخول.
account-panel-retry-status = إعادة محاولة حالة الحساب
account-panel-signed-in-fallback = تم تسجيل الدخول
account-panel-features =
    .aria-label = ميزات الحساب
account-panel-sign-out = تسجيل الخروج
account-panel-signing-out = جارٍ تسجيل الخروج…
account-panel-sign-in = تسجيل الدخول
account-panel-signing-in = جارٍ تسجيل الدخول…
account-panel-sign-in-wallet = تسجيل الدخول بالمحفظة
account-panel-opening-browser = جارٍ فتح المتصفح…
account-panel-continue-browser = المتابعة في المتصفح
account-panel-sign-in-email = تسجيل الدخول بالبريد الإلكتروني
account-panel-new-to = جديد على { -brand }؟
account-panel-create-account = إنشاء حساب
account-panel-unlocks-title = مشمول مع الحساب
account-panel-back-to-options = العودة إلى خيارات تسجيل الدخول
account-panel-email-label = البريد الإلكتروني
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = كلمة المرور
account-panel-password-input =
    .placeholder = كلمة المرور الخاصة بك
account-panel-need-account = تحتاج إلى حساب أو نسيت كلمة المرور؟
account-panel-open-website = فتح screenerbot.io
