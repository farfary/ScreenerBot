desktop-action-ok = OK

desktop-splash-starting = Запуск { -brand }
desktop-splash-restarting = Перезапуск { -brand }
desktop-splash-recovering = Восстановление
desktop-splash-opening-dashboard = Открытие дашборда
desktop-splash-checking-dependencies = Проверка зависимостей
desktop-splash-installing-dependencies = Установка системных зависимостей
desktop-splash-installing-dependencies-detail = Для работы { -brand } нужен Microsoft Visual C++ Redistributable.
desktop-splash-resetting-wallet = Сброс данных кошелька
desktop-splash-resetting-wallet-detail = Перед очисткой существующие данные кошелька сохраняются в резервной копии.
desktop-splash-updating = Обновление до v{ $version }
desktop-splash-updating-detail = Ваши настройки и данные останутся без изменений.
desktop-splash-restoring = Восстановление v{ $version }
desktop-splash-restoring-detail = Обновление v{ $failed } не запустилось, поэтому управление возвращается предыдущей версии.

desktop-boot-title-fallback = { -brand } не удалось запустить
desktop-boot-detail-fallback = Бэкенд неожиданно остановился.
desktop-boot-remedy-label = Как исправить
desktop-boot-log-file-label = Файл журнала:
desktop-boot-action-reset-wallet = Сбросить данные кошелька и перезапустить
desktop-boot-action-working = Выполняется...
desktop-boot-action-open-logs = Открыть папку журналов
desktop-boot-action-copy = Скопировать сведения
desktop-boot-action-copied = Скопировано
desktop-boot-action-quit = Выйти
desktop-boot-subtitle-wallet-mismatch = Обнаружен другой кошелёк
desktop-boot-subtitle-port-in-use = Нужный сетевой порт занят
desktop-boot-subtitle-lock-held = { -brand } уже запущен
desktop-boot-subtitle-config-invalid = Проблема с конфигурацией
desktop-boot-subtitle-directory-setup = Проблема с хранилищем
desktop-boot-subtitle-generic = Ошибка запуска

desktop-boot-error-title = { -brand } не удалось запустить
desktop-boot-error-remedy = Откройте папку журналов, чтобы узнать, что произошло, затем перезапустите приложение. Если проблема не исчезнет, обратитесь в поддержку: t.me/screenerbotio_support.
desktop-boot-error-default = Бэкенд неожиданно остановился до готовности дашборда.
desktop-boot-error-restore-failed = Обновлённый бэкенд завершился с ошибкой, а предыдущую версию восстановить не удалось ({ $error }).
desktop-boot-error-spawn-failed = Не удалось запустить программу бэкенда ({ $error }).
desktop-boot-error-spawn-missing = Не удалось запустить программу бэкенда. Возможно, она отсутствует или заблокирована антивирусом.
desktop-boot-error-exited-running = Бэкенд остановился во время работы дашборда (код выхода { $code }).
desktop-boot-error-exited-early = Бэкенд остановился до готовности дашборда (код выхода { $code }).
desktop-boot-error-dashboard-load = Не удалось загрузить дашборд ({ $description }, { $code }).
desktop-boot-error-renderer-gone = Процесс отрисовки дашборда остановился ({ $reason }).
desktop-boot-error-unresponsive = Дашборд перестал отвечать.
desktop-boot-error-url-failed = Не удалось загрузить URL дашборда ({ $error }).
desktop-boot-error-relaunch-setup = Не удалось перезапустить бэкенд после настройки.
desktop-boot-error-relaunch-recovery = Не удалось перезапустить бэкенд для восстановления.
desktop-boot-error-restart-offline = Бэкенд не вернулся в сеть после перезапуска.
desktop-boot-error-recovery-offline = Восстановление завершено, но бэкенд не стал готов к работе.
desktop-boot-error-start-timeout = Бэкенд не успел запуститься вовремя. Так бывает при медленном первом запуске или если другая программа блокирует соединение.

desktop-tray-tooltip = { -brand } - торговый бот для Solana
desktop-tray-show = Показать { -brand }
desktop-tray-open-dashboard = Открыть дашборд
desktop-tray-quit = Выйти из { -brand }

desktop-menu-open-data-folder = Открыть папку данных
desktop-menu-open-logs-folder = Открыть папку журналов
desktop-menu-documentation = Документация
desktop-menu-telegram-support = Поддержка в { -telegram }
desktop-menu-check-updates = Проверить обновления...

desktop-menu-file = Файл
desktop-menu-edit = Правка
desktop-menu-view = Вид
desktop-menu-window = Окно
desktop-menu-help = Справка
desktop-menu-reset-zoom = Сбросить масштаб
desktop-menu-zoom-in = Увеличить
desktop-menu-zoom-out = Уменьшить
desktop-menu-keyboard-shortcuts = Горячие клавиши
desktop-menu-telegram-channel = Канал в { -telegram }
desktop-menu-telegram-community = Сообщество в { -telegram }
desktop-menu-follow-x = Подписаться в { -x } ({ -twitter })
desktop-menu-visit-website = Открыть сайт
desktop-menu-about = О { -brand }

desktop-about-title = О { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Версия { $version }

    Продвинутое управление кошельками Solana и бот для автоматической торговли.

    https://screenerbot.io

    © 2024-2026 { -brand }

desktop-shortcuts-title = Горячие клавиши
desktop-shortcuts-message = Горячие клавиши { -brand }
desktop-shortcuts-body-mac =
    Горячие клавиши:

    Управление окном:
      Cmd+M          Свернуть
      Cmd+W          Закрыть окно
      Cmd+Q          Выйти
      Cmd+Ctrl+F     Полноэкранный режим

    Масштаб:
      Cmd++          Увеличить
      Cmd+-          Уменьшить
      Cmd+0          Сбросить масштаб

    Навигация:
      Cmd+R          Перезагрузить дашборд
      Cmd+Shift+D    Открыть папку данных

    Прочее:
      F1             Открыть документацию
      Cmd+Alt+I      Инструменты разработчика
desktop-shortcuts-body-other =
    Горячие клавиши:

    Управление окном:
      Alt+F4         Выйти
      F11            Полноэкранный режим

    Масштаб:
      Ctrl++         Увеличить
      Ctrl+-         Уменьшить
      Ctrl+0         Сбросить масштаб

    Навигация:
      Ctrl+R         Перезагрузить дашборд
      Ctrl+Shift+D   Открыть папку данных

    Прочее:
      F1             Открыть документацию
      Ctrl+Shift+I   Инструменты разработчика

desktop-close-title = Закрыть { -brand }
desktop-close-message = Что вы хотите сделать?
desktop-close-detail = { -brand } может продолжить работу в фоне. Торговый бот продолжит мониторинг и торговлю, пока приложение свёрнуто в системный трей.
desktop-close-minimize = Свернуть в трей
desktop-close-quit = Выйти полностью
desktop-close-cancel = Отмена

desktop-vcredist-missing-title = Не хватает зависимости
desktop-vcredist-missing-message = Отсутствует Visual C++ Redistributable
desktop-vcredist-missing-detail = Для работы { -brand } нужен Microsoft Visual C++ Redistributable. Установить его сейчас?
desktop-vcredist-install = Установить и исправить
desktop-vcredist-exit = Выйти
desktop-vcredist-not-found-title = Установщик не найден
desktop-vcredist-not-found-message = Не удалось правильно найти { $name }.
desktop-vcredist-done-title = Установка завершена
desktop-vcredist-done-message = Зависимости успешно установлены.
desktop-vcredist-done-detail = Сейчас { -brand } запустится.
desktop-vcredist-failed-title = Ошибка установки
desktop-vcredist-failed-message = Установите Visual C++ Redistributable вручную.
