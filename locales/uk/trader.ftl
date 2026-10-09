# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Стоп-лос
trader-exit-type-take-profit = Тейк-профіт
trader-exit-type-roi = Ціль ROI
trader-exit-type-roi-exit = Ціль ROI
trader-exit-type-trailing-stop = Трейлінг-стоп
trader-exit-type-time-override = Часове перевизначення
trader-exit-type-time-rule = Часове правило
trader-exit-type-manual = Вручну
trader-exit-type-manual-close = Вручну
trader-exit-type-dca = DCA
trader-exit-type-unknown = Невідомо

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Статистика
trader-tab-strategy-control = Керування стратегіями
trader-tab-strategies = Стратегії
trader-tab-stop-loss = Стоп-лос
trader-tab-trailing-stop = Трейлінг-стоп
trader-tab-roi = Тейк-профіт
trader-tab-time-rules = Часові правила
trader-tab-dca = DCA
trader-tab-settings = Налаштування

## Feature status badges and their messages

trader-feature-coming-soon = Незабаром
    .message = Ця функція незабаром з’явиться й поки недоступна.
trader-feature-beta = Бета
trader-feature-disabled = Вимкнено
    .message = Цю функцію наразі вимкнено.

## Status bar and trading controls

trader-status-title = Автотрейдер
trader-status-loading = Завантаження...
trader-status-running = Працює
trader-status-stopped = Зупинено
trader-status-setup-required = Потрібне налаштування
trader-status-unavailable = Завершіть налаштування гаманця та RPC, щоб користуватися автотрейдером
trader-toggle-on = УВІМКНЕНО
trader-toggle-off = ВИМКНЕНО
trader-toggle-unavailable = НЕДОСТУПНО
trader-toggle-start-failed = Не вдалося запустити трейдер
trader-toggle-stop-failed = Не вдалося зупинити трейдер
trader-controls-title = Керування торгівлею
trader-halt-title = ТОРГІВЛЮ ЗУПИНЕНО
trader-halt-reason-default = Примусова зупинка вручну
trader-halt-resume = Відновити
trader-monitor-entry = Монітор входу
trader-monitor-exit = Монітор виходу
trader-monitor-master-off = Автотрейдер вимкнено
trader-loss-limit-title = Ліміт збитків за період
trader-loss-limit-resume = Відновити торгівлю
trader-loss-limit-reset = Скинути період
trader-loss-limit-off = Вимкнено
trader-loss-limit-none = Ліміт збитків за період не налаштовано
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = Скидання через { $hours } { $minutes }
trader-loss-limit-reached = ЛІМІТ ДОСЯГНУТО
trader-force-stop = Примусово зупинити все

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = Примусово зупинити торгівлю
    .message = Це негайно зупинить УСІ торгові операції. Продовжити?
    .confirm = Зупинити торгівлю
trader-loss-limit-resume-confirm = Відновити після ліміту збитків
    .message = Ліміт збитків за період зупинив нові входи. Відновлення дозволить трейдеру знову відкривати позиції до завершення періоду. Продовжити?
trader-loss-limit-reset-confirm = Скинути період ліміту збитків
    .message = Це очистить накопичений збиток за поточний період і почне новий. Продовжити?

## Toasts

trader-toast-control-failed = Не вдалося керувати автотрейдером
trader-toast-force-stop-on = Примусову зупинку активовано
trader-toast-force-stop-failed = Не вдалося активувати примусову зупинку
trader-toast-force-stop-cleared = Примусову зупинку знято
trader-toast-resume-failed = Не вдалося відновити торгівлю
trader-toast-loss-limit-reset-failed = Не вдалося скинути ліміт збитків
trader-toast-entry-monitor-failed = Не вдалося перемкнути монітор входу
trader-toast-exit-monitor-failed = Не вдалося перемкнути монітор виходу
trader-toast-load-failed = Помилка завантаження
    .message = Не вдалося завантажити конфігурацію трейдера
trader-toast-saved = Конфігурацію збережено
    .message = Налаштування трейдера успішно застосовано
trader-toast-save-failed = Помилка збереження
    .message = Не вдалося зберегти конфігурацію трейдера
trader-toast-feature-enabled = Функцію увімкнено
trader-toast-feature-disabled = Функцію вимкнено
trader-toast-feature-applied = Налаштування автотрейдера застосовано
trader-toast-strategy-enabled = Стратегію увімкнено
    .message = Стратегія активна
trader-toast-strategy-disabled = Стратегію вимкнено
    .message = Стратегія неактивна
trader-toast-strategy-failed = Помилка оновлення
    .message = Не вдалося оновити статус стратегії

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Період статистики
trader-stats-window-day = 24 год
trader-stats-window-week = 7 д
trader-stats-window-month = 30 д
trader-realized-title = Реалізована результативність
trader-metric-net-pnl = Чистий прибуток/збиток
trader-metric-win-rate = Відсоток виграшних угод
trader-metric-profit-factor = Профіт-фактор
trader-metric-max-drawdown = Макс. просадка
trader-metric-capital = Капітал у роботі
trader-metric-avg-win-loss = Сер. виграш / програш
trader-metric-closed-trades = Закриті угоди
trader-metric-median-hold = Медіанне утримання
trader-stats-empty = У цьому періоді немає закритих угод
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = виграно { $won } · програно { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } виграш
        [few] { $amount } виграші
        [many] { $amount } виграшів
       *[other] { $amount } виграшу
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } програш
        [few] { $amount } програші
        [many] { $amount } програшів
       *[other] { $amount } програшу
    }
# $amount is a formatted SOL amount.
trader-stats-expected = очікується { $amount } на угоду
trader-stats-profit-factor-basis = Загальний виграш ÷ загальний програш
trader-stats-drawdown-basis = Найглибша реалізована просадка від піку до мінімуму
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [one] Використано слотів позицій: { $used } із { $max }
        [few] Використано слотів позицій: { $used } із { $max }
        [many] Використано слотів позицій: { $used } із { $max }
       *[other] Використано слотів позицій: { $used } із { $max }
    }
trader-stats-avg-basis = Середній результат виграшної та програшної угоди
trader-stats-closed =
    { $count ->
        [one] Закрито позицій: { $amount }
        [few] Закрито позицій: { $amount }
        [many] Закрито позицій: { $amount }
       *[other] Закрито позицій: { $amount }
    }
# $span is a formatted duration.
trader-stats-hold-average = у середньому { $span }
trader-stats-excluded =
    { $count ->
        [one] Виключено закритих раундів: { $amount } — немає повної собівартості, тож немає достовірного прибутку/збитку.
        [few] Виключено закритих раундів: { $amount } — немає повної собівартості, тож немає достовірного прибутку/збитку.
        [many] Виключено закритих раундів: { $amount } — немає повної собівартості, тож немає достовірного прибутку/збитку.
       *[other] Виключено закритих раундів: { $amount } — немає повної собівартості, тож немає достовірного прибутку/збитку.
    }

## Stats: daily P&L and extremes

trader-daily-title = Щоденний прибуток/збиток
trader-daily-subtitle = Реалізований { -sol } за день із наростаючим підсумком
trader-daily-loading = Завантаження щоденного прибутку/збитку...
trader-daily-chart = Щоденний реалізований прибуток і збиток у { -sol }
trader-extreme-best = Найкраща угода
trader-extreme-worst = Найгірша угода

## Stats: exit breakdown

trader-exit-title = Розподіл стратегій виходу
trader-exit-subtitle = Як закривалися позиції та що повернув кожен вихід
trader-exit-loading = Завантаження даних про виходи...
trader-exit-empty-day = За останні 24 години немає закритих угод
trader-exit-empty-days =
    { $count ->
        [one] За останні { $amount } д немає закритих угод
        [few] За останні { $amount } д немає закритих угод
        [many] За останні { $amount } д немає закритих угод
       *[other] За останні { $amount } д немає закритих угод
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [one] { $amount } угода · { $share } виходів
        [few] { $amount } угоди · { $share } виходів
        [many] { $amount } угод · { $share } виходів
       *[other] { $amount } угоди · { $share } виходів
    }
# $value is a formatted average percentage.
trader-exit-average = сер. { $value }

## Shared example vocabulary

trader-impact-label = Вплив:
trader-current-label = Зараз:
trader-readable-label = Читабельно:
trader-example-how-it-works = Як це працює
trader-step-entry = Вхід
trader-step-initial-position = Початкова позиція
trader-step-auto-exit = Автовихід
trader-step-exit = Вихід
trader-step-full-exit = Повний вихід із позиції
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% прибутку

## Stop loss

trader-stop-loss-title = Стоп-лос
trader-stop-loss-subtitle = Автоматичний вихід із позиції, коли збиток перевищує ваш поріг
# $threshold is the threshold as typed.
trader-stop-loss-impact = Вихід, коли ціна впала на { $threshold }% від входу
trader-stop-loss-hold-immediate = Негайно
# $span is a formatted duration.
trader-stop-loss-hold-delay = затримка { $span }
trader-stop-loss-price-falls = Ціна падає
trader-stop-loss-threshold-reached = Поріг досягнуто
trader-stop-loss-partial = Дозволено часткові виходи
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = Збиток обмежено до <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Примітка:</strong> стоп-лос захищає від більших збитків завдяки раннім виходам

## Trailing stop

trader-trailing-title = Трейлінг-стоп
trader-trailing-subtitle = Автоматичний захист прибутку: стоп рухається за ціною, коли вона зростає
# $value is the activation percentage as typed.
trader-trailing-activation-impact = Трейлінг починається при прибутку +{ $value }%
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = Вихід на -{ $value }% від піку
trader-trailing-activation = Активація
trader-trailing-peak = Пік
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% підсумок
# $value is a formatted percentage.
trader-trailing-summary-protected = Захищено прибуток <strong>{ $value }</strong>
# $value is a formatted percentage.
trader-trailing-summary-avoided = Уникнуто збитку <strong>{ $value }</strong> від піку

## Take profit

trader-roi-title = Тейк-профіт
trader-roi-subtitle = Автоматичний вихід з усієї позиції, коли прибуток досягає вашої цілі
# $target is the target percentage as typed.
trader-roi-impact = Вихід при прибутку +{ $target }%
trader-roi-example-title = Приклад сценарію
trader-roi-initial-buy = Початкова купівля
trader-roi-target-hit = Ціль досягнуто
trader-roi-full-position = Уся позиція
trader-roi-sold = 100% продано
# $target is the target percentage as typed.
trader-roi-summary = Зафіксовано прибуток <strong>+{ $target }%</strong>

## Time-based exit

trader-time-title = Вихід за часом
trader-time-subtitle = Автоматичний вихід із позицій після максимального часу утримання, якщо збиток перевищує поріг
trader-time-unit-seconds = с
trader-time-unit-minutes = хв
trader-time-unit-hours = год
trader-time-unit-days = д
# Shown before the configured duration loads.
trader-time-conversion-default = 168 год = 7 д
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } с
        [few] { $amount } с
        [many] { $amount } с
       *[other] { $amount } с
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } хв
        [few] { $amount } хв
        [many] { $amount } хв
       *[other] { $amount } хв
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } год
        [few] { $amount } год
        [many] { $amount } год
       *[other] { $amount } год
    }
trader-duration-days =
    { $count ->
        [one] { $amount } д
        [few] { $amount } д
        [many] { $amount } д
       *[other] { $amount } д
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = Вихід, якщо збиток { $value }% або більше після періоду утримання
# $day is the day number of the example.
trader-time-day = День { $day }
trader-time-position-opened = Позицію відкрито
trader-time-limit = Ліміт часу
trader-time-hold-reached = Період утримання досягнуто
trader-time-loss-met = Поріг збитку досягнуто
trader-time-note = <strong>Примітка:</strong> позиції з прибутком або меншим збитком НЕ буде закрито
trader-time-positions-title = Стан поточних позицій
trader-time-positions-loading = Завантаження позицій...
trader-time-positions-empty = Немає відкритих позицій
trader-time-positions-hold = Час утримання:
trader-time-positions-roi = ROI:

## Strategy control

trader-strategy-entry-title = Стратегії входу
trader-strategy-entry-subtitle = Сигнали, що можуть відкрити нову позицію.
trader-strategy-exit-title = Стратегії виходу
trader-strategy-exit-subtitle = Сигнали, що можуть закрити або захистити відкриту позицію.
trader-strategy-active-unknown = -- активні
trader-strategy-active = Активні: { $enabled }/{ $total }
trader-strategy-loading = Завантаження стратегій...
trader-strategy-load-failed = Не вдалося завантажити стратегії
trader-strategy-empty = Стратегій не визначено
trader-strategy-no-description = Опису немає.
trader-strategy-unnamed = Стратегія без назви
trader-strategy-priority-auto = Авто
trader-strategy-priority = Пріоритет { $priority }

## Dollar-cost averaging

trader-dca-title = Усереднення (DCA)
trader-dca-subtitle = Автоматична докупівля до збиткових позицій, щоб знизити середню ціну входу
trader-dca-example-title = Приклад DCA
trader-dca-example = 0,01 { -sol } початково → DCA №1: 0,005 { -sol } при -10% → DCA №2: 0,005 { -sol } ще при -10%
trader-dca-info-title = Про стратегію DCA
trader-dca-info-subtitle = Важливі міркування щодо торгівлі з DCA
trader-dca-how-title = Як працює DCA
trader-dca-how-trigger = <strong>Тригер:</strong> позиція падає нижче порога DCA (напр., -10%)
trader-dca-how-action = <strong>Дія:</strong> докупити ще { -sol }, щоб знизити середню собівартість
trader-dca-how-repeat = <strong>Повтор:</strong> можна робити DCA кілька разів залежно від максимальної кількості
trader-dca-risk-title = Попередження про ризики
trader-dca-risk-exposure = <strong>Більша експозиція:</strong> DCA збільшує загальний капітал під ризиком на одну позицію
trader-dca-risk-knife = <strong>Падаючий ніж:</strong> DCA не допоможе, якщо токен і далі падає
trader-dca-risk-cooldown = <strong>Пауза:</strong> використовуйте паузу, щоб уникнути швидких послідовних входів DCA

## General settings

trader-sizing-title = Розмір позиції
trader-sizing-subtitle = Керуйте тим, скільки інвестувати в одну позицію
trader-timing-title = Час і паузи
trader-timing-subtitle = Керуйте часом між операціями
trader-timing-close-cooldown = Пауза після закриття позиції
trader-timing-close-cooldown-hint = Скільки хвилин чекати перед повторним відкриттям того самого токена
trader-timing-concurrency = Паралельність перевірки входу
trader-timing-concurrency-hint = Кількість токенів для одночасної перевірки (більше = швидше, але більше навантаження на ЦП)
trader-timing-unit-minutes = хв
trader-timing-unit-tokens = токенів
