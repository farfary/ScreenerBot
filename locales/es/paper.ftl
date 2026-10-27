paper-readiness-history = Historial simulado
paper-readiness-history-met =
    { $count ->
        [one] { $count } ronda simulada cerrada, se necesitan { $needed }
        [many] { $count } rondas simuladas cerradas, se necesitan { $needed }
       *[other] { $count } rondas simuladas cerradas, se necesitan { $needed }
    }
paper-readiness-history-short = { $count } de { $needed } rondas simuladas cerradas
paper-readiness-profit = Rentable en simulación
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realizados en { $count } ronda, { $wins } ganadas
        [many] { $realized } { -sol } realizados en { $count } rondas, { $wins } ganadas
       *[other] { $realized } { -sol } realizados en { $count } rondas, { $wins } ganadas
    }
paper-readiness-exits = Las salidas se ejecutan
paper-readiness-exits-ok = Las salidas simuladas recientes se ejecutaron
paper-readiness-exits-failing = Salidas simuladas seguidas sin ejecutar: { $count }
paper-live-block-setup-incomplete = Termina primero la configuración de la billetera y del RPC
paper-live-block-force-stop = La parada de emergencia está activada
