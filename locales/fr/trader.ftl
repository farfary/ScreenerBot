trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Take profit
trader-exit-type-roi-exit = Take profit
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Sortie temporelle
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
trader-tab-take-profit = Take profit
trader-tab-time-rules = Sortie temporelle
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
trader-setup-action = Terminer la configuration
trader-toggle-on = ON
trader-toggle-off = OFF
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
trader-force-stop-hint = Arrête d’un coup tous les moniteurs, tâches de copie et trades.
trader-force-stop-idle = Rien ne tourne qu’un arrêt forcé pourrait interrompre.

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
trader-toast-strategy-armed = { $name } trade désormais en réel
trader-toast-strategy-paper = { $name } trade désormais en simulé
trader-toast-strategy-mode-failed = Impossible de changer le mode de trading

trader-stats-window =
    .aria-label = Période des statistiques
trader-stats-book =
    .aria-label = Registre de trading
trader-paper-run-since = Session simulée depuis le { $date }
trader-paper-start = Départ
trader-paper-cash = Liquidités
trader-paper-open = Ouvert
trader-paper-unrealized = Latent
trader-paper-reset = Réinitialiser la session simulée
trader-paper-reset-title = Réinitialiser la session simulée ?
trader-paper-reset-run-message = Chaque position simulée ouverte, y compris les détentions simulées des tâches de copie, est clôturée à son prix actuel et une nouvelle session démarre avec le solde initial configuré. Les sessions précédentes restent dans l'historique.
trader-paper-reset-done = Une nouvelle session simulée a démarré
trader-paper-reset-failed = Impossible de réinitialiser la session simulée
trader-paper-account-failed = Impossible de charger la session simulée
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
trader-weekly-title = P&L hebdomadaire
trader-weekly-subtitle = { -sol } réalisés par semaine ISO, avec le cumul
trader-weekly-chart = Profits et pertes réalisés par semaine en { -sol }
trader-extreme-best = Meilleur trade
trader-extreme-worst = Pire trade

trader-exit-title = Répartition des stratégies de sortie
trader-exit-subtitle = Comment les positions ont été clôturées et ce que chaque sortie a rapporté
trader-exit-loading = Chargement des données de sortie...
trader-exit-empty-day = Aucun trade clôturé au cours des dernières 24 heures
trader-exit-empty-all = Aucun trade clôturé pour l’instant
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
trader-stop-loss-sell-part = Vendre { $part }
trader-stop-loss-part-of-position = Une partie de la position
trader-stop-loss-sell-rest = Vendre le reste
trader-stop-loss-still-below = Toujours sous le stop
trader-stop-loss-summary = Perte limitée à <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Remarque :</strong> le stop loss protège contre des pertes plus importantes en sortant tôt

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Protégez automatiquement vos profits en suivant le prix à la hausse
trader-trailing-activation-impact = Début du suivi à +{ $value } % de profit
trader-trailing-distance-impact = Sortie à -{ $value } % depuis le pic
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Minimum verrouillé à l’activation : { $value }
trader-trailing-locked-minimum-warning = Le stop se place au prix d’entrée ou en dessous à l’activation ; il ne vend qu’en gain. Activez le stop au point mort pour garder un plancher.
trader-trailing-activation = Activation
trader-trailing-peak = Pic
trader-trailing-final = +{ $value } % final
trader-trailing-summary-protected = Profit de <strong>{ $value }</strong> protégé
trader-trailing-summary-avoided = Perte de <strong>{ $value }</strong> évitée depuis le pic
# $step is the band's position in the list, from 1.
trader-trailing-band = Palier { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Profit maximal du palier { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Écart du palier { $step }
trader-trailing-band-profit-caption = Dès un profit maximal de
trader-trailing-band-distance-caption = Suivre à
trader-trailing-add-band = Ajouter un palier
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Supprimer le palier { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = Le profit maximal de chaque palier doit être compris entre { $min } % et { $max } %
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = L'écart de chaque palier doit être compris entre { $min } % et { $max } %
trader-trailing-band-error-order = Chaque palier doit avoir un profit maximal supérieur à l'activation et au palier précédent
trader-trailing-band-error-distance = Chaque palier doit suivre à un écart inférieur à son profit maximal

## Break-even stop

trader-break-even-title = Stop au point mort
trader-break-even-subtitle = Une fois la position en gain, tout vendre avant qu’elle ne passe en perte
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Dès que le meilleur prix depuis le dernier achat atteint { $trigger }, un retour à { $offset } vend tout.

trader-take-profit-title = Take profit
trader-take-profit-subtitle = Vend la position par paliers à mesure que le gain atteint chaque niveau
trader-take-profit-initial-buy = Achat initial
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Palier { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Niveau de gain du palier { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Part à vendre du palier { $step }
trader-take-profit-profit-caption = Niveau de gain
trader-take-profit-sell-caption = Part à vendre
trader-take-profit-add-step = Ajouter un palier
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Retirer le palier { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Chaque niveau de gain doit être compris entre { $min }% et { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Chaque part doit être comprise entre { $min }% et { $max }%
trader-take-profit-error-order = Chaque palier doit avoir un niveau de gain supérieur au précédent
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Les paliers peuvent vendre au total { $max }% au maximum
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% de la position restent ouverts comme runner
trader-take-profit-runner-none = Le dernier palier vend le reste de la position
trader-take-profit-runner-warning = Avec le trailing stop, le stop au point mort, les deux sorties temporelles et toutes les stratégies de sortie désactivés, le runner ne se ferme que par le stop loss ou à la main
trader-take-profit-info = Les parts portent sur la position initiale. Si le prix franchit plusieurs niveaux d'un coup, un palier est vendu par vérification.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = À +{ $profit }%, vendre { $share }%
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Laissé aux autres règles de sortie
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Vend <strong>{ $sold }%</strong> d'ici +{ $profit }% de gain

trader-time-loss-title = Fermer les positions en perte
trader-time-subtitle = Sortez automatiquement des positions après une durée de détention maximale si la perte dépasse le seuil
trader-time-flat-title = Fermer les positions stagnantes
trader-time-flat-subtitle = Libérer le capital des positions qui ne bougent ni à la hausse ni à la baisse
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Une position détenue { $span } ou plus avec un P&L compris dans ±{ $band } est fermée.
trader-time-unit-seconds = secondes
trader-time-unit-minutes = minutes
trader-time-unit-hours = heures
trader-time-unit-days = jours
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
trader-time-loss-hint = La perte à laquelle la sortie temporelle vend une fois la durée de détention écoulée (40 = sortie à -40 %)
trader-time-day = Jour { $day }
trader-time-position-opened = Position ouverte
trader-time-limit = Limite de temps
trader-time-hold-reached = Période de détention atteinte
trader-time-loss-met = Seuil de perte atteint
trader-time-note = <strong>Remarque :</strong> les positions en profit ou avec une perte plus faible ne seront PAS clôturées
trader-time-positions-title = État des positions actuelles
trader-time-positions-loading = Chargement des positions...
trader-time-positions-empty = Aucune position ouverte
trader-time-positions-empty-message = Les positions ouvertes apparaissent ici avec leur durée de détention et leur ROI par rapport à la limite de temps.
trader-time-positions-token = Token
trader-time-positions-hold = Durée de détention
trader-time-positions-roi = ROI
trader-time-positions-rule = Sortie temporelle
trader-time-status-off = Les sorties temporelles sont désactivées
trader-time-status-none = Aucune sortie temporelle ne s’applique pour l’instant
trader-time-status-not-managed = Non gérée par Auto Trader
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = En perte de { $pnl } ; fermeture dans { $remaining }
trader-time-status-losing-due = En perte de { $pnl } ; fermeture au prochain contrôle
# $held and $remaining are formatted durations.
trader-time-status-flat = Stagnante depuis { $held } ; fermeture dans { $remaining }
trader-time-status-flat-due = Stagnante depuis { $held } ; fermeture au prochain contrôle

trader-strategy-entry-title = Stratégies d'entrée
trader-strategy-entry-subtitle = Signaux pouvant ouvrir une nouvelle position.
trader-strategy-exit-title = Stratégies de sortie
trader-strategy-exit-subtitle = Signaux pouvant clôturer ou protéger une position ouverte.
trader-strategy-active-unknown = -- actives
trader-strategy-active = { $enabled }/{ $total } actives
trader-strategy-loading = Chargement des stratégies...
trader-strategy-load-failed = Impossible de charger les stratégies
trader-strategy-empty = Aucune stratégie définie
trader-strategy-unnamed = Stratégie sans nom
trader-strategy-priority-auto = Auto
trader-strategy-priority = Priorité { $priority }
trader-strategy-mode-arm = Armer le mode réel
trader-strategy-mode-return-paper = Revenir en simulé
trader-strategy-arm-title = Armer « { $name } » en réel
trader-strategy-arm-lead = Les nouvelles entrées de « { $name } » passeront par de vrais swaps depuis votre portefeuille de trading.
trader-strategy-arm-readiness = Préparation issue du registre simulé
trader-strategy-arm-open-paper = Les positions simulées ouvertes restent simulées et se terminent en simulé.
trader-strategy-arm-ack-real-native = Les nouvelles entrées dépensent de vrais { -sol } depuis votre portefeuille de trading.
trader-strategy-arm-ack-fees = Les swaps réels paient de vrais frais réseau et du slippage ; les résultats simulés ne garantissent pas les résultats réels.
trader-strategy-arm-ack-unready = Certains contrôles de préparation n'ont pas été validés. Armer quand même cette stratégie.
trader-strategy-arm-unavailable = Le trading réel est indisponible pour le moment ; consultez la dernière vérification.
trader-strategy-return-title = Repasser « { $name } » en simulé
trader-strategy-return-message = Les nouvelles entrées s'ouvrent dans le registre simulé sans dépenser de { -sol }. Les positions réelles ouvertes restent réelles et se terminent en réel.
trader-strategy-readiness-failed = Impossible de lire la préparation de la stratégie
trader-paper-explore-title = Le trading simulé démarre après la configuration

trader-dca-title = Investissement programmé (DCA)
trader-dca-subtitle = Renforcez automatiquement les positions perdantes pour abaisser votre prix d'entrée moyen
trader-dca-measure-average = Entrée moyenne
trader-dca-measure-first-entry = Première entrée
trader-dca-step = Renfort { $step }
trader-dca-drop-label = Baisse de prix du renfort { $step }
trader-dca-size-label = Taille du renfort { $step }, en fois le premier achat
trader-dca-drop-caption = Baisse de prix
trader-dca-size-caption = Achat × premier achat
trader-dca-add-step = Ajouter un palier
trader-dca-remove-step = Retirer le renfort { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Sous le trade minimum de { $minimum } : ce renfort n'est pas envoyé
trader-dca-error-drop-range = Chaque baisse doit être comprise entre { $min } % et { $max } %
trader-dca-error-size-range = Chaque taille doit être comprise entre { $min }× et { $max }× le premier achat
trader-dca-error-order = Mesuré depuis la première entrée, chaque renfort doit avoir une baisse plus profonde que le précédent
trader-dca-cooldown-impact = Au moins { $span } entre deux entrées supplémentaires
trader-dca-min-hold-none = Le premier renfort peut suivre l'entrée immédiatement
trader-dca-min-hold-impact = Le premier renfort attend au moins { $span } après l'entrée
trader-dca-capacity = Le maximum que cette position peut détenir : { $amount } ({ $multiple }× le premier achat)
trader-dca-info-title = Infos sur la stratégie DCA
trader-dca-info-subtitle = Points importants à considérer pour le DCA
trader-dca-how-title = Fonctionnement du DCA
trader-dca-how-trigger = <strong>Déclencheur :</strong> le prix passe sous le palier suivant, mesuré depuis l'entrée moyenne ou la première entrée
trader-dca-how-action = <strong>Action :</strong> ajout de { -sol } pour réduire le prix de revient moyen
trader-dca-how-repeat = <strong>Échelle :</strong> chaque renfort passe au palier suivant ; les renforts s'arrêtent une fois tous les paliers utilisés
trader-dca-how-pause = <strong>Pauses :</strong> aucun renfort tant que le précédent est en cours de règlement, ni tant que la limite de pertes bloque les nouvelles entrées
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
