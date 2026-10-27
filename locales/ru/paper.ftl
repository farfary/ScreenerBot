paper-readiness-history = Виртуальная история
paper-readiness-history-met =
    { $count ->
        [one] { $count } закрытый виртуальный раунд, нужно { $needed }
        [few] { $count } закрытых виртуальных раунда, нужно { $needed }
        [many] { $count } закрытых виртуальных раундов, нужно { $needed }
       *[other] { $count } закрытого виртуального раунда, нужно { $needed }
    }
paper-readiness-history-short = { $count } из { $needed } закрытых виртуальных раундов
paper-readiness-profit = Прибыльность в виртуальном режиме
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } реализовано за { $count } раунд, выиграно: { $wins }
        [few] { $realized } { -sol } реализовано за { $count } раунда, выиграно: { $wins }
        [many] { $realized } { -sol } реализовано за { $count } раундов, выиграно: { $wins }
       *[other] { $realized } { -sol } реализовано за { $count } раунда, выиграно: { $wins }
    }
paper-readiness-exits = Выходы исполняются
paper-readiness-exits-ok = Последние виртуальные выходы исполнены
paper-readiness-exits-failing = Виртуальных выходов подряд без исполнения: { $count }
paper-live-block-setup-incomplete = Сначала завершите настройку кошелька и RPC
paper-live-block-force-stop = Включена экстренная остановка
