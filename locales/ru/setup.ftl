setup-wallet-required = Введите приватный ключ кошелька.
setup-wallet-json-recognized = Формат JSON-ключа из 64 байт распознан.
setup-wallet-json-invalid = Используйте JSON-массив ровно из 64 значений байтов (0–255).
setup-wallet-format-invalid = Используйте приватный ключ base58 или JSON-массив из 64 байт.
setup-wallet-base58-recognized = Формат ключа base58 распознан.

setup-rpc-required = Введите хотя бы один RPC-эндпоинт.
setup-rpc-too-many = Используйте не более 10 RPC-эндпоинтов.
setup-rpc-url-invalid = Каждый эндпоинт должен быть корректным HTTPS-адресом.
setup-rpc-url-credentials = RPC-адреса не могут содержать имя пользователя или пароль.
setup-rpc-url-fragment = RPC-адреса не могут содержать фрагменты.
setup-rpc-public-endpoint = Публичный RPC Solana не подходит для непрерывного опроса.
setup-rpc-private-host = RPC-эндпоинты не могут использовать локальные или частные сетевые хосты.
setup-rpc-duplicate = Удалите повторяющиеся RPC-эндпоинты.
setup-rpc-ready =
    { $count ->
        [one] Готов к проверке: { $count } HTTPS-эндпоинт.
        [few] Готовы к проверке: { $count } HTTPS-эндпоинта.
        [many] Готовы к проверке: { $count } HTTPS-эндпоинтов.
       *[other] Готовы к проверке: { $count } HTTPS-эндпоинта.
    }

setup-wallet-verified = Кошелёк проверен
setup-wallet-unverified = Не удалось проверить кошелёк
setup-wallet-address-detail = Адрес { $address }
setup-wallet-format-hint = Проверьте формат приватного ключа.
setup-rpc-none-working = Нет работающего RPC для mainnet
setup-rpc-health-failed = Ни один эндпоинт не прошёл проверки работоспособности mainnet.
setup-rpc-partial = Работает: { $working }; недоступно: { $failed }
setup-rpc-verified =
    { $count ->
        [one] Проверен { $count } эндпоинт mainnet
        [few] Проверены { $count } эндпоинта mainnet
        [many] Проверено { $count } эндпоинтов mainnet
       *[other] Проверено { $count } эндпоинта mainnet
    }
setup-rpc-fastest = Самый быстрый: { $url } ({ $latency } мс).
setup-error-request-failed = Запрос не выполнен ({ $status })
setup-error-restart-timeout = Настройка сохранена, но { -brand } ещё не переподключился.

setup-verify-wallet-parsing = Разбор приватного ключа
setup-verify-wallet-parsing-detail = Проверка ключа и вычисление публичного адреса.
setup-verify-wallet-waiting = Ожидание проверки
setup-verify-rpc-testing = Проверка Solana mainnet
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Проверяется { $count } эндпоинт.
        [few] Проверяются { $count } эндпоинта.
        [many] Проверяются { $count } эндпоинтов.
       *[other] Проверяются { $count } эндпоинта.
    }
setup-verify-rpc-waiting = Ожидание проверки эндпоинтов
setup-verify-save-waiting = Ожидание сохранения
setup-verify-save-running = Шифрование и сохранение
setup-verify-save-running-detail = Проверенная конфигурация записывается на это устройство.
setup-verify-save-done = Конфигурация сохранена
setup-verify-save-done-detail = Приватный ключ зашифрован; рабочие RPC-эндпоинты сохранены.
setup-verify-save-failed = Не удалось сохранить настройку
setup-verify-save-skipped = Не сохранено
setup-verify-request-failed = Запрос проверки не выполнен
setup-verify-summary-checking = Проверка вашего кошелька и подключений к Solana mainnet.
setup-verify-summary-running = Проверка именно тех данных, которые вы ввели.
setup-verify-summary-saving = Данные проверены. Безопасное сохранение.
setup-verify-summary-failed = Разберитесь с проблемой и повторите проверку.

setup-error-credentials-failed = Не удалось проверить учётные данные.
setup-error-save-failed = Не удалось сохранить настройку.
setup-error-verify-failed = Проверка не удалась.
setup-error-explore-failed = Не удалось запустить режим обзора.
setup-error-gateway-failed = Не удалось сохранить настройку шлюза.
setup-action-review-credentials = Проверить данные

setup-explore-opening = Открытие режима обзора…
setup-complete-restarting = Перезапуск { -brand } с вашей проверенной конфигурацией.
setup-complete-finishing = Завершение перезапуска…
setup-complete-ready = { -brand } готов. Открытие дашборда…
setup-complete-stored = Ваша проверенная конфигурация надёжно сохранена на этом устройстве.

setup-wallet-show-key = Показать приватный ключ
setup-wallet-hide-key = Скрыть приватный ключ
setup-wallet-copy =
    .aria-label = Скопировать адрес кошелька
    .title = Скопировать адрес кошелька
setup-wallet-copy-done =
    .aria-label = Адрес кошелька скопирован
    .title = Скопировано
setup-wallet-copy-failed =
    .aria-label = Не удалось скопировать адрес кошелька
    .title = Не удалось скопировать

setup-dialog-title = Настройка кошелька и RPC
setup-dialog-subtitle = Подключите кошелёк Solana и премиум RPC-эндпоинт, чтобы включить торговлю и ончейн-данные в реальном времени. Ваш приватный ключ шифруется на этом устройстве и никогда его не покидает.
setup-dialog-close =
    .title = Закрыть
    .aria-label = Закрыть
setup-dialog-wallet-label = Приватный ключ кошелька
setup-dialog-wallet-input =
    .placeholder = Строка Base58 или массив JSON [1,2,3,...]
setup-dialog-rpc-label = RPC-эндпоинты
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (по одному на строку)
setup-dialog-rpc-hint = Настоятельно рекомендуется премиум-провайдер ({ -helius }, { -quicknode }, { -alchemy }) — публичный RPC Solana ограничен по частоте запросов и может не работать.
setup-dialog-submit = Проверить и подключить
setup-dialog-working = Выполняется…
setup-dialog-validating = Проверка…
setup-dialog-saving = Сохранение…
setup-dialog-restarting = Перезапуск…
setup-dialog-saved = Настройка сохранена — перезапуск { -brand } в полном режиме…
setup-dialog-error-missing-fields = Введите приватный ключ кошелька и хотя бы один RPC-адрес.
setup-dialog-error-validation = Проверка не удалась.
setup-dialog-error-incomplete = Не удалось завершить настройку.
setup-dialog-error-restart-helper = Помощник автоматического перезапуска недоступен. Скоро перезагрузите дашборд.
setup-dialog-error-unexpected = Непредвиденная ошибка.

setup-wizard-progress =
    .aria-label = Ход настройки
setup-wizard-step-credentials = Данные
setup-wizard-step-verification = Проверка
setup-wizard-step-complete = Готово
setup-wizard-credentials-title = Настройка данных доступа
setup-wizard-credentials-description = Подключите локальный кошелёк и надёжные RPC-эндпоинты Solana mainnet.
setup-wizard-wallet-toggle =
    .title = Показать приватный ключ
    .aria-label = Показать приватный ключ
setup-wizard-wallet-security-note = Шифруется перед сохранением.
setup-wizard-rpc-title = RPC-эндпоинты
setup-wizard-rpc-input =
    .placeholder = Один HTTPS-адрес на строку
setup-wizard-rpc-guidance = Для непрерывного опроса рекомендуется надёжный RPC для mainnet.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = рекомендуется
setup-wizard-gateway-title = Бесплатная отправка транзакций
setup-wizard-gateway-hint = Доступно после входа в аккаунт. Ваш RPC остаётся запасным вариантом.
setup-wizard-account-title = Аккаунт { -brand }
setup-wizard-account-optional = Необязательно
setup-wizard-account-loading = Проверка статуса аккаунта…
setup-wizard-verify-title = Проверка и сохранение
setup-wizard-verify-list =
    .aria-label = Статус проверки настройки
setup-wizard-verify-wallet = Кошелёк
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = Безопасная конфигурация
setup-wizard-complete-title = Настройка сохранена
setup-wizard-reconnect = Повторить подключение
setup-wizard-reload = Перезагрузить дашборд
setup-wizard-error-title = Настройка требует внимания
setup-wizard-explore = Открыть дашборд
setup-wizard-continue = Продолжить
