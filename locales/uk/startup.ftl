## Wallet mismatch.

startup-wallet-mismatch-title = Гаманець змінено
startup-wallet-mismatch-detail =
    Гаманець у вашій конфігурації не збігається з гаманцем, записаним у локальній історії цього комп’ютера.

    Поточний гаманець: { $current }
    Попередній гаманець: { $stored }

    Уражені локальні дані: { $systems }

    Зазвичай це трапляється після імпорту іншого приватного ключа або відновлення іншої конфігурації. Торгівля, позиції та історія належать попередньому гаманцю й мають бути очищені, перш ніж новий гаманець зможе безпечно запуститися.
startup-wallet-mismatch-systems-default = Транзакції, позиції, історія гаманця
startup-wallet-mismatch-remedy =
    Очистьте локальну історію попереднього гаманця, щоб продовжити (спершу автоматично створюються резервні копії ваших баз даних):

      - У застосунку: виберіть «{ $action }» нижче.
      - З терміналу: виконайте  screenerbot --clean-wallet-data

    Кошти в блокчейні не зачіпаються; скидається лише локальна історія угод і позицій цього комп’ютера. Резервні копії записуються в:
      { $path }
startup-recovery-reset-wallet = Скинути дані гаманця й перезапустити

## Port in use.

startup-port-in-use-title = Мережевий порт зайнятий
startup-port-in-use-detail = Порт панелі керування { $address } уже використовується.
startup-port-in-use-remedy = Інша програма використовує порт, потрібний { -brand }. Закрийте цю програму або змініть порт вебсервера в налаштуваннях, а потім запустіть { -brand } знову.

## Another instance is running.

startup-lock-held-title = { -brand } уже запущено
startup-lock-held-detail = На цьому комп’ютері вже працює інша копія { -brand }, тому друга не може запуститися.
startup-lock-held-remedy = Перейдіть до вже відкритого вікна. Якщо його немає, завершіть будь-який фоновий процес { -brand } і спробуйте ще раз. Якщо проблема не зникає після перезавантаження, файл блокування міг залишитися застарілим, і його можна видалити з теки даних (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Не вдалося прочитати конфігурацію
startup-config-parse-detail = Не вдалося розібрати config.toml: { $detail }
startup-config-load-parse-detail = Не вдалося завантажити конфігурацію: не вдалося розібрати config.toml: { $detail }
startup-config-parse-remedy = Не вдалося прочитати файл конфігурації. Відновіть резервну копію з теки даних або скиньте конфігурацію до типової й налаштуйте гаманець та RPC знову.
startup-config-load-parse-remedy = Відновіть дійсну конфігурацію або завершіть налаштування знову.
startup-option-invalid-title = Недійсний параметр запуску
startup-option-invalid-remedy = Параметр командного рядка недійсний. Запустіть { -brand } без цього параметра або виправте його й спробуйте ще раз.

## Storage upgrade.

startup-storage-upgrade-title = Не вдалося оновити ваші дані
startup-storage-upgrade-detail =
    { -brand } не зміг оновити { $database } до цієї версії й зупинився, нічого не змінивши. Ваші дані не змінено.

    Причина:
    { $error }
startup-storage-upgrade-remedy = Скопіюйте подробиці й надішліть їх разом із файлом журналу до підтримки: t.me/screenerbotio_support. Не редагуйте, не переміщуйте й не видаляйте базу даних: { -brand } знову відкриє її після встановлення виправлення.

## Wallet replacement and wallet data reset.

startup-wallet-replacement-incomplete-title = Не вдалося завершити заміну гаманця
startup-wallet-replacement-incomplete-detail =
    { -brand } зупинився під час переходу на новий торговий гаманець. Нічого не видалено: ключ попереднього гаманця залишається у сховищі гаманців, а його локальна історія або на своєму місці, або в папці резервної копії, вказаній у файлі журналу.

    Причина:
    { $error }
startup-wallet-replacement-incomplete-remedy = Запустіть { -brand } знову, щоб продовжити заміну. Якщо повідомлення з'явиться знову, скопіюйте подробиці та надішліть їх разом із файлом журналу до підтримки: t.me/screenerbotio_support. Не редагуйте, не переміщуйте й не видаляйте wallet_replacement.json і папку резервних копій.
startup-wallet-reset-refused-title = Дані гаманця не скинуто
startup-wallet-reset-refused-open-positions = Торговий гаманець має відкриті позиції ({ $count }), тому дані гаманця не скинуто.
startup-wallet-reset-refused-positions-unreadable = Не вдалося прочитати сховище позицій, тож відкриті позиції не можна виключити; локальну історію гаманця збережено.
startup-wallet-reset-refused-remedy = Нічого не переміщено. Запустіть { -brand } без скидання, закрийте або архівуйте відкриті позиції торгового гаманця, а потім знову скиньте дані гаманця.
startup-wallet-reset-refused-unreadable-remedy = Нічого не переміщено. Запустіть { -brand } без скидання, щоб він перевірив сховище позицій, а потім знову скиньте дані гаманця. Якщо повідомлення з'явиться знову, надішліть файл журналу до підтримки: t.me/screenerbotio_support.
startup-wallet-reset-incomplete-title = Не вдалося завершити скидання даних гаманця
startup-wallet-reset-incomplete-detail =
    { -brand } зупинився під час перенесення даних гаманця до { $directory } і не зміг повернути всі файли. Частина локальної історії гаманця в цій папці, а частина залишилася на своєму місці.

    Причина:
    { $error }
startup-wallet-reset-incomplete-remedy = Не запускайте { -brand } знову й не переміщуйте, не редагуйте та не видаляйте жодних файлів у { $directory }. Скопіюйте подробиці та надішліть їх разом із файлом журналу до підтримки: t.me/screenerbotio_support.

## Generic failures.

startup-generic-title = Не вдалося запустити { -brand }
startup-generic-remedy = Докладніше дивіться у файлі журналу, а потім перезапустіть застосунок. Якщо проблема не зникає, зверніться до підтримки: t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Не вдалося створити потрібні каталоги: { $error }
startup-failure-config-load = Не вдалося завантажити конфігурацію: { $error }
startup-failure-actions-sync = Не вдалося синхронізувати дії з бази даних: { $error }
startup-failure-analysis-init = Не вдалося ініціалізувати рушій аналізу: { $error }
startup-failure-assistant-init = Не вдалося ініціалізувати рушій чату асистента: { $error }
startup-failure-wallets-init = Не вдалося ініціалізувати гаманці: { $error }
startup-failure-wallet-validation = Не вдалося перевірити узгодженість гаманця: { $error }
startup-failure-storage-phase = Етап сховища несподівано зупинився: { $error }
