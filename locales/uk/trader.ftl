# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Стоп-лос
trader-exit-type-take-profit = Тейк-профіт
trader-exit-type-roi = Тейк-профіт
trader-exit-type-roi-exit = Тейк-профіт
trader-exit-type-trailing-stop = Трейлінг-стоп
trader-exit-type-time-override = Вихід за часом
trader-exit-type-time-rule = Часове правило
trader-exit-type-manual = Вручну
trader-exit-type-manual-close = Вручну
trader-exit-type-dca = DCA
trader-exit-type-unknown = Невідомо

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Статистика
trader-tab-strategies = Стратегії
trader-tab-stop-loss = Стоп-лос
trader-tab-trailing-stop = Трейлінг-стоп
trader-tab-take-profit = Тейк-профіт
trader-tab-time-rules = Вихід за часом
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
trader-setup-action = Завершити налаштування
trader-toggle-on = УВІМКНЕНО
trader-toggle-off = ВИМКНЕНО
trader-toggle-start-failed = Не вдалося запустити трейдер
trader-toggle-stop-failed = Не вдалося зупинити трейдер
trader-controls-title = Керування торгівлею
trader-halt-title = Торгівлю зупинено
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
trader-force-stop-hint = Одразу зупиняє всі монітори, завдання копіювання та угоди.
trader-force-stop-idle = Зараз не працює нічого, що зупинила б примусова зупинка.

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

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Період статистики
trader-stats-book =
    .aria-label = Торговий портфель
trader-paper-run-since = Віртуальний прогін з { $date }
trader-paper-start = Старт
trader-paper-cash = Вільно
trader-paper-open = У позиціях
trader-paper-unrealized = Нереалізований
trader-paper-reset = Скинути віртуальний прогін
trader-paper-reset-title = Скинути віртуальний прогін?
trader-paper-reset-run-message = Кожна відкрита віртуальна позиція, зокрема віртуальні утримувані токени завдань копіювання, закриється за поточною ціною, і почнеться новий прогін із заданим стартовим балансом. Попередні прогони залишаться в історії.
trader-paper-reset-done = Почався новий віртуальний прогін
trader-paper-reset-failed = Не вдалося скинути віртуальний прогін
trader-paper-account-failed = Не вдалося завантажити віртуальний прогін
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
trader-weekly-title = Щотижневий прибуток/збиток
trader-weekly-subtitle = Реалізований { -sol } за тиждень ISO із наростаючим підсумком
trader-weekly-chart = Щотижневий реалізований прибуток і збиток у { -sol }
trader-extreme-best = Найкраща угода
trader-extreme-worst = Найгірша угода

## Stats: exit breakdown

trader-exit-title = Розподіл стратегій виходу
trader-exit-subtitle = Як закривалися позиції та що повернув кожен вихід
trader-exit-loading = Завантаження даних про виходи...
trader-exit-empty-day = За останні 24 години немає закритих угод
trader-exit-empty-all = Закритих угод поки немає
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
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = Продати { $part }
trader-stop-loss-part-of-position = Частина позиції
trader-stop-loss-sell-rest = Продати решту
trader-stop-loss-still-below = Досі нижче за стоп
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
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Зафіксований мінімум під час спрацювання: { $value }
trader-trailing-locked-minimum-warning = Під час активації стоп опиняється на ціні входу або нижче; він продає лише в прибутку. Увімкніть стоп у беззбиток, щоб мати нижню межу.
trader-trailing-activation = Активація
trader-trailing-peak = Пік
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% підсумок
# $value is a formatted percentage.
trader-trailing-summary-protected = Захищено прибуток <strong>{ $value }</strong>
# $value is a formatted percentage.
trader-trailing-summary-avoided = Уникнуто збитку <strong>{ $value }</strong> від піку
# $step is the band's position in the list, from 1.
trader-trailing-band = Смуга { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Піковий прибуток смуги { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Відступ смуги { $step }
trader-trailing-band-profit-caption = З пікового прибутку
trader-trailing-band-distance-caption = Відступ
trader-trailing-add-band = Додати смугу
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Видалити смугу { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = Піковий прибуток кожної смуги має бути від { $min }% до { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = Відступ кожної смуги має бути від { $min }% до { $max }%
trader-trailing-band-error-order = Піковий прибуток кожної смуги має бути вищим за активацію та попередню смугу
trader-trailing-band-error-distance = Відступ кожної смуги має бути меншим за її піковий прибуток

## Break-even stop

trader-break-even-title = Стоп у беззбиток
trader-break-even-subtitle = Коли позиція вийшла в прибуток, продати все, доки вона не стала збитковою
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Щойно найкраща ціна з останньої покупки сягне { $trigger }, повернення до { $offset } продасть усе.

## Take profit

trader-take-profit-title = Тейк-профіт
trader-take-profit-subtitle = Продає позицію сходинками, коли прибуток досягає кожного рівня
trader-take-profit-initial-buy = Початкова купівля
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Сходинка { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Рівень прибутку сходинки { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Частка продажу сходинки { $step }
trader-take-profit-profit-caption = Рівень прибутку
trader-take-profit-sell-caption = Частка продажу
trader-take-profit-add-step = Додати сходинку
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Видалити сходинку { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Кожен рівень прибутку має бути від { $min }% до { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Кожна частка має бути від { $min }% до { $max }%
trader-take-profit-error-order = Кожна сходинка потребує вищого рівня прибутку, ніж попередня
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Усі сходинки разом можуть продати не більше { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% позиції залишається відкритими як ранер
trader-take-profit-runner-none = Остання сходинка продає залишок позиції
trader-take-profit-runner-warning = Коли трейлінг-стоп, стоп у беззбиток, обидва виходи за часом і всі стратегії виходу вимкнені, ранер закривається лише стоп-лосом або вручну
trader-take-profit-info = Частки рахуються від початкової позиції. Якщо ціна проходить кілька рівнів одразу, за одну перевірку продається одна сходинка.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = При +{ $profit }% продати { $share }%
trader-take-profit-example-runner = Ранер
trader-take-profit-example-runner-detail = Залишається іншим правилам виходу
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Продає <strong>{ $sold }%</strong> до +{ $profit }% прибутку

## Time-based exit

trader-time-loss-title = Закривати збиткові позиції
trader-time-subtitle = Автоматичний вихід із позицій після максимального часу утримання, якщо збиток перевищує поріг
trader-time-flat-title = Закривати позиції без руху
trader-time-flat-subtitle = Звільняє капітал із позицій, які не ростуть і не падають
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Позиція, утримувана { $span } або довше з P&L у межах ±{ $band }, закривається.
trader-time-unit-seconds = с
trader-time-unit-minutes = хв
trader-time-unit-hours = год
trader-time-unit-days = д
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
trader-time-loss-hint = Збиток, за якого вихід за часом продає після спливу часу утримання (40 = вихід при -40%)
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
trader-time-positions-empty-message = Тут відображаються відкриті позиції з часом утримання та ROI відносно ліміту часу.
trader-time-positions-token = Токен
trader-time-positions-hold = Час утримання
trader-time-positions-roi = ROI
trader-time-positions-rule = Вихід за часом
trader-time-status-off = Виходи за часом вимкнені
trader-time-status-none = Зараз не діє жоден вихід за часом
trader-time-status-not-managed = Не керується автотрейдером
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = У збитку { $pnl }; закриється через { $remaining }
trader-time-status-losing-due = У збитку { $pnl }; закриється під час наступної перевірки
# $held and $remaining are formatted durations.
trader-time-status-flat = Без руху { $held }; закриється через { $remaining }
trader-time-status-flat-due = Без руху { $held }; закриється під час наступної перевірки

## Dollar-cost averaging

trader-dca-title = Усереднення (DCA)
trader-dca-subtitle = Автоматична докупівля до збиткових позицій, щоб знизити середню ціну входу
trader-dca-measure-average = Середній вхід
trader-dca-measure-first-entry = Перший вхід
trader-dca-step = Докупівля { $step }
trader-dca-drop-label = Падіння ціни для докупівлі { $step }
trader-dca-size-label = Розмір докупівлі { $step }, кратно першій покупці
trader-dca-drop-caption = Падіння ціни
trader-dca-size-caption = Покупка × перша покупка
trader-dca-add-step = Додати рівень
trader-dca-remove-step = Видалити докупівлю { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Менше за мінімальну угоду { $minimum }: ця докупівля не надсилається
trader-dca-error-drop-range = Кожне падіння має бути від { $min }% до { $max }%
trader-dca-error-size-range = Кожен розмір має бути від { $min }× до { $max }× першої покупки
trader-dca-error-order = Якщо вимірювати від першого входу, кожній докупівлі потрібне глибше падіння, ніж попередній
trader-dca-cooldown-impact = Щонайменше { $span } між додатковими входами
trader-dca-min-hold-none = Перша докупівля може відбутися одразу після входу
trader-dca-min-hold-impact = Перша докупівля чекає щонайменше { $span } після входу
trader-dca-capacity = Найбільше, що може тримати ця позиція: { $amount } ({ $multiple }× перша покупка)
trader-dca-info-title = Про стратегію DCA
trader-dca-info-subtitle = Важливі міркування щодо торгівлі з DCA
trader-dca-how-title = Як працює DCA
trader-dca-how-trigger = <strong>Тригер:</strong> ціна опускається нижче наступного рівня, рахуючи від середнього або першого входу
trader-dca-how-action = <strong>Дія:</strong> докупити ще { -sol }, щоб знизити середню собівартість
trader-dca-how-repeat = <strong>Драбина:</strong> кожна докупівля переходить до наступного рівня; коли всі рівні використано, докупівлі припиняються
trader-dca-how-pause = <strong>Паузи:</strong> жодної докупівлі, поки попередня ще проводиться або поки ліміт збитків блокує нові входи
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
