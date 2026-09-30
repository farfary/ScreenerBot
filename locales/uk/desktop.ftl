# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = OK

## Splash and loading status.

desktop-splash-starting = Запуск { -brand }
desktop-splash-restarting = Перезапуск { -brand }
desktop-splash-recovering = Відновлення
desktop-splash-opening-dashboard = Відкриття панелі керування
desktop-splash-checking-dependencies = Перевірка залежностей
desktop-splash-installing-dependencies = Встановлення системних залежностей
desktop-splash-installing-dependencies-detail = Для роботи { -brand } потрібен Microsoft Visual C++ Redistributable.
desktop-splash-resetting-wallet = Скидання даних гаманця
desktop-splash-resetting-wallet-detail = Перед очищенням наявні дані гаманця буде збережено в резервній копії.
desktop-splash-updating = Оновлення до v{ $version }
desktop-splash-updating-detail = Ваші налаштування та дані залишаються без змін.
desktop-splash-restoring = Відновлення v{ $version }
desktop-splash-restoring-detail = Оновлення v{ $failed } не запустилося, тож попередня версія перебирає керування.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = Не вдалося запустити { -brand }
desktop-boot-detail-fallback = Бекенд несподівано зупинився.
desktop-boot-remedy-label = Як виправити
desktop-boot-log-file-label = Файл журналу:
desktop-boot-action-reset-wallet = Скинути дані гаманця й перезапустити
desktop-boot-action-working = Виконується...
desktop-boot-action-open-logs = Відкрити теку журналів
desktop-boot-action-copy = Копіювати подробиці
desktop-boot-action-copied = Скопійовано
desktop-boot-action-quit = Вийти
desktop-boot-subtitle-wallet-mismatch = Виявлено інший гаманець
desktop-boot-subtitle-port-in-use = Потрібний мережевий порт зайнятий
desktop-boot-subtitle-lock-held = { -brand } уже запущено
desktop-boot-subtitle-config-invalid = Проблема з конфігурацією
desktop-boot-subtitle-directory-setup = Проблема зі сховищем
desktop-boot-subtitle-generic = Помилка запуску

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = Не вдалося запустити { -brand }
desktop-boot-error-remedy = Відкрийте теку журналів, щоб побачити, що сталося, і перезапустіть застосунок. Якщо проблема не зникає, зверніться до підтримки: t.me/screenerbotio_support.
desktop-boot-error-default = Бекенд несподівано зупинився до того, як панель керування була готова.
desktop-boot-error-restore-failed = Оновлений бекенд зазнав збою, а попередню версію не вдалося відновити ({ $error }).
desktop-boot-error-spawn-failed = Не вдалося запустити програму бекенда ({ $error }).
desktop-boot-error-spawn-missing = Не вдалося запустити програму бекенда. Можливо, її немає або її заблокувало захисне програмне забезпечення.
desktop-boot-error-exited-running = Бекенд зупинився, поки працювала панель керування (код виходу { $code }).
desktop-boot-error-exited-early = Бекенд зупинився до того, як панель керування була готова (код виходу { $code }).
desktop-boot-error-dashboard-load = Не вдалося завантажити панель керування ({ $description }, { $code }).
desktop-boot-error-renderer-gone = Рендерер панелі керування зупинився ({ $reason }).
desktop-boot-error-unresponsive = Панель керування перестала відповідати.
desktop-boot-error-url-failed = Не вдалося завантажити URL панелі керування ({ $error }).
desktop-boot-error-relaunch-setup = Не вдалося повторно запустити бекенд після налаштування.
desktop-boot-error-relaunch-recovery = Не вдалося повторно запустити бекенд для відновлення.
desktop-boot-error-restart-offline = Бекенд не повернувся в мережу після перезапуску.
desktop-boot-error-recovery-offline = Відновлення завершено, але бекенд не став готовим.
desktop-boot-error-start-timeout = Бекенд не встиг завершити запуск. Таке може статися під час повільного першого запуску або якщо інша програма блокує з’єднання.

## System tray.

desktop-tray-tooltip = { -brand } - торговий бот для Solana
desktop-tray-show = Показати { -brand }
desktop-tray-open-dashboard = Відкрити панель керування
desktop-tray-quit = Вийти з { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = Відкрити теку даних
desktop-menu-open-logs-folder = Відкрити теку журналів
desktop-menu-documentation = Документація
desktop-menu-telegram-support = Підтримка в { -telegram }
desktop-menu-check-updates = Перевірити оновлення...

## Application menu.

desktop-menu-file = Файл
desktop-menu-edit = Правка
desktop-menu-view = Вигляд
desktop-menu-window = Вікно
desktop-menu-help = Довідка
desktop-menu-reset-zoom = Скинути масштаб
desktop-menu-zoom-in = Збільшити
desktop-menu-zoom-out = Зменшити
desktop-menu-keyboard-shortcuts = Комбінації клавіш
desktop-menu-telegram-channel = Канал у { -telegram }
desktop-menu-telegram-community = Спільнота в { -telegram }
desktop-menu-follow-x = Стежити в { -x } ({ -twitter })
desktop-menu-visit-website = Відвідати вебсайт
desktop-menu-about = Про { -brand }

## About dialog.

desktop-about-title = Про { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Версія { $version }

    Розширений бот для керування гаманцями Solana та автоматичної торгівлі.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = Комбінації клавіш
desktop-shortcuts-message = Комбінації клавіш { -brand }
desktop-shortcuts-body-mac =
    Комбінації клавіш:

    Керування вікном:
      Cmd+M          Згорнути
      Cmd+W          Закрити вікно
      Cmd+Q          Вийти
      Cmd+Ctrl+F     Повноекранний режим

    Масштаб:
      Cmd++          Збільшити
      Cmd+-          Зменшити
      Cmd+0          Скинути масштаб

    Навігація:
      Cmd+R          Перезавантажити панель керування
      Cmd+Shift+D    Відкрити теку даних

    Інше:
      F1             Відкрити документацію
      Cmd+Alt+I      Інструменти розробника
desktop-shortcuts-body-other =
    Комбінації клавіш:

    Керування вікном:
      Alt+F4         Вийти
      F11            Повноекранний режим

    Масштаб:
      Ctrl++         Збільшити
      Ctrl+-         Зменшити
      Ctrl+0         Скинути масштаб

    Навігація:
      Ctrl+R         Перезавантажити панель керування
      Ctrl+Shift+D   Відкрити теку даних

    Інше:
      F1             Відкрити документацію
      Ctrl+Shift+I   Інструменти розробника

## Close confirmation (Windows and Linux).

desktop-close-title = Закрити { -brand }
desktop-close-message = Що ви хочете зробити?
desktop-close-detail = { -brand } може продовжити роботу у фоновому режимі. Торговий бот і надалі відстежуватиме ринок та торгуватиме, поки згорнутий у системний трей.
desktop-close-minimize = Згорнути в трей
desktop-close-quit = Повністю вийти
desktop-close-cancel = Скасувати

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = Відсутня залежність
desktop-vcredist-missing-message = Відсутній Visual C++ Redistributable
desktop-vcredist-missing-detail = Для роботи { -brand } потрібен Microsoft Visual C++ Redistributable. Встановити його зараз?
desktop-vcredist-install = Встановити й виправити
desktop-vcredist-exit = Вийти
desktop-vcredist-not-found-title = Інсталятор не знайдено
desktop-vcredist-not-found-message = Не вдалося правильно знайти { $name }.
desktop-vcredist-done-title = Встановлення завершено
desktop-vcredist-done-message = Залежності успішно встановлено.
desktop-vcredist-done-detail = Зараз запуститься { -brand }.
desktop-vcredist-failed-title = Помилка встановлення
desktop-vcredist-failed-message = Встановіть Visual C++ Redistributable вручну.
