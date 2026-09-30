strategies-filter-all = Все
strategies-filter-entry = Вход
strategies-filter-exit = Выход
strategies-type-entry = Вход
strategies-type-exit = Выход
strategies-list-empty-title = Стратегий пока нет
strategies-list-empty-hint = Создайте свою первую стратегию
strategies-new = Новая стратегия
strategies-import =
    .title = Импортировать стратегию
    .aria-label = Импортировать стратегию
strategies-item-enable =
    .title = Включить
strategies-item-disable =
    .title = Отключить

strategies-new-name = Новая стратегия

strategies-editor-name =
    .placeholder = Название стратегии
strategies-editor-dirty =
    .title = Есть несохранённые изменения
strategies-action-validate = Проверить
strategies-editor-empty = Выберите стратегию для редактирования или создайте новую
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

strategies-summary-param = { $label }: { $value }
strategies-summary-parts =
    { $count ->
        [1] { $first }
        [2] { $first }, { $second }
       *[3] { $first }, { $second }, { $third }
    }
strategies-summary-none = Нет параметров
strategies-summary-period-seconds = Период: { $amount } с
strategies-summary-period-minutes = Период: { $amount } мин
strategies-summary-period-hours = Период: { $amount } ч

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
strategies-unit-sol = { -sol }
strategies-unit-hours = ч
strategies-unit-multiplier = ×

strategies-catalog-search =
    .placeholder = Поиск условий...
strategies-catalog-search-clear =
    .aria-label = Очистить поиск
strategies-catalog-fold-all = Свернуть все
strategies-catalog-unfold-all = Развернуть все
strategies-catalog-no-description = Описание недоступно

strategies-create-title = Создать новую стратегию
strategies-create-prompt = Выберите тип стратегии, которую хотите создать:
strategies-create-entry-name = Стратегия входа
strategies-create-entry-description = Задайте условия, при которых нужно КУПИТЬ токен
strategies-create-exit-name = Стратегия выхода
strategies-create-exit-description = Задайте условия, при которых нужно ПРОДАТЬ токен

strategies-delete-title = Удалить стратегию
strategies-delete-message = Удалить стратегию «{ $name }»? Это действие нельзя отменить.

strategies-toast-fix-validation = Исправьте ошибки проверки перед сохранением
strategies-toast-enabled = Стратегия включена
    .message = «{ $name }» включена
strategies-toast-disabled = Стратегия отключена
    .message = «{ $name }» отключена
strategies-toast-toggle-failed = Не удалось переключить
    .message = Не удалось обновить статус стратегии
strategies-toast-load-failed = Ошибка загрузки
    .message = Не удалось загрузить стратегии с сервера
strategies-toast-created = Новая стратегия
    .message =
        { $type ->
            [EXIT] Создана новая стратегия выхода
           *[ENTRY] Создана новая стратегия входа
        }
strategies-toast-load-strategy-failed = Не удалось загрузить стратегию
strategies-toast-no-strategy = Стратегия не создана
    .message = Добавьте хотя бы одно условие или нажмите «Новая стратегия», чтобы сначала создать стратегию
strategies-toast-no-conditions-save = Нет условий
    .message = Перед сохранением добавьте в стратегию хотя бы одно условие
strategies-toast-name-required = Нужно название
    .message = Перед сохранением введите название стратегии
strategies-toast-saved = Стратегия сохранена
    .message = «{ $name }» успешно сохранена
strategies-toast-save-failed = Ошибка сохранения
    .message = Не удалось сохранить стратегию в базе данных
strategies-toast-no-strategy-validate = Нет стратегии для проверки
strategies-toast-no-conditions-validate = Нет условий
    .message = Перед проверкой добавьте хотя бы одно условие
strategies-toast-valid = Стратегия корректна
strategies-toast-invalid = В стратегии есть ошибки
strategies-toast-validation-failed = Ошибка проверки
strategies-toast-item-enabled = Стратегия включена
strategies-toast-item-disabled = Стратегия отключена
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
strategies-toast-condition-added = Условие добавлено
    .message = { $name } добавлено в стратегию


strategies-condition-candle-size = Паттерн размера свечи
    .description = Обнаруживает определённые паттерны свечей: большое тело, маленькое тело (доджи), длинные тени
strategies-condition-candle-size-param-pattern = Тип паттерна
    .description = Паттерн свечи, который нужно обнаружить
strategies-condition-candle-size-param-pattern-option-large-body = Большое тело (сильное движение)
strategies-condition-candle-size-param-pattern-option-small-body = Маленькое тело (доджи/неопределённость)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Длинная верхняя тень (отторжение)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Длинная нижняя тень (поддержка)
strategies-condition-candle-size-param-threshold = Порог размера, %
    .description = Процентный порог для обнаружения паттерна

strategies-condition-consecutive-candles = Свечи подряд
    .description = Обнаруживает подряд идущие зелёные (бычьи) или красные (медвежьи) свечи с фильтром по минимальному размеру
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
    .description = Проверяет, как долго удерживается позиция (для стратегий выхода — выходы по времени)
strategies-condition-position-holding-time-param-hours = Порог времени (часы)
    .description = Длительность в часах с момента открытия позиции
strategies-condition-position-holding-time-param-comparison = Сравнение
    .description = Как сравнивать возраст позиции с порогом
strategies-condition-position-holding-time-param-comparison-option-greater-than = Старше (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Не моложе (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Моложе ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Не старше (≤)

strategies-condition-price-breakout = Пробой цены
    .description = Обнаруживает пробой цены выше сопротивления (максимум периода) или ниже поддержки (минимум периода)
strategies-condition-price-breakout-param-lookback = Период просмотра
    .description = Количество свечей для определения уровня поддержки/сопротивления
strategies-condition-price-breakout-param-direction = Направление пробоя
    .description = Направление пробоя
strategies-condition-price-breakout-param-direction-option-upward = Вверх (пробой сопротивления)
strategies-condition-price-breakout-param-direction-option-downward = Вниз (пробой поддержки)
strategies-condition-price-breakout-param-confirmation = Подтверждение, %
    .description = На сколько цена должна выйти за уровень, чтобы подтвердить пробой (защита от ложных сигналов)

strategies-condition-price-change-percent = Изменение цены, %
    .description = Проверяет, изменилась ли цена на заданный процент за период времени
strategies-condition-price-change-percent-param-percentage = Порог изменения, %
    .description = Процентное изменение цены для срабатывания (0.1–1000%)
strategies-condition-price-change-percent-param-direction = Направление
    .description = Направление движения цены
strategies-condition-price-change-percent-param-direction-option-above = Рост (+%)
strategies-condition-price-change-percent-param-direction-option-below = Падение (-%)
strategies-condition-price-change-percent-param-direction-option-within = В диапазоне (±%)
strategies-condition-price-change-percent-param-time-value = Период времени
    .description = Значение периода просмотра (1–3600 для секунд, 1–1440 для минут, 1–720 для часов)
strategies-condition-price-change-percent-param-time-unit = Единица времени
    .description = Единица времени для периода просмотра
strategies-condition-price-change-percent-param-time-unit-option-seconds = Секунды
strategies-condition-price-change-percent-param-time-unit-option-minutes = Минуты
strategies-condition-price-change-percent-param-time-unit-option-hours = Часы

strategies-condition-price-to-ma = Цена и скользящая средняя
    .description = Проверяет, находится ли цена выше, ниже или в диапазоне простой скользящей средней (SMA)
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
    .description = Обнаруживает всплески объёма относительно среднего (признак растущего интереса)
strategies-condition-volume-spike-param-lookback = Период просмотра
    .description = Количество свечей для расчёта среднего объёма
strategies-condition-volume-spike-param-multiplier = Множитель объёма
    .description = Во сколько раз выше среднего (например, 2.0 = 200% от среднего)

strategies-condition-param-timeframe = Таймфрейм
    .description = Таймфрейм свечей для анализа (если не задан, используется таймфрейм стратегии)
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
strategies-error-invalid-rule-tree = Недопустимое дерево правил: { $reason }
strategies-error-evaluation-timeout = Время оценки стратегии истекло через { $timeout } мс
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

strategies-error-expected-boolean = логическое значение
strategies-error-expected-number = число
strategies-error-expected-string = строка

strategies-error-data-current-price = Текущая цена
strategies-error-data-liquidity-data = Данные о ликвидности
strategies-error-data-market-data = Рыночные данные
strategies-error-data-ohlcv-data = Данные OHLCV
strategies-error-data-position-data = Данные о позиции

strategies-error-indicator-consecutive-candles = свечи подряд
strategies-error-indicator-moving-average = скользящая средняя
strategies-error-indicator-price-breakout = пробой цены
strategies-error-indicator-price-change-lookback = период просмотра изменения цены
strategies-error-indicator-volume-spike = всплеск объёма

strategies-error-rule-branch-node-missing-conditions = У узла ветви нет условий
strategies-error-rule-branch-node-missing-operator = У узла ветви нет оператора
strategies-error-rule-branch-node-must-have-at-least-one-child = У узла ветви должен быть хотя бы один дочерний узел
strategies-error-rule-invalid-rule-tree-structure = Недопустимая структура дерева правил
strategies-error-rule-leaf-node-missing-condition = У листового узла нет условия
strategies-error-rule-not-operator-must-have-exactly-one-child = У оператора NOT должен быть ровно один дочерний узел
