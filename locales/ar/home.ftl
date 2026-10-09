## Portfolio overview

home-portfolio-title = قيمة المحفظة الاستثمارية
home-portfolio-today = اليوم
home-stat-available = { -sol } المتاح
home-stat-holdings = حيازات الرموز
home-stat-open-pnl = أرباح وخسائر المراكز المفتوحة
home-stat-realized-today = المحقق اليوم

home-holdings-token-count =
    { $count ->
        [zero] { $count } رمز
        [one] { $count } رمز
        [two] { $count } رمزان
        [few] { $count } رموز
        [many] { $count } رمزًا
       *[other] { $count } رمز
    }
home-holdings-with-unpriced = { $tokens } · بدون سعر: { $count }
home-holdings-unpriced-note =
    { $count ->
        [zero] لا توجد رموز محتفَظ بها بلا سعر ({ $count })
        [one] رمز واحد محتفَظ به ({ $count }) لا يتوفر له سعر ويُحتسب بقيمة 0 في الإجمالي
        [two] رمزان محتفَظ بهما ({ $count }) لا يتوفر لهما سعر ويُحتسبان بقيمة 0 في الإجمالي
        [few] { $count } رموز محتفَظ بها لا يتوفر لها سعر وتُحتسب بقيمة 0 في الإجمالي
        [many] { $count } رمزًا محتفَظًا به لا يتوفر لها سعر وتُحتسب بقيمة 0 في الإجمالي
       *[other] { $count } رمز محتفَظ به لا يتوفر لها سعر وتُحتسب بقيمة 0 في الإجمالي
    }

## Wallet address and QR code

home-wallet-copy =
    .title = نسخ عنوان المحفظة
    .aria-label = نسخ عنوان المحفظة
home-wallet-qr-open =
    .title = عرض رمز QR للمحفظة
    .aria-label = عرض رمز QR للمحفظة
home-wallet-qr-popover =
    .aria-label = رمز QR للمحفظة
home-wallet-qr-receive = استلام
home-wallet-qr-assets = { -sol } ورموز SPL
home-wallet-qr-close =
    .title = إغلاق
    .aria-label = إغلاق رمز QR للمحفظة
home-wallet-qr-preparing = جارٍ تجهيز رمز QR
home-wallet-qr-unavailable = رمز QR غير متاح
home-wallet-qr-image =
    .alt = رمز QR لعنوان المحفظة الرئيسية

## Performance calendar

home-calendar-title = تقويم الأداء
home-calendar-previous =
    .title = الشهر السابق
    .aria-label = الشهر السابق
home-calendar-next =
    .title = الشهر التالي
    .aria-label = الشهر التالي
home-calendar-month-pnl = أرباح وخسائر الشهر
home-calendar-trades = الصفقات
home-calendar-pop-net-pnl = صافي الأرباح والخسائر
home-calendar-pop-win-rate = نسبة الربح
home-calendar-pop-win-rate-value = { $rate } · { $wins } رابحة / { $losses } خاسرة
home-calendar-pop-gross-profit = إجمالي الربح
home-calendar-pop-gross-loss = إجمالي الخسارة
home-calendar-pop-end-balance = الرصيد النهائي

## Position exposure and market pipeline

home-operations =
    .aria-label = حالة المحفظة والسوق
home-exposure-title = انكشاف المراكز
home-exposure-open = مفتوحة
home-exposure-invested = المستثمر
home-exposure-avg-size = متوسط الحجم
home-exposure-avg-hold = متوسط مدة الاحتفاظ
home-exposure-best = الأفضل
home-exposure-worst = الأسوأ
home-pipeline-title = مسار السوق
home-pipeline-tracked = المتتبَّعة
home-pipeline-priced = المسعّرة
home-pipeline-passed = اجتازت المرشحات
home-pipeline-not-passed = لم تجتز (كل المتتبَّعة)
home-pipeline-blacklisted = في القائمة السوداء
home-pipeline-ohlcv = OHLCV
