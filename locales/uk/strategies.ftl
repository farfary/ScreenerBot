# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = Усі
strategies-filter-entry = Входу
strategies-filter-exit = Виходу
strategies-type-entry = Входу
strategies-type-exit = Виходу
strategies-list-empty-title = Стратегій ще немає
strategies-list-empty-hint = Створіть свою першу стратегію
strategies-new = Нова стратегія
strategies-import =
    .title = Імпортувати стратегію
    .aria-label = Імпортувати стратегію
strategies-item-enable =
    .title = Увімкнути
strategies-item-disable =
    .title = Вимкнути

# Name given to a strategy before it is saved.
strategies-new-name = Нова стратегія

## Editor

strategies-editor-name =
    .placeholder = Назва стратегії
strategies-editor-dirty =
    .title = Незбережені зміни
strategies-editor-enabled =
    .aria-label = Стратегію ввімкнено
    .title = Стратегію ввімкнено
strategies-action-validate = Перевірити
strategies-editor-empty = Виберіть стратегію для редагування або створіть нову
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
strategies-summary-period-seconds = Період огляду: { $amount } с
strategies-summary-period-minutes = Період огляду: { $amount } хв
strategies-summary-period-hours = Період огляду: { $amount } год

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

## Condition catalog

strategies-catalog-search =
    .placeholder = Пошук умов...
strategies-catalog-search-clear =
    .aria-label = Очистити пошук
strategies-catalog-fold-all = Згорнути все
strategies-catalog-unfold-all = Розгорнути все
strategies-catalog-no-description = Опис недоступний

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
strategies-toast-enabled = Стратегію увімкнено
    .message = «{ $name }» увімкнено
strategies-toast-disabled = Стратегію вимкнено
    .message = «{ $name }» вимкнено
strategies-toast-toggle-failed = Не вдалося перемкнути
    .message = Не вдалося оновити статус стратегії
strategies-toast-load-failed = Помилка завантаження
    .message = Не вдалося завантажити стратегії із сервера
strategies-toast-load-strategy-failed = Не вдалося завантажити стратегію
strategies-toast-no-strategy = Стратегію не створено
    .message = Спершу додайте хоча б одну умову або натисніть «Нова стратегія», щоб створити стратегію
strategies-toast-no-conditions-save = Немає умов
    .message = Додайте до стратегії хоча б одну умову перед збереженням
strategies-toast-name-required = Потрібна назва
    .message = Введіть назву стратегії перед збереженням
strategies-toast-saved = Стратегію збережено
    .message = «{ $name }» успішно збережено
strategies-toast-save-failed = Помилка збереження
    .message = Не вдалося зберегти стратегію в базу даних
strategies-toast-no-strategy-validate = Немає стратегії для перевірки
strategies-toast-no-conditions-validate = Немає умов
    .message = Додайте хоча б одну умову перед перевіркою
strategies-toast-valid = Стратегія дійсна
strategies-toast-invalid = Стратегія має помилки
strategies-toast-validation-failed = Помилка перевірки
strategies-toast-item-enabled = Стратегію увімкнено
strategies-toast-item-disabled = Стратегію вимкнено
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
strategies-toast-condition-added = Умову додано
    .message = { $name } додано до стратегії


## Conditions

strategies-condition-candle-size = Шаблон розміру свічки
    .description = Виявлення певних свічкових шаблонів: велике тіло, мале тіло (доджі), довгі тіні
strategies-condition-candle-size-param-pattern = Тип шаблону
    .description = Свічковий шаблон для виявлення
strategies-condition-candle-size-param-pattern-option-large-body = Велике тіло (сильний рух)
strategies-condition-candle-size-param-pattern-option-small-body = Мале тіло (доджі/невизначеність)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Довга верхня тінь (відторгнення)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Довга нижня тінь (підтримка)
strategies-condition-candle-size-param-threshold = Поріг розміру %
    .description = Відсотковий поріг для виявлення шаблону

strategies-condition-consecutive-candles = Послідовні свічки
    .description = Виявлення послідовних зелених (бичачих) або червоних (ведмежих) свічок із фільтром мінімального розміру
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
    .description = Перевірка, як довго утримується позиція (для стратегій виходу — виходи за часом)
strategies-condition-position-holding-time-param-hours = Часовий поріг (години)
    .description = Тривалість у годинах з моменту відкриття позиції
strategies-condition-position-holding-time-param-comparison = Порівняння
    .description = Як порівнювати вік позиції з порогом
strategies-condition-position-holding-time-param-comparison-option-greater-than = Старша за (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Щонайменше (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Молодша за ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Щонайбільше (≤)

strategies-condition-price-breakout = Пробій ціни
    .description = Виявлення пробою ціни вище опору (максимум періоду) або нижче підтримки (мінімум періоду)
strategies-condition-price-breakout-param-lookback = Період огляду
    .description = Кількість свічок для визначення рівня підтримки/опору
strategies-condition-price-breakout-param-direction = Напрямок пробою
    .description = Напрямок пробою
strategies-condition-price-breakout-param-direction-option-upward = Вгору (пробій опору)
strategies-condition-price-breakout-param-direction-option-downward = Вниз (пробій підтримки)
strategies-condition-price-breakout-param-confirmation = Підтвердження %
    .description = Наскільки далеко за рівень має вийти ціна для підтвердження пробою (уникає хибних сигналів)

strategies-condition-price-change-percent = Зміна ціни %
    .description = Перевірка, чи змінилася ціна на відсотковий поріг за певний період
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
    .description = Перевірка, чи ціна вище, нижче або в межах діапазону від її простої ковзної середньої
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
    .description = Виявлення сплесків обсягу порівняно із середнім обсягом (вказує на зростання інтересу)
strategies-condition-volume-spike-param-lookback = Період огляду
    .description = Кількість свічок для обчислення середнього обсягу
strategies-condition-volume-spike-param-multiplier = Множник обсягу
    .description = У скільки разів вище середнього (напр., 2,0 = 200% від середнього)

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
strategies-error-invalid-rule-tree = Недійсне дерево правил: { $reason }
strategies-error-evaluation-timeout = Час оцінювання стратегії вичерпано через { $timeout } мс
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

# Indicators

strategies-error-indicator-consecutive-candles = послідовні свічки
strategies-error-indicator-moving-average = ковзна середня
strategies-error-indicator-price-breakout = пробій ціни
strategies-error-indicator-price-change-lookback = огляд зміни ціни
strategies-error-indicator-volume-spike = сплеск обсягу

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = У вузла гілки немає умов
strategies-error-rule-branch-node-missing-operator = У вузла гілки немає оператора
strategies-error-rule-branch-node-must-have-at-least-one-child = Вузол гілки має мати щонайменше один дочірній елемент
strategies-error-rule-invalid-rule-tree-structure = Недійсна структура дерева правил
strategies-error-rule-leaf-node-missing-condition = У листового вузла немає умови
strategies-error-rule-not-operator-must-have-exactly-one-child = Оператор NOT має мати рівно один дочірній елемент
