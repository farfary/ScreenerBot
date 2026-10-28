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
setup-dialog-rpc-optional-note = Необов'язково: залиште поле порожнім, щоб зберегти вже налаштовані RPC-ендпоінти.
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
setup-error-restart-timeout = { -brand } ще не перепідключився.

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
setup-verify-save-skipped = Не збережено. Усуньте проблему вище й перевірте ще раз.
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
setup-error-gateway-failed = Не вдалося зберегти налаштування надсилання транзакцій.
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

setup-dialog-close =
    .title = Закрити
    .aria-label = Закрити
setup-dialog-wallet-label = Приватний ключ гаманця
setup-dialog-wallet-input =
    .placeholder = Рядок base58 або масив JSON [1,2,3,...]
setup-dialog-working = Виконується…
setup-dialog-validating = Перевірка…
setup-dialog-saving = Збереження…
setup-dialog-restarting = Перезапуск…
setup-dialog-error-validation = Перевірка не вдалася.
setup-dialog-error-incomplete = Не вдалося завершити налаштування.
setup-dialog-error-restart-helper = Помічник автоматичного перезапуску недоступний. Незабаром перезавантажте панель керування.
setup-dialog-error-unexpected = Неочікувана помилка.
setup-dialog-replace-title = Замінити торговий гаманець
setup-dialog-replace-subtitle = Торгуйте з іншим гаманцем. Його приватний ключ шифрується на цьому пристрої й ніколи його не залишає.
setup-dialog-replace-submit = Перевірити й замінити
setup-dialog-replace-saved = Заміну підготовлено — перезапуск { -brand } з новим гаманцем…
setup-dialog-replace-confirm-intro = Заміна торгового гаманця:
setup-dialog-replace-confirm-stop = Зупиняє трейдер і копітрейдинг. Увімкніть їх знову після перезапуску.
setup-dialog-replace-confirm-backup = Переносить позиції, транзакції та історію поточного гаманця до папки резервних копій.
setup-dialog-replace-confirm-keep = Зберігає ключ поточного гаманця в розділі «Інші гаманці», де його й далі можна експортувати.
setup-dialog-replace-confirm-funds = Кошти не переказуються: токени та { -sol } залишаються в поточному гаманці.
setup-dialog-replace-confirm-rpc = Замінює налаштовані RPC-ендпоінти введеними; якщо поле порожнє, поточні ендпоінти залишаються.
setup-dialog-replace-confirm-restart = Перезапускає { -brand } з новим гаманцем.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Перебіг налаштування
setup-wizard-wallet-security-note = Шифрується перед збереженням.
setup-wizard-rpc-title = RPC-ендпоінти
setup-wizard-rpc-input =
    .placeholder = Одна HTTPS-адреса в рядку
setup-wizard-rpc-guidance = Використовуйте надійного провайдера мейннету; публічні ендпоінти не підтримуються.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = рекомендовано
setup-wizard-account-title = Обліковий запис { -brand }
setup-wizard-account-optional = Необов’язково
setup-wizard-account-loading = Перевірка стану облікового запису…
setup-wizard-verify-title = Перевірити й зберегти
setup-wizard-verify-list =
    .aria-label = Стан перевірки налаштувань
setup-wizard-verify-wallet = Гаманець
setup-wizard-verify-save = Безпечна конфігурація
setup-wizard-complete-title = Налаштування збережено
setup-wizard-reconnect = Повторити підключення
setup-wizard-reload = Перезавантажити панель керування
setup-wizard-error-title = Налаштування потребують уваги
setup-wizard-explore = Переглянути панель керування
setup-wizard-continue = Продовжити

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Гаманець
setup-wizard-step-connection = Підключення
setup-wizard-step-verify = Перевірка
setup-wizard-step-ready = Готово
setup-wallet-title = Налаштуйте торговий гаманець
setup-wallet-description = Гаманець, яким торгує { -brand }. Його ключ шифрується на цьому комп’ютері й ніколи його не залишає.
setup-wallet-source =
    .aria-label = Джерело гаманця
setup-wallet-create-title = Створити новий гаманець
setup-wallet-create-detail = { -brand } створює новий гаманець. Ви зберігаєте його ключ, а потім надсилаєте на нього { -sol }.
setup-wallet-import-title = Імпортувати гаманець
setup-wallet-import-detail = Використайте наявний приватний ключ (base58 або JSON-масив із 64 чисел).
setup-wallet-creating = Створюємо гаманець…
setup-wallet-create-failed = Не вдалося створити гаманець. Спробуйте ще раз.
setup-backup-title = Збережіть приватний ключ
setup-backup-description = Хто має цей ключ, той керує гаманцем. { -brand } не може його відновити.
setup-backup-address = Адреса
setup-backup-key = Приватний ключ
setup-backup-copy-address = Скопіювати адресу
setup-backup-download = Завантажити
setup-backup-copied = Скопійовано. Очистіть буфер обміну після збереження.
setup-backup-machine-warning = Зашифрована копія, яку зберігає { -brand }, відкривається лише на цьому комп’ютері. Якщо комп’ютер втрачено або замінено, відновити гаманець можна лише ключем, який ви збережете зараз. Зберігайте його офлайн, наприклад на папері чи в менеджері паролів.
setup-backup-ack-saved = Я зберіг приватний ключ у надійному місці поза цим комп’ютером.
setup-backup-ack-control = Я розумію, що будь-хто з цим ключем може забрати кошти з цього гаманця.
setup-backup-ack-recovery = Я розумію, що { -brand } не може відновити цей ключ. Без нього втрата комп’ютера означає втрату коштів.
setup-backup-create-another = Створити інший
setup-backup-replace-confirm = Відмовитися від цього ключа й створити новий гаманець? Робіть це, лише якщо ще нічого на нього не надсилали.
setup-connection-title = Як { -brand } підключатиметься до { -solana }?
setup-connection-description = Це можна змінити пізніше в розділі Налаштування → Підключення.
setup-connection-choice =
    .aria-label = Підключення
setup-connection-service-title = Сервіс { -brand }
setup-connection-service-badge = RPC не потрібен
setup-connection-service-detail = Використовується ваш безкоштовний акаунт { -brand }.
setup-connection-service-prices = Ціни оновлюються раз на кілька секунд
setup-connection-service-features = Для копітрейдингу, відстеження гаманців, балансів інших гаманців, старішої історії, прямих свопів у пулах і живих цін пулів потрібен власний RPC
setup-connection-service-pause = Торгівля призупиняється, якщо ви вийдете з акаунта або screenerbot.io недоступний
setup-connection-service-privacy = Ваш приватний ключ ніколи не залишає цей комп’ютер: свопи підписуються тут, а screenerbot.io використовує лише публічну адресу гаманця, щоб читати його баланси й транзакції
setup-connection-own-title = Власний RPC
setup-connection-own-badge = Найшвидше
setup-connection-own-detail = Ціни приблизно двічі на секунду й усі функції. Потрібен RPC-ендпоінт від провайдера.
setup-connection-signin-required = Увійдіть або створіть безкоштовний акаунт, щоб користуватися сервісом { -brand }.
setup-connection-reauthorize = Увійдіть знову, щоб дозволити сервісу { -brand } запити гаманця.
setup-connection-sending = Надсилати транзакції через { -brand }
setup-connection-sending-hint = Доступно після входу. Коли це неможливо, використовується ваш RPC.
setup-wizard-account-required = Обов’язково
setup-wizard-verify-connection = Підключення
setup-verify-service-checking = Перевіряємо сервіс { -brand }
setup-verify-service-ready = Сервіс { -brand } готовий
setup-verify-service-busy = Сервіс { -brand } зараз зайнятий. Повторіть за хвилину або використайте власний RPC.
setup-verify-service-daily-limit = Ваш акаунт вичерпав сьогоднішній ліміт сервісу { -brand }. Повторіть після 00:00 UTC або використайте власний RPC.
setup-verify-service-unreachable = screenerbot.io недоступний. Перевірте підключення до інтернету.
setup-verify-service-version = Сервіс { -brand } більше не підтримує цю версію. Оновіть { -brand }, щоб продовжити.
setup-verify-service-signin = Увійдіть, щоб користуватися сервісом { -brand }.
setup-verify-save-done-service-detail = Приватний ключ зашифровано; вибрано сервіс { -brand }.
setup-action-try-again = Повторити
setup-action-use-own-rpc = Використати власний RPC
setup-action-sign-in-again = Увійти знову
setup-ready-title = { -brand } готовий
setup-fund-title = Поповніть торговий гаманець
setup-fund-waiting = Очікування { -sol }…
setup-fund-received = Отримано { $amount } { -sol }
setup-fund-balance-unavailable = Зчитуємо баланс… Він оновиться сам.
setup-fund-open-dashboard = Відкрити панель
