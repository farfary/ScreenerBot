system-result-config-differs = La configuration en mémoire diffère de la version sur disque
system-result-config-matches = La configuration en mémoire correspond à la version sur disque

system-result-config-imported =
    Import réussi de { $count ->
        [one] { $count } section
        [many] { $count } sections
       *[other] { $count } sections
    }
system-result-config-imported-with-warnings =
    Import de { $count ->
        [one] { $count } section
        [many] { $count } sections
       *[other] { $count } sections
    } avec { $warnings ->
        [one] { $warnings } avertissement
        [many] { $warnings } avertissements
       *[other] { $warnings } avertissements
    } : { $details }

system-config-search =
    .placeholder = Rechercher des paramètres...
system-config-export-title =
    .title = Exporter la configuration vers un fichier
system-config-import-title =
    .title = Importer la configuration depuis un fichier
system-config-reload = Recharger depuis le disque
system-config-reset-defaults = Rétablir les valeurs par défaut
system-config-select-section = Sélectionnez une section de configuration
system-config-select-section-details = Sélectionnez une section de configuration pour afficher les détails.
system-config-no-metadata = Aucune métadonnée pour <code>{ $section }</code>
system-config-technical-settings = Paramètres techniques
system-config-expand-title = Développer toutes les sections et toutes les sous-configurations imbriquées
system-config-collapse-title = Réduire toutes les sections et toutes les sous-configurations imbriquées
system-config-toolbar-no-changes = Aucune modification dans la section
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> modification dans la section
        [many] <strong>{ $count }</strong> modifications dans la section
       *[other] <strong>{ $count }</strong> modifications dans la section
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> modification au total
        [many] <strong>{ $count }</strong> modifications au total
       *[other] <strong>{ $count }</strong> modifications au total
    }

system-config-loading = Chargement de la configuration…
system-config-refreshing = Actualisation de la configuration…
system-config-saving-title = Enregistrement des modifications…
system-config-saving-detail = Mise à jour de la configuration
system-config-validation-issues = <strong>Problèmes de validation détectés.</strong> Vérifiez les champs signalés.

system-config-save-changes = Enregistrer les modifications
system-config-saving = Enregistrement…
system-config-compare = Comparer avec le disque
system-config-revert-section = Annuler les modifications de la section
system-config-summary-critical = { $count } critiques
system-config-summary-performance = { $count } de performance
system-config-summary-pending =
    { $count ->
        [one] { $count } modification en attente
        [many] { $count } modifications en attente
       *[other] { $count } modifications en attente
    }
system-config-summary-none = Aucun résumé de métadonnées
system-config-fields-count =
    { $count ->
        [one] { $count } champ
        [many] { $count } champs
       *[other] { $count } champs
    }
system-config-chip-pending = { $fields } · { $pending } en attente
system-config-chip-visible = { $visible } sur { $fields }

system-config-field-unit = Unité : { $unit }
system-config-field-default = Par défaut : { $value }
system-config-field-reset = Rétablir la valeur par défaut
system-config-array-invalid-title = Entrée de tableau invalide
system-config-json-invalid-title = JSON invalide
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
        [one] La ligne { $lines } doit être un entier valide.
        [many] Les lignes { $lines } doivent être des entiers valides.
       *[other] Les lignes { $lines } doivent être des entiers valides.
    }
system-config-array-invalid-number =
    { $count ->
        [one] La ligne { $lines } doit être un nombre valide.
        [many] Les lignes { $lines } doivent être des nombres valides.
       *[other] Les lignes { $lines } doivent être des nombres valides.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] La ligne { $lines } doit être un booléen valide.
        [many] Les lignes { $lines } doivent être des booléens valides.
       *[other] Les lignes { $lines } doivent être des booléens valides.
    }
system-config-array-invalid-value =
    { $count ->
        [one] La ligne { $lines } doit être une valeur valide.
        [many] Les lignes { $lines } doivent être des valeurs valides.
       *[other] Les lignes { $lines } doivent être des valeurs valides.
    }

system-config-telegram-actions = Actions
system-config-telegram-test-title = Tester la connexion
system-config-telegram-test-description = Envoyez un message de test pour vérifier que votre configuration { -telegram } fonctionne
system-config-telegram-send-test = Envoyer un message de test
system-config-telegram-sending = Envoi...
system-config-telegram-configure-token-title = Configurez d'abord le token du bot
system-config-telegram-configure-token-status = Configurez le token du bot ci-dessus pour activer le test
system-config-telegram-test-sent-status = Message de test envoyé avec succès ! Consultez votre { -telegram }.
system-config-telegram-test-sent = Message de test { -telegram } envoyé
system-config-telegram-test-failed = Impossible d'envoyer le message de test
system-config-telegram-auth-title = Authentification du bot
system-config-telegram-totp-title = Authentification à deux facteurs (TOTP)
system-config-telegram-totp-configured = Configurée
system-config-telegram-totp-not-configured = Non configurée
system-config-telegram-totp-active = L'authentification à deux facteurs est active. Les sessions { -telegram } expirées exigent un code TOTP de votre application d'authentification.
system-config-telegram-totp-inactive = Activez l'authentification à deux facteurs dans les paramètres de sécurité pour protéger les commandes { -telegram }.
system-config-telegram-totp-note = Le TOTP est partagé avec l'écran de verrouillage du tableau de bord. Configurez-le dans les paramètres de sécurité.
system-config-telegram-require-2fa = Exiger la 2FA pour les commandes
system-config-telegram-save-rejected = Enregistrement rejeté ({ $status })
system-config-telegram-save-failed = Impossible d'enregistrer le paramètre { -telegram }

system-config-saved = Configuration enregistrée
system-config-save-failed = Impossible d'enregistrer la configuration
system-config-reloaded = Configuration rechargée depuis le disque
system-config-reload-failed = Impossible de recharger la configuration
system-config-diff-title = Différences de configuration
system-config-diff-console = Écrit dans la console du navigateur
system-config-diff-failed = Impossible de calculer les différences
system-config-reset-title = Réinitialiser la configuration
system-config-reset-message =
    Cette action rétablit toute la configuration aux valeurs par défaut intégrées. Tous les paramètres actuels seront perdus.

    Cette action est irréversible.
system-config-reset-done-title = Configuration réinitialisée
system-config-reset-done-message = Tous les paramètres ont été rétablis à leurs valeurs par défaut
system-config-reset-failed = Impossible de réinitialiser la configuration
system-config-load-failed = Impossible de charger la configuration
system-config-metadata-failed = Impossible de charger les métadonnées de configuration

system-config-dialog-close =
    .aria-label = Fermer
system-config-select-none = Tout désélectionner
system-config-section-gui = Interface
system-config-changes-count =
    { $count ->
        [one] { $count } modification
        [many] { $count } modifications
       *[other] { $count } modifications
    }
system-config-sections-count =
    { $count ->
        [one] { $count } section
        [many] { $count } sections
       *[other] { $count } sections
    }

system-config-section-hint-chains = Activation des chaînes, endpoints RPC et routage des swaps
system-config-section-hint-trader = Règles de trading et automatisation
system-config-section-hint-positions = Paramètres de gestion des positions
system-config-section-hint-filtering = Règles et seuils de filtrage des tokens
system-config-section-hint-tokens = Découverte des tokens et sources de données
system-config-section-hint-events = Paramètres d'enregistrement des événements
system-config-section-hint-services = Paramètres des services en arrière-plan
system-config-section-hint-monitoring = Configuration de la surveillance du système
system-config-section-hint-ohlcv = Paramètres des données de bougies
system-config-section-hint-gui = Paramètres du tableau de bord et de l'interface
system-config-section-hint-telegram = Configuration du bot { -telegram }

system-config-export-dialog-title = Exporter la configuration
system-config-export-intro = Sélectionnez les sections de configuration à exporter. Le fichier exporté pourra être importé plus tard pour restaurer ou partager les paramètres.
system-config-export-sections = Sections
system-config-export-timestamp = Inclure l'horodatage de l'export
system-config-sections-selected =
    { $count ->
        [one] { $count } section sélectionnée
        [many] { $count } sections sélectionnées
       *[other] { $count } sections sélectionnées
    }
system-config-exporting = Exportation...
system-config-export-invalid-response = Réponse invalide du serveur
system-config-exported-title = Configuration exportée
system-config-exported-message =
    { $count ->
        [one] { $count } section exportée
        [many] { $count } sections exportées
       *[other] { $count } sections exportées
    }
system-config-export-failed-title = Échec de l'exportation
system-config-export-failed = Impossible d'exporter la configuration

system-config-import-dialog-title = Importer la configuration
system-config-import-upload-intro = Chargez un fichier de configuration exporté précédemment. Vous pourrez prévisualiser et sélectionner les sections à importer.
system-config-import-dropzone-title = Déposez le fichier de configuration ici
system-config-import-dropzone-hint = ou cliquez pour parcourir
system-config-import-analyzing = Analyse de la configuration...
system-config-import-preview = Aperçu
system-config-import-preview-intro = Vérifiez les sections de configuration ci-dessous. Sélectionnez celles à importer.
system-config-import-sections = Sections du fichier
system-config-import-select-valid = Sélectionner toutes les valides
system-config-import-merge-label = Fusionner avec l'existant
system-config-import-merge-hint = Ne met à jour que les champs présents dans le fichier. Décoché = remplace les sections entières.
system-config-import-save-label = Enregistrer sur le disque
system-config-import-save-hint = Enregistre les modifications dans config.toml après l'importation
system-config-import-selected = Importer la sélection
system-config-import-warnings =
    { $count ->
        [one] { $count } avertissement
        [many] { $count } avertissements
       *[other] { $count } avertissements
    }
system-config-import-warning-unknown-section = La section inconnue « { $section } » sera ignorée
system-config-import-warning-sensitive-field = L'importation de { $field } peut écraser les paramètres d'authentification
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Absente du fichier
system-config-import-status-invalid = Configuration invalide
system-config-import-status-unchanged = Aucune modification
system-config-import-not-included = Non incluse dans le fichier
system-config-import-show-changes = Afficher les modifications
system-config-import-hide-changes = Masquer les modifications
system-config-import-value-current = Valeur actuelle
system-config-import-value-new = Nouvelle valeur
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } modification de plus
        [many] +{ $count } modifications de plus
       *[other] +{ $count } modifications de plus
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } élément
        [many] { $count } éléments
       *[other] { $count } éléments
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } clé
        [many] { $count } clés
       *[other] { $count } clés
    }{ "}" }
system-config-importing = Importation...
system-config-import-failed = Échec de l'importation
system-config-import-invalid-file-title = Fichier invalide
system-config-import-invalid-file = Impossible d'analyser le fichier de configuration
system-config-imported-title = Configuration importée
system-config-imported-message =
    { $count ->
        [one] { $count } section importée
        [many] { $count } sections importées
       *[other] { $count } sections importées
    }
system-config-import-failed-title = Échec de l'importation
system-config-import-failed-message = Impossible d'importer la configuration
