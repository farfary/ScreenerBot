# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

# Framing for an operation message followed by its technical cause.
errors-with-details = { $message }: { $details }

# Configuration
errors-config-save-failed = تعذّر حفظ الإعدادات

# Authentication
errors-auth-current-password-incorrect = كلمة المرور الحالية غير صحيحة
errors-auth-current-password-required = كلمة المرور الحالية مطلوبة لتغيير كلمة المرور
errors-auth-password-too-short = يجب ألا تقل كلمة المرور عن 4 أحرف
errors-auth-password-too-long = يجب ألا تزيد كلمة المرور على 128 حرفًا
errors-auth-hash-failed = تعذّر تشفير كلمة المرور
errors-auth-not-enabled = المصادقة غير مفعّلة
errors-auth-no-password = لم يتم ضبط كلمة مرور
errors-auth-password-incorrect = كلمة المرور غير صحيحة
errors-auth-required = المصادقة مطلوبة. يرجى تسجيل الدخول للوصول إلى نقطة الاتصال هذه.
errors-auth-totp-invalid = رمز 2FA غير صالح أو منتهي الصلاحية
errors-auth-totp-verify-failed = تعذّر التحقق من رمز 2FA
errors-auth-totp-password-required = يجب ضبط كلمة مرور قبل تفعيل 2FA
errors-auth-totp-uri-failed = تعذّر إنشاء عنوان TOTP URI
errors-auth-totp-qr-failed = تعذّر إنشاء رمز QR
errors-auth-totp-secret-required = المفتاح السري مطلوب
errors-auth-totp-save-failed = تعذّر حفظ إعدادات TOTP
errors-auth-totp-code-invalid = رمز التحقق غير صالح. يرجى التحقق من الرمز والمحاولة مرة أخرى.
errors-auth-totp-code-verify-failed = تعذّر التحقق من الرمز

# Request security
errors-security-invalid-local-request = يجب أن يصدر الطلب من لوحة التحكم المحلية
errors-security-invalid-token = رمز الأمان غير صالح
errors-security-token-required = رمز الأمان مطلوب. نقطة الاتصال هذه متاحة فقط من داخل { -brand }.

# Lockscreen
errors-lockscreen-no-password-set = لم يتم تعيين كلمة مرور
errors-lockscreen-invalid-type = نوع كلمة المرور غير صالح. يجب أن يكون 'pin4' أو 'pin6' أو 'text'
errors-lockscreen-invalid-format = كلمة المرور لا تطابق النوع المحدد
errors-lockscreen-no-current-password = لا توجد كلمة مرور معيّنة حاليًا
errors-lockscreen-password-incorrect = كلمة المرور غير صحيحة
errors-lockscreen-enable-needs-password = لا يمكن تفعيل شاشة القفل دون تعيين كلمة مرور أولًا

# Account
errors-account-signin-failed = فشل تسجيل الدخول
# The reason is the account server's own wording, shown exactly as sent.
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = تسجيل الدخول غير متاح
errors-account-browser-open-failed = تعذّر فتح المتصفح. افتح متصفحك الافتراضي وحاول مرة أخرى.
errors-account-signup-open-failed = تعذّر فتح صفحة التسجيل. افتح screenerbot.io/signup في متصفحك.
errors-account-credentials-required = أدخل عنوان بريدك الإلكتروني وكلمة المرور.
errors-account-signout-failed = فشل تسجيل الخروج
errors-account-gateway-update-failed = تعذّر تحديث إعداد البوابة

# Localization
errors-i18n-locale-not-registered = اللغة غير مسجّلة
errors-i18n-catalog-encode-failed = تعذّر ترميز الكتالوج

# System
errors-system-paths-init-failed = تعذّر إنشاء مجلدات التطبيق
errors-system-open-data-failed = تعذّر فتح مجلد البيانات
errors-system-url-empty = لا يمكن أن يكون عنوان URL فارغًا
errors-system-open-url-failed = تعذّر فتح عنوان URL

# Initialization
errors-initialization-required = تهيئة البوت مطلوبة قبل الوصول إلى نقطة الاتصال هذه. يرجى إكمال عملية التهيئة عبر واجهة الويب.
errors-initialization-onboarding-update-failed = تعذّر تحديث حالة الإعداد الأولي
errors-initialization-validation-required = التحقق من بيانات الاعتماد مطلوب قبل حفظ الإعداد
errors-initialization-encrypt-failed = تعذّر تشفير المفتاح الخاص

# Dashboard state
errors-ui-state-save-failed = تعذّر حفظ الحالة
errors-ui-state-clear-failed = تعذّر مسح الحالة

# Agent control
errors-agent-config-failed = فشل إعداد التحكم بالوكيل
errors-agent-invalid-parameters = معاملات غير صالحة
errors-agent-wallet-key-material = مواد مفاتيح المحفظة غير متاحة للوكيل
errors-agent-store-failed = فشل مخزن التحكم بالوكيل
errors-agent-invalid-pairing-request = طلب الاقتران غير صالح
errors-agent-pairing-rejected = تم رفض بيانات اعتماد الاقتران
errors-agent-disabled = التحكم بالوكيل معطّل
errors-agent-approval-not-pending = الموافقة لم تعد قيد الانتظار
errors-agent-approval-not-found = لم يتم العثور على الموافقة
errors-agent-bridge-task-failed = فشلت مهمة جسر التحكم بالوكيل
errors-agent-task-failed = فشلت مهمة التحكم بالوكيل
errors-agent-pairing-not-found = لا يوجد اقتران نشط بهذا المعرّف
errors-agent-permissions-update-failed = تعذّر تحديث الأذونات

# Connectivity
errors-connectivity-endpoint-not-found = نقطة الاتصال '{ $endpoint }' غير موجودة أو غير مراقبة

# Copy trading
errors-copy-task-not-found = لم يتم العثور على مهمة النسخ
errors-copy-holding-not-found = لا توجد حيازة تجريبية مفتوحة في هذا الرمز
errors-copy-live-confirmation-required = يتطلب تفعيل نسخ التداول الحي تأكيدًا صريحًا
errors-copy-task-invalid = مهمة نسخ غير صالحة
errors-copy-request-rejected = تم رفض طلب نسخ التداول
errors-copy-task-limit = تم بلوغ الحد الأقصى لمهام النسخ النشطة
errors-copy-watch-rejected = تعذّرت مراقبة هدف النسخ
errors-copy-live-unavailable = نسخ التداول الحي غير متاح
errors-copy-task-live = أوقف المهمة الحية مؤقتًا قبل حذفها
errors-copy-task-owns-positions = لا تزال مهمة النسخ تملك مراكز مفتوحة
errors-copy-request-failed = فشل طلب نسخ التداول

# Strategies
errors-strategies-not-found = لم يتم العثور على الاستراتيجية
errors-strategies-invalid-type = نوع الاستراتيجية غير صالح. يجب أن يكون ENTRY أو EXIT
errors-strategies-list-failed = تعذّر جلب الاستراتيجيات
errors-strategies-get-failed = تعذّر جلب الاستراتيجية
errors-strategies-serialize-rules-failed = تعذّر تسلسل القواعد
errors-strategies-invalid-rules-json = JSON القواعد غير صالح
errors-strategies-already-exists = الاستراتيجية ذات المعرّف '{ $id }' موجودة بالفعل
errors-strategies-validation-failed = فشل التحقق من الاستراتيجية
errors-strategies-create-failed = تعذّر إنشاء الاستراتيجية
errors-strategies-update-failed = تعذّر تحديث الاستراتيجية
errors-strategies-update-enabled-failed = تعذّر تحديث حالة تفعيل الاستراتيجية
errors-strategies-delete-failed = تعذّر حذف الاستراتيجية
errors-strategies-deploy-failed = تعذّر نشر الاستراتيجية
errors-strategies-no-performance = لا تتوفر بيانات أداء لهذه الاستراتيجية
errors-strategies-performance-failed = تعذّر جلب إحصاءات الأداء
errors-strategies-schemas-failed = تعذّر جلب مخططات الشروط
errors-strategies-evaluation-failed = فشل تقييم الاستراتيجية

# Transactions
errors-transactions-own-wallet-unavailable = المحفظة الرئيسية غير مهيأة
errors-transactions-invalid-subject = موضوع المعاملة ليس عنوان Solana صالحًا
errors-transactions-subject-not-watched = موضوع المعاملة ليس محفظة مراقبة
errors-transactions-watch-store-unavailable = المحافظ المراقبة غير متاحة

# Wallet
errors-wallet-unavailable = المحفظة الرئيسية غير متاحة
errors-wallet-changed = تغيّرت المحفظة الرئيسية؛ حدّث الصفحة وحاول مرة أخرى
errors-wallet-qr-failed = تعذّر إنشاء رمز QR للمحفظة

# Updates
errors-updates-none-available = لا يوجد تحديث متاح للتنزيل
errors-updates-version-changed = تغيّر التحديث المتاح؛ تحقق من التحديثات مرة أخرى
errors-updates-check-failed = فشل التحقق من التحديثات
errors-updates-download-failed = تعذّر بدء تنزيل التحديث
errors-updates-history-unavailable = سجل الإصدارات غير متاح
errors-updates-apply-failed = تعذّر تطبيق التحديث
errors-updates-install-failed = تعذّر فتح مثبّت التحديث

# Telegram
errors-telegram-settings-update-failed = تعذّر تحديث الإعدادات
errors-telegram-disabled = { -telegram } غير مفعّل
errors-telegram-not-configured = رمز البوت أو معرّف الدردشة غير مضبوط
errors-telegram-send-failed = تعذّر إرسال الرسالة
errors-telegram-notifier-failed = تعذّر إنشاء المُخطِر
errors-telegram-token-required = يجب ضبط رمز البوت أولًا
errors-telegram-discovery-failed = تعذّر بدء الاكتشاف
errors-telegram-chat-select-failed = تعذّر اختيار الدردشة

# Assistant chat
errors-chat-message-empty = لا يمكن أن تكون الرسالة فارغة
errors-chat-message-too-long = تتجاوز الرسالة الحد الأقصى للطول وهو 10,000 حرف
errors-chat-database-unavailable = قاعدة بيانات المحادثات غير مهيأة
errors-chat-session-not-found = لم يتم العثور على جلسة المحادثة { $id }
errors-chat-session-validate-failed = تعذّر التحقق من الجلسة
errors-chat-engine-unavailable = محرك المحادثة غير مهيأ
errors-chat-process-failed = تعذّرت معالجة رسالة المحادثة
errors-chat-stream-serialize-failed = تعذّر تسلسل حدث المحادثة
errors-chat-sessions-list-failed = تعذّر عرض جلسات المحادثة
errors-chat-session-create-failed = تعذّر إنشاء جلسة المحادثة
errors-chat-session-get-failed = تعذّر جلب جلسة المحادثة
errors-chat-messages-get-failed = تعذّر جلب رسائل المحادثة
errors-chat-session-delete-failed = تعذّر حذف جلسة المحادثة
errors-chat-messages-load-failed = تعذّر جلب الرسائل
errors-chat-summarize-empty = لا يمكن تلخيص جلسة محادثة فارغة
errors-chat-provider-invalid = مزوّد غير صالح: { $provider }
errors-chat-summary-save-failed = تعذّر حفظ الملخص
errors-chat-title-empty-session = لا يمكن إنشاء عنوان لجلسة محادثة فارغة
errors-chat-no-user-message = لم يتم العثور على رسائل مستخدم في الجلسة
errors-chat-title-save-failed = تعذّر تحديث عنوان الجلسة
errors-chat-confirmation-save-failed = تعذّر حفظ رد التأكيد
errors-chat-confirmation-failed = تعذّرت معالجة التأكيد
errors-chat-summary-failed = تعذّر إنشاء الملخص

# Assistant automation
errors-automation-database-unavailable = قاعدة البيانات غير مهيأة
errors-automation-tasks-list-failed = تعذّر عرض المهام
errors-automation-name-empty = لا يمكن أن يكون اسم المهمة فارغًا
errors-automation-instruction-empty = لا يمكن أن تكون تعليمات المهمة فارغة
errors-automation-schedule-type-invalid = schedule_type غير صالح. يجب أن يكون: interval أو daily أو weekly
errors-automation-schedule-value-invalid = schedule_value غير صالح
errors-automation-task-create-failed = تعذّر إنشاء المهمة
errors-automation-task-not-found = لم يتم العثور على المهمة
errors-automation-task-get-failed = تعذّر جلب المهمة
errors-automation-schedule-invalid = جدولة غير صالحة
errors-automation-tool-permissions-invalid = يجب أن تكون tool_permissions هي 'full' أو 'readonly'
errors-automation-priority-invalid = يجب أن تكون priority هي 'low' أو 'medium' أو 'high'
errors-automation-task-update-failed = تعذّر تحديث المهمة
errors-automation-task-running-delete = لا يمكن حذف المهمة أثناء تشغيلها
errors-automation-task-delete-failed = تعذّر حذف المهمة
errors-automation-task-toggle-failed = تعذّر تبديل حالة المهمة
errors-automation-task-disabled = لا يمكن تشغيل مهمة معطّلة
errors-automation-task-already-running = المهمة قيد التشغيل بالفعل
errors-automation-runs-list-failed = تعذّر عرض التشغيلات
errors-automation-recent-runs-failed = تعذّر عرض التشغيلات الأخيرة
errors-automation-run-not-found = لم يتم العثور على التشغيل
errors-automation-run-get-failed = تعذّر جلب التشغيل
errors-automation-stats-failed = تعذّر جلب الإحصاءات

# LLM providers
errors-llm-config-update-failed = تعذّر تحديث إعدادات LLM
errors-llm-provider-unknown = مزوّد غير معروف: { $provider }
errors-llm-manager-unavailable = مدير LLM غير مهيأ
errors-llm-provider-disabled = المزوّد '{ $provider }' غير مهيأ أو معطّل
errors-llm-provider-config-update-failed = تعذّر تحديث إعدادات المزوّد
errors-llm-provider-test-failed = فشل اختبار المزوّد
# The reason is the provider's own wording, shown exactly as sent.
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = تعذّر تحديث إعدادات التحليل
errors-llm-analysis-unavailable = محرك التحليل غير مهيأ
errors-llm-analysis-disabled = ميزات LLM معطّلة. فعّل [llm] أولًا.
errors-llm-analysis-priority-invalid = أولوية غير صالحة: '{ $priority }'. استخدم 'high' أو 'medium' أو 'low'.
errors-llm-analysis-evaluation-failed = فشل تحليل النموذج
errors-llm-analysis-instructions-list-failed = تعذّر عرض التعليمات
errors-llm-analysis-instruction-not-found = لم يتم العثور على التعليمات { $id }
errors-llm-analysis-instruction-get-failed = تعذّر جلب التعليمات
errors-llm-analysis-instruction-created-retrieve-failed = تعذّر استرجاع التعليمات المنشأة
errors-llm-analysis-instruction-create-failed = تعذّر إنشاء التعليمات
errors-llm-analysis-instruction-updated-retrieve-failed = تعذّر استرجاع التعليمات المحدَّثة
errors-llm-analysis-instruction-update-failed = تعذّر تحديث التعليمات
errors-llm-analysis-instruction-delete-failed = تعذّر حذف التعليمات
errors-llm-analysis-instructions-reorder-failed = تعذّرت إعادة ترتيب التعليمات
errors-llm-analysis-decisions-list-failed = تعذّر عرض سجل القرارات
errors-llm-analysis-decision-not-found = لم يتم العثور على القرار { $id }
errors-llm-analysis-decision-get-failed = تعذّر جلب القرار

# Wallets
errors-wallets-list-failed = تعذّر عرض المحافظ
errors-wallets-name-empty = لا يمكن أن يكون اسم المحفظة فارغًا
errors-wallets-create-failed = تعذّر إنشاء المحفظة
errors-wallets-key-empty = لا يمكن أن يكون المفتاح الخاص فارغًا
errors-wallets-already-exists = المحفظة موجودة بالفعل
errors-wallets-key-invalid = صيغة المفتاح الخاص غير صالحة
errors-wallets-import-failed = تعذّر استيراد المحفظة
errors-wallets-summary-failed = تعذّر جلب ملخص المحافظ
errors-wallets-no-main-wallet = لم يتم ضبط محفظة رئيسية
errors-wallets-main-get-failed = تعذّر جلب المحفظة الرئيسية
errors-wallets-not-found = لم يتم العثور على المحفظة
errors-wallets-get-failed = تعذّر جلب المحفظة
errors-wallets-update-failed = تعذّر تحديث المحفظة
errors-wallets-delete-failed = تعذّر حذف المحفظة
errors-wallets-export-failed = تعذّر تصدير المحفظة
errors-wallets-set-main-failed = تعذّر تعيين المحفظة الرئيسية
errors-wallets-archive-failed = تعذّرت أرشفة المحفظة
errors-wallets-restore-failed = تعذّرت استعادة المحفظة
errors-wallets-export-format-unsupported = صيغة CSV فقط مدعومة حاليًا
# The confirmation is the exact phrase the request must carry.
errors-wallets-export-confirmation-required = يجب التأكيد بتقديم العبارة: "{ $confirmation }"
errors-wallets-export-no-ids = لم يتم تقديم معرّفات محافظ
errors-wallets-export-bulk-failed = تعذّر تصدير المحافظ
errors-wallets-export-no-match = لم يتم العثور على محافظ تطابق المعرّفات المقدمة
errors-wallets-import-file-too-large = يتجاوز الملف الحد الأقصى للحجم وهو { $megabytes }MB
errors-wallets-import-read-failed = تعذّرت قراءة الملف المرفوع
errors-wallets-import-no-file = لم يتم رفع ملف. استخدم الحقل 'file' في النموذج متعدد الأجزاء
errors-wallets-import-encoding-invalid = يجب أن يكون ملف CSV بترميز UTF-8
errors-wallets-import-csv-parse-failed = تعذّر تحليل ملف CSV
errors-wallets-import-excel-parse-failed = تعذّر تحليل ملف Excel
errors-wallets-import-format-unsupported = صيغة الملف غير مدعومة. استخدم .csv أو .xlsx أو .xls
errors-wallets-import-file-empty = الملف لا يحتوي على صفوف بيانات
errors-wallets-import-existing-check-failed = تعذّر فحص المحافظ الموجودة
errors-wallets-import-mapping-invalid = أعمدة مطلوبة مفقودة: { $columns }
errors-wallets-import-session-not-found = جلسة الاستيراد غير موجودة أو منتهية. يرجى رفع الملف مرة أخرى
errors-wallets-import-no-valid-rows = لا توجد صفوف صالحة للاستيراد

# Wallet watching
errors-wallet-watch-list-failed = تعذّر عرض أهداف المراقبة
errors-wallet-watch-address-empty = لا يمكن أن يكون العنوان فارغًا
errors-wallet-watch-add-failed = تعذّرت إضافة هدف المراقبة
errors-wallet-watch-remove-failed = تعذّرت إزالة هدف المراقبة
errors-wallet-watch-update-failed = تعذّر تحديث هدف المراقبة
errors-wallet-watch-budget-failed = تعذّر تحديث ميزانية المراقبة
errors-wallet-watch-resume-failed = تعذّر استئناف المراقبة
errors-wallet-watch-approval-failed = تعذّر تحديث موافقة { -helius }
errors-wallet-watch-status-failed = تعذّر جلب حالة المراقبة

# Tools
errors-tools-wallet-failed = تعذّر جلب المحفظة
errors-tools-wallet-address-failed = تعذّر جلب عنوان المحفظة
errors-tools-accounts-scan-failed = تعذّر فحص الحسابات
errors-tools-token-accounts-scan-failed = تعذّر فحص حسابات الرموز
errors-tools-token-accounts-get-failed = تعذّر جلب حسابات الرموز
errors-tools-cleanup-failed = فشل التنظيف
errors-tools-cache-clear-failed = تعذّر مسح الذاكرة المؤقتة
errors-tools-no-tokens = لم يتم تحديد رموز للحرق
errors-tools-favorites-list-failed = تعذّر جلب المفضلات
errors-tools-favorite-type-invalid = نوع الأداة غير صالح. يجب أن يكون أحد: { $types }
errors-tools-favorite-add-failed = تعذّرت إضافة المفضلة
errors-tools-favorite-not-found = لم يتم العثور على المفضلة
errors-tools-favorite-update-failed = تعذّر تحديث المفضلة
errors-tools-favorite-delete-failed = تعذّر حذف المفضلة
errors-tools-favorite-use-failed = تعذّر تحديث عدد الاستخدام
errors-tools-pool-search-failed = فشل البحث عن مجمعات السيولة للرمز { $mint }
errors-tools-watched-list-failed = تعذّر عرض الرموز المراقبة
errors-tools-watched-add-failed = تعذّرت إضافة الرمز المراقب
errors-tools-watched-delete-failed = تعذّر حذف الرمز المراقب
errors-tools-mint-invalid = عنوان إصدار الرمز غير صالح
errors-tools-wallets-get-failed = تعذّر جلب المحافظ
errors-tools-balance-failed = تعذّر جلب رصيد المحفظة
errors-tools-session-active = عملية متعددة المحافظ أخرى قيد التنفيذ بالفعل
# The reason is the tool configuration check's own wording, shown exactly as produced.
errors-tools-config-invalid = إعدادات الأداة غير صالحة: { $reason }
errors-tools-config-rejected = إعدادات الأداة غير صالحة
errors-tools-consolidate-failed = تعذّر تجميع المحافظ
errors-tools-ata-cleanup-failed = تعذّر تنظيف حسابات ATA
errors-tools-routers-unavailable = موجّهات المبادلة غير جاهزة بعد
errors-tools-router-disabled-chain-settings = { $router } معطّل في الإعدادات > السلاسل
errors-tools-router-unknown = موجّه مبادلة غير معروف '{ $router }'
errors-tools-session-type-mismatch = الجلسة من نوع { $actual } وليست { $expected }
errors-tools-session-not-found = لم يتم العثور على الجلسة
errors-tools-session-complete = الجلسة مكتملة بالفعل

# Configuration import and reload
errors-config-reload-failed = تعذّرت إعادة تحميل الإعدادات
errors-config-reset-failed = تعذّرت إعادة ضبط الإعدادات
errors-config-disk-parse-failed = تعذّر تحليل إعدادات القرص
errors-config-disk-read-failed = تعذّرت قراءة إعدادات القرص
errors-config-update-failed = تعذّر تحديث الإعدادات
errors-config-import-not-object = يجب أن تكون الإعدادات كائن JSON
errors-config-import-no-sections = لم يتم العثور على أقسام صالحة للاستيراد
errors-config-import-validation-failed = فشل التحقق من الإعدادات. لم يتم تطبيق أي تغييرات.
errors-config-import-commit-failed = تعذّر اعتماد تغييرات الإعدادات
errors-config-import-failed = تعذّر استيراد الإعدادات

# Filtering
errors-filtering-analytics-failed = تعذّر جلب التحليلات
errors-filtering-refresh-failed = تعذّرت إعادة بناء لقطة الترشيح
errors-filtering-rejection-stats-failed = تعذّر جلب إحصاءات الرفض
errors-filtering-rejected-tokens-failed = تعذّر جلب الرموز المرفوضة
errors-filtering-csv-header-failed = تعذّرت كتابة ترويسة CSV
errors-filtering-csv-record-failed = تعذّرت كتابة سجل CSV
errors-filtering-csv-finalize-failed = تعذّر إنهاء ملف CSV
errors-filtering-export-response-failed = تعذّر إنشاء الاستجابة

# OHLCV
errors-ohlcv-fetch-failed = تعذّر جلب بيانات OHLCV
errors-ohlcv-pools-failed = تعذّر جلب المجمعات
errors-ohlcv-gaps-failed = تعذّر جلب الفجوات
errors-ohlcv-refresh-failed = تعذّر التحديث
errors-ohlcv-monitor-start-failed = تعذّر بدء المراقبة
errors-ohlcv-monitor-stop-failed = تعذّر إيقاف المراقبة
errors-ohlcv-activity-failed = تعذّر تسجيل النشاط
errors-ohlcv-list-failed = تعذّر عرض رموز OHLCV
errors-ohlcv-delete-failed = تعذّر حذف بيانات الرمز
errors-ohlcv-clear-failed = تعذّر مسح ذاكرة OHLCV المؤقتة
errors-ohlcv-cleanup-failed = تعذّر تنظيف الرموز غير النشطة

# Trader and manual trading
errors-trade-already-running = المتداول الآلي قيد التشغيل بالفعل
errors-trade-already-stopped = المتداول الآلي متوقف بالفعل
errors-trade-config-update-failed = فشل تحديث إعدادات المتداول الآلي
errors-trade-trader-unavailable = أكمل إعداد المحفظة وRPC قبل استخدام المتداول الآلي
errors-trade-force-stop-active = الإيقاف الطارئ نشط؛ ألغِه أولًا
errors-trade-template-not-found = لا يوجد قالب متداول آلي باسم { $template }
errors-trade-manual-force-stopped = التداول اليدوي معطّل أثناء نشاط الإيقاف الطارئ
errors-trade-core-services-not-ready = الخدمات الأساسية غير جاهزة للتداول: { $pending }
errors-trade-mint-invalid = عنوان إصدار الرمز { $mint } غير صالح
errors-trade-blacklisted = الرمز { $mint } في القائمة السوداء
errors-trade-slippage-invalid = يجب أن يكون الانزلاق السعري { $slippage }% ضمن ⁨(0, { $maximum }]⁩
errors-trade-percentage-invalid = يجب أن تكون نسبة البيع { $percentage } ضمن ⁨(0, 100]⁩
errors-trade-record-failed = تعذّر تسجيل الصفقة اليدوية
errors-trade-no-open-position = لا يوجد مركز مفتوح للرمز { $mint }
errors-trade-size-invalid = حجم الصفقة { $amount } { -sol } غير صالح
errors-trade-management-invalid = إدارة المركز { $management } غير صالحة
errors-trade-strategy-evaluation-failed = فشل تقييم الاستراتيجية للرمز { $mint }
errors-trade-token-data-missing = بيانات الرمز غير متاحة لـ { $mint }
errors-trade-endpoints-unhealthy = لا توجد نقاط اتصال سليمة متاحة
errors-trade-dependency-failed = فشلت التبعية { $dependency }
errors-trade-storage-failed = تعذّر إكمال طلب الصفقة
errors-trade-manual-failed = فشلت الصفقة اليدوية
# The reason is the trader's own wording for a refused trade, shown exactly as produced.
errors-trade-manual-refused = { $reason }
errors-trade-wallet-not-configured = المحفظة غير مهيأة
errors-trade-amount-sol-invalid = amount_sol مطلوب للشراء ويجب أن يكون موجبًا
errors-trade-no-tokens-in-wallet = لم يتم العثور على رموز في المحفظة لهذا المركز. رصيد الرمز 0؛ لا يمكن إغلاق المركز عبر المبادلة.
errors-trade-percentage-range = يجب أن تكون النسبة ضمن ⁨(0, 100]⁩
errors-trade-amount-tokens-invalid = يجب أن يكون amount_tokens موجبًا
errors-trade-sell-amount-zero = كمية البيع المحسوبة تساوي صفرًا

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = توجيه المبادلة غير جاهز بعد
    .hint = لا تزال خدمة المبادلة قيد البدء. انتظر حتى تصبح الخدمات جاهزة ثم أعد المحاولة.
errors-trade-quote-no-routers-enabled = لا يوجد مزوّدو مبادلة مفعّلون
    .hint = فعّل موجّه مبادلة واحدًا على الأقل في إعدادات المتداول ثم حاول مرة أخرى.
errors-trade-quote-not-tradable = هذا الرمز غير قابل للتداول الآن
    .hint = لا توجد سيولة أو مسار مبادلة متاح. قد يكون الرمز غير مطلق أو مهجورًا أو بلا مجمع سيولة. حاول لاحقًا أو اختر رمزًا آخر.
errors-trade-quote-no-route = لا يوجد مسار مبادلة متاح
    .hint = لم يتمكن أي مزوّد من توجيه هذه الصفقة بالمبلغ المطلوب. جرّب مبلغًا أصغر أو حاول مرة أخرى بعد قليل.
errors-trade-quote-rate-limited = مزوّدو المبادلة يقيّدون معدل طلباتنا
    .hint = يقوم مزوّدو المبادلة بتقييد الطلبات. انتظر بضع ثوانٍ ثم أعد المحاولة.
errors-trade-quote-timeout = انتهت مهلة طلب عرض السعر
    .hint = لم يستجب مزوّدو المبادلة في الوقت المناسب. تحقق من اتصالك وأعد المحاولة.
errors-trade-quote-router-rejected = تم رفض عرض السعر
    .hint = أعاد أحد المزوّدين عرض سعر لم يجتز فحوصات الأمان لدينا فتم تجاهله. أعد المحاولة لجلب عرض جديد.
errors-trade-quote-unavailable = تعذّر جلب عرض السعر
    .hint = لم يتمكن مزوّدو المبادلة من تسعير هذه الصفقة. حاول مرة أخرى بعد قليل.

# Positions
errors-positions-not-found = لم يتم العثور على المركز
errors-positions-already-closed = المركز مغلق بالفعل
errors-positions-already-archived = المركز مؤرشف بالفعل
errors-positions-not-archived = المركز غير مؤرشف
errors-positions-archive-failed = تعذّرت أرشفة المركز
errors-positions-unarchive-failed = تعذّر إلغاء أرشفة المركز
errors-positions-management-invalid = الإدارة المملوكة للنسخ تتطلب مركزًا من أصل نسخ
errors-positions-management-failed = تعذّر تحديث إدارة المركز
errors-positions-delete-failed = تعذّر حذف المركز
errors-positions-bulk-delete-failed = تعذّر حذف المراكز المؤرشفة
errors-positions-detail-failed = تعذّر تحميل تفاصيل المركز
errors-positions-resolve-failed = تعذّر حل المركز
errors-positions-wrapped-sol-activity = SOL الملفوف ليس له نشاط رمز

# Tokens
errors-tokens-database-unavailable = قاعدة بيانات الرموز غير متاحة
errors-tokens-blacklist-failed = تعذّرت إضافة الرمز إلى القائمة السوداء
errors-tokens-blacklist-internal = خطأ داخلي أثناء عملية القائمة السوداء
errors-tokens-unblacklist-failed = تعذّرت الإزالة من القائمة السوداء
errors-tokens-unblacklist-internal = خطأ داخلي أثناء عملية الإزالة من القائمة السوداء
errors-tokens-blacklist-status-failed = تعذّر فحص حالة القائمة السوداء
errors-tokens-blacklist-status-internal = خطأ داخلي أثناء فحص حالة القائمة السوداء
errors-tokens-favorites-fetch-failed = تعذّر جلب المفضلات
errors-tokens-favorite-add-failed = تعذّرت إضافة المفضلة
errors-tokens-favorite-remove-failed = تعذّرت إزالة المفضلة
errors-tokens-favorite-update-failed = تعذّر تحديث المفضلة
errors-tokens-detail-not-found = لم يتم العثور على الرمز في قاعدة البيانات أو المصادر الخارجية
errors-tokens-fetch-failed = تعذّر جلب الرمز
errors-tokens-refresh-all-failed = فشلت جميع مصادر البيانات
errors-tokens-refresh-failed = تعذّر تحديث الرمز
errors-tokens-search-query-required = استعلام البحث 'q' مطلوب
errors-tokens-search-failed = فشل البحث عن الرموز

# Actions and services
errors-actions-not-found = لم يتم العثور على الإجراء { $id }
errors-services-not-found = لم يتم العثور على الخدمة '{ $name }'
