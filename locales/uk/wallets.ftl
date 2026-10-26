# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = Створений
wallets-type-imported = Імпортований
wallets-type-migrated = Перенесений

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = Призупинено вами
wallets-watch-disabled-signature-budget = Призупинено: досягнуто ліміту перевірки підписів ({ $limit }) до того, як стеження наздогнало активність
wallets-watch-disabled-unknown = Призупинено: не вдалося прочитати збережену причину безпеки стеження
wallets-watch-disabled-helius-unavailable = Призупинено: провайдер для високої активності недоступний; курсор збережено
wallets-watch-disabled-processing-failed = Призупинено: не вдалося обробити активність гаманця; курсор збережено

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = Провайдер для високої активності недоступний; стеження призупинено
wallets-watch-error-provider-repeated-failure = Перевірки через { -helius } раз у раз завершуються помилкою; стеження призупинено
wallets-watch-error-processing-repeated-failure = Обробка активності гаманця раз у раз завершується помилкою; стеження призупинено
wallets-watch-error-position-unreadable = Стеження не змогло прочитати збережену позицію; повторна спроба
wallets-watch-error-provider-check-failed = Перевірка через провайдера для високої активності не вдалася; повторна спроба
wallets-watch-error-decode-failed = Не вдалося декодувати транзакцію з високої активності; курсор збережено
wallets-watch-error-processing-failed = Не вдалося обробити активність гаманця; повторна спроба
wallets-watch-error-position-save-failed = Стеження не змогло зберегти свою позицію; повторна спроба

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = Призупинено вами.
wallets-watch-reason-signature-budget = Цей гаманець має більше активності, ніж поточне стеження здатне перевірити.
wallets-watch-reason-helius-unavailable = Перевірки через { -helius } не вдалися. Збережений прогрес не втрачено.
wallets-watch-reason-processing-failed = Не вдалося обробити активність гаманця. Збережений прогрес не втрачено.

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = Закрити вікно
wallets-summary-native = { -sol }
wallets-copied-private-key = Приватний ключ

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Основний гаманець
wallets-tab-other = Інші гаманці
wallets-tab-watched = Відстежувані
wallets-refresh-failed = Не вдалося оновити гаманці
wallets-action-failed = Помилка
wallets-toast-failed = Помилка: { $reason }
wallets-export-busy = Розшифровування...
wallets-export-revealed = Ключ показано — поводьтеся з ним обережно

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = Стежити за гаманцем
wallets-watch-add-address = Адреса гаманця
wallets-watch-add-address-input =
    .placeholder = Адреса Solana
wallets-watch-add-address-hint = Записує ончейн-активність гаманця та надсилає сповіщення про угоди відповідно до ваших налаштувань { -telegram }.
wallets-watch-add-label = Мітка
wallets-watch-add-label-input =
    .placeholder = Необов’язкова назва
wallets-watch-add-submit = Додати стеження

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = Параметри стеження за гаманцем
wallets-watch-budget-title-restore = Відновити стеження за гаманцем
wallets-watch-budget-close =
    .aria-label = Закрити
wallets-watch-budget-label-signatures = Підписів за одну перевірку
wallets-watch-budget-label-transactions = Успішних повних транзакцій за одну перевірку
wallets-watch-budget-hint-signatures = Поточний ліміт: { $limit }. Оберіть від 500 до 5 000 підписів на перевірку з кроком 100.
wallets-watch-budget-hint-transactions = Поточний ліміт: { $limit }. Оберіть від 500 до 5 000 успішних транзакцій на перевірку з кроком 100.
wallets-watch-budget-error-range = Оберіть від 500 до 5 000 записів на перевірку з кроком 100.
wallets-watch-budget-error-ack = Підтвердьте, що підписи після останньої завершеної перевірки буде пропущено.
wallets-watch-budget-save-failed = Не вдалося зберегти ліміт стеження.
wallets-watch-budget-save = Зберегти ліміт
wallets-watch-budget-resume = Відновити з поточного моменту
wallets-watch-budget-resume-notice = Цей гаманець досяг ліміту перевірки, не наздогнавши активність. Відновлення з поточного моменту починається з найновішої активності гаманця; активність після останньої завершеної перевірки не буде скопійовано.
wallets-watch-budget-resume-tasks = Копі-завдання залишаються призупиненими, доки ви не відновите кожне з них у розділі «Копітрейдинг».
wallets-watch-budget-resume-ack = Я розумію, що пропущену активність не буде скопійовано.
wallets-watch-budget-resumed = Стеження відновлено з поточного стану гаманця
wallets-watch-budget-updated = Ліміт стеження за гаманцем оновлено
wallets-watch-helius-allow = Дозволити наздоганяння через { -helius }, якщо потрібно
wallets-watch-helius-try = Спробувати наздогнати через { -helius }
wallets-watch-helius-stop = Зупинити наздоганяння через { -helius } для цього гаманця
wallets-watch-helius-description-approved = Наздоганяння через { -helius } дозволено для цього гаманця. Якщо вимкнути його, стеження повернеться до стандартних перевірок, які можуть відставати на гаманці з високою активністю.
wallets-watch-helius-description-available = { -helius } може перевіряти успішні транзакції Solana від збереженої позиції, не пропускаючи неперевірений проміжок. Це може витрачати більше кредитів провайдера, і відставання все ж можливе.
wallets-watch-helius-description-unavailable = Наздоганяння через { -helius } недоступне. Щоб його використовувати, налаштуйте увімкнений RPC-ендпоінт { -helius }.
wallets-watch-helius-description-unsupported = Для цього стеження немає підтримуваного провайдера наздоганяння. Якщо стеження досягне ліміту, можна відновити з поточного моменту.
wallets-watch-helius-allow-title = Дозволити наздоганяння через { -helius } для цього гаманця
wallets-watch-helius-allow-message = { -helius } може перевіряти успішні транзакції Solana від збереженої позиції, не пропускаючи неперевірений проміжок. Наразі стягується 10 кредитів за кожні 100 повернутих повних транзакцій із округленням угору, мінімум 10 кредитів за запит. Одна перевірка може робити кілька запитів; витрати та ціни провайдера можуть відрізнятися. Копі-завдання залишаються призупиненими, доки їх не буде відновлено окремо.
wallets-watch-helius-allow-confirm = Дозволити для цього гаманця
wallets-watch-helius-stop-message = Цей гаманець повернеться до стандартних перевірок. Гаманець із високою активністю може знову досягти ліміту стеження й призупинитися. Інші гаманці та ваша конфігурація RPC { -helius } не зміняться.
wallets-watch-helius-stop-confirm = Зупинити для цього гаманця
wallets-watch-helius-stop-keep = Залишити дозволеним
wallets-watch-helius-restored = Стеження відновлено зі збереженого прогресу; копі-завдання залишаються призупиненими
wallets-watch-helius-allowed = Наздоганяння через { -helius } дозволено для цього гаманця, коли потрібно
wallets-watch-helius-stopped = Наздоганяння через { -helius } для цього гаманця зупинено
wallets-watch-helius-update-failed = Не вдалося оновити налаштування наздоганяння для гаманця

# wallets.html: Export Private Key dialog.
wallets-export-title = Експорт приватного ключа
wallets-export-warning-title = Критичне попередження щодо безпеки
wallets-export-warning-body = Ніколи нікому не передавайте свій приватний ключ. Будь-хто з доступом до цього ключа може викрасти всі кошти з цього гаманця.
wallets-export-key-label = Приватний ключ (Base58)
wallets-export-copy =
    .title = Копіювати в буфер обміну
    .aria-label = Копіювати в буфер обміну
wallets-export-reveal = Показати ключ

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Токен
wallets-holdings-col-balance = Баланс
wallets-holdings-col-value = Вартість ({ -sol })
wallets-holdings-col-type = Тип
wallets-holdings-col-decimals = Десяткові знаки
wallets-holdings-empty-title = Немає токенів
wallets-holdings-unreadable-title = Активи невідомі
wallets-holdings-empty-message = Токени, які утримує цей гаманець, з’являться тут.
wallets-funding-title = Поповніть торговий гаманець, щоб почати
wallets-funding-message = На цьому гаманці немає { -sol }. Надішліть { -sol } на цю адресу з біржі або іншого гаманця; кожна покупка витрачає свою суму плюс мережеві комісії.
wallets-holdings-no-main = Немає основного гаманця
wallets-holdings-main-title = Основний гаманець
wallets-holdings-tokens = Токени
wallets-holdings-last-used = Востаннє використано
wallets-holdings-never = Ніколи
wallets-holdings-search =
    .placeholder = Пошук за символом або мінтом...
wallets-holdings-export = Експортувати ключ
wallets-holdings-export-tooltip = Експортувати приватний ключ цього гаманця
wallets-holdings-replace = Замінити гаманець
wallets-holdings-replace-tooltip = Торгувати з іншим гаманцем; ключ та історія цього гаманця зберігаються
wallets-list-col-name = Назва
wallets-list-col-balance = Баланс ({ -sol })
wallets-list-col-type = Тип
wallets-list-col-created = Створено
wallets-list-col-replaced = Замінено
wallets-list-replaced-on = { $date }
wallets-list-col-actions = Дії
wallets-list-action-export = Експортувати приватний ключ
wallets-list-count = Гаманці
wallets-list-search =
    .placeholder = Пошук за назвою або адресою...
wallets-list-loading-title = Завантаження гаманців…
wallets-list-loading-description = Підготовка вибраного подання гаманців.
wallets-list-kept-empty-message = Тут відображаються гаманці з попередніх версій, щоб ви й надалі могли експортувати їхні ключі.
wallets-other-empty-title = Інших гаманців немає

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = Гаманець
wallets-watched-col-status = Статус
wallets-watched-col-progress = Збережений прогрес
wallets-watched-col-last-check = Остання перевірка
wallets-watched-unlabelled = Гаманець без мітки
wallets-watched-generic-name = гаманець
wallets-watched-not-synced = Ще не синхронізовано
wallets-watched-not-checked = Ще не перевірено
wallets-watched-action-copy = Копіювати угоди
    .title = Відкрити цей гаманець у розділі «Копітрейдинг»
wallets-watched-action-restore = Відновити стеження
wallets-watched-action-options = Параметри стеження
wallets-watched-action-retry = Повторити стеження
wallets-watched-action-pause = Призупинити
wallets-watched-action-enable = Увімкнути
wallets-watched-action-remove =
    .title = Прибрати
    .aria-label = Прибрати { $name }
wallets-watch-state-paused = Призупинено
wallets-watch-state-catching-up = Наздоганяє
wallets-watch-state-watching = Стежить
wallets-watch-state-streaming = Потік
wallets-watch-state-polling = Опитування
wallets-watched-detail-helius = Перевірка цього гаманця через { -helius }.
wallets-watched-empty-title = Немає відстежуваних адрес
wallets-watched-empty-message = Скористайтеся «Стежити за гаманцем», щоб записувати ончейн-активність публічного гаманця.
wallets-watched-count = Відстежувані
wallets-watched-search =
    .placeholder = Пошук відстежуваних гаманців...
wallets-watched-add = Стежити за гаманцем
wallets-watched-refresh = Оновити відстежувані гаманці
wallets-watched-loading-title = Завантаження відстежуваних гаманців...
wallets-watched-loading-description = Отримання цілей спостереження.
wallets-watched-load-error-title = Не вдалося завантажити відстежувані адреси
wallets-watched-load-error-description = Натисніть «Оновити», щоб спробувати ще раз.
wallets-watched-address-invalid = Введіть дійсну адресу гаманця Solana.
wallets-watched-added = Стеження за гаманцем додано
wallets-watched-duplicate = За цим гаманцем уже стежать.
wallets-watched-add-failed = Не вдалося додати стеження за гаманцем.
wallets-watched-retried = Стеження за гаманцем відновлено зі збереженим курсором
wallets-watched-paused = Стеження за гаманцем призупинено
wallets-watched-enabled = Стеження за гаманцем увімкнено
wallets-watched-removed = Стеження за гаманцем прибрано
wallets-watched-update-failed = Не вдалося оновити стеження за гаманцем
wallets-setup-gate-title = Для гаманців потрібне налаштування
