# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = Подія OHLCV: { $subtype }
events-filtering-default = Подія фільтрації: { $subtype }
events-trader-default = Подія трейдера: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = Завдання «{ $name }» виконано
events-task-failed = Завдання «{ $name }» завершилося помилкою
events-task-timed-out = Завдання «{ $name }» перевищило ліміт часу

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = Завдання виконано
events-subtype-task-failed = Помилка завдання
events-subtype-task-timed-out = Ліміт часу завдання

# Shown when an event carries no display text.
events-message-none = Немає повідомлення

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = Не вдалося очистити кеш OHLCV
events-ohlcv-gap-cleanup-failed = Не вдалося очистити записи заповнених розривів
events-ohlcv-gap-fill-failed = Помилка заповнення розриву для { $mint }
events-ohlcv-backfill-scheduled = Заплановано дозавантаження історії для кількох таймфреймів для { $mint } через { $pool }
events-ohlcv-fetch-failed = Не вдалося отримати OHLCV для { $mint } через { $pool }: { $error }
events-ohlcv-gap-detection-failed = Не вдалося виявити розриви для { $mint } через { $pool }
events-ohlcv-fetch-success = Збережено точок OHLCV для { $mint }: { $count }
events-ohlcv-retention-backfill-failed = Не вдалося дозавантажити історію для утримання для { $mint } через { $pool }
events-ohlcv-empty-fetch = Порожня вибірка OHLCV для { $mint } через { $pool }
events-ohlcv-pool-discovery-failed = Не вдалося знайти пули для { $mint }
events-ohlcv-pool-discovery-success = Знайдено пули для { $mint }
events-ohlcv-process-token-error = Помилка обробки { $mint }: { $error }
events-ohlcv-rate-limit-hit = Спрацював ліміт запитів під час обробки { $mint }
events-ohlcv-pool-unavailable = Немає справних пулів для { $mint }; відкладено
events-ohlcv-token-missing = Токен { $mint } зник під час обробки
events-monitors-stopped = Монітори автоматичної торгівлі зупинено
events-monitors-starting = Монітори автоматичної торгівлі запускаються
events-entry-monitor-started = Монітор можливостей входу запущено
events-exit-monitor-started = Монітор виходів і позицій запущено
events-trader-service-stopped = Сервіс трейдера коректно зупинено
events-trader-service-stopping = Розпочато зупинку сервісу трейдера
events-trader-service-started = Сервіс трейдера повністю ініціалізовано й запущено
events-trader-auto-trading-error = Автоторгівля зіткнулася з помилкою
events-trader-trading-enabled = Торгівлю увімкнено й активовано
events-trader-trading-disabled = Торгівлю вимкнено в конфігурації
events-trader-service-initializing = Розпочато ініціалізацію сервісу трейдера
events-connectivity-monitoring-stopped = Моніторинг з’єднання зупинено
events-connectivity-monitoring-started = Моніторинг з’єднання запущено (інтервал={ $seconds } с)
events-connectivity-service-initialized = Сервіс з’єднання ініціалізовано, моніторів: { $count }
events-connectivity-critical-unhealthy = Несправних критичних ендпоінтів: { $count } - система має призупинити операції
events-connectivity-endpoint-recovered = Ендпоінт відновився зі стану { $from } до справного
events-position-entry-not-landed = Купівля { $symbol } не потрапила в мережу; позицію видалено
events-position-fill-after-force-close = Угода з { $symbol } потрапила в мережу після примусового закриття позиції; її враховано, позицію перераховано

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = Своп
events-category-transaction = Транзакція
events-category-pool = Пул
events-category-position = Позиція
events-category-token = Токен
events-category-wallet = Гаманець
events-category-trader = Трейдер
events-category-entry = Вхід
events-category-system = Система
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Безпека
events-category-connectivity = З’єднання
events-category-filtering = Фільтрація
events-category-scheduled-task = Заплановане завдання
events-category-learner = Навчання
events-category-other = Інше

events-loading = Завантаження подій...
events-load-failed = Не вдалося завантажити події
events-load-failed-description = Очікування відповіді бекенда. Ми автоматично повторимо спробу.
events-load-error = Не вдалося завантажити події
events-search-placeholder = Пошук подій...
events-summary-total = Усього
events-filter-category = Категорія
events-filter-all-categories = Усі категорії
events-filter-all-severities = Усі рівні серйозності
events-col-time = Час
events-col-category = Категорія
events-col-type = Тип
events-col-severity = Серйозність
events-col-message = Повідомлення
events-col-token = Токен
events-col-details = Подробиці
# $count is the number of payload entries not shown in the preview.
events-payload-more = ще +{ $count }

## Event details dialog (ui/events_dialog.js)

events-dialog-title = Подробиці події
events-dialog-close =
    .aria-label = Закрити вікно
events-dialog-payload = Корисне навантаження
events-dialog-copy = Копіювати подробиці
events-dialog-copy-title =
    .title = Копіювати всі подробиці події
events-dialog-copy-done = Скопійовано!
events-dialog-copy-failed = Помилка
events-dialog-not-available = н/д
# $category is the category label; shown when an event has no message.
events-dialog-category-event = Подія: { $category }
events-dialog-field-id = ID події
events-dialog-field-severity = Серйозність
events-dialog-field-category = Категорія
events-dialog-field-subtype = Підтип
events-dialog-field-mint = Мінт токена
events-dialog-field-reference = Посилання
events-dialog-field-time = Час події
events-dialog-field-age = Вік
events-dialog-field-created = Створено
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = ПОДРОБИЦІ ПОДІЇ
events-dialog-export-message = ПОВІДОМЛЕННЯ
events-dialog-export-payload = КОРИСНЕ НАВАНТАЖЕННЯ
events-dialog-export-line = { $label }: { $value }
