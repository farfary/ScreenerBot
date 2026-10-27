paper-readiness-history = Historique simulé
paper-readiness-history-met =
    { $count ->
        [one] { $count } cycle simulé clôturé, { $needed } requis
        [many] { $count } cycles simulés clôturés, { $needed } requis
       *[other] { $count } cycles simulés clôturés, { $needed } requis
    }
paper-readiness-history-short = { $count } cycles simulés clôturés sur { $needed }
paper-readiness-profit = Rentable en simulé
paper-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } réalisés sur { $count } cycle, { $wins } gagné
        [many] { $realized } { -sol } réalisés sur { $count } cycles, { $wins } gagnés
       *[other] { $realized } { -sol } réalisés sur { $count } cycles, { $wins } gagnés
    }
paper-readiness-exits = Les sorties s’exécutent
paper-readiness-exits-ok = Les dernières sorties simulées ont été exécutées
paper-readiness-exits-failing = Sorties simulées consécutives sans exécution : { $count }
paper-live-block-setup-incomplete = Terminez d'abord la configuration du portefeuille et du RPC
paper-live-block-force-stop = L'arrêt d'urgence est activé
