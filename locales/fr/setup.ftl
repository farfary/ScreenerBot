setup-wallet-required = Saisissez la clé privée d'un portefeuille.
setup-wallet-json-recognized = Format de clé JSON de 64 octets reconnu.
setup-wallet-json-invalid = Utilisez un tableau JSON contenant exactement 64 valeurs d'octets (0–255).
setup-wallet-format-invalid = Utilisez une clé privée base58 ou un tableau JSON de 64 octets.
setup-wallet-base58-recognized = Format de clé base58 reconnu.

setup-rpc-required = Saisissez au moins un endpoint RPC.
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
setup-error-restart-timeout = La configuration est enregistrée, mais { -brand } ne s'est pas encore reconnecté.

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
setup-verify-save-skipped = Non enregistrée
setup-verify-request-failed = Échec de la requête de vérification
setup-verify-summary-checking = Vérification de votre portefeuille et des connexions au mainnet Solana.
setup-verify-summary-running = Vérification des identifiants exacts que vous avez saisis.
setup-verify-summary-saving = Identifiants vérifiés. Enregistrement sécurisé.
setup-verify-summary-failed = Examinez le problème, puis relancez la vérification.

setup-error-credentials-failed = Échec de la vérification des identifiants.
setup-error-save-failed = La configuration n'a pas pu être enregistrée.
setup-error-verify-failed = Échec de la vérification.
setup-error-explore-failed = Le mode Explorer n'a pas pu être démarré.
setup-error-gateway-failed = La préférence de passerelle n'a pas pu être enregistrée.
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
setup-dialog-rpc-label = Endpoint(s) RPC
setup-dialog-rpc-input =
    .placeholder = https://votre-endpoint... (un par ligne)
setup-dialog-rpc-hint = Un fournisseur premium (Helius, QuickNode, Alchemy) est fortement recommandé : le RPC public de Solana est limité en débit et peut ne pas fonctionner.
setup-dialog-submit = Valider et connecter
setup-dialog-working = En cours…
setup-dialog-validating = Validation…
setup-dialog-saving = Enregistrement…
setup-dialog-restarting = Redémarrage…
setup-dialog-saved = Configuration enregistrée — redémarrage de { -brand } en mode complet…
setup-dialog-error-missing-fields = Saisissez une clé privée de portefeuille et au moins une URL RPC.
setup-dialog-error-validation = Échec de la validation.
setup-dialog-error-incomplete = La configuration n'a pas pu être terminée.
setup-dialog-error-restart-helper = L'assistant de redémarrage automatique est indisponible. Rechargez le tableau de bord dans un instant.
setup-dialog-error-unexpected = Erreur inattendue.

setup-wizard-progress =
    .aria-label = Progression de la configuration
setup-wizard-step-credentials = Identifiants
setup-wizard-step-verification = Vérification
setup-wizard-step-complete = Terminé
setup-wizard-credentials-title = Configurer les identifiants
setup-wizard-credentials-description = Connectez un portefeuille local et des endpoints RPC fiables pour le mainnet Solana.
setup-wizard-wallet-toggle =
    .title = Afficher la clé privée
    .aria-label = Afficher la clé privée
setup-wizard-wallet-security-note = Chiffrée avant d'être enregistrée.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Une URL HTTPS par ligne
setup-wizard-rpc-guidance = Un RPC mainnet fiable est recommandé pour l'interrogation continue.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recommandé
setup-wizard-gateway-title = Envoi gratuit des transactions
setup-wizard-gateway-hint = Disponible une fois connecté. Votre RPC reste disponible en secours.
setup-wizard-account-title = Compte { -brand }
setup-wizard-account-optional = Facultatif
setup-wizard-account-loading = Vérification du statut du compte…
setup-wizard-verify-title = Vérifier et enregistrer
setup-wizard-verify-list =
    .aria-label = Statut de la vérification de la configuration
setup-wizard-verify-wallet = Portefeuille
setup-wizard-verify-rpc = RPC Solana
setup-wizard-verify-save = Configuration sécurisée
setup-wizard-complete-title = Configuration enregistrée
setup-wizard-reconnect = Réessayer la connexion
setup-wizard-reload = Recharger le tableau de bord
setup-wizard-error-title = La configuration requiert votre attention
setup-wizard-explore = Explorer le tableau de bord
setup-wizard-continue = Continuer
