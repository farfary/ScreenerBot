paper-readiness-history = Paper-Verlauf
paper-readiness-history-met =
    { $count ->
        [one] { $count } geschlossene Paper-Runde, { $needed } erforderlich
       *[other] { $count } geschlossene Paper-Runden, { $needed } erforderlich
    }
paper-readiness-history-short = { $count } von { $needed } geschlossenen Paper-Runden
paper-readiness-profit = Im Paper-Modus profitabel
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realisiert in { $count } Runde, { $wins } gewonnen
       *[other] { $realized } { -sol } realisiert in { $count } Runden, { $wins } gewonnen
    }
paper-readiness-exits = Ausstiege werden ausgeführt
paper-readiness-exits-ok = Die letzten Paper-Ausstiege wurden ausgeführt
paper-readiness-exits-failing = Paper-Ausstiege in Folge ohne Ausführung: { $count }
paper-live-block-setup-incomplete = Schließen Sie zuerst die Wallet- und RPC-Einrichtung ab
paper-live-block-force-stop = Der Not-Stopp ist aktiv
