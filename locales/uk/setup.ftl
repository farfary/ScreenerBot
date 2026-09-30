# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = Введіть приватний ключ гаманця.
setup-wallet-json-recognized = Розпізнано формат JSON-ключа з 64 байтів.
setup-wallet-json-invalid = Використайте масив JSON, що містить рівно 64 значення байтів (0–255).
setup-wallet-format-invalid = Використайте приватний ключ у base58 або масив JSON із 64 байтів.
setup-wallet-base58-recognized = Розпізнано формат ключа base58.

## RPC endpoint validation

setup-rpc-required = Введіть принаймні один RPC-ендпоінт.
setup-rpc-too-many = Використовуйте не більше 10 RPC-ендпоінтів.
setup-rpc-url-invalid = Кожен ендпоінт має бути дійсним URL-адресою HTTPS.
setup-rpc-url-credentials = RPC-адреси не можуть містити імені користувача чи пароля.
setup-rpc-url-fragment = RPC-адреси не можуть містити фрагментів.
setup-rpc-public-endpoint = Публічний RPC Solana не підтримує безперервне опитування.
setup-rpc-private-host = RPC-ендпоінти не можуть використовувати локальні чи приватні мережеві хости.
setup-rpc-duplicate = Видаліть дубльовані RPC-ендпоінти.
setup-rpc-ready =
    { $count ->
        [one] { $count } HTTPS-ендпоінт готовий до перевірки.
        [few] { $count } HTTPS-ендпоінти готові до перевірки.
        [many] { $count } HTTPS-ендпоінтів готові до перевірки.
       *[other] { $count } HTTPS-ендпоінта готові до перевірки.
    }

## Verification results

setup-wallet-verified = Гаманець перевірено
setup-wallet-unverified = Не вдалося перевірити гаманець
setup-wallet-address-detail = Адреса { $address }
setup-wallet-format-hint = Перевірте формат приватного ключа.
setup-rpc-none-working = Немає робочого RPC мейннету
setup-rpc-health-failed = Жоден ендпоінт не пройшов перевірки працездатності мейннету.
setup-rpc-partial = Працюють: { $working }; недоступні: { $failed }
setup-rpc-verified =
    { $count ->
        [one] Перевірено { $count } ендпоінт мейннету
        [few] Перевірено { $count } ендпоінти мейннету
        [many] Перевірено { $count } ендпоінтів мейннету
       *[other] Перевірено { $count } ендпоінта мейннету
    }
setup-rpc-fastest = Найшвидший: { $url } ({ $latency } мс).
setup-error-request-failed = Запит не вдався ({ $status })
setup-error-restart-timeout = Налаштування збережено, але { -brand } ще не перепідключився.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = Розбір приватного ключа
setup-verify-wallet-parsing-detail = Перевірка ключа й отримання його публічної адреси.
setup-verify-wallet-waiting = Очікування перевірки
setup-verify-rpc-testing = Перевірка мейннету Solana
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Перевірка { $count } ендпоінта.
        [few] Перевірка { $count } ендпоінтів.
        [many] Перевірка { $count } ендпоінтів.
       *[other] Перевірка { $count } ендпоінта.
    }
setup-verify-rpc-waiting = Очікування перевірки ендпоінтів
setup-verify-save-waiting = Очікування збереження
setup-verify-save-running = Шифрування та збереження
setup-verify-save-running-detail = Запис перевіреної конфігурації на цей пристрій.
setup-verify-save-done = Конфігурацію збережено
setup-verify-save-done-detail = Приватний ключ зашифровано; робочі RPC-ендпоінти збережено.
setup-verify-save-failed = Не вдалося зберегти налаштування
setup-verify-save-skipped = Не збережено
setup-verify-request-failed = Запит на перевірку не вдався
setup-verify-summary-checking = Перевірка вашого гаманця та підключень до мейннету Solana.
setup-verify-summary-running = Перевірка саме тих облікових даних, які ви ввели.
setup-verify-summary-saving = Облікові дані перевірено. Безпечне збереження.
setup-verify-summary-failed = Перегляньте проблему й перевірте ще раз.

## Errors

setup-error-credentials-failed = Перевірка облікових даних не вдалася.
setup-error-save-failed = Не вдалося зберегти налаштування.
setup-error-verify-failed = Перевірка не вдалася.
setup-error-explore-failed = Не вдалося запустити режим огляду.
setup-error-gateway-failed = Не вдалося зберегти налаштування шлюзу.
setup-action-review-credentials = Переглянути облікові дані

## Completion

setup-explore-opening = Відкриття режиму огляду…
setup-complete-restarting = Перезапуск { -brand } із вашою перевіреною конфігурацією.
setup-complete-finishing = Завершення перезапуску…
setup-complete-ready = { -brand } готовий. Відкриття панелі керування…
setup-complete-stored = Вашу перевірену конфігурацію безпечно збережено на цьому пристрої.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = Показати приватний ключ
setup-wallet-hide-key = Сховати приватний ключ
setup-wallet-copy =
    .aria-label = Копіювати адресу гаманця
    .title = Копіювати адресу гаманця
setup-wallet-copy-done =
    .aria-label = Адресу гаманця скопійовано
    .title = Скопійовано
setup-wallet-copy-failed =
    .aria-label = Не вдалося скопіювати адресу гаманця
    .title = Не вдалося скопіювати

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = Налаштування гаманця та RPC
setup-dialog-subtitle = Підключіть свій гаманець Solana та преміальний RPC-ендпоінт, щоб увімкнути торгівлю й ончейн-дані в реальному часі. Ваш приватний ключ шифрується на цьому пристрої й ніколи його не залишає.
setup-dialog-close =
    .title = Закрити
    .aria-label = Закрити
setup-dialog-wallet-label = Приватний ключ гаманця
setup-dialog-wallet-input =
    .placeholder = Рядок base58 або масив JSON [1,2,3,...]
setup-dialog-rpc-label = RPC-ендпоінт(и)
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (по одному в рядку)
setup-dialog-rpc-hint = Наполегливо рекомендуємо преміальний провайдер ({ -helius }, { -quicknode }, { -alchemy }) — публічний RPC Solana має обмеження швидкості й може не працювати.
setup-dialog-submit = Перевірити й підключити
setup-dialog-working = Виконується…
setup-dialog-validating = Перевірка…
setup-dialog-saving = Збереження…
setup-dialog-restarting = Перезапуск…
setup-dialog-saved = Налаштування збережено — перезапуск { -brand } у повному режимі…
setup-dialog-error-missing-fields = Введіть приватний ключ гаманця та принаймні одну RPC-адресу.
setup-dialog-error-validation = Перевірка не вдалася.
setup-dialog-error-incomplete = Не вдалося завершити налаштування.
setup-dialog-error-restart-helper = Помічник автоматичного перезапуску недоступний. Незабаром перезавантажте панель керування.
setup-dialog-error-unexpected = Неочікувана помилка.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Перебіг налаштування
setup-wizard-step-credentials = Облікові дані
setup-wizard-step-verification = Перевірка
setup-wizard-step-complete = Завершення
setup-wizard-credentials-title = Налаштуйте облікові дані
setup-wizard-credentials-description = Підключіть локальний гаманець і надійні RPC-ендпоінти мейннету Solana.
setup-wizard-wallet-toggle =
    .title = Показати приватний ключ
    .aria-label = Показати приватний ключ
setup-wizard-wallet-security-note = Шифрується перед збереженням.
setup-wizard-rpc-title = RPC-ендпоінти
setup-wizard-rpc-input =
    .placeholder = Одна HTTPS-адреса в рядку
setup-wizard-rpc-guidance = Для безперервного опитування рекомендовано надійний RPC мейннету.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = рекомендовано
setup-wizard-gateway-title = Безкоштовне надсилання транзакцій
setup-wizard-gateway-hint = Доступно після входу в обліковий запис. Ваш RPC залишається запасним варіантом.
setup-wizard-account-title = Обліковий запис { -brand }
setup-wizard-account-optional = Необов’язково
setup-wizard-account-loading = Перевірка стану облікового запису…
setup-wizard-verify-title = Перевірити й зберегти
setup-wizard-verify-list =
    .aria-label = Стан перевірки налаштувань
setup-wizard-verify-wallet = Гаманець
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = Безпечна конфігурація
setup-wizard-complete-title = Налаштування збережено
setup-wizard-reconnect = Повторити підключення
setup-wizard-reload = Перезавантажити панель керування
setup-wizard-error-title = Налаштування потребують уваги
setup-wizard-explore = Переглянути панель керування
setup-wizard-continue = Продовжити
