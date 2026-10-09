# Assistant page shell and chat widget. Model output (chat replies, tool
# results, decision reasoning) is data and is never part of this catalog.

# Provider names shown by the Assistant page. Ids are the `Provider::as_str`
# values in src/apis/llm/mod.rs.
assistant-provider-openai = { -openai }
assistant-provider-anthropic = { -anthropic }
assistant-provider-groq = { -groq }
assistant-provider-deepseek = { -deepseek }
assistant-provider-gemini = { -google-gemini }
assistant-provider-ollama = { -ollama }
assistant-provider-together = { -together-ai }
assistant-provider-openrouter = { -openrouter }
assistant-provider-mistral = { -mistral-ai }
assistant-provider-select = اختر المزوّد...

# Verdicts recorded for a model evaluation. `allow` is the alias the overview
# feed may carry; the rest are the ids written by src/llm_analysis/engine.rs.
assistant-decision-allow = سماح
assistant-decision-pass = ناجح
assistant-decision-reject = مرفوض
assistant-decision-buy = شراء
assistant-decision-sell = بيع
assistant-decision-hold = احتفاظ

# Risk levels of `RiskLevel` in src/llm_analysis/types.rs.
assistant-risk-low = منخفضة
assistant-risk-medium = متوسطة
assistant-risk-high = عالية
assistant-risk-critical = حرجة

# assistant.html: page shell.
assistant-overview-title = نظرة عامة على المساعد
assistant-overview-description = حالة نماذج LLM والتحليل المباشرة، ومقاييس الأداء، وأحدث تقييمات النماذج.
assistant-features-title = ميزات النموذج
assistant-metric-total-evaluations = إجمالي التقييمات
assistant-metric-cache-hit-rate = نسبة إصابة الذاكرة المؤقتة
assistant-metric-avg-latency = متوسط زمن الاستجابة
assistant-metric-active-providers = المزوّدون النشطون
assistant-decisions-title = القرارات الأخيرة
assistant-decisions-subtitle = أحدث قرارات تحليل النموذج

assistant-providers-title = مزوّدو LLM
assistant-providers-description = إعداد مزوّدي LLM المشتركين المستخدمين في التحليل والمساعد

assistant-master-title = الإعدادات الرئيسية
assistant-master-description = المفتاح الرئيسي المشترك لـ LLM واختيار المزوّد
assistant-setting-enabled-description = تفعيل التحليل الذكي للرموز باستخدام مزوّدي LLM
assistant-setting-default-provider-label = المزوّد الافتراضي
assistant-setting-default-provider-description = المزوّد الوحيد المستخدم في التحليل والمساعد والمهام المجدولة

assistant-filtering-title = الترشيح
assistant-filtering-description = ترشيح الرموز بواسطة المساعد قبل قرارات الدخول
assistant-setting-filtering-enabled-label = ترشيح الرموز بواسطة المساعد
assistant-setting-filtering-enabled-description = تقييم الرموز بواسطة المساعد قبل السماح بصفقات الدخول
assistant-setting-min-confidence-label = الحد الأدنى للثقة
assistant-setting-min-confidence-description = يجب أن يبلغ تقييم الرمز هذا المستوى من الثقة على الأقل لينجح (0-100)
assistant-setting-fallback-pass-label = النجاح عند الفشل
assistant-setting-fallback-pass-description = السماح بالدخول في الرمز إذا فشل تقييم المساعد أو انتهت مهلته

assistant-trading-title = التداول
assistant-trading-description = تحليل المساعد أثناء عمليات التداول
assistant-setting-entry-analysis-label = تحليل الدخول
assistant-setting-entry-analysis-description = تشغيل تحليل المساعد قبل فتح المراكز
assistant-setting-exit-analysis-label = تحليل الخروج
assistant-setting-exit-analysis-description = تشغيل تحليل المساعد قبل إغلاق المراكز
assistant-setting-trailing-stop-label = تحليل الوقف المتحرك
assistant-setting-trailing-stop-description = تشغيل تحليل المساعد عند تفعيل الوقف المتحرك

assistant-blacklist-title = القائمة السوداء التلقائية
assistant-blacklist-description = حظر الرموز تلقائيًا بناءً على تقييمات المساعد
assistant-setting-auto-blacklist-label = القائمة السوداء التلقائية
assistant-setting-auto-blacklist-description = إضافة الرموز ذات التقييمات المنخفضة جدًا من المساعد إلى القائمة السوداء تلقائيًا
assistant-setting-blacklist-threshold-label = حد القائمة السوداء
assistant-setting-blacklist-threshold-description = تُضاف الرموز التي تقل تقييماتها عن هذا المستوى إلى القائمة السوداء تلقائيًا (0-100)

assistant-performance-title = الأداء
assistant-performance-description = إعدادات التخزين المؤقت والتزامن
assistant-setting-cache-ttl-label = مدة صلاحية الذاكرة المؤقتة
assistant-setting-cache-ttl-description = مدة الاحتفاظ بنتائج المساعد في الذاكرة المؤقتة (60-3600 ثانية)
assistant-setting-max-evaluations-label = الحد الأقصى للتقييمات
assistant-setting-max-evaluations-description = الحد الأقصى لتقييمات المساعد المتزامنة المسموح بها (1-20)

assistant-cache-title = إدارة الذاكرة المؤقتة
assistant-cache-description = عرض تقييمات المساعد المخزنة مؤقتًا ومسحها
assistant-cache-size-label = حجم الذاكرة المؤقتة:
assistant-cache-memory-label = استخدام الذاكرة:
assistant-cache-clear = مسح الذاكرة المؤقتة

assistant-testing-title = ساحة اختبار المساعد
assistant-testing-subtitle = اختبر تحليل المساعد على أي رمز
assistant-testing-mint-label = عنوان الإصدار
assistant-testing-mint-input =
    .placeholder = أدخل عنوان إصدار رمز Solana...
assistant-testing-priority-label = الأولوية
assistant-testing-priority-low = منخفضة
assistant-testing-priority-medium = متوسطة
assistant-testing-priority-high = عالية
assistant-testing-evaluate = تقييم
assistant-testing-results = النتائج

assistant-instructions-title = تعليمات مخصصة
assistant-instructions-description = أضف موجّهات نصية مخصصة تُدرج في تقييمات المساعد
assistant-instructions-new = تعليمات جديدة
assistant-instructions-category-all = جميع الفئات
assistant-instructions-category-filtering = الترشيح
assistant-instructions-category-trading = التداول
assistant-instructions-category-analysis = التحليل
assistant-instructions-category-general = عامة
assistant-instructions-status-all = جميع الحالات
assistant-instructions-status-active = النشطة فقط
assistant-instructions-status-inactive = غير النشطة فقط
assistant-instructions-search =
    .placeholder = البحث في التعليمات...
assistant-instructions-empty = لا توجد تعليمات مخصصة بعد
assistant-instructions-empty-add = أضف أولى تعليماتك
assistant-templates-title = قوالب التعليمات
assistant-templates-description = قوالب تعليمات جاهزة يمكنك إضافتها

assistant-automation-title = الأتمتة
assistant-automation-description = جدولة مهام المساعد للتشغيل تلقائيًا على فترات أو في أوقات محددة
assistant-automation-new =
    .aria-label = إنشاء مهمة أتمتة جديدة
assistant-automation-new-label = مهمة جديدة
assistant-automation-stat-total = إجمالي المهام
assistant-automation-stat-active = نشطة
assistant-automation-stat-runs = إجمالي التشغيلات
assistant-automation-stat-success-rate = نسبة النجاح
assistant-automation-empty = لا توجد مهام مجدولة بعد
assistant-automation-empty-subtitle = أنشئ أولى مهام المساعد الآلية للبدء
assistant-automation-empty-add = أنشئ أولى مهامك
    .aria-label = إنشاء أول مهمة أتمتة
assistant-automation-runs-title = التشغيلات الأخيرة

assistant-history-title = سجل القرارات
assistant-history-subtitle = أحدث تقييمات المساعد

# assistant.js: page tabs, overview, settings and history.
assistant-tab-chat = المحادثة
assistant-tab-overview = نظرة عامة
assistant-tab-providers = المزوّدون
assistant-tab-instructions = التعليمات
assistant-tab-automation = الأتمتة
assistant-tab-history = السجل
assistant-tab-testing = الاختبار
assistant-tab-settings = الإعدادات
assistant-toggle-on = مفعّل
assistant-toggle-off = معطّل
assistant-status-active = المساعد مفعّل
assistant-status-disabled = المساعد معطّل
assistant-status-load-failed = تعذّر تحميل حالة ميزات النموذج
assistant-toggle-enabled-title = تم تفعيل المساعد
assistant-toggle-enabled-message = ميزات النموذج نشطة الآن
assistant-toggle-disabled-title = تم تعطيل المساعد
assistant-toggle-disabled-message = ميزات النموذج معطّلة
assistant-toggle-failed = تعذّر تحديث حالة ميزات النموذج
assistant-decisions-empty = لا توجد قرارات حديثة
assistant-decision-latency =
    .title = زمن الاستجابة
assistant-decision-confidence =
    .title = الثقة
assistant-config-load-failed = تعذّر تحميل إعدادات التحليل
assistant-cache-clear-message = هل أنت متأكد من رغبتك في مسح الذاكرة المؤقتة للتحليل؟ سيؤدي هذا إلى إزالة جميع قرارات النموذج المخزنة مؤقتًا.
assistant-cache-cleared-title = تم مسح الذاكرة المؤقتة
assistant-cache-cleared-message = الذاكرة المؤقتة للتحليل فارغة
assistant-cache-clear-failed = تعذّر مسح الذاكرة المؤقتة
assistant-config-saved-title = تم الحفظ
assistant-config-saved-message = تم حفظ الإعدادات بنجاح
assistant-config-save-failed = تعذّر حفظ الإعدادات
assistant-history-load-failed = تعذّر تحميل السجل
assistant-history-empty = لا توجد طلبات تحليل LLM بعد
assistant-history-column-token = الرمز
assistant-history-column-decision = القرار
assistant-history-column-confidence = الثقة
assistant-history-column-risk = المخاطرة
assistant-history-column-reasoning = التعليل
assistant-history-column-model = النموذج
assistant-history-column-latency = زمن الاستجابة
assistant-history-column-when = الوقت
assistant-history-previous = السابق
assistant-history-page = الصفحة { $page } من { $total }
assistant-history-cached = مخزّن مؤقتًا

# chat_widget.js: sessions sidebar and header.
assistant-chat-sessions-title = الجلسات
assistant-chat-search =
    .placeholder = البحث في المحادثات...
    .aria-label = البحث في جلسات المحادثة
assistant-chat-history-close =
    .aria-label = إغلاق سجل المحادثات
assistant-chat-history-open =
    .title = سجل المحادثات
    .aria-label = فتح سجل المحادثات
assistant-chat-header-new =
    .title = محادثة جديدة
    .aria-label = بدء محادثة جديدة
assistant-chat-delete =
    .title = حذف
    .aria-label = حذف الجلسة
assistant-chat-close =
    .title = إغلاق
    .aria-label = إغلاق المساعد
assistant-chat-title-new = محادثة جديدة
assistant-chat-sessions-empty = لا توجد جلسات محادثة بعد
assistant-chat-sessions-empty-search = لا توجد محادثات مطابقة
assistant-chat-group-today = اليوم
assistant-chat-group-yesterday = أمس
assistant-chat-group-week = آخر 7 أيام
assistant-chat-group-older = أقدم

# chat_widget.js: empty state and quick prompts. The prompt texts are sent as the
# user's message.
assistant-chat-empty-kicker = المساعد
assistant-chat-empty-title = كيف يمكنني مساعدتك اليوم؟
assistant-chat-empty-subtitle = راجع محفظتك الاستثمارية، أو افحص رمزًا، أو افهم نشاط التداول الأخير.
assistant-chat-prompt-positions-label = مراجعة المراكز المفتوحة
assistant-chat-prompt-positions-text = ما هو رصيد محفظتي الحالي ومراكزي المفتوحة؟
assistant-chat-prompt-token-label = تحليل رمز
assistant-chat-prompt-token-text = حلّل أمان ومخاطر هذا الرمز:
assistant-chat-prompt-activity-label = شرح النشاط الأخير
assistant-chat-prompt-activity-text = اشرح نشاط التداول الأخير الخاص بي وأي نتائج مهمة

# chat_widget.js: input area.
assistant-chat-input =
    .placeholder = راسل المساعد...
    .aria-label = حقل الرسالة
assistant-chat-hint-key-enter = Enter
assistant-chat-hint-key-shift = Shift
assistant-chat-hint-send = للإرسال
assistant-chat-hint-newline = لسطر جديد
assistant-chat-send =
    .title = إرسال الرسالة
    .aria-label = إرسال الرسالة
assistant-chat-send-empty =
    .aria-label = اكتب رسالة لإرسالها
assistant-chat-stop =
    .title = إيقاف الرد (Esc)
    .aria-label = إيقاف الرد
assistant-chat-cancelled-title = تم الإلغاء
assistant-chat-cancelled-message = تم إلغاء الطلب
assistant-chat-message-too-long-title = الرسالة طويلة جدًا
assistant-chat-message-too-long-message = يرجى اختصار رسالتك إلى أقل من { $limit } حرف
assistant-chat-start-failed = تعذّر بدء جلسة المحادثة
assistant-chat-send-failed = تعذّر على المساعد إكمال الرد. رسالتك جاهزة لإعادة المحاولة.

# chat_widget.js: messages and tool calls.
assistant-chat-role-user = أنت
assistant-chat-role-assistant = المساعد
assistant-chat-message-actions =
    .aria-label = إجراءات الرسالة
assistant-chat-message-copy =
    .title = نسخ
    .aria-label = نسخ الرسالة
assistant-chat-message-regenerate =
    .title = إعادة التوليد
    .aria-label = إعادة توليد الرد
assistant-chat-copied-message = الرسالة
assistant-chat-copy-failed = تعذّر نسخ الرسالة
assistant-chat-regenerate-none = لا توجد رسالة لإعادة توليدها
assistant-chat-regenerate-reload = أعد تحميل المحادثة قبل إعادة توليد هذا الرد
assistant-chat-regenerate-done-title = تمت إعادة التوليد
assistant-chat-regenerate-done-message = تمت إعادة توليد الرد بنجاح
assistant-chat-regenerate-failed = تعذّرت إعادة التوليد
assistant-chat-regenerate-failed-toast = تعذّرت إعادة توليد الرد

# Status of a tool call; ids are the `ToolCallStatus` variants in
# src/assistant/chat/types.rs, lowercased and hyphenated.
assistant-chat-tool-status-executed = تم التنفيذ
assistant-chat-tool-status-failed = فشل
assistant-chat-tool-status-denied = مرفوض
assistant-chat-tool-status-pending-confirmation = بانتظار التأكيد
assistant-chat-tool-status-pending = قيد الانتظار
assistant-chat-tool-section-input = المدخلات:
assistant-chat-tool-section-output = المخرجات:
assistant-chat-tool-section-error = الخطأ:

# chat_widget.js: progress while a response is generated.
assistant-chat-typing =
    .aria-label = المساعد يفكّر
assistant-chat-progress-preparing = جارٍ تحضير الرد
assistant-chat-progress-planning = جارٍ التخطيط للرد
assistant-chat-progress-reviewing = جارٍ مراجعة نتائج الأدوات
assistant-chat-progress-using-tools = جارٍ استخدام الأدوات
assistant-chat-progress-running = قيد التشغيل
assistant-chat-progress-failed = فشل
assistant-chat-progress-complete = اكتمل
assistant-chat-error-unknown = خطأ غير معروف
assistant-chat-stream-http = خطأ API: { $status }
assistant-chat-stream-unavailable = بث تقدّم المساعد غير متاح
assistant-chat-stream-failed = فشل طلب المساعد
assistant-chat-stream-incomplete = انتهى رد المساعد قبل اكتماله

# chat_widget.js: tool approval and sessions.
assistant-chat-tool-review = مراجعة المدخلات
assistant-chat-tool-deny = رفض
assistant-chat-tool-allow = سماح
assistant-chat-tool-unknown = أداة غير معروفة
assistant-chat-tool-default-description = تتطلب هذه الأداة موافقتك للتنفيذ.
assistant-chat-tool-executed = تم تنفيذ الأداة
assistant-chat-tool-cancelled = تم إلغاء تنفيذ الأداة
assistant-chat-tool-confirm-failed = تعذّر تأكيد تنفيذ الأداة
assistant-chat-sessions-load-failed = تعذّر تحميل جلسات المحادثة
assistant-chat-delete-title = حذف جلسة المحادثة
assistant-chat-delete-message = هل أنت متأكد من رغبتك في حذف جلسة المحادثة هذه؟ لا يمكن التراجع عن هذا الإجراء.
assistant-chat-delete-done = تم حذف جلسة المحادثة
assistant-chat-delete-failed = تعذّر حذف جلسة المحادثة

# Agent tool labels. Ids are the tool names registered by
# `create_tool_registry` in src/agent_control/tools/mod.rs, with hyphens.
assistant-tool-analyze-token = تحليل رمز
assistant-tool-get-market-data = جلب بيانات السوق
assistant-tool-check-security = فحص الأمان
assistant-tool-get-positions = جلب المراكز
assistant-tool-get-position = جلب مركز
assistant-tool-get-balance = جلب الرصيد
assistant-tool-get-pnl = جلب الأرباح والخسائر
assistant-tool-buy-token = شراء رمز
assistant-tool-add-to-position = إضافة إلى مركز
assistant-tool-sell-token = بيع رمز
assistant-tool-close-position = إغلاق مركز
assistant-tool-get-config = جلب الإعدادات
assistant-tool-describe-config = وصف الإعدادات
assistant-tool-update-config = تحديث الإعدادات
assistant-tool-get-status = جلب الحالة
assistant-tool-get-events = جلب الأحداث
assistant-tool-force-stop = إيقاف قسري
assistant-tool-clear-force-stop = إلغاء الإيقاف القسري
assistant-tool-get-trader-status = جلب حالة المتداول الآلي
assistant-tool-get-trader-stats = جلب إحصاءات المتداول الآلي
assistant-tool-set-trader-enabled = تفعيل المتداول الآلي
assistant-tool-set-trader-monitor = ضبط مراقبة المتداول الآلي
assistant-tool-manage-loss-limit = إدارة حد الخسارة
assistant-tool-list-trader-templates = عرض قوالب المتداول الآلي
assistant-tool-apply-trader-template = تطبيق قالب المتداول الآلي
assistant-tool-get-copy-trading-overview = جلب نظرة عامة على نسخ التداول
assistant-tool-get-copy-task = جلب مهمة نسخ
assistant-tool-get-copy-activity = جلب نشاط النسخ
assistant-tool-create-copy-task = إنشاء مهمة نسخ
assistant-tool-update-copy-task = تحديث مهمة نسخ
assistant-tool-delete-copy-task = حذف مهمة نسخ
assistant-tool-set-copy-task-mode = ضبط وضع مهمة النسخ
assistant-tool-get-copy-insights = جلب رؤى النسخ
assistant-tool-get-copy-wallet-profile = جلب ملف محفظة النسخ
assistant-tool-clone-copy-task = استنساخ مهمة نسخ
assistant-tool-reset-copy-paper-book = إعادة ضبط دفتر النسخ التجريبي
assistant-tool-close-copy-paper-holding = إغلاق حيازة النسخ التجريبي

# Built-in instruction templates (src/llm_analysis/database.rs
# `get_builtin_templates`). The template body is model input and stays in Rust.
assistant-template-liquidity-guard-name = حارس السيولة
assistant-template-liquidity-guard-description = يرفض الرموز ذات السيولة الضعيفة، مما يرفع مخاطر الانزلاق السعري ويصعّب الخروج.
assistant-template-holder-distribution-name = فحص توزيع الحاملين
assistant-template-holder-distribution-description = ينبّه إلى الرموز التي يسيطر كبار حامليها على حصة كبيرة من المعروض.
assistant-template-honeypot-detection-name = كشف فخ العسل
assistant-template-honeypot-detection-description = يرفض الرموز ذات صلاحية التجميد أو الإصدار النشطة أو قيود التحويل غير المعتادة.
assistant-template-momentum-filter-name = مرشح الزخم
assistant-template-momentum-filter-description = يفضّل الرموز ذات الزخم السعري الإيجابي المؤكد بحجم تداول متزايد.
assistant-template-new-token-caution-name = الحذر من الرموز الجديدة
assistant-template-new-token-caution-description = يتطلب ثقة أعلى للرموز التي يقل عمرها عن يوم.
assistant-template-whale-activity-name = مراقب نشاط الحيتان
assistant-template-whale-activity-description = يرصد تحركات كبار الحاملين ونشاط الناشر أو المحافظ المبكرة غير المعتاد.

# Template tags; ids are the `tags` of a built-in template.
assistant-template-tag-activity = النشاط
assistant-template-tag-age = العمر
assistant-template-tag-authority = الصلاحية
assistant-template-tag-caution = الحذر
assistant-template-tag-distribution = التوزيع
assistant-template-tag-holders = الحاملون
assistant-template-tag-honeypot = فخ العسل
assistant-template-tag-large-holders = كبار الحاملين
assistant-template-tag-liquidity = السيولة
assistant-template-tag-momentum = الزخم
assistant-template-tag-new-tokens = الرموز الجديدة
assistant-template-tag-price-action = حركة السعر
assistant-template-tag-risk-management = إدارة المخاطر
assistant-template-tag-rug-risk = مخاطر سحب السجادة
assistant-template-tag-safety = السلامة
assistant-template-tag-security = الأمان
assistant-template-tag-volume = حجم التداول
assistant-template-tag-whales = الحيتان

# Shared by the Assistant tab dialogs.
assistant-modal-close =
    .aria-label = إغلاق

# providers_tab.js: provider list.
assistant-providers-load-failed = تعذّر تحميل المزوّدين
assistant-providers-select-default =
    .title = تعيين كافتراضي
assistant-providers-use-default =
    .aria-label = استخدام { $name } كمزوّد افتراضي
assistant-providers-model-none = غير مهيأ
assistant-providers-status-ready = جاهز
assistant-providers-status-not-set-up = غير معدّ
assistant-providers-status-default = افتراضي
assistant-providers-test = اختبار
assistant-providers-configure = إعداد
assistant-providers-default-set-title = تم تعيين المزوّد الافتراضي
assistant-providers-default-set-message = أصبح { $name } المزوّد الافتراضي الآن
assistant-providers-default-set-failed = تعذّر تعيين المزوّد الافتراضي
assistant-providers-testing-title = اختبار المزوّد
assistant-providers-testing-message = جارٍ اختبار { $name }...
assistant-providers-test-http = HTTP { $status }
assistant-providers-test-success-title = نجح الاتصال
assistant-providers-test-success-message = { $name } يعمل بشكل صحيح
assistant-providers-test-failed = فشل الاختبار

# providers_tab.js: configuration dialog.
assistant-providers-config-title = إعدادات { $name }
assistant-providers-api-key = مفتاح API
assistant-providers-key-saved = المفتاح محفوظ
assistant-providers-key-missing = لم يُعيَّن مفتاح
assistant-providers-api-key-update =
    .placeholder = أدخل مفتاحًا جديدًا للتحديث...
assistant-providers-api-key-enter =
    .placeholder = أدخل مفتاح API...
assistant-providers-key-toggle =
    .title = إظهار/إخفاء
assistant-providers-key-help-saved = اتركه فارغًا للاحتفاظ بالمفتاح الحالي، أو أدخل مفتاحًا جديدًا للتحديث
assistant-providers-key-help-new = يُخزَّن مفتاح API الخاص بك بأمان ولا تتم مشاركته أبدًا
assistant-providers-model = النموذج
assistant-providers-model-input =
    .placeholder = مثال: gpt-4, claude-3-opus...
assistant-providers-model-help = النموذج المستخدم في طلبات تحليل المساعد
assistant-providers-enable = تفعيل هذا المزوّد
assistant-providers-enable-help = عند التفعيل، سيكون هذا المزوّد متاحًا لتحليل المساعد
assistant-providers-connection-test = اختبار الاتصال
assistant-providers-test-connection = اختبار الاتصال
assistant-providers-testing = جارٍ الاختبار...
assistant-providers-save = حفظ الإعدادات
assistant-providers-saving = جارٍ الحفظ...
assistant-providers-missing-key-title = مفتاح API مفقود
assistant-providers-missing-key-test = يرجى إدخال مفتاح API أولًا
assistant-providers-missing-key-enable = يرجى إدخال مفتاح API لتفعيل هذا المزوّد
assistant-providers-missing-model-title = النموذج مفقود
assistant-providers-missing-model-message = يرجى إدخال اسم النموذج
assistant-providers-test-save-failed = تعذّر حفظ الإعدادات للاختبار
assistant-providers-test-connected = نجح الاتصال!
assistant-providers-detail-model = النموذج:
assistant-providers-detail-none = غير متاح
assistant-providers-detail-latency = زمن الاستجابة:
assistant-providers-detail-tokens = الرموز النصية:
assistant-providers-saved-title = تم حفظ المزوّد
assistant-providers-saved-message = تم حفظ إعدادات { $name }
assistant-providers-save-failed = تعذّر حفظ إعدادات المزوّد

# instructions_tab.js: list, templates and dialogs.
assistant-instructions-load-failed = تعذّر تحميل التعليمات
assistant-instructions-priority = الأولوية: { $position }
assistant-instructions-actions =
    .aria-label = إجراءات التعليمات
assistant-instructions-hint-filtering = تعليمات لقرارات ترشيح الرموز - تساعد تحليل LLM على تحديد الرموز التي يجب تخطيها
assistant-instructions-hint-trading = تعليمات لتحليل الدخول/الخروج - توجّه قرارات التداول المقيّمة بالنموذج
assistant-instructions-hint-analysis = إرشادات عامة لتحليل السوق للقرارات المقيّمة بالنموذج
assistant-instructions-hint-general = تعليمات أخرى للسلوك المعتمد على النموذج
assistant-instructions-char-count =
    { $count ->
        [zero] { $amount } حرف
        [one] { $amount } حرف
        [two] { $amount } حرفان
        [few] { $amount } أحرف
        [many] { $amount } حرفًا
       *[other] { $amount } حرف
    }
assistant-instructions-reordered-title = تمت إعادة الترتيب
assistant-instructions-reordered-message = تمت إعادة ترتيب التعليمات بنجاح
assistant-instructions-reorder-failed = تعذّرت إعادة ترتيب التعليمات
assistant-templates-empty = لا تتوفر قوالب
assistant-templates-preview-title = معاينة القالب: { $name }
assistant-templates-preview-content = المحتوى:
assistant-templates-customize-add = تخصيص وإضافة
assistant-templates-customize-title = تخصيص القالب
assistant-instructions-field-name = الاسم
assistant-instructions-field-category = الفئة
assistant-instructions-field-content = المحتوى
assistant-instructions-name-input =
    .placeholder = مثال: حارس السيولة
assistant-instructions-content-input =
    .placeholder = أدخل تعليماتك...
assistant-instructions-create = إنشاء
assistant-instructions-create-title = إنشاء تعليمات
assistant-instructions-missing-title = حقول مفقودة
assistant-instructions-missing-message = الاسم والمحتوى مطلوبان
assistant-instructions-created-title = تم الإنشاء
assistant-instructions-created-message = تم إنشاء التعليمات بنجاح
assistant-instructions-created-from-template = تم إنشاء التعليمات من القالب: { $name }
assistant-instructions-create-failed = تعذّر إنشاء التعليمات
assistant-instructions-create-from-template-failed = تعذّر إنشاء التعليمات من القالب
assistant-instructions-toggle-failed = تعذّر تبديل حالة التعليمات
assistant-instructions-edit-title = تعديل التعليمات
assistant-instructions-preview = معاينة
assistant-instructions-save-changes = حفظ التغييرات
assistant-instructions-untitled = بلا عنوان
assistant-instructions-load-item-failed = تعذّر تحميل بيانات التعليمات
assistant-instructions-updated-title = تم التحديث
assistant-instructions-updated-message = تم تحديث التعليمات بنجاح
assistant-instructions-update-failed = تعذّر تحديث التعليمات
assistant-instructions-delete-title = حذف التعليمات
assistant-instructions-delete-message = هل أنت متأكد من رغبتك في حذف هذه التعليمات؟
assistant-instructions-deleted-title = تم الحذف
assistant-instructions-deleted-message = تم حذف التعليمات بنجاح
assistant-instructions-delete-failed = تعذّر حذف التعليمات
assistant-instructions-copy-name = { $name } (نسخة)
assistant-instructions-duplicated-title = تم التكرار
assistant-instructions-duplicated-message = تم تكرار التعليمات بنجاح
assistant-instructions-duplicate-failed = تعذّر تكرار التعليمات

# automation_tab.js: task list, runs and dialogs. Schedule type, permission and
# run status ids are the `as_str` values of `ScheduleType`, `TaskToolPermissions`
# and `RunStatus` in src/assistant/scheduled/types.rs.
assistant-automation-schedule-type-interval = فاصل زمني
assistant-automation-schedule-type-daily = يومي
assistant-automation-schedule-type-weekly = أسبوعي
assistant-automation-permission-read-only = للقراءة فقط
assistant-automation-permission-full = وصول كامل
assistant-automation-permission-option-read-only = للقراءة فقط (آمن)
assistant-automation-permission-option-full = وصول كامل (يمكنه التداول)
assistant-automation-run-status-running = قيد التشغيل
assistant-automation-run-status-success = نجاح
assistant-automation-run-status-failed = فشل
assistant-automation-run-status-timeout = انتهت المهلة
assistant-automation-run-status-skipped = تم التخطي
assistant-automation-hint-interval = الفاصل بالثواني (مثال: 300 = كل 5 دقائق)
assistant-automation-hint-daily = الوقت بصيغة HH:MM بتوقيت UTC (مثال: 14:00)
assistant-automation-hint-weekly = الأيام والوقت: mon,wed,fri:09:00
assistant-automation-schedule-every = كل { $span }
assistant-automation-schedule-daily = يوميًا عند { $time } UTC
assistant-automation-schedule-weekly = { $days } عند { $time } UTC
assistant-automation-schedule-day-separator = { "، " }
assistant-automation-task-active = نشطة
assistant-automation-task-paused = متوقفة مؤقتًا
assistant-automation-never = أبدًا
assistant-automation-last-run = آخر تشغيل: { $when }
assistant-automation-next-run = التشغيل التالي: { $when }
assistant-automation-run-now =
    .title = تشغيل الآن
    .aria-label = تشغيل الآن
assistant-automation-actions =
    .aria-label = إجراءات الأتمتة
assistant-automation-runs-count =
    { $count ->
        [zero] { $amount } تشغيلة
        [one] { $amount } تشغيلة
        [two] { $amount } تشغيلتان
        [few] { $amount } تشغيلات
        [many] { $amount } تشغيلةً
       *[other] { $amount } تشغيلة
    }
assistant-automation-runs-empty = لا توجد تشغيلات بعد
assistant-automation-task-runs-empty = لا توجد تشغيلات لهذه المهمة بعد
assistant-automation-task-fallback = المهمة #{ $id }
assistant-automation-task-generic = مهمة
assistant-automation-create-title = إنشاء مهمة أتمتة
assistant-automation-edit-title = تعديل المهمة
assistant-automation-field-name = اسم المهمة
assistant-automation-name-input =
    .placeholder = مثال: مراقب المحفظة
assistant-automation-field-instruction = التعليمات
assistant-automation-instruction-input =
    .placeholder = ماذا يجب أن يفعل المساعد؟ مثال: افحص المراكز المفتوحة بحثًا عن علامات الانعكاس وأبلغ بالنتائج.
assistant-automation-field-schedule-type = نوع الجدولة
assistant-automation-field-schedule-value = قيمة الجدولة
assistant-automation-field-permissions = أذونات الأدوات
assistant-automation-field-timeout = المهلة (بالثواني)
assistant-automation-notify-telegram = الإشعار عبر { -telegram }
assistant-automation-notify-success = إشعار عند النجاح
assistant-automation-notify-failure = إشعار عند الفشل
assistant-automation-create-task = إنشاء مهمة
assistant-automation-save-changes = حفظ التغييرات
assistant-automation-validation-title = التحقق
assistant-automation-validation-required = يرجى ملء جميع الحقول المطلوبة
assistant-automation-validation-interval = يجب ألا يقل الفاصل عن 60 ثانية
assistant-automation-validation-daily = يجب أن تكون الجدولة اليومية بصيغة HH:MM
assistant-automation-validation-weekly = يجب أن تكون الجدولة الأسبوعية بصيغة: mon,wed,fri:09:00
assistant-automation-created = تم إنشاء المهمة
assistant-automation-create-failed = تعذّر إنشاء المهمة
assistant-automation-updated = تم تحديث المهمة
assistant-automation-update-failed = تعذّر تحديث المهمة
assistant-automation-toggle-failed = تعذّر تبديل حالة المهمة
assistant-automation-triggered = تم تشغيل المهمة
assistant-automation-trigger-failed = تعذّر تشغيل المهمة
assistant-automation-delete-title = حذف المهمة
assistant-automation-delete-message = هل أنت متأكد من رغبتك في حذف مهمة الأتمتة هذه؟ لا يمكن التراجع عن هذا الإجراء.
assistant-automation-deleted = تم حذف المهمة
assistant-automation-delete-failed = تعذّر حذف المهمة
assistant-automation-view-runs = عرض التشغيلات
assistant-automation-runs-load-failed = تعذّر تحميل التشغيلات
assistant-automation-runs-history-title = سجل التشغيل — { $task }
assistant-automation-run-load-failed = تعذّر تحميل تفاصيل التشغيل
assistant-automation-run-details-title = تفاصيل التشغيل
assistant-automation-run-task = المهمة
assistant-automation-run-status = الحالة
assistant-automation-run-started = وقت البدء
assistant-automation-run-duration = المدة
assistant-automation-run-provider = المزوّد
assistant-automation-run-tokens = الرموز النصية
assistant-automation-run-tools-title = استدعاءات الأدوات ({ $amount })
assistant-automation-run-response = رد المساعد
