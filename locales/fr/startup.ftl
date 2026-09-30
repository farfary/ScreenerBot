## Wallet mismatch.

startup-wallet-mismatch-title = Portefeuille modifié
startup-wallet-mismatch-detail =
    Le portefeuille de votre configuration ne correspond pas à celui enregistré dans l'historique local de cet ordinateur.

    Portefeuille actuel : { $current }
    Portefeuille précédent : { $stored }

    Données locales concernées : { $systems }

    Cela se produit généralement après l'import d'une autre clé privée ou la restauration d'une autre configuration. Le trading, les positions et l'historique appartiennent au portefeuille précédent et doivent être effacés avant que le nouveau portefeuille puisse démarrer en toute sécurité.
startup-wallet-mismatch-systems-default = Transactions, Positions, Historique du portefeuille
startup-wallet-mismatch-remedy =
    Effacez l'historique local du portefeuille précédent pour continuer (vos bases de données sont d'abord sauvegardées automatiquement) :

      - Dans l'application : choisissez « { $action } » ci-dessous.
      - Depuis un terminal : exécutez  screenerbot --clean-wallet-data

    Aucun fonds on-chain n'est affecté ; seul l'historique local des trades et positions de cet ordinateur est réinitialisé. Les sauvegardes sont écrites dans :
      { $path }
startup-recovery-reset-wallet = Réinitialiser les données du portefeuille et redémarrer

## Port in use.

startup-port-in-use-title = Le port réseau est occupé
startup-port-in-use-detail = Le port du tableau de bord { $address } est déjà utilisé.
startup-port-in-use-remedy = Un autre programme utilise le port dont { -brand } a besoin. Fermez ce programme, ou modifiez le port du serveur web dans les Paramètres, puis relancez { -brand }.

## Another instance is running.

startup-lock-held-title = { -brand } est déjà en cours d'exécution
startup-lock-held-detail = Une autre instance de { -brand } est déjà en cours d'exécution sur cet ordinateur ; une seconde ne peut donc pas démarrer.
startup-lock-held-remedy = Passez à la fenêtre déjà ouverte. Si vous n'en voyez aucune, quittez tout processus { -brand } en arrière-plan et réessayez. Si le problème persiste après un redémarrage, le fichier de verrouillage est peut-être obsolète et peut être supprimé du dossier de données (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Impossible de lire la configuration
startup-config-parse-detail = Impossible d'analyser config.toml : { $detail }
startup-config-load-parse-detail = Échec du chargement de la configuration : impossible d'analyser config.toml : { $detail }
startup-config-parse-remedy = Votre fichier de configuration n'a pas pu être lu. Restaurez une sauvegarde depuis le dossier de données, ou rétablissez la configuration par défaut et configurez à nouveau votre portefeuille et votre RPC.
startup-config-load-parse-remedy = Restaurez une configuration valide ou terminez de nouveau la configuration initiale.
startup-option-invalid-title = Option de démarrage non valide
startup-option-invalid-remedy = Une option de ligne de commande n'est pas valide. Démarrez { -brand } sans cette option, ou corrigez-la et réessayez.

## Generic failures.

startup-generic-title = { -brand } n'a pas pu démarrer
startup-generic-remedy = Consultez le fichier journal pour plus de détails, puis redémarrez l'application. Si le problème persiste, contactez le support à t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Échec de la création des dossiers requis : { $error }
startup-failure-config-load = Échec du chargement de la configuration : { $error }
startup-failure-actions-init = Échec de l'initialisation de la base de données des actions : { $error }
startup-failure-actions-sync = Échec de la synchronisation des actions depuis la base de données : { $error }
startup-failure-strategy-init = Échec de l'initialisation du système de stratégies : { $error }
startup-failure-analysis-init = Échec de l'initialisation du moteur d'analyse : { $error }
startup-failure-assistant-init = Échec de l'initialisation du moteur de chat de l'Assistant : { $error }
startup-failure-wallets-init = Échec de l'initialisation des portefeuilles : { $error }
startup-failure-wallet-validation = Échec de la validation de la cohérence des portefeuilles : { $error }
