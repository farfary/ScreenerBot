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
strategies-summary-period-seconds = Période de référence : { $amount }s
strategies-summary-period-minutes = Période de référence : { $amount }min
strategies-summary-period-hours = Période de référence : { $amount }h

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
strategies-action-export = Exporter

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
    .description = Vérifie depuis combien de temps une position est détenue (pour les stratégies de sortie : sorties temporelles)
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
    .description = Se déclenche lorsque le profit latent de la position ouverte est supérieur ou inférieur à un pourcentage du prix d'entrée, mesuré sur le prix du pool. Stratégies de sortie uniquement.
strategies-condition-position-profit-param-comparison = Comparaison
    .description = Indique si le profit doit être au moins ou au plus égal au seuil
strategies-condition-position-profit-param-comparison-option-above = Au moins (≥)
strategies-condition-position-profit-param-comparison-option-below = Au plus (≤)
strategies-condition-position-profit-param-threshold-pct = Seuil de profit %
    .description = Profit latent en pourcentage du prix d'entrée ; les valeurs négatives sont des pertes

strategies-condition-drawdown-from-peak = Repli depuis le sommet
    .description = Se déclenche lorsque le prix du pool a reculé d'un pourcentage par rapport au prix le plus haut atteint depuis l'entrée, une fois ce sommet suffisamment au-dessus du prix d'entrée. Lit uniquement les prix du pool. Stratégies de sortie uniquement.
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
strategies-error-field-peak-price = le prix du sommet
strategies-error-field-slow-period = la période lente
strategies-error-field-window = la fenêtre
strategies-error-field-within-candles = le nombre de bougies du croisement

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

strategies-error-indicator-consecutive-candles = les bougies consécutives
strategies-error-indicator-moving-average = la moyenne mobile
strategies-error-indicator-price-breakout = la cassure de prix
strategies-error-indicator-price-change-lookback = la période de variation du prix
strategies-error-indicator-volume-spike = le pic de volume
strategies-error-indicator-moving-average-cross = le croisement de moyennes mobiles
strategies-error-indicator-rsi = le RSI

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
