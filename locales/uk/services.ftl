services-health-component-unavailable = Компонент { $component } недоступний
services-health-unavailable = Стан здоров’я недоступний
services-health-pools-not-running = Сервіс пулів не працює
services-health-events-db-uninitialized = Базу даних подій не ініціалізовано
services-health-sol-price-not-running = Сервіс ціни SOL не працює
services-health-sol-price-stale = Дані про ціну SOL застаріли ({ $seconds } с)
services-health-sol-price-no-data = Даних про ціну SOL ще немає
services-health-telegram-discovery = Режим виявлення
services-health-telegram-disconnected = Відключено
services-health-wallet-watch-polling-only = Виявлення працює лише на опитуванні
services-health-assistant-tasks-disabled = Вимкнено в конфігурації
services-health-connectivity-critical-unhealthy = Критичні ендпоінти несправні: { $endpoints }
services-health-filtering-snapshot-stale = Знімок фільтрації застарів на { $seconds } с

services-status-healthy = Справний
services-status-starting = Запускається
services-status-degraded = Погіршений
services-status-unhealthy = Несправний
services-status-stopping = Зупиняється
services-status-disabled = Вимкнено
services-status-unknown = Невідомо

services-name-account = Обліковий запис
services-name-assistant-scheduled-tasks = Заплановані завдання асистента
services-name-ata-cleanup = Очищення токен-акаунтів
services-name-connectivity = Підключення
services-name-copy-trading = Копітрейдинг
services-name-events = Події
services-name-filtering = Фільтрація
services-name-llm-analysis = Аналіз LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = Розрахунок цін пулів
services-name-pools = Пули
services-name-positions = Позиції
services-name-referral = Реферали
services-name-rpc-stats = Статистика RPC
services-name-sol-price = Ціна { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Токени
services-name-trader = Трейдер
services-name-transactions = Транзакції
services-name-update-check = Перевірка оновлень
services-name-wallet = Гаманець
services-name-wallet-watch = Стеження за гаманцем
services-name-webserver = Вебсервер

services-loading = Завантаження сервісів...
services-load-failed = Не вдалося завантажити сервіси
services-load-failed-description = Очікування відповіді від бекенду. Ми повторимо спробу автоматично.
services-refresh-failed = Не вдалося оновити сервіси
services-search-placeholder = Пошук сервісів...
services-summary-total = Усього
services-summary-alerts = Сповіщення
services-summary-alerts-tooltip = Погіршених: { $degraded } / несправних: { $unhealthy }
services-filter-status = Статус
services-filter-all-statuses = Усі статуси
services-filter-all-services = Усі сервіси
services-filter-enabled-only = Лише ввімкнені
services-filter-disabled-only = Лише вимкнені
services-col-service = Сервіс
services-col-health = Стан
services-col-priority = Пріоритет
services-col-uptime = Аптайм
services-col-activity = Активність
services-col-last-cycle = Останній цикл
services-col-avg-cycle = Сер. цикл
services-col-avg-poll = Сер. опитування
services-col-cycle-rate = Циклів/с
services-col-tasks = Завдання
services-col-ops = Операцій/с
services-col-errors = Помилки
services-col-dependencies = Залежності
services-activity-busy = Зайнятість: { $percent }
services-activity-polls =
    { $count ->
        [one] { $count } опитування
        [few] { $count } опитування
        [many] { $count } опитувань
       *[other] { $count } опитування
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } завдання
        [few] { $count } завдання
        [many] { $count } завдань
       *[other] { $count } завдання
    }
    Останнє: { $last }
    Сер.: { $avg }
    Опитування: { $poll }
    Простій: { $idle }
    Усього опитувань: { $polls }
services-tasks-none = Немає інструментованих завдань

# Empty table (scripts/pages/services.js)
services-empty = Немає запущених сервісів
    .message = Сервіси з’являються тут після того, як бот їх запустить.
