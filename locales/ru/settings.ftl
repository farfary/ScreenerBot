## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } минута
        [few] { $count } минуты
        [many] { $count } минут
       *[other] { $count } минуты
    }
settings-duration-hours =
    { $count ->
        [one] { $count } час
        [few] { $count } часа
        [many] { $count } часов
       *[other] { $count } часа
    }

## settings_dialog.js

settings-dialog-title = Настройки
settings-dialog-close =
    .title = Закрыть (ESC)
    .aria-label = Закрыть настройки
settings-dialog-save = Сохранить изменения
settings-dialog-saving = Сохранение...
settings-dialog-saved = Сохранено
settings-dialog-save-success = Настройки сохранены
settings-dialog-save-failed = Не удалось сохранить настройки
settings-dialog-update-attention = Обновление требует внимания
settings-dialog-tab-interface = Интерфейс
settings-dialog-tab-navigation = Навигация
settings-dialog-tab-startup = Запуск
settings-dialog-tab-hints = Подсказки
settings-dialog-tab-data = Данные
settings-dialog-tab-security = Безопасность
settings-dialog-tab-account = Аккаунт
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Подключения агентов
settings-dialog-tab-updates = Обновления
settings-dialog-tab-licenses = Лицензии
settings-dialog-tab-about = О программе
settings-dialog-link-privacy = Политика конфиденциальности
settings-dialog-link-terms = Условия использования

## settings_dialog.js: Startup tab

settings-startup-section-title = Поведение при запуске
settings-startup-auto-start-label = Автозапуск трейдера
settings-startup-auto-start-hint = Автоматически запускать трейдер при старте приложения
settings-startup-coming-soon = Скоро
settings-startup-default-page-label = Страница по умолчанию
settings-startup-default-page-hint = Страница, которая открывается при запуске приложения
settings-startup-notifications-label = Показывать фоновые уведомления
settings-startup-notifications-hint = Отображать уведомления о фоновых событиях

## settings_dialog.js: About tab

settings-about-tagline = Нативный торговый движок для Solana
settings-about-link-github = { -github }
settings-about-link-docs = Документация
settings-about-link-telegram = { -telegram }
settings-about-link-website = Сайт
settings-about-credits = Создано для трейдеров Solana
settings-about-copyright = © { $year } { -brand }. Все права защищены.

## interface_tab.js

settings-interface-section-appearance = Внешний вид
settings-interface-theme-label = Тема
settings-interface-theme-hint = Выберите цветовую схему
settings-interface-theme-dark = Тёмная
settings-interface-theme-light = Светлая
settings-interface-language-label = Язык
settings-interface-language-hint = Язык интерфейса дашборда
settings-interface-logo-shape-label = Форма логотипов токенов
settings-interface-logo-shape-hint = «Круг» обрезает каждый логотип по кругу, «Исходная» сохраняет силуэт оригинального изображения
settings-interface-logo-shape-circle = Круг
settings-interface-logo-shape-natural = Исходная
settings-interface-animations-label = Включить анимации
settings-interface-animations-hint = Плавные переходы и эффекты
settings-interface-compact-label = Компактный режим
settings-interface-compact-hint = Уменьшить отступы, чтобы вместить больше содержимого
settings-interface-section-data = Данные и отображение
settings-interface-refresh-label = Интервал обновления
settings-interface-refresh-hint = Как часто обновлять данные
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } секунда
        [few] { $count } секунды
        [many] { $count } секунд
       *[other] { $count } секунды
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } минута
        [few] { $count } минуты
        [many] { $count } минут
       *[other] { $count } минуты
    }
settings-interface-ticker-label = Показывать строку с тикером
settings-interface-ticker-hint = Живая строка метрик в шапке
settings-interface-page-size-label = Размер страницы таблицы
settings-interface-page-size-hint = Количество строк на странице таблицы по умолчанию
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } строка
        [few] { $count } строки
        [many] { $count } строк
       *[other] { $count } строки
    }
settings-interface-auto-expand-label = Автоматически разворачивать категории
settings-interface-auto-expand-hint = Разворачивать категории конфигурации по умолчанию
settings-interface-hints-label = Показывать контекстные подсказки
settings-interface-hints-hint = Отображать значки справки с пояснениями к функциям дашборда
settings-interface-featured-label = Показывать строку избранных токенов
settings-interface-featured-hint = Отображать строку избранных токенов на страницах «Главная» и «Токены»
settings-interface-section-sound = Звуковые эффекты
settings-interface-sounds-label = Включить звуки
settings-interface-sounds-hint = Тактильные сигналы для навигации, смены состояний и результатов

## security_tab.js

settings-security-loading = Загрузка настроек безопасности...
settings-security-load-failed = Не удалось загрузить настройки безопасности

settings-security-type-pin4 = 4-значный PIN-код
settings-security-type-pin6 = 6-значный PIN-код
settings-security-type-text = Текстовый пароль
settings-security-type-unset = Не задан

settings-security-lockscreen-title = Экран блокировки дашборда
settings-security-lockscreen-description = Защитите дашборд PIN-кодом или паролем. Экран блокировки появляется при срабатывании условий и требует аутентификации для продолжения.
settings-security-enable-label = Включить экран блокировки
settings-security-enable-hint = Защитить дашборд парольной аутентификацией
settings-security-password-status-label = Состояние пароля
settings-security-password-current = Текущий: { $type }
settings-security-password-none = Пароль не задан
settings-security-change = Изменить
settings-security-remove = Удалить
settings-security-set-password = Задать пароль
settings-security-auto-lock-label = Автоблокировка при бездействии
settings-security-auto-lock-hint = Автоматически блокировать после периода бездействия
settings-security-auto-lock-never = Никогда
settings-security-lock-blur-label = Блокировать при потере фокуса окна
settings-security-lock-blur-hint = Автоматически блокировать при переключении на другое приложение
settings-security-quick-actions-title = Быстрые действия
settings-security-lock-now-label = Заблокировать дашборд сейчас
settings-security-lock-now-hint = Немедленно заблокировать дашборд
settings-security-needs-password = Сначала задайте пароль, чтобы использовать это
settings-security-needs-lockscreen = Сначала включите экран блокировки, чтобы использовать это
settings-security-lock-now = Заблокировать
settings-security-lock-not-ready = Невозможно заблокировать — экран блокировки не готов
settings-security-setting-save-failed = Не удалось сохранить настройку безопасности

## security_tab.js: two-factor authentication

settings-security-2fa-title = Двухфакторная аутентификация
settings-security-2fa-description = Добавьте дополнительный уровень защиты с помощью приложения-аутентификатора (Google Authenticator, Authy и др.)
settings-security-2fa-status-label = Состояние 2FA
settings-security-2fa-status-enabled = Двухфакторная аутентификация включена
settings-security-2fa-status-none = Не настроена
settings-security-2fa-disable = Отключить 2FA
settings-security-2fa-enable = Включить 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Закрыть
settings-security-password-set-title = Задать пароль
settings-security-password-change-title = Изменить пароль
settings-security-password-current-label = Текущий пароль
settings-security-password-current-input =
    .placeholder = Введите текущий пароль
settings-security-password-type-label = Тип пароля
settings-security-password-new-label = Новый пароль
settings-security-password-new-input =
    .placeholder = Введите новый пароль
settings-security-password-confirm-label = Подтверждение пароля
settings-security-password-confirm-input =
    .placeholder = Подтвердите пароль
settings-security-password-update = Обновить пароль
settings-security-placeholder-pin4 = Введите 4-значный PIN-код
settings-security-placeholder-pin6 = Введите 6-значный PIN-код
settings-security-placeholder-text = Введите пароль
settings-security-password-required = Введите пароль
settings-security-password-mismatch = Пароли не совпадают
settings-security-pin4-invalid = PIN-код должен состоять ровно из 4 цифр
settings-security-pin6-invalid = PIN-код должен состоять ровно из 6 цифр
settings-security-text-too-short = Пароль должен содержать не менее 4 символов
settings-security-password-saved = Пароль сохранён
settings-security-password-save-failed = Не удалось сохранить пароль
settings-security-password-save-failed-detail = Не удалось сохранить пароль: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Удалить пароль
settings-security-remove-description = Введите текущий пароль, чтобы снять защиту экрана блокировки.
settings-security-remove-confirm = Удалить пароль
settings-security-current-required = Введите текущий пароль
settings-security-password-removed = Пароль удалён
settings-security-password-remove-failed = Не удалось удалить пароль
settings-security-password-remove-failed-detail = Не удалось удалить пароль: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Включить двухфакторную аутентификацию
settings-security-2fa-password-prompt = Введите пароль, чтобы продолжить:
settings-security-2fa-password-input =
    .placeholder = Введите пароль
settings-security-2fa-continue = Продолжить
settings-security-2fa-manual-code = Код для ручного ввода:
settings-security-2fa-qr =
    .alt = QR-код TOTP
settings-security-2fa-code-prompt = Введите 6-значный код из приложения-аутентификатора:
settings-security-2fa-verify-enable = Подтвердить и включить
settings-security-2fa-password-required = Введите пароль
settings-security-2fa-setup-failed = Не удалось настроить 2FA
settings-security-2fa-code-invalid-length = Введите 6-значный код
settings-security-2fa-code-invalid = Неверный код
settings-security-2fa-enabled = Двухфакторная аутентификация включена
settings-security-2fa-verify-failed = Не удалось проверить код
settings-security-2fa-disable-title = Отключить двухфакторную аутентификацию
settings-security-2fa-disable-prompt = Введите пароль, чтобы отключить 2FA:
settings-security-2fa-disable-failed = Не удалось отключить 2FA
settings-security-2fa-disabled = Двухфакторная аутентификация отключена

## agent_connections_tab.js

settings-agent-category-analysis = Анализ
settings-agent-category-portfolio = Портфель
settings-agent-category-trading = Торговля
settings-agent-category-config = Конфигурация
settings-agent-category-system = Система
settings-agent-category-analysis-description = Анализ токенов, рыночные данные и проверки безопасности.
settings-agent-category-portfolio-description = Открытые позиции, балансы и P&L.
settings-agent-category-trading-description = Покупка, продажа и закрытие позиций на реальные средства.
settings-agent-category-config-description = Все настройки бота, включая RPC-эндпоинты. Ключи кошелька недоступны никогда.
settings-agent-category-system-description = Статус, события и экстренная остановка.
settings-agent-category-analysis-inline = анализ
settings-agent-category-portfolio-inline = портфель
settings-agent-category-trading-inline = торговля
settings-agent-category-config-inline = конфигурация
settings-agent-category-system-inline = система

settings-agent-level-allow = Разрешить
settings-agent-level-ask-user = Спросить
settings-agent-level-deny = Выкл.
settings-agent-level-allow-hint = Выполняется сразу.
settings-agent-level-ask-user-hint = Ожидает вашего подтверждения в приложении.
settings-agent-level-deny-hint = Отклоняется и скрыто от агента.

settings-agent-preset-full = Полный доступ
settings-agent-preset-ask = Сначала спрашивать
settings-agent-preset-read = Только чтение
settings-agent-preset-full-description = Всё выполняется без запроса. Ключи кошелька остаются недоступными.
settings-agent-preset-ask-description = Каждое действие ожидает вашего подтверждения в приложении.
settings-agent-preset-read-description = Чтение данных анализа и портфеля. Изменять ничего нельзя.
settings-agent-preset-custom = Свой набор
settings-agent-preset-group =
    .aria-label = Набор разрешений
settings-agent-permission-group = Разрешение: { $category }

settings-agent-summary-asks-only = Ограничено — запрашивает подтверждение: { $asking }
settings-agent-summary-off-only = Ограничено — нет доступа: { $off }
settings-agent-summary-asks-and-off = Ограничено — запрашивает подтверждение: { $asking }; нет доступа: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Универсальный stdio MCP

settings-agent-note-placeholder = Замените /absolute/path/to/screenerbot на абсолютный путь к исполняемому файлу { -brand } — запущенное приложение не смогло определить путь к своему исполняемому файлу в этой системе.
settings-agent-note-data-dir = Если вы запускаете { -brand } с нестандартным каталогом данных, также задайте SCREENERBOT_DATA_DIR на стороне клиента (ещё один флаг -e / --env или запись env) с тем же путём.
settings-agent-note-codex-run = Выполните команду или добавьте блок TOML в ~/.codex/config.toml ($CODEX_HOME/config.toml). После этого перезапустите { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` скрывает секрет в выводе.
settings-agent-note-claude-code = { -claude } Code: выполните команду и перезапустите { -claude } Code. `claude mcp get screenerbot` выведет настроенное окружение, включая секрет.
settings-agent-note-claude-desktop = { -claude } Desktop: добавьте JSON в claude_desktop_config.json в раздел `mcpServers` и перезапустите приложение.
settings-agent-note-openclaw = Выполните команду, затем проверьте командой `openclaw mcp doctor screenerbot --probe`, что сохранённый stdio-сервер запускается и предоставляет инструменты.
settings-agent-note-hermes = Добавьте это в раздел `mcp_servers` файла конфигурации { -hermes }, затем перезапустите { -hermes }.
settings-agent-note-generic = Любой MCP-клиент с поддержкой stdio: запустите эту команду с указанными аргументами и окружением там, где клиент хранит список серверов.
settings-agent-block-codex-command = { -codex } CLI — команда терминала
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (запасной вариант)
settings-agent-block-claude-command = { -claude } Code — команда терминала
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — команда терминала
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Универсальный stdio MCP-клиент

settings-agent-name-required = Введите название подключения.
settings-agent-name-too-long = Максимальная длина названия: { $max } симв.
settings-agent-name-control-characters = Название не должно содержать управляющих символов.

settings-agent-title = Подключения агентов
settings-agent-description = Подключайте { -claude }, { -codex }, { -hermes }, { -openclaw } или любой MCP-клиент со stdio. { -brand } должен оставаться запущенным. У каждого подключения свои разрешения: по умолчанию полный доступ, а при необходимости его можно ограничить для отдельного подключения. Ни одно подключение не может прочитать или изменить ключ вашего кошелька.
settings-agent-name-label = Название подключения
settings-agent-name-hint = Отображается в списке ниже, чтобы вы могли различать подключения.
settings-agent-name-input =
    .placeholder = Агент для кода на ноутбуке
settings-agent-client-label = Клиент
settings-agent-client-hint = Определяет инструкцию по настройке, которая показывается после создания подключения.
settings-agent-permissions-label = Разрешения
settings-agent-permissions-hint = Новое подключение может всё. Ограничьте любую категорию сейчас или позже в списке ниже — ключи кошелька недоступны в любом случае.
settings-agent-create = Создать подключение
settings-agent-issued-group =
    .aria-label = Учётные данные нового подключения
settings-agent-issued-warning = Скопируйте секрет сейчас. Он показывается один раз, и получить его повторно нельзя — если вы его потеряете, отзовите подключение и создайте заново. { -brand } хранит только односторонний верификатор, а открытый текст хранится в конфигурации вашего MCP-клиента.
settings-agent-issued-client-id = ID клиента
settings-agent-issued-secret = Одноразовый секрет
settings-agent-setup-for = Настройка для
settings-agent-done = Готово
settings-agent-list-title = Подключения
settings-agent-loading = Загрузка подключений...
settings-agent-active-count = Активных: { $count }
settings-agent-empty = Подключений пока нет. Создайте первое выше, чтобы связать клиент.
settings-agent-empty-active = Нет активных подключений.
settings-agent-revoked-title = Отозванные подключения
settings-agent-created = Создано { $time }
settings-agent-last-used = Последнее использование: { $time }
settings-agent-never-used = Не использовалось
settings-agent-permissions-edit = Разрешения
settings-agent-revoke = Отозвать
settings-agent-permissions-save = Сохранить разрешения

settings-agent-load-failed = Не удалось загрузить подключения агентов
settings-agent-list-failed = Не удалось загрузить подключения
settings-agent-create-failed = Не удалось создать подключение.
settings-agent-unreachable-create = Не удалось связаться с { -brand } для создания подключения.
settings-agent-permissions-update-failed = Не удалось обновить разрешения
settings-agent-permissions-updated = Разрешения обновлены
settings-agent-permissions-updated-detail = Применяются со следующего запроса подключения.
settings-agent-unreachable-save = Не удалось связаться с { -brand } для сохранения
settings-agent-revoke-title = Отзыв подключения
settings-agent-revoke-message = Отозвать «{ $label }»? Клиент перестанет работать со следующего запроса, восстановить подключение будет нельзя.
settings-agent-revoke-fallback-name = это подключение
settings-agent-revoke-failed = Не удалось отозвать подключение
settings-agent-unreachable-revoke = Не удалось связаться с { -brand } для отзыва

## telegram_tab.js

settings-telegram-loading = Загрузка настроек { -telegram }...
settings-telegram-load-failed = Не удалось загрузить настройки { -telegram }
settings-telegram-unknown = Неизвестно
settings-telegram-session-active = Активна: { $duration }
settings-telegram-sessions-empty = Нет активных сессий
settings-telegram-session-revoke = Отозвать

settings-telegram-connection-title = Подключение
settings-telegram-connection-description = Подключите бота { -telegram }, чтобы получать уведомления и управлять { -brand } удалённо.
settings-telegram-enable-label = Включить { -telegram }
settings-telegram-enable-hint = Включить интеграцию с ботом { -telegram }
settings-telegram-token-label = Токен бота
settings-telegram-token-saved = Токен сохранён
settings-telegram-token-help = Получите его у @BotFather в { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Токен сохранён (введите новый, чтобы изменить)
settings-telegram-token-input =
    .placeholder = Введите токен бота
settings-telegram-token-toggle =
    .title = Показать/скрыть
settings-telegram-chat-label = ID чата
settings-telegram-chat-connected = Подключено к чату:
settings-telegram-chat-discover-hint = Определить ID чата автоматически
settings-telegram-chat-change =
    .title = Изменить
settings-telegram-chat-discover = Определить ID чата
settings-telegram-discovery-step-add = Добавьте бота в группу { -telegram } или начните с ним личный чат
settings-telegram-discovery-step-privacy = Для групп: проверьте @BotFather → /mybots → [ваш бот] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Режим приватности выкл.:</strong> бот получает все сообщения группы<br/><strong>Режим приватности вкл.:</strong> бот получает сообщения только при упоминании через @
settings-telegram-discovery-step-send = Отправьте любое сообщение (или упомяните бота через @, если режим приватности включён)
settings-telegram-discovery-listening = Ожидание сообщений...
settings-telegram-discovery-select = Выбрать
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Язык сообщений
settings-telegram-language-hint = Язык сообщений и кнопок бота { -telegram }
settings-telegram-language-follow-app = Как в приложении
settings-telegram-test-label = Проверка подключения
settings-telegram-test-hint = Отправьте тестовое сообщение, чтобы проверить настройки
settings-telegram-test-send = Отправить тест
settings-telegram-test-sending = Отправка...

settings-telegram-chat-type-private = личный
settings-telegram-chat-type-group = группа
settings-telegram-chat-type-supergroup = супергруппа
settings-telegram-chat-type-channel = канал

settings-telegram-auth-title = Аутентификация команд
settings-telegram-auth-description = Команды { -telegram } используют ту же 2FA, что и экран блокировки дашборда.
settings-telegram-auth-protected = Защищено
settings-telegram-auth-disabled = Отключено
settings-telegram-auth-not-configured = Не настроено
settings-telegram-auth-error = Ошибка
settings-telegram-auth-protected-note = Команды защищены 2FA экрана блокировки. Когда сессия истекает, пользователь должен ввести код аутентификатора с помощью команды <code>/login</code>.
settings-telegram-auth-disabled-note = 2FA экрана блокировки настроена, но отключена для { -telegram }. Включите параметр «Требовать 2FA для команд» выше, чтобы защитить команды { -telegram }.
settings-telegram-auth-missing-note = 2FA экрана блокировки не настроена. Без 2FA истёкшие сессии будут автоматически возобновляться без проверки.
settings-telegram-auth-managed-in = 2FA управляется в разделе
settings-telegram-auth-configure-in = Настройте 2FA в разделе
settings-telegram-auth-configure-suffix = чтобы требовать подтверждение для команд { -telegram }.
settings-telegram-security-link = Настройки безопасности
settings-telegram-timeout-title = Тайм-аут сессии
settings-telegram-timeout-description = Как долго остаётся активной аутентифицированная сессия
settings-telegram-sessions-title = Активные сессии

settings-telegram-notifications-title = Настройки уведомлений
settings-telegram-notifications-description = Выберите, какие события вызывают уведомления { -telegram }.
settings-telegram-notify-opened-label = Позиция открыта
settings-telegram-notify-opened-hint = Уведомлять об открытии новой позиции
settings-telegram-notify-closed-label = Позиция закрыта
settings-telegram-notify-closed-hint = Уведомлять о закрытии позиции
settings-telegram-notify-partial-label = Частичный выход
settings-telegram-notify-partial-hint = Уведомлять о частичных выходах из позиций
settings-telegram-notify-dca-label = DCA выполнен
settings-telegram-notify-dca-hint = Уведомлять о выполнении ордеров DCA
settings-telegram-notify-errors-label = Ошибки
settings-telegram-notify-errors-hint = Уведомлять об ошибках и сбоях
settings-telegram-notify-startup-label = Запуск/остановка
settings-telegram-notify-startup-hint = Уведомлять о запуске и остановке бота
settings-telegram-notify-filtering-label = Оповещения фильтрации
settings-telegram-notify-filtering-hint = Уведомлять, когда новые токены проходят критерии фильтрации
settings-telegram-notify-trades-label = Оповещения о сделках
settings-telegram-notify-trades-hint = Уведомлять о крупных сделках по отслеживаемым токенам
settings-telegram-notify-daily-label = Ежедневная сводка
settings-telegram-notify-daily-hint = Получать ежедневную сводку по торговой активности и P&L

settings-telegram-features-title = Функции
settings-telegram-features-description = Настройте возможности бота { -telegram }.
settings-telegram-commands-label = Включить команды
settings-telegram-commands-hint = Разрешить управление ботом через команды { -telegram }
settings-telegram-require-2fa-label = Требовать 2FA для команд
settings-telegram-require-2fa-hint = Когда сессия истекает, для её возобновления требуется код 2FA. Используется 2FA экрана блокировки.
settings-telegram-inline-label = Встроенные кнопки действий
settings-telegram-inline-hint = Показывать кнопки действий в сообщениях-уведомлениях

settings-telegram-setting-save-failed = Не удалось сохранить настройку { -telegram }
settings-telegram-discovery-start-failed = Не удалось запустить определение
settings-telegram-chat-selected = Чат выбран
settings-telegram-chat-select-failed = Не удалось выбрать чат
settings-telegram-test-sent = Тестовое сообщение отправлено
settings-telegram-test-failed = Не удалось отправить тестовое сообщение
settings-telegram-session-revoked = Сессия отозвана
settings-telegram-session-revoke-failed = Не удалось отозвать сессию

## licenses_tab.js

settings-licenses-title = Лицензии открытого ПО
settings-licenses-subtitle = { -brand } создан с использованием следующего программного обеспечения с открытым исходным кодом
settings-licenses-footer = Полные тексты лицензий доступны в репозитории проекта и в исходном коде каждой зависимости.
settings-licenses-category-framework = Фреймворк приложения
settings-licenses-category-solana = Блокчейн Solana
settings-licenses-category-data = Данные и хранилище
settings-licenses-category-networking = Сеть
settings-licenses-category-cryptography = Криптография и кодирование
settings-licenses-category-assets = Ресурсы интерфейса
settings-licenses-desc-electron = Фреймворк для десктопных приложений
settings-licenses-desc-tokio = Асинхронная среда выполнения для Rust
settings-licenses-desc-axum = Фреймворк веб-сервера
settings-licenses-desc-tower = Абстракции сервисов
settings-licenses-desc-hyper = Реализация HTTP
settings-licenses-desc-solana-sdk = Ядро Solana SDK
settings-licenses-desc-solana-client = RPC-клиент
settings-licenses-desc-solana-program = Библиотека программ
settings-licenses-desc-spl-token = Программа SPL Token
settings-licenses-desc-spl-token-2022 = Расширения Token-2022
settings-licenses-desc-spl-associated-token-account = Ассоциированные токен-аккаунты
settings-licenses-desc-sqlite = Встраиваемый движок базы данных
settings-licenses-desc-rusqlite = Привязки SQLite для Rust
settings-licenses-desc-r2d2 = Пул подключений к базе данных
settings-licenses-desc-serde = Фреймворк сериализации
settings-licenses-desc-toml = Разбор конфигурации
settings-licenses-desc-reqwest = HTTP-клиент
settings-licenses-desc-tokio-tungstenite = WebSocket-клиент
settings-licenses-desc-rustls = Реализация TLS
settings-licenses-desc-blake3 = Хеш-функция
settings-licenses-desc-sha-2 = Хеширование SHA-256/512
settings-licenses-desc-bs58 = Кодирование Base58
settings-licenses-desc-base64 = Кодирование Base64
settings-licenses-desc-lucide-icons = Библиотека иконочного шрифта
settings-licenses-desc-inter = Шрифт интерфейса
settings-licenses-desc-jetbrains-mono = Моноширинный шрифт
settings-licenses-desc-orbitron = Акцидентный шрифт
settings-licenses-desc-vazirmatn = Шрифт для арабского и персидского
settings-licenses-desc-noto-sans-devanagari = Шрифт для деванагари
settings-licenses-desc-noto-sans-sc = Шрифт для упрощённого китайского
settings-licenses-desc-pretendard = Шрифт для корейского
settings-licenses-desc-pretendard-jp = Шрифт для японского

## hints_tab.js

settings-hints-title = Контекстные подсказки
settings-hints-description = Контекстные подсказки — это значки справки, которые поясняют функции дашборда. Просмотрите все подсказки ниже и верните те, что вы скрыли кнопкой «Больше не показывать», — по одной или все сразу.
settings-hints-hidden-label = Скрытые подсказки
settings-hints-hidden-summary = Скрыто подсказок: { $hidden } из { $total }.
settings-hints-restore-all = Вернуть все подсказки
settings-hints-toggle-shown =
    .title = Показывать эту подсказку
settings-hints-toggle-shown-title = Показана
settings-hints-toggle-hidden-title = Скрыта — включите, чтобы показать
settings-hints-restore-title = Вернуть все подсказки
settings-hints-restore-message = Снова показать все контекстные подсказки, включая скрытые вами?
settings-hints-restore-confirm = Вернуть все
settings-hints-restored = Все подсказки возвращены

## account_tab.js

settings-account-title = Аккаунт { -brand }
settings-account-description = Бесплатный и необязательный. { -brand } торгует, находит токены и строит графики и без аккаунта — просто работает через публичных провайдеров. Панель ниже показывает, что даёт вход в аккаунт.
settings-account-data-title = Данные { -brand }
settings-account-data-description = Мы поддерживаем общий сервис рыночных данных на screenerbot.io: объединённые свечи по семи таймфреймам, готовый реестр пулов, кэшированные отчёты о безопасности и нормализованные данные о токенах. Он нужен, чтобы публичные провайдеры не ограничивали по частоте каждую установку отдельно, а для его использования требуется аккаунт, чтобы у общих затрат был конкретный владелец.
settings-account-data-fallback = Если сервис недоступен, { -brand } автоматически переключается на публичных провайдеров. Ничего не останавливается; графики заполняются медленнее и содержат меньше истории.
settings-account-gateway-title = Отправка транзакций
settings-account-gateway-description = После входа в аккаунт { -brand } может отправлять ваши свопы через screenerbot.io вместо вашего собственного RPC. Бот по-прежнему создаёт и подписывает каждую транзакцию на этом компьютере — сервер только пересылает её и не может изменить подписанную транзакцию, не сделав подпись недействительной.
settings-account-gateway-label = Использовать RPC { -brand } для отправки транзакций
settings-account-gateway-hint = Только для отправки. Данные о ценах всегда поступают с вашего RPC — опрос пулов слишком тяжёл для общего эндпоинта, поэтому туда он не отправляется.
settings-account-manage-title = Управление аккаунтом
settings-account-manage-description = Пароль, адрес электронной почты, подключённые устройства и реферальные выплаты управляются на сайте. Отзыв устройства там выполняет выход на всех устройствах, включая это.
settings-account-open-dashboard = Открыть ваш дашборд

## navigation_tab.js

settings-navigation-title = Вкладки навигации
settings-navigation-hint = Перетаскивайте элементы, чтобы изменить порядок. Видимость переключается тумблером.
settings-navigation-section-layout = Расположение
settings-navigation-overflow-label = Вкладки, которые не помещаются
settings-navigation-overflow-hint = Прокручивать ряд вкладок вбок или собирать не поместившиеся вкладки в меню «Ещё» в конце.
settings-navigation-overflow-scroll = Прокрутка
settings-navigation-overflow-menu = Меню «Ещё»
settings-navigation-drag-handle =
    .title = Перетащите, чтобы изменить порядок
settings-navigation-defaults-failed = Не удалось загрузить навигацию по умолчанию
settings-navigation-reset = Навигация сброшена к значениям по умолчанию

## data_tab.js

settings-data-storage-title = Хранилище баз данных
settings-data-storage-description = Обзор всех баз данных, в которых хранятся ваши торговые данные, позиции и история.
settings-data-stats-loading = Загрузка статистики баз данных...
settings-data-stats-load-failed = Не удалось загрузить статистику баз данных
settings-data-total-storage = Общий объём баз данных
settings-data-db-tokens = Токены
settings-data-db-transactions = Транзакции
settings-data-db-positions = Позиции
settings-data-db-events = События
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Кошелёк
settings-data-db-pools = Пулы
settings-data-db-strategies = Стратегии
settings-data-db-actions = Действия
settings-data-directory-label = Каталог данных
settings-data-directory-copied = Каталог данных
settings-data-config-path-copied = Путь к конфигурации
settings-data-path-unavailable = Недоступно
settings-data-path-copy-title = Нажмите, чтобы скопировать путь
settings-data-path-copy-failed = Не удалось скопировать путь

settings-data-config-title = Управление конфигурацией
settings-data-config-description = Экспортируйте, импортируйте и настраивайте конфигурацию бота. Делайте резервные копии перед серьёзными изменениями.
settings-data-config-export = Экспорт конфигурации
settings-data-config-import = Импорт конфигурации
settings-data-config-reset = Сбросить по умолчанию
settings-data-config-location-label = Расположение конфигурации
settings-data-config-fetch-failed = Не удалось получить конфигурацию
settings-data-config-exported = Конфигурация экспортирована
settings-data-config-export-failed = Не удалось экспортировать конфигурацию: { $message }
settings-data-config-import-title = Импорт конфигурации
settings-data-config-import-message = Импортировать эту конфигурацию? Текущие настройки будут перезаписаны. Учётные данные кошелька сохранятся.
settings-data-config-imported = Конфигурация импортирована. Для некоторых изменений может потребоваться перезапуск.
settings-data-config-import-failed = Не удалось импортировать конфигурацию: { $message }
settings-data-config-reset-title = Сброс конфигурации
settings-data-config-reset-message = Сбросить все настройки по умолчанию? Учётные данные кошелька сохранятся, но все остальные настройки будут сброшены.
settings-data-config-reset-done = Конфигурация сброшена по умолчанию
settings-data-config-reset-failed = Не удалось сбросить конфигурацию: { $message }
settings-data-unknown-error = Неизвестная ошибка

settings-data-cleanup-title = Очистка данных
settings-data-cleanup-description = Освободите место на диске, удалив старые или неиспользуемые данные. Эти действия нельзя отменить.
settings-data-ohlcv-cleanup-label = Очистка данных OHLCV
settings-data-ohlcv-cleanup-hint = Удалить данные свечей для токенов, которые не были активны заданное время.
settings-data-cleanup-hours-unit = ч
settings-data-cleanup-ohlcv = Очистить OHLCV
settings-data-cleanup-running = Очистка...
settings-data-cleanup-hours-invalid = Недопустимое значение часов
settings-data-cleanup-confirm-title = Удаление данных OHLCV
settings-data-cleanup-confirm-message =
    Удалить данные OHLCV токенов, неактивных более { $hours ->
        [one] { $hours } часа
        [few] { $hours } часов
        [many] { $hours } часов
       *[other] { $hours } часа
    }?
settings-data-cleanup-done =
    Очищено: { $count ->
        [one] { $count } неактивный токен
        [few] { $count } неактивных токена
        [many] { $count } неактивных токенов
       *[other] { $count } неактивного токена
    }
settings-data-cleanup-failed = Очистка не удалась
settings-data-cleanup-failed-detail = Очистка не удалась: { $message }

settings-data-cache-clear-label = Очистить весь кэш OHLCV
settings-data-cache-clear-hint = Удалить все кэшированные данные свечей и заново загрузить их для каждого отслеживаемого токена. Используйте, если графики выглядят неверно или после обновления логики данных.
settings-data-cache-clear = Очистить кэш OHLCV
settings-data-cache-clearing = Очистка...
settings-data-cache-confirm-title = Очистка всего кэша OHLCV
settings-data-cache-confirm-message = Удалить все кэшированные данные свечей для каждого токена? Отслеживаемые токены заново загрузят историю с нуля. Это действие нельзя отменить.
settings-data-candles-count =
    { $count ->
        [one] { $count } свеча
        [few] { $count } свечи
        [many] { $count } свечей
       *[other] { $count } свечи
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токена
        [many] { $count } токенов
       *[other] { $count } токена
    }
settings-data-cache-cleared = Очищено: { $candles } ({ $tokens }); идёт повторная загрузка
settings-data-cache-clear-failed = Не удалось очистить кэш OHLCV
settings-data-cache-clear-failed-detail = Не удалось очистить кэш OHLCV: { $message }

settings-data-ui-cache-label = Кэш состояния интерфейса
settings-data-ui-cache-hint = Удалить сохранённые настройки таблиц, состояния фильтров и параметры отображения.
settings-data-ui-cache-clear = Очистить кэш интерфейса
settings-data-ui-cache-confirm-title = Очистка состояния интерфейса
settings-data-ui-cache-confirm-message = Удалить все сохранённые настройки интерфейса? Будут сброшены столбцы таблиц, фильтры и параметры отображения.
settings-data-ui-cache-cleared =
    Очищено: { $count ->
        [one] { $count } кэшированная настройка интерфейса
        [few] { $count } кэшированных настройки интерфейса
        [many] { $count } кэшированных настроек интерфейса
       *[other] { $count } кэшированной настройки интерфейса
    }

settings-data-folder-label = Открыть папку данных
settings-data-folder-hint = Открыть в файловом менеджере папку со всеми данными { -brand }.
settings-data-folder-open = Открыть папку
settings-data-folder-open-failed = Не удалось открыть папку данных
