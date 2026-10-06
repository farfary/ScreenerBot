wallets-type-generated = Généré
wallets-type-imported = Importé
wallets-type-migrated = Migré

wallets-watch-disabled-user = Suspendu par vos soins
wallets-watch-disabled-signature-budget = Suspendu : limite de { $limit } signatures par vérification atteinte avant le rattrapage
wallets-watch-disabled-unknown = Suspendu : le motif de sécurité enregistré du suivi est illisible
wallets-watch-disabled-helius-unavailable = Suspendu : le fournisseur pour forte activité est indisponible ; curseur conservé
wallets-watch-disabled-processing-failed = Suspendu : l'activité du portefeuille n'a pas pu être traitée ; curseur conservé

wallets-watch-error-provider-unavailable = Fournisseur pour forte activité indisponible ; suivi suspendu
wallets-watch-error-provider-repeated-failure = Les vérifications { -helius } échouent de façon répétée ; suivi suspendu
wallets-watch-error-processing-repeated-failure = Le traitement de l'activité du portefeuille échoue de façon répétée ; suivi suspendu
wallets-watch-error-position-unreadable = Le suivi du portefeuille n'a pas pu lire sa position enregistrée ; nouvelle tentative
wallets-watch-error-provider-check-failed = Échec de la vérification auprès du fournisseur pour forte activité ; nouvelle tentative
wallets-watch-error-decode-failed = Une transaction de forte activité n'a pas pu être décodée ; curseur conservé
wallets-watch-error-processing-failed = L'activité du portefeuille n'a pas pu être traitée ; nouvelle tentative
wallets-watch-error-position-save-failed = Le suivi du portefeuille n'a pas pu enregistrer sa position ; nouvelle tentative

wallets-watch-reason-user = Suspendu par vos soins.
wallets-watch-reason-signature-budget = Ce portefeuille a plus d'activité que son suivi actuel ne peut en vérifier.
wallets-watch-reason-helius-unavailable = Échec des vérifications { -helius }. La progression enregistrée est conservée.
wallets-watch-reason-processing-failed = L'activité du portefeuille n'a pas pu être traitée. La progression enregistrée est conservée.

wallets-field-address = Adresse
wallets-field-name = Nom du portefeuille
wallets-field-notes = Notes
wallets-field-private-key = Clé privée
wallets-address-copy = Copier l'adresse
wallets-modal-close =
    .aria-label = Fermer la fenêtre
wallets-this-wallet = ce portefeuille
wallets-summary-native = { -sol }
wallets-copied-address = Adresse
wallets-copied-mint = Adresse de mint
wallets-copied-private-key = Clé privée

wallets-tab-main = Portefeuille principal
wallets-tab-secondaries = Secondaires
wallets-tab-archive = Archives
wallets-tab-watched = Suivis
wallets-refresh-failed = Impossible d'actualiser les portefeuilles
wallets-action-failed = Échec
wallets-toast-failed = Échec : { $reason }
wallets-create-busy = Création...
wallets-create-fallback = Échec de la création
wallets-create-done = Portefeuille « { $name } » créé !
wallets-import-busy = Importation...
wallets-import-failed = Échec de l'importation
wallets-import-done = Portefeuille « { $name } » importé !
wallets-archive-busy = Archivage...
wallets-archive-confirm-text = Voulez-vous vraiment archiver <strong>{ $name }</strong> ?
wallets-archive-done = Portefeuille archivé
wallets-restore-done = Portefeuille restauré
wallets-export-busy = Déchiffrement...
wallets-export-revealed = Clé révélée : manipulez-la avec précaution
wallets-delete-busy = Suppression...
wallets-delete-confirm-text = Voulez-vous vraiment supprimer <strong>{ $name }</strong> ?
wallets-delete-done = Portefeuille supprimé définitivement

wallets-add-title = Ajouter un portefeuille
wallets-add-tab-create = Créer
wallets-add-tab-import = Importer
wallets-create-name-input =
    .placeholder = p. ex. Portefeuille de trading
wallets-create-name-hint = Un nom lisible pour identifier ce portefeuille
wallets-create-notes-input =
    .placeholder = Description ou usage (facultatif)...
wallets-create-submit = Créer le portefeuille
wallets-import-warning-title = Avertissement de sécurité
wallets-import-warning-body = N'importez des clés privées que de sources fiables. Votre clé sera chiffrée et stockée en toute sécurité sur cet appareil.
wallets-import-name-input =
    .placeholder = p. ex. Mon portefeuille
wallets-import-key-input =
    .placeholder = Chaîne base58 ou tableau JSON [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Afficher ou masquer la clé privée
wallets-import-key-hint = Clé encodée en base58 ou tableau d'octets acceptés
wallets-import-notes-input =
    .placeholder = Description (facultatif)...
wallets-import-submit = Importer le portefeuille

wallets-watch-add-title = Suivre un portefeuille
wallets-watch-add-address = Adresse du portefeuille
wallets-watch-add-address-input =
    .placeholder = Adresse Solana
wallets-watch-add-address-hint = Enregistre l'activité on-chain du portefeuille et envoie des alertes de trade via vos paramètres { -telegram }.
wallets-watch-add-label = Libellé
wallets-watch-add-label-input =
    .placeholder = Nom (facultatif)
wallets-watch-add-submit = Ajouter le suivi

wallets-watch-budget-title-options = Options de suivi du portefeuille
wallets-watch-budget-title-restore = Restaurer le suivi du portefeuille
wallets-watch-budget-close =
    .aria-label = Fermer
wallets-watch-budget-label-signatures = Signatures vérifiées par vérification
wallets-watch-budget-label-transactions = Transactions complètes réussies vérifiées par vérification
wallets-watch-budget-hint-signatures = Limite actuelle : { $limit }. Choisissez de 500 à 5 000 signatures par vérification, par pas de 100.
wallets-watch-budget-hint-transactions = Limite actuelle : { $limit }. Choisissez de 500 à 5 000 transactions réussies par vérification, par pas de 100.
wallets-watch-budget-error-range = Choisissez entre 500 et 5 000 enregistrements par vérification, par pas de 100.
wallets-watch-budget-error-ack = Confirmez que les signatures depuis la dernière vérification terminée seront ignorées.
wallets-watch-budget-save-failed = Impossible d'enregistrer la limite de suivi.
wallets-watch-budget-save = Enregistrer la limite
wallets-watch-budget-resume = Reprendre à partir de maintenant
wallets-watch-budget-resume-notice = Ce portefeuille a atteint sa limite de vérification avant de rattraper son retard. « Reprendre à partir de maintenant » repart de l'activité la plus récente du portefeuille ; l'activité depuis la dernière vérification terminée ne sera pas copiée.
wallets-watch-budget-resume-tasks = Les tâches de copie restent suspendues jusqu'à ce que vous les repreniez une par une dans Copy trading.
wallets-watch-budget-resume-ack = Je comprends que l'activité manquée ne sera pas copiée.
wallets-watch-budget-resumed = Suivi repris à partir de l'état actuel du portefeuille
wallets-watch-budget-updated = Limite de suivi du portefeuille mise à jour
wallets-watch-helius-allow = Autoriser le rattrapage { -helius } si nécessaire
wallets-watch-helius-try = Tenter le rattrapage avec { -helius }
wallets-watch-helius-stop = Arrêter le rattrapage { -helius } pour ce portefeuille
wallets-watch-helius-description-approved = Le rattrapage { -helius } est autorisé pour ce portefeuille. Le désactiver revient aux vérifications standard, qui peuvent prendre du retard sur un portefeuille très actif.
wallets-watch-helius-description-available = { -helius } peut vérifier les transactions Solana réussies depuis la position enregistrée sans sauter l'intervalle non vérifié. Cela peut consommer davantage de crédits du fournisseur et un retard reste possible.
wallets-watch-helius-description-unavailable = Le rattrapage { -helius } est indisponible. Configurez un endpoint RPC { -helius } activé pour l'utiliser.
wallets-watch-helius-description-unsupported = Aucun fournisseur de rattrapage n'est pris en charge pour ce suivi. « Reprendre à partir de maintenant » est disponible si le suivi atteint sa limite.
wallets-watch-helius-allow-title = Autoriser le rattrapage { -helius } pour ce portefeuille
wallets-watch-helius-allow-message = { -helius } peut vérifier les transactions Solana réussies depuis la position enregistrée sans sauter l'intervalle non vérifié. Il facture actuellement 10 crédits par tranche de 100 transactions complètes renvoyées, arrondie au supérieur, avec un minimum de 10 crédits par requête. Une vérification peut comporter plusieurs requêtes ; l'utilisation et la tarification du fournisseur peuvent varier. Les tâches de copie restent suspendues jusqu'à leur reprise séparée.
wallets-watch-helius-allow-confirm = Autoriser pour ce portefeuille
wallets-watch-helius-stop-message = Ce portefeuille reviendra aux vérifications standard. Un portefeuille très actif peut atteindre sa limite de suivi et être de nouveau suspendu. Les autres portefeuilles et votre configuration RPC { -helius } ne changent pas.
wallets-watch-helius-stop-confirm = Arrêter pour ce portefeuille
wallets-watch-helius-stop-keep = Conserver l'autorisation
wallets-watch-helius-restored = Suivi restauré à partir de la progression enregistrée ; les tâches de copie restent suspendues
wallets-watch-helius-allowed = Rattrapage { -helius } autorisé pour ce portefeuille en cas de besoin
wallets-watch-helius-stopped = Rattrapage { -helius } arrêté pour ce portefeuille
wallets-watch-helius-update-failed = Impossible de mettre à jour le paramètre de rattrapage du portefeuille

wallets-export-title = Exporter la clé privée
wallets-export-warning-title = Avertissement de sécurité critique
wallets-export-warning-body = Ne partagez jamais votre clé privée. Toute personne ayant accès à cette clé peut voler tous les fonds de ce portefeuille.
wallets-export-key-label = Clé privée (Base58)
wallets-export-copy =
    .title = Copier dans le presse-papiers
    .aria-label = Copier dans le presse-papiers
wallets-export-reveal = Révéler la clé

wallets-archive-title = Archiver le portefeuille
wallets-archive-note = Les portefeuilles archivés ne sont utilisés dans aucune opération mais peuvent être restaurés à tout moment.
wallets-archive-confirm = Oui, archiver
wallets-delete-title = Supprimer le portefeuille
wallets-delete-warning-title = Cette action est irréversible !
wallets-delete-warning-body = Supprimer ce portefeuille le retirera définitivement de cet appareil, ainsi que sa clé privée chiffrée.
wallets-delete-confirm = Oui, supprimer

wallets-bulk-import-title = Importer des portefeuilles
wallets-bulk-import-submit = Importer les portefeuilles
wallets-bulk-step-upload = Charger le fichier
wallets-bulk-step-map = Associer les colonnes
wallets-bulk-step-results = Résultats
wallets-bulk-import-file-warning-body = N'importez que des fichiers de sources fiables. Les clés privées seront chiffrées et stockées en toute sécurité sur cet appareil.
wallets-bulk-drop-title = Déposez votre fichier ici
wallets-bulk-drop-subtitle = ou cliquez pour parcourir
wallets-bulk-drop-formats = CSV et Excel (.xlsx, .xls) pris en charge
wallets-bulk-file-remove =
    .aria-label = Retirer le fichier
wallets-bulk-map-subtitle = Associez les colonnes de votre fichier aux champs du portefeuille
wallets-bulk-preview-title = Aperçu (5 premières lignes)
wallets-bulk-summary-valid = <strong>{ $count }</strong> valides
wallets-bulk-summary-invalid = <strong>{ $count }</strong> invalides
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> doublon
        [many] <strong>{ $count }</strong> doublons
       *[other] <strong>{ $count }</strong> doublons
    }
wallets-bulk-done = Terminé
wallets-bulk-file-invalid = Type de fichier invalide. Utilisez un fichier CSV ou Excel.
wallets-bulk-preview-busy = Traitement...
wallets-bulk-preview-fallback = Échec du traitement du fichier
wallets-bulk-preview-failed = Échec du traitement du fichier : { $reason }
wallets-bulk-column-select = -- Choisir une colonne --
wallets-bulk-preview-empty = Aucune ligne de données dans le fichier
wallets-bulk-preview-status = Statut
wallets-bulk-status-valid = Valide
wallets-bulk-status-duplicate = Doublon
wallets-bulk-status-invalid = Invalide
wallets-bulk-import-busy = Importation...
wallets-bulk-import-toast =
    { $count ->
        [one] { $count } portefeuille importé
        [many] { $count } portefeuilles importés
       *[other] { $count } portefeuilles importés
    }
wallets-bulk-import-error = Échec de l'importation : { $reason }
wallets-bulk-result-success-title = Importation réussie
wallets-bulk-result-success-detail =
    { $count ->
        [one] { $count } portefeuille importé avec succès
        [many] Les { $count } portefeuilles ont été importés avec succès
       *[other] Les { $count } portefeuilles ont été importés avec succès
    }
wallets-bulk-result-partial-title = Réussite partielle
wallets-bulk-result-partial-detail = { $imported } importés, { $failed } en échec
wallets-bulk-result-failed-title = Échec de l'importation
wallets-bulk-result-failed-detail =
    { $count ->
        [one] L'importation du portefeuille ({ $count }) a échoué
        [many] L'importation des { $count } portefeuilles a échoué
       *[other] L'importation des { $count } portefeuilles a échoué
    }
wallets-bulk-result-imported = Importés
wallets-bulk-result-failed = En échec

wallets-bulk-export-title = Exporter les portefeuilles
wallets-bulk-export-format = Format
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Inclure les portefeuilles archivés
wallets-bulk-export-safe-title = Export sécurisé
wallets-bulk-export-safe-body = Exporte uniquement les adresses et les métadonnées des portefeuilles, sans clés privées.
wallets-bulk-export-safe-submit = Exporter les adresses
wallets-bulk-export-or = ou
wallets-bulk-export-danger-title = Export dangereux
wallets-bulk-export-danger-body = Inclut les clés privées dans l'export. Toute personne détenant ce fichier peut voler vos fonds.
wallets-bulk-export-danger-submit = Exporter avec les clés privées
wallets-bulk-export-busy = Exportation...
wallets-bulk-export-done = Portefeuilles exportés vers { $filename }
wallets-bulk-export-fallback = Échec de l'exportation
wallets-bulk-export-error = Échec de l'exportation : { $reason }
wallets-bulk-confirm-title = Confirmer l'export dangereux
wallets-bulk-confirm-warning =
    { $count ->
        [one] Vous êtes sur le point d'exporter <strong>{ $count }</strong> clé privée. C'est extrêmement dangereux !
        [many] Vous êtes sur le point d'exporter <strong>{ $count }</strong> clés privées. C'est extrêmement dangereux !
       *[other] Vous êtes sur le point d'exporter <strong>{ $count }</strong> clés privées. C'est extrêmement dangereux !
    }
wallets-bulk-confirm-risk-steal = Toute personne détenant ce fichier peut voler tous les fonds
wallets-bulk-confirm-risk-share = Ne partagez ce fichier avec personne
wallets-bulk-confirm-risk-delete = Supprimez le fichier immédiatement après usage
wallets-bulk-confirm-prompt = Saisissez la phrase ci-dessous pour confirmer
wallets-bulk-confirm-submit = Exporter les clés

wallets-holdings-col-token = Token
wallets-holdings-col-balance = Solde
wallets-holdings-col-value = Valeur ({ -sol })
wallets-holdings-col-type = Type
wallets-holdings-col-decimals = Décimales
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Aucun token détenu
wallets-holdings-empty-message = Les tokens détenus par ce portefeuille apparaîtront ici.
wallets-holdings-no-main = Aucun portefeuille principal
wallets-holdings-main-tag = Principal
wallets-holdings-main-title = Portefeuille principal
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Dernière utilisation
wallets-holdings-never = Jamais
wallets-holdings-search =
    .placeholder = Rechercher par symbole ou mint...
wallets-holdings-export = Exporter la clé
wallets-holdings-export-tooltip = Exporter la clé privée de ce portefeuille
wallets-list-col-name = Nom
wallets-list-col-balance = Solde ({ -sol })
wallets-list-col-type = Type
wallets-list-col-created = Créé
wallets-list-col-actions = Actions
wallets-list-action-export = Exporter la clé privée
wallets-list-action-archive = Archiver le portefeuille
wallets-list-action-restore = Restaurer le portefeuille
wallets-list-action-delete = Supprimer définitivement
wallets-list-count = Portefeuilles
wallets-list-search =
    .placeholder = Rechercher par nom ou adresse...
wallets-list-loading-title = Chargement des portefeuilles…
wallets-list-loading-description = Préparation de la vue de portefeuille sélectionnée.
wallets-secondaries-empty-title = Aucun portefeuille secondaire
wallets-secondaries-empty-message = Créez des portefeuilles supplémentaires pour organiser votre activité de trading sur plusieurs comptes.
wallets-secondaries-add = Ajouter un portefeuille
wallets-archive-empty-title = Aucun portefeuille archivé
wallets-archive-empty-message = Les portefeuilles que vous archivez seront conservés ici en toute sécurité.

wallets-watched-col-wallet = Portefeuille
wallets-watched-col-status = Statut
wallets-watched-col-progress = Progression enregistrée
wallets-watched-col-last-check = Dernière vérification
wallets-watched-unlabelled = Portefeuille sans libellé
wallets-watched-generic-name = portefeuille
wallets-watched-not-synced = Pas encore synchronisé
wallets-watched-not-checked = Pas encore vérifié
wallets-watched-action-copy = Copier les trades
    .title = Ouvrir ce portefeuille dans Copy trading
wallets-watched-action-restore = Restaurer le suivi
wallets-watched-action-options = Options de suivi
wallets-watched-action-retry = Relancer le suivi
wallets-watched-action-pause = Suspendre
wallets-watched-action-enable = Activer
wallets-watched-action-remove =
    .title = Retirer
    .aria-label = Retirer { $name }
wallets-watch-state-paused = Suspendu
wallets-watch-state-catching-up = Rattrapage
wallets-watch-state-watching = Suivi actif
wallets-watch-state-streaming = Flux continu
wallets-watch-state-polling = Interrogation
wallets-watched-detail-helius = Vérification via { -helius } pour ce portefeuille.
wallets-watched-empty-title = Aucune adresse suivie
wallets-watched-empty-message = Utilisez « Suivre un portefeuille » pour enregistrer l'activité on-chain d'un portefeuille public.
wallets-watched-count = Suivis
wallets-watched-search =
    .placeholder = Rechercher parmi les portefeuilles suivis...
wallets-watched-add = Suivre un portefeuille
wallets-watched-refresh = Actualiser les portefeuilles suivis
wallets-watched-loading-title = Chargement des portefeuilles suivis...
wallets-watched-loading-description = Récupération des cibles d'observation.
wallets-watched-load-error-title = Impossible de charger les adresses suivies
wallets-watched-load-error-description = Actualisez pour réessayer.
wallets-watched-address-invalid = Saisissez une adresse de portefeuille Solana valide.
wallets-watched-added = Suivi du portefeuille ajouté
wallets-watched-duplicate = Ce portefeuille est déjà suivi.
wallets-watched-add-failed = Impossible d'ajouter le suivi du portefeuille.
wallets-watched-retried = Suivi du portefeuille restauré avec son curseur enregistré
wallets-watched-paused = Suivi du portefeuille suspendu
wallets-watched-enabled = Suivi du portefeuille activé
wallets-watched-removed = Suivi du portefeuille retiré
wallets-watched-update-failed = Impossible de mettre à jour le suivi du portefeuille
