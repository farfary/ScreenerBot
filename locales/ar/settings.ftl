## Shared

settings-duration-minutes =
    { $count ->
        [zero] { $count } دقيقة
        [one] { $count } دقيقة
        [two] { $count } دقيقتان
        [few] { $count } دقائق
        [many] { $count } دقيقة
       *[other] { $count } دقيقة
    }
settings-duration-hours =
    { $count ->
        [zero] { $count } ساعة
        [one] { $count } ساعة
        [two] { $count } ساعتان
        [few] { $count } ساعات
        [many] { $count } ساعة
       *[other] { $count } ساعة
    }

## settings_dialog.js

settings-dialog-title = الإعدادات
settings-dialog-close =
    .title = إغلاق (ESC)
    .aria-label = إغلاق الإعدادات
settings-dialog-save = حفظ التغييرات
settings-dialog-saving = جارٍ الحفظ...
settings-dialog-saved = تم الحفظ
settings-dialog-save-success = تم حفظ الإعدادات بنجاح
settings-dialog-save-failed = فشل حفظ الإعدادات
settings-dialog-update-attention = التحديث يحتاج إلى انتباه
settings-dialog-tab-interface = الواجهة
settings-dialog-tab-navigation = التنقل
settings-dialog-tab-startup = بدء التشغيل
settings-dialog-tab-hints = التلميحات
settings-dialog-tab-data = البيانات
settings-dialog-tab-security = الأمان
settings-dialog-tab-account = الحساب
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = اتصالات الوكلاء
settings-dialog-tab-updates = التحديثات
settings-dialog-tab-licenses = التراخيص
settings-dialog-tab-about = حول
settings-dialog-link-privacy = سياسة الخصوصية
settings-dialog-link-terms = شروط الخدمة

## settings_dialog.js: Startup tab

settings-startup-section-title = سلوك بدء التشغيل
settings-startup-auto-start-label = تشغيل المتداول الآلي تلقائيًا
settings-startup-auto-start-hint = تشغيل المتداول الآلي تلقائيًا عند فتح التطبيق
settings-startup-coming-soon = قريبًا
settings-startup-default-page-label = الصفحة الافتراضية
settings-startup-default-page-hint = الصفحة التي تظهر عند فتح التطبيق
settings-startup-page-dashboard = لوحة التحكم
settings-startup-page-tokens = الرموز
settings-startup-page-positions = المراكز
settings-startup-page-wallet = المحفظة
settings-startup-page-config = الإعدادات
settings-startup-notifications-label = إظهار إشعارات الخلفية
settings-startup-notifications-hint = عرض إشعارات لأحداث الخلفية

## settings_dialog.js: About tab

settings-about-tagline = محرك تداول أصلي على Solana
settings-about-link-github = { -github }
settings-about-link-docs = التوثيق
settings-about-link-telegram = { -telegram }
settings-about-link-website = الموقع الإلكتروني
settings-about-credits = صُنع لمتداولي Solana
settings-about-copyright = © { $year } { -brand }. جميع الحقوق محفوظة.

## interface_tab.js

settings-interface-section-appearance = المظهر
settings-interface-theme-label = السمة
settings-interface-theme-hint = اختر نظام الألوان المفضل لديك
settings-interface-theme-dark = داكن
settings-interface-theme-light = فاتح
settings-interface-language-label = اللغة
settings-interface-language-hint = لغة عرض لوحة التحكم
settings-interface-logo-shape-label = شكل شعار الرمز
settings-interface-logo-shape-hint = الدائري يقصّ كل شعار، والطبيعي يحافظ على الشكل الأصلي لكل تصميم
settings-interface-logo-shape-circle = دائري
settings-interface-logo-shape-natural = طبيعي
settings-interface-animations-label = تفعيل الحركات
settings-interface-animations-hint = انتقالات وتأثيرات سلسة
settings-interface-compact-label = الوضع المضغوط
settings-interface-compact-hint = تقليل الهوامش لعرض محتوى أكثر
settings-interface-section-data = البيانات والعرض
settings-interface-refresh-label = فترة التحديث
settings-interface-refresh-hint = عدد مرات تحديث البيانات
settings-interface-refresh-seconds =
    { $count ->
        [zero] { $count } ثانية
        [one] { $count } ثانية
        [two] { $count } ثانيتان
        [few] { $count } ثوانٍ
        [many] { $count } ثانية
       *[other] { $count } ثانية
    }
settings-interface-refresh-minutes =
    { $count ->
        [zero] { $count } دقيقة
        [one] { $count } دقيقة
        [two] { $count } دقيقتان
        [few] { $count } دقائق
        [many] { $count } دقيقة
       *[other] { $count } دقيقة
    }
settings-interface-ticker-label = إظهار شريط المؤشرات
settings-interface-ticker-hint = شريط مؤشرات مباشر في الترويسة
settings-interface-page-size-label = حجم صفحة الجدول
settings-interface-page-size-hint = عدد الصفوف الافتراضي في كل صفحة جدول
settings-interface-page-size-rows =
    { $count ->
        [zero] { $count } صف
        [one] { $count } صف
        [two] { $count } صفان
        [few] { $count } صفوف
        [many] { $count } صفًا
       *[other] { $count } صف
    }
settings-interface-auto-expand-label = توسيع الفئات تلقائيًا
settings-interface-auto-expand-hint = توسيع فئات الإعدادات افتراضيًا
settings-interface-hints-label = إظهار التلميحات السياقية
settings-interface-hints-hint = عرض أيقونات المساعدة التي تشرح ميزات لوحة التحكم
settings-interface-featured-label = إظهار صف الرموز المميّزة
settings-interface-featured-hint = عرض صف الرموز المميّزة في صفحتي الرئيسية والرموز
settings-interface-section-sound = المؤثرات الصوتية
settings-interface-sounds-label = تفعيل الأصوات
settings-interface-sounds-hint = إشارات ملموسة للتنقل وتغيّر الحالة والنتائج

## security_tab.js

settings-security-loading = جارٍ تحميل إعدادات الأمان...
settings-security-load-failed = فشل تحميل إعدادات الأمان

settings-security-type-pin4 = رمز PIN من 4 أرقام
settings-security-type-pin6 = رمز PIN من 6 أرقام
settings-security-type-text = كلمة مرور نصية
settings-security-type-unset = غير معيّنة

settings-security-lockscreen-title = شاشة قفل لوحة التحكم
settings-security-lockscreen-description = احمِ لوحة التحكم برمز PIN أو كلمة مرور. ستظهر شاشة القفل عند تفعيلها، ويلزم التحقق من الهوية للمتابعة.
settings-security-enable-label = تفعيل شاشة القفل
settings-security-enable-hint = احمِ لوحة التحكم بالمصادقة بكلمة مرور
settings-security-password-status-label = حالة كلمة المرور
settings-security-password-current = الحالية: { $type }
settings-security-password-none = لم تُعيَّن كلمة مرور
settings-security-change = تغيير
settings-security-remove = إزالة
settings-security-set-password = تعيين كلمة المرور
settings-security-auto-lock-label = القفل التلقائي بعد الخمول
settings-security-auto-lock-hint = القفل تلقائيًا بعد فترة من عدم النشاط
settings-security-auto-lock-never = أبدًا
settings-security-lock-blur-label = القفل عند فقدان تركيز النافذة
settings-security-lock-blur-hint = القفل تلقائيًا عند التبديل إلى تطبيق آخر
settings-security-quick-actions-title = إجراءات سريعة
settings-security-lock-now-label = قفل لوحة التحكم الآن
settings-security-lock-now-hint = قفل لوحة التحكم فورًا
settings-security-lock-now = قفل الآن
settings-security-lock-not-ready = تعذّر القفل - شاشة القفل غير جاهزة
settings-security-setting-save-failed = تعذّر حفظ إعداد الأمان

## security_tab.js: two-factor authentication

settings-security-2fa-title = المصادقة الثنائية
settings-security-2fa-description = أضف طبقة أمان إضافية باستخدام تطبيق مصادقة (Google Authenticator وAuthy وغيرهما)
settings-security-2fa-status-label = حالة 2FA
settings-security-2fa-status-enabled = المصادقة الثنائية مفعّلة
settings-security-2fa-status-none = غير مهيأة
settings-security-2fa-disable = تعطيل 2FA
settings-security-2fa-enable = تفعيل 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = إغلاق
settings-security-password-set-title = تعيين كلمة المرور
settings-security-password-change-title = تغيير كلمة المرور
settings-security-password-current-label = كلمة المرور الحالية
settings-security-password-current-input =
    .placeholder = أدخل كلمة المرور الحالية
settings-security-password-type-label = نوع كلمة المرور
settings-security-password-new-label = كلمة المرور الجديدة
settings-security-password-new-input =
    .placeholder = أدخل كلمة المرور الجديدة
settings-security-password-confirm-label = تأكيد كلمة المرور
settings-security-password-confirm-input =
    .placeholder = أكّد كلمة المرور
settings-security-password-update = تحديث كلمة المرور
settings-security-placeholder-pin4 = أدخل رمز PIN من 4 أرقام
settings-security-placeholder-pin6 = أدخل رمز PIN من 6 أرقام
settings-security-placeholder-text = أدخل كلمة المرور
settings-security-password-required = يرجى إدخال كلمة المرور
settings-security-password-mismatch = كلمتا المرور غير متطابقتين
settings-security-pin4-invalid = يجب أن يتكون رمز PIN من 4 أرقام بالضبط
settings-security-pin6-invalid = يجب أن يتكون رمز PIN من 6 أرقام بالضبط
settings-security-text-too-short = يجب ألا تقل كلمة المرور عن 4 أحرف
settings-security-password-saved = تم حفظ كلمة المرور
settings-security-password-save-failed = فشل حفظ كلمة المرور
settings-security-password-save-failed-detail = فشل حفظ كلمة المرور: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = إزالة كلمة المرور
settings-security-remove-description = أدخل كلمة المرور الحالية لإزالة حماية شاشة القفل.
settings-security-remove-confirm = إزالة كلمة المرور
settings-security-current-required = يرجى إدخال كلمة المرور الحالية
settings-security-password-removed = تمت إزالة كلمة المرور
settings-security-password-remove-failed = فشلت إزالة كلمة المرور
settings-security-password-remove-failed-detail = فشلت إزالة كلمة المرور: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = تفعيل المصادقة الثنائية
settings-security-2fa-password-prompt = أدخل كلمة المرور للمتابعة:
settings-security-2fa-password-input =
    .placeholder = أدخل كلمة المرور
settings-security-2fa-continue = متابعة
settings-security-2fa-manual-code = رمز الإدخال اليدوي:
settings-security-2fa-qr =
    .alt = رمز QR الخاص بـ TOTP
settings-security-2fa-code-prompt = أدخل رمز التحقق المكوّن من 6 أرقام من تطبيق المصادقة:
settings-security-2fa-verify-enable = تحقق وفعّل
settings-security-2fa-password-required = يرجى إدخال كلمة المرور
settings-security-2fa-setup-failed = فشل إعداد 2FA
settings-security-2fa-code-invalid-length = يرجى إدخال رمز تحقق مكوّن من 6 أرقام
settings-security-2fa-code-invalid = رمز التحقق غير صالح
settings-security-2fa-enabled = تم تفعيل المصادقة الثنائية
settings-security-2fa-verify-failed = فشل التحقق من الرمز
settings-security-2fa-disable-title = تعطيل المصادقة الثنائية
settings-security-2fa-disable-prompt = أدخل كلمة المرور لتعطيل 2FA:
settings-security-2fa-disable-failed = فشل تعطيل 2FA
settings-security-2fa-disabled = تم تعطيل المصادقة الثنائية

## agent_connections_tab.js

settings-agent-category-analysis = التحليل
settings-agent-category-portfolio = المحفظة
settings-agent-category-trading = التداول
settings-agent-category-config = الإعدادات
settings-agent-category-system = النظام
settings-agent-category-analysis-description = تحليل الرموز وبيانات السوق وفحوصات الأمان.
settings-agent-category-portfolio-description = المراكز المفتوحة والأرصدة والأرباح والخسائر.
settings-agent-category-trading-description = شراء المراكز وبيعها وإغلاقها بأموال حقيقية.
settings-agent-category-config-description = كل إعدادات البوت، بما فيها نقاط اتصال RPC. لا تشمل مفاتيح المحفظة إطلاقًا.
settings-agent-category-system-description = الحالة والأحداث والإيقاف الطارئ.
settings-agent-category-analysis-inline = التحليل
settings-agent-category-portfolio-inline = المحفظة
settings-agent-category-trading-inline = التداول
settings-agent-category-config-inline = الإعدادات
settings-agent-category-system-inline = النظام

settings-agent-level-allow = سماح
settings-agent-level-ask-user = سؤال
settings-agent-level-deny = معطّل
settings-agent-level-allow-hint = يُنفَّذ فورًا.
settings-agent-level-ask-user-hint = ينتظر موافقتك داخل التطبيق.
settings-agent-level-deny-hint = مرفوض ومخفي عن الوكيل.

settings-agent-preset-full = وصول كامل
settings-agent-preset-ask = السؤال أولًا
settings-agent-preset-read = للقراءة فقط
settings-agent-preset-full-description = كل شيء يُنفَّذ دون سؤال. تبقى مفاتيح المحفظة بعيدة المنال.
settings-agent-preset-ask-description = كل إجراء ينتظر موافقتك داخل التطبيق.
settings-agent-preset-read-description = قراءة التحليل والمحفظة. لا يمكن تغيير أي شيء.
settings-agent-preset-custom = مخصص
settings-agent-preset-group =
    .aria-label = إعداد الصلاحيات المسبق
settings-agent-permission-group = صلاحية { $category }

settings-agent-summary-asks-only = محدود — يطلب الموافقة على: { $asking }
settings-agent-summary-off-only = محدود — بدون: { $off }
settings-agent-summary-asks-and-off = محدود — يطلب الموافقة على: { $asking }؛ بدون: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP عام عبر stdio

settings-agent-note-placeholder = استبدل /absolute/path/to/screenerbot بالمسار المطلق للملف التنفيذي لـ { -brand } — تعذّر على التطبيق العامل تمثيل مسار ملفه التنفيذي على هذا النظام.
settings-agent-note-data-dir = إذا شغّلت { -brand } بمجلد بيانات غير افتراضي، فاضبط أيضًا SCREENERBOT_DATA_DIR في العميل (عبر وسيط -e / --env آخر، أو عنصر env) على المسار نفسه.
settings-agent-note-codex-run = شغّل الأمر، أو أضف كتلة TOML إلى ~/.codex/config.toml ($CODEX_HOME/config.toml). ثم أعد تشغيل { -codex }.
settings-agent-note-codex-get = الأمر `codex mcp get screenerbot` يخفي السر في مخرجاته.
settings-agent-note-claude-code = { -claude } Code: شغّل الأمر ثم أعد تشغيل { -claude } Code. الأمر `claude mcp get screenerbot` سيطبع البيئة المهيأة بما فيها السر.
settings-agent-note-claude-desktop = { -claude } Desktop: ادمج JSON في claude_desktop_config.json ضمن `mcpServers` ثم أعد تشغيل التطبيق.
settings-agent-note-openclaw = شغّل الأمر، ثم استخدم `openclaw mcp doctor screenerbot --probe` للتحقق من أن خادم stdio المحفوظ يبدأ ويعرض الأدوات.
settings-agent-note-hermes = أضف هذا ضمن `mcp_servers` في ملف إعدادات { -hermes } ثم أعد تشغيل { -hermes }.
settings-agent-note-generic = أي عميل MCP يدعم stdio: شغّل هذا الأمر بهذه الوسائط والبيئة، أينما يحفظ العميل قائمة خوادمه.
settings-agent-block-codex-command = { -codex } CLI — أمر الطرفية
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (بديل)
settings-agent-block-claude-command = { -claude } Code — أمر الطرفية
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — أمر الطرفية
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = عميل MCP عام عبر stdio

settings-agent-name-required = أدخل اسمًا لهذا الاتصال.
settings-agent-name-too-long = يجب ألا يتجاوز الاسم { $max } من الأحرف.
settings-agent-name-control-characters = يجب ألا يحتوي الاسم على أحرف تحكم.

settings-agent-title = اتصالات الوكلاء
settings-agent-description = اربط { -claude } أو { -codex } أو { -hermes } أو { -openclaw } أو أي عميل MCP عبر stdio. يجب أن يبقى { -brand } قيد التشغيل. لكل اتصال صلاحياته الخاصة: وصول كامل افتراضيًا، ويمكن تقييده لكل اتصال متى شئت. لا يمكن لأي اتصال قراءة مفتاح محفظتك أو تغييره.
settings-agent-name-label = اسم الاتصال
settings-agent-name-hint = يظهر في القائمة أدناه لتمييز الاتصالات عن بعضها.
settings-agent-name-input =
    .placeholder = وكيل البرمجة على الحاسوب المحمول
settings-agent-client-label = العميل
settings-agent-client-hint = يحدد الإعداد المعروض بعد إنشاء الاتصال.
settings-agent-permissions-label = الصلاحيات
settings-agent-permissions-hint = يمكن للاتصال الجديد فعل كل شيء. قيّد أي فئة الآن أو لاحقًا من القائمة أدناه — مفاتيح المحفظة غير متاحة في كلتا الحالتين.
settings-agent-create = إنشاء اتصال
settings-agent-issued-group =
    .aria-label = بيانات اعتماد الاتصال الجديد
settings-agent-issued-warning = انسخ السر الآن. يظهر مرة واحدة ولا يمكن استرجاعه مجددًا — إذا فقدته فاسحب الاتصال وأنشئه من جديد. يحتفظ { -brand } بمُحقِّق أحادي الاتجاه فقط، ويخزن عميل MCP النص الصريح ضمن إعداداته.
settings-agent-issued-client-id = معرّف العميل
settings-agent-issued-secret = سر لمرة واحدة
settings-agent-setup-for = الإعداد لـ
settings-agent-done = تم
settings-agent-list-title = الاتصالات
settings-agent-loading = جارٍ تحميل الاتصالات...
settings-agent-active-count = النشطة: { $count }
settings-agent-empty = لا توجد اتصالات بعد. أنشئ اتصالًا أعلاه لإقران عميل.
settings-agent-empty-active = لا توجد اتصالات نشطة.
settings-agent-revoked-title = الاتصالات المسحوبة
settings-agent-created = أُنشئ { $time }
settings-agent-last-used = آخر استخدام { $time }
settings-agent-never-used = لم يُستخدم أبدًا
settings-agent-permissions-edit = الصلاحيات
settings-agent-revoke = سحب
settings-agent-permissions-save = حفظ الصلاحيات

settings-agent-load-failed = فشل تحميل اتصالات الوكلاء
settings-agent-list-failed = تعذّر تحميل الاتصالات
settings-agent-create-failed = تعذّر إنشاء الاتصال.
settings-agent-unreachable-create = تعذّر الوصول إلى { -brand } لإنشاء الاتصال.
settings-agent-permissions-update-failed = تعذّر تحديث الصلاحيات
settings-agent-permissions-updated = تم تحديث الصلاحيات
settings-agent-permissions-updated-detail = تُطبَّق على الطلب التالي للاتصال.
settings-agent-unreachable-save = تعذّر الوصول إلى { -brand } للحفظ
settings-agent-revoke-title = سحب الاتصال
settings-agent-revoke-message = هل تريد سحب «{ $label }»؟ سيتوقف العميل عن العمل عند طلبه التالي ولا يمكن استعادته.
settings-agent-revoke-fallback-name = هذا الاتصال
settings-agent-revoke-failed = تعذّر سحب الاتصال
settings-agent-unreachable-revoke = تعذّر الوصول إلى { -brand } للسحب

## telegram_tab.js

settings-telegram-loading = جارٍ تحميل إعدادات { -telegram }...
settings-telegram-load-failed = فشل تحميل إعدادات { -telegram }
settings-telegram-unknown = غير معروف
settings-telegram-session-active = نشطة: { $duration }
settings-telegram-sessions-empty = لا توجد جلسات نشطة
settings-telegram-session-revoke = سحب

settings-telegram-connection-title = الاتصال
settings-telegram-connection-description = اربط بوت { -telegram } لتلقي الإشعارات والتحكم في { -brand } عن بُعد.
settings-telegram-enable-label = تفعيل { -telegram }
settings-telegram-enable-hint = تفعيل تكامل بوت { -telegram }
settings-telegram-token-label = رمز البوت
settings-telegram-token-saved = تم حفظ الرمز
settings-telegram-token-help = احصل عليه من @BotFather على { -telegram }
settings-telegram-token-input-saved =
    .placeholder = تم حفظ الرمز (أدخل رمزًا جديدًا لتغييره)
settings-telegram-token-input =
    .placeholder = أدخل رمز البوت
settings-telegram-token-toggle =
    .title = إظهار/إخفاء
settings-telegram-chat-label = معرّف الدردشة
settings-telegram-chat-connected = متصل بالدردشة:
settings-telegram-chat-discover-hint = اكتشف معرّف الدردشة تلقائيًا
settings-telegram-chat-change =
    .title = تغيير
settings-telegram-chat-discover = اكتشاف معرّف الدردشة
settings-telegram-discovery-step-add = أضف البوت إلى مجموعة { -telegram }، أو ابدأ دردشة مباشرة معه
settings-telegram-discovery-step-privacy = للمجموعات: راجع @BotFather ← /mybots ← [البوت الخاص بك] ← Bot Settings ← Group Privacy
settings-telegram-discovery-privacy = <strong>وضع الخصوصية معطّل:</strong> يتلقى البوت جميع رسائل المجموعة<br/><strong>وضع الخصوصية مفعّل:</strong> يتلقى البوت الرسائل فقط عند الإشارة إليه بـ @
settings-telegram-discovery-step-send = أرسل أي رسالة (أو أشر إلى البوت بـ @ إذا كان وضع الخصوصية مفعّلًا)
settings-telegram-discovery-listening = جارٍ الاستماع للرسائل...
settings-telegram-discovery-select = اختيار
settings-telegram-chat-id-label = المعرّف:
settings-telegram-language-label = لغة الرسائل
settings-telegram-language-hint = لغة رسائل وأزرار بوت { -telegram }
settings-telegram-language-follow-app = اتباع لغة التطبيق
settings-telegram-test-label = اختبار الاتصال
settings-telegram-test-hint = أرسل رسالة اختبار للتحقق من الإعداد
settings-telegram-test-send = إرسال اختبار
settings-telegram-test-sending = جارٍ الإرسال...

settings-telegram-chat-type-private = خاصة
settings-telegram-chat-type-group = مجموعة
settings-telegram-chat-type-supergroup = مجموعة كبرى
settings-telegram-chat-type-channel = قناة

settings-telegram-auth-title = مصادقة الأوامر
settings-telegram-auth-description = تستخدم أوامر { -telegram } المصادقة الثنائية نفسها المستخدمة في شاشة قفل لوحة التحكم.
settings-telegram-auth-protected = محمي
settings-telegram-auth-disabled = معطّل
settings-telegram-auth-not-configured = غير مهيأ
settings-telegram-auth-error = خطأ
settings-telegram-auth-protected-note = الأوامر محمية بالمصادقة الثنائية لشاشة القفل. عند انتهاء الجلسات، على المستخدمين تقديم رمز تطبيق المصادقة عبر الأمر <code>/login</code>.
settings-telegram-auth-disabled-note = المصادقة الثنائية لشاشة القفل مهيأة لكنها معطلة لـ { -telegram }. فعّل «طلب 2FA للأوامر» أعلاه لحماية أوامر { -telegram }.
settings-telegram-auth-missing-note = المصادقة الثنائية لشاشة القفل غير مهيأة. بدونها ستُعاد الجلسات المنتهية تلقائيًا دون تحقق.
settings-telegram-auth-managed-in = تُدار المصادقة الثنائية في
settings-telegram-auth-configure-in = هيّئ المصادقة الثنائية في
settings-telegram-auth-configure-suffix = لطلب التحقق عند تنفيذ أوامر { -telegram }.
settings-telegram-security-link = إعدادات الأمان
settings-telegram-timeout-title = مهلة الجلسة
settings-telegram-timeout-description = المدة التي تبقى فيها الجلسة الموثقة نشطة
settings-telegram-sessions-title = الجلسات النشطة

settings-telegram-notifications-title = إعدادات الإشعارات
settings-telegram-notifications-description = اختر الأحداث التي تُطلق إشعارات { -telegram }.
settings-telegram-notify-opened-label = فتح مركز
settings-telegram-notify-opened-hint = إشعار عند فتح مركز جديد
settings-telegram-notify-closed-label = إغلاق مركز
settings-telegram-notify-closed-hint = إشعار عند إغلاق مركز
settings-telegram-notify-partial-label = خروج جزئي
settings-telegram-notify-partial-hint = إشعار عند الخروج الجزئي من المراكز
settings-telegram-notify-dca-label = تنفيذ DCA
settings-telegram-notify-dca-hint = إشعار عند تنفيذ أوامر DCA
settings-telegram-notify-errors-label = الأخطاء
settings-telegram-notify-errors-hint = إشعار عند حدوث الأخطاء والإخفاقات
settings-telegram-notify-startup-label = التشغيل/الإيقاف
settings-telegram-notify-startup-hint = إشعار عند بدء البوت أو إيقافه
settings-telegram-notify-filtering-label = تنبيهات الترشيح
settings-telegram-notify-filtering-hint = إشعار عند اجتياز رموز جديدة معايير الترشيح
settings-telegram-notify-trades-label = تنبيهات الصفقات
settings-telegram-notify-trades-hint = إشعار عند الصفقات الكبيرة على الرموز المراقَبة
settings-telegram-notify-daily-label = الملخص اليومي
settings-telegram-notify-daily-hint = تلقي ملخص يومي لنشاط التداول والأرباح والخسائر

settings-telegram-features-title = الميزات
settings-telegram-features-description = اضبط إمكانات بوت { -telegram }.
settings-telegram-commands-label = تفعيل الأوامر
settings-telegram-commands-hint = السماح بالتحكم في البوت عبر أوامر { -telegram }
settings-telegram-require-2fa-label = طلب 2FA للأوامر
settings-telegram-require-2fa-hint = عند انتهاء الجلسات، يُطلب رمز 2FA لإعادة التفعيل. يستخدم 2FA الخاصة بشاشة القفل.
settings-telegram-inline-label = أزرار الإجراءات المضمّنة
settings-telegram-inline-hint = عرض أزرار الإجراءات في رسائل الإشعارات

settings-telegram-setting-save-failed = تعذّر حفظ إعداد { -telegram }
settings-telegram-discovery-start-failed = تعذّر بدء الاكتشاف
settings-telegram-chat-selected = تم اختيار الدردشة
settings-telegram-chat-select-failed = تعذّر اختيار الدردشة
settings-telegram-test-sent = تم إرسال رسالة الاختبار
settings-telegram-test-failed = فشلت رسالة الاختبار
settings-telegram-session-revoked = تم سحب الجلسة
settings-telegram-session-revoke-failed = تعذّر سحب الجلسة

## licenses_tab.js

settings-licenses-title = تراخيص المصدر المفتوح
settings-licenses-subtitle = بُني { -brand } باستخدام برمجيات المصدر المفتوح التالية
settings-licenses-footer = النصوص الكاملة للتراخيص متاحة في مستودع المشروع وفي الشيفرة المصدرية لكل اعتمادية.
settings-licenses-category-framework = إطار التطبيق
settings-licenses-category-solana = بلوكتشين Solana
settings-licenses-category-data = البيانات والتخزين
settings-licenses-category-networking = الشبكات
settings-licenses-category-cryptography = التشفير والترميز
settings-licenses-category-assets = أصول الواجهة
settings-licenses-desc-electron = إطار تطبيقات سطح المكتب
settings-licenses-desc-tokio = بيئة تشغيل غير متزامنة لـ Rust
settings-licenses-desc-axum = إطار خادم الويب
settings-licenses-desc-tower = تجريدات الخدمات
settings-licenses-desc-hyper = تنفيذ HTTP
settings-licenses-desc-solana-sdk = نواة Solana SDK
settings-licenses-desc-solana-client = عميل RPC
settings-licenses-desc-solana-program = مكتبة البرامج
settings-licenses-desc-spl-token = برنامج SPL Token
settings-licenses-desc-spl-token-2022 = إضافات Token-2022
settings-licenses-desc-spl-associated-token-account = حسابات الرموز المرتبطة
settings-licenses-desc-sqlite = محرك قاعدة بيانات مضمّن
settings-licenses-desc-rusqlite = روابط SQLite لـ Rust
settings-licenses-desc-r2d2 = مجمع اتصالات قاعدة البيانات
settings-licenses-desc-serde = إطار التسلسل
settings-licenses-desc-toml = تحليل الإعدادات
settings-licenses-desc-reqwest = عميل HTTP
settings-licenses-desc-tokio-tungstenite = عميل WebSocket
settings-licenses-desc-rustls = تنفيذ TLS
settings-licenses-desc-blake3 = دالة تجزئة
settings-licenses-desc-sha-2 = تجزئة SHA-256/512
settings-licenses-desc-bs58 = ترميز Base58
settings-licenses-desc-base64 = ترميز Base64
settings-licenses-desc-lucide-icons = مكتبة خط الأيقونات
settings-licenses-desc-inter = خط الواجهة
settings-licenses-desc-jetbrains-mono = خط أحادي المسافة
settings-licenses-desc-orbitron = خط العرض
settings-licenses-desc-vazirmatn = خط للعربية والفارسية
settings-licenses-desc-noto-sans-devanagari = خط للديفاناغارية
settings-licenses-desc-noto-sans-sc = خط للصينية المبسطة
settings-licenses-desc-pretendard = خط للكورية
settings-licenses-desc-pretendard-jp = خط لليابانية

## hints_tab.js

settings-hints-title = التلميحات السياقية
settings-hints-description = التلميحات السياقية هي أيقونات المساعدة التي تشرح ميزات لوحة التحكم. راجع كل التلميحات أدناه واستعد ما أخفيته بخيار «عدم الإظهار مجددًا» — واحدًا تلو الآخر أو جميعها معًا.
settings-hints-hidden-label = التلميحات المخفية
settings-hints-hidden-summary = التلميحات المخفية حاليًا: { $hidden } من { $total }.
settings-hints-restore-all = استعادة جميع التلميحات
settings-hints-toggle-shown =
    .title = إظهار هذا التلميح
settings-hints-toggle-shown-title = ظاهر
settings-hints-toggle-hidden-title = مخفي — فعّله للإظهار
settings-hints-restore-title = استعادة جميع التلميحات
settings-hints-restore-message = هل تريد إظهار جميع التلميحات السياقية مجددًا، بما فيها كل ما أخفيته؟
settings-hints-restore-confirm = استعادة الكل
settings-hints-restored = تمت استعادة جميع التلميحات

## account_tab.js

settings-account-title = حساب { -brand }
settings-account-description = مجاني واختياري. يتداول { -brand } ويكتشف ويعرض المخططات دون حساب — لكنه يعتمد حينها على المزوّدين العامين. تسرد اللوحة أدناه ما يضيفه تسجيل الدخول.
settings-account-data-title = بيانات { -brand }
settings-account-data-description = نشغّل خدمة بيانات سوق مشتركة على screenerbot.io: شموع مجمّعة عبر سبعة أطر زمنية، وسجل مجمعات مُحلَّل، وتقارير أمان مخزّنة مؤقتًا، وهوية رموز موحّدة. وُجدت حتى لا تخضع كل نسخة مثبتة لحدود معدل منفصلة لدى المزوّدين العامين، ويتطلب استخدامها حسابًا حتى تُنسب هذه التكلفة المشتركة إلى صاحبها.
settings-account-data-fallback = عند عدم توفرها يعود { -brand } تلقائيًا إلى المزوّدين العامين. لا شيء يتوقف؛ تمتلئ المخططات ببطء أكبر وبسجل أقل.
settings-account-gateway-title = إرسال المعاملات
settings-account-gateway-description = عند تسجيل الدخول، يمكن لـ { -brand } بث مبادلاتك عبر screenerbot.io بدلًا من RPC الخاص بك. يظل البوت يبني ويوقّع كل معاملة على هذا الجهاز — الخادم يمرّرها فقط، ولا يستطيع تغيير معاملة موقّعة دون إبطال توقيعها.
settings-account-gateway-label = استخدام RPC الخاص بـ { -brand } لإرسال المعاملات
settings-account-gateway-hint = للإرسال فقط. تأتي بيانات الأسعار دائمًا من RPC الخاص بك — الاستعلام الدوري عن المجمعات ثقيل جدًا على نقطة اتصال مشتركة، لذا لا يُرسل إليها أبدًا.
settings-account-manage-title = إدارة حسابك
settings-account-manage-description = تُدار كلمة المرور وعنوان البريد الإلكتروني والأجهزة المتصلة ومدفوعات الإحالة على الموقع. سحب جهاز من هناك يسجّل خروجه من كل مكان، بما فيه هذا الجهاز.
settings-account-open-dashboard = افتح لوحة التحكم الخاصة بك

## navigation_tab.js

settings-navigation-title = تبويبات التنقل
settings-navigation-hint = اسحب العناصر لإعادة ترتيبها. بدّل الظهور بالمفتاح.
settings-navigation-section-layout = التخطيط
settings-navigation-overflow-label = علامات التبويب التي لا تتسع
settings-navigation-overflow-hint = مرّر صف علامات التبويب أفقيًا، أو اجمع العلامات التي لا تتسع في قائمة «المزيد» في نهايته.
settings-navigation-overflow-scroll = تمرير
settings-navigation-overflow-menu = قائمة «المزيد»
settings-navigation-drag-handle =
    .title = اسحب لإعادة الترتيب
settings-navigation-defaults-failed = تعذّر تحميل التنقل الافتراضي
settings-navigation-reset = تمت إعادة التنقل إلى الافتراضي

## data_tab.js

settings-data-storage-title = تخزين قواعد البيانات
settings-data-storage-description = نظرة عامة على جميع قواعد البيانات التي تخزّن بيانات تداولك ومراكزك ومعلوماتك التاريخية.
settings-data-stats-loading = جارٍ تحميل إحصاءات قواعد البيانات...
settings-data-stats-load-failed = فشل تحميل إحصاءات قواعد البيانات
settings-data-total-storage = إجمالي تخزين قواعد البيانات
settings-data-db-tokens = الرموز
settings-data-db-transactions = المعاملات
settings-data-db-positions = المراكز
settings-data-db-events = الأحداث
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = المحفظة
settings-data-db-pools = المجمعات
settings-data-db-strategies = الاستراتيجيات
settings-data-db-actions = الإجراءات
settings-data-directory-label = مجلد البيانات
settings-data-directory-copied = مجلد البيانات
settings-data-config-path-copied = مسار الإعدادات
settings-data-path-unavailable = غير متاح
settings-data-path-copy-title = انقر لنسخ المسار
settings-data-path-copy-failed = فشل نسخ المسار

settings-data-config-title = إدارة الإعدادات
settings-data-config-description = صدّر إعدادات البوت واستورِدها وأدرها. احتفظ بنسخ احتياطية قبل إجراء تغييرات كبيرة.
settings-data-config-export = تصدير الإعدادات
settings-data-config-import = استيراد الإعدادات
settings-data-config-reset = إعادة التعيين إلى الافتراضي
settings-data-config-location-label = موقع الإعدادات
settings-data-config-fetch-failed = فشل جلب الإعدادات
settings-data-config-exported = تم تصدير الإعدادات
settings-data-config-export-failed = فشل تصدير الإعدادات: { $message }
settings-data-config-import-title = استيراد الإعدادات
settings-data-config-import-message = هل تريد استيراد هذه الإعدادات؟ سيتم استبدال الإعدادات الحالية. ستُحفظ بيانات اعتماد المحفظة.
settings-data-config-imported = تم استيراد الإعدادات بنجاح. قد تتطلب بعض التغييرات إعادة التشغيل.
settings-data-config-import-failed = فشل استيراد الإعدادات: { $message }
settings-data-config-reset-title = إعادة تعيين الإعدادات
settings-data-config-reset-message = هل تريد إعادة جميع الإعدادات إلى الافتراضي؟ ستُحفظ بيانات اعتماد المحفظة، وتُعاد بقية الإعدادات.
settings-data-config-reset-done = تمت إعادة الإعدادات إلى الافتراضي
settings-data-config-reset-failed = فشلت إعادة تعيين الإعدادات: { $message }
settings-data-unknown-error = خطأ غير معروف

settings-data-cleanup-title = تنظيف البيانات
settings-data-cleanup-description = حرّر مساحة القرص بإزالة البيانات القديمة أو غير المستخدمة. لا يمكن التراجع عن هذه الإجراءات.
settings-data-ohlcv-cleanup-label = تنظيف بيانات OHLCV
settings-data-ohlcv-cleanup-hint = إزالة بيانات الشموع للرموز التي لم تنشط خلال المدة المحددة.
settings-data-cleanup-hours-unit = ساعة
settings-data-cleanup-ohlcv = تنظيف OHLCV
settings-data-cleanup-running = جارٍ التنظيف...
settings-data-cleanup-hours-invalid = قيمة الساعات غير صالحة
settings-data-cleanup-confirm-title = حذف بيانات OHLCV
settings-data-cleanup-confirm-message =
    { $hours ->
        [zero] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من { $hours } ساعة؟
        [one] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من ساعة واحدة ({ $hours })؟
        [two] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من { $hours } ساعتين؟
        [few] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من { $hours } ساعات؟
        [many] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من { $hours } ساعة؟
       *[other] هل تريد حذف بيانات OHLCV للرموز غير النشطة لأكثر من { $hours } ساعة؟
    }
settings-data-cleanup-done =
    { $count ->
        [zero] تم تنظيف { $count } رمز غير نشط
        [one] تم تنظيف رمز غير نشط ({ $count })
        [two] تم تنظيف رمزين غير نشطين ({ $count })
        [few] تم تنظيف { $count } رموز غير نشطة
        [many] تم تنظيف { $count } رمزًا غير نشط
       *[other] تم تنظيف { $count } رمز غير نشط
    }
settings-data-cleanup-failed = فشل التنظيف
settings-data-cleanup-failed-detail = فشل التنظيف: { $message }

settings-data-cache-clear-label = مسح كل ذاكرة OHLCV المؤقتة
settings-data-cache-clear-hint = امسح كل بيانات الشموع المخزنة مؤقتًا وأعد جلب كل رمز مراقَب من البداية. استخدمه إذا بدت المخططات خاطئة أو بعد تحديث منطق البيانات.
settings-data-cache-clear = مسح ذاكرة OHLCV المؤقتة
settings-data-cache-clearing = جارٍ المسح...
settings-data-cache-confirm-title = مسح كل ذاكرة OHLCV المؤقتة
settings-data-cache-confirm-message = هل تريد مسح كل بيانات الشموع المخزنة مؤقتًا لكل رمز؟ ستعيد الرموز المراقَبة جلب سجلها من البداية. لا يمكن التراجع عن ذلك.
settings-data-candles-count =
    { $count ->
        [zero] { $count } شمعة
        [one] { $count } شمعة
        [two] { $count } شمعتان
        [few] { $count } شموع
        [many] { $count } شمعة
       *[other] { $count } شمعة
    }
settings-data-tokens-count =
    { $count ->
        [zero] { $count } رمز
        [one] { $count } رمز
        [two] { $count } رمزان
        [few] { $count } رموز
        [many] { $count } رمزًا
       *[other] { $count } رمز
    }
settings-data-cache-cleared = تم مسح { $candles } عبر { $tokens }؛ جارٍ إعادة الجلب
settings-data-cache-clear-failed = فشل مسح ذاكرة OHLCV المؤقتة
settings-data-cache-clear-failed-detail = فشل مسح ذاكرة OHLCV المؤقتة: { $message }

settings-data-ui-cache-label = ذاكرة حالة الواجهة المؤقتة
settings-data-ui-cache-hint = مسح تفضيلات الجداول وحالات المرشحات وإعدادات العرض المحفوظة.
settings-data-ui-cache-clear = مسح ذاكرة الواجهة المؤقتة
settings-data-ui-cache-confirm-title = مسح حالة الواجهة
settings-data-ui-cache-confirm-message = هل تريد مسح جميع تفضيلات الواجهة المحفوظة؟ سيؤدي ذلك إلى إعادة تعيين أعمدة الجداول والمرشحات وإعدادات العرض.
settings-data-ui-cache-cleared =
    { $count ->
        [zero] تم مسح { $count } إعداد واجهة مخزّن مؤقتًا
        [one] تم مسح إعداد واجهة واحد مخزّن مؤقتًا ({ $count })
        [two] تم مسح إعدادَي واجهة مخزّنين مؤقتًا ({ $count })
        [few] تم مسح { $count } إعدادات واجهة مخزّنة مؤقتًا
        [many] تم مسح { $count } إعداد واجهة مخزّنًا مؤقتًا
       *[other] تم مسح { $count } إعداد واجهة مخزّن مؤقتًا
    }

settings-data-folder-label = فتح مجلد البيانات
settings-data-folder-hint = افتح المجلد الذي يحتوي على جميع بيانات { -brand } في مدير الملفات.
settings-data-folder-open = فتح المجلد
settings-data-folder-open-failed = تعذّر فتح مجلد البيانات
