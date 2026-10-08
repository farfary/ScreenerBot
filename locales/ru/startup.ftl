## Wallet mismatch.

startup-wallet-mismatch-title = Кошелёк изменился
startup-wallet-mismatch-detail =
    Кошелёк в конфигурации не совпадает с кошельком, записанным в локальной истории этого компьютера.

    Текущий кошелёк: { $current }
    Предыдущий кошелёк: { $stored }

    Затронутые локальные данные: { $systems }

    Обычно так бывает после импорта другого приватного ключа или восстановления другой конфигурации. Торговля, позиции и история относятся к предыдущему кошельку и должны быть очищены, прежде чем новый кошелёк сможет безопасно запуститься.
startup-wallet-mismatch-systems-default = Транзакции, позиции, история кошелька
startup-wallet-mismatch-remedy =
    Чтобы продолжить, очистите локальную историю предыдущего кошелька (перед этим автоматически создаётся резервная копия баз данных):

      - В приложении: нажмите «{ $action }» ниже.
      - Из терминала: выполните  screenerbot --clean-wallet-data

    Средства в блокчейне не затрагиваются; сбрасывается только локальная история сделок и позиций на этом компьютере. Резервные копии сохраняются в:
      { $path }
startup-recovery-reset-wallet = Сбросить данные кошелька и перезапустить

## Port in use.

startup-port-in-use-title = Сетевой порт занят
startup-port-in-use-detail = Порт дашборда { $address } уже используется.
startup-port-in-use-remedy = Другая программа занимает порт, который нужен { -brand }. Закройте её или измените порт веб-сервера в настройках, затем снова запустите { -brand }.

## Another instance is running.

startup-lock-held-title = { -brand } уже запущен
startup-lock-held-detail = Другая копия { -brand } уже работает на этом компьютере, поэтому вторую запустить нельзя.
startup-lock-held-remedy = Переключитесь на уже открытое окно. Если его нет, завершите фоновый процесс { -brand } и повторите попытку. Если после перезагрузки проблема не исчезла, файл блокировки мог устареть: его можно удалить из папки данных (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Не удалось прочитать конфигурацию
startup-config-parse-detail = Не удалось разобрать config.toml: { $detail }
startup-config-load-parse-detail = Не удалось загрузить конфигурацию: не удалось разобрать config.toml: { $detail }
startup-config-parse-remedy = Файл конфигурации не удалось прочитать. Восстановите резервную копию из папки данных или сбросьте конфигурацию к значениям по умолчанию и заново настройте кошелёк и RPC.
startup-config-load-parse-remedy = Восстановите корректную конфигурацию или заново пройдите настройку.
startup-option-invalid-title = Недопустимый параметр запуска
startup-option-invalid-remedy = Параметр командной строки недопустим. Запустите { -brand } без этого параметра или исправьте его и повторите попытку.

## Storage upgrade.

startup-storage-upgrade-title = Не удалось обновить ваши данные
startup-storage-upgrade-detail =
    { -brand } не смог обновить { $database } до этой версии и остановился, ничего не изменив. Ваши данные не изменены.

    Причина:
    { $error }
startup-storage-upgrade-remedy = Скопируйте подробности и отправьте их вместе с файлом журнала в поддержку: t.me/screenerbotio_support. Не редактируйте, не перемещайте и не удаляйте базу данных: { -brand } снова откроет её после установки исправления.

## Generic failures.

startup-generic-title = Не удалось запустить { -brand }
startup-generic-remedy = Подробности смотрите в файле журнала, затем перезапустите приложение. Если проблема сохраняется, обратитесь в поддержку: t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Не удалось создать необходимые папки: { $error }
startup-failure-config-load = Не удалось загрузить конфигурацию: { $error }
startup-failure-actions-init = Не удалось инициализировать базу данных действий: { $error }
startup-failure-actions-sync = Не удалось синхронизировать действия из базы данных: { $error }
startup-failure-strategy-init = Не удалось инициализировать систему стратегий: { $error }
startup-failure-analysis-init = Не удалось инициализировать движок анализа: { $error }
startup-failure-assistant-init = Не удалось инициализировать чат-движок ассистента: { $error }
startup-failure-wallets-init = Не удалось инициализировать кошельки: { $error }
startup-failure-wallet-validation = Не удалось проверить согласованность кошелька: { $error }
