setup-wallet-required = Saisissez la clé privée d'un portefeuille.
setup-wallet-json-recognized = Format de clé JSON de 64 octets reconnu.
setup-wallet-json-invalid = Utilisez un tableau JSON contenant exactement 64 valeurs d'octets (0–255).
setup-wallet-format-invalid = Utilisez une clé privée base58 ou un tableau JSON de 64 octets.
setup-wallet-base58-recognized = Format de clé base58 reconnu.

setup-rpc-required = Saisissez au moins un endpoint RPC.
setup-dialog-rpc-optional-note = Facultatif : laissez le champ vide pour conserver les endpoints RPC déjà configurés.
setup-rpc-too-many = Utilisez au maximum 10 endpoints RPC.
setup-rpc-url-invalid = Chaque endpoint doit être une URL HTTPS valide.
setup-rpc-url-credentials = Les URL RPC ne peuvent pas contenir de nom d'utilisateur ni de mot de passe.
setup-rpc-url-fragment = Les URL RPC ne peuvent pas contenir de fragment.
setup-rpc-public-endpoint = Le RPC public de Solana ne peut pas supporter une interrogation continue.
setup-rpc-private-host = Les endpoints RPC ne peuvent pas utiliser d'hôtes locaux ou de réseau privé.
setup-rpc-duplicate = Supprimez les endpoints RPC en double.
setup-rpc-ready =
    { $count ->
        [one] { $count } endpoint HTTPS prêt à être testé.
        [many] { $count } endpoints HTTPS prêts à être testés.
       *[other] { $count } endpoints HTTPS prêts à être testés.
    }

setup-wallet-verified = Portefeuille vérifié
setup-wallet-unverified = Le portefeuille n'a pas pu être vérifié
setup-wallet-address-detail = Adresse { $address }
setup-wallet-format-hint = Vérifiez le format de la clé privée.
setup-rpc-none-working = Aucun RPC mainnet fonctionnel
setup-rpc-health-failed = Aucun endpoint n'a passé les contrôles de santé du mainnet.
setup-rpc-partial = { $working } fonctionnels ; { $failed } indisponibles
setup-rpc-verified =
    { $count ->
        [one] { $count } endpoint mainnet vérifié
        [many] { $count } endpoints mainnet vérifiés
       *[other] { $count } endpoints mainnet vérifiés
    }
setup-rpc-fastest = Le plus rapide : { $url } ({ $latency }ms).
setup-error-request-failed = Échec de la requête ({ $status })
setup-error-restart-timeout = { -brand } ne s'est pas encore reconnecté.

setup-verify-wallet-parsing = Analyse de la clé privée
setup-verify-wallet-parsing-detail = Vérification de la clé et dérivation de son adresse publique.
setup-verify-wallet-waiting = En attente de validation
setup-verify-rpc-testing = Test du mainnet Solana
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Vérification de { $count } endpoint.
        [many] Vérification de { $count } endpoints.
       *[other] Vérification de { $count } endpoints.
    }
setup-verify-rpc-waiting = En attente du test des endpoints
setup-verify-save-waiting = En attente de l'enregistrement
setup-verify-save-running = Chiffrement et enregistrement
setup-verify-save-running-detail = Écriture de la configuration vérifiée sur cet appareil.
setup-verify-save-done = Configuration enregistrée
setup-verify-save-done-detail = Clé privée chiffrée ; endpoints RPC fonctionnels enregistrés.
setup-verify-save-failed = Impossible d'enregistrer la configuration
setup-verify-save-skipped = Non enregistrée. Corrigez le problème ci-dessus, puis vérifiez à nouveau.
setup-verify-request-failed = Échec de la requête de vérification
setup-verify-summary-checking = Vérification de votre portefeuille et des connexions au mainnet Solana.
setup-verify-summary-running = Vérification des identifiants exacts que vous avez saisis.
setup-verify-summary-saving = Identifiants vérifiés. Enregistrement sécurisé.
setup-verify-summary-failed = Examinez le problème, puis relancez la vérification.

setup-error-credentials-failed = Échec de la vérification des identifiants.
setup-error-save-failed = La configuration n'a pas pu être enregistrée.
setup-error-verify-failed = Échec de la vérification.
setup-error-explore-failed = Le mode Explorer n'a pas pu être démarré.
setup-error-gateway-failed = Le paramètre d'envoi des transactions n'a pas pu être enregistré.
setup-action-review-credentials = Revoir les identifiants

setup-explore-opening = Ouverture du mode Explorer…
setup-complete-restarting = Redémarrage de { -brand } avec votre configuration vérifiée.
setup-complete-finishing = Finalisation du redémarrage…
setup-complete-ready = { -brand } est prêt. Ouverture du tableau de bord…
setup-complete-stored = Votre configuration vérifiée est stockée en toute sécurité sur cet appareil.

setup-wallet-show-key = Afficher la clé privée
setup-wallet-hide-key = Masquer la clé privée
setup-wallet-copy =
    .aria-label = Copier l'adresse du portefeuille
    .title = Copier l'adresse du portefeuille
setup-wallet-copy-done =
    .aria-label = Adresse du portefeuille copiée
    .title = Copié
setup-wallet-copy-failed =
    .aria-label = Impossible de copier l'adresse du portefeuille
    .title = Échec de la copie

setup-dialog-title = Configurer le portefeuille et le RPC
setup-dialog-subtitle = Connectez votre portefeuille Solana et un endpoint RPC premium pour activer le trading et les données on-chain en direct. Votre clé privée est chiffrée sur cet appareil et ne le quitte jamais.
setup-dialog-close =
    .title = Fermer
    .aria-label = Fermer
setup-dialog-wallet-label = Clé privée du portefeuille
setup-dialog-wallet-input =
    .placeholder = Chaîne base58 ou tableau JSON [1,2,3,...]
setup-dialog-submit = Valider et connecter
setup-dialog-working = En cours…
setup-dialog-validating = Validation…
setup-dialog-saving = Enregistrement…
setup-dialog-restarting = Redémarrage…
setup-dialog-saved = Configuration enregistrée — redémarrage de { -brand } en mode complet…
setup-dialog-error-validation = Échec de la validation.
setup-dialog-error-incomplete = La configuration n'a pas pu être terminée.
setup-dialog-error-restart-helper = L'assistant de redémarrage automatique est indisponible. Rechargez le tableau de bord dans un instant.
setup-dialog-error-unexpected = Erreur inattendue.
setup-dialog-replace-title = Remplacer le portefeuille de trading
setup-dialog-replace-subtitle = Tradez avec un autre portefeuille. Sa clé privée est chiffrée sur cet appareil et ne le quitte jamais.
setup-dialog-replace-submit = Valider et remplacer
setup-dialog-replace-saved = Remplacement préparé — redémarrage de { -brand } avec le nouveau portefeuille…
setup-dialog-replace-confirm-intro = Remplacer le portefeuille de trading :
setup-dialog-replace-confirm-stop = Arrête le trader et le copy trading. Réactivez-les après le redémarrage.
setup-dialog-replace-confirm-backup = Déplace les positions, transactions et l'historique du portefeuille actuel dans un dossier de sauvegarde.
setup-dialog-replace-confirm-keep = Conserve la clé du portefeuille actuel dans Autres portefeuilles, où vous pouvez toujours l'exporter.
setup-dialog-replace-confirm-funds = Ne déplace aucun fonds : les tokens et les { -sol } restent dans le portefeuille actuel.
setup-dialog-replace-confirm-rpc = Remplace les endpoints RPC configurés par ceux que vous saisissez ; si le champ reste vide, les endpoints actuels sont conservés.
setup-dialog-replace-confirm-restart = Redémarre { -brand } avec le nouveau portefeuille.

setup-wizard-progress =
    .aria-label = Progression de la configuration
setup-wizard-wallet-security-note = Chiffrée avant d'être enregistrée.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Une URL HTTPS par ligne
setup-wizard-rpc-guidance = Utilisez un fournisseur mainnet fiable ; les endpoints publics ne sont pas pris en charge.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recommandé
setup-wizard-account-title = Compte { -brand }
setup-wizard-account-optional = Facultatif
setup-wizard-account-loading = Vérification du statut du compte…
setup-wizard-verify-title = Vérifier et enregistrer
setup-wizard-verify-list =
    .aria-label = Statut de la vérification de la configuration
setup-wizard-verify-wallet = Portefeuille
setup-wizard-verify-save = Configuration sécurisée
setup-wizard-complete-title = Configuration enregistrée
setup-wizard-reconnect = Réessayer la connexion
setup-wizard-reload = Recharger le tableau de bord
setup-wizard-error-title = La configuration requiert votre attention
setup-wizard-explore = Explorer le tableau de bord
setup-wizard-continue = Continuer

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Wallet
setup-wizard-step-connection = Connexion
setup-wizard-step-verify = Vérifier
setup-wizard-step-ready = Prêt
setup-wallet-title = Configurez votre wallet de trading
setup-wallet-description = Le wallet avec lequel { -brand } trade. Sa clé est chiffrée sur cet ordinateur et ne le quitte jamais.
setup-wallet-source =
    .aria-label = Source du wallet
setup-wallet-create-title = Créer un nouveau wallet
setup-wallet-create-detail = { -brand } crée un nouveau wallet. Vous sauvegardez sa clé, puis vous y envoyez des { -sol }.
setup-wallet-import-title = Importer un wallet
setup-wallet-import-detail = Utilisez une clé privée que vous possédez déjà (base58 ou tableau JSON de 64 nombres).
setup-wallet-creating = Création de votre wallet…
setup-wallet-create-failed = Le wallet n’a pas pu être créé. Réessayez.
setup-backup-title = Sauvegardez votre clé privée
setup-backup-description = Quiconque détient cette clé contrôle le wallet. { -brand } ne peut pas la récupérer.
setup-backup-address = Adresse
setup-backup-key = Clé privée
setup-backup-copy-address = Copier l’adresse
setup-backup-download = Télécharger
setup-backup-copied = Copié. Videz votre presse-papiers après l’avoir sauvegardée.
setup-backup-machine-warning = La copie chiffrée conservée par { -brand } ne s’ouvre que sur cet ordinateur. Si l’ordinateur est perdu ou remplacé, seule la clé que vous sauvegardez maintenant restaure le wallet. Conservez-la hors ligne, par exemple sur papier ou dans un gestionnaire de mots de passe.
setup-backup-ack-saved = J’ai sauvegardé la clé privée en lieu sûr, en dehors de cet ordinateur.
setup-backup-ack-control = Je comprends que quiconque possède cette clé peut prendre les fonds de ce wallet.
setup-backup-ack-recovery = Je comprends que { -brand } ne peut pas récupérer cette clé. Sans elle, un ordinateur perdu signifie des fonds perdus.
setup-backup-create-another = En créer un autre
setup-backup-replace-confirm = Abandonner cette clé et créer un nouveau wallet ? Ne le faites que si vous n’y avez rien envoyé.
setup-connection-title = Comment { -brand } doit-il se connecter à { -solana } ?
setup-connection-description = Vous pourrez le modifier plus tard dans Paramètres → Connexion.
setup-connection-choice =
    .aria-label = Connexion
setup-connection-service-title = Service { -brand }
setup-connection-service-badge = Aucun RPC requis
setup-connection-service-detail = Utilise votre compte { -brand } gratuit.
setup-connection-service-prices = Les prix se mettent à jour toutes les quelques secondes
setup-connection-service-features = Le copy trading, le suivi de wallets, les soldes des autres wallets, l'historique ancien, les swaps directs en pool et les prix de pool en direct nécessitent votre propre RPC
setup-connection-service-pause = Le trading se met en pause si vous vous déconnectez ou si screenerbot.io est injoignable
setup-connection-service-privacy = Votre clé privée ne quitte jamais cet ordinateur : les swaps sont signés ici, et screenerbot.io n'utilise que l'adresse publique de votre portefeuille pour lire ses soldes et ses transactions
setup-connection-own-title = Votre propre RPC
setup-connection-own-badge = Le plus rapide
setup-connection-own-detail = Prix environ deux fois par seconde et toutes les fonctionnalités. Nécessite un endpoint RPC d’un fournisseur.
setup-connection-signin-required = Connectez-vous ou créez un compte gratuit pour utiliser le service { -brand }.
setup-connection-reauthorize = Reconnectez-vous pour autoriser les lectures du wallet par le service { -brand }.
setup-connection-sending = Envoyer les transactions via { -brand }
setup-connection-sending-hint = Disponible une fois connecté. Votre RPC est utilisé quand ce n’est pas possible.
setup-wizard-account-required = Requis
setup-wizard-verify-connection = Connexion
setup-verify-service-checking = Vérification du service { -brand }
setup-verify-service-ready = Service { -brand } prêt
setup-verify-service-busy = Le service { -brand } est occupé pour le moment. Réessayez dans un instant, ou utilisez votre propre RPC.
setup-verify-service-daily-limit = Votre compte a atteint la limite du jour pour le service { -brand }. Réessayez après 00:00 UTC, ou utilisez votre propre RPC.
setup-verify-service-unreachable = screenerbot.io est injoignable. Vérifiez votre connexion internet.
setup-verify-service-version = Cette version n’est plus prise en charge par le service { -brand }. Mettez à jour { -brand } pour continuer.
setup-verify-service-signin = Connectez-vous pour utiliser le service { -brand }.
setup-verify-save-done-service-detail = Clé privée chiffrée ; service { -brand } sélectionné.
setup-action-try-again = Réessayer
setup-action-use-own-rpc = Utiliser mon propre RPC
setup-action-sign-in-again = Se reconnecter
setup-ready-title = { -brand } est prêt
setup-fund-title = Alimentez votre wallet de trading
setup-fund-waiting = En attente de { -sol }…
setup-fund-received = { $amount } { -sol } reçus
setup-fund-balance-unavailable = Lecture du solde… Il se met à jour tout seul.
setup-fund-open-dashboard = Ouvrir le tableau de bord
