trader-tab-stats = Statistiques
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
trader-status-running = En marche
trader-status-stopped = Arrêté
trader-status-setup-required = Configuration requise
trader-setup-action = Terminer la configuration
trader-toggle-start-failed = Impossible de démarrer le trader
trader-toggle-stop-failed = Impossible d'arrêter le trader
trader-halt-title = Trading suspendu
trader-halt-reason-default = Arrêt forcé manuel
trader-halt-resume = Reprendre
trader-monitor-entry = Moniteur d'entrée
trader-monitor-exit = Moniteur de sortie
trader-loss-limit-title = Limite de perte par période
trader-loss-limit-resume = Reprendre le trading
trader-loss-limit-reset = Réinitialiser la période
trader-loss-limit-off = Désactivée
trader-loss-limit-resets-in = Réinitialisation dans { $hours } { $minutes }
trader-force-stop = Tout arrêter de force
trader-force-stop-hint = Arrête d’un coup tous les moniteurs, tâches de copie et trades.
trader-force-stop-idle = Rien ne tourne qu’un arrêt forcé pourrait interrompre.
trader-run-state-force-stopped = Arrêt forcé
trader-run-state-waiting = En attente des services
trader-run-state-idle = Moniteurs arrêtés
trader-entries-label = Nouvelles entrées
trader-entries-open = Ouvertes
trader-entries-monitor-off = Moniteur d'entrée arrêté
trader-entries-no-live-strategy = Aucune stratégie d'entrée réelle
trader-entries-no-paper-strategy = Aucune stratégie d'entrée simulée
trader-action-start = Démarrer l'Auto Trader
trader-action-stop = Arrêter l'Auto Trader
trader-notice-loss-limit = Limite de pertes atteinte
trader-notice-loss-limit-message = Les nouvelles entrées réelles sont en pause jusqu'à la réinitialisation de la période. Les sorties continuent.
trader-notice-stats-failed = Impossible de charger les statistiques
trader-controls-group =
    .aria-label = Commandes de trading
trader-monitor-dormant = Démarre avec l'Auto Trader

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

trader-stats-window =
    .aria-label = Période des statistiques
trader-stats-book =
    .aria-label = Registre de trading
trader-exposure-cash = Liquidités
trader-exposure-open = Ouvert
trader-exposure-unrealized = Latent
trader-exposure-open-title = Positions qui occupent un emplacement, sur la limite de positions ouvertes
trader-exposure-in-use = Engagé
trader-exposure-in-use-title = { -sol } engagés dans les positions ouvertes
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
        [one] { $amount } position ouverte n'a pas encore de prix en direct
        [many] { $amount } positions ouvertes n'ont pas encore de prix en direct
       *[other] { $amount } positions ouvertes n'ont pas encore de prix en direct
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = Session simulée depuis le { $date } · démarrée avec { $amount }
trader-paper-reset = Réinitialiser la session simulée
trader-paper-reset-title = Réinitialiser la session simulée ?
trader-paper-reset-run-message = Chaque position simulée ouverte, y compris les détentions simulées des tâches de copie, est clôturée à son prix actuel et une nouvelle session démarre avec le solde initial configuré. Les sessions précédentes restent dans l'historique.
trader-paper-reset-done = Une nouvelle session simulée a démarré
trader-paper-reset-failed = Impossible de réinitialiser la session simulée
trader-realized-title = Performance réalisée
trader-metric-net-pnl = P&L net
trader-metric-win-rate = Taux de réussite
trader-metric-profit-factor = Facteur de profit
trader-metric-max-drawdown = Drawdown max
trader-metric-avg-win-loss = Gain / perte moy.
trader-metric-closed-trades = Trades clôturés
trader-metric-median-hold = Durée médiane de détention
trader-metric-fees = Frais de réseau
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
trader-stats-excluded =
    { $count ->
        [one] { $amount } trade clôturé est exclu : son prix de revient est incomplet, son P&L est donc inconnu.
        [many] { $amount } trades clôturés sont exclus : leur prix de revient est incomplet, leur P&L est donc inconnu.
       *[other] { $amount } trades clôturés sont exclus : leur prix de revient est incomplet, leur P&L est donc inconnu.
    }
# $ratio is a formatted ratio.
trader-stats-payoff = Payoff { $ratio }
trader-stats-streak-win =
    { $count ->
        [one] Dernier trade gagnant
        [many] { $amount } gains d'affilée
       *[other] { $amount } gains d'affilée
    }
trader-stats-streak-loss =
    { $count ->
        [one] Dernier trade perdant
        [many] { $amount } pertes d'affilée
       *[other] { $amount } pertes d'affilée
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = Gains { $wins } · pertes { $losses }
trader-stats-fees-basis = Inclus dans le P&L net
trader-stats-loading = Chargement des statistiques...
trader-open-strategies = Ouvrir les stratégies

trader-daily-title = P&L quotidien
trader-daily-subtitle = { -sol } réalisés par jour, avec le cumul
trader-daily-chart = Profits et pertes réalisés par jour en { -sol }
trader-weekly-title = P&L hebdomadaire
trader-weekly-subtitle = { -sol } réalisés par semaine ISO, avec le cumul
trader-weekly-chart = Profits et pertes réalisés par semaine en { -sol }
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
        [one] { $date } · { $pnl } · { $amount } trade
        [many] { $date } · { $pnl } · { $amount } trades
       *[other] { $date } · { $pnl } · { $amount } trades
    }
trader-weekly-bar =
    { $count ->
        [one] Semaine du { $date } · { $pnl } · { $amount } trade
        [many] Semaine du { $date } · { $pnl } · { $amount } trades
       *[other] Semaine du { $date } · { $pnl } · { $amount } trades
    }
trader-extreme-best = Meilleur trade
trader-extreme-worst = Pire trade
trader-extreme-runs = Plus longues séries consécutives
trader-outcome-title = Résultats des trades
trader-outcome-subtitle = Trades clôturés par rendement, en pourcentage
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = Moins de { $value }
trader-outcome-range = De { $from } à { $to }
trader-outcome-above = { $value } ou plus
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
        [one] { $range } · { $amount } trade · { $pnl }
        [many] { $range } · { $amount } trades · { $pnl }
       *[other] { $range } · { $amount } trades · { $pnl }
    }

trader-stats-empty-day = Aucun trade clôturé au cours des dernières 24 heures
trader-stats-empty-all = Aucun trade clôturé pour l’instant
trader-stats-empty-hint = Les trades clôturés apparaissent ici dès qu'une position est vendue. Démarrez l'Auto Trader ou choisissez une période plus longue.
trader-stats-empty-days =
    { $count ->
        [one] Aucun trade clôturé au cours du dernier jour ({ $amount })
        [many] Aucun trade clôturé au cours des { $amount } derniers jours
       *[other] Aucun trade clôturé au cours des { $amount } derniers jours
    }
trader-exits-title = Motifs de sortie
trader-exits-subtitle = Comment les trades clôturés se sont terminés, et ce que chaque sortie a rapporté
trader-sources-title = Sources d'entrée
trader-sources-subtitle = Ce qui a ouvert les trades clôturés
trader-recent-title = Clôtures récentes
trader-recent-subtitle = Les derniers trades clôturés sur cette période
trader-column-token = Token
trader-column-exit = Sortie
trader-column-source = Source
trader-column-trades = Trades
trader-column-win-rate = Taux de réussite
trader-column-avg-return = Rend. moyen
trader-column-net-pnl = P&L net
trader-column-held = Détenu
trader-column-closed = Clôturé
trader-column-pnl = P&L
trader-column-return = Rendement

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

## Service mode notice

trader-service-notice-title = Fonctionne sur le service { -brand }
trader-service-notice-message = Les prix se mettent à jour toutes les quelques secondes, donc les sorties rapides comme les trailing stops serrés réagissent plus tard qu’avec votre propre RPC.
