setup-wallet-required = Введите приватный ключ кошелька.
setup-wallet-json-recognized = Формат JSON-ключа из 64 байт распознан.
setup-wallet-json-invalid = Используйте JSON-массив ровно из 64 значений байтов (0–255).
setup-wallet-format-invalid = Используйте приватный ключ base58 или JSON-массив из 64 байт.
setup-wallet-base58-recognized = Формат ключа base58 распознан.

setup-rpc-required = Введите хотя бы один RPC-эндпоинт.
setup-dialog-rpc-optional-note = Необязательно: оставьте поле пустым, чтобы сохранить уже настроенные RPC-эндпоинты.
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
setup-error-restart-timeout = { -brand } ещё не переподключился.

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
setup-verify-save-skipped = Не сохранено. Устраните проблему выше и проверьте снова.
setup-verify-request-failed = Запрос проверки не выполнен
setup-verify-summary-checking = Проверка вашего кошелька и подключений к Solana mainnet.
setup-verify-summary-running = Проверка именно тех данных, которые вы ввели.
setup-verify-summary-saving = Данные проверены. Безопасное сохранение.
setup-verify-summary-failed = Разберитесь с проблемой и повторите проверку.

setup-error-credentials-failed = Не удалось проверить учётные данные.
setup-error-save-failed = Не удалось сохранить настройку.
setup-error-verify-failed = Проверка не удалась.
setup-error-explore-failed = Не удалось запустить режим обзора.
setup-error-gateway-failed = Не удалось сохранить настройку отправки транзакций.
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
setup-dialog-submit = Проверить и подключить
setup-dialog-working = Выполняется…
setup-dialog-validating = Проверка…
setup-dialog-saving = Сохранение…
setup-dialog-restarting = Перезапуск…
setup-dialog-saved = Настройка сохранена — перезапуск { -brand } в полном режиме…
setup-dialog-error-validation = Проверка не удалась.
setup-dialog-error-incomplete = Не удалось завершить настройку.
setup-dialog-error-restart-helper = Помощник автоматического перезапуска недоступен. Скоро перезагрузите дашборд.
setup-dialog-error-unexpected = Непредвиденная ошибка.
setup-dialog-replace-title = Заменить торговый кошелёк
setup-dialog-replace-subtitle = Торгуйте с другим кошельком. Его приватный ключ шифруется на этом устройстве и никогда его не покидает.
setup-dialog-replace-submit = Проверить и заменить
setup-dialog-replace-saved = Замена подготовлена — перезапуск { -brand } с новым кошельком…
setup-dialog-replace-confirm-intro = Замена торгового кошелька:
setup-dialog-replace-confirm-stop = Останавливает трейдер и копи-трейдинг. Включите их снова после перезапуска.
setup-dialog-replace-confirm-backup = Переносит позиции, транзакции и историю текущего кошелька в папку резервных копий.
setup-dialog-replace-confirm-keep = Сохраняет ключ текущего кошелька в разделе «Другие кошельки», где его по-прежнему можно экспортировать.
setup-dialog-replace-confirm-funds = Средства не переводятся: токены и { -sol } остаются в текущем кошельке.
setup-dialog-replace-confirm-rpc = Заменяет настроенные RPC-эндпоинты введёнными; если поле пустое, текущие эндпоинты сохраняются.
setup-dialog-replace-confirm-restart = Перезапускает { -brand } с новым кошельком.

setup-wizard-progress =
    .aria-label = Ход настройки
setup-wizard-wallet-security-note = Шифруется перед сохранением.
setup-wizard-rpc-title = RPC-эндпоинты
setup-wizard-rpc-input =
    .placeholder = Один HTTPS-адрес на строку
setup-wizard-rpc-guidance = Используйте надёжного провайдера mainnet; публичные эндпоинты не поддерживаются.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = рекомендуется
setup-wizard-account-title = Аккаунт { -brand }
setup-wizard-account-optional = Необязательно
setup-wizard-account-loading = Проверка статуса аккаунта…
setup-wizard-verify-title = Проверка и сохранение
setup-wizard-verify-list =
    .aria-label = Статус проверки настройки
setup-wizard-verify-wallet = Кошелёк
setup-wizard-verify-save = Безопасная конфигурация
setup-wizard-complete-title = Настройка сохранена
setup-wizard-reconnect = Повторить подключение
setup-wizard-reload = Перезагрузить дашборд
setup-wizard-error-title = Настройка требует внимания
setup-wizard-explore = Открыть дашборд
setup-wizard-continue = Продолжить

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Кошелёк
setup-wizard-step-connection = Подключение
setup-wizard-step-verify = Проверка
setup-wizard-step-ready = Готово
setup-wallet-title = Настройте торговый кошелёк
setup-wallet-description = Кошелёк, которым торгует { -brand }. Его ключ шифруется на этом компьютере и никогда его не покидает.
setup-wallet-source =
    .aria-label = Источник кошелька
setup-wallet-create-title = Создать новый кошелёк
setup-wallet-create-detail = { -brand } создаёт новый кошелёк. Вы сохраняете его ключ, затем отправляете на него { -sol }.
setup-wallet-import-title = Импортировать кошелёк
setup-wallet-import-detail = Используйте уже имеющийся приватный ключ (base58 или JSON-массив из 64 чисел).
setup-wallet-creating = Создаём кошелёк…
setup-wallet-create-failed = Не удалось создать кошелёк. Попробуйте ещё раз.
setup-backup-title = Сохраните приватный ключ
setup-backup-description = Тот, у кого есть этот ключ, управляет кошельком. { -brand } не может его восстановить.
setup-backup-address = Адрес
setup-backup-key = Приватный ключ
setup-backup-copy-address = Скопировать адрес
setup-backup-download = Скачать
setup-backup-copied = Скопировано. Очистите буфер обмена после сохранения.
setup-backup-machine-warning = Зашифрованная копия, которую хранит { -brand }, открывается только на этом компьютере. Если компьютер потерян или заменён, восстановить кошелёк можно только ключом, который вы сохраните сейчас. Храните его офлайн, например на бумаге или в менеджере паролей.
setup-backup-ack-saved = Я сохранил приватный ключ в надёжном месте за пределами этого компьютера.
setup-backup-ack-control = Я понимаю, что любой, у кого есть этот ключ, может забрать средства из этого кошелька.
setup-backup-ack-recovery = Я понимаю, что { -brand } не может восстановить этот ключ. Без него потеря компьютера означает потерю средств.
setup-backup-create-another = Создать другой
setup-backup-replace-confirm = Отказаться от этого ключа и создать новый кошелёк? Делайте это, только если ещё ничего на него не отправляли.
setup-connection-title = Как { -brand } будет подключаться к { -solana }?
setup-connection-description = Это можно изменить позже в разделе Настройки → Подключение.
setup-connection-choice =
    .aria-label = Подключение
setup-connection-service-title = Сервис { -brand }
setup-connection-service-badge = RPC не нужен
setup-connection-service-detail = Используется ваш бесплатный аккаунт { -brand }.
setup-connection-service-prices = Цены обновляются раз в несколько секунд
setup-connection-service-features = Для копитрейдинга, отслеживания кошельков, балансов других кошельков, старой истории, прямых свопов в пулах и живых цен пулов нужен свой RPC
setup-connection-service-pause = Торговля приостанавливается, если вы выйдете из аккаунта или screenerbot.io недоступен
setup-connection-service-privacy = Ваш закрытый ключ никогда не покидает этот компьютер: свопы подписываются здесь, а screenerbot.io использует только публичный адрес кошелька, чтобы читать его балансы и транзакции
setup-connection-own-title = Свой RPC
setup-connection-own-badge = Быстрее всего
setup-connection-own-detail = Цены примерно дважды в секунду и все функции. Нужен RPC-эндпоинт от провайдера.
setup-connection-signin-required = Войдите или создайте бесплатный аккаунт, чтобы пользоваться сервисом { -brand }.
setup-connection-reauthorize = Войдите снова, чтобы разрешить сервису { -brand } запросы кошелька.
setup-connection-sending = Отправлять транзакции через { -brand }
setup-connection-sending-hint = Доступно после входа. Когда это невозможно, используется ваш RPC.
setup-wizard-account-required = Обязательно
setup-wizard-verify-connection = Подключение
setup-verify-service-checking = Проверяем сервис { -brand }
setup-verify-service-ready = Сервис { -brand } готов
setup-verify-service-busy = Сервис { -brand } сейчас занят. Повторите через минуту или используйте свой RPC.
setup-verify-service-daily-limit = Ваш аккаунт исчерпал сегодняшний лимит сервиса { -brand }. Повторите после 00:00 UTC или используйте свой RPC.
setup-verify-service-unreachable = screenerbot.io недоступен. Проверьте подключение к интернету.
setup-verify-service-version = Сервис { -brand } больше не поддерживает эту версию. Обновите { -brand }, чтобы продолжить.
setup-verify-service-signin = Войдите, чтобы пользоваться сервисом { -brand }.
setup-verify-save-done-service-detail = Приватный ключ зашифрован; выбран сервис { -brand }.
setup-action-try-again = Повторить
setup-action-use-own-rpc = Использовать свой RPC
setup-action-sign-in-again = Войти снова
setup-ready-title = { -brand } готов
setup-fund-title = Пополните торговый кошелёк
setup-fund-waiting = Ожидание { -sol }…
setup-fund-received = Получено { $amount } { -sol }
setup-fund-balance-unavailable = Читаем баланс… Он обновится сам.
setup-fund-open-dashboard = Открыть панель
