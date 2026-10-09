trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Objectif de ROI
trader-exit-type-roi-exit = Objectif de ROI
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Forçage temporel
trader-exit-type-time-rule = Règle temporelle
trader-exit-type-manual = Manuelle
trader-exit-type-manual-close = Manuelle
trader-exit-type-dca = DCA
trader-exit-type-unknown = Inconnue

trader-tab-stats = Statistiques
trader-tab-strategy-control = Contrôle des stratégies
trader-tab-strategies = Stratégies
trader-tab-stop-loss = Stop loss
trader-tab-trailing-stop = Trailing stop
trader-tab-roi = Take profit
trader-tab-time-rules = Règles temporelles
trader-tab-dca = DCA
trader-tab-settings = Paramètres

trader-feature-coming-soon = Bientôt disponible
    .message = Cette fonctionnalité sera bientôt disponible.
trader-feature-beta = Bêta
trader-feature-disabled = Désactivée
    .message = Cette fonctionnalité est actuellement désactivée.

trader-status-title = Auto Trader
trader-status-loading = Chargement...
trader-status-running = En marche
trader-status-stopped = Arrêté
trader-status-setup-required = Configuration requise
trader-status-unavailable = Terminez la configuration du portefeuille et du RPC pour utiliser Auto Trader
trader-toggle-on = ON
trader-toggle-off = OFF
trader-toggle-unavailable = INDISPONIBLE
trader-toggle-start-failed = Impossible de démarrer le trader
trader-toggle-stop-failed = Impossible d'arrêter le trader
trader-controls-title = Contrôles de trading
trader-halt-title = TRADING SUSPENDU
trader-halt-reason-default = Arrêt forcé manuel
trader-halt-resume = Reprendre
trader-monitor-entry = Moniteur d'entrée
trader-monitor-exit = Moniteur de sortie
trader-monitor-master-off = Auto Trader désactivé
trader-loss-limit-title = Limite de perte par période
trader-loss-limit-resume = Reprendre le trading
trader-loss-limit-reset = Réinitialiser la période
trader-loss-limit-off = Désactivée
trader-loss-limit-none = Aucune limite de perte par période configurée
trader-loss-limit-resets-in = Réinitialisation dans { $hours } { $minutes }
trader-loss-limit-reached = LIMITE ATTEINTE
trader-force-stop = Tout arrêter de force

trader-force-stop-confirm = Arrêt forcé du trading
    .message = Cette action suspend immédiatement TOUTES les opérations de trading. Continuer ?
    .confirm = Arrêter le trading
trader-loss-limit-resume-confirm = Reprendre après la limite de perte
    .message = La limite de perte par période a bloqué les nouvelles entrées. Reprendre permet au trader d'ouvrir de nouveau des positions avant la réinitialisation de la période. Continuer ?
trader-loss-limit-reset-confirm = Réinitialiser la période de limite de perte
    .message = Cette action efface la perte cumulée de la période en cours et en démarre une nouvelle. Continuer ?

trader-toast-control-failed = Échec du contrôle d'Auto Trader
trader-toast-force-stop-on = Arrêt forcé activé
trader-toast-force-stop-failed = Impossible d'activer l'arrêt forcé
trader-toast-force-stop-cleared = Arrêt forcé levé
trader-toast-resume-failed = Impossible de reprendre le trading
trader-toast-loss-limit-reset-failed = Impossible de réinitialiser la limite de perte
trader-toast-entry-monitor-failed = Impossible de basculer le moniteur d'entrée
trader-toast-exit-monitor-failed = Impossible de basculer le moniteur de sortie
trader-toast-load-failed = Échec du chargement
    .message = Impossible de charger la configuration du trader
trader-toast-saved = Configuration enregistrée
    .message = Paramètres du trader appliqués avec succès
trader-toast-save-failed = Échec de l'enregistrement
    .message = Impossible d'enregistrer la configuration du trader
trader-toast-feature-enabled = Fonctionnalité activée
trader-toast-feature-disabled = Fonctionnalité désactivée
trader-toast-feature-applied = Paramètre d'Auto Trader appliqué
trader-toast-strategy-enabled = Stratégie activée
    .message = La stratégie est active
trader-toast-strategy-disabled = Stratégie désactivée
    .message = La stratégie est inactive
trader-toast-strategy-failed = Échec de la mise à jour
    .message = Impossible de mettre à jour le statut de la stratégie

trader-stats-window =
    .aria-label = Période des statistiques
trader-stats-window-day = 24h
trader-stats-window-week = 7j
trader-stats-window-month = 30j
trader-realized-title = Performance réalisée
trader-metric-net-pnl = P&L net
trader-metric-win-rate = Taux de réussite
trader-metric-profit-factor = Facteur de profit
trader-metric-max-drawdown = Drawdown max
trader-metric-capital = Capital engagé
trader-metric-avg-win-loss = Gain / perte moy.
trader-metric-closed-trades = Trades clôturés
trader-metric-median-hold = Durée médiane de détention
trader-stats-empty = Aucun trade clôturé sur cette période
trader-stats-won-lost = { $won } gagnés · { $lost } perdus
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } gain
        [many] { $amount } gains
       *[other] { $amount } gains
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } perte
        [many] { $amount } pertes
       *[other] { $amount } pertes
    }
trader-stats-expected = { $amount } attendus par trade
trader-stats-profit-factor-basis = Gains bruts ÷ pertes brutes
trader-stats-drawdown-basis = Plus forte baisse réalisée du pic au creux
trader-stats-slots =
    { $count ->
        [one] { $used } emplacement de position utilisé sur { $max }
        [many] { $used } emplacements de position utilisés sur { $max }
       *[other] { $used } emplacements de position utilisés sur { $max }
    }
trader-stats-avg-basis = Résultat moyen d'un trade gagnant par rapport à un trade perdant
trader-stats-closed =
    { $count ->
        [one] { $amount } position clôturée
        [many] { $amount } positions clôturées
       *[other] { $amount } positions clôturées
    }
trader-stats-hold-average = { $span } en moyenne
trader-stats-excluded =
    { $count ->
        [one] { $amount } cycle clôturé exclu : prix de revient incomplet, donc pas de P&L fiable.
        [many] { $amount } cycles clôturés exclus : prix de revient incomplet, donc pas de P&L fiable.
       *[other] { $amount } cycles clôturés exclus : prix de revient incomplet, donc pas de P&L fiable.
    }

trader-daily-title = P&L quotidien
trader-daily-subtitle = { -sol } réalisés par jour, avec le cumul
trader-daily-loading = Chargement du P&L quotidien...
trader-daily-chart = Profits et pertes réalisés par jour en { -sol }
trader-extreme-best = Meilleur trade
trader-extreme-worst = Pire trade

trader-exit-title = Répartition des stratégies de sortie
trader-exit-subtitle = Comment les positions ont été clôturées et ce que chaque sortie a rapporté
trader-exit-loading = Chargement des données de sortie...
trader-exit-empty-day = Aucun trade clôturé au cours des dernières 24 heures
trader-exit-empty-days =
    { $count ->
        [one] Aucun trade clôturé au cours du dernier jour ({ $amount })
        [many] Aucun trade clôturé au cours des { $amount } derniers jours
       *[other] Aucun trade clôturé au cours des { $amount } derniers jours
    }
trader-exit-share =
    { $count ->
        [one] { $amount } trade · { $share } des sorties
        [many] { $amount } trades · { $share } des sorties
       *[other] { $amount } trades · { $share } des sorties
    }
trader-exit-average = { $value } en moy.

trader-impact-label = Impact :
trader-current-label = Actuel :
trader-readable-label = Lisible :
trader-example-how-it-works = Fonctionnement
trader-step-entry = Entrée
trader-step-initial-position = Position initiale
trader-step-auto-exit = Sortie auto
trader-step-exit = Sortie
trader-step-full-exit = Sortie complète de la position
trader-value-percent = { $value } %
trader-example-profit = +{ $value } % de profit

trader-stop-loss-title = Stop loss
trader-stop-loss-subtitle = Sortez automatiquement d'une position lorsque sa perte dépasse votre seuil
trader-stop-loss-impact = Sortie en cas de baisse de { $threshold } % depuis l'entrée
trader-stop-loss-hold-immediate = Immédiat
trader-stop-loss-hold-delay = Délai de { $span }
trader-stop-loss-price-falls = Le prix baisse
trader-stop-loss-threshold-reached = Seuil atteint
trader-stop-loss-partial = Sorties partielles autorisées
trader-stop-loss-summary = Perte limitée à <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Remarque :</strong> le stop loss protège contre des pertes plus importantes en sortant tôt

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Protégez automatiquement vos profits en suivant le prix à la hausse
trader-trailing-activation-impact = Début du suivi à +{ $value } % de profit
trader-trailing-distance-impact = Sortie à -{ $value } % depuis le pic
trader-trailing-activation = Activation
trader-trailing-peak = Pic
trader-trailing-final = +{ $value } % final
trader-trailing-summary-protected = Profit de <strong>{ $value }</strong> protégé
trader-trailing-summary-avoided = Perte de <strong>{ $value }</strong> évitée depuis le pic

trader-roi-title = Take profit
trader-roi-subtitle = Sortez automatiquement de toute la position lorsque le profit atteint votre objectif
trader-roi-impact = Sortie à +{ $target } % de profit
trader-roi-example-title = Scénario d'exemple
trader-roi-initial-buy = Achat initial
trader-roi-target-hit = Objectif atteint
trader-roi-full-position = Position entière
trader-roi-sold = 100 % vendu
trader-roi-summary = Profit de <strong>+{ $target } %</strong> sécurisé

trader-time-title = Sortie temporelle
trader-time-subtitle = Sortez automatiquement des positions après une durée de détention maximale si la perte dépasse le seuil
trader-time-unit-seconds = secondes
trader-time-unit-minutes = minutes
trader-time-unit-hours = heures
trader-time-unit-days = jours
trader-time-conversion-default = 168 heures = 7 jours
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } seconde
        [many] { $amount } secondes
       *[other] { $amount } secondes
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } minute
        [many] { $amount } minutes
       *[other] { $amount } minutes
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } heure
        [many] { $amount } heures
       *[other] { $amount } heures
    }
trader-duration-days =
    { $count ->
        [one] { $amount } jour
        [many] { $amount } jours
       *[other] { $amount } jours
    }
trader-time-loss-impact = Sortie en cas de baisse de { $value } % ou plus après la période de détention
trader-time-day = Jour { $day }
trader-time-position-opened = Position ouverte
trader-time-limit = Limite de temps
trader-time-hold-reached = Période de détention atteinte
trader-time-loss-met = Seuil de perte atteint
trader-time-note = <strong>Remarque :</strong> les positions en profit ou avec une perte plus faible ne seront PAS clôturées
trader-time-positions-title = État des positions actuelles
trader-time-positions-loading = Chargement des positions...
trader-time-positions-empty = Aucune position ouverte
trader-time-positions-hold = Durée de détention :
trader-time-positions-roi = ROI :

trader-strategy-entry-title = Stratégies d'entrée
trader-strategy-entry-subtitle = Signaux pouvant ouvrir une nouvelle position.
trader-strategy-exit-title = Stratégies de sortie
trader-strategy-exit-subtitle = Signaux pouvant clôturer ou protéger une position ouverte.
trader-strategy-active-unknown = -- actives
trader-strategy-active = { $enabled }/{ $total } actives
trader-strategy-loading = Chargement des stratégies...
trader-strategy-load-failed = Impossible de charger les stratégies
trader-strategy-empty = Aucune stratégie définie
trader-strategy-no-description = Aucune description fournie.
trader-strategy-unnamed = Stratégie sans nom
trader-strategy-priority-auto = Auto
trader-strategy-priority = Priorité { $priority }

trader-dca-title = Investissement programmé (DCA)
trader-dca-subtitle = Renforcez automatiquement les positions perdantes pour abaisser votre prix d'entrée moyen
trader-dca-example-title = Exemple de DCA
trader-dca-example = 0.01 { -sol } initial → DCA n°1 : 0.005 { -sol } à -10 % → DCA n°2 : 0.005 { -sol } à -10 % supplémentaires
trader-dca-info-title = Infos sur la stratégie DCA
trader-dca-info-subtitle = Points importants à considérer pour le DCA
trader-dca-how-title = Fonctionnement du DCA
trader-dca-how-trigger = <strong>Déclencheur :</strong> la position passe sous le seuil de DCA (p. ex. -10 %)
trader-dca-how-action = <strong>Action :</strong> ajout de { -sol } pour réduire le prix de revient moyen
trader-dca-how-repeat = <strong>Répétition :</strong> le DCA peut se répéter jusqu'au nombre maximal défini
trader-dca-risk-title = Avertissements sur les risques
trader-dca-risk-exposure = <strong>Exposition accrue :</strong> le DCA augmente le capital total à risque par position
trader-dca-risk-knife = <strong>Couteau qui tombe :</strong> le DCA n'aide pas si le token poursuit sa baisse
trader-dca-risk-cooldown = <strong>Délai de récupération :</strong> utilisez-le pour éviter des entrées DCA en rafale

trader-sizing-title = Taille des positions
trader-sizing-subtitle = Définissez le montant à investir par position
trader-timing-title = Timing et délais
trader-timing-subtitle = Contrôlez le délai entre les opérations
trader-timing-close-cooldown = Délai après clôture de position
trader-timing-close-cooldown-hint = Minutes d'attente avant de rouvrir le même token
trader-timing-concurrency = Concurrence des vérifications d'entrée
trader-timing-concurrency-hint = Nombre de tokens vérifiés simultanément (plus élevé = plus rapide mais plus de CPU)
trader-timing-unit-minutes = min
trader-timing-unit-tokens = tokens
