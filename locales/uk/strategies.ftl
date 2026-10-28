# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-type-entry = Входу
strategies-type-exit = Виходу
strategies-list-empty-title = Стратегій ще немає
strategies-list-empty-hint = Створіть свою першу стратегію
strategies-new = Нова стратегія
strategies-import =
    .title = Імпортувати стратегію
    .aria-label = Імпортувати стратегію

# Name given to a strategy before it is saved.
strategies-new-name = Нова стратегія

## Editor

strategies-editor-name =
    .placeholder = Назва стратегії
strategies-editor-dirty =
    .title = Незбережені зміни
strategies-action-validate = Перевірити
strategies-conditions-empty-title = Умов ще немає
strategies-conditions-empty-hint = Натисніть «{ strategies-add-condition }», щоб почати
strategies-add-condition = Додати умову
strategies-modal-close =
    .aria-label = Закрити
strategies-card-move-up =
    .title = Перемістити вгору
strategies-card-move-down =
    .title = Перемістити вниз
strategies-card-duplicate =
    .title = Дублювати
strategies-card-delete =
    .title = Видалити
# $name is the condition name.
strategies-card-delete-confirm = Вилучити умову
    .message = Вилучити «{ $name }» з цієї стратегії?

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = Без параметрів
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Налаштування стратегії ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } с
strategies-value-duration-minutes = { $amount } хв
strategies-value-duration-hours = { $amount } год
strategies-value-duration-days = { $amount } д
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Ваш місцевий час: { $time }
strategies-hint-price-impact = Угода такого розміру зсуває ціну приблизно на { $amount }%

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } год
        [few] { $amount } год
        [many] { $amount } год
       *[other] { $amount } год
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } свічка
        [few] { $amount } свічки
        [many] { $amount } свічок
       *[other] { $amount } свічки
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = год
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = Пошук умов...
    .aria-label = Пошук умов
strategies-catalog-search-clear =
    .aria-label = Очистити пошук
strategies-catalog-no-description = Опис недоступний

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Увімк.
strategies-editor-timeframe = Таймфрейм за замовчуванням
    .title = Таймфрейм свічок для умов, які не задають власного
strategies-editor-order = Порядок
    .title = Порядок обчислення від 1 до 100; менші числа обчислюються першими
strategies-editor-description = Опис
strategies-editor-description-field =
    .placeholder = Що робить ця стратегія
    .aria-label = Опис стратегії
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Усі стратегії входу
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Керує
strategies-editor-entry-binding-hint = Керує позиціями, відкритими позначеними стратегіями входу. Якщо нічого не позначено, керує всіма позиціями, зокрема ручними та скопійованими.
strategies-editor-entry-binding-empty = Стратегій входу ще немає
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (видалена)
strategies-action-export = Експорт
strategies-action-test = Перевірити на токені
# $token is the tested token's symbol or address.
strategies-test-title = Перевірка на { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Перевірка на { $token }…
strategies-test-verdict-pass = Виконано
strategies-test-verdict-fail = Не виконано
strategies-test-verdict-unknown = Невідомо

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Умови, що мають виконуватися: { $mode }
strategies-group-match = Мають виконуватися: { $mode }
strategies-match-all = усі
strategies-match-any = принаймні одна
strategies-match-mode =
    .aria-label = Усі умови або принаймні одна
strategies-invert = Інвертувати
    .title = Інвертувати: істинно, коли це не виконується
strategies-chip-off = Вимк.
strategies-chip-inverted = Інвертовано
strategies-row-switch =
    .title = Вимкніть, щоб зберегти, але не обчислювати
    .aria-label = Обчислюється цією стратегією
strategies-row-expand =
    .aria-label = Показати або сховати параметри
strategies-add-group = Додати групу
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Вилучити групу
    .message =
        { $count ->
            [one] Вилучити цю групу та її умову?
            [few] Вилучити цю групу та її { $count } умови?
            [many] Вилучити цю групу та її { $count } умов?
           *[other] Вилучити цю групу та її { $count } умови?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] Виправте { $count } проблему: { $first }
        [few] Виправте { $count } проблеми: { $first }
        [many] Виправте { $count } проблем: { $first }
       *[other] Виправте { $count } проблеми: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Додайте принаймні одну ввімкнену умову
strategies-problem-group-empty = У групі має бути принаймні одна ввімкнена умова
strategies-problem-not-applicable = Цю умову не можна використовувати в стратегії цього типу
strategies-problem-unknown-condition = Невідомий тип умови: { $type }
strategies-problem-too-deep = Правила можуть вкладатися не глибше ніж на { $max } рівнів; інвертована група займає ще один рівень
strategies-problem-required = Введіть число
strategies-problem-range = Значення має бути від { $min } до { $max }
strategies-problem-invalid-option = Виберіть один із запропонованих варіантів

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Усі
strategies-catalog-categories =
    .aria-label = Категорії
strategies-catalog-empty = Жодна умова не відповідає запиту
strategies-catalog-exit-only = Лише для стратегій виходу
strategies-catalog-entry-only = Лише для стратегій входу
strategies-catalog-candles-note = { $description } Читає лише свічки.
strategies-data-candles = Свічки
strategies-data-pool = Пул
strategies-data-position = Позиція
strategies-data-market = Ринкова активність
strategies-data-clock = Час доби

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = угод
strategies-value-trades =
    { $count ->
        [one] { $amount } угода
        [few] { $amount } угоди
        [many] { $amount } угод
       *[other] { $amount } угоди
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Забагато рівнів
    .message = Правила можуть вкладатися не глибше ніж на { $max } рівнів
strategies-toast-priority-invalid = Недійсний пріоритет
    .message = Введіть ціле число від 1 до 100
strategies-toast-exported = Стратегію експортовано
# $name is the strategy name.
strategies-unsaved-confirm = Відхилити незбережені зміни?
    .message = У «{ $name }» є незбережені зміни.
strategies-unsaved-discard = Відхилити зміни
strategies-import-error-json = Файл не є коректним JSON
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = Пріоритет у файлі має бути цілим числом від { $min } до { $max }
strategies-import-error-shape = Файл не описує стратегію
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Файл використовує умову, якої немає в цій версії: { $type }
strategies-import-error-not-applicable = Умову «{ $name }» не можна використовувати в стратегії цього типу

## New strategy dialog

strategies-create-title = Створити нову стратегію
strategies-create-prompt = Виберіть тип стратегії, яку хочете створити:
strategies-create-entry-name = Стратегія входу
strategies-create-entry-description = Визначте умови, за яких купувати токен
strategies-create-exit-name = Стратегія виходу
strategies-create-exit-description = Визначте умови, за яких продавати токен

## Delete dialog

strategies-delete-title = Видалити стратегію
# $name is the strategy name.
strategies-delete-message = Видалити стратегію «{ $name }»? Цю дію не можна скасувати.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Виправте помилки перевірки перед збереженням
strategies-toast-load-failed = Помилка завантаження
    .message = Не вдалося завантажити стратегії із сервера
strategies-toast-load-strategy-failed = Не вдалося завантажити стратегію
strategies-toast-no-strategy = Стратегію не створено
    .message = Спершу додайте хоча б одну умову або натисніть «Нова стратегія», щоб створити стратегію
strategies-toast-name-required = Потрібна назва
    .message = Введіть назву стратегії перед збереженням
strategies-toast-saved = Стратегію збережено
    .message = «{ $name }» успішно збережено
strategies-toast-save-failed = Помилка збереження
    .message = Не вдалося зберегти стратегію в базу даних
strategies-toast-no-strategy-validate = Немає стратегії для перевірки
strategies-toast-no-strategy-test = Немає стратегії для перевірки
strategies-toast-test-save-first = Збережіть стратегію перед перевіркою: перевірка читає збережені правила
strategies-toast-test-failed = Перевірка не вдалася
    .message = Не вдалося перевірити стратегію
strategies-toast-valid = Стратегія дійсна
strategies-toast-invalid = Стратегія має помилки
strategies-toast-validation-failed = Помилка перевірки
strategies-toast-item-toggle-failed = Не вдалося перемкнути стратегію
strategies-toast-deleted = Стратегію видалено
    .message = «{ $name }» успішно видалено
strategies-toast-delete-failed = Помилка видалення
    .message = Не вдалося видалити стратегію з бази даних
strategies-toast-imported = Стратегію імпортовано
strategies-toast-import-failed = Не вдалося імпортувати стратегію
strategies-toast-unknown-condition = Невідома умова
    .message = Тип умови не знайдено
strategies-toast-create-first = Спершу створіть стратегію
    .message = Натисніть «Нова стратегія», щоб створити стратегію перед додаванням умов

## Conditions

strategies-condition-candle-size = Шаблон розміру свічки
    .description = Виявлення певних свічкових шаблонів: велике тіло, мале тіло (доджі), довгі тіні.
strategies-condition-candle-size-param-pattern = Тип шаблону
    .description = Свічковий шаблон для виявлення
strategies-condition-candle-size-param-pattern-option-large-body = Велике тіло (сильний рух)
strategies-condition-candle-size-param-pattern-option-small-body = Мале тіло (доджі/невизначеність)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Довга верхня тінь (відторгнення)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Довга нижня тінь (підтримка)
strategies-condition-candle-size-param-threshold = Поріг розміру %
    .description = Відсотковий поріг для виявлення шаблону

strategies-condition-consecutive-candles = Послідовні свічки
    .description = Виявлення послідовних зелених (бичачих) або червоних (ведмежих) свічок із фільтром мінімального розміру.
strategies-condition-consecutive-candles-param-count = Кількість свічок
    .description = Потрібна кількість послідовних свічок
strategies-condition-consecutive-candles-param-direction = Напрямок свічок
    .description = Колір/напрямок послідовних свічок
strategies-condition-consecutive-candles-param-direction-option-green = Зелені (бичачі)
strategies-condition-consecutive-candles-param-direction-option-red = Червоні (ведмежі)
strategies-condition-consecutive-candles-param-minimum-change = Мінімальна зміна %
    .description = Мінімальна зміна у % для кожної свічки (відсіює шум)

strategies-condition-liquidity-level = Рівень ліквідності пулу
    .description = Перевірка ліквідності пулу в { -sol } (вхід: переконатися в достатній ліквідності, вихід: виявити відтік ліквідності)
strategies-condition-liquidity-level-param-threshold = Поріг ліквідності ({ -sol })
    .description = Рівень ліквідності пулу в { -sol }
strategies-condition-liquidity-level-param-comparison = Порівняння
    .description = Як порівнювати ліквідність пулу з порогом
strategies-condition-liquidity-level-param-comparison-option-greater-than = Більше (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Більше або дорівнює (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Менше ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Менше або дорівнює (≤)

strategies-condition-position-holding-time = Час утримання позиції
    .description = Перевірка, як довго утримується позиція
strategies-condition-position-holding-time-param-hours = Часовий поріг (години)
    .description = Тривалість у годинах з моменту відкриття позиції
strategies-condition-position-holding-time-param-comparison = Порівняння
    .description = Як порівнювати вік позиції з порогом
strategies-condition-position-holding-time-param-comparison-option-greater-than = Старша за (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Щонайменше (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Молодша за ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Щонайбільше (≤)

strategies-condition-price-breakout = Пробій ціни
    .description = Виявлення пробою ціни вище опору (максимум періоду) або нижче підтримки (мінімум періоду).
strategies-condition-price-breakout-param-lookback = Період огляду
    .description = Кількість свічок для визначення рівня підтримки/опору
strategies-condition-price-breakout-param-direction = Напрямок пробою
    .description = Напрямок пробою
strategies-condition-price-breakout-param-direction-option-upward = Вгору (пробій опору)
strategies-condition-price-breakout-param-direction-option-downward = Вниз (пробій підтримки)
strategies-condition-price-breakout-param-confirmation = Підтвердження %
    .description = Наскільки далеко за рівень має вийти ціна для підтвердження пробою (уникає хибних сигналів)

strategies-condition-price-change-percent = Зміна ціни %
    .description = Перевірка, чи змінилася ціна на відсотковий поріг за певний період.
strategies-condition-price-change-percent-param-percentage = Поріг зміни %
    .description = Відсоткова зміна ціни для спрацювання (0,1–1000%)
strategies-condition-price-change-percent-param-direction = Напрямок
    .description = Напрямок руху ціни
strategies-condition-price-change-percent-param-direction-option-above = Зростання (+%)
strategies-condition-price-change-percent-param-direction-option-below = Падіння (-%)
strategies-condition-price-change-percent-param-direction-option-within = У межах діапазону (±%)
strategies-condition-price-change-percent-param-time-value = Період огляду
    .description = Наскільки далеко назад вимірюється зміна, в одиниці, обраній у полі (1–3600 с, 1–1440 хв, 1–720 год)
strategies-condition-price-change-percent-param-time-unit = Одиниця часу
    .description = Одиниця часу для періоду огляду
strategies-condition-price-change-percent-param-time-unit-option-seconds = с
strategies-condition-price-change-percent-param-time-unit-option-minutes = хв
strategies-condition-price-change-percent-param-time-unit-option-hours = год

strategies-condition-price-to-ma = Ціна відносно ковзної середньої
    .description = Перевірка, чи ціна вище, нижче або в межах діапазону від її простої ковзної середньої.
strategies-condition-price-to-ma-param-period = Період MA
    .description = Кількість свічок для обчислення ковзної середньої
strategies-condition-price-to-ma-param-position = Положення
    .description = Положення ціни відносно MA
strategies-condition-price-to-ma-param-position-option-above = Вище MA
strategies-condition-price-to-ma-param-position-option-below = Нижче MA
strategies-condition-price-to-ma-param-position-option-within = У межах діапазону
strategies-condition-price-to-ma-param-distance = Відстань %
    .description = Мінімальна відстань від MA (для ВИЩЕ/НИЖЧЕ) або максимальний діапазон (для У МЕЖАХ)

strategies-condition-volume-spike = Сплеск обсягу
    .description = Виявлення сплесків обсягу порівняно із середнім обсягом (вказує на зростання інтересу).
strategies-condition-volume-spike-param-lookback = Період огляду
    .description = Кількість свічок для обчислення середнього обсягу
strategies-condition-volume-spike-param-multiplier = Множник обсягу
    .description = У скільки разів вище середнього (напр., 2,0 = 200% від середнього)

strategies-condition-rsi = RSI (індекс відносної сили)
    .description = Спрацьовує, коли RSI зі згладжуванням Вайлдера за закритими свічками вищий або нижчий за поріг.
strategies-condition-rsi-param-period = Період RSI
    .description = Кількість свічок, за якими обчислюється RSI; умові потрібна ще одна свічка понад це значення
strategies-condition-rsi-param-comparison = Порівняння
    .description = Чи має RSI бути нижчим, чи вищим за поріг
strategies-condition-rsi-param-comparison-option-below = Нижче (перепроданість)
strategies-condition-rsi-param-comparison-option-above = Вище (перекупленість)
strategies-condition-rsi-param-threshold = Поріг RSI
    .description = Рівень RSI від 1 до 99 (звично 30 для перепроданості та 70 для перекупленості)

strategies-condition-ma-cross = Перетин ковзних середніх
    .description = Спрацьовує, коли швидка ковзна середня перетнула повільну в останніх свічках.
strategies-condition-ma-cross-param-ma-type = Тип середньої
    .description = Експоненційні середні сильніше враховують нещодавні свічки, прості враховують усі свічки однаково
strategies-condition-ma-cross-param-ma-type-option-ema = Експоненційна (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Проста (SMA)
strategies-condition-ma-cross-param-fast-period = Швидкий період
    .description = Кількість свічок у швидкій середній
strategies-condition-ma-cross-param-slow-period = Повільний період
    .description = Кількість свічок у повільній середній; має бути більшою за швидкий період
strategies-condition-ma-cross-param-direction = Напрямок перетину
    .description = У який бік швидка середня перетинає повільну
strategies-condition-ma-cross-param-direction-option-bullish = Бичачий (швидка перетинає знизу вгору)
strategies-condition-ma-cross-param-direction-option-bearish = Ведмежий (швидка перетинає згори вниз)
strategies-condition-ma-cross-param-within-candles = Перетин за останні свічки
    .description = Скільки останніх свічок може містити перетин; 1 означає лише останню свічку

strategies-condition-position-profit = Прибуток позиції
    .description = Спрацьовує, коли нереалізований прибуток відкритої позиції вищий або нижчий за заданий відсоток від ціни входу, за ціною пулу.
strategies-condition-position-profit-param-comparison = Порівняння
    .description = Чи має прибуток бути не меншим, чи не більшим за поріг
strategies-condition-position-profit-param-comparison-option-above = Щонайменше (≥)
strategies-condition-position-profit-param-comparison-option-below = Щонайбільше (≤)
strategies-condition-position-profit-param-threshold-pct = Поріг прибутку, %
    .description = Нереалізований прибуток у відсотках від ціни входу; від'ємні значення означають збиток

strategies-condition-drawdown-from-peak = Просадка від піку
    .description = Спрацьовує, коли ціна пулу впала на заданий відсоток від найвищої ціни після останньої покупки, якщо цей пік був достатньо вищим за ціну входу.
strategies-condition-drawdown-from-peak-param-drop-pct = Просадка від піку, %
    .description = Наскільки поточна ціна має бути нижчою за пік
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Вмикати після прибутку, %
    .description = Пік мав перевищити ціну входу щонайменше на цей відсоток, перш ніж умова зможе спрацювати; 0 означає, що вона діє завжди

strategies-condition-buy-pressure = Тиск покупок
    .description = Спрацьовує, коли покупки становлять не менше заданої частки транзакцій за часове вікно, а у вікні достатньо транзакцій. Читає кількість покупок і продажів токена.
strategies-condition-buy-pressure-param-window = Часове вікно
    .description = Період, який охоплюють кількості покупок і продажів
strategies-condition-buy-pressure-param-window-option-5m = 5 хвилин
strategies-condition-buy-pressure-param-window-option-1h = 1 година
strategies-condition-buy-pressure-param-window-option-6h = 6 годин
strategies-condition-buy-pressure-param-window-option-24h = 24 години
strategies-condition-buy-pressure-param-min-buy-share-pct = Мінімальна частка покупок, %
    .description = Покупки у відсотках від усіх транзакцій вікна
strategies-condition-buy-pressure-param-min-transactions = Мінімум транзакцій
    .description = Вікна з меншою кількістю транзакцій ніколи не запускають умову

strategies-condition-transaction-velocity = Швидкість транзакцій
    .description = Спрацьовує, коли темп транзакцій недавнього вікна не менший або не більший за кратне темпу довшого вікна, що його містить. Використовує кількість купівель і продажів токена.
strategies-condition-transaction-velocity-param-window = Вікна
    .description = Недавнє вікно і довше вікно, з яким воно порівнюється
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 хвилин і 1 година
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 година і 6 годин
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 годин і 24 години
strategies-condition-transaction-velocity-param-comparison = Порівняння
    .description = Чи має недавній темп бути не меншим або не більшим за відношення
strategies-condition-transaction-velocity-param-comparison-option-above = Не менше (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Не більше (≤)
strategies-condition-transaction-velocity-param-ratio = Відношення темпів
    .description = Транзакції за секунду недавнього вікна, поділені на транзакції за секунду довгого вікна
strategies-condition-transaction-velocity-param-min-transactions = Мінімум транзакцій
    .description = Недавні вікна з меншою кількістю транзакцій ніколи не запускають умову

strategies-condition-pullback-from-high = Відкат від максимуму
    .description = Спрацьовує, коли поточна ціна між мінімальним і максимальним падінням нижче найвищого максимуму останніх свічок.
strategies-condition-pullback-from-high-param-lookback = Період огляду
    .description = Кількість свічок, з яких береться найвищий максимум
strategies-condition-pullback-from-high-param-min-drop-pct = Мінімальне падіння %
    .description = Ціна має бути нижче максимуму щонайменше на стільки
strategies-condition-pullback-from-high-param-max-drop-pct = Максимальне падіння %
    .description = Ціна має бути нижче максимуму не більше ніж на стільки; має перевищувати мінімальне падіння

strategies-condition-profit-giveback = Віддача прибутку
    .description = Спрацьовує, коли позиція віддала частку свого найкращого нереалізованого прибутку від останньої купівлі після того, як цей прибуток досяг рівня.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Звести після прибутку %
    .description = Найкращий прибуток має досягти цього відсотка ціни входу, перш ніж умова зможе спрацювати
strategies-condition-profit-giveback-param-giveback-pct = Віддача %
    .description = Частка відданого найкращого прибутку; 100% означає, що ціна повернулася до ціни входу

strategies-condition-token-age = Вік токена
    .description = Спрацьовує, коли час від створення токена в блокчейні більший або менший за тривалість. Якщо час створення невідомий, умова лишається невизначеною.
strategies-condition-token-age-param-comparison = Порівняння
    .description = Чи має токен бути старшим або молодшим за тривалість
strategies-condition-token-age-param-comparison-option-older-than = Старший за (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Молодший за ({ "<" })
strategies-condition-token-age-param-time-value = Вік
    .description = Час від створення токена в одиниці, вибраній у полі (1-10000)
strategies-condition-token-age-param-time-unit = Одиниця часу
    .description = Одиниця віку
strategies-condition-token-age-param-time-unit-option-minutes = хв
strategies-condition-token-age-param-time-unit-option-hours = год
strategies-condition-token-age-param-time-unit-option-days = д

strategies-condition-vwap = VWAP
    .description = Спрацьовує, коли поточна ціна вища або нижча за середньозважену за обсягом ціну останніх свічок щонайменше на задану відстань.
strategies-condition-vwap-param-lookback = Період огляду
    .description = Кількість свічок, які охоплює середньозважена за обсягом ціна
strategies-condition-vwap-param-position = Положення
    .description = Положення ціни відносно VWAP
strategies-condition-vwap-param-position-option-above = Вище VWAP
strategies-condition-vwap-param-position-option-below = Нижче VWAP
strategies-condition-vwap-param-distance-pct = Відстань %
    .description = Мінімальна відстань від VWAP; 0 означає будь-яку відстань з вибраного боку

strategies-condition-volatility-range = Діапазон волатильності
    .description = Спрацьовує, коли середній істинний діапазон останніх свічок у відсотках від поточної ціни вищий або нижчий за поріг. Дві такі умови в групі задають діапазон.
strategies-condition-volatility-range-param-period = Період ATR
    .description = Кількість свічок для згладжування середнього істинного діапазону; умові потрібна ще одна свічка
strategies-condition-volatility-range-param-comparison = Порівняння
    .description = Чи має діапазон бути вищим або нижчим за поріг
strategies-condition-volatility-range-param-comparison-option-above = Вище
strategies-condition-volatility-range-param-comparison-option-below = Нижче
strategies-condition-volatility-range-param-threshold-pct = Поріг діапазону %
    .description = Середній істинний діапазон у відсотках від поточної ціни

strategies-condition-bollinger-bands = Смуги Боллінджера
    .description = Спрацьовує, коли поточна ціна нижча за нижню смугу або вища за верхню чи коли смуги стискаються. Смуги віддалені на кілька стандартних відхилень від середньої ціни закриття.
strategies-condition-bollinger-bands-param-period = Період
    .description = Кількість свічок для середньої і стандартного відхилення
strategies-condition-bollinger-bands-param-std-dev = Стандартні відхилення
    .description = На скільки стандартних відхилень кожна смуга віддалена від середньої
strategies-condition-bollinger-bands-param-signal = Сигнал
    .description = Що мають показувати смуги
strategies-condition-bollinger-bands-param-signal-option-below-lower = Нижче нижньої смуги
strategies-condition-bollinger-bands-param-signal-option-above-upper = Вище верхньої смуги
strategies-condition-bollinger-bands-param-signal-option-squeeze = Стиснення
strategies-condition-bollinger-bands-param-squeeze-width-pct = Ширина стиснення %
    .description = Для стиснення: найбільша відстань між смугами у відсотках від середньої

strategies-condition-liquidity-to-size = Ліквідність до розміру
    .description = Спрацьовує, коли резерви пулу не менші або менші за кратне розміру на кону: розміру угоди у стратегії входу, залишкової вартості позиції у стратегії виходу.
strategies-condition-liquidity-to-size-param-comparison = Порівняння
    .description = Чи мають резерви бути не меншими або меншими за кратне
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Не менше (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Менше ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Кратність ліквідності
    .description = Резерви пулу, поділені на розмір на кону; кратність N зсуває ціну приблизно на 1/(N+1)

strategies-condition-position-adds = Докупівлі позиції
    .description = Спрацьовує, коли кількість докупівель після першого входу або часткових продажів не менша або не більша за задану.
strategies-condition-position-adds-param-subject = Що рахувати
    .description = Які дії рахуються
strategies-condition-position-adds-param-subject-option-adds = Докупівлі
strategies-condition-position-adds-param-subject-option-partial-sells = Часткові продажі
strategies-condition-position-adds-param-comparison = Порівняння
    .description = Чи має кількість бути не меншою або не більшою за задану
strategies-condition-position-adds-param-comparison-option-at-least = Не менше (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Не більше (≤)
strategies-condition-position-adds-param-count = Кількість
    .description = Кількість дій

strategies-condition-time-of-day = Час доби
    .description = Спрацьовує, коли поточна година потрапляє у вікно годин UTC. Початок після закінчення переходить через північ.
strategies-condition-time-of-day-param-start-hour = Година початку (UTC)
    .description = Перша година вікна, від 0 до 23
strategies-condition-time-of-day-param-end-hour = Година закінчення (UTC)
    .description = Година, якою закінчується вікно, від 1 до 24; сама ця година поза вікном

strategies-condition-liquidity-change-since-entry = Зміна ліквідності від входу
    .description = Спрацьовує, коли резерви пулу впали або зросли щонайменше на відсоток від відкриття позиції. Без записаної ліквідності під час входу умова лишається невизначеною.
strategies-condition-liquidity-change-since-entry-param-comparison = Порівняння
    .description = Чи мають резерви впасти або зрости
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Впала на
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Зросла на
strategies-condition-liquidity-change-since-entry-param-change-pct = Зміна %
    .description = Зміна резервів пулу від входу

## Shared by every condition

strategies-condition-param-timeframe = Таймфрейм
    .description = Розмір свічки: тривалість кожної свічки, яку читає умова (таймфрейм стратегії, якщо не задано)
strategies-condition-timeframe-option-1m = 1 хв
strategies-condition-timeframe-option-5m = 5 хв
strategies-condition-timeframe-option-15m = 15 хв
strategies-condition-timeframe-option-1h = 1 год
strategies-condition-timeframe-option-4h = 4 год
strategies-condition-timeframe-option-12h = 12 год
strategies-condition-timeframe-option-1d = 1 д

## Condition categories

strategies-condition-category-price-analysis = Аналіз ціни
strategies-condition-category-candle-patterns = Свічкові шаблони
strategies-condition-category-technical-indicators = Технічні індикатори
strategies-condition-category-market-context = Ринковий контекст
strategies-condition-category-position-performance = Позиція та результативність
strategies-condition-category-volume-analysis = Аналіз обсягу

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = Бракує параметра «{ $field }»
strategies-error-parameter-type = Параметр «{ $field }» має бути таким: { $expected }
strategies-error-invalid-value = «{ $value }» — недійсне значення для «{ $field }»
strategies-error-missing-data = Недоступно: { $data }
strategies-error-no-candle-data = Для таймфрейму { $timeframe } немає даних свічок
strategies-error-insufficient-history = Недостатньо історії для «{ $indicator }»: доступно { $available } с, потрібно { $required } с
strategies-error-insufficient-candles = Недостатньо свічок для «{ $indicator }»: є { $available }, потрібно { $required }
strategies-error-stale-candle-data = Дані свічок для таймфрейму { $timeframe } застарілі: їхній вік { $age } с перевищує { $max } с
strategies-error-stale-market-data = Кількість транзакцій за { $window } застаріла: її вік { $age } с перевищує { $max } с
strategies-error-unusable-data = Значення «{ $value }» для «{ $field }» непридатне
strategies-error-invalid-rule-tree = Недійсне дерево правил: { $reason }
strategies-error-evaluation-timeout = Час оцінювання стратегії вичерпано через { $timeout } мс
strategies-error-condition-not-applicable = { $condition } не можна використовувати в стратегіях { $strategy }
strategies-error-invalid-rules = Не вдалося прочитати правила: { $reason }

# Parameter names

strategies-error-field-average-volume = середній обсяг
strategies-error-field-candle-open = відкриття свічки
strategies-error-field-comparison = порівняння
strategies-error-field-condition-type = тип умови
strategies-error-field-confirmation = підтвердження
strategies-error-field-count = кількість
strategies-error-field-current-price = поточна ціна
strategies-error-field-direction = напрямок
strategies-error-field-distance = відстань
strategies-error-field-hours = години
strategies-error-field-lookback = глибина огляду
strategies-error-field-minimum-change = мінімальна зміна
strategies-error-field-multiplier = множник
strategies-error-field-pattern = шаблон
strategies-error-field-percentage = відсоток
strategies-error-field-period = період
strategies-error-field-position = положення
strategies-error-field-threshold = поріг
strategies-error-field-time-unit = одиниця часу
strategies-error-field-time-value = значення часу
strategies-error-field-timeframe = таймфрейм
strategies-error-field-arm-after-profit = вмикання після прибутку
strategies-error-field-drop-percentage = відсоток просадки
strategies-error-field-entry-price = ціна входу
strategies-error-field-fast-period = швидкий період
strategies-error-field-minimum-buy-share = мінімальна частка покупок
strategies-error-field-minimum-transactions = мінімум транзакцій
strategies-error-field-moving-average-type = тип середньої
strategies-error-field-moving-average = ковзна середня
strategies-error-field-reference-candle-close = закриття опорної свічки
strategies-error-field-peak-price = ціна піку
strategies-error-field-slow-period = повільний період
strategies-error-field-window = вікно
strategies-error-field-within-candles = кількість свічок перетину
strategies-error-field-change = зміна
strategies-error-field-end-hour = година закінчення
strategies-error-field-entry-liquidity = ліквідність під час входу
strategies-error-field-giveback = віддача
strategies-error-field-highest-high = максимум
strategies-error-field-long-window-transactions = транзакції довгого вікна
strategies-error-field-maximum-drop = максимальне падіння
strategies-error-field-minimum-drop = мінімальне падіння
strategies-error-field-multiple = кратність
strategies-error-field-pool-liquidity = ліквідність пулу
strategies-error-field-ratio = відношення
strategies-error-field-remaining-position-value = залишкова вартість позиції
strategies-error-field-signal = сигнал
strategies-error-field-squeeze-width = ширина стиснення
strategies-error-field-standard-deviations = стандартні відхилення
strategies-error-field-start-hour = година початку
strategies-error-field-subject = що рахувати
strategies-error-field-token-creation-time = час створення токена
strategies-error-field-total-volume = загальний обсяг
strategies-error-field-trade-size = розмір угоди
strategies-error-field-vwap = VWAP

# Expected parameter types

strategies-error-expected-boolean = логічне значення
strategies-error-expected-number = число
strategies-error-expected-string = рядок

# Missing context data

strategies-error-data-current-price = Поточна ціна
strategies-error-data-liquidity-data = Дані ліквідності
strategies-error-data-market-data = Ринкові дані
strategies-error-data-ohlcv-data = Дані OHLCV
strategies-error-data-position-data = Дані позиції
strategies-error-data-peak-price = Ціна піку
strategies-error-data-transaction-counts = Кількість транзакцій
strategies-error-data-entry-liquidity = Ліквідність під час входу
strategies-error-data-entry-pool-liquidity = Ліквідність пулу входу
strategies-error-data-real-pool-reserves = Реальна ліквідність пулу
strategies-error-data-remaining-position-value = Залишкова вартість позиції
strategies-error-data-token-creation-time = Час створення токена
strategies-error-data-trade-size = Розмір угоди

# Indicators

strategies-error-indicator-consecutive-candles = послідовні свічки
strategies-error-indicator-moving-average = ковзна середня
strategies-error-indicator-price-breakout = пробій ціни
strategies-error-indicator-price-change-lookback = огляд зміни ціни
strategies-error-indicator-volume-spike = сплеск обсягу
strategies-error-indicator-moving-average-cross = перетин ковзних середніх
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = смуги Боллінджера
strategies-error-indicator-pullback-from-high = відкат від максимуму
strategies-error-indicator-volatility-range = діапазон волатильності
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = У вузла гілки немає умов
strategies-error-rule-branch-node-missing-operator = У вузла гілки немає оператора
strategies-error-rule-branch-node-must-have-at-least-one-child = Вузол гілки має мати щонайменше один дочірній елемент
strategies-error-rule-invalid-rule-tree-structure = Недійсна структура дерева правил
strategies-error-rule-leaf-node-missing-condition = У листового вузла немає умови
strategies-error-rule-not-operator-must-have-exactly-one-child = Оператор NOT має мати рівно один дочірній елемент
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Групі потрібна принаймні одна ввімкнена умова
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = Оператор NOT не може мати вимкнений дочірній вузол
strategies-error-rule-rules-nest-deeper-than-four-levels = Правила можуть бути вкладені не глибше ніж на чотири рівні
strategies-error-rule-the-root-group-cannot-be-switched-off = Кореневу групу не можна вимкнути

strategies-readiness-history-since-met =
    { $count ->
        [one] { $count } закритий віртуальний раунд від зміни правил { $changed }, потрібно { $needed }
        [few] { $count } закриті віртуальні раунди від зміни правил { $changed }, потрібно { $needed }
        [many] { $count } закритих віртуальних раундів від зміни правил { $changed }, потрібно { $needed }
       *[other] { $count } закритого віртуального раунду від зміни правил { $changed }, потрібно { $needed }
    }
strategies-readiness-history-since-short = Закритих віртуальних раундів від зміни правил { $changed }: { $count } із { $needed }
strategies-readiness-priced = Кожна позиція має ціну
strategies-readiness-priced-ok = Кожна відкрита віртуальна позиція має ціну пулу
strategies-readiness-priced-missing =
    { $count ->
        [one] { $count } відкрита віртуальна позиція без ціни пулу
        [few] { $count } відкриті віртуальні позиції без ціни пулу
        [many] { $count } відкритих віртуальних позицій без ціни пулу
       *[other] { $count } відкритої віртуальної позиції без ціни пулу
    }
strategies-readiness-runtime = Реальна торгівля доступна
strategies-readiness-runtime-ok = Налаштування й запобіжники дозволяють реальні угоди

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = Усі стратегії
strategies-view-kind = Тип стратегії
strategies-view-all = Усі
strategies-search-placeholder = Пошук стратегій...
strategies-unnamed = Стратегія без назви
strategies-col-on = Увімк.
strategies-col-strategy = Стратегія
strategies-col-rank = Ранг
    .title = Ранг обчислення серед увімкнених стратегій входу одного режиму, спершу менший порядок; перша із сигналом забирає токен
strategies-col-mode = Режим
strategies-col-status = Статус
strategies-col-open = Відкрито
strategies-col-watching = Під наглядом
strategies-col-kind = Тип
strategies-col-rounds = Раунди
strategies-col-closed = Закрито
strategies-col-win-rate = Частка прибуткових
strategies-col-realized = Реалізовано ({ -sol })
strategies-col-manages = Керує
strategies-col-last-signal = Останній сигнал
strategies-status-running = Працює
strategies-status-not-running = Не працює
strategies-status-off = Вимкнена
strategies-status-setup-required = Потрібне налаштування
strategies-status-live-paused = Реальні входи призупинено
strategies-status-paper-cash = Бракує віртуальних коштів
strategies-status-slots-full = Усі слоти зайняті
strategies-status-entry-monitor-off = Монітор входів вимкнено
strategies-status-exit-monitor-off = Монітор виходів вимкнено
strategies-notice-setup-required = Віртуальна торгівля почнеться після налаштування
strategies-notice-force-stopped = Увімкнено аварійну зупинку
strategies-notice-trader-off = Auto Trader вимкнено
strategies-notice-waiting = Очікування основних сервісів
strategies-notice-monitors-off = Монітори входів і виходів вимкнено
strategies-notice-start = Запустити Auto Trader
# $closed and $needed count closed paper rounds; $check is the title of a readiness check.
strategies-readiness-progress = Раунди: { $closed }/{ $needed }
strategies-readiness-ready = Готова до реальної торгівлі
strategies-readiness-failing = Не виконано: { $check }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
        [one] { $book }: { $count } позиція
        [few] { $book }: { $count } позиції
        [many] { $book }: { $count } позицій
       *[other] { $book }: { $count } позиції
    }
strategies-manages-all = Усі стратегії входу
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
        [one] { $first } і ще { $more }
        [few] { $first } і ще { $more }
        [many] { $first } і ще { $more }
       *[other] { $first } і ще { $more }
    }
strategies-exit-before = Перевіряються першими
strategies-exit-hint = Ці правила виходу перевіряються для кожної позиції раніше за будь-яку стратегію виходу
strategies-action-edit = Змінити { $name }
strategies-action-delete = Видалити { $name }
strategies-roster-stale = Останнє оновлення

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = Змінити правила реальної стратегії «{ $name }»
    .message = Нові правила торгують реальними { -sol } з наступної перевірки, а віртуальне підтвердження цієї стратегії починається заново з цього збереження.
strategies-live-rules-save = Зберегти й торгувати реально
strategies-toast-evidence-restarted = Стратегію збережено
    .message = Правила «{ $name }» змінилися; її віртуальне підтвердження починається заново

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = Активувати реальну торгівлю
strategies-mode-return-paper = Повернути у віртуальний режим
strategies-mode-arm-title = Активувати реальну торгівлю для «{ $name }»
strategies-mode-arm-lead = Нові входи «{ $name }» здійснюватимуться реальними свопами з вашого торгового гаманця.
strategies-mode-arm-lead-exit = За своїм сигналом «{ $name }» продаватиме реальні позиції справжніми свопами з вашого торгового гаманця.
strategies-mode-arm-readiness = Готовність за віртуальним портфелем
strategies-mode-arm-open-paper = Відкриті віртуальні позиції залишаються віртуальними й закриваються віртуально.
strategies-mode-arm-ack-real-native = Нові входи витрачають реальні { -sol } з вашого торгового гаманця.
strategies-mode-arm-ack-real-native-exit = Реальні позиції під її керуванням продаються справжніми свопами.
strategies-mode-arm-ack-fees = Реальні свопи сплачують справжні мережеві комісії та проковзування; віртуальні результати не гарантують реальних.
strategies-mode-arm-ack-unready = Деякі перевірки готовності не пройдено. Усе одно активувати цю стратегію.
strategies-mode-arm-unavailable = Реальна торгівля зараз недоступна; див. останню перевірку.
strategies-mode-return-title = Повернути «{ $name }» у віртуальний режим
strategies-mode-return-message = Нові входи відкриваються у віртуальному портфелі без витрати { -sol }. Відкриті реальні позиції залишаються реальними й закриваються реально.
strategies-mode-return-message-exit = Вона перестає оцінювати реальні позиції; у них лишаються їхні правила виходу.
strategies-mode-readiness-failed = Не вдалося отримати готовність стратегії
strategies-mode-toast-armed = { $name } тепер торгує реально
strategies-mode-toast-paper = { $name } тепер торгує віртуально
strategies-mode-toast-failed = Не вдалося змінити режим торгівлі
