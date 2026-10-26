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

## Storage upgrade.

startup-storage-upgrade-title = Vos données n'ont pas pu être mises à niveau
startup-storage-upgrade-detail =
    { -brand } n'a pas pu mettre { $database } à niveau vers cette version et s'est arrêté avant de la modifier. Vos données n'ont pas été modifiées.

    Cause :
    { $error }
startup-storage-upgrade-remedy = Copiez les détails et envoyez-les avec le fichier journal au support sur t.me/screenerbotio_support. Ne modifiez, ne déplacez et ne supprimez pas la base de données : { -brand } la rouvrira dès qu'un correctif sera installé.

## Wallet replacement and wallet data reset.

startup-wallet-replacement-incomplete-title = Le remplacement du portefeuille n'a pas pu aboutir
startup-wallet-replacement-incomplete-detail =
    { -brand } s'est arrêté pendant le passage au nouveau portefeuille de trading. Rien n'a été supprimé : la clé de l'ancien portefeuille reste dans le coffre des portefeuilles, et son historique local est soit toujours en place, soit dans le dossier de sauvegarde indiqué dans le fichier journal.

    Cause :
    { $error }
startup-wallet-replacement-incomplete-remedy = Relancez { -brand } pour reprendre le remplacement. Si ce message réapparaît, copiez les détails et envoyez-les avec le fichier journal au support sur t.me/screenerbotio_support. Ne modifiez, ne déplacez et ne supprimez ni wallet_replacement.json ni le dossier de sauvegarde.
startup-wallet-reset-refused-title = Les données du portefeuille n'ont pas été réinitialisées
startup-wallet-reset-refused-open-positions = Le portefeuille de trading a des positions ouvertes ({ $count }) : les données du portefeuille n'ont donc pas été réinitialisées.
startup-wallet-reset-refused-positions-unreadable = Le stockage des positions n'a pas pu être lu : des positions ouvertes ne peuvent pas être exclues et l'historique local du portefeuille a été conservé.
startup-wallet-reset-refused-remedy = Rien n'a été déplacé. Démarrez { -brand } sans la réinitialisation, clôturez ou archivez les positions ouvertes du portefeuille de trading, puis réinitialisez à nouveau les données du portefeuille.
startup-wallet-reset-refused-unreadable-remedy = Rien n'a été déplacé. Démarrez { -brand } sans la réinitialisation pour qu'il vérifie le stockage des positions, puis réinitialisez à nouveau les données du portefeuille. Si ce message réapparaît, envoyez le fichier journal au support sur t.me/screenerbotio_support.
startup-wallet-reset-incomplete-title = La réinitialisation des données du portefeuille n'a pas pu se terminer
startup-wallet-reset-incomplete-detail =
    { -brand } s'est arrêté pendant le déplacement des données du portefeuille vers { $directory } et n'a pas pu remettre tous les fichiers en place. Une partie de l'historique local du portefeuille se trouve dans ce dossier et une partie est toujours en place.

    Cause :
    { $error }
startup-wallet-reset-incomplete-remedy = Ne redémarrez pas { -brand } et ne déplacez, ne modifiez ni ne supprimez aucun fichier de { $directory }. Copiez les détails et envoyez-les avec le fichier journal au support sur t.me/screenerbotio_support.

## Generic failures.

startup-generic-title = { -brand } n'a pas pu démarrer
startup-generic-remedy = Consultez le fichier journal pour plus de détails, puis redémarrez l'application. Si le problème persiste, contactez le support à t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Échec de la création des dossiers requis : { $error }
startup-failure-config-load = Échec du chargement de la configuration : { $error }
startup-failure-actions-sync = Échec de la synchronisation des actions depuis la base de données : { $error }
startup-failure-analysis-init = Échec de l'initialisation du moteur d'analyse : { $error }
startup-failure-assistant-init = Échec de l'initialisation du moteur de chat de l'Assistant : { $error }
startup-failure-wallets-init = Échec de l'initialisation des portefeuilles : { $error }
startup-failure-wallet-validation = Échec de la validation de la cohérence des portefeuilles : { $error }
startup-failure-storage-phase = La phase de stockage s'est arrêtée de manière inattendue : { $error }
