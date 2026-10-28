# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = أدخل المفتاح الخاص للمحفظة.
setup-wallet-json-recognized = تم التعرف على صيغة مفتاح JSON بطول 64 بايت.
setup-wallet-json-invalid = استخدم مصفوفة JSON تحتوي على 64 قيمة بايت بالضبط (0–255).
setup-wallet-format-invalid = استخدم مفتاحًا خاصًا بصيغة base58 أو مصفوفة JSON بطول 64 بايت.
setup-wallet-base58-recognized = تم التعرف على صيغة مفتاح Base58.

## RPC endpoint validation

setup-rpc-required = أدخل نقطة اتصال RPC واحدة على الأقل.
setup-dialog-rpc-optional-note = اختياري: اترك الحقل فارغًا للإبقاء على نقاط اتصال RPC المضبوطة حاليًا.
setup-rpc-too-many = لا تستخدم أكثر من 10 نقاط اتصال RPC.
setup-rpc-url-invalid = يجب أن تكون كل نقطة اتصال رابط HTTPS صالحًا.
setup-rpc-url-credentials = لا يمكن أن تتضمن روابط RPC أسماء مستخدمين أو كلمات مرور.
setup-rpc-url-fragment = لا يمكن أن تتضمن روابط RPC أجزاء (fragments).
setup-rpc-public-endpoint = لا يمكن لـ RPC العام لـ Solana دعم الاستعلام الدوري المستمر.
setup-rpc-private-host = لا يمكن لنقاط اتصال RPC استخدام مضيفين محليين أو على شبكة خاصة.
setup-rpc-duplicate = أزل نقاط اتصال RPC المكررة.
setup-rpc-ready =
    { $count ->
        [zero] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
        [one] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
        [two] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
        [few] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
        [many] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
       *[other] نقاط اتصال HTTPS الجاهزة للاختبار: { $count }.
    }

## Verification results

setup-wallet-verified = تم التحقق من المحفظة
setup-wallet-unverified = تعذّر التحقق من المحفظة
setup-wallet-address-detail = العنوان { $address }
setup-wallet-format-hint = تحقق من صيغة المفتاح الخاص.
setup-rpc-none-working = لا توجد نقطة اتصال RPC عاملة على الشبكة الرئيسية
setup-rpc-health-failed = لم تجتز أي نقطة اتصال فحوصات سلامة الشبكة الرئيسية.
setup-rpc-partial = العاملة: { $working }؛ غير المتاحة: { $failed }
setup-rpc-verified =
    { $count ->
        [zero] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
        [one] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
        [two] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
        [few] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
        [many] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
       *[other] نقاط اتصال الشبكة الرئيسية التي تم التحقق منها: { $count }
    }
setup-rpc-fastest = الأسرع: { $url } ({ $latency } ms).
setup-error-request-failed = فشل الطلب ({ $status })
setup-error-restart-timeout = لم يُعِد { -brand } الاتصال بعد.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = جارٍ تحليل المفتاح الخاص
setup-verify-wallet-parsing-detail = جارٍ فحص المفتاح واشتقاق عنوانه العام.
setup-verify-wallet-waiting = بانتظار التحقق
setup-verify-rpc-testing = جارٍ اختبار الشبكة الرئيسية لـ Solana
setup-verify-rpc-testing-detail =
    { $count ->
        [zero] جارٍ فحص نقاط الاتصال: { $count }.
        [one] جارٍ فحص نقاط الاتصال: { $count }.
        [two] جارٍ فحص نقاط الاتصال: { $count }.
        [few] جارٍ فحص نقاط الاتصال: { $count }.
        [many] جارٍ فحص نقاط الاتصال: { $count }.
       *[other] جارٍ فحص نقاط الاتصال: { $count }.
    }
setup-verify-rpc-waiting = بانتظار اختبار نقاط الاتصال
setup-verify-save-waiting = بانتظار الحفظ
setup-verify-save-running = جارٍ التشفير والحفظ
setup-verify-save-running-detail = جارٍ كتابة الإعدادات التي تم التحقق منها على هذا الجهاز.
setup-verify-save-done = تم حفظ الإعدادات
setup-verify-save-done-detail = تم تشفير المفتاح الخاص وتخزين نقاط اتصال RPC العاملة.
setup-verify-save-failed = تعذّر حفظ الإعداد
setup-verify-save-skipped = لم يتم الحفظ. أصلح المشكلة أعلاه، ثم تحقّق مرة أخرى.
setup-verify-request-failed = فشل طلب التحقق
setup-verify-summary-checking = جارٍ فحص محفظتك واتصالات الشبكة الرئيسية لـ Solana.
setup-verify-summary-running = جارٍ التحقق من بيانات الاعتماد التي أدخلتها كما هي.
setup-verify-summary-saving = تم التحقق من بيانات الاعتماد. جارٍ الحفظ بأمان.
setup-verify-summary-failed = راجع المشكلة ثم تحقق مرة أخرى.

## Errors

setup-error-credentials-failed = فشل التحقق من بيانات الاعتماد.
setup-error-save-failed = تعذّر حفظ الإعداد.
setup-error-verify-failed = فشل التحقق.
setup-error-explore-failed = تعذّر بدء وضع الاستكشاف.
setup-error-gateway-failed = تعذّر حفظ إعداد إرسال المعاملات.
setup-action-review-credentials = مراجعة بيانات الاعتماد

## Completion

setup-explore-opening = جارٍ فتح وضع الاستكشاف…
setup-complete-restarting = جارٍ إعادة تشغيل { -brand } بإعداداتك التي تم التحقق منها.
setup-complete-finishing = جارٍ إنهاء إعادة التشغيل…
setup-complete-ready = { -brand } جاهز. جارٍ فتح لوحة التحكم…
setup-complete-stored = تم تخزين إعداداتك التي تم التحقق منها بأمان على هذا الجهاز.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = إظهار المفتاح الخاص
setup-wallet-hide-key = إخفاء المفتاح الخاص
setup-wallet-copy =
    .aria-label = نسخ عنوان المحفظة
    .title = نسخ عنوان المحفظة
setup-wallet-copy-done =
    .aria-label = تم نسخ عنوان المحفظة
    .title = تم النسخ
setup-wallet-copy-failed =
    .aria-label = تعذّر نسخ عنوان المحفظة
    .title = فشل النسخ

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = إعداد المحفظة وRPC
setup-dialog-subtitle = اربط محفظة Solana ونقطة اتصال RPC متميزة لتفعيل التداول وبيانات السلسلة الحية. يُشفَّر مفتاحك الخاص على هذا الجهاز ولا يغادره أبدًا.
setup-dialog-close =
    .title = إغلاق
    .aria-label = إغلاق
setup-dialog-wallet-label = المفتاح الخاص للمحفظة
setup-dialog-wallet-input =
    .placeholder = سلسلة Base58 أو مصفوفة JSON مثل ⁨[1,2,3,...]⁩
setup-dialog-submit = تحقق واتصل
setup-dialog-working = جارٍ العمل…
setup-dialog-validating = جارٍ التحقق…
setup-dialog-saving = جارٍ الحفظ…
setup-dialog-restarting = جارٍ إعادة التشغيل…
setup-dialog-saved = تم حفظ الإعداد، وجارٍ إعادة تشغيل { -brand } في الوضع الكامل…
setup-dialog-error-validation = فشل التحقق.
setup-dialog-error-incomplete = تعذّر إكمال الإعداد.
setup-dialog-error-restart-helper = مساعد إعادة التشغيل التلقائي غير متاح. أعد تحميل لوحة التحكم بعد قليل.
setup-dialog-error-unexpected = خطأ غير متوقع.
setup-dialog-replace-title = استبدال محفظة التداول
setup-dialog-replace-subtitle = تداول بمحفظة أخرى. يُشفَّر مفتاحها الخاص على هذا الجهاز ولا يغادره أبدًا.
setup-dialog-replace-submit = تحقق واستبدل
setup-dialog-replace-saved = تم تجهيز الاستبدال، وجارٍ إعادة تشغيل { -brand } بالمحفظة الجديدة…
setup-dialog-replace-confirm-intro = استبدال محفظة التداول:
setup-dialog-replace-confirm-stop = يوقف المتداول والتداول بالنسخ. أعد تشغيلهما بعد إعادة التشغيل.
setup-dialog-replace-confirm-backup = ينقل مراكز المحفظة الحالية ومعاملاتها وسجلها إلى مجلد نسخ احتياطي.
setup-dialog-replace-confirm-keep = يحتفظ بمفتاح المحفظة الحالية ضمن محافظ أخرى، حيث يمكنك تصديره دائمًا.
setup-dialog-replace-confirm-funds = لا ينقل أي أموال: تبقى الرموز و{ -sol } في المحفظة الحالية.
setup-dialog-replace-confirm-rpc = يستبدل نقاط اتصال RPC المضبوطة بالتي تُدخلها؛ وإذا تركت الحقل فارغًا تبقى نقاط الاتصال الحالية.
setup-dialog-replace-confirm-restart = يعيد تشغيل { -brand } بالمحفظة الجديدة.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = تقدم الإعداد
setup-wizard-wallet-security-note = يُشفَّر قبل الحفظ.
setup-wizard-rpc-title = نقاط اتصال RPC
setup-wizard-rpc-input =
    .placeholder = رابط HTTPS واحد في كل سطر
setup-wizard-rpc-guidance = استخدم مزوّد RPC موثوقًا للشبكة الرئيسية؛ النقاط العامة غير مدعومة.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = موصى به
setup-wizard-account-title = حساب { -brand }
setup-wizard-account-optional = اختياري
setup-wizard-account-loading = جارٍ التحقق من حالة الحساب…
setup-wizard-verify-title = التحقق والحفظ
setup-wizard-verify-list =
    .aria-label = حالة التحقق من الإعداد
setup-wizard-verify-wallet = المحفظة
setup-wizard-verify-save = إعداد آمن
setup-wizard-complete-title = تم حفظ الإعداد
setup-wizard-reconnect = إعادة محاولة الاتصال
setup-wizard-reload = إعادة تحميل لوحة التحكم
setup-wizard-error-title = الإعداد يحتاج إلى انتباهك
setup-wizard-explore = استكشاف لوحة التحكم
setup-wizard-continue = متابعة

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = المحفظة
setup-wizard-step-connection = الاتصال
setup-wizard-step-verify = التحقق
setup-wizard-step-ready = جاهز
setup-wallet-title = إعداد محفظة التداول
setup-wallet-description = المحفظة التي يتداول بها { -brand }. يُشفَّر مفتاحها على هذا الكمبيوتر ولا يغادره أبدًا.
setup-wallet-source =
    .aria-label = مصدر المحفظة
setup-wallet-create-title = إنشاء محفظة جديدة
setup-wallet-create-detail = ينشئ { -brand } محفظة جديدة. تحفظ مفتاحها، ثم ترسل إليها { -sol }.
setup-wallet-import-title = استيراد محفظة
setup-wallet-import-detail = استخدم مفتاحًا خاصًا تملكه بالفعل (base58 أو مصفوفة JSON من 64 رقمًا).
setup-wallet-creating = جارٍ إنشاء محفظتك…
setup-wallet-create-failed = تعذّر إنشاء المحفظة. حاول مرة أخرى.
setup-backup-title = احفظ مفتاحك الخاص
setup-backup-description = من يملك هذا المفتاح يتحكم في المحفظة. لا يستطيع { -brand } استعادته.
setup-backup-address = العنوان
setup-backup-key = المفتاح الخاص
setup-backup-copy-address = نسخ العنوان
setup-backup-download = تنزيل
setup-backup-copied = تم النسخ. امسح الحافظة بعد حفظه.
setup-backup-machine-warning = النسخة المشفّرة التي يحتفظ بها { -brand } لا تُفتح إلا على هذا الكمبيوتر. إذا فُقد الكمبيوتر أو استُبدل، فلن يستعيد المحفظة إلا المفتاح الذي تحفظه الآن. احتفظ به دون اتصال، مثلًا على الورق أو في مدير كلمات المرور.
setup-backup-ack-saved = حفظت المفتاح الخاص في مكان آمن خارج هذا الكمبيوتر.
setup-backup-ack-control = أفهم أن أي شخص يملك هذا المفتاح يمكنه أخذ الأموال الموجودة في هذه المحفظة.
setup-backup-ack-recovery = أفهم أن { -brand } لا يستطيع استعادة هذا المفتاح. بدونه، فقدان الكمبيوتر يعني فقدان الأموال.
setup-backup-create-another = إنشاء محفظة أخرى
setup-backup-replace-confirm = هل تريد تجاهل هذا المفتاح وإنشاء محفظة جديدة؟ افعل ذلك فقط إن لم ترسل إليها أي شيء.
setup-connection-title = كيف يتصل { -brand } بشبكة { -solana }؟
setup-connection-description = يمكنك تغيير ذلك لاحقًا من الإعدادات ← الاتصال.
setup-connection-choice =
    .aria-label = الاتصال
setup-connection-service-title = خدمة { -brand }
setup-connection-service-badge = لا حاجة إلى RPC
setup-connection-service-detail = يستخدم حساب { -brand } المجاني الخاص بك.
setup-connection-service-prices = تتحدّث الأسعار كل بضع ثوانٍ
setup-connection-service-features = يحتاج نسخ التداول ومراقبة المحافظ وأرصدة المحافظ الأخرى والسجل الأقدم والمبادلات المباشرة عبر المجمّعات وأسعار المجمّعات الحية إلى RPC خاص بك
setup-connection-service-pause = يتوقف التداول مؤقتًا إذا سجّلت الخروج أو تعذّر الوصول إلى screenerbot.io
setup-connection-service-privacy = لا يغادر مفتاحك الخاص هذا الجهاز أبدًا: توقَّع المبادلات هنا، ولا يستخدم screenerbot.io إلا العنوان العام لمحفظتك لقراءة أرصدتها ومعاملاتها
setup-connection-own-title = RPC الخاص بك
setup-connection-own-badge = الأسرع
setup-connection-own-detail = أسعار نحو مرتين في الثانية وكل الميزات. يتطلب نقطة اتصال RPC من مزوّد.
setup-connection-signin-required = سجّل الدخول أو أنشئ حسابًا مجانيًا لاستخدام خدمة { -brand }.
setup-connection-reauthorize = سجّل الدخول مرة أخرى للسماح لخدمة { -brand } بقراءة المحفظة.
setup-connection-sending = إرسال المعاملات عبر { -brand }
setup-connection-sending-hint = متاح عند تسجيل الدخول. عندما يتعذّر ذلك يُستخدم RPC الخاص بك.
setup-wizard-account-required = مطلوب
setup-wizard-verify-connection = الاتصال
setup-verify-service-checking = جارٍ فحص خدمة { -brand }
setup-verify-service-ready = خدمة { -brand } جاهزة
setup-verify-service-busy = خدمة { -brand } مشغولة الآن. حاول مرة أخرى بعد لحظات، أو استخدم RPC الخاص بك.
setup-verify-service-daily-limit = بلغ حسابك حد اليوم لخدمة { -brand }. حاول مرة أخرى بعد 00:00 UTC، أو استخدم RPC الخاص بك.
setup-verify-service-unreachable = تعذّر الوصول إلى screenerbot.io. تحقّق من اتصالك بالإنترنت.
setup-verify-service-version = لم تعد خدمة { -brand } تدعم هذا الإصدار. حدّث { -brand } للمتابعة.
setup-verify-service-signin = سجّل الدخول لاستخدام خدمة { -brand }.
setup-verify-save-done-service-detail = شُفّر المفتاح الخاص؛ واختيرت خدمة { -brand }.
setup-action-try-again = إعادة المحاولة
setup-action-use-own-rpc = استخدام RPC الخاص بي
setup-action-sign-in-again = تسجيل الدخول مرة أخرى
setup-ready-title = { -brand } جاهز
setup-fund-title = موّل محفظة التداول
setup-fund-waiting = بانتظار { -sol }…
setup-fund-received = تم استلام { $amount } { -sol }
setup-fund-balance-unavailable = جارٍ قراءة الرصيد… يتحدّث تلقائيًا.
setup-fund-open-dashboard = فتح لوحة التحكم
