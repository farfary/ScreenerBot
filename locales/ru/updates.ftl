updates-defer-automatic-install-disabled = Автоматическая установка отключена. Обновление готово и будет применено, когда вы решите.
updates-defer-trading-active = Выполняется операция с позицией, сделкой или инструментом, поэтому перезапуск отложен. Обновление применится автоматически, когда приложение освободится.
updates-defer-needs-installer = Этот выпуск также обновляет оболочку приложения, поэтому нужно один раз запустить установщик.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

updates-download-started = Загрузка обновления v{ $version }...
updates-apply-started = Установка обновления. { -brand } перезапустится и переподключится автоматически.
updates-install-opened = Проверенный установщик обновления открыт. Завершите работу установщика операционной системы.

updates-installer-toast-title = Установщик открыт
updates-installer-toast-message = { -brand } сейчас корректно завершит работу.

updates-tab-status = Статус
updates-tab-release-notes = Примечания к выпуску
updates-tab-preferences = Параметры
updates-tab-sections = Разделы обновлений
updates-checking-installation = Проверка этой установки...

updates-phase-idle-headline = Готово к проверке обновлений
updates-phase-idle-detail = Установлен { -brand } v{ $version }.
updates-phase-up-to-date-headline = У вас последняя версия
updates-phase-up-to-date-detail = { -brand } v{ $version } — последняя версия.
updates-phase-checking-headline = Проверка обновлений
updates-phase-checking-detail = Поиск последнего опубликованного выпуска.
updates-phase-available-headline = Доступна версия { $version }
updates-phase-downloading-headline = Загрузка v{ $version }
updates-phase-verifying-headline = Проверка v{ $version }
updates-phase-verifying-detail = Сверка загрузки с опубликованной контрольной суммой.
updates-phase-ready-to-apply-headline = Версия { $version } готова
updates-phase-ready-to-apply-detail = Обновление можно установить сейчас с коротким перезапуском или автоматически при следующем запуске.
updates-phase-ready-to-install-headline = Версия { $version } готова
updates-phase-ready-to-install-detail = Установщик приложения готов завершить это обновление.
updates-phase-applying-headline = Установка обновления
updates-phase-applying-detail = { -brand } перезапускается на новую версию.
updates-phase-applied-headline = Обновлено до v{ $version }
updates-phase-applied-detail = Обновление установлено. Больше ничего не требуется.
updates-phase-failed-headline = Обновление не завершено
updates-phase-failed-detail = Повторите обновление.
updates-phase-check-failed-headline = Не удалось проверить обновления
updates-phase-check-failed-detail = Не удалось связаться со службой выпусков.
updates-status-unavailable-headline = Статус обновления недоступен
updates-phase-unrecognized-detail = Сообщённое состояние обновления не распознано.
updates-status-load-failed-detail = Не удалось загрузить статус установки.

updates-kind-core = Обновление ядра · { $size } · короткий перезапуск
updates-kind-full = Обновление приложения · { $size } · нужен установщик
updates-size-unknown = размер неизвестен

updates-action-check-now = Проверить сейчас
updates-action-check-again = Проверить снова
updates-action-try-again = Повторить
updates-action-download = Скачать обновление
updates-action-restart = Перезапустить для обновления
updates-action-open-installer = Открыть установщик

updates-busy-checking = Проверка...
updates-busy-resuming = Возобновление загрузки...
updates-busy-starting-download = Запуск загрузки...
updates-busy-restarting = Перезапуск...
updates-busy-opening-installer = Открытие установщика...

updates-progress-downloading = Загрузка обновления
updates-progress-verifying = Проверка обновления
updates-progress-transferred = { $done } из { $total }
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Сведения об установке
updates-detail-installed-version = Установленная версия
updates-detail-system = Система
updates-detail-last-checked = Последняя проверка
updates-detail-never = Никогда
updates-detail-available-version = Доступная версия
updates-detail-download-size = Размер загрузки

updates-version-installed = Установлена
updates-version-available = Доступна

updates-notes-highlights = Главное
updates-notes-empty-title = Примечаний к выпуску пока нет
updates-notes-empty-error = Не удалось загрузить историю выпусков. Проверьте соединение и повторите попытку.
updates-notes-empty-none = Примечания к выпуску появятся здесь после публикации выпуска.
updates-notes-history-notice = Показано то, что уже известно этой установке — историю выпусков загрузить не удалось.
updates-release-empty = Для этого выпуска изменения не указаны.
updates-release-changes =
    { $count ->
        [one] { $count } изменение
        [few] { $count } изменения
        [many] { $count } изменений
       *[other] { $count } изменения
    }

updates-preferences-unavailable-title = Параметры обновления недоступны
updates-preferences-unavailable-detail = Не удалось загрузить конфигурацию обновлений.
updates-preference-fallback-name = параметр обновления
updates-preference-save-failed = Не удалось сохранить: { $preference }

updates-request-failed = Запрос не выполнен
updates-check-request-failed = Не удалось проверить обновления
updates-resume-failed = Не удалось возобновить загрузку обновления
updates-download-failed = Не удалось начать загрузку обновления
updates-apply-failed = Не удалось установить обновление
updates-install-failed = Не удалось открыть установщик обновления
updates-apply-confirm-title = Установить v{ $version }
updates-apply-confirm-message = { -brand } перезапустится на новую версию. Торговля остановится на несколько секунд и возобновится автоматически; открытые позиции не затрагиваются.
updates-install-confirm-title = Запустить установщик
updates-install-confirm-message = Откроется проверенный установщик, а { -brand } корректно завершит работу. Завершите установку и снова откройте { -brand }.

updates-version-number = v{ $version }
