paper-readiness-history = Віртуальна історія
paper-readiness-history-met =
    { $count ->
        [one] { $count } закритий віртуальний раунд, потрібно { $needed }
        [few] { $count } закриті віртуальні раунди, потрібно { $needed }
        [many] { $count } закритих віртуальних раундів, потрібно { $needed }
       *[other] { $count } закритого віртуального раунду, потрібно { $needed }
    }
paper-readiness-history-short = Закритих віртуальних раундів: { $count } із { $needed }
paper-readiness-profit = Прибутковість у віртуальному режимі
paper-readiness-profit-detail = Реалізовано { $realized } { -sol } за раундів: { $count }, виграно: { $wins }
paper-readiness-exits = Виходи виконуються
paper-readiness-exits-ok = Останні віртуальні виходи виконано
paper-readiness-exits-failing = Віртуальних виходів поспіль без виконання: { $count }
paper-live-block-setup-incomplete = Спершу завершіть налаштування гаманця та RPC
paper-live-block-force-stop = Активна екстрена зупинка
