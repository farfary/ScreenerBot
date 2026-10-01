# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = Автоматичне встановлення вимкнено. Оновлення готове й буде застосовано, коли ви вирішите.
updates-defer-trading-active = Активна позиція, угода або операція інструмента, тому перезапуск відкладено. Оновлення застосується автоматично, коли застосунок буде вільний.
updates-defer-needs-installer = Цей випуск також оновлює оболонку застосунку, тому інсталятор потрібно запустити один раз.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = Завантаження оновлення v{ $version }...
updates-apply-started = Встановлення оновлення. { -brand } перезапуститься й перепідключиться автоматично.
updates-install-opened = Інсталятор перевіреного оновлення відкрито. Завершіть роботу інсталятора операційної системи.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = Інсталятор відкрито
updates-installer-toast-message = { -brand } зараз коректно завершить роботу.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = Статус
updates-tab-release-notes = Примітки до випуску
updates-tab-preferences = Параметри
updates-tab-sections = Розділи оновлень
updates-checking-installation = Перевірка цієї інсталяції...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = Готово до перевірки оновлень
updates-phase-idle-detail = Встановлено { -brand } v{ $version }.
updates-phase-up-to-date-headline = У вас найновіша версія
updates-phase-up-to-date-detail = { -brand } v{ $version } — остання версія.
updates-phase-checking-headline = Перевірка оновлень
updates-phase-checking-detail = Пошук останнього опублікованого випуску.
updates-phase-available-headline = Доступна версія { $version }
updates-phase-downloading-headline = Завантаження v{ $version }
updates-phase-verifying-headline = Перевірка v{ $version }
updates-phase-verifying-detail = Звірка завантаженого файлу з опублікованою контрольною сумою.
updates-phase-ready-to-apply-headline = Версія { $version } готова
updates-phase-ready-to-apply-detail = Оновлення можна встановити зараз із коротким перезапуском або автоматично під час наступного запуску.
updates-phase-ready-to-install-headline = Версія { $version } готова
updates-phase-ready-to-install-detail = Інсталятор для настільної версії готовий завершити це оновлення.
updates-phase-applying-headline = Встановлення оновлення
updates-phase-applying-detail = { -brand } перезапускається на нову версію.
updates-phase-applied-headline = Оновлено до v{ $version }
updates-phase-applied-detail = Оновлення встановлено. Більше нічого не потрібно.
updates-phase-failed-headline = Оновлення не завершено
updates-phase-failed-detail = Спробуйте оновити ще раз.
updates-phase-check-failed-headline = Не вдалося перевірити оновлення
updates-phase-check-failed-detail = Не вдалося з’єднатися зі службою випусків.
updates-status-unavailable-headline = Статус оновлення недоступний
updates-phase-unrecognized-detail = Повідомлений стан оновлення не розпізнано.
updates-status-load-failed-detail = Не вдалося завантажити статус інсталяції.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = Оновлення ядра · { $size } · короткий перезапуск
updates-kind-full = Оновлення настільної версії · { $size } · потрібен інсталятор
updates-size-unknown = розмір невідомий

updates-action-check-now = Перевірити зараз
updates-action-check-again = Перевірити ще раз
updates-action-try-again = Спробувати ще раз
updates-action-download = Завантажити оновлення
updates-action-restart = Перезапустити для оновлення
updates-action-open-installer = Відкрити інсталятор

updates-busy-checking = Перевірка...
updates-busy-resuming = Відновлення завантаження...
updates-busy-starting-download = Початок завантаження...
updates-busy-restarting = Перезапуск...
updates-busy-opening-installer = Відкриття інсталятора...

updates-progress-downloading = Завантаження оновлення
updates-progress-verifying = Перевірка оновлення
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } із { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Подробиці інсталяції
updates-detail-installed-version = Встановлена версія
updates-detail-system = Система
updates-detail-last-checked = Остання перевірка
updates-detail-never = Ніколи
updates-detail-available-version = Доступна версія
updates-detail-download-size = Розмір завантаження

updates-version-installed = Встановлена
updates-version-available = Доступна

updates-notes-highlights = Головне
updates-notes-empty-title = Приміток до випуску ще немає
updates-notes-empty-error = Не вдалося завантажити історію випусків. Перевірте з’єднання й спробуйте ще раз.
updates-notes-empty-none = Примітки до випуску з’являться тут після публікації випуску.
updates-notes-history-notice = Показано те, що вже відомо цій інсталяції — історію випусків не вдалося завантажити.
updates-release-empty = Для цього випуску не вказано жодних змін.
updates-release-changes =
    { $count ->
        [one] { $count } зміна
        [few] { $count } зміни
        [many] { $count } змін
       *[other] { $count } зміни
    }

updates-preferences-unavailable-title = Параметри оновлення недоступні
updates-preferences-unavailable-detail = Не вдалося завантажити конфігурацію оновлень.
updates-preference-fallback-name = параметр оновлення
updates-preference-save-failed = Не вдалося зберегти: { $preference }

updates-request-failed = Запит не вдався
updates-check-request-failed = Не вдалося перевірити оновлення
updates-resume-failed = Не вдалося відновити завантаження оновлення
updates-download-failed = Не вдалося розпочати завантаження оновлення
updates-apply-failed = Не вдалося встановити оновлення
updates-install-failed = Не вдалося відкрити інсталятор оновлення
updates-apply-confirm-title = Встановити v{ $version }
updates-apply-confirm-message = { -brand } перезапуститься на нову версію. Торгівля зупиниться на кілька секунд і відновиться автоматично; відкриті позиції не зміняться.
updates-install-confirm-title = Запустити інсталятор
updates-install-confirm-message = Відкриється перевірений інсталятор, а { -brand } коректно завершить роботу. Завершіть роботу інсталятора, потім знову відкрийте { -brand }.

# A release version as displayed.
updates-version-number = v{ $version }

# The Home update notice, shown while a release is in play.
updates-notice-region =
    .aria-label = Стан оновлення
updates-notice-view = Переглянути оновлення
updates-notice-whats-new = Що нового
updates-notice-available-detail = Перегляньте зміни та встановіть оновлення в налаштуваннях.
updates-notice-updated-detail = Перегляньте, що змінилося в цій версії.
# A headless installation cannot download or install a release itself.
updates-headless-install-detail = Установлення без інтерфейсу оновлюються поза панеллю. У Linux виконайте { "screenerbot-manager update" }.
