# Copy trading messages.

copy-skip-not-buy-swap = L'activité du portefeuille n'était pas un achat
copy-skip-task-disabled = La tâche est en pause
copy-skip-mode-transition-required = Le mode d'exécution doit être changé séparément
copy-skip-live-confirmation-required = L'exécution réelle nécessite une confirmation
copy-skip-unsupported-sizing-mode = Ce mode de dimensionnement n'est pas encore pris en charge
copy-skip-self-copy = Le portefeuille fait partie des vôtres
copy-skip-target-below-minimum = Trade du portefeuille inférieur au minimum
copy-skip-target-above-maximum = Trade du portefeuille supérieur au maximum
copy-skip-already-bought = Token déjà acheté (achat unique)
copy-skip-blacklisted = Token bloqué par les contrôles de risque
copy-skip-filter-required = Le token n'a pas passé le filtrage
copy-skip-budget-exhausted = Le budget de la tâche est épuisé
copy-skip-token-cap-reached = Limite par token atteinte
copy-skip-below-minimum-size = Taille de copie trop faible
copy-skip-invalid-sizing = Le dimensionnement de la tâche est invalide
copy-skip-invalid-slippage = Le slippage de la tâche est invalide
copy-skip-invalid-exit-policy = Les règles de sortie de la tâche sont invalides
copy-skip-invalid-price = Aucun prix de marché exploitable
copy-skip-not-sell-swap = L'activité du portefeuille n'était pas une vente
copy-skip-exit-mode-disabled = Vente du portefeuille ignorée : la tâche vend selon ses propres règles
copy-skip-force-stopped = Le trading est en arrêt forcé
copy-skip-copy-position-not-found = Aucune position appartenant à cette tâche
copy-skip-position-user-only = La position est gérée par vous
copy-skip-position-management-mismatch = La position ne suit plus les ventes copiées
copy-skip-latency-kill-switch = Mise en pause automatique : trades détectés trop tard
copy-skip-claim-reconciled-abandoned = Envoi réel interrompu, clôturé sans nouvelle tentative
copy-skip-stale-observation = Rejoué après une interruption, trop ancien pour être copié
copy-skip-unknown-observation-time = Le trade rejoué n'a pas d'horodatage de bloc
copy-skip-entry-blocked = Entrée bloquée

copy-entry-block-force-stopped = Le trading est en arrêt forcé
copy-entry-block-loss-limit = La limite de perte bloque les nouvelles entrées
copy-entry-block-connectivity = Les services requis sont indisponibles
copy-entry-block-position-limit = Limite de positions ouvertes atteinte
copy-entry-block-already-open = Une position est déjà ouverte
copy-entry-block-reentry-cooldown = Délai avant nouvelle entrée sur le token
copy-entry-block-open-cooldown = Délai global entre entrées
copy-entry-block-entry-reserved = Une autre entrée est en cours de traitement
copy-entry-block-blacklisted = Token bloqué par les contrôles de risque
copy-entry-block-check-failed = Un contrôle de sécurité n'a pas pu aboutir

copy-pause-user = Mise en pause par vous
copy-pause-latency-kill-switch = Mise en pause automatique : les trades sont arrivés avec { $average } s de retard en moyenne (limite : { $threshold } s)
copy-pause-watch-detached = Mise en pause automatique : le portefeuille n'est plus surveillé
copy-pause-watch-budget-exceeded = En pause : ce portefeuille a atteint sa limite de { $limit } signatures vérifiées avant d'avoir rattrapé son retard
copy-pause-helius-unavailable = En pause : les vérifications de portefeuille via { -helius } ont échoué
copy-pause-watch-processing-failed = En pause : l'activité du portefeuille n'a pas pu être traitée
copy-pause-unspecified = En pause

copy-pause-short-user = par vous
copy-pause-short-latency-kill-switch = trop lent
copy-pause-short-watch-detached = suivi perdu
copy-pause-short-watch-budget-exceeded = limite de suivi
copy-pause-short-helius-unavailable = fournisseur de suivi
copy-pause-short-watch-processing-failed = traitement du suivi
copy-state-paused = En pause
copy-state-paused-reason = En pause · { $reason }

copy-readiness-history = Historique simulé
copy-readiness-history-met =
    { $count ->
        [one] { $count } cycle simulé clôturé, { $needed } requis
        [many] { $count } cycles simulés clôturés, { $needed } requis
       *[other] { $count } cycles simulés clôturés, { $needed } requis
    }
copy-readiness-history-short = { $count } cycles simulés clôturés sur { $needed }
copy-readiness-profit = Rentable en simulé
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } réalisés sur { $count } cycle, { $wins } gagné
        [many] { $realized } { -sol } réalisés sur { $count } cycles, { $wins } gagnés
       *[other] { $realized } { -sol } réalisés sur { $count } cycles, { $wins } gagnés
    }
copy-readiness-latency = Trades détectés à temps
copy-readiness-latency-detail = arrivée p95 { $p95 } s, limite { $limit } s
copy-readiness-latency-none = Aucun échantillon d'arrivée pour l'instant
copy-readiness-priced = Toutes les détentions valorisées
copy-readiness-priced-ok = Chaque détention simulée ouverte a un prix de pool
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } détention ouverte sans prix de pool
        [many] { $count } détentions ouvertes sans prix de pool
       *[other] { $count } détentions ouvertes sans prix de pool
    }
copy-readiness-runtime = Exécution réelle disponible
copy-readiness-runtime-ok = La configuration et les garde-fous autorisent les copies réelles

copy-live-block-setup-incomplete = Terminez d'abord la configuration du portefeuille et du RPC
copy-live-block-force-stop = L'arrêt d'urgence est activé
copy-live-block-copy-trading-disabled = Le traitement des copies est en pause globale
copy-live-block-unavailable = L'exécution réelle est indisponible

## Task state, mode and exit labels. Ids come from effective_state
## (src/trader/copy/control.rs), CopyMode, ExitMode and PaperExitRule
## (src/trader/copy/types.rs) plus the target_sell exit bucket.

copy-state-system-paused = En pause globale
copy-state-force-stopped = Arrêt forcé
copy-state-entries-blocked = Entrées bloquées
copy-state-running-live = Actif
copy-state-running-paper = Actif
copy-mode-paper = Simulé
copy-mode-live = Réel
copy-exit-mode-buy-only = Mes règles de sortie
copy-exit-mode-mirror = Répliquer les ventes du portefeuille
copy-exit-mode-hybrid = Ventes du portefeuille et mes règles
copy-exit-target-sell = Vendu par le portefeuille
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Règle de durée
copy-exit-manual = Clôturé manuellement

## Shared wording

copy-request-failed = La requête a échoué
copy-keep-paused = Laisser en pause
copy-paused-suffix = · en pause
copy-mode-paused = { $mode } · en pause
copy-task-ref = « { $name } » ({ $mode })
copy-metric-realized-pnl = P&L réalisé
copy-metric-unrealized-pnl = P&L latent
copy-metric-win-rate = Taux de réussite
copy-metric-budget-spent = Budget dépensé
copy-metric-median-arrival = Arrivée médiane
copy-metric-open-holdings = Détentions ouvertes
copy-record-won-lost = { $won } gagnés · { $lost } perdus
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Exécutions
copy-kind-exits = Sorties
copy-kind-skips = Ignorés
copy-kind-errors = Erreurs
copy-field-per-trade-cap = Plafond par trade
copy-field-per-token-cap = Plafond par token
copy-field-total-budget = Budget total
copy-field-slippage = Slippage
copy-rules-wallet-sells-only = Ventes du portefeuille uniquement
copy-filter-copy-setting-required = Réglage du Copy trading (requis)
copy-filter-copy-setting-not-required = Réglage du Copy trading (non requis)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } cycle clôturé
        [many] { $count } cycles clôturés
       *[other] { $count } cycles clôturés
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } détention ouverte
        [many] { $count } détentions ouvertes
       *[other] { $count } détentions ouvertes
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } détention valorisée · { $unpriced } sans prix
        [many] { $priced } détentions valorisées · { $unpriced } sans prix
       *[other] { $priced } détentions valorisées · { $unpriced } sans prix
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } détention sans prix
        [many] { $count } détentions sans prix
       *[other] { $count } détentions sans prix
    }
copy-range-24h = 24 h
copy-range-7d = 7 j
copy-range-30d = 30 j
copy-range-all = Tout
copy-range-label =
    .aria-label = Période

## Page strip (pages/copy.html, pages/copy/summary.js)

copy-page-title = Copy trading
copy-page-beta = Bêta
copy-strip-loading = Chargement
copy-strip-unavailable = Indisponible
copy-strip-setup-required = Configuration requise · le copy trading nécessite un portefeuille et un RPC
copy-strip-pause-all = Tout mettre en pause
copy-strip-resume = Reprendre le traitement
copy-strip-settings = Paramètres
copy-strip-add-wallet = Ajouter un portefeuille
copy-strip-paused-globally = En pause globale · aucune nouvelle copie, les sorties continuent
copy-strip-force-stopped = Arrêt forcé · rien n'est copié
copy-strip-loss-limit = Limite de perte · nouvelles entrées bloquées, les sorties continuent
copy-strip-idle-paused =
    { $count ->
        [one] Inactif · { $count } tâche en pause
        [many] Inactif · { $count } tâches en pause
       *[other] Inactif · { $count } tâches en pause
    }
copy-strip-idle-empty = Inactif · aucune tâche pour l'instant
copy-strip-processing = Traitement · { $paper } en simulé
copy-strip-processing-live = Traitement · { $live } en réel · { $paper } en simulé
copy-figures-label =
    .aria-label = Totaux du copy trading
copy-figure-marked-at-pool = Valorisé au prix du pool
copy-figure-across-tasks = Toutes tâches confondues
copy-figure-budget-lifetime = Dépenses cumulées des tâches activées
copy-figure-budget-none = Aucune tâche activée
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } trade
        [many] { $count } trades
       *[other] { $count } trades
    }
copy-figure-arrival-none = Aucun échantillon des tâches activées

## Page frame (pages/copy.js)

copy-load-failed = Le copy trading n'a pas pu être chargé : { $error }
copy-resume-all-title = Reprendre le traitement des copies
copy-resume-all-message =
    { $count ->
        [one] { $count } tâche réelle enverra de vrais swaps dès que son portefeuille tradera à nouveau.
        [many] { $count } tâches réelles enverront de vrais swaps dès que leurs portefeuilles traderont à nouveau.
       *[other] { $count } tâches réelles enverront de vrais swaps dès que leurs portefeuilles traderont à nouveau.
    }
copy-toast-resumed-all = Traitement des copies repris
copy-toast-paused-all = Traitement de toutes les copies mis en pause
copy-toast-global-failed = Le traitement des copies n'a pas pu être modifié

## Onboarding (pages/copy.html)

copy-onboarding-title = Copiez les portefeuilles de confiance, en les validant d'abord en simulé
copy-onboarding-body = Chaque tâche démarre en simulé : les trades ciblés sont simulés au prix du pool avec votre slippage et vos frais, et vos règles de sortie s'appliquent au registre simulé. Armez le mode réel portefeuille par portefeuille une fois que ses résultats simulés le justifient.
copy-onboarding-add = Ajouter votre premier portefeuille
copy-setup-gate-title = Le copy trading nécessite un portefeuille
copy-onboarding-observe = Observer
copy-onboarding-observe-detail = Détectez les swaps du portefeuille sans dépenser de { -sol }.
copy-onboarding-evaluate = Évaluer
copy-onboarding-evaluate-detail = Consultez le P&L simulé, le taux de réussite, les trades ignorés, la vitesse de détection et le slippage.
copy-onboarding-arm = Armer
copy-onboarding-arm-detail = Validez les contrôles de préparation, puis activez les swaps réels.

## Wallet list (pages/copy.html, pages/copy/list.js)

copy-list-label =
    .aria-label = Portefeuilles copiés
copy-list-title = Portefeuilles
copy-list-compare = Comparer
copy-list-sort-label = Trier les portefeuilles
copy-list-count = { $active } actifs · { $total } au total
copy-sort-pnl = P&L
copy-sort-state = État
copy-sort-name = Nom
copy-compare-label =
    .aria-label = Comparer les portefeuilles

## Dialog chrome (pages/copy.html)

copy-dialog-close =
    .aria-label = Fermer
copy-editor-title-add = Ajouter un portefeuille
copy-editor-sub-add = Les nouvelles tâches démarrent en simulé
copy-arm-title = Armer la copie réelle
copy-arm-sub = De vrais swaps depuis votre portefeuille
copy-arm-keep-paper = Rester en simulé
copy-arm-confirm = Armer le mode réel
copy-profile-title = Profil du portefeuille
copy-profile-sub = Ce que ce bot a observé du portefeuille

## Settings dialog (pages/copy.html, pages/copy/settings.js). Field labels and hints
## come from the config catalog.

copy-settings-title = Paramètres du copy trading
copy-settings-subtitle = Politique globale pour toutes les tâches
copy-settings-filter-warning = Avec la configuration de filtrage par défaut, cette option rejette presque tous les tokens, donc rien n'est copié. Laissez-la désactivée sauf si vos filtres valident les tokens échangés par vos portefeuilles.
copy-settings-unit-seconds = s
copy-settings-unit-trades = trades
copy-settings-unit-tasks = tâches
copy-settings-unit-rounds = tours
copy-settings-save = Enregistrer les paramètres
copy-settings-load-failed = Les paramètres de copie n'ont pas pu être chargés
copy-settings-saved = Paramètres du copy trading enregistrés

## Workspace (pages/copy/workspace.js)

copy-tab-overview = Aperçu
copy-tab-holdings = Détentions
copy-tab-activity = Activité
copy-tab-rules = Règles
copy-tab-execution = Exécution
copy-tabs-label = Vues de la tâche
copy-workspace-select = Sélectionnez un portefeuille pour ouvrir son espace de travail.
copy-workspace-loading = Chargement de la tâche…
copy-workspace-load-failed = Cette tâche n'a pas pu être chargée : { $error }

copy-state-detail-paper = Actif en simulé · les trades sont simulés, rien n'est dépensé
copy-state-detail-live = Actif en réel · les trades du portefeuille sont copiés avec de vrais swaps
copy-state-detail-system-paused = En attente · le traitement des copies est en pause globale, les sorties continuent
copy-state-detail-entries-blocked = Entrées bloquées par la limite de perte · les sorties continuent
copy-state-detail-force-stopped = Arrêt forcé · rien n'est copié

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = La reprise conserve la même limite, la tâche se remettra donc en pause tant que les trades arrivent en retard. Vérifiez le flux RPC ou relevez la limite d'arrivée dans les paramètres.
copy-paused-resume-detached = La reprise surveille de nouveau le portefeuille.
copy-paused-holdings-rules =
    { $count ->
        [one] Ses règles de sortie clôturent toujours sa { $count } détention ouverte.
        [many] Ses règles de sortie clôturent toujours ses { $count } détentions ouvertes.
       *[other] Ses règles de sortie clôturent toujours ses { $count } détentions ouvertes.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Les ventes du portefeuille clôturent toujours sa { $count } détention ouverte.
        [many] Les ventes du portefeuille clôturent toujours ses { $count } détentions ouvertes.
       *[other] Les ventes du portefeuille clôturent toujours ses { $count } détentions ouvertes.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Les ventes du portefeuille et ses règles de sortie clôturent toujours sa { $count } détention ouverte.
        [many] Les ventes du portefeuille et ses règles de sortie clôturent toujours ses { $count } détentions ouvertes.
       *[other] Les ventes du portefeuille et ses règles de sortie clôturent toujours ses { $count } détentions ouvertes.
    }

copy-watch-state-catching-up = Suivi du portefeuille : rattrapage en cours. Vérification de ce portefeuille via { -helius }.
copy-watch-state-watching = Suivi du portefeuille : en cours. Vérification de ce portefeuille via { -helius }.
copy-watch-last-check = Dernière vérification { $ago }.
copy-watch-recovery-active = Suivi du portefeuille actif
copy-watch-recovery-catching-up = Le suivi du portefeuille rattrape son retard
copy-watch-recovery-still-paused = La tâche de copie est toujours en pause. Reprenez la copie lorsque vous êtes prêt.
copy-watch-recovery-title = Rétablir le suivi du portefeuille
copy-watch-recovery-processing-failed = L'activité du portefeuille n'a pas pu être traitée. La progression enregistrée est conservée. Réessayez une fois le problème résolu.
copy-watch-recovery-provider-failed = Les vérifications { -helius } ont échoué. La progression enregistrée est conservée. Réessayez lorsque le fournisseur sera disponible.
copy-watch-recovery-budget-intro = Ce portefeuille a plus d'activité que son suivi actuel ne peut en vérifier. Choisissez comment continuer.
copy-watch-approve = Tenter de rattraper le retard avec { -helius }
copy-watch-approve-help = Reprend depuis la progression enregistrée. Peut consommer davantage de crédits { -helius } et peut quand même prendre du retard.
copy-watch-approve-unavailable = Le rattrapage via { -helius } est indisponible. Configurez un endpoint RPC { -helius } activé pour continuer sans ignorer l'activité non vérifiée.
copy-watch-no-provider = Aucun fournisseur de rattrapage n'est pris en charge pour ce suivi.
copy-watch-budget-label = Signatures vérifiées par contrôle
copy-watch-budget-hint = Ou ignorez l'activité non vérifiée et reprenez à partir de maintenant. Choisissez entre { $min } et { $max } signatures par contrôle ; une limite plus élevée peut utiliser davantage d'appels RPC.
copy-watch-ack = Je comprends que l'activité manquée ne sera pas copiée.
copy-watch-toast-range = Choisissez entre { $min } et { $max } signatures par interrogation, par paliers de { $step } signatures
copy-watch-toast-ack = Confirmez que les signatures depuis le dernier contrôle terminé seront ignorées
copy-watch-resumed = Suivi du portefeuille repris à partir de maintenant ; la tâche de copie reste en pause
copy-watch-resume-failed = Le suivi du portefeuille n'a pas pu être repris
copy-watch-retry-started = Nouvelle tentative de suivi lancée depuis la progression enregistrée ; la tâche de copie reste en pause
copy-watch-retry-failed = Le suivi du portefeuille n'a pas pu être relancé
copy-watch-approve-title = Autoriser le rattrapage { -helius } pour ce portefeuille
copy-watch-approve-message = { -helius } peut vérifier les transactions Solana réussies depuis la progression enregistrée sans ignorer l'intervalle non vérifié. Il facture actuellement 10 crédits pour 100 transactions complètes renvoyées, arrondis au supérieur, avec un minimum de 10 crédits par requête. Une vérification peut effectuer plusieurs requêtes ; la consommation et les tarifs du fournisseur peuvent varier. La copie reste en pause jusqu'à ce que vous la repreniez séparément.
copy-watch-approve-confirm = Autoriser pour ce portefeuille
copy-watch-approved = Suivi du portefeuille lancé depuis la progression enregistrée ; la tâche de copie reste en pause
copy-watch-restore-failed = Le suivi du portefeuille n'a pas pu être rétabli

copy-action-pause = Mettre en pause
copy-action-resume = Reprendre
copy-action-resume-copy = Reprendre la copie
copy-action-resume-from-now = Reprendre à partir de maintenant
copy-action-retry-watch = Relancer le suivi du portefeuille
copy-action-return-paper = Repasser en simulé
copy-action-edit-rules = Modifier les règles
copy-action-clone = Dupliquer
copy-action-profile = Profil du portefeuille
copy-resume-live-title = Reprendre la copie réelle
copy-resume-live-message = « { $name } » enverra de vrais swaps depuis votre portefeuille dès que ce portefeuille tradera à nouveau.
copy-resume-live-confirm = Reprendre en réel
copy-task-resumed = Tâche reprise
copy-task-paused = Tâche mise en pause
copy-task-state-failed = L'état de la tâche n'a pas pu être modifié
copy-return-paper-message = Les nouvelles copies de « { $name } » seront de nouveau simulées, sans dépenser de { -sol }.
copy-return-paper-cancel = Rester en réel
copy-task-returned-paper = Tâche repassée en simulé
copy-mode-change-failed = Le mode d'exécution n'a pas pu être modifié
copy-delete-title = Supprimer la tâche de copie
copy-delete-message = Supprimer « { $name } » ? Ses décisions et ses résultats simulés sont effacés, et le portefeuille n'est plus surveillé pour cette tâche.
copy-delete-confirm = Supprimer la tâche
copy-delete-cancel = Conserver la tâche
copy-task-deleted = Tâche de copie supprimée
copy-task-delete-failed = La tâche de copie n'a pas pu être supprimée

## Overview tab (pages/copy/overview.js)

copy-overview-results = Résultats
copy-analytics-load-failed = Les statistiques n'ont pas pu être chargées : { $error }
copy-analytics-loading = Chargement des statistiques…
copy-exit-bucket =
    { $count ->
        [one] { $count } vente · { $pnl }
        [many] { $count } ventes · { $pnl }
       *[other] { $count } ventes · { $pnl }
    }
copy-overview-average-win = Gain moyen
copy-overview-average-loss = Perte moyenne { $amount }
copy-overview-profit-factor = Facteur de profit
copy-overview-profit-factor-note = Gains bruts ÷ pertes brutes
copy-overview-average-hold = Durée moyenne de détention
copy-overview-average-hold-note = De l'entrée à la sortie
copy-overview-best-round = Meilleur cycle
copy-overview-worst-round = Pire { $amount }
copy-overview-curve-title = P&L cumulé
copy-overview-exits-title = Ventes par type de sortie
copy-overview-skips-title = Pourquoi des trades ont été ignorés
copy-book-title-live = Registre réel
copy-book-title-paper = Registre simulé
copy-book-all-time = Depuis le début
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] achat
        [many] achats
       *[other] achats
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] sortie selon vos règles
        [many] sorties selon vos règles
       *[other] sorties selon vos règles
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] vente du portefeuille
        [many] ventes du portefeuille
       *[other] ventes du portefeuille
    }
copy-book-manual-closes = <strong>{ $count }</strong> clôturés manuellement
copy-book-skipped = <strong>{ $count }</strong> ignorés
copy-book-failed = <strong>{ $count }</strong> échoués
copy-book-closed = { $count } clôturés
copy-book-budget-note = Dépenses { $mode } sur { $total } · { $remaining } restants
copy-check-passed = validé
copy-check-not-passed = non validé
copy-readiness-title = Avant de passer en réel
copy-readiness-live-note = Cette tâche trade en réel. Repassez-la en simulé depuis l'en-tête ci-dessus.
copy-readiness-all-pass = Tous les contrôles sont validés.
copy-readiness-needs-review = L'armement exige une revue explicite de ce qui n'est pas prêt.
copy-readiness-arm = Vérifier et armer le mode réel

## Rules tab and review (pages/copy/rules.js)

copy-rules-title = Règles en vigueur
copy-rules-size-ratio = { $pct } du trade du portefeuille
copy-rules-size-fixed = { $amount } par copie
copy-rules-target-any = Toute taille
copy-rules-target-min = Au moins { $amount }
copy-rules-target-max = Au plus { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Surcharge de la tâche · Trader { $value }
copy-rules-source-default = Valeur par défaut du Trader
copy-rules-not-used = Inutilisé : les ventes du portefeuille décident
copy-rules-col-rule = Règle
copy-rules-col-applies = Valeur
copy-rules-col-source = Source
copy-rules-budget-note = { $spent } dépensés en { $mode } · { $remaining } restants
copy-rules-token-copies =
    { $count ->
        [one] Environ { $count } copie complète d'un token
        [many] Environ { $count } copies complètes d'un token
       *[other] Environ { $count } copies complètes d'un token
    }
copy-rules-sizing = Dimensionnement
copy-rules-copy-size = Taille de copie
copy-rules-entry-filters = Filtres d'entrée
copy-rules-target-size = Taille du trade du portefeuille
copy-rules-repeat-buys = Achats répétés
copy-rules-repeat-first-only = Premier achat de chaque token uniquement
copy-rules-repeat-every = Tous les achats, jusqu'au plafond par token
copy-rules-filter-pass = Validation du filtrage
copy-rules-filter-required = Requise
copy-rules-filter-not-required = Non requise
copy-rules-filter-task-override = Surcharge de la tâche
copy-rules-exits = Sorties
copy-rules-exits-inactive = Les détentions ne sont vendues que lorsque le portefeuille vend ; les règles ci-dessous ne s'appliquent pas dans ce mode.

## Exit rules (pages/copy/policy.js)

copy-rule-status = Statut
copy-rule-on = Activée
copy-rule-off = Désactivée
copy-rule-unit-seconds = s
copy-rule-unit-minutes = min
copy-rule-stop-loss-threshold = Vend à une perte de
copy-rule-stop-loss-min-hold = Pas avant une détention de
copy-rule-no-minimum = Aucun minimum
copy-rule-partial-exits = Sorties partielles
copy-rule-partial-allowed = Autorisées
copy-rule-partial-full-only = Sortie complète uniquement
copy-rule-partial-size = Taille de la sortie partielle
copy-rule-trailing-activation = S'arme à un gain de
copy-rule-trailing-distance = Vend sous le pic de
copy-rule-take-profit-target = Vend à un gain de
copy-rule-time-duration = Vérifie après une détention de
copy-rule-time-threshold = Vend tant que le P&L est inférieur ou égal à
copy-preset-inherit = Valeurs par défaut du Trader
copy-preset-conservative = Prudent
copy-preset-balanced = Équilibré
copy-preset-aggressive = Agressif
copy-preset-custom = Personnalisé
copy-validate-stop-loss = Le stop loss doit être supérieur à 0 % et au plus égal à 100 %.
copy-validate-partial-size = La taille de la sortie partielle doit être comprise entre 0 % et 100 %.
copy-validate-min-hold = La détention minimale doit être un nombre entier de secondes.
copy-validate-trailing-activation = L'activation du trailing doit être supérieure à 0 % et au plus égale à 100 %.
copy-validate-trailing-distance = La distance du trailing doit être supérieure à 0 % et au plus égale à 100 %.
copy-validate-take-profit = Le take profit doit être supérieur à 0 %.
copy-validate-time-duration = La règle de durée exige une durée supérieure à zéro.
copy-validate-time-threshold = Le seuil de la règle de durée est une perte : utilisez 0 % ou un nombre négatif.
copy-warning-mirror = Seules les ventes du portefeuille clôturent les détentions : aucun stop loss ne les protège, et un token que le portefeuille ne vend jamais reste détenu.
copy-warning-no-rules = Aucune règle de sortie n'est activée et les ventes du portefeuille sont ignorées : les détentions ne sont jamais vendues.
copy-warning-no-stop-loss = Aucun stop loss ne s'applique : un token en baisse est conservé jusqu'à ce qu'une autre règle ou le portefeuille vende.
copy-warning-stop-delay = Le stop loss attend { $hold } après chaque achat : un token qui chute plus vite est clôturé bien au-delà de { $threshold }.
copy-warning-take-profit-cost = Un take profit à { $target } ne couvre pas la vente ({ $slippage } de slippage et { $fee } de frais de swap), il clôture donc les cycles à perte.
copy-warning-trailing-distance = La distance du trailing est au moins égale à son gain d'activation, un trailing armé peut donc vendre sous le prix d'entrée.

## Execution tab (pages/copy/execution.js)

copy-execution-title = Qualité d'exécution
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Toutes
copy-execution-limit-on =
    { $count ->
        [one] Pause au-delà de { $limit } en moyenne sur { $count } trade
        [many] Pause au-delà de { $limit } en moyenne sur { $count } trades
       *[other] Pause au-delà de { $limit } en moyenne sur { $count } trades
    }
copy-execution-limit-off = Kill switch désactivé
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } trade vu en temps réel
        [many] { $count } trades vus en temps réel
       *[other] { $count } trades vus en temps réel
    }
copy-execution-p95 = Arrivée p95
copy-execution-median-slippage = Slippage médian
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } exécution mesurée
        [many] { $count } exécutions mesurées
       *[other] { $count } exécutions mesurées
    }
copy-execution-worst-slippage = Pire slippage
copy-execution-average-slippage = Moyenne { $amount }
copy-execution-delay-title = Délai de détection
copy-execution-delay-note = Temps écoulé entre le bloc du portefeuille et la détection du trade par ce bot. Les rejeux après une interruption sont exclus.
copy-execution-delay-limit = Les barres au-delà de la limite d'arrivée de { $limit } sont en orange.
copy-execution-fastest = Plus rapide
copy-execution-average = Moyenne
copy-execution-slowest = Plus lent
copy-execution-fill-title = Exécution par rapport au portefeuille
copy-execution-fill-note = Une valeur positive signifie moins bon que le portefeuille : payé plus cher à l'achat, reçu moins sur une vente répliquée. Une exécution simulée d'un token sans prix de pool est valorisée au trade du portefeuille lui-même, elle ne mesure donc rien et est exclue.
copy-execution-samples = Échantillons
copy-execution-median = Médiane
copy-execution-worst = Pire
copy-execution-decisions = Décisions sur la période

## Compare view (pages/copy/compare.js)

copy-compare-title = Comparer les portefeuilles
copy-compare-back = Retour au portefeuille
copy-compare-load-failed = La comparaison n'a pas pu être chargée : { $error }
copy-compare-loading = Chargement de la comparaison…
copy-compare-empty = Aucune tâche à comparer.
copy-compare-empty-message = Ajoutez une tâche de copie pour comparer ses résultats aux autres.
copy-compare-curve-title = P&L réalisé cumulé
copy-table-wallet = Portefeuille
copy-table-mode = Mode
copy-table-rounds = Cycles
copy-table-realized = Réalisé
copy-table-profit-factor = Facteur de profit
copy-table-average-hold = Détention moy.
copy-table-median-slippage = Slippage médian

## Charts (pages/copy/charts.js)

copy-chart-curve-label = P&L cumulé { $amount } { -sol }
copy-chart-compare-label = P&L cumulé par tâche
copy-chart-empty-curve = Aucun cycle clôturé sur cette période pour l'instant.
copy-chart-empty-bars = Rien d'enregistré sur cette période.
copy-chart-empty-histogram = Aucun échantillon d'arrivée sur cette période.
copy-chart-empty-compare = Aucun cycle clôturé à comparer sur cette période.
copy-chart-histogram-title = { $count } sur { $total }

## Wallet profile (pages/copy/profile.js)

copy-profile-copy = Copier ce portefeuille
copy-profile-copy-other = Copier avec d'autres règles
copy-profile-loading = Chargement du profil du portefeuille…
copy-profile-watch-title = Suivi
copy-profile-watched = Suivi actif
copy-profile-watch-resume-hint = Reprendre une tâche relance son suivi
copy-profile-watch-add-hint = Ajouter une tâche démarre son suivi
copy-profile-stream = Flux
copy-profile-subscribed = Abonné
copy-profile-not-subscribed = Non abonné
copy-profile-sources =
    { $count ->
        [one] { $count } source
        [many] { $count } sources
       *[other] { $count } sources
    }
copy-profile-last-activity = Dernière activité
copy-profile-last-error = Dernière erreur
copy-profile-own-wallet = Ce portefeuille fait partie des vôtres ; sa copie est refusée.
copy-profile-observed-title = Trades observés
copy-profile-observed-none = Aucun trade de ce portefeuille dans ce bot pour l'instant. Une tâche simulée l'observe sans dépenser de { -sol }.
copy-profile-swaps-seen = Swaps observés
copy-profile-swaps-seen-note = Swaps distincts du portefeuille sur l'ensemble de vos tâches
copy-profile-buys-sells = Achats / ventes
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Tokens échangés
copy-profile-first-seen = Première observation
copy-profile-last-seen = Dernière observation
copy-profile-tasks-title = Vos tâches sur ce portefeuille
copy-table-task = Tâche

## Arm live dialog (pages/copy/arm_gate.js)

copy-arm-acks-left =
    { $count ->
        [one] { $count } confirmation restante à cocher
        [many] { $count } confirmations restantes à cocher
       *[other] { $count } confirmations restantes à cocher
    }
copy-arm-readiness-title = Préparation issue du registre simulé
copy-arm-exposure-title = Exposition
copy-arm-per-copy = Par copie
copy-arm-budget-left-value = { $left } sur { $total } { -sol }
copy-arm-budget-left = Budget réel restant
copy-arm-budget-left-note = Les dépenses simulées sont comptées séparément et n'entament pas ce budget
copy-arm-exits = Sorties
copy-arm-stop-note = Pas avant une détention de { $hold } : une chute plus rapide clôture plus bas
copy-arm-shared = Ce portefeuille est aussi copié par { $tasks } : chaque tâche copie ses trades avec son propre budget.
copy-arm-unavailable = L'exécution réelle est indisponible pour le moment ; consultez la dernière vérification.
copy-arm-ack-real-native = { -sol } réels : cette tâche peut dépenser jusqu'à { $budget } { -sol } depuis votre portefeuille, au plus { $trade } { -sol } par copie.
copy-arm-ack-fees = Les copies réelles paient de vrais frais réseau et du slippage ; les résultats simulés ne garantissent pas les résultats réels.
copy-arm-ack-unready = Certains contrôles de préparation n'ont pas été validés. Armer quand même cette tâche.
copy-arm-lead = « { $name } » copiera les trades de ce portefeuille avec de vrais swaps depuis votre portefeuille.
copy-arm-confirmation-missing = La confirmation du mode réel n'a pas pu être chargée
copy-arm-armed = Copie réelle armée
copy-arm-failed = La copie réelle n'a pas pu être armée

## Holdings tab (pages/copy/holdings.js)

copy-holdings-title = Détentions
copy-holdings-view-label = Vue des détentions
copy-holdings-view-open = Ouvertes ({ $count })
copy-holdings-view-closed = Cycles clôturés ({ $count })
copy-holdings-reset = Réinitialiser le registre simulé
copy-holdings-live-note = Les copies réelles sont de vraies positions.
copy-holdings-open-positions = Positions ouvertes
copy-holdings-token-details = Ouvrir les détails du token
copy-holdings-opened = Ouverte le { $time }
copy-holdings-no-pool-price = Aucun prix de pool
copy-holdings-close = Clôturer
copy-holdings-write-off = Passer en perte
copy-holdings-activity = Activité
copy-holdings-no-exit-rule = Aucune règle de sortie
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } dans { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail armé à { $level }
copy-holdings-watch-time = Durée ≤ { $level }
copy-holdings-watch-time-until = Durée ≤ { $level } dans { $span }
copy-holdings-watch-wallet-sells = Ventes du portefeuille
copy-holdings-empty = Aucune détention simulée ouverte. Les achats copiés depuis le portefeuille apparaissent ici.
copy-holdings-col-token = Token
copy-holdings-col-cost = Coût
copy-holdings-col-entry = Entrée
copy-holdings-col-mark = Valorisation
copy-holdings-col-peak = Pic
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Règles de sortie
copy-holdings-col-held = Détenu
copy-holdings-col-actions = Actions
copy-holdings-col-invested = Investi
copy-holdings-col-proceeds = Produit
copy-holdings-col-exit = Sortie
copy-holdings-col-closed = Clôturé
copy-holdings-price-note = Les prix sont en { -sol } par token. L'entrée inclut le slippage et les frais de l'achat ; le pic et les niveaux de sortie sont relatifs à celle-ci, une détention s'ouvre donc avec son pic sous l'entrée. Survolez une valeur pour voir son prix de pool.
copy-holdings-paused-rules = En pause : aucune nouvelle copie. Vos règles de sortie clôturent toujours ces détentions.
copy-holdings-paused-mirror = En pause : aucune nouvelle copie. Les ventes du portefeuille clôturent toujours ces détentions.
copy-holdings-paused-hybrid = En pause : aucune nouvelle copie. Les ventes du portefeuille et vos règles de sortie clôturent toujours ces détentions.
copy-holdings-closed-load-failed = Les cycles clôturés n'ont pas pu être chargés : { $error }
copy-holdings-closed-loading = Chargement des cycles clôturés…
copy-holdings-closed-empty = Aucun cycle clôturé pour l'instant.
copy-holdings-closed-latest = { $shown } derniers cycles sur { $total }.
copy-holdings-close-title = Clôturer la détention simulée
copy-holdings-close-message = Vend { $token } dans le registre simulé au prix du pool ({ $price }) avec le slippage et les frais de la tâche.
copy-holdings-close-confirm = Clôturer la détention
copy-holdings-write-off-title = Passer la détention simulée en perte
copy-holdings-write-off-message = { $token } n'a aucun prix de pool auquel être vendu. Le passer en perte le clôture à zéro et comptabilise son coût de { $cost } comme une perte.
copy-holdings-keep = Conserver
copy-holdings-written-off = { $token } passé en perte
copy-holdings-closed = { $token } clôturé
copy-holdings-written-off-detail = Clôturé pour un produit nul
copy-holdings-sold-at = Vendu à { $price }
copy-holdings-close-failed = La détention n'a pas pu être clôturée
copy-holdings-reset-message = Repartir de zéro pour « { $name } » : ses détentions simulées, dépenses, exécutions, sorties et trades ignorés sont supprimés. Les règles et le portefeuille sont conservés.
copy-holdings-reset-cancel = Conserver l'historique
copy-holdings-reset-done = Registre simulé réinitialisé
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } décision supprimée
        [many] { $count } décisions supprimées
       *[other] { $count } décisions supprimées
    }
copy-holdings-reset-failed = Le registre simulé n'a pas pu être réinitialisé

## Activity tab (pages/copy/activity.js). Ids come from CopyOutcome
## (src/trader/copy/types.rs).

copy-activity-title = Activité
copy-activity-filter-label = Filtre d'activité
copy-filter-all = Tout
copy-outcome-paper-filled = Achat simulé
copy-outcome-live-submitted = Achat réel envoyé
copy-outcome-live-confirmed = Achat réel confirmé
copy-outcome-live-failed = Achat réel échoué
copy-outcome-paper-sell-observed = Vente simulée · vendu par le portefeuille
copy-outcome-live-sell-submitted = Vente réelle envoyée
copy-outcome-live-sell-failed = Vente réelle échouée
copy-outcome-skipped = Ignoré
copy-activity-decision = Décision
copy-activity-paper-exit = Sortie simulée · { $rule }
copy-activity-filled = { $input } à { $price } · le portefeuille a acheté { $target }
copy-activity-filled-slippage = { $input } à { $price } · le portefeuille a acheté { $target } · slippage { $slippage }
copy-activity-filled-unpriced = { $input } à { $price } · le portefeuille a acheté { $target } · valorisé au trade du portefeuille, sans prix de pool
copy-activity-live-sized = { $sized } · le portefeuille a acheté { $target }
copy-activity-sell-nothing = Le portefeuille a vendu { $amount } · rien à vendre en détention
copy-activity-written-off = Passé en perte à zéro : aucun prix de pool
copy-activity-sold = { $tokens } tokens pour { $proceeds } à { $price }
copy-activity-full-close = Clôture complète
copy-activity-partial-exit = Sortie de { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = minimum { $amount }
copy-activity-skip-maximum = maximum { $value }
copy-activity-skip-stale = { $arrival } de retard, limite { $limit }
copy-activity-skip-latency = { $average } en moyenne, limite { $limit }
copy-activity-arrival-replayed = Rejoué { $span } après le bloc
copy-activity-arrival-seen = Vu { $span } après le bloc
copy-activity-link-wallet-tx = Tx du portefeuille
copy-activity-link-own-tx = Votre tx
copy-activity-only-token = Ce token uniquement
copy-activity-skipped-group = Ignorés ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } token · depuis le { $since }
        [many] { $tokens } tokens · depuis le { $since }
       *[other] { $tokens } tokens · depuis le { $since }
    }
copy-activity-mint-filter =
    .placeholder = Adresse de mint du token
    .aria-label = Filtrer par adresse de mint du token
copy-activity-clear = Effacer
copy-activity-load-failed = L'activité n'a pas pu être chargée : { $error }
copy-activity-loading = Chargement de l'activité…
copy-activity-no-match = Rien ne correspond à ce filtre.
copy-activity-empty = Aucune décision pour l'instant. Les exécutions, sorties et trades ignorés apparaissent ici au fil des trades du portefeuille.
copy-activity-load-older = Charger plus ancien
copy-activity-start = Début de l'historique
copy-activity-older-failed = L'activité plus ancienne n'a pas pu être chargée

## Task editor (pages/copy/editor.js, pages/copy/editor_steps.js)

copy-step-wallet = Portefeuille
copy-step-sizing = Dimensionnement
copy-step-entry = Filtres d'entrée
copy-step-exits = Sorties
copy-step-review = Récapitulatif
copy-editor-title-edit = Modifier { $name }
copy-editor-title-clone = Dupliquer { $name }
copy-editor-sub-edit = Tâche { $mode } · les modifications s'appliquent à ses prochaines décisions
copy-editor-sub-clone = Mêmes règles, registre simulé vide, démarre en simulé
copy-editor-save-edit = Enregistrer les modifications
copy-editor-save-clone = Créer le duplicata
copy-editor-save-create = Créer la tâche simulée
copy-editor-clone-suffix = (copie)
copy-editor-discard-edit = Abandonner les modifications
copy-editor-discard-create = Abandonner cette tâche
copy-editor-discard-edit-message = Vos modifications de « { $name } » ne sont pas enregistrées.
copy-editor-discard-create-message = Le portefeuille et les règles saisis jusqu'ici ne sont pas enregistrés.
copy-editor-discard-confirm = Abandonner
copy-editor-keep-editing = Continuer la modification
copy-editor-toast-updated = Tâche mise à jour
copy-editor-toast-clone = Duplicata créé
copy-editor-toast-created = Tâche simulée créée
copy-unit-native = { -sol }
copy-editor-any = Toutes
copy-editor-duplicate = Déjà copié par { $tasks }. Cette tâche copie de nouveau les mêmes trades, avec ses propres règles et son propre budget.
copy-editor-wallet = Portefeuille
copy-editor-wallet-identity = Le portefeuille d'une tâche est son identité. Pour copier un autre portefeuille avec ces règles, dupliquez la tâche.
copy-editor-address-label = Adresse du portefeuille
copy-editor-address-placeholder = Adresse de portefeuille Solana
copy-editor-address-help-clone = Mêmes règles avec un registre simulé vide. Conservez ce portefeuille pour y tester d'autres règles, ou saisissez-en un autre.
copy-editor-address-help-create = Le portefeuille dont cette tâche copie les achats (et, si vous le choisissez, les ventes).
copy-editor-name-label = Nom <em>facultatif</em>
copy-editor-name-placeholder = p. ex. Rotation rapide
copy-editor-enabled-title = Traiter les trades du portefeuille
copy-editor-enabled-help = Désactivé, la tâche reste en pause jusqu'à sa reprise.
copy-editor-note-live = Cette tâche est en réel : les modifications s'appliquent à ses prochaines copies réelles.
copy-editor-note-paper = Les tâches tournent en simulé jusqu'à ce que vous les armiez : les trades sont simulés au prix du pool et rien n'est dépensé.
copy-editor-copy-size = Taille de copie
copy-editor-sizing-fixed = Montant fixe
copy-editor-sizing-ratio = Part du trade du portefeuille
copy-editor-amount-fixed = Montant par copie
copy-editor-amount-ratio = Part de chaque trade
copy-editor-amount-help-fixed = Dépensé pour chaque achat copié, au moins { $minimum }.
copy-editor-amount-help-ratio = De l'achat du portefeuille lui-même, dans la limite du plafond par trade.
copy-editor-help-trade-cap = Aucune copie ne dépense davantage.
copy-editor-help-token-cap = Total dépensé sur un même token.
copy-editor-help-budget = Tout ce que cette tâche peut dépenser sur sa durée de vie ; le simulé et le réel comptent chacun leurs propres dépenses.
copy-editor-preview-title = Ce que coûte une copie
copy-editor-preview-empty = Saisissez le dimensionnement pour voir ce que coûte une copie.
copy-editor-preview-example = Le portefeuille achète { $target } → vous copiez <strong>{ $copy }</strong>
copy-editor-preview-once = Un token prend une seule copie de { $size }, chaque token n'étant acheté qu'une fois
copy-editor-preview-token-cap =
    { $count ->
        [one] Un token prend au plus { $count } copie de { $size }
        [many] Un token prend au plus { $count } copies de { $size }
       *[other] Un token prend au plus { $count } copies de { $size }
    }
copy-editor-preview-summary-exact = { $perToken } ; le budget en couvre environ { $count }. Les frais réseau et de priorité s'ajoutent.
copy-editor-preview-summary-minimum = { $perToken } ; le budget en couvre au moins { $count }. Les frais réseau et de priorité s'ajoutent.
copy-editor-target-min = Plus petit trade du portefeuille copié
copy-editor-target-min-help = Ignore les plus petits achats du portefeuille. Laissez vide pour ne fixer aucun minimum.
copy-editor-target-max = Plus gros trade du portefeuille copié
copy-editor-target-max-help = Ignore les plus gros achats du portefeuille. Laissez vide pour ne fixer aucun maximum.
copy-editor-buy-once-title = Acheter chaque token une fois
copy-editor-buy-once-help = Ne copie que le premier achat d'un token par le portefeuille ; ses achats suivants sont ignorés.
copy-editor-filter-require = Exiger
copy-editor-filter-skip = Ne pas exiger
copy-editor-filter-help = Exige qu'un token passe votre pipeline de filtrage avant d'être copié.
copy-editor-filter-warning = Avec la configuration de filtrage par défaut, presque tous les tokens échouent, une tâche qui exige la validation ne copie donc rien. Ne l'exigez que si vos filtres valident les tokens échangés par ce portefeuille.
copy-editor-exit-both = Les deux
copy-editor-exit-help-buy-only = Vos règles ci-dessous vendent chaque détention ; les ventes du portefeuille sont ignorées.
copy-editor-exit-help-hybrid = Le premier des deux : le portefeuille vend, ou l'une de vos règles se déclenche.
copy-editor-exit-help-mirror = Les détentions ne sont vendues que lorsque le portefeuille vend. Vos règles de sortie ne s'appliquent pas.
copy-editor-who-sells = Qui vend
copy-editor-preset = Préréglage
copy-editor-preset-help = Un préréglage renseigne toutes les règles ci-dessous ; vous pouvez ensuite en ajuster chacune.
copy-editor-mirror-note = Ces règles ne s'appliquent pas tant que les ventes du portefeuille décident. Elles s'appliquent si vous passez à { $mine } ou { $both }.
copy-editor-rule-inherit = Valeur par défaut du Trader
copy-editor-inherit-value = Valeur par défaut du Trader ({ $value })
copy-editor-rule-aria = Réglage { $rule }
copy-editor-rule-empty-uses = Vide, la valeur par défaut du Trader est utilisée : { $value }
copy-editor-rule-follows = Suit le Trader : { $summary }
copy-editor-rule-follows-plain = Suit le réglage du Trader.
copy-editor-rule-follows-own = Suit l'interrupteur du Trader avec les valeurs de cette tâche : { $summary }
copy-editor-rule-off-note = Désactivée pour cette tâche, quel que soit le réglage du Trader.
copy-task-unnamed = Tâche sans nom
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = traite les trades une fois enregistrée
copy-editor-review-paused = enregistrée en pause
copy-editor-error-address = Saisissez une adresse de portefeuille Solana valide.
copy-editor-error-sizing = Chaque valeur de dimensionnement doit être supérieure à zéro.
copy-editor-error-min-copy = Une copie doit être d'au moins { $minimum } : augmentez le montant par copie.
copy-editor-error-min-cap = Une copie doit être d'au moins { $minimum } : augmentez le plafond par trade.
copy-editor-error-trade-cap = Le plafond par trade ne peut pas dépasser le plafond par token.
copy-editor-error-token-cap = Le plafond par token ne peut pas dépasser le budget total.
copy-editor-error-slippage = Le slippage doit être compris entre { $min } et { $max }.
copy-editor-error-target-limits = Les limites de trade du portefeuille doivent être supérieures ou égales à zéro.
copy-editor-error-target-order = Le plus petit trade du portefeuille ne peut pas dépasser le plus gros.

## Copy notices: toasts, the event log and Telegram (trader/copy/notify.rs).

copy-notice-task-unnamed = Tâche n° { $id }
copy-notice-heading = { $task } : { $title }
copy-notice-event = { $task } : { $title } — { $detail }
copy-notice-title-paper-buy = Achat de copie simulé
copy-notice-title-paper-sell = Vente de copie simulée
copy-notice-title-paper-closed = Détention simulée clôturée
copy-notice-title-paper-exit = Sortie simulée : { $rule }
copy-notice-title-live-buy-submitted = Achat de copie réel envoyé
copy-notice-title-live-buy-confirmed = Achat de copie réel confirmé
copy-notice-title-live-buy-failed = Achat de copie réel échoué
copy-notice-title-live-sell-submitted = Vente de copie réelle envoyée
copy-notice-title-live-sell-failed = Vente de copie réelle échouée
copy-notice-title-auto-paused = Tâche de copie mise en pause automatiquement
copy-notice-detail-bought = Acheté pour { $amount } { -sol }
copy-notice-detail-sold = Vendu pour { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent } % de la détention
copy-notice-detail-full-close = Clôture complète
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Le swap a échoué
