strategies-type-entry = Вход
strategies-type-exit = Выход
strategies-list-empty-title = Стратегий пока нет
strategies-list-empty-hint = Создайте свою первую стратегию
strategies-new = Новая стратегия
strategies-import =
    .title = Импортировать стратегию
    .aria-label = Импортировать стратегию

strategies-new-name = Новая стратегия

strategies-editor-name =
    .placeholder = Название стратегии
strategies-editor-dirty =
    .title = Есть несохранённые изменения
strategies-action-validate = Проверить
strategies-conditions-empty-title = Условий пока нет
strategies-conditions-empty-hint = Нажмите «{ strategies-add-condition }», чтобы начать
strategies-add-condition = Добавить условие
strategies-modal-close =
    .aria-label = Закрыть
strategies-card-move-up =
    .title = Переместить вверх
strategies-card-move-down =
    .title = Переместить вниз
strategies-card-duplicate =
    .title = Дублировать
strategies-card-delete =
    .title = Удалить
# $name is the condition name.
strategies-card-delete-confirm = Удалить условие
    .message = Удалить «{ $name }» из этой стратегии?

strategies-summary-param = { $label }: { $value }
strategies-summary-none = Нет параметров
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Настройка стратегии ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } с
strategies-value-duration-minutes = { $amount } мин
strategies-value-duration-hours = { $amount } ч
strategies-value-duration-days = { $amount } д
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Ваше местное время: { $time }
strategies-hint-price-impact = Сделка такого размера сдвигает цену примерно на { $amount }%

strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } час
        [few] { $amount } часа
        [many] { $amount } часов
       *[other] { $amount } часа
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } свеча
        [few] { $amount } свечи
        [many] { $amount } свечей
       *[other] { $amount } свечи
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = ч
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

strategies-catalog-search =
    .placeholder = Поиск условий...
    .aria-label = Поиск условий
strategies-catalog-search-clear =
    .aria-label = Очистить поиск
strategies-catalog-no-description = Описание недоступно

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Вкл.
strategies-editor-timeframe = Таймфрейм по умолчанию
    .title = Таймфрейм свечей для условий, у которых нет своего
strategies-editor-order = Порядок
    .title = Порядок вычисления от 1 до 100; меньшие числа вычисляются первыми
strategies-editor-description = Описание
strategies-editor-description-field =
    .placeholder = Что делает эта стратегия
    .aria-label = Описание стратегии
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Все стратегии входа
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Управляет
strategies-editor-entry-binding-hint = Управляет позициями, открытыми отмеченными стратегиями входа. Если ничего не отмечено, управляет всеми позициями, включая ручные и скопированные.
strategies-editor-entry-binding-empty = Стратегий входа пока нет
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (удалена)
strategies-action-export = Экспорт
strategies-action-test = Проверить на токене
# $token is the tested token's symbol or address.
strategies-test-title = Проверка на { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Проверка на { $token }…
strategies-test-verdict-pass = Выполнено
strategies-test-verdict-fail = Не выполнено
strategies-test-verdict-unknown = Неизвестно

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Условия должны выполняться: { $mode }
strategies-group-match = Должны выполняться: { $mode }
strategies-match-all = все
strategies-match-any = хотя бы одно
strategies-match-mode =
    .aria-label = Все условия или хотя бы одно
strategies-invert = Инвертировать
    .title = Инвертировать: истинно, когда это не выполняется
strategies-chip-off = Выкл.
strategies-chip-inverted = Инвертировано
strategies-row-switch =
    .title = Выключите, чтобы сохранить, но не вычислять
    .aria-label = Вычисляется этой стратегией
strategies-row-expand =
    .aria-label = Показать или скрыть параметры
strategies-add-group = Добавить группу
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Удалить группу
    .message =
        { $count ->
            [one] Удалить эту группу и её условие?
            [few] Удалить эту группу и её { $count } условия?
            [many] Удалить эту группу и её { $count } условий?
           *[other] Удалить эту группу и её { $count } условия?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] Исправьте { $count } проблему: { $first }
        [few] Исправьте { $count } проблемы: { $first }
        [many] Исправьте { $count } проблем: { $first }
       *[other] Исправьте { $count } проблемы: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Добавьте хотя бы одно включённое условие
strategies-problem-group-empty = В группе должно быть хотя бы одно включённое условие
strategies-problem-not-applicable = Это условие нельзя использовать в стратегии этого типа
strategies-problem-unknown-condition = Неизвестный тип условия: { $type }
strategies-problem-too-deep = Правила могут вкладываться не глубже { $max } уровней; инвертированная группа занимает ещё один уровень
strategies-problem-required = Введите число
strategies-problem-range = Значение должно быть от { $min } до { $max }
strategies-problem-invalid-option = Выберите один из предложенных вариантов

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Все
strategies-catalog-categories =
    .aria-label = Категории
strategies-catalog-empty = Ни одно условие не соответствует запросу
strategies-catalog-exit-only = Только для стратегий выхода
strategies-catalog-entry-only = Только для стратегий входа
strategies-catalog-candles-note = { $description } Читает только свечи.
strategies-data-candles = Свечи
strategies-data-pool = Пул
strategies-data-position = Позиция
strategies-data-market = Рыночная активность
strategies-data-clock = Время суток

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = сделок
strategies-value-trades =
    { $count ->
        [one] { $amount } сделка
        [few] { $amount } сделки
        [many] { $amount } сделок
       *[other] { $amount } сделки
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Слишком много уровней
    .message = Правила могут вкладываться не глубже { $max } уровней
strategies-toast-priority-invalid = Недопустимый приоритет
    .message = Введите целое число от 1 до 100
strategies-toast-exported = Стратегия экспортирована
# $name is the strategy name.
strategies-unsaved-confirm = Отменить несохранённые изменения?
    .message = В «{ $name }» есть несохранённые изменения.
strategies-unsaved-discard = Отменить изменения
strategies-import-error-json = Файл не является корректным JSON
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = Приоритет в файле должен быть целым числом от { $min } до { $max }
strategies-import-error-shape = Файл не описывает стратегию
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Файл использует условие, которого нет в этой версии: { $type }
strategies-import-error-not-applicable = Условие «{ $name }» нельзя использовать в стратегии этого типа

strategies-create-title = Создать новую стратегию
strategies-create-prompt = Выберите тип стратегии, которую хотите создать:
strategies-create-entry-name = Стратегия входа
strategies-create-entry-description = Задайте условия, при которых нужно купить токен
strategies-create-exit-name = Стратегия выхода
strategies-create-exit-description = Задайте условия, при которых нужно продать токен

strategies-delete-title = Удалить стратегию
strategies-delete-message = Удалить стратегию «{ $name }»? Это действие нельзя отменить.

strategies-toast-fix-validation = Исправьте ошибки проверки перед сохранением
strategies-toast-load-failed = Ошибка загрузки
    .message = Не удалось загрузить стратегии с сервера
strategies-toast-load-strategy-failed = Не удалось загрузить стратегию
strategies-toast-no-strategy = Стратегия не создана
    .message = Добавьте хотя бы одно условие или нажмите «Новая стратегия», чтобы сначала создать стратегию
strategies-toast-name-required = Нужно название
    .message = Перед сохранением введите название стратегии
strategies-toast-saved = Стратегия сохранена
    .message = «{ $name }» успешно сохранена
strategies-toast-save-failed = Ошибка сохранения
    .message = Не удалось сохранить стратегию в базе данных
strategies-toast-no-strategy-validate = Нет стратегии для проверки
strategies-toast-no-strategy-test = Нет стратегии для проверки
strategies-toast-test-save-first = Сохраните стратегию перед проверкой: проверка читает сохранённые правила
strategies-toast-test-failed = Проверка не удалась
    .message = Не удалось проверить стратегию
strategies-toast-valid = Стратегия корректна
strategies-toast-invalid = В стратегии есть ошибки
strategies-toast-validation-failed = Ошибка проверки
strategies-toast-item-toggle-failed = Не удалось переключить стратегию
strategies-toast-deleted = Стратегия удалена
    .message = «{ $name }» успешно удалена
strategies-toast-delete-failed = Ошибка удаления
    .message = Не удалось удалить стратегию из базы данных
strategies-toast-imported = Стратегия импортирована
strategies-toast-import-failed = Не удалось импортировать стратегию
strategies-toast-unknown-condition = Неизвестное условие
    .message = Тип условия не найден
strategies-toast-create-first = Сначала создайте стратегию
    .message = Нажмите «Новая стратегия», чтобы создать стратегию перед добавлением условий

strategies-condition-candle-size = Паттерн размера свечи
    .description = Обнаруживает определённые паттерны свечей: большое тело, маленькое тело (доджи), длинные тени.
strategies-condition-candle-size-param-pattern = Тип паттерна
    .description = Паттерн свечи, который нужно обнаружить
strategies-condition-candle-size-param-pattern-option-large-body = Большое тело (сильное движение)
strategies-condition-candle-size-param-pattern-option-small-body = Маленькое тело (доджи/неопределённость)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Длинная верхняя тень (отторжение)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Длинная нижняя тень (поддержка)
strategies-condition-candle-size-param-threshold = Порог размера, %
    .description = Процентный порог для обнаружения паттерна

strategies-condition-consecutive-candles = Свечи подряд
    .description = Обнаруживает подряд идущие зелёные (бычьи) или красные (медвежьи) свечи с фильтром по минимальному размеру.
strategies-condition-consecutive-candles-param-count = Количество свечей
    .description = Сколько свечей подряд требуется
strategies-condition-consecutive-candles-param-direction = Направление свечей
    .description = Цвет/направление свечей подряд
strategies-condition-consecutive-candles-param-direction-option-green = Зелёные (бычьи)
strategies-condition-consecutive-candles-param-direction-option-red = Красные (медвежьи)
strategies-condition-consecutive-candles-param-minimum-change = Минимальное изменение, %
    .description = Минимальное изменение каждой свечи в % (отсекает шум)

strategies-condition-liquidity-level = Уровень ликвидности пула
    .description = Проверяет ликвидность пула в { -sol } (вход: убедиться в достаточной ликвидности, выход: обнаружить отток ликвидности)
strategies-condition-liquidity-level-param-threshold = Порог ликвидности ({ -sol })
    .description = Уровень ликвидности пула в { -sol }
strategies-condition-liquidity-level-param-comparison = Сравнение
    .description = Как сравнивать ликвидность пула с порогом
strategies-condition-liquidity-level-param-comparison-option-greater-than = Больше (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Больше или равно (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Меньше ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Меньше или равно (≤)

strategies-condition-position-holding-time = Время удержания позиции
    .description = Проверяет, как долго удерживается позиция
strategies-condition-position-holding-time-param-hours = Порог времени (часы)
    .description = Длительность в часах с момента открытия позиции
strategies-condition-position-holding-time-param-comparison = Сравнение
    .description = Как сравнивать возраст позиции с порогом
strategies-condition-position-holding-time-param-comparison-option-greater-than = Старше (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Не моложе (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Моложе ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Не старше (≤)

strategies-condition-price-breakout = Пробой цены
    .description = Обнаруживает пробой цены выше сопротивления (максимум периода) или ниже поддержки (минимум периода).
strategies-condition-price-breakout-param-lookback = Период просмотра
    .description = Количество свечей для определения уровня поддержки/сопротивления
strategies-condition-price-breakout-param-direction = Направление пробоя
    .description = Направление пробоя
strategies-condition-price-breakout-param-direction-option-upward = Вверх (пробой сопротивления)
strategies-condition-price-breakout-param-direction-option-downward = Вниз (пробой поддержки)
strategies-condition-price-breakout-param-confirmation = Подтверждение, %
    .description = На сколько цена должна выйти за уровень, чтобы подтвердить пробой (защита от ложных сигналов)

strategies-condition-price-change-percent = Изменение цены, %
    .description = Проверяет, изменилась ли цена на заданный процент за период времени.
strategies-condition-price-change-percent-param-percentage = Порог изменения, %
    .description = Процентное изменение цены для срабатывания (0.1–1000%)
strategies-condition-price-change-percent-param-direction = Направление
    .description = Направление движения цены
strategies-condition-price-change-percent-param-direction-option-above = Рост (+%)
strategies-condition-price-change-percent-param-direction-option-below = Падение (-%)
strategies-condition-price-change-percent-param-direction-option-within = В диапазоне (±%)
strategies-condition-price-change-percent-param-time-value = Период просмотра
    .description = Насколько далеко назад измеряется изменение, в единице, выбранной в поле (1–3600 с, 1–1440 мин, 1–720 ч)
strategies-condition-price-change-percent-param-time-unit = Единица времени
    .description = Единица времени для периода просмотра
strategies-condition-price-change-percent-param-time-unit-option-seconds = с
strategies-condition-price-change-percent-param-time-unit-option-minutes = мин
strategies-condition-price-change-percent-param-time-unit-option-hours = ч

strategies-condition-price-to-ma = Цена и скользящая средняя
    .description = Проверяет, находится ли цена выше, ниже или в диапазоне простой скользящей средней (SMA).
strategies-condition-price-to-ma-param-period = Период MA
    .description = Количество свечей для расчёта скользящей средней
strategies-condition-price-to-ma-param-position = Положение
    .description = Положение цены относительно MA
strategies-condition-price-to-ma-param-position-option-above = Выше MA
strategies-condition-price-to-ma-param-position-option-below = Ниже MA
strategies-condition-price-to-ma-param-position-option-within = В диапазоне
strategies-condition-price-to-ma-param-distance = Расстояние, %
    .description = Минимальное расстояние от MA (для ВЫШЕ/НИЖЕ) или максимальный диапазон (для В ДИАПАЗОНЕ)

strategies-condition-volume-spike = Всплеск объёма
    .description = Обнаруживает всплески объёма относительно среднего (признак растущего интереса).
strategies-condition-volume-spike-param-lookback = Период просмотра
    .description = Количество свечей для расчёта среднего объёма
strategies-condition-volume-spike-param-multiplier = Множитель объёма
    .description = Во сколько раз выше среднего (например, 2.0 = 200% от среднего)

strategies-condition-rsi = RSI (индекс относительной силы)
    .description = Срабатывает, когда RSI со сглаживанием Уайлдера по закрытым свечам выше или ниже порога.
strategies-condition-rsi-param-period = Период RSI
    .description = Число свечей, по которым считается RSI; условию нужна на одну свечу больше этого значения
strategies-condition-rsi-param-comparison = Сравнение
    .description = Должен ли RSI быть ниже или выше порога
strategies-condition-rsi-param-comparison-option-below = Ниже (перепроданность)
strategies-condition-rsi-param-comparison-option-above = Выше (перекупленность)
strategies-condition-rsi-param-threshold = Порог RSI
    .description = Уровень RSI от 1 до 99 (обычно 30 для перепроданности и 70 для перекупленности)

strategies-condition-ma-cross = Пересечение скользящих средних
    .description = Срабатывает, когда быстрая скользящая средняя пересекла медленную в последних свечах.
strategies-condition-ma-cross-param-ma-type = Тип средней
    .description = Экспоненциальные средние сильнее учитывают недавние свечи, простые учитывают все свечи одинаково
strategies-condition-ma-cross-param-ma-type-option-ema = Экспоненциальная (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Простая (SMA)
strategies-condition-ma-cross-param-fast-period = Быстрый период
    .description = Число свечей в быстрой средней
strategies-condition-ma-cross-param-slow-period = Медленный период
    .description = Число свечей в медленной средней; должно быть больше быстрого периода
strategies-condition-ma-cross-param-direction = Направление пересечения
    .description = В какую сторону быстрая средняя пересекает медленную
strategies-condition-ma-cross-param-direction-option-bullish = Бычье (быстрая пересекает снизу вверх)
strategies-condition-ma-cross-param-direction-option-bearish = Медвежье (быстрая пересекает сверху вниз)
strategies-condition-ma-cross-param-within-candles = Пересечение за последние свечи
    .description = Сколько последних свечей может содержать пересечение; 1 означает только последнюю свечу

strategies-condition-position-profit = Прибыль позиции
    .description = Срабатывает, когда нереализованная прибыль открытой позиции выше или ниже заданного процента от цены входа, по цене пула.
strategies-condition-position-profit-param-comparison = Сравнение
    .description = Должна ли прибыль быть не меньше или не больше порога
strategies-condition-position-profit-param-comparison-option-above = Не меньше (≥)
strategies-condition-position-profit-param-comparison-option-below = Не больше (≤)
strategies-condition-position-profit-param-threshold-pct = Порог прибыли, %
    .description = Нереализованная прибыль в процентах от цены входа; отрицательные значения означают убыток

strategies-condition-drawdown-from-peak = Просадка от пика
    .description = Срабатывает, когда цена пула упала на заданный процент от максимальной цены после последней покупки, при условии что этот пик был достаточно выше цены входа.
strategies-condition-drawdown-from-peak-param-drop-pct = Просадка от пика, %
    .description = Насколько текущая цена должна быть ниже пика
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Включать после прибыли, %
    .description = Пик должен был превысить цену входа не меньше чем на этот процент, прежде чем условие сможет сработать; 0 означает, что оно действует всегда

strategies-condition-buy-pressure = Давление покупок
    .description = Срабатывает, когда покупки составляют не меньше заданной доли транзакций за временное окно и в окне достаточно транзакций. Читает число покупок и продаж токена.
strategies-condition-buy-pressure-param-window = Временное окно
    .description = Период, который охватывают числа покупок и продаж
strategies-condition-buy-pressure-param-window-option-5m = 5 минут
strategies-condition-buy-pressure-param-window-option-1h = 1 час
strategies-condition-buy-pressure-param-window-option-6h = 6 часов
strategies-condition-buy-pressure-param-window-option-24h = 24 часа
strategies-condition-buy-pressure-param-min-buy-share-pct = Минимальная доля покупок, %
    .description = Покупки в процентах от всех транзакций окна
strategies-condition-buy-pressure-param-min-transactions = Минимум транзакций
    .description = Окна с меньшим числом транзакций никогда не запускают условие

strategies-condition-transaction-velocity = Скорость транзакций
    .description = Срабатывает, когда темп транзакций недавнего окна не меньше или не больше кратного темпа более длинного окна, в которое оно входит. Использует число покупок и продаж токена.
strategies-condition-transaction-velocity-param-window = Окна
    .description = Недавнее окно и более длинное окно, с которым оно сравнивается
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 минут и 1 час
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 час и 6 часов
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 часов и 24 часа
strategies-condition-transaction-velocity-param-comparison = Сравнение
    .description = Должен ли недавний темп быть не меньше или не больше отношения
strategies-condition-transaction-velocity-param-comparison-option-above = Не меньше (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Не больше (≤)
strategies-condition-transaction-velocity-param-ratio = Отношение темпов
    .description = Транзакции в секунду недавнего окна, делённые на транзакции в секунду длинного окна
strategies-condition-transaction-velocity-param-min-transactions = Минимум транзакций
    .description = Недавние окна с меньшим числом транзакций никогда не запускают условие

strategies-condition-pullback-from-high = Откат от максимума
    .description = Срабатывает, когда текущая цена находится между минимальным и максимальным падением ниже наивысшего максимума последних свечей.
strategies-condition-pullback-from-high-param-lookback = Период просмотра
    .description = Количество свечей, из которых берётся наивысший максимум
strategies-condition-pullback-from-high-param-min-drop-pct = Минимальное падение %
    .description = Цена должна быть ниже максимума как минимум на столько
strategies-condition-pullback-from-high-param-max-drop-pct = Максимальное падение %
    .description = Цена должна быть ниже максимума не более чем на столько; должно быть больше минимального падения

strategies-condition-profit-giveback = Отдача прибыли
    .description = Срабатывает, когда позиция отдала долю своей лучшей нереализованной прибыли с последней покупки после того, как эта прибыль достигла уровня.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Взвести после прибыли %
    .description = Лучшая прибыль должна достичь этого процента от цены входа, прежде чем условие сможет сработать
strategies-condition-profit-giveback-param-giveback-pct = Отдача %
    .description = Доля отданной лучшей прибыли; 100% означает, что цена вернулась к цене входа

strategies-condition-token-age = Возраст токена
    .description = Срабатывает, когда время с момента создания токена в блокчейне больше или меньше заданной длительности. Если время создания неизвестно, условие остаётся неопределённым.
strategies-condition-token-age-param-comparison = Сравнение
    .description = Должен ли токен быть старше или моложе длительности
strategies-condition-token-age-param-comparison-option-older-than = Старше (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Моложе ({ "<" })
strategies-condition-token-age-param-time-value = Возраст
    .description = Время с создания токена в единице, выбранной в поле (1-10000)
strategies-condition-token-age-param-time-unit = Единица времени
    .description = Единица возраста
strategies-condition-token-age-param-time-unit-option-minutes = мин
strategies-condition-token-age-param-time-unit-option-hours = ч
strategies-condition-token-age-param-time-unit-option-days = д

strategies-condition-vwap = VWAP
    .description = Срабатывает, когда текущая цена выше или ниже средневзвешенной по объёму цены последних свечей как минимум на заданное расстояние.
strategies-condition-vwap-param-lookback = Период просмотра
    .description = Количество свечей, охватываемых средневзвешенной по объёму ценой
strategies-condition-vwap-param-position = Положение
    .description = Положение цены относительно VWAP
strategies-condition-vwap-param-position-option-above = Выше VWAP
strategies-condition-vwap-param-position-option-below = Ниже VWAP
strategies-condition-vwap-param-distance-pct = Расстояние %
    .description = Минимальное расстояние от VWAP; 0 означает любое расстояние с выбранной стороны

strategies-condition-volatility-range = Диапазон волатильности
    .description = Срабатывает, когда средний истинный диапазон последних свечей в процентах от текущей цены выше или ниже порога. Два таких условия в группе задают диапазон.
strategies-condition-volatility-range-param-period = Период ATR
    .description = Количество свечей для сглаживания среднего истинного диапазона; условию нужна ещё одна свеча
strategies-condition-volatility-range-param-comparison = Сравнение
    .description = Должен ли диапазон быть выше или ниже порога
strategies-condition-volatility-range-param-comparison-option-above = Выше
strategies-condition-volatility-range-param-comparison-option-below = Ниже
strategies-condition-volatility-range-param-threshold-pct = Порог диапазона %
    .description = Средний истинный диапазон в процентах от текущей цены

strategies-condition-bollinger-bands = Полосы Боллинджера
    .description = Срабатывает, когда текущая цена ниже нижней полосы или выше верхней либо когда полосы сжимаются. Полосы отстоят на несколько стандартных отклонений от средней цены закрытия.
strategies-condition-bollinger-bands-param-period = Период
    .description = Количество свечей для средней и стандартного отклонения
strategies-condition-bollinger-bands-param-std-dev = Стандартные отклонения
    .description = На сколько стандартных отклонений каждая полоса отстоит от средней
strategies-condition-bollinger-bands-param-signal = Сигнал
    .description = Что должны показывать полосы
strategies-condition-bollinger-bands-param-signal-option-below-lower = Ниже нижней полосы
strategies-condition-bollinger-bands-param-signal-option-above-upper = Выше верхней полосы
strategies-condition-bollinger-bands-param-signal-option-squeeze = Сжатие
strategies-condition-bollinger-bands-param-squeeze-width-pct = Ширина сжатия %
    .description = Для сжатия: наибольшее расстояние между полосами в процентах от средней

strategies-condition-liquidity-to-size = Ликвидность к размеру
    .description = Срабатывает, когда резервы пула не меньше или меньше кратного размера на кону: размера сделки в стратегии входа, оставшейся стоимости позиции в стратегии выхода.
strategies-condition-liquidity-to-size-param-comparison = Сравнение
    .description = Должны ли резервы быть не меньше или меньше кратного
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Не меньше (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Меньше ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Кратность ликвидности
    .description = Резервы пула, делённые на размер на кону; кратность N сдвигает цену примерно на 1/(N+1)

strategies-condition-position-adds = Докупки позиции
    .description = Срабатывает, когда число докупок после первого входа или частичных продаж не меньше или не больше заданного.
strategies-condition-position-adds-param-subject = Что считать
    .description = Какие действия считаются
strategies-condition-position-adds-param-subject-option-adds = Докупки
strategies-condition-position-adds-param-subject-option-partial-sells = Частичные продажи
strategies-condition-position-adds-param-comparison = Сравнение
    .description = Должно ли число быть не меньше или не больше заданного
strategies-condition-position-adds-param-comparison-option-at-least = Не меньше (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Не больше (≤)
strategies-condition-position-adds-param-count = Количество
    .description = Количество действий

strategies-condition-time-of-day = Время суток
    .description = Срабатывает, когда текущий час попадает в окно часов UTC. Начало позже окончания переходит через полночь.
strategies-condition-time-of-day-param-start-hour = Час начала (UTC)
    .description = Первый час окна, от 0 до 23
strategies-condition-time-of-day-param-end-hour = Час окончания (UTC)
    .description = Час, которым заканчивается окно, от 1 до 24; сам этот час вне окна

strategies-condition-liquidity-change-since-entry = Изменение ликвидности с входа
    .description = Срабатывает, когда резервы пула упали или выросли как минимум на процент с открытия позиции. Без записанной ликвидности при входе условие остаётся неопределённым.
strategies-condition-liquidity-change-since-entry-param-comparison = Сравнение
    .description = Должны ли резервы упасть или вырасти
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Упала на
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Выросла на
strategies-condition-liquidity-change-since-entry-param-change-pct = Изменение %
    .description = Изменение резервов пула с момента входа

strategies-condition-param-timeframe = Таймфрейм
    .description = Размер свечи: длительность каждой свечи, которую читает условие (таймфрейм стратегии, если не задан)
strategies-condition-timeframe-option-1m = 1 минута
strategies-condition-timeframe-option-5m = 5 минут
strategies-condition-timeframe-option-15m = 15 минут
strategies-condition-timeframe-option-1h = 1 час
strategies-condition-timeframe-option-4h = 4 часа
strategies-condition-timeframe-option-12h = 12 часов
strategies-condition-timeframe-option-1d = 1 день

strategies-condition-category-price-analysis = Анализ цены
strategies-condition-category-candle-patterns = Паттерны свечей
strategies-condition-category-technical-indicators = Технические индикаторы
strategies-condition-category-market-context = Рыночный контекст
strategies-condition-category-position-performance = Позиция и результат
strategies-condition-category-volume-analysis = Анализ объёма

strategies-error-missing-parameter = Отсутствует параметр: { $field }
strategies-error-parameter-type = Ожидаемый тип параметра { $field }: { $expected }
strategies-error-invalid-value = Недопустимое значение «{ $value }» для параметра: { $field }
strategies-error-missing-data = Данные недоступны: { $data }
strategies-error-no-candle-data = Для таймфрейма { $timeframe } нет данных свечей
strategies-error-insufficient-history = Недостаточно истории (индикатор: { $indicator }): доступно { $available } с, нужно { $required } с
strategies-error-insufficient-candles = Недостаточно свечей (индикатор: { $indicator }): есть { $available }, нужно { $required }
strategies-error-stale-candle-data = Данные свечей для таймфрейма { $timeframe } устарели: возраст { $age } с превышает { $max } с
strategies-error-stale-market-data = Число транзакций за { $window } устарело: возраст { $age } с превышает { $max } с
strategies-error-unusable-data = Значение «{ $value }» для параметра «{ $field }» непригодно
strategies-error-invalid-rule-tree = Недопустимое дерево правил: { $reason }
strategies-error-evaluation-timeout = Время оценки стратегии истекло через { $timeout } мс
strategies-error-condition-not-applicable = { $condition } нельзя использовать в стратегиях типа «{ $strategy }»
strategies-error-invalid-rules = Не удалось прочитать правила: { $reason }

strategies-error-field-average-volume = средний объём
strategies-error-field-candle-open = открытие свечи
strategies-error-field-comparison = сравнение
strategies-error-field-condition-type = тип условия
strategies-error-field-confirmation = подтверждение
strategies-error-field-count = количество
strategies-error-field-current-price = текущая цена
strategies-error-field-direction = направление
strategies-error-field-distance = расстояние
strategies-error-field-hours = часы
strategies-error-field-lookback = период просмотра
strategies-error-field-minimum-change = минимальное изменение
strategies-error-field-multiplier = множитель
strategies-error-field-pattern = паттерн
strategies-error-field-percentage = процент
strategies-error-field-period = период
strategies-error-field-position = положение
strategies-error-field-threshold = порог
strategies-error-field-time-unit = единица времени
strategies-error-field-time-value = значение времени
strategies-error-field-timeframe = таймфрейм
strategies-error-field-arm-after-profit = включение после прибыли
strategies-error-field-drop-percentage = процент просадки
strategies-error-field-entry-price = цена входа
strategies-error-field-fast-period = быстрый период
strategies-error-field-minimum-buy-share = минимальная доля покупок
strategies-error-field-minimum-transactions = минимум транзакций
strategies-error-field-moving-average-type = тип средней
strategies-error-field-moving-average = скользящая средняя
strategies-error-field-reference-candle-close = закрытие опорной свечи
strategies-error-field-peak-price = цена пика
strategies-error-field-slow-period = медленный период
strategies-error-field-window = окно
strategies-error-field-within-candles = число свечей пересечения
strategies-error-field-change = изменение
strategies-error-field-end-hour = час окончания
strategies-error-field-entry-liquidity = ликвидность при входе
strategies-error-field-giveback = отдача
strategies-error-field-highest-high = максимум
strategies-error-field-long-window-transactions = транзакции длинного окна
strategies-error-field-maximum-drop = максимальное падение
strategies-error-field-minimum-drop = минимальное падение
strategies-error-field-multiple = кратность
strategies-error-field-pool-liquidity = ликвидность пула
strategies-error-field-ratio = отношение
strategies-error-field-remaining-position-value = оставшаяся стоимость позиции
strategies-error-field-signal = сигнал
strategies-error-field-squeeze-width = ширина сжатия
strategies-error-field-standard-deviations = стандартные отклонения
strategies-error-field-start-hour = час начала
strategies-error-field-subject = что считать
strategies-error-field-token-creation-time = время создания токена
strategies-error-field-total-volume = общий объём
strategies-error-field-trade-size = размер сделки
strategies-error-field-vwap = VWAP

strategies-error-expected-boolean = логическое значение
strategies-error-expected-number = число
strategies-error-expected-string = строка

strategies-error-data-current-price = Текущая цена
strategies-error-data-liquidity-data = Данные о ликвидности
strategies-error-data-market-data = Рыночные данные
strategies-error-data-ohlcv-data = Данные OHLCV
strategies-error-data-position-data = Данные о позиции
strategies-error-data-peak-price = Цена пика
strategies-error-data-transaction-counts = Число транзакций
strategies-error-data-entry-liquidity = Ликвидность при входе
strategies-error-data-entry-pool-liquidity = Ликвидность пула входа
strategies-error-data-real-pool-reserves = Реальная ликвидность пула
strategies-error-data-remaining-position-value = Оставшаяся стоимость позиции
strategies-error-data-token-creation-time = Время создания токена
strategies-error-data-trade-size = Размер сделки

strategies-error-indicator-consecutive-candles = свечи подряд
strategies-error-indicator-moving-average = скользящая средняя
strategies-error-indicator-price-breakout = пробой цены
strategies-error-indicator-price-change-lookback = период просмотра изменения цены
strategies-error-indicator-volume-spike = всплеск объёма
strategies-error-indicator-moving-average-cross = пересечение скользящих средних
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = полосы Боллинджера
strategies-error-indicator-pullback-from-high = откат от максимума
strategies-error-indicator-volatility-range = диапазон волатильности
strategies-error-indicator-vwap = VWAP

strategies-error-rule-branch-node-missing-conditions = У узла ветви нет условий
strategies-error-rule-branch-node-missing-operator = У узла ветви нет оператора
strategies-error-rule-branch-node-must-have-at-least-one-child = У узла ветви должен быть хотя бы один дочерний узел
strategies-error-rule-invalid-rule-tree-structure = Недопустимая структура дерева правил
strategies-error-rule-leaf-node-missing-condition = У листового узла нет условия
strategies-error-rule-not-operator-must-have-exactly-one-child = У оператора NOT должен быть ровно один дочерний узел
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Группе нужно хотя бы одно включённое условие
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = Оператор NOT не может иметь отключённый дочерний узел
strategies-error-rule-rules-nest-deeper-than-four-levels = Правила могут быть вложены не глубже четырёх уровней
strategies-error-rule-the-root-group-cannot-be-switched-off = Корневую группу нельзя отключить

strategies-readiness-history-since-met =
    { $count ->
        [one] { $count } закрытый виртуальный раунд с изменения правил { $changed }, нужно { $needed }
        [few] { $count } закрытых виртуальных раунда с изменения правил { $changed }, нужно { $needed }
        [many] { $count } закрытых виртуальных раундов с изменения правил { $changed }, нужно { $needed }
       *[other] { $count } закрытого виртуального раунда с изменения правил { $changed }, нужно { $needed }
    }
strategies-readiness-history-since-short = { $count } из { $needed } закрытых виртуальных раундов с изменения правил { $changed }
strategies-readiness-priced = У каждой позиции есть цена
strategies-readiness-priced-ok = У каждой открытой виртуальной позиции есть цена пула
strategies-readiness-priced-missing =
    { $count ->
        [one] { $count } открытая виртуальная позиция без цены пула
        [few] { $count } открытые виртуальные позиции без цены пула
        [many] { $count } открытых виртуальных позиций без цены пула
       *[other] { $count } открытой виртуальной позиции без цены пула
    }
strategies-readiness-runtime = Боевая торговля доступна
strategies-readiness-runtime-ok = Настройка и защитные условия допускают боевые сделки

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = Все стратегии
strategies-view-kind = Тип стратегии
strategies-unnamed = Стратегия без названия
strategies-col-on = Вкл.
strategies-col-strategy = Стратегия
strategies-col-order = Порядок
    .title = Порядок вычисления среди включённых стратегий входа одного режима; первая с сигналом забирает токен
strategies-col-mode = Режим
strategies-col-status = Статус
strategies-col-open = Открыто
strategies-col-watching = Под наблюдением
strategies-col-rounds = Раунды
strategies-col-closed = Закрыто
strategies-col-win-rate = Доля прибыльных
strategies-col-realized = Реализовано ({ -sol })
strategies-col-manages = Управляет
strategies-col-last-signal = Последний сигнал
strategies-status-running = Работает
strategies-status-not-running = Не работает
strategies-status-off = Выключена
strategies-status-setup-required = Нужна настройка
strategies-status-live-paused = Боевые входы приостановлены
strategies-status-paper-cash = Не хватает виртуальных средств
strategies-status-slots-full = Все слоты заняты
strategies-status-entry-monitor-off = Монитор входов выключен
strategies-status-exit-monitor-off = Монитор выходов выключен
strategies-notice-setup-required = Виртуальная торговля начнётся после настройки
strategies-notice-force-stopped = Включена экстренная остановка
strategies-notice-trader-off = Auto Trader выключен
strategies-notice-waiting = Ожидание основных сервисов
strategies-notice-monitors-off = Мониторы входов и выходов выключены
strategies-notice-start = Запустить Auto Trader
# $closed and $needed count closed paper rounds; $changed is a formatted date.
strategies-readiness-progress = Виртуальные раунды { $closed }/{ $needed }
strategies-readiness-ready = Готова к боевому режиму
strategies-readiness-since = Правила изменены { $changed }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
        [one] { $book }: { $count } позиция
        [few] { $book }: { $count } позиции
        [many] { $book }: { $count } позиций
       *[other] { $book }: { $count } позиции
    }
strategies-manages-all = Все стратегии входа
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
        [one] { $first } и ещё { $more }
        [few] { $first } и ещё { $more }
        [many] { $first } и ещё { $more }
       *[other] { $first } и ещё { $more }
    }
strategies-exit-before = Проверяются первыми
strategies-exit-hint = Эти правила выхода проверяются для каждой позиции раньше любой стратегии выхода
strategies-action-edit = Изменить { $name }
strategies-action-delete = Удалить { $name }
strategies-roster-stale = Последнее обновление

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = Изменить правила боевой стратегии «{ $name }»
    .message = Новые правила торгуют реальными { -sol } со следующей проверки, а виртуальное подтверждение этой стратегии начинается заново с этого сохранения.
strategies-live-rules-save = Сохранить и торговать в боевом режиме
strategies-toast-evidence-restarted = Стратегия сохранена
    .message = Правила «{ $name }» изменились; её виртуальное подтверждение начинается заново

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = Включить боевой режим
strategies-mode-return-paper = Вернуть в виртуальный режим
strategies-mode-arm-title = Включить боевой режим для «{ $name }»
strategies-mode-arm-lead = Новые входы «{ $name }» будут совершаться реальными свопами с вашего торгового кошелька.
strategies-mode-arm-lead-exit = По своему сигналу «{ $name }» будет продавать боевые позиции реальными свопами из вашего торгового кошелька.
strategies-mode-arm-readiness = Готовность по виртуальному портфелю
strategies-mode-arm-open-paper = Открытые виртуальные позиции остаются виртуальными и закрываются виртуально.
strategies-mode-arm-ack-real-native = Новые входы тратят реальные { -sol } с вашего торгового кошелька.
strategies-mode-arm-ack-real-native-exit = Боевые позиции под её управлением продаются реальными свопами.
strategies-mode-arm-ack-fees = Боевые свопы оплачивают реальные сетевые комиссии и проскальзывание; виртуальные результаты не гарантируют боевых.
strategies-mode-arm-ack-unready = Часть проверок готовности не пройдена. Всё равно включить боевой режим для этой стратегии.
strategies-mode-arm-unavailable = Боевая торговля сейчас недоступна; см. последнюю проверку.
strategies-mode-return-title = Вернуть «{ $name }» в виртуальный режим
strategies-mode-return-message = Новые входы открываются в виртуальном портфеле без расхода { -sol }. Открытые боевые позиции остаются боевыми и закрываются в боевом режиме.
strategies-mode-return-message-exit = Она перестаёт оценивать боевые позиции; у них остаются их правила выхода.
strategies-mode-readiness-failed = Не удалось получить готовность стратегии
strategies-mode-toast-armed = { $name } теперь торгует в боевом режиме
strategies-mode-toast-paper = { $name } теперь торгует виртуально
strategies-mode-toast-failed = Не удалось изменить режим торговли
