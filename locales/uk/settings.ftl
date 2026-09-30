## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } хвилина
        [few] { $count } хвилини
        [many] { $count } хвилин
       *[other] { $count } хвилини
    }
settings-duration-hours =
    { $count ->
        [one] { $count } година
        [few] { $count } години
        [many] { $count } годин
       *[other] { $count } години
    }

## settings_dialog.js

settings-dialog-title = Налаштування
settings-dialog-close =
    .title = Закрити (ESC)
    .aria-label = Закрити налаштування
settings-dialog-save = Зберегти зміни
settings-dialog-saving = Збереження...
settings-dialog-saved = Збережено
settings-dialog-save-success = Налаштування успішно збережено
settings-dialog-save-failed = Не вдалося зберегти налаштування
settings-dialog-update-attention = Оновлення потребує уваги
settings-dialog-tab-interface = Інтерфейс
settings-dialog-tab-navigation = Навігація
settings-dialog-tab-startup = Запуск
settings-dialog-tab-hints = Підказки
settings-dialog-tab-data = Дані
settings-dialog-tab-security = Безпека
settings-dialog-tab-account = Обліковий запис
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Підключення агентів
settings-dialog-tab-updates = Оновлення
settings-dialog-tab-licenses = Ліцензії
settings-dialog-tab-about = Про застосунок
settings-dialog-link-privacy = Політика конфіденційності
settings-dialog-link-terms = Умови користування

## settings_dialog.js: Startup tab

settings-startup-section-title = Поведінка під час запуску
settings-startup-auto-start-label = Автозапуск трейдера
settings-startup-auto-start-hint = Автоматично запускати трейдер під час старту застосунку
settings-startup-coming-soon = Незабаром
settings-startup-default-page-label = Початкова сторінка
settings-startup-default-page-hint = Сторінка, яка відкривається під час запуску застосунку
settings-startup-page-dashboard = Панель керування
settings-startup-page-tokens = Токени
settings-startup-page-positions = Позиції
settings-startup-page-wallet = Гаманець
settings-startup-page-config = Конфігурація
settings-startup-notifications-label = Показувати фонові сповіщення
settings-startup-notifications-hint = Показувати сповіщення про події, що відбуваються у фоні

## settings_dialog.js: About tab

settings-about-logo =
    .alt = { -brand }
settings-about-tagline = Нативний торговий рушій для Solana
settings-about-link-github = { -github }
settings-about-link-docs = Документація
settings-about-link-telegram = { -telegram }
settings-about-link-website = Вебсайт
settings-about-credits = Створено для трейдерів Solana
settings-about-copyright = © { $year } { -brand }. Усі права захищено.

## interface_tab.js

settings-interface-section-appearance = Вигляд
settings-interface-theme-label = Тема
settings-interface-theme-hint = Виберіть бажану кольорову схему
settings-interface-theme-dark = Темна
settings-interface-theme-light = Світла
settings-interface-language-label = Мова
settings-interface-language-hint = Мова інтерфейсу панелі керування
settings-interface-logo-shape-label = Форма логотипів токенів
settings-interface-logo-shape-hint = «Коло» обрізає кожен логотип; «Природна» зберігає власний силует зображення
settings-interface-logo-shape-circle = Коло
settings-interface-logo-shape-natural = Природна
settings-interface-animations-label = Увімкнути анімації
settings-interface-animations-hint = Плавні переходи та ефекти
settings-interface-compact-label = Компактний режим
settings-interface-compact-hint = Зменшити відступи, щоб вмістити більше контенту
settings-interface-section-data = Дані та відображення
settings-interface-refresh-label = Інтервал оновлення
settings-interface-refresh-hint = Як часто оновлювати дані
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } секунда
        [few] { $count } секунди
        [many] { $count } секунд
       *[other] { $count } секунди
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } хвилина
        [few] { $count } хвилини
        [many] { $count } хвилин
       *[other] { $count } хвилини
    }
settings-interface-ticker-label = Показувати рядок-тікер
settings-interface-ticker-hint = Стрічка live-метрик у заголовку
settings-interface-page-size-label = Розмір сторінки таблиці
settings-interface-page-size-hint = Кількість рядків на сторінці таблиці за замовчуванням
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } рядок
        [few] { $count } рядки
        [many] { $count } рядків
       *[other] { $count } рядка
    }
settings-interface-auto-expand-label = Автоматично розгортати категорії
settings-interface-auto-expand-hint = Розгортати категорії конфігурації за замовчуванням
settings-interface-hints-label = Показувати контекстні підказки
settings-interface-hints-hint = Показувати значки довідки, що пояснюють функції панелі керування
settings-interface-featured-label = Показувати рядок рекомендованих
settings-interface-featured-hint = Показувати рядок рекомендованих токенів на сторінках «Головна» і «Токени»
settings-interface-section-sound = Звукові ефекти
settings-interface-sounds-label = Увімкнути звуки
settings-interface-sounds-hint = Звукові сигнали для навігації, змін стану та результатів

## security_tab.js

settings-security-loading = Завантаження налаштувань безпеки...
settings-security-load-failed = Не вдалося завантажити налаштування безпеки

settings-security-type-pin4 = PIN-код із 4 цифр
settings-security-type-pin6 = PIN-код із 6 цифр
settings-security-type-text = Текстовий пароль
settings-security-type-unset = Не задано

settings-security-lockscreen-title = Екран блокування панелі керування
settings-security-lockscreen-description = Захистіть панель керування PIN-кодом або паролем. Екран блокування з’являтиметься за спрацювання тригера й вимагатиме автентифікації для продовження.
settings-security-enable-label = Увімкнути екран блокування
settings-security-enable-hint = Захистити панель керування автентифікацією за паролем
settings-security-password-status-label = Стан пароля
settings-security-password-current = Поточний: { $type }
settings-security-password-none = Пароль не задано
settings-security-change = Змінити
settings-security-remove = Видалити
settings-security-set-password = Задати пароль
settings-security-auto-lock-label = Автоблокування після бездіяльності
settings-security-auto-lock-hint = Автоматично блокувати після певного часу без активності
settings-security-auto-lock-never = Ніколи
settings-security-lock-blur-label = Блокувати, коли вікно втрачає фокус
settings-security-lock-blur-hint = Автоматично блокувати, коли ви переходите до іншого застосунку
settings-security-quick-actions-title = Швидкі дії
settings-security-lock-now-label = Заблокувати панель керування зараз
settings-security-lock-now-hint = Негайно заблокувати панель керування
settings-security-lock-now = Заблокувати
settings-security-lock-not-ready = Неможливо заблокувати — екран блокування не готовий
settings-security-setting-save-failed = Не вдалося зберегти налаштування безпеки

## security_tab.js: two-factor authentication

settings-security-2fa-title = Двофакторна автентифікація
settings-security-2fa-description = Додайте ще один рівень захисту за допомогою застосунку-автентифікатора (Google Authenticator, Authy тощо)
settings-security-2fa-status-label = Стан 2FA
settings-security-2fa-status-enabled = Двофакторну автентифікацію увімкнено
settings-security-2fa-status-none = Не налаштовано
settings-security-2fa-disable = Вимкнути 2FA
settings-security-2fa-enable = Увімкнути 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Закрити
settings-security-password-set-title = Задати пароль
settings-security-password-change-title = Змінити пароль
settings-security-password-current-label = Поточний пароль
settings-security-password-current-input =
    .placeholder = Введіть поточний пароль
settings-security-password-type-label = Тип пароля
settings-security-password-new-label = Новий пароль
settings-security-password-new-input =
    .placeholder = Введіть новий пароль
settings-security-password-confirm-label = Підтвердження пароля
settings-security-password-confirm-input =
    .placeholder = Підтвердьте пароль
settings-security-password-update = Оновити пароль
settings-security-placeholder-pin4 = Введіть PIN-код із 4 цифр
settings-security-placeholder-pin6 = Введіть PIN-код із 6 цифр
settings-security-placeholder-text = Введіть пароль
settings-security-password-required = Введіть пароль
settings-security-password-mismatch = Паролі не збігаються
settings-security-pin4-invalid = PIN-код має містити рівно 4 цифри
settings-security-pin6-invalid = PIN-код має містити рівно 6 цифр
settings-security-text-too-short = Пароль має містити щонайменше 4 символи
settings-security-password-saved = Пароль збережено
settings-security-password-save-failed = Не вдалося зберегти пароль
settings-security-password-save-failed-detail = Не вдалося зберегти пароль: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Видалити пароль
settings-security-remove-description = Введіть поточний пароль, щоб зняти захист екрана блокування.
settings-security-remove-confirm = Видалити пароль
settings-security-current-required = Введіть поточний пароль
settings-security-password-removed = Пароль видалено
settings-security-password-remove-failed = Не вдалося видалити пароль
settings-security-password-remove-failed-detail = Не вдалося видалити пароль: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Увімкнення двофакторної автентифікації
settings-security-2fa-password-prompt = Введіть пароль, щоб продовжити:
settings-security-2fa-password-input =
    .placeholder = Введіть пароль
settings-security-2fa-continue = Продовжити
settings-security-2fa-manual-code = Код для ручного введення:
settings-security-2fa-qr =
    .alt = QR-код TOTP
settings-security-2fa-code-prompt = Введіть 6-значний код із застосунку-автентифікатора:
settings-security-2fa-verify-enable = Підтвердити й увімкнути
settings-security-2fa-password-required = Введіть пароль
settings-security-2fa-setup-failed = Не вдалося налаштувати 2FA
settings-security-2fa-code-invalid-length = Введіть 6-значний код
settings-security-2fa-code-invalid = Недійсний код
settings-security-2fa-enabled = Двофакторну автентифікацію увімкнено
settings-security-2fa-verify-failed = Не вдалося перевірити код
settings-security-2fa-disable-title = Вимкнення двофакторної автентифікації
settings-security-2fa-disable-prompt = Введіть пароль, щоб вимкнути 2FA:
settings-security-2fa-disable-failed = Не вдалося вимкнути 2FA
settings-security-2fa-disabled = Двофакторну автентифікацію вимкнено

## agent_connections_tab.js

settings-agent-category-analysis = Аналіз
settings-agent-category-portfolio = Портфель
settings-agent-category-trading = Торгівля
settings-agent-category-config = Конфігурація
settings-agent-category-system = Система
settings-agent-category-analysis-description = Аналіз токенів, ринкові дані та перевірки безпеки.
settings-agent-category-portfolio-description = Відкриті позиції, баланси та прибуток/збиток.
settings-agent-category-trading-description = Купівля, продаж і закриття позицій за реальні кошти.
settings-agent-category-config-description = Усі налаштування бота, зокрема RPC-ендпоінти. Ключі гаманців — ніколи.
settings-agent-category-system-description = Стан, події та екстрена зупинка.
settings-agent-category-analysis-inline = аналіз
settings-agent-category-portfolio-inline = портфель
settings-agent-category-trading-inline = торгівля
settings-agent-category-config-inline = конфігурація
settings-agent-category-system-inline = система

settings-agent-level-allow = Дозволити
settings-agent-level-ask-user = Питати
settings-agent-level-deny = Вимкнено
settings-agent-level-allow-hint = Виконується негайно.
settings-agent-level-ask-user-hint = Очікує вашого схвалення в застосунку.
settings-agent-level-deny-hint = Відхиляється й приховується від агента.

settings-agent-preset-full = Повний доступ
settings-agent-preset-ask = Питати спершу
settings-agent-preset-read = Лише читання
settings-agent-preset-full-description = Усе виконується без запитів. Ключі гаманців залишаються недоступними.
settings-agent-preset-ask-description = Кожна дія очікує вашого схвалення в застосунку.
settings-agent-preset-read-description = Читання аналізу й портфеля. Нічого не можна змінити.
settings-agent-preset-custom = Власний
settings-agent-preset-group =
    .aria-label = Набір дозволів
settings-agent-permission-group = Дозвіл: { $category }

settings-agent-summary-asks-only = Обмежено — запитує дозвіл: { $asking }
settings-agent-summary-off-only = Обмежено — вимкнено: { $off }
settings-agent-summary-asks-and-off = Обмежено — запитує дозвіл: { $asking }; вимкнено: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Універсальний stdio MCP

settings-agent-note-placeholder = Замініть /absolute/path/to/screenerbot на абсолютний шлях до виконуваного файлу { -brand } — запущений застосунок не зміг визначити шлях до свого виконуваного файлу в цій системі.
settings-agent-note-data-dir = Якщо ви запускаєте { -brand } з нестандартним каталогом даних, також задайте SCREENERBOT_DATA_DIR на стороні клієнта (ще один прапорець -e / --env або запис env) з тим самим шляхом.
settings-agent-note-codex-run = Виконайте команду або додайте блок TOML до ~/.codex/config.toml ($CODEX_HOME/config.toml). Після цього перезапустіть { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` приховує секрет у своєму виводі.
settings-agent-note-claude-code = { -claude } Code: виконайте команду, потім перезапустіть { -claude } Code. `claude mcp get screenerbot` виведе налаштоване середовище, зокрема секрет.
settings-agent-note-claude-desktop = { -claude } Desktop: додайте JSON до claude_desktop_config.json у розділ `mcpServers` і перезапустіть застосунок.
settings-agent-note-openclaw = Виконайте команду, потім за допомогою `openclaw mcp doctor screenerbot --probe` переконайтеся, що збережений stdio-сервер запускається й надає інструменти.
settings-agent-note-hermes = Додайте це в розділ `mcp_servers` конфігураційного файлу { -hermes }, потім перезапустіть { -hermes }.
settings-agent-note-generic = Будь-який MCP-клієнт зі stdio: виконайте цю команду з цими аргументами й середовищем там, де клієнт зберігає список серверів.
settings-agent-block-codex-command = { -codex } CLI — команда термінала
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (запасний варіант)
settings-agent-block-claude-command = { -claude } Code — команда термінала
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — команда термінала
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Універсальний stdio MCP-клієнт

settings-agent-name-required = Введіть назву цього підключення.
settings-agent-name-too-long = Максимальна довжина назви (символів): { $max }.
settings-agent-name-control-characters = Назва не має містити керівних символів.

settings-agent-title = Підключення агентів
settings-agent-description = Підключайте { -claude }, { -codex }, { -hermes }, { -openclaw } або будь-який stdio MCP-клієнт. { -brand } має залишатися запущеним. Кожне підключення має власні дозволи: за замовчуванням повний доступ, а обмежити його можна для кожного підключення окремо будь-коли. Жодне підключення ніколи не може прочитати чи змінити ключ вашого гаманця.
settings-agent-name-label = Назва підключення
settings-agent-name-hint = Показується в списку нижче, щоб ви могли розрізняти підключення.
settings-agent-name-input =
    .placeholder = Агент для кодування на ноутбуці
settings-agent-client-label = Клієнт
settings-agent-client-hint = Визначає інструкцію з налаштування, що показується після створення підключення.
settings-agent-permissions-label = Дозволи
settings-agent-permissions-hint = Нове підключення може все. Обмежте будь-яку категорію зараз або пізніше зі списку нижче — ключі гаманців недоступні в жодному разі.
settings-agent-create = Створити підключення
settings-agent-issued-group =
    .aria-label = Облікові дані нового підключення
settings-agent-issued-warning = Скопіюйте секрет зараз. Він показується один раз і не може бути отриманий повторно — якщо ви його втратите, відкличте підключення й створіть заново. { -brand } зберігає лише односторонній верифікатор; ваш MCP-клієнт зберігає відкритий текст у власній конфігурації.
settings-agent-issued-client-id = ID клієнта
settings-agent-issued-secret = Одноразовий секрет
settings-agent-setup-for = Налаштування для
settings-agent-done = Готово
settings-agent-list-title = Підключення
settings-agent-loading = Завантаження підключень...
settings-agent-active-count = Активних: { $count }
settings-agent-empty = Підключень ще немає. Створіть одне вище, щоб з’єднати клієнт.
settings-agent-empty-active = Активних підключень немає.
settings-agent-revoked-title = Відкликані підключення
settings-agent-created = Створено { $time }
settings-agent-last-used = Востаннє використано { $time }
settings-agent-never-used = Ніколи не використовувалось
settings-agent-permissions-edit = Дозволи
settings-agent-revoke = Відкликати
settings-agent-permissions-save = Зберегти дозволи

settings-agent-load-failed = Не вдалося завантажити підключення агентів
settings-agent-list-failed = Не вдалося завантажити підключення
settings-agent-create-failed = Не вдалося створити підключення.
settings-agent-unreachable-create = Не вдалося зв’язатися з { -brand } для створення підключення.
settings-agent-permissions-update-failed = Не вдалося оновити дозволи
settings-agent-permissions-updated = Дозволи оновлено
settings-agent-permissions-updated-detail = Застосовуються до наступного запиту підключення.
settings-agent-unreachable-save = Не вдалося зв’язатися з { -brand } для збереження
settings-agent-revoke-title = Відкликати підключення
settings-agent-revoke-message = Відкликати «{ $label }»? Клієнт перестане працювати з наступного запиту, і відновити його буде неможливо.
settings-agent-revoke-fallback-name = це підключення
settings-agent-revoke-failed = Не вдалося відкликати підключення
settings-agent-unreachable-revoke = Не вдалося зв’язатися з { -brand } для відкликання

## telegram_tab.js

settings-telegram-loading = Завантаження налаштувань { -telegram }...
settings-telegram-load-failed = Не вдалося завантажити налаштування { -telegram }
settings-telegram-unknown = Невідомо
settings-telegram-session-active = Активна: { $duration }
settings-telegram-sessions-empty = Активних сеансів немає
settings-telegram-session-revoke = Відкликати

settings-telegram-connection-title = Підключення
settings-telegram-connection-description = Підключіть свого бота { -telegram }, щоб отримувати сповіщення та керувати { -brand } віддалено.
settings-telegram-enable-label = Увімкнути { -telegram }
settings-telegram-enable-hint = Увімкнути інтеграцію з ботом { -telegram }
settings-telegram-token-label = Токен бота
settings-telegram-token-saved = Токен збережено
settings-telegram-token-help = Отримайте його в @BotFather у { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Токен збережено (введіть новий, щоб змінити)
settings-telegram-token-input =
    .placeholder = Введіть токен бота
settings-telegram-token-toggle =
    .title = Показати або приховати
settings-telegram-chat-label = ID чату
settings-telegram-chat-connected = Підключено до чату:
settings-telegram-chat-discover-hint = Автоматично визначити ID вашого чату
settings-telegram-chat-change =
    .title = Змінити
settings-telegram-chat-discover = Визначити ID чату
settings-telegram-discovery-step-add = Додайте бота до групи { -telegram } або почніть із ним особисту розмову
settings-telegram-discovery-step-privacy = Для груп: перевірте @BotFather → /mybots → [ваш бот] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Режим приватності вимкнено:</strong> бот отримує всі повідомлення групи<br/><strong>Режим приватності увімкнено:</strong> бот отримує повідомлення лише тоді, коли його згадано через @
settings-telegram-discovery-step-send = Надішліть будь-яке повідомлення (або згадайте бота через @, якщо режим приватності увімкнено)
settings-telegram-discovery-listening = Очікування повідомлень...
settings-telegram-discovery-select = Вибрати
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Мова повідомлень
settings-telegram-language-hint = Мова повідомлень і кнопок бота { -telegram }
settings-telegram-language-follow-app = Як у застосунку
settings-telegram-test-label = Перевірити підключення
settings-telegram-test-hint = Надіслати тестове повідомлення для перевірки конфігурації
settings-telegram-test-send = Надіслати тест
settings-telegram-test-sending = Надсилання...

settings-telegram-chat-type-private = приватний
settings-telegram-chat-type-group = група
settings-telegram-chat-type-supergroup = супергрупа
settings-telegram-chat-type-channel = канал

settings-telegram-auth-title = Автентифікація команд
settings-telegram-auth-description = Команди { -telegram } використовують ту саму 2FA, що й екран блокування панелі керування.
settings-telegram-auth-protected = Захищено
settings-telegram-auth-disabled = Вимкнено
settings-telegram-auth-not-configured = Не налаштовано
settings-telegram-auth-error = Помилка
settings-telegram-auth-protected-note = Команди захищено 2FA екрана блокування. Коли сеанси спливають, користувачі мають ввести код автентифікатора командою <code>/login</code>.
settings-telegram-auth-disabled-note = 2FA екрана блокування налаштовано, але вимкнено для { -telegram }. Увімкніть «Вимагати 2FA для команд» вище, щоб захистити команди { -telegram }.
settings-telegram-auth-missing-note = 2FA екрана блокування не налаштовано. Без 2FA сеанси, що спливли, повторно активуватимуться без перевірки.
settings-telegram-auth-managed-in = 2FA налаштовується в розділі
settings-telegram-auth-configure-in = Налаштуйте 2FA в розділі
settings-telegram-auth-configure-suffix = щоб вимагати підтвердження для команд { -telegram }.
settings-telegram-security-link = Налаштування безпеки
settings-telegram-timeout-title = Тайм-аут сеансу
settings-telegram-timeout-description = Як довго автентифікований сеанс залишається активним
settings-telegram-sessions-title = Активні сеанси

settings-telegram-notifications-title = Налаштування сповіщень
settings-telegram-notifications-description = Виберіть, які події викликають сповіщення в { -telegram }.
settings-telegram-notify-opened-label = Позицію відкрито
settings-telegram-notify-opened-hint = Сповіщати, коли відкривається нова позиція
settings-telegram-notify-closed-label = Позицію закрито
settings-telegram-notify-closed-hint = Сповіщати, коли позицію закрито
settings-telegram-notify-partial-label = Частковий вихід
settings-telegram-notify-partial-hint = Сповіщати про часткові виходи з позицій
settings-telegram-notify-dca-label = DCA виконано
settings-telegram-notify-dca-hint = Сповіщати, коли виконуються ордери DCA
settings-telegram-notify-errors-label = Помилки
settings-telegram-notify-errors-hint = Сповіщати про помилки та збої
settings-telegram-notify-startup-label = Запуск і зупинка
settings-telegram-notify-startup-hint = Сповіщати, коли бот запускається або зупиняється
settings-telegram-notify-filtering-label = Сповіщення фільтрації
settings-telegram-notify-filtering-hint = Сповіщати, коли нові токени проходять критерії фільтрації
settings-telegram-notify-trades-label = Сповіщення про угоди
settings-telegram-notify-trades-hint = Сповіщати про значні угоди з токенами зі списку відстеження
settings-telegram-notify-daily-label = Щоденний підсумок
settings-telegram-notify-daily-hint = Отримувати щоденний підсумок торгової активності та прибутку/збитку

settings-telegram-features-title = Функції
settings-telegram-features-description = Налаштуйте можливості бота { -telegram }.
settings-telegram-commands-label = Увімкнути команди
settings-telegram-commands-hint = Дозволити керувати ботом за допомогою команд { -telegram }
settings-telegram-require-2fa-label = Вимагати 2FA для команд
settings-telegram-require-2fa-hint = Коли сеанси спливають, вимагати код 2FA для повторної активації. Використовує 2FA екрана блокування.
settings-telegram-inline-label = Вбудовані кнопки дій
settings-telegram-inline-hint = Показувати кнопки дій у повідомленнях-сповіщеннях

settings-telegram-setting-save-failed = Не вдалося зберегти налаштування { -telegram }
settings-telegram-discovery-start-failed = Не вдалося запустити визначення
settings-telegram-chat-selected = Чат вибрано
settings-telegram-chat-select-failed = Не вдалося вибрати чат
settings-telegram-test-sent = Тестове повідомлення надіслано
settings-telegram-test-failed = Тестове повідомлення не вдалося надіслати
settings-telegram-session-revoked = Сеанс відкликано
settings-telegram-session-revoke-failed = Не вдалося відкликати сеанс

## licenses_tab.js

settings-licenses-title = Ліцензії відкритого коду
settings-licenses-subtitle = { -brand } створено з використанням такого відкритого програмного забезпечення
settings-licenses-footer = Повні тексти ліцензій доступні в репозиторії проєкту та у вихідному коді кожної залежності.
settings-licenses-category-framework = Фреймворк застосунку
settings-licenses-category-solana = Блокчейн Solana
settings-licenses-category-data = Дані та сховище
settings-licenses-category-networking = Мережа
settings-licenses-category-cryptography = Криптографія та кодування
settings-licenses-category-assets = Ресурси інтерфейсу
settings-licenses-desc-electron = Фреймворк для настільних застосунків
settings-licenses-desc-tokio = Асинхронне середовище виконання для Rust
settings-licenses-desc-axum = Фреймворк вебсервера
settings-licenses-desc-tower = Абстракції сервісів
settings-licenses-desc-hyper = Реалізація HTTP
settings-licenses-desc-solana-sdk = Ядро Solana SDK
settings-licenses-desc-solana-client = RPC-клієнт
settings-licenses-desc-solana-program = Бібліотека програм
settings-licenses-desc-spl-token = Програма SPL Token
settings-licenses-desc-spl-token-2022 = Розширення Token-2022
settings-licenses-desc-spl-associated-token-account = Асоційовані токен-акаунти
settings-licenses-desc-sqlite = Вбудований рушій баз даних
settings-licenses-desc-rusqlite = Прив’язки SQLite для Rust
settings-licenses-desc-r2d2 = Пул з’єднань із базою даних
settings-licenses-desc-serde = Фреймворк серіалізації
settings-licenses-desc-toml = Розбір конфігурації
settings-licenses-desc-reqwest = HTTP-клієнт
settings-licenses-desc-tokio-tungstenite = WebSocket-клієнт
settings-licenses-desc-rustls = Реалізація TLS
settings-licenses-desc-blake3 = Геш-функція
settings-licenses-desc-sha-2 = Гешування SHA-256/512
settings-licenses-desc-bs58 = Кодування Base58
settings-licenses-desc-base64 = Кодування Base64
settings-licenses-desc-lucide-icons = Бібліотека шрифту значків
settings-licenses-desc-inter = Шрифт інтерфейсу
settings-licenses-desc-jetbrains-mono = Моноширинний шрифт
settings-licenses-desc-orbitron = Шрифт для заголовків
settings-licenses-desc-vazirmatn = Шрифт для арабської та перської
settings-licenses-desc-noto-sans-devanagari = Шрифт для деванагарі
settings-licenses-desc-noto-sans-sc = Шрифт для спрощеної китайської
settings-licenses-desc-pretendard = Шрифт для корейської
settings-licenses-desc-pretendard-jp = Шрифт для японської

## hints_tab.js

settings-hints-title = Контекстні підказки
settings-hints-description = Контекстні підказки — це значки довідки, що пояснюють функції панелі керування. Перегляньте всі підказки нижче та відновіть ті, які ви приховали через «Більше не показувати», — по одній або всі разом.
settings-hints-hidden-label = Приховані підказки
settings-hints-hidden-summary = Наразі приховано підказок: { $hidden } із { $total }.
settings-hints-restore-all = Відновити всі підказки
settings-hints-toggle-shown =
    .title = Показувати цю підказку
settings-hints-toggle-shown-title = Показується
settings-hints-toggle-hidden-title = Приховано — увімкніть, щоб показати
settings-hints-restore-title = Відновити всі підказки
settings-hints-restore-message = Знову показати всі контекстні підказки, зокрема ті, які ви приховали?
settings-hints-restore-confirm = Відновити всі
settings-hints-restored = Усі підказки відновлено

## account_tab.js

settings-account-title = Обліковий запис { -brand }
settings-account-description = Безкоштовний і необов’язковий. { -brand } торгує, знаходить ринки та будує графіки без облікового запису — просто робить це через публічних провайдерів. Панель нижче показує, що додає вхід в обліковий запис.
settings-account-data-title = Дані { -brand }
settings-account-data-description = Ми запускаємо спільний сервіс ринкових даних на screenerbot.io: об’єднані свічки за сімома таймфреймами, розв’язаний реєстр пулів, кешовані звіти безпеки та нормалізована ідентифікація токенів. Він існує, щоб публічні провайдери не обмежували швидкість для кожної інсталяції окремо, а користування ним потребує облікового запису, щоб ці спільні витрати мали конкретного власника.
settings-account-data-fallback = Коли він недоступний, { -brand } автоматично перемикається на публічних провайдерів. Нічого не зупиняється; графіки заповнюються повільніше й містять менше історії.
settings-account-gateway-title = Надсилання транзакцій
settings-account-gateway-description = Коли ви ввійшли в обліковий запис, { -brand } може транслювати ваші свопи через screenerbot.io замість вашого власного RPC. Бот, як і раніше, будує та підписує кожну транзакцію на цьому комп’ютері — сервер лише передає її далі й не може змінити підписану транзакцію без втрати чинності підпису.
settings-account-gateway-label = Використовувати RPC { -brand } для надсилання транзакцій
settings-account-gateway-hint = Лише надсилання. Дані про ціни завжди надходять із вашого власного RPC — опитування пулів надто навантажує спільний ендпоінт, тому туди воно ніколи не надсилається.
settings-account-manage-title = Керування обліковим записом
settings-account-manage-description = Пароль, адреса електронної пошти, підключені пристрої та реферальні виплати керуються на вебсайті. Відкликання пристрою там виводить його з облікового запису всюди, зокрема й тут.
settings-account-open-dashboard = Відкрити вашу панель керування

## navigation_tab.js

settings-navigation-title = Вкладки навігації
settings-navigation-hint = Перетягуйте елементи, щоб змінити порядок. Перемикач вмикає або вимикає видимість.
settings-navigation-note = Зміни застосовуються після збереження. Оновіть сторінку, щоб побачити зміни на панелі навігації.
settings-navigation-drag-handle =
    .title = Перетягніть, щоб змінити порядок
settings-navigation-defaults-failed = Не вдалося завантажити стандартну навігацію
settings-navigation-reset = Навігацію скинуто до стандартної

## data_tab.js

settings-data-storage-title = Сховище баз даних
settings-data-storage-description = Огляд усіх баз даних, що зберігають ваші торгові дані, позиції та історичну інформацію.
settings-data-stats-loading = Завантаження статистики баз даних...
settings-data-stats-load-failed = Не вдалося завантажити статистику баз даних
settings-data-total-storage = Загальний обсяг баз даних
settings-data-db-tokens = Токени
settings-data-db-transactions = Транзакції
settings-data-db-positions = Позиції
settings-data-db-events = Події
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Гаманець
settings-data-db-pools = Пули
settings-data-db-strategies = Стратегії
settings-data-db-actions = Дії
settings-data-directory-label = Каталог даних
settings-data-directory-copied = Каталог даних
settings-data-config-path-copied = Шлях до конфігурації
settings-data-path-unavailable = Недоступно
settings-data-path-copy-title = Натисніть, щоб скопіювати шлях
settings-data-path-copy-failed = Не вдалося скопіювати шлях

settings-data-config-title = Керування конфігурацією
settings-data-config-description = Експортуйте, імпортуйте та керуйте конфігурацією бота. Робіть резервні копії перед великими змінами.
settings-data-config-export = Експортувати конфігурацію
settings-data-config-import = Імпортувати конфігурацію
settings-data-config-reset = Скинути до стандартних
settings-data-config-location-label = Розташування конфігурації
settings-data-config-fetch-failed = Не вдалося отримати конфігурацію
settings-data-config-exported = Конфігурацію експортовано
settings-data-config-export-failed = Не вдалося експортувати конфігурацію: { $message }
settings-data-config-import-title = Імпорт конфігурації
settings-data-config-import-message = Імпортувати цю конфігурацію? Поточні налаштування буде перезаписано. Облікові дані гаманців буде збережено.
settings-data-config-imported = Конфігурацію успішно імпортовано. Для деяких змін може знадобитися перезапуск.
settings-data-config-import-failed = Не вдалося імпортувати конфігурацію: { $message }
settings-data-config-reset-title = Скидання конфігурації
settings-data-config-reset-message = Скинути всі налаштування до стандартних? Облікові дані гаманців буде збережено, але всі інші налаштування буде скинуто.
settings-data-config-reset-done = Конфігурацію скинуто до стандартної
settings-data-config-reset-failed = Не вдалося скинути конфігурацію: { $message }
settings-data-unknown-error = Невідома помилка

settings-data-cleanup-title = Очищення даних
settings-data-cleanup-description = Звільніть місце на диску, видаливши старі або непотрібні дані. Ці дії неможливо скасувати.
settings-data-ohlcv-cleanup-label = Очищення даних OHLCV
settings-data-ohlcv-cleanup-hint = Видалити дані свічок для токенів, які не були активні впродовж указаного часу.
settings-data-cleanup-hours-unit = год
settings-data-cleanup-ohlcv = Очистити OHLCV
settings-data-cleanup-running = Очищення...
settings-data-cleanup-hours-invalid = Недійсне значення годин
settings-data-cleanup-confirm-title = Видалення даних OHLCV
settings-data-cleanup-confirm-message =
    Видалити дані OHLCV для токенів, неактивних понад { $hours ->
        [one] { $hours } годину
        [few] { $hours } години
        [many] { $hours } годин
       *[other] { $hours } години
    }?
settings-data-cleanup-done =
    Очищено { $count ->
        [one] { $count } неактивний токен
        [few] { $count } неактивні токени
        [many] { $count } неактивних токенів
       *[other] { $count } неактивного токена
    }
settings-data-cleanup-failed = Очищення не вдалося
settings-data-cleanup-failed-detail = Очищення не вдалося: { $message }

settings-data-cache-clear-label = Очистити весь кеш OHLCV
settings-data-cache-clear-hint = Видалити всі кешовані дані свічок і завантажити історію кожного відстежуваного токена з нуля. Використовуйте, якщо графіки виглядають неправильно або після оновлення логіки даних.
settings-data-cache-clear = Очистити кеш OHLCV
settings-data-cache-clearing = Очищення...
settings-data-cache-confirm-title = Очищення всього кешу OHLCV
settings-data-cache-confirm-message = Видалити всі кешовані дані свічок для кожного токена? Відстежувані токени повторно завантажать свою історію з нуля. Цю дію неможливо скасувати.
settings-data-candles-count =
    { $count ->
        [one] { $count } свічка
        [few] { $count } свічки
        [many] { $count } свічок
       *[other] { $count } свічки
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } токен
        [few] { $count } токени
        [many] { $count } токенів
       *[other] { $count } токена
    }
settings-data-cache-cleared = Кеш очищено ({ $candles }; { $tokens }); повторне завантаження
settings-data-cache-clear-failed = Не вдалося очистити кеш OHLCV
settings-data-cache-clear-failed-detail = Не вдалося очистити кеш OHLCV: { $message }

settings-data-ui-cache-label = Кеш стану інтерфейсу
settings-data-ui-cache-hint = Очистити збережені налаштування таблиць, стани фільтрів і параметри перегляду.
settings-data-ui-cache-clear = Очистити кеш інтерфейсу
settings-data-ui-cache-confirm-title = Очищення стану інтерфейсу
settings-data-ui-cache-confirm-message = Очистити всі збережені налаштування інтерфейсу? Буде скинуто стовпці таблиць, фільтри й параметри перегляду.
settings-data-ui-cache-cleared =
    Очищено { $count ->
        [one] { $count } кешоване налаштування інтерфейсу
        [few] { $count } кешовані налаштування інтерфейсу
        [many] { $count } кешованих налаштувань інтерфейсу
       *[other] { $count } кешованого налаштування інтерфейсу
    }

settings-data-folder-label = Відкрити папку даних
settings-data-folder-hint = Відкрийте у файловому менеджері папку з усіма даними { -brand }.
settings-data-folder-open = Відкрити папку
settings-data-folder-open-failed = Не вдалося відкрити папку даних
