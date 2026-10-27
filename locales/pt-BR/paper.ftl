paper-readiness-history = Histórico em Paper
paper-readiness-history-met =
    { $count ->
        [one] { $count } rodada fechada em Paper, { $needed } necessárias
        [many] { $count } rodadas fechadas em Paper, { $needed } necessárias
       *[other] { $count } rodadas fechadas em Paper, { $needed } necessárias
    }
paper-readiness-history-short = { $count } de { $needed } rodadas fechadas em Paper
paper-readiness-profit = Lucrativa em Paper
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realizados em { $count } rodada, { $wins } vencedoras
        [many] { $realized } { -sol } realizados em { $count } rodadas, { $wins } vencedoras
       *[other] { $realized } { -sol } realizados em { $count } rodadas, { $wins } vencedoras
    }
paper-readiness-exits = As saídas são executadas
paper-readiness-exits-ok = As saídas paper recentes foram executadas
paper-readiness-exits-failing = Saídas paper seguidas sem execução: { $count }
paper-live-block-setup-incomplete = Conclua antes a configuração da carteira e do RPC
paper-live-block-force-stop = A parada de emergência está ativa
