services-health-component-unavailable = Компонент { $component } недоступен
services-health-unavailable = Статус работоспособности недоступен
services-health-pools-not-running = Сервис пулов не запущен
services-health-events-db-uninitialized = База данных событий не инициализирована
services-health-sol-price-not-running = Сервис цены { -sol } не запущен
services-health-sol-price-stale = Данные о цене { -sol } устарели ({ $seconds } с)
services-health-sol-price-no-data = Данных о цене { -sol } пока нет
services-health-telegram-discovery = Режим обнаружения
services-health-telegram-disconnected = Отключено
services-health-wallet-watch-polling-only = Обнаружение работает только через опрос
services-health-assistant-tasks-disabled = Отключено в конфигурации
services-health-connectivity-critical-unhealthy = Критичные эндпоинты неисправны: { $endpoints }
services-health-filtering-snapshot-stale = Снимок фильтрации устарел на { $seconds } с

## Services page (pages/services.js)

services-status-healthy = Исправен
services-status-starting = Запускается
services-status-degraded = Ухудшен
services-status-unhealthy = Неисправен
services-status-stopping = Останавливается
services-status-disabled = Отключён
services-status-unknown = Неизвестно

services-name-account = Аккаунт
services-name-assistant-scheduled-tasks = Запланированные задачи ассистента
services-name-ata-cleanup = Очистка токен-аккаунтов
services-name-connectivity = Подключение
services-name-copy-trading = Копитрейдинг
services-name-events = События
services-name-filtering = Фильтрация
services-name-llm-analysis = Анализ LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = Расчёт цен пулов
services-name-pools = Пулы
services-name-positions = Позиции
services-name-referral = Рефералы
services-name-rpc-stats = Статистика RPC
services-name-sol-price = Цена { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Токены
services-name-trader = Трейдер
services-name-transactions = Транзакции
services-name-update-check = Проверка обновлений
services-name-wallet = Кошелёк
services-name-wallet-watch = Отслеживание кошельков
services-name-webserver = Веб-сервер

services-loading = Загрузка сервисов...
services-load-failed = Не удалось загрузить сервисы
services-load-failed-description = Ожидаем ответа от бэкенда. Попытка повторится автоматически.
services-refresh-failed = Не удалось обновить сервисы
services-search-placeholder = Поиск сервисов...
services-summary-total = Всего
services-summary-alerts = Предупреждения
services-summary-alerts-tooltip = { $degraded } с ухудшением / { $unhealthy } неисправных
services-filter-status = Статус
services-filter-all-statuses = Все статусы
services-filter-all-services = Все сервисы
services-filter-enabled-only = Только включённые
services-filter-disabled-only = Только отключённые
services-col-service = Сервис
services-col-health = Состояние
services-col-priority = Приоритет
services-col-uptime = Время работы
services-col-activity = Активность
services-col-last-cycle = Последний цикл
services-col-avg-cycle = Сред. цикл
services-col-avg-poll = Сред. опрос
services-col-cycle-rate = Частота циклов
services-col-tasks = Задачи
services-col-ops = Оп./с
services-col-errors = Ошибки
services-col-dependencies = Зависимости
services-dependencies-none = Нет
services-activity-busy = занят на { $percent }
services-activity-polls =
    { $count ->
        [one] { $count } опрос
        [few] { $count } опроса
        [many] { $count } опросов
       *[other] { $count } опроса
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } задача
        [few] { $count } задачи
        [many] { $count } задач
       *[other] { $count } задачи
    }
    Последний: { $last }
    Сред.: { $avg }
    Опрос: { $poll }
    Простой: { $idle }
    Всего опросов: { $polls }
services-tasks-none = Нет отслеживаемых задач

# Empty table (scripts/pages/services.js)
services-empty = Нет запущенных сервисов
    .message = Сервисы появляются здесь после того, как бот их запустит.
