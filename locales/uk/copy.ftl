copy-skip-not-buy-swap = Активність гаманця не була купівлею
copy-skip-task-disabled = Завдання призупинено
copy-skip-mode-transition-required = Режим виконання потрібно змінити окремо
copy-skip-live-confirmation-required = Реальне виконання потребує підтвердження
copy-skip-unsupported-sizing-mode = Режим розрахунку розміру поки не підтримується
copy-skip-self-copy = Цей гаманець — один із ваших
copy-skip-target-below-minimum = Угода гаманця нижча за мінімум
copy-skip-target-above-maximum = Угода гаманця вища за максимум
copy-skip-already-bought = Цей токен уже куплено (купівля один раз)
copy-skip-blacklisted = Токен заблоковано контролем ризиків
copy-skip-filter-required = Токен не пройшов фільтрацію
copy-skip-budget-exhausted = Бюджет завдання вичерпано
copy-skip-token-cap-reached = Досягнуто ліміту на токен
copy-skip-below-minimum-size = Розмір копії замалий
copy-skip-invalid-sizing = Розмір у завданні некоректний
copy-skip-invalid-slippage = Проковзування в завданні некоректне
copy-skip-invalid-exit-policy = Правила виходу в завданні некоректні
copy-skip-invalid-price = Немає придатної ринкової ціни
copy-skip-paper-quote-unavailable = Не вдалося отримати котирування свопу для віртуального виконання
copy-skip-paper-quote-unfillable = За цим котируванням не можна отримати придатне віртуальне виконання
copy-skip-paper-minimum-output-not-met = Друге котирування виявилося нижчим за мінімальний вихід першого
copy-skip-paper-queue-full = Черга віртуального виконання цього завдання заповнена
copy-skip-paper-execution-unavailable = Віртуальне виконання не завершено: сервіс зупинився, завдання або утримуваний токен змінилися, або запит дублював той, що виконується
copy-skip-not-sell-swap = Активність гаманця не була продажем
copy-skip-exit-mode-disabled = Продаж гаманця проігноровано: завдання продає за власними правилами
copy-skip-force-stopped = Торгівлю примусово зупинено
copy-skip-copy-position-not-found = Немає позиції, що належить цьому завданню
copy-skip-position-user-only = Позицією керуєте ви
copy-skip-position-management-mismatch = Позиція більше не слідує за копійованими продажами
copy-skip-latency-kill-switch = Автопауза: угоди виявлено надто пізно
copy-skip-claim-reconciled-abandoned = Перервану реальну відправку закрито без повторної спроби
copy-skip-stale-observation = Відтворено після простою, надто стара для копіювання
copy-skip-unknown-observation-time = Відтворена угода не має часу блока
copy-skip-entry-blocked = Вхід заблоковано
copy-skip-stable-quote-disabled = Сплачено USDC або USDT; угоди зі стейблкоїнами для цього завдання вимкнено
copy-skip-stable-quote-price-unavailable = Сплачено USDC або USDT, поточної ціни SOL для розрахунку розміру копії немає
copy-skip-swap-mixed-quote = Сплачено одночасно SOL і стейблкоїном, тому угода не має єдиного розміру
copy-skip-swap-multiple-tokens = В одній транзакції торгувалося більше ніж один токен
copy-skip-swap-unreadable = Не вдалося прочитати суми угоди

copy-entry-block-force-stopped = Торгівлю примусово зупинено
copy-entry-block-loss-limit = Ліміт збитків блокує нові входи
copy-entry-block-connectivity = Потрібні сервіси недоступні
copy-entry-block-storage-unavailable = Торгові дані недоступні або відновлюються
copy-entry-block-position-limit = Досягнуто ліміту відкритих позицій
copy-entry-block-already-open = Позицію вже відкрито
copy-entry-block-reentry-cooldown = Пауза перед повторним входом у токен
copy-entry-block-open-cooldown = Загальна пауза між входами
copy-entry-block-entry-reserved = Обробляється інший вхід
copy-entry-block-blacklisted = Токен заблоковано контролем ризиків
copy-entry-block-check-failed = Перевірку безпеки не вдалося завершити

copy-pause-user = Призупинено вами
copy-pause-latency-kill-switch = Автопауза: угоди надходили із середнім запізненням { $average } с (ліміт { $threshold } с)
copy-pause-watch-detached = Автопауза: за гаманцем більше не стежать
copy-pause-watch-budget-exceeded = Призупинено: цей гаманець досяг ліміту перевірок стеження (підписів за перевірку: { $limit }), не наздогнавши активність
copy-pause-helius-unavailable = Призупинено: перевірки гаманця через { -helius } не вдалися
copy-pause-watch-processing-failed = Призупинено: не вдалося обробити активність гаманця
copy-pause-storage-restored = Призупинено: дані копіювання відновлено після збою сховища
copy-pause-unspecified = Призупинено

copy-pause-short-user = вами
copy-pause-short-latency-kill-switch = надто повільно
copy-pause-short-watch-detached = стеження втрачено
copy-pause-short-watch-budget-exceeded = ліміт стеження
copy-pause-short-helius-unavailable = провайдер стеження
copy-pause-short-watch-processing-failed = обробка стеження
copy-pause-short-storage-restored = дані відновлено
copy-state-paused = Призупинено
copy-state-paused-reason = Призупинено · { $reason }

copy-readiness-history = Віртуальна історія
copy-readiness-history-met =
    { $count ->
        [one] { $count } закритий віртуальний раунд, потрібно { $needed }
        [few] { $count } закриті віртуальні раунди, потрібно { $needed }
        [many] { $count } закритих віртуальних раундів, потрібно { $needed }
       *[other] { $count } закритого віртуального раунду, потрібно { $needed }
    }
copy-readiness-history-short = Закритих віртуальних раундів: { $count } із { $needed }
copy-readiness-profit = Прибутковість у віртуальному режимі
copy-readiness-profit-detail = Реалізовано { $realized } { -sol } за раундів: { $count }, виграно: { $wins }
copy-readiness-latency = Угоди виявляються вчасно
copy-readiness-latency-detail = Надходження p95: { $p95 } с, ліміт: { $limit } с
copy-readiness-latency-none = Зразків надходження ще немає
copy-readiness-priced = Усі утримувані токени мають ціну
copy-readiness-priced-ok = Кожен відкритий віртуальний утримуваний токен має ціну пулу
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } відкритий утримуваний токен без ціни пулу
        [few] { $count } відкриті утримувані токени без ціни пулу
        [many] { $count } відкритих утримуваних токенів без ціни пулу
       *[other] { $count } відкритого утримуваного токена без ціни пулу
    }
copy-readiness-runtime = Реальне виконання доступне
copy-readiness-runtime-ok = Налаштування й запобіжники дозволяють реальні копії

copy-live-block-setup-incomplete = Спершу завершіть налаштування гаманця та RPC
copy-live-block-force-stop = Активна екстрена зупинка
copy-live-block-copy-trading-disabled = Обробку копіювання призупинено глобально
copy-live-block-unavailable = Реальне виконання недоступне

## Task state, mode and exit labels.

copy-state-system-paused = Призупинено глобально
copy-state-force-stopped = Примусово зупинено
copy-state-entries-blocked = Входи заблоковано
copy-state-running-live = Працює
copy-state-running-paper = Працює
copy-mode-paper = Віртуальний
copy-mode-live = Реальний
copy-exit-mode-buy-only = Мої правила виходу
copy-exit-mode-mirror = Дзеркалити продажі гаманця
copy-exit-mode-hybrid = Продажі гаманця та мої правила
copy-exit-target-sell = Гаманець продав
copy-exit-stop-loss = Стоп-лос
copy-exit-trailing-stop = Трейлінг-стоп
copy-exit-take-profit = Тейк-профіт
copy-exit-time-override = Вихід за часом
copy-exit-manual = Закрито вручну

## Shared wording

copy-request-failed = Запит не вдався
copy-keep-paused = Залишити призупиненим
copy-paused-suffix = · призупинено
copy-mode-paused = { $mode } · призупинено
copy-task-ref = «{ $name }» ({ $mode })
copy-metric-realized-pnl = Реалізований прибуток/збиток
copy-metric-unrealized-pnl = Нереалізований прибуток/збиток
copy-metric-win-rate = Відсоток виграшних угод
copy-metric-budget-spent = Витрачено бюджету
copy-metric-median-arrival = Медіана надходження
copy-metric-open-holdings = Відкриті утримувані токени
copy-record-won-lost = виграно: { $won } · програно: { $lost }
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Виконання
copy-kind-exits = Виходи
copy-kind-skips = Пропуски
copy-kind-errors = Помилки
copy-field-per-trade-cap = Ліміт на угоду
copy-field-per-token-cap = Ліміт на токен
copy-field-total-budget = Загальний бюджет
copy-field-slippage = Проковзування
copy-rules-wallet-sells-only = Лише продажі гаманця
copy-filter-copy-setting-required = Налаштування копітрейдингу (обов’язково)
copy-filter-copy-setting-not-required = Налаштування копітрейдингу (необов’язково)
copy-stable-copy-setting-on = Налаштування копітрейдингу (копіюється)
copy-stable-copy-setting-off = Налаштування копітрейдингу (не копіюється)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } закритий раунд
        [few] { $count } закриті раунди
        [many] { $count } закритих раундів
       *[other] { $count } закритого раунду
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } відкритий утримуваний токен
        [few] { $count } відкриті утримувані токени
        [many] { $count } відкритих утримуваних токенів
       *[other] { $count } відкритого утримуваного токена
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } токен із ціною · без ціни: { $unpriced }
        [few] { $priced } токени з ціною · без ціни: { $unpriced }
        [many] { $priced } токенів із ціною · без ціни: { $unpriced }
       *[other] { $priced } токена з ціною · без ціни: { $unpriced }
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } утримуваний токен без ціни
        [few] { $count } утримувані токени без ціни
        [many] { $count } утримуваних токенів без ціни
       *[other] { $count } утримуваного токена без ціни
    }
copy-range-24h = 24 год
copy-range-7d = 7 д
copy-range-30d = 30 д
copy-range-all = Усе
copy-range-label =
    .aria-label = Діапазон дат

## Page strip

copy-page-title = Копітрейдинг
copy-strip-loading = Завантаження
copy-strip-unavailable = Недоступно
copy-strip-setup-required = Потрібне налаштування · для копітрейдингу потрібні гаманець і RPC
copy-strip-pause-all = Призупинити все
copy-strip-resume = Відновити обробку
copy-strip-settings = Налаштування
copy-strip-add-wallet = Додати гаманець
copy-strip-paused-globally = Призупинено глобально · нових копій немає, виходи працюють
copy-strip-force-stopped = Примусово зупинено · нічого не копіюється
copy-strip-loss-limit = Ліміт збитків · нові входи заблоковано, виходи працюють
copy-strip-idle-paused =
    { $count ->
        [one] Очікування · призупинено завдань: { $count }
        [few] Очікування · призупинено завдань: { $count }
        [many] Очікування · призупинено завдань: { $count }
       *[other] Очікування · призупинено завдань: { $count }
    }
copy-strip-idle-empty = Очікування · завдань ще немає
copy-strip-processing = Обробка · віртуальних: { $paper }
copy-strip-processing-live = Обробка · реальних: { $live } · віртуальних: { $paper }
copy-figures-label =
    .aria-label = Підсумки копітрейдингу
copy-figure-marked-at-pool = За ціною пулу
copy-figure-across-tasks = За всіма завданнями
copy-figure-budget-lifetime = Витрати за весь час увімкнених завдань
copy-figure-budget-none = Немає увімкнених завдань
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } угода
        [few] { $count } угоди
        [many] { $count } угод
       *[other] { $count } угоди
    }
copy-figure-arrival-none = Немає зразків від увімкнених завдань

## Page frame

copy-load-failed = Не вдалося завантажити копітрейдинг: { $error }
copy-resume-all-title = Відновлення обробки копіювання
copy-resume-all-message =
    { $count ->
        [one] { $count } реальне завдання надсилатиме реальні свопи, коли його гаманець знову торгуватиме.
        [few] { $count } реальні завдання надсилатимуть реальні свопи, коли їхні гаманці знову торгуватимуть.
        [many] { $count } реальних завдань надсилатимуть реальні свопи, коли їхні гаманці знову торгуватимуть.
       *[other] { $count } реального завдання надсилатимуть реальні свопи, коли їхні гаманці знову торгуватимуть.
    }
copy-toast-resumed-all = Обробку копіювання відновлено
copy-toast-paused-all = Усю обробку копіювання призупинено
copy-toast-global-failed = Не вдалося змінити обробку копіювання

## Onboarding

copy-onboarding-title = Копіюйте гаманці, яким довіряєте, — спершу перевірте їх у віртуальному режимі
copy-onboarding-paper-body = Кожне завдання починається у віртуальному режимі: угоди цільового гаманця симулюються за двома котируваннями роутера з оціночними мережевими комісіями, а ваші правила виходу працюють на віртуальному портфелі. Активуйте реальну торгівлю для кожного гаманця окремо, коли його віртуальні результати це заслужать.
copy-onboarding-add = Додайте перший гаманець
copy-setup-gate-title = Для копітрейдингу потрібен гаманець
copy-onboarding-observe = Спостереження
copy-onboarding-observe-detail = Виявляйте свопи гаманця, не витрачаючи { -sol }.
copy-onboarding-evaluate = Оцінка
copy-onboarding-evaluate-paper-detail = Вивчайте віртуальний прибуток/збиток, відсоток виграшних угод, пропуски, швидкість виявлення та різницю з ціною цільового гаманця.
copy-onboarding-arm = Активація
copy-onboarding-arm-detail = Пройдіть перевірки готовності, потім увімкніть реальні свопи.

## Wallet list

copy-list-label =
    .aria-label = Копійовані гаманці
copy-list-title = Гаманці
copy-list-compare = Порівняти
copy-list-sort-label = Сортування гаманців
copy-list-count = Активних: { $active } · Усього: { $total }
copy-sort-pnl = Прибуток/збиток
copy-sort-state = Стан
copy-sort-name = Назва
copy-compare-label =
    .aria-label = Порівняння гаманців

## Dialog chrome

copy-dialog-close =
    .aria-label = Закрити
copy-editor-title-add = Додати гаманець
copy-editor-sub-add = Нові завдання починаються у віртуальному режимі
copy-arm-title = Активація реального копіювання
copy-arm-sub = Реальні свопи з вашого гаманця
copy-arm-keep-paper = Залишити віртуальний режим
copy-arm-confirm = Активувати реальну торгівлю
copy-profile-title = Профіль гаманця
copy-profile-sub = Що цей бот бачив про гаманець

## Settings dialog

copy-settings-title = Налаштування копітрейдингу
copy-settings-subtitle = Глобальна політика для кожного завдання
copy-settings-filter-warning = За стандартного налаштування фільтрації це відхиляє майже кожен токен, тож нічого не копіюється. Залиште вимкненим, доки ваші фільтри не пропускатимуть токени, якими торгують ваші гаманці.
copy-settings-unit-seconds = с
copy-settings-unit-trades = угод
copy-settings-unit-tasks = завдань
copy-settings-unit-rounds = раундів
copy-settings-save = Зберегти налаштування
copy-settings-load-failed = Не вдалося завантажити налаштування копіювання
copy-settings-saved = Налаштування копітрейдингу збережено
copy-outcome-paper-filled-legacy = Попередня віртуальна оцінка
copy-outcome-paper-filled-quote = Віртуальне виконання за котируванням
copy-outcome-paper-sell-legacy = Попередня віртуальна оцінка виходу
copy-outcome-paper-sell-quote = Віртуальний вихід за котируванням

## Paper execution model

copy-paper-model = Модель віртуального виконання
copy-paper-model-note = Кожне віртуальне виконання використовує два котирування роутера під розмір угоди, отримані з інтервалом, що дорівнює припущеному очікуванню. Друге котирування, як і раніше, має досягати мінімального виходу першого. Ціни пулу, як і раніше, оцінюють утримувані токени й запускають правила виходу. Мережева та пріоритетна комісії — одна оцінка, що враховується один раз; комісія платформи вже входить у котирований вихід. MEV і порядок валідаторів не симулюються.
copy-paper-wait-label = Припущене очікування виконання
copy-paper-queue-label = Розмір черги на завдання
copy-paper-legacy-title = Попередні віртуальні оцінки
copy-paper-legacy-note = Оцінки лише для читання, збережені з часів до симуляції за котируваннями. Вони не входять до витрат і утримуваних токенів поточного портфеля й не пропонують дій. Залишки оцінюються за останньою збереженою ціною кожного токена, а не за поточною ринковою ціною.
copy-paper-legacy-spent = Попередні оціночні витрати
copy-paper-legacy-realized = Попередня реалізована оцінка
copy-paper-legacy-col-tokens = Токени
copy-paper-legacy-col-cost = Залишкова вартість ({ -sol })
copy-paper-legacy-col-realized = Реалізована оцінка ({ -sol })
copy-paper-legacy-col-price = Остання збережена ціна ({ -sol })
copy-paper-legacy-col-remaining = Залишковий P&L за збереженою ціною ({ -sol })

## Workspace

copy-tab-overview = Огляд
copy-tab-holdings = Утримувані токени
copy-tab-activity = Активність
copy-tab-rules = Правила
copy-tab-execution = Виконання
copy-tabs-label = Подання завдання
copy-workspace-select = Виберіть гаманець, щоб відкрити його робочу область.
copy-workspace-loading = Завантаження завдання…
copy-workspace-load-failed = Не вдалося завантажити це завдання: { $error }

copy-state-detail-paper = Працює у віртуальному режимі · угоди симулюються, кошти не витрачаються
copy-state-detail-live = Працює в реальному режимі · угоди гаманця копіюються реальними свопами
copy-state-detail-system-paused = Очікування · обробку копіювання призупинено глобально, виходи працюють
copy-state-detail-entries-blocked = Входи заблоковано лімітом збитків · виходи працюють
copy-state-detail-force-stopped = Примусово зупинено · нічого не копіюється

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Відновлення зберігає той самий ліміт, тож завдання знову призупиниться, поки угоди надходять із запізненням. Перевірте потік RPC або збільште ліміт надходження в налаштуваннях.
copy-paused-resume-detached = Відновлення знову почне стежити за гаманцем.
copy-paused-holdings-rules =
    { $count ->
        [one] Його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
        [few] Його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
        [many] Його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
       *[other] Його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Продажі гаманця й далі закривають його відкриті утримувані токени ({ $count }).
        [few] Продажі гаманця й далі закривають його відкриті утримувані токени ({ $count }).
        [many] Продажі гаманця й далі закривають його відкриті утримувані токени ({ $count }).
       *[other] Продажі гаманця й далі закривають його відкриті утримувані токени ({ $count }).
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Продажі гаманця та його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
        [few] Продажі гаманця та його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
        [many] Продажі гаманця та його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
       *[other] Продажі гаманця та його правила виходу й далі закривають його відкриті утримувані токени ({ $count }).
    }

copy-watch-state-catching-up = Стеження за гаманцем: наздоганяє. Перевірка цього гаманця через { -helius }.
copy-watch-state-watching = Стеження за гаманцем: стежить. Перевірка цього гаманця через { -helius }.
copy-watch-last-check = Остання перевірка: { $ago }.
copy-watch-recovery-active = Стеження за гаманцем активне
copy-watch-recovery-catching-up = Стеження за гаманцем наздоганяє
copy-watch-recovery-still-paused = Завдання копіювання досі призупинено. Відновіть копіювання, коли будете готові.
copy-watch-recovery-title = Відновлення стеження за гаманцем
copy-watch-recovery-processing-failed = Не вдалося обробити активність гаманця. Збережений прогрес зберігається. Повторіть спробу після усунення проблеми.
copy-watch-recovery-provider-failed = Перевірки через { -helius } не вдалися. Збережений прогрес зберігається. Повторіть спробу, коли провайдер буде доступний.
copy-watch-recovery-budget-intro = Цей гаманець має більше активності, ніж може перевірити поточне стеження. Виберіть, як продовжити.
copy-watch-approve = Спробувати наздогнати через { -helius }
copy-watch-approve-help = Продовжує зі збереженого прогресу. Може витратити більше кредитів { -helius } і все одно може відставати.
copy-watch-approve-unavailable = Наздоганяння через { -helius } недоступне. Налаштуйте увімкнений RPC-ендпоінт { -helius }, щоб продовжити без пропуску неперевіреної активності.
copy-watch-no-provider = Для цього стеження немає підтримуваного провайдера наздоганяння.
copy-watch-budget-label = Підписів за одну перевірку
copy-watch-budget-hint = Або пропустіть неперевірену активність і відновіть з поточного моменту. Виберіть від { $min } до { $max } підписів за перевірку; вищий ліміт може використовувати більше RPC-викликів.
copy-watch-ack = Я розумію, що пропущена активність не буде скопійована.
copy-watch-toast-range = Виберіть від { $min } до { $max } підписів за опитування з кроком { $step }
copy-watch-toast-ack = Підтвердьте, що підписи після останньої завершеної перевірки буде пропущено
copy-watch-resumed = Стеження за гаманцем відновлено з поточного моменту; завдання копіювання залишається призупиненим
copy-watch-resume-failed = Не вдалося відновити стеження за гаманцем
copy-watch-retry-started = Повторну спробу стеження за гаманцем запущено зі збереженого прогресу; завдання копіювання залишається призупиненим
copy-watch-retry-failed = Не вдалося повторити стеження за гаманцем
copy-watch-approve-title = Дозволити наздоганяння через { -helius } для цього гаманця
copy-watch-approve-message = { -helius } може перевіряти успішні транзакції Solana зі збереженого прогресу, не пропускаючи неперевірений інтервал. Наразі він стягує 10 кредитів за 100 повернутих повних транзакцій із округленням угору, з мінімумом 10 кредитів за запит. Одна перевірка може робити кілька запитів; використання та ціни провайдера можуть відрізнятися. Копіювання залишається призупиненим, доки ви не відновите його окремо.
copy-watch-approve-confirm = Дозволити для цього гаманця
copy-watch-approved = Стеження за гаманцем запущено зі збереженого прогресу; завдання копіювання залишається призупиненим
copy-watch-restore-failed = Не вдалося відновити стеження за гаманцем

copy-action-pause = Призупинити
copy-action-resume = Відновити
copy-action-resume-copy = Відновити копіювання
copy-action-resume-from-now = Відновити з поточного моменту
copy-action-retry-watch = Повторити стеження за гаманцем
copy-action-return-paper = Повернути у віртуальний режим
copy-action-edit-rules = Редагувати правила
copy-action-clone = Клонувати
copy-action-profile = Профіль гаманця
copy-resume-live-title = Відновлення реального копіювання
copy-resume-live-message = «{ $name }» надсилатиме реальні свопи з вашого гаманця, коли цей гаманець знову торгуватиме.
copy-resume-live-confirm = Відновити реальний режим
copy-task-resumed = Завдання відновлено
copy-task-paused = Завдання призупинено
copy-task-state-failed = Не вдалося змінити стан завдання
copy-return-paper-message = Нові копії від «{ $name }» знову симулюватимуться без витрат { -sol }.
copy-return-paper-cancel = Залишити реальний режим
copy-task-returned-paper = Завдання повернено у віртуальний режим
copy-mode-change-failed = Не вдалося змінити режим виконання
copy-delete-title = Видалення завдання копіювання
copy-delete-message = Видалити «{ $name }»? Його рішення та віртуальні результати буде видалено, а за гаманцем більше не стежитимуть для цього завдання.
copy-delete-confirm = Видалити завдання
copy-delete-cancel = Залишити завдання
copy-task-deleted = Завдання копіювання видалено
copy-task-delete-failed = Не вдалося видалити завдання копіювання

## Overview tab

copy-overview-results = Результати
copy-analytics-load-failed = Не вдалося завантажити аналітику: { $error }
copy-analytics-loading = Завантаження аналітики…
copy-exit-bucket =
    { $count ->
        [one] { $count } продаж · { $pnl }
        [few] { $count } продажі · { $pnl }
        [many] { $count } продажів · { $pnl }
       *[other] { $count } продажу · { $pnl }
    }
copy-overview-average-win = Середній виграш
copy-overview-average-loss = Середній програш { $amount }
copy-overview-profit-factor = Профіт-фактор
copy-overview-profit-factor-note = Загальні виграші ÷ загальні програші
copy-overview-average-hold = Середній час утримання
copy-overview-average-hold-note = Від входу до виходу
copy-overview-best-round = Найкращий раунд
copy-overview-worst-round = Найгірший { $amount }
copy-overview-curve-title = Накопичений прибуток/збиток
copy-overview-exits-title = Продажі за виходом
copy-overview-skips-title = Чому угоди пропущено
copy-book-title-live = Реальний портфель
copy-book-title-paper = Віртуальний портфель
copy-book-all-time = За весь час
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] купівля
        [few] купівлі
        [many] купівель
       *[other] купівлі
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] вихід за вашими правилами
        [few] виходи за вашими правилами
        [many] виходів за вашими правилами
       *[other] виходу за вашими правилами
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] продаж гаманця
        [few] продажі гаманця
        [many] продажів гаманця
       *[other] продажу гаманця
    }
copy-book-manual-closes = <strong>{ $count }</strong> закрито вручну
copy-book-skipped = <strong>{ $count }</strong> пропущено
copy-book-failed = <strong>{ $count }</strong> невдало
copy-book-closed = закрито: { $count }
copy-book-budget-note = { $mode }: витрачено з { $total } · залишилось { $remaining }
copy-check-passed = пройдено
copy-check-not-passed = не пройдено
copy-readiness-title = Перед переходом у реальний режим
copy-readiness-live-note = Це завдання торгує в реальному режимі. Поверніть його у віртуальний режим із заголовка вище.
copy-readiness-all-pass = Усі перевірки пройдено.
copy-readiness-needs-review = Активація потребує явного перегляду того, що не готове.
copy-readiness-arm = Переглянути й активувати реальну торгівлю

## Rules tab and review

copy-rules-title = Чинні правила
copy-rules-size-ratio = { $pct } від угоди гаманця
copy-rules-size-fixed = { $amount } за копію
copy-rules-target-any = Будь-який розмір
copy-rules-target-min = Щонайменше { $amount }
copy-rules-target-max = Щонайбільше { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Перевизначення завдання · трейдер { $value }
copy-rules-source-default = Стандарт трейдера
# $profit is the step's signed profit level; $share is the share of the position it sells, both formatted.
copy-rules-take-profit-step = При { $profit } продати { $share }
copy-rules-not-used = Не використовується: вирішують продажі гаманця
copy-rules-col-rule = Правило
copy-rules-col-applies = Застосування
copy-rules-col-source = Джерело
copy-rules-budget-note = Витрачено в режимі «{ $mode }»: { $spent } · залишилось { $remaining }
copy-rules-token-copies =
    { $count ->
        [one] Приблизно { $count } повна копія одного токена
        [few] Приблизно { $count } повні копії одного токена
        [many] Приблизно { $count } повних копій одного токена
       *[other] Приблизно { $count } повної копії одного токена
    }
copy-rules-sizing = Розмір
copy-rules-copy-size = Розмір копії
copy-rules-entry-filters = Фільтри входу
copy-rules-target-size = Розмір угоди гаманця
copy-rules-repeat-buys = Повторні купівлі
copy-rules-repeat-first-only = Лише перша купівля кожного токена
copy-rules-repeat-every = Кожна купівля, до ліміту на токен
copy-rules-filter-pass = Проходження фільтрації
copy-rules-filter-required = Обов’язково
copy-rules-filter-not-required = Необов’язково
copy-rules-filter-task-override = Перевизначення завдання
copy-rules-stable-trades = Угоди зі стейблкоїнами
copy-rules-stable-copied = Копіюється за поточною ціною SOL
copy-rules-stable-not-copied = Не копіюється
copy-rules-exits = Виходи
copy-rules-exits-inactive = Утримувані токени продаються лише тоді, коли продає гаманець; правила нижче в цьому режимі не діють.

## Exit rules

copy-rule-status = Стан
copy-rule-on = Увімк.
copy-rule-off = Вимк.
copy-rule-unit-seconds = с
copy-rule-unit-minutes = хв
copy-rule-stop-loss-threshold = Продає за збитку
copy-rule-stop-loss-min-hold = Не раніше ніж через
copy-rule-no-minimum = Без мінімуму
copy-rule-partial-exits = Спершу продати частину
copy-rule-partial-allowed = Дозволено
copy-rule-partial-full-only = Лише повний вихід
copy-rule-partial-size = Частка для продажу
copy-rule-trailing-activation = Активується за зростання
copy-rule-trailing-distance = Продає за падіння від піку на
copy-rule-take-profit-target = Продає за зростання
copy-rule-take-profit-target-hint = Ціль тут замінює сходинки тейк-профіту трейдера однією сходинкою, що продає всю позицію
copy-rule-time-duration = Перевіряє після утримання
copy-rule-time-threshold = Продає, поки прибуток/збиток на рівні або нижче
copy-preset-inherit = Стандарти трейдера
copy-preset-conservative = Консервативний
copy-preset-balanced = Збалансований
copy-preset-aggressive = Агресивний
copy-preset-custom = Власний
copy-validate-stop-loss = Стоп-лос має бути більшим за 0% і не більшим за 100%.
copy-validate-partial-size = Частка для продажу має бути від 0% до 100%.
copy-validate-min-hold = Мінімальний час утримання має бути цілою кількістю секунд.
copy-validate-trailing-activation = Активація трейлінгу має бути більшою за 0% і не більшою за 100%.
copy-validate-trailing-distance = Відстань трейлінгу має бути більшою за 0% і не більшою за 100%.
copy-validate-take-profit = Тейк-профіт має бути більшим за 0%.
copy-validate-time-duration = Правило часу потребує тривалості більше нуля.
copy-validate-time-threshold = Поріг правила часу — це збиток: використайте 0% або від’ємне число.
copy-warning-mirror = Утримувані токени закривають лише продажі гаманця: жоден стоп-лос їх не захищає, а токен, який гаманець ніколи не продає, залишається утримуваним.
copy-warning-no-rules = Жодне правило виходу не ввімкнено, а продажі гаманця ігноруються: утримувані токени ніколи не продаються.
copy-warning-no-stop-loss = Стоп-лос не діє: токен, що падає, утримується, доки його не продасть інше правило або гаманець.
copy-warning-stop-delay = Стоп-лос чекає { $hold } після кожної купівлі: токен, що падає швидше, закривається значно нижче за { $threshold }.
copy-warning-take-profit-small = Тейк-профіт на рівні { $target } не перевищує витрат на продаж: допуск проковзування до { $slippage } і комісія за своп { $fee }. Невелика ціль може їх не покрити. Допуск — це межа ціни продажу, а не прогнозовані витрати.
copy-warning-trailing-distance = Відстань трейлінгу не менша за прибуток активації, тож активований трейлінг може продати нижче за вхід.

## Execution tab

copy-execution-title = Якість виконання
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Будь-який
copy-execution-limit-on =
    { $count ->
        [one] Призупиняє, якщо середнє понад { $limit } за { $count } угоду
        [few] Призупиняє, якщо середнє понад { $limit } за { $count } угоди
        [many] Призупиняє, якщо середнє понад { $limit } за { $count } угод
       *[other] Призупиняє, якщо середнє понад { $limit } за { $count } угоди
    }
copy-execution-limit-off = Аварійний вимикач вимкнено
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } угода зафіксована в момент здійснення
        [few] { $count } угоди зафіксовані в момент здійснення
        [many] { $count } угод зафіксовано в момент здійснення
       *[other] { $count } угоди зафіксовано в момент здійснення
    }
copy-execution-p95 = Надходження p95
copy-execution-target-price-difference = Медіана різниці з ціною цілі
copy-execution-target-difference-samples =
    { $count ->
        [one] { $count } виміряне виконання
        [few] { $count } виміряні виконання
        [many] { $count } виміряних виконань
       *[other] { $count } виміряного виконання
    }
copy-execution-target-difference-worst = Найбільша різниця з ціною цілі
copy-execution-target-difference-average = Середня різниця з ціною цілі { $amount }
copy-execution-delay-title = Затримка виявлення
copy-execution-delay-note = Час від блока гаманця до моменту, коли цей бот побачив угоду. Відтворення після простою не враховуються.
copy-execution-delay-limit = Стовпці за межею ліміту надходження ({ $limit }) позначено бурштиновим.
copy-execution-fastest = Найшвидше
copy-execution-average = Середнє
copy-execution-slowest = Найповільніше
copy-execution-target-difference-title = Різниця з ціною цілі
copy-execution-target-difference-note = Порівнює ціну кожного виконання з ціною власної угоди гаманця, коли обидві більші за нуль. Додатне значення означає гірше виконання, ніж у гаманця: заплачено більше під час купівлі або отримано менше під час продажу. Це порівняння, а не допуск проковзування завдання.
copy-execution-wait-note = Затримка виявлення відраховується від блока гаманця. Очікування виконання — припущення віртуальної моделі; час, який рішення проводить у черзі завдання, додається до нього й записується в кожне виконання.
copy-execution-samples = Зразки
copy-execution-median = Медіана
copy-execution-worst = Найгірше
copy-execution-decisions = Рішення в діапазоні

## Compare view

copy-compare-title = Порівняння гаманців
copy-compare-back = Назад до гаманця
copy-compare-load-failed = Не вдалося завантажити порівняння: { $error }
copy-compare-loading = Завантаження порівняння…
copy-compare-empty = Немає завдань для порівняння.
copy-compare-empty-message = Додайте завдання копіювання, щоб порівняти його результати з іншими.
copy-compare-curve-title = Накопичений реалізований прибуток/збиток
copy-table-wallet = Гаманець
copy-table-mode = Режим
copy-table-rounds = Раунди
copy-table-realized = Реалізовано ({ -sol })
copy-table-profit-factor = Профіт-фактор
copy-table-average-hold = Сер. утримання
copy-table-target-difference = Медіана різниці з ціною цілі

## Charts

copy-chart-curve-label = Накопичений прибуток/збиток { $amount } { -sol }
copy-chart-compare-label = Накопичений прибуток/збиток за завданнями
copy-chart-empty-curve = У цьому діапазоні ще немає закритих раундів.
copy-chart-empty-bars = У цьому діапазоні нічого не записано.
copy-chart-empty-histogram = У цьому діапазоні немає зразків надходження.
copy-chart-empty-compare = У цьому діапазоні немає закритих раундів для порівняння.
copy-chart-histogram-title = { $count } із { $total }

## Wallet profile

copy-profile-copy = Копіювати цей гаманець
copy-profile-copy-other = Копіювати за іншими правилами
copy-profile-loading = Завантаження профілю гаманця…
copy-profile-watch-title = Стеження
copy-profile-watched = Під наглядом
copy-profile-watch-resume-hint = Відновлення завдання знову вмикає стеження
copy-profile-watch-add-hint = Додавання завдання запускає стеження
copy-profile-stream = Потік
copy-profile-subscribed = Підписано
copy-profile-not-subscribed = Не підписано
copy-profile-sources =
    { $count ->
        [one] { $count } джерело
        [few] { $count } джерела
        [many] { $count } джерел
       *[other] { $count } джерела
    }
copy-profile-last-activity = Остання активність
copy-profile-last-error = Остання помилка
copy-profile-own-wallet = Це один із ваших власних гаманців; копіювати його заборонено.
copy-profile-observed-title = Виявлені угоди
copy-profile-observed-none = Цей бот ще не бачив угод цього гаманця. Віртуальне завдання спостерігає за ним, не витрачаючи { -sol }.
copy-profile-swaps-seen = Виявлено свопів
copy-profile-swaps-seen-note = Окремі свопи гаманця за всіма вашими завданнями
copy-profile-buys-sells = Купівлі / продажі
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Токенів у торгівлі
copy-profile-first-seen = Уперше виявлено
copy-profile-last-seen = Востаннє виявлено
copy-profile-tasks-title = Ваші завдання для цього гаманця
copy-table-task = Завдання

## Arm live dialog

copy-arm-acks-left = Залишилося підтвердити: { $count }
copy-arm-readiness-title = Готовність за віртуальним портфелем
copy-arm-exposure-title = Експозиція
copy-arm-per-copy = За копію
copy-arm-budget-left-value = { $left } із { $total } { -sol }
copy-arm-budget-left = Залишок реального бюджету
copy-arm-budget-left-note = Віртуальні витрати враховуються окремо й не використовують його
copy-arm-exits = Виходи
copy-arm-stop-note = Не раніше ніж через { $hold } утримання: швидше падіння закриється нижче
copy-arm-shared = Цей гаманець також копіюють: { $tasks }. Кожне завдання копіює його угоди з власним бюджетом.
copy-arm-unavailable = Реальне виконання зараз недоступне; див. останню перевірку.
copy-arm-ack-real-native = Реальні { -sol }: це завдання може витратити з вашого гаманця до { $budget } { -sol }, щонайбільше { $trade } { -sol } за копію.
copy-arm-ack-fees = Реальні копії сплачують справжні мережеві комісії та проковзування; віртуальні результати не гарантують реальних.
copy-arm-ack-unready = Деякі перевірки готовності не пройдено. Усе одно активувати це завдання.
copy-arm-lead = «{ $name }» копіюватиме угоди цього гаманця реальними свопами з вашого гаманця.
copy-arm-confirmation-missing = Не вдалося завантажити підтвердження реального режиму
copy-arm-armed = Реальне копіювання активовано
copy-arm-failed = Не вдалося активувати реальне копіювання

## Holdings tab

copy-holdings-title = Утримувані токени
copy-holdings-view-label = Подання утримуваних токенів
copy-holdings-view-open = Відкриті ({ $count })
copy-holdings-view-closed = Закриті раунди ({ $count })
copy-holdings-reset = Скинути віртуальний портфель
copy-holdings-live-note = Реальні копії — це справжні позиції.
copy-holdings-open-positions = Відкриті позиції
copy-holdings-token-details = Відкрити деталі токена
copy-holdings-opened = Відкрито { $time }
copy-holdings-no-pool-price = Немає ціни пулу
copy-holdings-close = Закрити
copy-holdings-close-pending = Закриття очікується
copy-holdings-activity = Активність
copy-holdings-no-exit-rule = Немає правила виходу
copy-holdings-watch-stop = Стоп { $level }
copy-holdings-watch-stop-until = Стоп { $level } через { $span }
copy-holdings-watch-take = Тейк { $level }
copy-holdings-watch-trail = Трейлінг { $level }
copy-holdings-watch-trail-arms = Трейлінг активується { $level }
copy-holdings-watch-time = Час ≤ { $level }
copy-holdings-watch-time-until = Час ≤ { $level } через { $span }
copy-holdings-watch-wallet-sells = Продажі гаманця
copy-holdings-empty = Відкритих віртуальних утримуваних токенів немає. Купівлі, скопійовані з гаманця, з’являться тут.
copy-holdings-col-token = Токен
copy-holdings-col-cost = Вартість ({ -sol })
copy-holdings-col-entry = Вхід ({ -sol })
copy-holdings-col-mark = Оцінка ({ -sol })
copy-holdings-col-peak = Пік
copy-holdings-col-pnl = Прибуток/збиток ({ -sol })
copy-holdings-col-exit-rules = Правила виходу
copy-holdings-col-held = Утримання
copy-holdings-col-actions = Дії
copy-holdings-col-invested = Інвестовано ({ -sol })
copy-holdings-col-proceeds = Виручка ({ -sol })
copy-holdings-col-exit = Вихід
copy-holdings-col-closed = Закрито
copy-holdings-marks-note = Ціни вказано в { -sol } за токен. Оцінка й рівні виходу використовують ціну пулу, яка також запускає правила виходу; записані виконання використовують котирування роутера. Вхід включає оціночну мережеву комісію купівлі; пік і рівні виходу рахуються відносно нього, тож утримуваний токен відкривається з піком нижче входу. Наведіть курсор, щоб побачити ціну пулу.
copy-holdings-paused-rules = Призупинено: нових копій немає. Ваші правила виходу й далі закривають ці утримувані токени.
copy-holdings-paused-mirror = Призупинено: нових копій немає. Продажі гаманця й далі закривають ці утримувані токени.
copy-holdings-paused-hybrid = Призупинено: нових копій немає. Продажі гаманця та ваші правила виходу й далі закривають ці утримувані токени.
copy-holdings-closed-load-failed = Не вдалося завантажити закриті раунди: { $error }
copy-holdings-closed-loading = Завантаження закритих раундів…
copy-holdings-closed-empty = Закритих раундів ще немає.
copy-holdings-closed-latest = Останні { $shown } із { $total } раундів.
copy-holdings-close-title = Закриття віртуального утримуваного токена
copy-holdings-close-quote-message = Закрити віртуальний утримуваний токен { $token } за двома котируваннями роутера з інтервалом близько { $delay }. Виконання використовує друге котирування, яке має досягати мінімального виходу першого, за вирахуванням оціночної мережевої комісії. Якщо придатного котирування немає, утримуваний токен залишається відкритим без змін.
copy-holdings-close-confirm = Закрити
copy-holdings-keep = Залишити
copy-holdings-closed = { $token } закрито
copy-holdings-sold-at = Продано за { $price }
copy-holdings-close-failed = Не вдалося закрити утримуваний токен
copy-holdings-close-pending-detail = Закриття в черзі або виконується. Результат з’явиться в розділі «Активність».
copy-holdings-close-unconfirmed = Результат закриття не підтверджено
copy-holdings-close-unconfirmed-detail = З’єднання перервалося до отримання результату, а в розділі «Активність» його ще немає. Перевірте «Активність», перш ніж закривати знову.
copy-holdings-closed-check-activity = Утримуваний токен закрито. Записане виконання є в розділі «Активність».
copy-holdings-reset-quote-message = Почати «{ $name }» спочатку: його поточні віртуальні утримувані токени, витрати, виконання, виходи й пропуски буде очищено, і почнеться новий портфель на основі котирувань. Попередні оцінки та реальна історія залишаться. Правила та гаманець залишаться.
copy-holdings-reset-cancel = Залишити історію
copy-holdings-reset-done = Віртуальний портфель скинуто
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } рішення видалено
        [few] { $count } рішення видалено
        [many] { $count } рішень видалено
       *[other] { $count } рішення видалено
    }
copy-holdings-reset-failed = Не вдалося скинути віртуальний портфель

## Activity tab

copy-activity-title = Активність
copy-activity-filter-label = Фільтр активності
copy-filter-all = Усі
copy-outcome-live-submitted = Реальну купівлю надіслано
copy-outcome-live-confirmed = Реальну купівлю підтверджено
copy-outcome-live-failed = Реальна купівля не вдалася
copy-outcome-paper-sell-observed = Віртуальний продаж · гаманець продав
copy-outcome-live-sell-submitted = Реальний продаж надіслано
copy-outcome-live-sell-failed = Реальний продаж не вдався
copy-outcome-skipped = Пропущено
copy-activity-decision = Рішення
copy-activity-paper-exit-quote = Віртуальний вихід за котируванням · { $rule }
copy-activity-paper-exit-legacy = Попередня віртуальна оцінка виходу · { $rule }
copy-activity-filled = { $input } за { $price } · гаманець купив { $target }
copy-activity-filled-target-difference = { $input } за { $price } · гаманець купив { $target } · різниця з ціною цілі { $difference }
copy-activity-filled-slippage = { $input } за { $price } · гаманець купив { $target } · проковзування { $slippage }
copy-activity-quote-router = Роутер { $router } · котирування отримано { $initial } і { $landing }
copy-activity-quote-wait = Припущене очікування { $assumed } · у черзі { $queued }
copy-activity-quote-fees = Оціночні мережева та пріоритетна комісії { $fee }, враховані один раз · комісія платформи вже входить у котирований вихід
copy-activity-filled-unpriced = { $input } за { $price } · гаманець купив { $target } · ціна за угодою гаманця, ціни пулу немає
copy-activity-live-sized = { $sized } · гаманець купив { $target }
copy-activity-sell-nothing = Гаманець продав { $amount } · немає що продавати
copy-activity-written-off = Списано з нульовою виручкою: ціни пулу немає
copy-activity-sold = Продано токенів: { $tokens } за { $proceeds } за ціною { $price }
copy-activity-full-close = Повне закриття
copy-activity-partial-exit = Вихід { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = мінімум { $amount }
copy-activity-skip-maximum = максимум { $value }
copy-activity-skip-stale = запізнення { $arrival }, ліміт { $limit }
copy-activity-skip-latency = середнє { $average }, ліміт { $limit }
copy-activity-arrival-replayed = Відтворено через { $span } після блока
copy-activity-arrival-seen = Виявлено через { $span } після блока
copy-activity-link-wallet-tx = Транзакція гаманця
copy-activity-link-own-tx = Ваша транзакція
copy-activity-only-token = Лише цей токен
copy-activity-skipped-group = Пропущено ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } токен · від { $since }
        [few] { $tokens } токени · від { $since }
        [many] { $tokens } токенів · від { $since }
       *[other] { $tokens } токена · від { $since }
    }
copy-activity-mint-filter =
    .placeholder = Мінт токена
    .aria-label = Фільтр за мінтом токена
copy-activity-clear = Очистити
copy-activity-load-failed = Не вдалося завантажити активність: { $error }
copy-activity-loading = Завантаження активності…
copy-activity-no-match = Нічого не відповідає цьому фільтру.
copy-activity-empty = Рішень ще немає. Виконання, виходи й пропуски з’являтимуться тут, коли гаманець торгуватиме.
copy-activity-load-older = Завантажити давніші
copy-activity-start = Початок історії
copy-activity-older-failed = Не вдалося завантажити давнішу активність

## Task editor

copy-step-wallet = Гаманець
copy-step-sizing = Розмір
copy-step-entry = Фільтри входу
copy-step-exits = Виходи
copy-step-review = Перегляд
copy-editor-title-edit = Редагування: { $name }
copy-editor-title-clone = Клонування: { $name }
copy-editor-sub-edit = Завдання ({ $mode }) · зміни застосовуються до його наступних рішень
copy-editor-sub-clone = Ті самі правила, порожній віртуальний портфель, починається у віртуальному режимі
copy-editor-save-edit = Зберегти зміни
copy-editor-save-clone = Створити клон
copy-editor-save-create = Створити віртуальне завдання
copy-editor-clone-suffix = (копія)
copy-editor-discard-edit = Скасувати зміни
copy-editor-discard-create = Скасувати це завдання
copy-editor-discard-edit-message = Ваші зміни в «{ $name }» не збережено.
copy-editor-discard-create-message = Введені досі гаманець і правила не збережено.
copy-editor-discard-confirm = Скасувати
copy-editor-keep-editing = Продовжити редагування
copy-editor-toast-updated = Завдання оновлено
copy-editor-toast-clone = Клон створено
copy-editor-toast-created = Віртуальне завдання створено
copy-unit-native = { -sol }
copy-editor-any = Будь-який
copy-editor-duplicate = Уже копіюється: { $tasks }. Це завдання знову копіює ті самі угоди зі своїми правилами та бюджетом.
copy-editor-wallet = Гаманець
copy-editor-wallet-identity = Гаманець завдання — це його ідентичність. Щоб скопіювати інший гаманець за цими правилами, клонуйте завдання.
copy-editor-address-label = Адреса гаманця
copy-editor-address-placeholder = Адреса гаманця Solana
copy-editor-address-help-clone = Ті самі правила з порожнім віртуальним портфелем. Залиште цей гаманець, щоб перевірити на ньому інші правила, або введіть інший.
copy-editor-address-help-create = Гаманець, чиї купівлі (і, за вашим вибором, продажі) копіює це завдання.
copy-editor-name-label = Назва <em>необов’язково</em>
copy-editor-name-placeholder = напр. Швидкий ротатор
copy-editor-enabled-title = Обробляти угоди гаманця
copy-editor-enabled-help = Вимкнено — завдання залишається призупиненим, доки ви його не відновите.
copy-editor-note-live = Це завдання реальне: зміни застосовуються до його наступних реальних копій.
copy-editor-note-paper-quote = Завдання працюють у віртуальному режимі, доки ви їх не активуєте: угоди симулюються за котируваннями роутера, кошти не витрачаються.
copy-editor-copy-size = Розмір копії
copy-editor-sizing-fixed = Фіксована сума
copy-editor-sizing-ratio = Частка угоди гаманця
copy-editor-amount-fixed = Сума за копію
copy-editor-amount-ratio = Частка кожної угоди
copy-editor-amount-help-fixed = Витрачається на кожну скопійовану купівлю, щонайменше { $minimum }.
copy-editor-amount-help-ratio = Від власної купівлі гаманця, до ліміту на угоду.
copy-editor-help-trade-cap = Жодна окрема копія не витрачає більше.
copy-editor-help-token-cap = Загальні витрати на один токен.
copy-editor-help-budget = Усе, що це завдання може витратити за весь час; віртуальний і реальний режими рахують власні витрати окремо.
copy-editor-preview-title = Скільки коштує копія
copy-editor-preview-empty = Введіть розмір, щоб побачити, скільки коштує копія.
copy-editor-preview-example = Гаманець купує на { $target } → ви копіюєте на <strong>{ $copy }</strong>
copy-editor-preview-once = Один токен отримує одну копію на { $size }, бо кожен токен купується один раз
copy-editor-preview-token-cap =
    { $count ->
        [one] Один токен отримує щонайбільше { $count } копію на { $size }
        [few] Один токен отримує щонайбільше { $count } копії по { $size }
        [many] Один токен отримує щонайбільше { $count } копій по { $size }
       *[other] Один токен отримує щонайбільше { $count } копії по { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; бюджет покриває приблизно таких копій: { $count }. Мережеві та пріоритетні комісії додаються.
copy-editor-preview-summary-minimum = { $perToken }; бюджет покриває щонайменше таких копій: { $count }. Мережеві та пріоритетні комісії додаються.
copy-editor-target-min = Найменша скопійована угода гаманця
copy-editor-target-min-help = Ігнорувати менші купівлі гаманця. Залиште порожнім, щоб не було мінімуму.
copy-editor-target-max = Найбільша скопійована угода гаманця
copy-editor-target-max-help = Ігнорувати більші купівлі гаманця. Залиште порожнім, щоб не було максимуму.
copy-editor-buy-once-title = Купувати кожен токен один раз
copy-editor-buy-once-help = Копіювати лише першу купівлю токена гаманцем; наступні його купівлі пропускаються.
copy-editor-filter-require = Вимагати
copy-editor-filter-skip = Не вимагати
copy-editor-filter-help = Вимагати, щоб токен пройшов вашу фільтрацію перед копіюванням.
copy-editor-filter-warning = За стандартного налаштування фільтрації майже кожен токен не проходить, тож завдання, що вимагає проходження, нічого не копіює. Вмикайте це, лише коли ваші фільтри пропускають токени, якими торгує цей гаманець.
copy-editor-stable-help = Копіювати покупки гаманця, сплачені USDC або USDT, з розміром за поточною ціною SOL. Продажі токенів, які має це завдання, копіюються незалежно від валюти оплати.
copy-editor-exit-both = Обидва
copy-editor-exit-help-buy-only = Ваші правила нижче продають кожен утримуваний токен; продажі гаманця ігноруються.
copy-editor-exit-help-hybrid = Що станеться раніше: гаманець продасть або спрацює одне з ваших правил.
copy-editor-exit-help-mirror = Утримувані токени продаються лише тоді, коли продає гаманець. Ваші правила виходу не діють.
copy-editor-who-sells = Хто продає
copy-editor-preset = Пресет
copy-editor-preset-help = Пресет заповнює кожне правило нижче; після цього можна змінити будь-яке з них.
copy-editor-mirror-note = Ці правила не діють, доки вирішують продажі гаманця. Вони застосовуються, якщо перемкнути на «{ $mine }» або «{ $both }».
copy-editor-rule-inherit = Стандарт трейдера
copy-editor-inherit-value = Стандарт трейдера ({ $value })
copy-editor-rule-aria = Налаштування: { $rule }
copy-editor-rule-empty-uses = Порожнє поле використовує стандарт трейдера: { $value }
copy-editor-rule-follows = Слідує трейдеру: { $summary }
copy-editor-rule-follows-plain = Слідує налаштуванню трейдера.
copy-editor-rule-follows-own = Увімкнення — як у трейдера, значення — цього завдання: { $summary }
copy-editor-rule-off-note = Вимкнено для цього завдання, незалежно від налаштувань трейдера.
copy-task-unnamed = Завдання без назви
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = обробляє угоди після збереження
copy-editor-review-paused = збережеться призупиненим
copy-editor-review-waits = чекає відновлення обробки копій
copy-editor-processing-paused = Обробку копій призупинено для всіх завдань, тож це завдання нічого не копіює, доки її не відновлено.
copy-editor-error-address = Введіть чинну адресу гаманця Solana.
copy-editor-error-sizing = Кожне значення розміру має бути більшим за нуль.
copy-editor-error-min-copy = Копія має становити щонайменше { $minimum }: збільште суму за копію.
copy-editor-error-min-cap = Копія має становити щонайменше { $minimum }: збільште ліміт на угоду.
copy-editor-error-trade-cap = Ліміт на угоду не може перевищувати ліміт на токен.
copy-editor-error-token-cap = Ліміт на токен не може перевищувати загальний бюджет.
copy-editor-error-slippage = Проковзування має бути від { $min } до { $max }.
copy-editor-error-target-limits = Межі угод гаманця мають бути нульовими або більшими.
copy-editor-error-target-order = Найменша угода гаманця не може перевищувати найбільшу.

## Copy notices

copy-notice-task-unnamed = Завдання №{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Віртуальна копія-купівля
copy-notice-title-paper-sell = Віртуальна копія-продаж
copy-notice-title-paper-closed = Віртуальний утримуваний токен закрито
copy-notice-title-paper-exit = Віртуальний вихід: { $rule }
copy-notice-title-live-buy-submitted = Реальну копію-купівлю надіслано
copy-notice-title-live-buy-confirmed = Реальну копію-купівлю підтверджено
copy-notice-title-live-buy-failed = Реальна копія-купівля не вдалася
copy-notice-title-live-sell-submitted = Реальну копію-продаж надіслано
copy-notice-title-live-sell-failed = Реальна копія-продаж не вдалася
copy-notice-title-auto-paused = Завдання копіювання автоматично призупинено
copy-notice-detail-bought = Куплено на { $amount } { -sol }
copy-notice-detail-sold = Продано на { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% утримуваного
copy-notice-detail-full-close = Повне закриття
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Своп не вдався
