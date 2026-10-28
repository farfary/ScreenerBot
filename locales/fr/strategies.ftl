strategies-filter-all = Toutes
strategies-filter-entry = Entrée
strategies-filter-exit = Sortie
strategies-type-entry = Entrée
strategies-type-exit = Sortie
strategies-list-empty-title = Aucune stratégie pour le moment
strategies-list-empty-hint = Créez votre première stratégie
strategies-new = Nouvelle stratégie
strategies-import =
    .title = Importer une stratégie
    .aria-label = Importer une stratégie
strategies-item-enable =
    .title = Activer
strategies-item-disable =
    .title = Désactiver

strategies-new-name = Nouvelle stratégie

strategies-editor-name =
    .placeholder = Nom de la stratégie
strategies-editor-dirty =
    .title = Modifications non enregistrées
strategies-action-validate = Valider
strategies-editor-empty = Sélectionnez une stratégie à modifier ou créez-en une nouvelle
strategies-conditions-empty-title = Aucune condition pour le moment
strategies-conditions-empty-hint = Utilisez « { strategies-add-condition } » pour commencer
strategies-add-condition = Ajouter une condition
strategies-modal-close =
    .aria-label = Fermer
strategies-card-move-up =
    .title = Monter
strategies-card-move-down =
    .title = Descendre
strategies-card-duplicate =
    .title = Dupliquer
strategies-card-delete =
    .title = Supprimer
# $name is the condition name.
strategies-card-delete-confirm = Retirer la condition
    .message = Retirer « { $name } » de cette stratégie ?

strategies-summary-param = { $label } : { $value }
strategies-summary-none = Aucun paramètre
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Réglage de la stratégie ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount }s
strategies-value-duration-minutes = { $amount }min
strategies-value-duration-hours = { $amount }h
strategies-value-duration-days = { $amount }j
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Votre heure locale : { $time }
strategies-hint-price-impact = Un trade de cette taille déplace le prix d'environ { $amount } %

strategies-value-percent = { $amount } %
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } heure
        [many] { $amount } heures
       *[other] { $amount } heures
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } bougie
        [many] { $amount } bougies
       *[other] { $amount } bougies
    }

strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = h
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

strategies-catalog-search =
    .placeholder = Rechercher des conditions...
    .aria-label = Rechercher des conditions
strategies-catalog-search-clear =
    .aria-label = Effacer la recherche
strategies-catalog-no-description = Aucune description disponible

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Active
strategies-editor-timeframe = Période par défaut
    .title = Période des bougies pour les conditions qui n'en définissent pas
strategies-editor-priority = Priorité
    .title = Ordre d'évaluation de 1 à 100 ; les nombres les plus bas sont évalués en premier
strategies-editor-description = Description
strategies-editor-description-field =
    .placeholder = Ce que fait cette stratégie
    .aria-label = Description de la stratégie
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Toutes les stratégies d'entrée
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Gère
strategies-editor-entry-binding-hint = Gère les positions ouvertes par les stratégies d'entrée cochées. Sans aucune cochée, elle gère toutes les positions, y compris manuelles et copiées.
strategies-editor-entry-binding-empty = Aucune stratégie d'entrée pour l'instant
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (supprimée)
strategies-action-export = Exporter
strategies-action-test = Tester sur un token
# $token is the tested token's symbol or address.
strategies-test-title = Test sur { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Test sur { $token } en cours…
strategies-test-verdict-pass = Remplie
strategies-test-verdict-fail = Non remplie
strategies-test-verdict-unknown = Inconnu

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Conditions à remplir : { $mode }
strategies-group-match = À remplir : { $mode }
strategies-match-all = toutes
strategies-match-any = au moins une
strategies-match-mode =
    .aria-label = Toutes les conditions ou au moins une
strategies-invert = Inverser
    .title = Inverser : vrai quand ceci ne correspond pas
strategies-chip-off = Désactivé
strategies-chip-inverted = Inversé
strategies-row-switch =
    .title = Désactiver pour le garder enregistré sans l'évaluer
    .aria-label = Évalué par cette stratégie
strategies-row-expand =
    .aria-label = Afficher ou masquer les paramètres
strategies-add-group = Ajouter un groupe
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Retirer le groupe
    .message =
        { $count ->
            [one] Retirer ce groupe et sa condition ?
            [many] Retirer ce groupe et ses { $count } conditions ?
           *[other] Retirer ce groupe et ses { $count } conditions ?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } problème à corriger : { $first }
        [many] { $count } problèmes à corriger : { $first }
       *[other] { $count } problèmes à corriger : { $first }
    }
strategies-problem-located = { $name } : { $problem }
strategies-problem-located-field = { $name }, { $field } : { $problem }
strategies-problem-rules-empty = Ajoutez au moins une condition activée
strategies-problem-group-empty = Un groupe a besoin d'au moins une condition activée
strategies-problem-not-applicable = Cette condition ne peut pas être utilisée dans ce type de stratégie
strategies-problem-unknown-condition = Type de condition inconnu : { $type }
strategies-problem-too-deep = Les règles peuvent s'imbriquer sur { $max } niveaux au maximum ; un groupe inversé compte pour un niveau de plus
strategies-problem-required = Saisissez un nombre
strategies-problem-range = Doit être compris entre { $min } et { $max }
strategies-problem-invalid-option = Choisissez l'une des options proposées

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Toutes
strategies-catalog-categories =
    .aria-label = Catégories
strategies-catalog-empty = Aucune condition ne correspond à la recherche
strategies-catalog-exit-only = Stratégies de sortie uniquement
strategies-catalog-entry-only = Stratégies d'entrée uniquement
strategies-catalog-candles-note = { $description } Lit uniquement les bougies.
strategies-data-candles = Bougies
strategies-data-pool = Pool
strategies-data-position = Position
strategies-data-market = Activité du marché
strategies-data-clock = Heure de la journée

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = trades
strategies-value-trades =
    { $count ->
        [one] { $amount } trade
        [many] { $amount } trades
       *[other] { $amount } trades
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Trop de niveaux
    .message = Les règles peuvent s'imbriquer sur { $max } niveaux au maximum
strategies-toast-priority-invalid = Priorité invalide
    .message = Saisissez un nombre entier de 1 à 100
strategies-toast-exported = Stratégie exportée
# $name is the strategy name.
strategies-unsaved-confirm = Abandonner les modifications non enregistrées ?
    .message = « { $name } » contient des modifications non enregistrées.
strategies-unsaved-discard = Abandonner
strategies-import-error-json = Le fichier n'est pas un JSON valide
strategies-import-error-shape = Le fichier ne décrit pas une stratégie
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = Le fichier utilise une condition que cette version ne connaît pas : { $type }
strategies-import-error-not-applicable = { $name } ne peut pas être utilisée dans ce type de stratégie

strategies-create-title = Créer une nouvelle stratégie
strategies-create-prompt = Choisissez le type de stratégie à créer :
strategies-create-entry-name = Stratégie d'entrée
strategies-create-entry-description = Définissez les conditions d'achat d'un token
strategies-create-exit-name = Stratégie de sortie
strategies-create-exit-description = Définissez les conditions de vente d'un token

strategies-delete-title = Supprimer la stratégie
strategies-delete-message = Supprimer la stratégie « { $name } » ? Cette action est irréversible.

strategies-toast-fix-validation = Corrigez les erreurs de validation avant d'enregistrer
strategies-toast-load-failed = Échec du chargement
    .message = Impossible de charger les stratégies depuis le serveur
strategies-toast-load-strategy-failed = Impossible de charger la stratégie
strategies-toast-no-strategy = Aucune stratégie créée
    .message = Ajoutez au moins une condition ou cliquez sur « Nouvelle stratégie » pour d'abord créer une stratégie
strategies-toast-name-required = Nom requis
    .message = Saisissez un nom de stratégie avant d'enregistrer
strategies-toast-saved = Stratégie enregistrée
    .message = « { $name } » enregistrée avec succès
strategies-toast-save-failed = Échec de l'enregistrement
    .message = Impossible d'enregistrer la stratégie dans la base de données
strategies-toast-no-strategy-validate = Aucune stratégie à valider
strategies-toast-no-strategy-test = Aucune stratégie à tester
strategies-toast-test-save-first = Enregistrez la stratégie avant de la tester : le test lit les règles enregistrées
strategies-toast-test-failed = Échec du test
    .message = Impossible de tester la stratégie
strategies-toast-valid = La stratégie est valide
strategies-toast-invalid = La stratégie contient des erreurs
strategies-toast-validation-failed = Échec de la validation
strategies-toast-item-enabled = Stratégie activée
strategies-toast-item-disabled = Stratégie désactivée
strategies-toast-item-toggle-failed = Impossible de basculer la stratégie
strategies-toast-deleted = Stratégie supprimée
    .message = « { $name } » supprimée avec succès
strategies-toast-delete-failed = Échec de la suppression
    .message = Impossible de supprimer la stratégie de la base de données
strategies-toast-imported = Stratégie importée
strategies-toast-import-failed = Impossible d'importer la stratégie
strategies-toast-unknown-condition = Condition inconnue
    .message = Type de condition introuvable
strategies-toast-create-first = Créez d'abord une stratégie
    .message = Cliquez sur « Nouvelle stratégie » pour créer une stratégie avant d'ajouter des conditions

strategies-condition-candle-size = Motif de taille de bougie
    .description = Détecte des motifs de bougies précis : grand corps, petit corps (doji), longues mèches.
strategies-condition-candle-size-param-pattern = Type de motif
    .description = Motif de bougie à détecter
strategies-condition-candle-size-param-pattern-option-large-body = Grand corps (mouvement fort)
strategies-condition-candle-size-param-pattern-option-small-body = Petit corps (doji/indécision)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Longue mèche haute (rejet)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Longue mèche basse (support)
strategies-condition-candle-size-param-threshold = Seuil de taille %
    .description = Seuil en pourcentage pour la détection du motif

strategies-condition-consecutive-candles = Bougies consécutives
    .description = Détecte des bougies vertes (haussières) ou rouges (baissières) consécutives avec un filtre de taille minimale.
strategies-condition-consecutive-candles-param-count = Nombre de bougies
    .description = Nombre de bougies consécutives requis
strategies-condition-consecutive-candles-param-direction = Sens des bougies
    .description = Couleur/sens des bougies consécutives
strategies-condition-consecutive-candles-param-direction-option-green = Vertes (haussières)
strategies-condition-consecutive-candles-param-direction-option-red = Rouges (baissières)
strategies-condition-consecutive-candles-param-minimum-change = Variation minimale %
    .description = Variation minimale en % de chaque bougie (filtre le bruit)

strategies-condition-liquidity-level = Niveau de liquidité du pool
    .description = Vérifie la liquidité du pool en { -sol } (entrée : s'assurer d'une liquidité suffisante ; sortie : détecter un drainage de liquidité)
strategies-condition-liquidity-level-param-threshold = Seuil de liquidité ({ -sol })
    .description = Niveau de liquidité du pool en { -sol }
strategies-condition-liquidity-level-param-comparison = Comparaison
    .description = Comment comparer la liquidité du pool au seuil
strategies-condition-liquidity-level-param-comparison-option-greater-than = Supérieure à (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Supérieure ou égale à (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Inférieure à ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Inférieure ou égale à (≤)

strategies-condition-position-holding-time = Durée de détention de la position
    .description = Vérifie depuis combien de temps une position est détenue
strategies-condition-position-holding-time-param-hours = Seuil de durée (heures)
    .description = Durée en heures depuis l'ouverture de la position
strategies-condition-position-holding-time-param-comparison = Comparaison
    .description = Comment comparer l'ancienneté de la position au seuil
strategies-condition-position-holding-time-param-comparison-option-greater-than = Plus ancienne que (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Au moins (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Plus récente que ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Au plus (≤)

strategies-condition-price-breakout = Cassure de prix
    .description = Détecte une cassure du prix au-dessus de la résistance (plus haut de la période) ou sous le support (plus bas de la période).
strategies-condition-price-breakout-param-lookback = Période de référence
    .description = Nombre de bougies pour trouver le niveau de support/résistance
strategies-condition-price-breakout-param-direction = Sens de la cassure
    .description = Sens de la cassure
strategies-condition-price-breakout-param-direction-option-upward = Haussière (cassure de résistance)
strategies-condition-price-breakout-param-direction-option-downward = Baissière (cassure de support)
strategies-condition-price-breakout-param-confirmation = Confirmation %
    .description = Distance à dépasser au-delà du niveau pour confirmer la cassure (évite les faux signaux)

strategies-condition-price-change-percent = Variation du prix %
    .description = Vérifie si le prix a varié d'un seuil en pourcentage sur une période donnée.
strategies-condition-price-change-percent-param-percentage = Seuil de variation %
    .description = Variation de prix en pourcentage déclenchant la condition (0.1-1000 %)
strategies-condition-price-change-percent-param-direction = Sens
    .description = Sens du mouvement du prix
strategies-condition-price-change-percent-param-direction-option-above = Hausse (+ %)
strategies-condition-price-change-percent-param-direction-option-below = Baisse (- %)
strategies-condition-price-change-percent-param-direction-option-within = Dans la fourchette (± %)
strategies-condition-price-change-percent-param-time-value = Période de référence
    .description = Jusqu'où remonte la mesure de la variation, dans l'unité choisie dans le champ (1-3600 s, 1-1440 min, 1-720 h)
strategies-condition-price-change-percent-param-time-unit = Unité de temps
    .description = Unité de temps de la période de référence
strategies-condition-price-change-percent-param-time-unit-option-seconds = s
strategies-condition-price-change-percent-param-time-unit-option-minutes = min
strategies-condition-price-change-percent-param-time-unit-option-hours = h

strategies-condition-price-to-ma = Prix vs moyenne mobile
    .description = Vérifie si le prix est au-dessus, en dessous ou dans la fourchette de sa moyenne mobile simple.
strategies-condition-price-to-ma-param-period = Période de la MM
    .description = Nombre de bougies pour le calcul de la moyenne mobile
strategies-condition-price-to-ma-param-position = Position
    .description = Position du prix par rapport à la MM
strategies-condition-price-to-ma-param-position-option-above = Au-dessus de la MM
strategies-condition-price-to-ma-param-position-option-below = En dessous de la MM
strategies-condition-price-to-ma-param-position-option-within = Dans la fourchette
strategies-condition-price-to-ma-param-distance = Distance %
    .description = Distance minimale à la MM (pour AU-DESSUS/EN DESSOUS) ou fourchette maximale (pour DANS LA FOURCHETTE)

strategies-condition-volume-spike = Pic de volume
    .description = Détecte les pics de volume par rapport au volume moyen (signale un intérêt accru).
strategies-condition-volume-spike-param-lookback = Période de référence
    .description = Nombre de bougies pour calculer le volume moyen
strategies-condition-volume-spike-param-multiplier = Multiplicateur de volume
    .description = Combien de fois au-dessus de la moyenne (p. ex. 2.0 = 200 % de la moyenne)

strategies-condition-rsi = RSI (indice de force relative)
    .description = Se déclenche lorsque le RSI, avec le lissage de Wilder sur les bougies clôturées, est supérieur ou inférieur à un seuil.
strategies-condition-rsi-param-period = Période du RSI
    .description = Nombre de bougies sur lesquelles le RSI est calculé ; la condition exige une bougie de plus que cette valeur
strategies-condition-rsi-param-comparison = Comparaison
    .description = Indique si le RSI doit être inférieur ou supérieur au seuil
strategies-condition-rsi-param-comparison-option-below = Inférieur (survente)
strategies-condition-rsi-param-comparison-option-above = Supérieur (surachat)
strategies-condition-rsi-param-threshold = Seuil du RSI
    .description = Niveau de RSI de 1 à 99 (30 et 70 sont les niveaux habituels de survente et de surachat)

strategies-condition-ma-cross = Croisement de moyennes mobiles
    .description = Se déclenche lorsqu'une moyenne mobile rapide croise une moyenne lente au cours des dernières bougies.
strategies-condition-ma-cross-param-ma-type = Type de moyenne
    .description = Les moyennes exponentielles donnent plus de poids aux bougies récentes ; les moyennes simples pèsent toutes les bougies de la même façon
strategies-condition-ma-cross-param-ma-type-option-ema = Exponentielle (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Simple (SMA)
strategies-condition-ma-cross-param-fast-period = Période rapide
    .description = Nombre de bougies de la moyenne rapide
strategies-condition-ma-cross-param-slow-period = Période lente
    .description = Nombre de bougies de la moyenne lente ; doit être supérieur à la période rapide
strategies-condition-ma-cross-param-direction = Sens du croisement
    .description = Sens dans lequel la moyenne rapide croise la lente
strategies-condition-ma-cross-param-direction-option-bullish = Haussier (la rapide passe au-dessus)
strategies-condition-ma-cross-param-direction-option-bearish = Baissier (la rapide passe en dessous)
strategies-condition-ma-cross-param-within-candles = Croisement sur les dernières bougies
    .description = Nombre de dernières bougies pouvant contenir le croisement ; 1 signifie la dernière bougie seulement

strategies-condition-position-profit = Profit de la position
    .description = Se déclenche lorsque le profit latent de la position ouverte est supérieur ou inférieur à un pourcentage du prix d'entrée, mesuré sur le prix du pool.
strategies-condition-position-profit-param-comparison = Comparaison
    .description = Indique si le profit doit être au moins ou au plus égal au seuil
strategies-condition-position-profit-param-comparison-option-above = Au moins (≥)
strategies-condition-position-profit-param-comparison-option-below = Au plus (≤)
strategies-condition-position-profit-param-threshold-pct = Seuil de profit %
    .description = Profit latent en pourcentage du prix d'entrée ; les valeurs négatives sont des pertes

strategies-condition-drawdown-from-peak = Repli depuis le sommet
    .description = Se déclenche lorsque le prix du pool a reculé d'un pourcentage par rapport au prix le plus haut atteint depuis le dernier achat, une fois ce sommet suffisamment au-dessus du prix d'entrée.
strategies-condition-drawdown-from-peak-param-drop-pct = Repli depuis le sommet %
    .description = De combien le prix actuel doit être sous le sommet
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Armement après profit %
    .description = Le sommet doit avoir dépassé le prix d'entrée d'au moins ce pourcentage avant que la condition puisse se déclencher ; 0 signifie toujours armée

strategies-condition-buy-pressure = Pression acheteuse
    .description = Se déclenche lorsque les achats représentent au moins une part des transactions d'une fenêtre de temps et que la fenêtre compte assez de transactions. Lit les nombres d'achats et de ventes du token.
strategies-condition-buy-pressure-param-window = Fenêtre de temps
    .description = Période couverte par les nombres d'achats et de ventes
strategies-condition-buy-pressure-param-window-option-5m = 5 minutes
strategies-condition-buy-pressure-param-window-option-1h = 1 heure
strategies-condition-buy-pressure-param-window-option-6h = 6 heures
strategies-condition-buy-pressure-param-window-option-24h = 24 heures
strategies-condition-buy-pressure-param-min-buy-share-pct = Part d'achats minimale %
    .description = Achats en pourcentage de toutes les transactions de la fenêtre
strategies-condition-buy-pressure-param-min-transactions = Transactions minimales
    .description = Les fenêtres comptant moins de transactions ne déclenchent jamais la condition

strategies-condition-transaction-velocity = Vitesse des transactions
    .description = Se déclenche lorsque le rythme des transactions d'une fenêtre récente est au moins ou au plus un multiple du rythme de la fenêtre plus longue qui la contient. Lit les nombres d'achats et de ventes du token.
strategies-condition-transaction-velocity-param-window = Fenêtres
    .description = La fenêtre récente et la fenêtre plus longue à laquelle elle est comparée
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 minutes vs 1 heure
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 heure vs 6 heures
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 heures vs 24 heures
strategies-condition-transaction-velocity-param-comparison = Comparaison
    .description = Si le rythme récent doit être au moins ou au plus le ratio
strategies-condition-transaction-velocity-param-comparison-option-above = Au moins (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Au plus (≤)
strategies-condition-transaction-velocity-param-ratio = Ratio de rythme
    .description = Transactions par seconde de la fenêtre récente divisées par celles de la fenêtre longue
strategies-condition-transaction-velocity-param-min-transactions = Transactions minimales
    .description = Les fenêtres récentes avec moins de transactions ne déclenchent jamais la condition

strategies-condition-pullback-from-high = Repli depuis le plus haut
    .description = Se déclenche lorsque le prix en direct se situe entre une baisse minimale et une baisse maximale sous le plus haut des dernières bougies.
strategies-condition-pullback-from-high-param-lookback = Période de référence
    .description = Nombre de bougies dont on prend le plus haut
strategies-condition-pullback-from-high-param-min-drop-pct = Baisse minimale %
    .description = Le prix doit être au moins aussi loin sous le plus haut
strategies-condition-pullback-from-high-param-max-drop-pct = Baisse maximale %
    .description = Le prix doit être au plus aussi loin sous le plus haut ; doit dépasser la baisse minimale

strategies-condition-profit-giveback = Restitution du gain
    .description = Se déclenche lorsque la position a rendu une part de son meilleur gain latent depuis le dernier achat, une fois ce gain parvenu à un seuil.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Armer après profit %
    .description = Le meilleur gain doit avoir atteint ce pourcentage du prix d'entrée avant que la condition puisse se déclencher
strategies-condition-profit-giveback-param-giveback-pct = Restitution %
    .description = Part du meilleur gain rendue ; 100 % signifie que le prix est revenu au prix d'entrée

strategies-condition-token-age = Âge du token
    .description = Se déclenche lorsque le temps écoulé depuis la création on-chain du token est plus long ou plus court qu'une durée. Si la date de création est inconnue, la condition reste indéterminée.
strategies-condition-token-age-param-comparison = Comparaison
    .description = Si le token doit être plus ancien ou plus récent que la durée
strategies-condition-token-age-param-comparison-option-older-than = Plus ancien que (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Plus récent que ({ "<" })
strategies-condition-token-age-param-time-value = Âge
    .description = Temps depuis la création du token, dans l'unité choisie dans le champ (1-10000)
strategies-condition-token-age-param-time-unit = Unité de temps
    .description = Unité de l'âge
strategies-condition-token-age-param-time-unit-option-minutes = min
strategies-condition-token-age-param-time-unit-option-hours = h
strategies-condition-token-age-param-time-unit-option-days = j

strategies-condition-vwap = VWAP
    .description = Se déclenche lorsque le prix en direct est au-dessus ou en dessous du prix moyen pondéré par le volume des dernières bougies d'au moins une distance.
strategies-condition-vwap-param-lookback = Période de référence
    .description = Nombre de bougies couvertes par la moyenne pondérée par le volume
strategies-condition-vwap-param-position = Position
    .description = Position du prix par rapport au VWAP
strategies-condition-vwap-param-position-option-above = Au-dessus du VWAP
strategies-condition-vwap-param-position-option-below = En dessous du VWAP
strategies-condition-vwap-param-distance-pct = Distance %
    .description = Distance minimale au VWAP ; 0 signifie n'importe quelle distance du côté choisi

strategies-condition-volatility-range = Plage de volatilité
    .description = Se déclenche lorsque la plage vraie moyenne des dernières bougies, en pourcentage du prix en direct, est au-dessus ou en dessous d'un seuil. Deux de ces conditions dans un groupe forment une plage.
strategies-condition-volatility-range-param-period = Période ATR
    .description = Nombre de bougies sur lesquelles la plage vraie moyenne est lissée ; la condition a besoin d'une bougie de plus
strategies-condition-volatility-range-param-comparison = Comparaison
    .description = Si la plage doit être au-dessus ou en dessous du seuil
strategies-condition-volatility-range-param-comparison-option-above = Au-dessus
strategies-condition-volatility-range-param-comparison-option-below = En dessous
strategies-condition-volatility-range-param-threshold-pct = Seuil de plage %
    .description = Plage vraie moyenne en pourcentage du prix en direct

strategies-condition-bollinger-bands = Bandes de Bollinger
    .description = Se déclenche lorsque le prix en direct est sous la bande basse ou au-dessus de la bande haute, ou lorsque les bandes se resserrent. Les bandes se placent à quelques écarts-types autour de la clôture moyenne.
strategies-condition-bollinger-bands-param-period = Période
    .description = Nombre de bougies couvertes par la moyenne et l'écart-type
strategies-condition-bollinger-bands-param-std-dev = Écarts-types
    .description = À combien d'écarts-types de la moyenne se trouve chaque bande
strategies-condition-bollinger-bands-param-signal = Signal
    .description = Ce que les bandes doivent montrer
strategies-condition-bollinger-bands-param-signal-option-below-lower = Sous la bande basse
strategies-condition-bollinger-bands-param-signal-option-above-upper = Au-dessus de la bande haute
strategies-condition-bollinger-bands-param-signal-option-squeeze = Compression
strategies-condition-bollinger-bands-param-squeeze-width-pct = Largeur de compression %
    .description = Pour une compression : l'écart maximal entre les bandes, en pourcentage de la moyenne

strategies-condition-liquidity-to-size = Liquidité par rapport à la taille
    .description = Se déclenche lorsque les réserves du pool sont au moins ou moins qu'un multiple de la taille en jeu : la taille du trade dans une stratégie d'entrée, la valeur restante de la position dans une stratégie de sortie.
strategies-condition-liquidity-to-size-param-comparison = Comparaison
    .description = Si les réserves doivent être au moins ou moins que le multiple
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Au moins (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Moins de ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Multiple de liquidité
    .description = Réserves du pool divisées par la taille en jeu ; un multiple N déplace le prix d'environ 1/(N+1)

strategies-condition-position-adds = Renforcements de position
    .description = Se déclenche lorsque le nombre d'achats ajoutés après la première entrée, ou de ventes partielles, est au moins ou au plus un nombre.
strategies-condition-position-adds-param-subject = Décompté
    .description = Quelles actions sont comptées
strategies-condition-position-adds-param-subject-option-adds = Achats ajoutés
strategies-condition-position-adds-param-subject-option-partial-sells = Ventes partielles
strategies-condition-position-adds-param-comparison = Comparaison
    .description = Si le décompte doit être au moins ou au plus le nombre
strategies-condition-position-adds-param-comparison-option-at-least = Au moins (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Au plus (≤)
strategies-condition-position-adds-param-count = Nombre
    .description = Nombre d'actions

strategies-condition-time-of-day = Heure de la journée
    .description = Se déclenche lorsque l'heure actuelle est dans une fenêtre d'heures UTC. Un début après la fin passe minuit.
strategies-condition-time-of-day-param-start-hour = Heure de début (UTC)
    .description = Première heure de la fenêtre, de 0 à 23
strategies-condition-time-of-day-param-end-hour = Heure de fin (UTC)
    .description = Heure où la fenêtre se termine, de 1 à 24 ; cette heure est elle-même hors de la fenêtre

strategies-condition-liquidity-change-since-entry = Variation de liquidité depuis l'entrée
    .description = Se déclenche lorsque les réserves du pool ont baissé ou augmenté d'au moins un pourcentage depuis l'ouverture de la position. Sans liquidité d'entrée enregistrée, la condition reste indéterminée.
strategies-condition-liquidity-change-since-entry-param-comparison = Comparaison
    .description = Si les réserves doivent avoir baissé ou augmenté
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = En baisse de
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = En hausse de
strategies-condition-liquidity-change-since-entry-param-change-pct = Variation %
    .description = Variation des réserves du pool depuis l'entrée

strategies-condition-param-timeframe = Période des bougies
    .description = Taille des bougies : la durée de chaque bougie lue par la condition (celle de la stratégie si non définie)
strategies-condition-timeframe-option-1m = 1 minute
strategies-condition-timeframe-option-5m = 5 minutes
strategies-condition-timeframe-option-15m = 15 minutes
strategies-condition-timeframe-option-1h = 1 heure
strategies-condition-timeframe-option-4h = 4 heures
strategies-condition-timeframe-option-12h = 12 heures
strategies-condition-timeframe-option-1d = 1 jour

strategies-condition-category-price-analysis = Analyse des prix
strategies-condition-category-candle-patterns = Motifs de bougies
strategies-condition-category-technical-indicators = Indicateurs techniques
strategies-condition-category-market-context = Contexte de marché
strategies-condition-category-position-performance = Position et performance
strategies-condition-category-volume-analysis = Analyse du volume

strategies-error-missing-parameter = Le paramètre { $field } est manquant
strategies-error-parameter-type = Le paramètre { $field } doit être { $expected }
strategies-error-invalid-value = « { $value } » n'est pas une valeur valide pour { $field }
strategies-error-missing-data = { $data } n'est pas disponible
strategies-error-no-candle-data = La période { $timeframe } n'a aucune donnée de bougies
strategies-error-insufficient-history = Historique insuffisant pour { $indicator } : { $available }s disponibles, { $required }s nécessaires
strategies-error-insufficient-candles = Bougies insuffisantes pour { $indicator } : { $available } disponibles, { $required } nécessaires
strategies-error-stale-candle-data = Les données de bougies { $timeframe } sont obsolètes : leur âge de { $age }s dépasse { $max }s
strategies-error-stale-market-data = Les nombres de transactions { $window } sont obsolètes : leur âge de { $age }s dépasse { $max }s
strategies-error-unusable-data = La valeur « { $value } » pour { $field } est inutilisable
strategies-error-invalid-rule-tree = Arbre de règles invalide : { $reason }
strategies-error-evaluation-timeout = Délai d'évaluation de la stratégie dépassé après { $timeout }ms
strategies-error-condition-not-applicable = { $condition } ne peut pas être utilisé dans les stratégies de type { $strategy }
strategies-error-invalid-rules = Impossible de lire les règles : { $reason }

strategies-error-field-average-volume = le volume moyen
strategies-error-field-candle-open = l'ouverture de la bougie
strategies-error-field-comparison = la comparaison
strategies-error-field-condition-type = le type de condition
strategies-error-field-confirmation = la confirmation
strategies-error-field-count = le nombre
strategies-error-field-current-price = le prix actuel
strategies-error-field-direction = le sens
strategies-error-field-distance = la distance
strategies-error-field-hours = les heures
strategies-error-field-lookback = la période de référence
strategies-error-field-minimum-change = la variation minimale
strategies-error-field-multiplier = le multiplicateur
strategies-error-field-pattern = le motif
strategies-error-field-percentage = le pourcentage
strategies-error-field-period = la période
strategies-error-field-position = la position
strategies-error-field-threshold = le seuil
strategies-error-field-time-unit = l'unité de temps
strategies-error-field-time-value = la valeur de temps
strategies-error-field-timeframe = la période des bougies
strategies-error-field-arm-after-profit = le gain avant armement
strategies-error-field-drop-percentage = le pourcentage de repli
strategies-error-field-entry-price = le prix d'entrée
strategies-error-field-fast-period = la période rapide
strategies-error-field-minimum-buy-share = la part d'achats minimale
strategies-error-field-minimum-transactions = le nombre minimal de transactions
strategies-error-field-moving-average-type = le type de moyenne
strategies-error-field-moving-average = la moyenne mobile
strategies-error-field-reference-candle-close = la clôture de la bougie de référence
strategies-error-field-peak-price = le prix du sommet
strategies-error-field-slow-period = la période lente
strategies-error-field-window = la fenêtre
strategies-error-field-within-candles = le nombre de bougies du croisement
strategies-error-field-change = variation
strategies-error-field-end-hour = heure de fin
strategies-error-field-entry-liquidity = liquidité d'entrée
strategies-error-field-giveback = restitution
strategies-error-field-highest-high = plus haut
strategies-error-field-long-window-transactions = transactions de la fenêtre longue
strategies-error-field-maximum-drop = baisse maximale
strategies-error-field-minimum-drop = baisse minimale
strategies-error-field-multiple = multiple
strategies-error-field-pool-liquidity = liquidité du pool
strategies-error-field-ratio = ratio
strategies-error-field-remaining-position-value = valeur restante de la position
strategies-error-field-signal = signal
strategies-error-field-squeeze-width = largeur de compression
strategies-error-field-standard-deviations = écarts-types
strategies-error-field-start-hour = heure de début
strategies-error-field-subject = décompte
strategies-error-field-token-creation-time = date de création du token
strategies-error-field-total-volume = volume total
strategies-error-field-trade-size = taille du trade
strategies-error-field-vwap = VWAP

strategies-error-expected-boolean = un booléen
strategies-error-expected-number = un nombre
strategies-error-expected-string = une chaîne de caractères

strategies-error-data-current-price = Le prix actuel
strategies-error-data-liquidity-data = Les données de liquidité
strategies-error-data-market-data = Les données de marché
strategies-error-data-ohlcv-data = Les données OHLCV
strategies-error-data-position-data = Les données de position
strategies-error-data-peak-price = Le prix du sommet
strategies-error-data-transaction-counts = Les nombres de transactions
strategies-error-data-entry-liquidity = Liquidité d'entrée
strategies-error-data-entry-pool-liquidity = Liquidité du pool d'entrée
strategies-error-data-real-pool-reserves = Liquidité réelle du pool
strategies-error-data-remaining-position-value = Valeur restante de la position
strategies-error-data-token-creation-time = Date de création du token
strategies-error-data-trade-size = Taille du trade

strategies-error-indicator-consecutive-candles = les bougies consécutives
strategies-error-indicator-moving-average = la moyenne mobile
strategies-error-indicator-price-breakout = la cassure de prix
strategies-error-indicator-price-change-lookback = la période de variation du prix
strategies-error-indicator-volume-spike = le pic de volume
strategies-error-indicator-moving-average-cross = le croisement de moyennes mobiles
strategies-error-indicator-rsi = le RSI
strategies-error-indicator-bollinger-bands = bandes de Bollinger
strategies-error-indicator-pullback-from-high = repli depuis le plus haut
strategies-error-indicator-volatility-range = plage de volatilité
strategies-error-indicator-vwap = VWAP

strategies-error-rule-branch-node-missing-conditions = Nœud de branche sans conditions
strategies-error-rule-branch-node-missing-operator = Nœud de branche sans opérateur
strategies-error-rule-branch-node-must-have-at-least-one-child = Un nœud de branche doit avoir au moins un enfant
strategies-error-rule-invalid-rule-tree-structure = Structure d'arbre de règles invalide
strategies-error-rule-leaf-node-missing-condition = Nœud feuille sans condition
strategies-error-rule-not-operator-must-have-exactly-one-child = L'opérateur NOT doit avoir exactement un enfant
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Un groupe doit avoir au moins une condition activée
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = L'opérateur NOT ne peut pas avoir d'enfant désactivé
strategies-error-rule-rules-nest-deeper-than-four-levels = Les règles ne peuvent pas être imbriquées sur plus de quatre niveaux
strategies-error-rule-the-root-group-cannot-be-switched-off = Le groupe racine ne peut pas être désactivé

strategies-readiness-history-since-met =
    { $count ->
        [one] { $count } cycle simulé clôturé depuis la modification des règles le { $changed }, { $needed } requis
        [many] { $count } cycles simulés clôturés depuis la modification des règles le { $changed }, { $needed } requis
       *[other] { $count } cycles simulés clôturés depuis la modification des règles le { $changed }, { $needed } requis
    }
strategies-readiness-history-since-short = { $count } cycles simulés clôturés sur { $needed } depuis la modification des règles le { $changed }
strategies-readiness-priced = Toutes les positions valorisées
strategies-readiness-priced-ok = Chaque position simulée ouverte a un prix de pool
strategies-readiness-priced-missing =
    { $count ->
        [one] { $count } position simulée ouverte sans prix de pool
        [many] { $count } positions simulées ouvertes sans prix de pool
       *[other] { $count } positions simulées ouvertes sans prix de pool
    }
strategies-readiness-runtime = Trading réel disponible
strategies-readiness-runtime-ok = La configuration et les garde-fous autorisent les transactions réelles
