## Wallet mismatch.

startup-wallet-mismatch-title = Wallet geändert
startup-wallet-mismatch-detail =
    Die Wallet in Ihrer Konfiguration stimmt nicht mit der Wallet überein, die im lokalen Verlauf dieses Computers gespeichert ist.

    Aktuelle Wallet: { $current }
    Vorherige Wallet: { $stored }

    Betroffene lokale Daten: { $systems }

    Das passiert meist nach dem Import eines anderen privaten Schlüssels oder der Wiederherstellung einer anderen Konfiguration. Trading, Positionen und Verlauf gehören zur vorherigen Wallet und müssen gelöscht werden, bevor die neue Wallet sicher starten kann.
startup-wallet-mismatch-systems-default = Transaktionen, Positionen, Wallet-Verlauf
startup-wallet-mismatch-remedy =
    Löschen Sie den lokalen Verlauf der vorherigen Wallet, um fortzufahren (Ihre Datenbanken werden vorher automatisch gesichert):

      - In der App: Wählen Sie unten „{ $action }“.
      - Über ein Terminal: Führen Sie  screenerbot --clean-wallet-data  aus

    On-Chain-Guthaben sind nicht betroffen; nur der lokale Trade- und Positionsverlauf dieses Computers wird zurückgesetzt. Sicherungen werden hier abgelegt:
      { $path }
startup-recovery-reset-wallet = Wallet-Daten zurücksetzen und neu starten

## Port in use.

startup-port-in-use-title = Netzwerkport ist belegt
startup-port-in-use-detail = Der Dashboard-Port { $address } wird bereits verwendet.
startup-port-in-use-remedy = Ein anderes Programm verwendet den Port, den { -brand } benötigt. Beenden Sie dieses Programm oder ändern Sie den Webserver-Port in den Einstellungen und starten Sie { -brand } dann erneut.

## Another instance is running.

startup-lock-held-title = { -brand } läuft bereits
startup-lock-held-detail = Auf diesem Computer läuft bereits eine weitere Instanz von { -brand }, daher kann keine zweite starten.
startup-lock-held-remedy = Wechseln Sie zum bereits geöffneten Fenster. Falls keines sichtbar ist, beenden Sie alle { -brand }-Hintergrundprozesse und versuchen Sie es erneut. Besteht das Problem nach einem Neustart weiter, ist die Sperrdatei möglicherweise veraltet und kann aus dem Datenordner entfernt werden (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Konfiguration konnte nicht gelesen werden
startup-config-parse-detail = config.toml konnte nicht geparst werden: { $detail }
startup-config-load-parse-detail = Konfiguration konnte nicht geladen werden: config.toml konnte nicht geparst werden: { $detail }
startup-config-parse-remedy = Ihre Konfigurationsdatei konnte nicht gelesen werden. Stellen Sie eine Sicherung aus dem Datenordner wieder her oder setzen Sie die Konfiguration auf die Standardwerte zurück und richten Sie Wallet und RPC erneut ein.
startup-config-load-parse-remedy = Stellen Sie eine gültige Konfiguration wieder her oder schließen Sie die Einrichtung erneut ab.
startup-option-invalid-title = Ungültige Startoption
startup-option-invalid-remedy = Eine Kommandozeilenoption ist ungültig. Starten Sie { -brand } ohne diese Option oder korrigieren Sie sie und versuchen Sie es erneut.

## Storage upgrade.

startup-storage-upgrade-title = Deine Daten konnten nicht aktualisiert werden
startup-storage-upgrade-detail =
    { -brand } konnte { $database } nicht auf diese Version aktualisieren und hat vor jeder Änderung angehalten. Deine Daten wurden nicht verändert.

    Ursache:
    { $error }
startup-storage-upgrade-remedy = Kopiere die Details und sende sie zusammen mit der Logdatei an den Support unter t.me/screenerbotio_support. Bearbeite, verschiebe oder lösche die Datenbank nicht: { -brand } öffnet sie wieder, sobald eine Korrektur installiert ist.

## Generic failures.

startup-generic-title = { -brand } konnte nicht gestartet werden
startup-generic-remedy = Details finden Sie in der Logdatei. Starten Sie die App anschließend neu. Besteht das Problem weiter, wenden Sie sich an den Support unter t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Erforderliche Verzeichnisse konnten nicht erstellt werden: { $error }
startup-failure-config-load = Konfiguration konnte nicht geladen werden: { $error }
startup-failure-actions-init = Aktionsdatenbank konnte nicht initialisiert werden: { $error }
startup-failure-actions-sync = Aktionen konnten nicht aus der Datenbank synchronisiert werden: { $error }
startup-failure-strategy-init = Strategiesystem konnte nicht initialisiert werden: { $error }
startup-failure-analysis-init = Analyse-Engine konnte nicht initialisiert werden: { $error }
startup-failure-assistant-init = Chat-Engine des Assistenten konnte nicht initialisiert werden: { $error }
startup-failure-wallets-init = Wallets konnten nicht initialisiert werden: { $error }
startup-failure-wallet-validation = Wallet-Konsistenz konnte nicht geprüft werden: { $error }
