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
strategies-summary-period-seconds = Période : { $amount }s
strategies-summary-period-minutes = Période : { $amount }min
strategies-summary-period-hours = Période : { $amount }h

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
strategies-catalog-search-clear =
    .aria-label = Effacer la recherche
strategies-catalog-fold-all = Tout replier
strategies-catalog-unfold-all = Tout déplier
strategies-catalog-no-description = Aucune description disponible

strategies-create-title = Créer une nouvelle stratégie
strategies-create-prompt = Choisissez le type de stratégie à créer :
strategies-create-entry-name = Stratégie d'entrée
strategies-create-entry-description = Définissez les conditions d'ACHAT d'un token
strategies-create-exit-name = Stratégie de sortie
strategies-create-exit-description = Définissez les conditions de VENTE d'un token

strategies-delete-title = Supprimer la stratégie
strategies-delete-message = Supprimer la stratégie « { $name } » ? Cette action est irréversible.

strategies-toast-fix-validation = Corrigez les erreurs de validation avant d'enregistrer
strategies-toast-enabled = Stratégie activée
    .message = « { $name } » activée
strategies-toast-disabled = Stratégie désactivée
    .message = « { $name } » désactivée
strategies-toast-toggle-failed = Échec du basculement
    .message = Impossible de mettre à jour le statut de la stratégie
strategies-toast-load-failed = Échec du chargement
    .message = Impossible de charger les stratégies depuis le serveur
strategies-toast-load-strategy-failed = Impossible de charger la stratégie
strategies-toast-no-strategy = Aucune stratégie créée
    .message = Ajoutez au moins une condition ou cliquez sur « Nouvelle stratégie » pour d'abord créer une stratégie
strategies-toast-no-conditions-save = Aucune condition
    .message = Ajoutez au moins une condition à la stratégie avant d'enregistrer
strategies-toast-name-required = Nom requis
    .message = Saisissez un nom de stratégie avant d'enregistrer
strategies-toast-saved = Stratégie enregistrée
    .message = « { $name } » enregistrée avec succès
strategies-toast-save-failed = Échec de l'enregistrement
    .message = Impossible d'enregistrer la stratégie dans la base de données
strategies-toast-no-strategy-validate = Aucune stratégie à valider
strategies-toast-no-conditions-validate = Aucune condition
    .message = Ajoutez au moins une condition avant de valider
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
strategies-toast-condition-added = Condition ajoutée
    .message = { $name } ajoutée à la stratégie

strategies-condition-candle-size = Motif de taille de bougie
    .description = Détecte des motifs de bougies précis : grand corps, petit corps (doji), longues mèches
strategies-condition-candle-size-param-pattern = Type de motif
    .description = Motif de bougie à détecter
strategies-condition-candle-size-param-pattern-option-large-body = Grand corps (mouvement fort)
strategies-condition-candle-size-param-pattern-option-small-body = Petit corps (doji/indécision)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Longue mèche haute (rejet)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Longue mèche basse (support)
strategies-condition-candle-size-param-threshold = Seuil de taille %
    .description = Seuil en pourcentage pour la détection du motif

strategies-condition-consecutive-candles = Bougies consécutives
    .description = Détecte des bougies vertes (haussières) ou rouges (baissières) consécutives avec un filtre de taille minimale
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
    .description = Détecte une cassure du prix au-dessus de la résistance (plus haut de la période) ou sous le support (plus bas de la période)
strategies-condition-price-breakout-param-lookback = Période de référence
    .description = Nombre de bougies pour trouver le niveau de support/résistance
strategies-condition-price-breakout-param-direction = Sens de la cassure
    .description = Sens de la cassure
strategies-condition-price-breakout-param-direction-option-upward = Haussière (cassure de résistance)
strategies-condition-price-breakout-param-direction-option-downward = Baissière (cassure de support)
strategies-condition-price-breakout-param-confirmation = Confirmation %
    .description = Distance à dépasser au-delà du niveau pour confirmer la cassure (évite les faux signaux)

strategies-condition-price-change-percent = Variation du prix %
    .description = Vérifie si le prix a varié d'un seuil en pourcentage sur une période donnée
strategies-condition-price-change-percent-param-percentage = Seuil de variation %
    .description = Variation de prix en pourcentage déclenchant la condition (0.1-1000 %)
strategies-condition-price-change-percent-param-direction = Sens
    .description = Sens du mouvement du prix
strategies-condition-price-change-percent-param-direction-option-above = Hausse (+ %)
strategies-condition-price-change-percent-param-direction-option-below = Baisse (- %)
strategies-condition-price-change-percent-param-direction-option-within = Dans la fourchette (± %)
strategies-condition-price-change-percent-param-time-value = Période
    .description = Valeur de la période de référence (1-3600 pour les secondes, 1-1440 pour les minutes, 1-720 pour les heures)
strategies-condition-price-change-percent-param-time-unit = Unité de temps
    .description = Unité de temps de la période de référence
strategies-condition-price-change-percent-param-time-unit-option-seconds = Secondes
strategies-condition-price-change-percent-param-time-unit-option-minutes = Minutes
strategies-condition-price-change-percent-param-time-unit-option-hours = Heures

strategies-condition-price-to-ma = Prix vs moyenne mobile
    .description = Vérifie si le prix est au-dessus, en dessous ou dans la fourchette de sa moyenne mobile simple
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
    .description = Détecte les pics de volume par rapport au volume moyen (signale un intérêt accru)
strategies-condition-volume-spike-param-lookback = Période de référence
    .description = Nombre de bougies pour calculer le volume moyen
strategies-condition-volume-spike-param-multiplier = Multiplicateur de volume
    .description = Combien de fois au-dessus de la moyenne (p. ex. 2.0 = 200 % de la moyenne)

strategies-condition-param-timeframe = Période des bougies
    .description = Période des bougies à analyser (par défaut, celle de la stratégie si non définie)
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

strategies-error-expected-boolean = un booléen
strategies-error-expected-number = un nombre
strategies-error-expected-string = une chaîne de caractères

strategies-error-data-current-price = Le prix actuel
strategies-error-data-liquidity-data = Les données de liquidité
strategies-error-data-market-data = Les données de marché
strategies-error-data-ohlcv-data = Les données OHLCV
strategies-error-data-position-data = Les données de position

strategies-error-indicator-consecutive-candles = les bougies consécutives
strategies-error-indicator-moving-average = la moyenne mobile
strategies-error-indicator-price-breakout = la cassure de prix
strategies-error-indicator-price-change-lookback = la période de variation du prix
strategies-error-indicator-volume-spike = le pic de volume

strategies-error-rule-branch-node-missing-conditions = Nœud de branche sans conditions
strategies-error-rule-branch-node-missing-operator = Nœud de branche sans opérateur
strategies-error-rule-branch-node-must-have-at-least-one-child = Un nœud de branche doit avoir au moins un enfant
strategies-error-rule-invalid-rule-tree-structure = Structure d'arbre de règles invalide
strategies-error-rule-leaf-node-missing-condition = Nœud feuille sans condition
strategies-error-rule-not-operator-must-have-exactly-one-child = L'opérateur NOT doit avoir exactement un enfant
