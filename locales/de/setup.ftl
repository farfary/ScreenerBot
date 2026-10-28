# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

## Wallet key validation

setup-wallet-required = Geben Sie einen privaten Wallet-Schlüssel ein.
setup-wallet-json-recognized = Das 64-Byte-JSON-Schlüsselformat wurde erkannt.
setup-wallet-json-invalid = Verwenden Sie ein JSON-Array mit genau 64 Bytewerten (0–255).
setup-wallet-format-invalid = Verwenden Sie einen privaten Base58-Schlüssel oder ein 64-Byte-JSON-Array.
setup-wallet-base58-recognized = Das Base58-Schlüsselformat wurde erkannt.

## RPC endpoint validation

setup-rpc-required = Geben Sie mindestens einen RPC-Endpunkt ein.
setup-dialog-rpc-optional-note = Optional: Lassen Sie das Feld leer, um die bereits konfigurierten RPC-Endpunkte zu behalten.
setup-rpc-too-many = Verwenden Sie höchstens 10 RPC-Endpunkte.
setup-rpc-url-invalid = Jeder Endpunkt muss eine gültige HTTPS-URL sein.
setup-rpc-url-credentials = RPC-URLs dürfen keine Benutzernamen oder Passwörter enthalten.
setup-rpc-url-fragment = RPC-URLs dürfen keine Fragmente enthalten.
setup-rpc-public-endpoint = Der öffentliche Solana-RPC kann kein kontinuierliches Polling unterstützen.
setup-rpc-private-host = RPC-Endpunkte dürfen keine lokalen oder privaten Netzwerk-Hosts verwenden.
setup-rpc-duplicate = Entfernen Sie doppelte RPC-Endpunkte.
setup-rpc-ready =
    { $count ->
        [one] { $count } HTTPS-Endpunkt bereit zum Testen.
       *[other] { $count } HTTPS-Endpunkte bereit zum Testen.
    }

## Verification results

setup-wallet-verified = Wallet verifiziert
setup-wallet-unverified = Wallet konnte nicht verifiziert werden
setup-wallet-address-detail = Adresse { $address }
setup-wallet-format-hint = Prüfen Sie das Format des privaten Schlüssels.
setup-rpc-none-working = Kein funktionierender Mainnet-RPC
setup-rpc-health-failed = Kein Endpunkt hat die Mainnet-Integritätsprüfungen bestanden.
setup-rpc-partial = { $working } funktionieren; { $failed } nicht verfügbar
setup-rpc-verified =
    { $count ->
        [one] { $count } Mainnet-Endpunkt verifiziert
       *[other] { $count } Mainnet-Endpunkte verifiziert
    }
setup-rpc-fastest = Am schnellsten: { $url } ({ $latency } ms).
setup-error-request-failed = Anfrage fehlgeschlagen ({ $status })
setup-error-restart-timeout = { -brand } hat sich noch nicht wieder verbunden.

## Verification steps

setup-verify-wallet-parsing = Privater Schlüssel wird gelesen
setup-verify-wallet-parsing-detail = Der Schlüssel wird geprüft und seine öffentliche Adresse abgeleitet.
setup-verify-wallet-waiting = Wartet auf Validierung
setup-verify-rpc-testing = Solana Mainnet wird getestet
setup-verify-rpc-testing-detail =
    { $count ->
        [one] { $count } Endpunkt wird geprüft.
       *[other] { $count } Endpunkte werden geprüft.
    }
setup-verify-rpc-waiting = Wartet auf Endpunkttests
setup-verify-save-waiting = Wartet auf Speicherung
setup-verify-save-running = Verschlüsseln und Speichern
setup-verify-save-running-detail = Die verifizierte Konfiguration wird auf diesem Gerät geschrieben.
setup-verify-save-done = Konfiguration gespeichert
setup-verify-save-done-detail = Privater Schlüssel verschlüsselt; funktionierende RPC-Endpunkte gespeichert.
setup-verify-save-failed = Einrichtung konnte nicht gespeichert werden
setup-verify-save-skipped = Nicht gespeichert. Beheben Sie das Problem oben und prüfen Sie dann erneut.
setup-verify-request-failed = Verifizierungsanfrage fehlgeschlagen
setup-verify-summary-checking = Ihre Wallet und die Solana-Mainnet-Verbindungen werden geprüft.
setup-verify-summary-running = Die von Ihnen eingegebenen Zugangsdaten werden genau verifiziert.
setup-verify-summary-saving = Zugangsdaten verifiziert. Sicheres Speichern läuft.
setup-verify-summary-failed = Prüfen Sie das Problem und verifizieren Sie erneut.

## Errors

setup-error-credentials-failed = Verifizierung der Zugangsdaten fehlgeschlagen.
setup-error-save-failed = Die Einrichtung konnte nicht gespeichert werden.
setup-error-verify-failed = Verifizierung fehlgeschlagen.
setup-error-explore-failed = Der Explore Mode konnte nicht gestartet werden.
setup-error-gateway-failed = Die Einstellung zum Senden von Transaktionen konnte nicht gespeichert werden.
setup-action-review-credentials = Zugangsdaten prüfen

## Completion

setup-explore-opening = Explore Mode wird geöffnet…
setup-complete-restarting = { -brand } wird mit Ihrer verifizierten Konfiguration neu gestartet.
setup-complete-finishing = Neustart wird abgeschlossen…
setup-complete-ready = { -brand } ist bereit. Dashboard wird geöffnet…
setup-complete-stored = Ihre verifizierte Konfiguration ist sicher auf diesem Gerät gespeichert.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = Privaten Schlüssel anzeigen
setup-wallet-hide-key = Privaten Schlüssel ausblenden
setup-wallet-copy =
    .aria-label = Wallet-Adresse kopieren
    .title = Wallet-Adresse kopieren
setup-wallet-copy-done =
    .aria-label = Wallet-Adresse kopiert
    .title = Kopiert
setup-wallet-copy-failed =
    .aria-label = Wallet-Adresse konnte nicht kopiert werden
    .title = Kopieren fehlgeschlagen

## Setup dialog

setup-dialog-close =
    .title = Schließen
    .aria-label = Schließen
setup-dialog-wallet-label = Privater Wallet-Schlüssel
setup-dialog-wallet-input =
    .placeholder = Base58-String oder JSON-Array [1,2,3,...]
setup-dialog-working = Wird ausgeführt…
setup-dialog-validating = Wird validiert…
setup-dialog-saving = Wird gespeichert…
setup-dialog-restarting = Neustart läuft…
setup-dialog-error-validation = Validierung fehlgeschlagen.
setup-dialog-error-incomplete = Die Einrichtung konnte nicht abgeschlossen werden.
setup-dialog-error-restart-helper = Der automatische Neustart-Helfer ist nicht verfügbar. Laden Sie das Dashboard in Kürze neu.
setup-dialog-error-unexpected = Unerwarteter Fehler.
setup-dialog-replace-title = Trading-Wallet ersetzen
setup-dialog-replace-subtitle = Handeln Sie mit einer anderen Wallet. Ihr privater Schlüssel wird auf diesem Gerät verschlüsselt und verlässt es nie.
setup-dialog-replace-submit = Validieren und ersetzen
setup-dialog-replace-saved = Ersetzung vorgemerkt — { -brand } wird mit der neuen Wallet neu gestartet…
setup-dialog-replace-confirm-intro = Beim Ersetzen der Trading-Wallet:
setup-dialog-replace-confirm-stop = Trader und Copy-Trading werden gestoppt. Schalten Sie sie nach dem Neustart wieder ein.
setup-dialog-replace-confirm-backup = Positionen, Transaktionen und Verlauf der aktuellen Wallet werden in einen Sicherungsordner verschoben.
setup-dialog-replace-confirm-keep = Der Schlüssel der aktuellen Wallet bleibt unter Weitere Wallets erhalten, wo Sie ihn weiterhin exportieren können.
setup-dialog-replace-confirm-funds = Es werden keine Guthaben bewegt: Token und { -sol } bleiben in der aktuellen Wallet.
setup-dialog-replace-confirm-rpc = Ersetzt die konfigurierten RPC-Endpunkte durch die eingegebenen; bleibt das Feld leer, bleiben die aktuellen Endpunkte erhalten.
setup-dialog-replace-confirm-restart = { -brand } wird mit der neuen Wallet neu gestartet.

## Setup wizard

setup-wizard-progress =
    .aria-label = Einrichtungsfortschritt
setup-wizard-wallet-security-note = Wird vor dem Speichern verschlüsselt.
setup-wizard-rpc-title = RPC-Endpunkte
setup-wizard-rpc-input =
    .placeholder = Eine HTTPS-URL pro Zeile
setup-wizard-rpc-guidance = Nutzen Sie einen zuverlässigen Mainnet-Anbieter; öffentliche Endpunkte werden nicht unterstützt.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = empfohlen
setup-wizard-account-title = { -brand }-Konto
setup-wizard-account-optional = Optional
setup-wizard-account-loading = Kontostatus wird geprüft…
setup-wizard-verify-title = Verifizieren und speichern
setup-wizard-verify-list =
    .aria-label = Verifizierungsstatus der Einrichtung
setup-wizard-verify-wallet = Wallet
setup-wizard-verify-save = Sichere Konfiguration
setup-wizard-complete-title = Einrichtung gespeichert
setup-wizard-reconnect = Verbindung erneut versuchen
setup-wizard-reload = Dashboard neu laden
setup-wizard-error-title = Die Einrichtung erfordert Ihre Aufmerksamkeit
setup-wizard-explore = Dashboard erkunden
setup-wizard-continue = Weiter

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Wallet
setup-wizard-step-connection = Verbindung
setup-wizard-step-verify = Prüfen
setup-wizard-step-ready = Bereit
setup-wallet-title = Richten Sie Ihre Trading-Wallet ein
setup-wallet-description = Die Wallet, mit der { -brand } handelt. Ihr Schlüssel wird auf diesem Computer verschlüsselt und verlässt ihn nie.
setup-wallet-source =
    .aria-label = Wallet-Quelle
setup-wallet-create-title = Neue Wallet erstellen
setup-wallet-create-detail = { -brand } erstellt eine neue Wallet. Sie sichern ihren Schlüssel und senden dann { -sol } an sie.
setup-wallet-import-title = Wallet importieren
setup-wallet-import-detail = Verwenden Sie einen vorhandenen privaten Schlüssel (base58 oder ein JSON-Array mit 64 Zahlen).
setup-wallet-creating = Wallet wird erstellt…
setup-wallet-create-failed = Die Wallet konnte nicht erstellt werden. Versuchen Sie es erneut.
setup-backup-title = Privaten Schlüssel sichern
setup-backup-description = Wer diesen Schlüssel besitzt, kontrolliert die Wallet. { -brand } kann ihn nicht wiederherstellen.
setup-backup-address = Adresse
setup-backup-key = Privater Schlüssel
setup-backup-copy-address = Adresse kopieren
setup-backup-download = Herunterladen
setup-backup-copied = Kopiert. Leeren Sie die Zwischenablage, nachdem Sie ihn gesichert haben.
setup-backup-machine-warning = Die verschlüsselte Kopie, die { -brand } aufbewahrt, lässt sich nur auf diesem Computer öffnen. Geht der Computer verloren oder wird er ersetzt, stellt nur der Schlüssel, den Sie jetzt sichern, die Wallet wieder her. Bewahren Sie ihn offline auf, zum Beispiel auf Papier oder in einem Passwortmanager.
setup-backup-ack-saved = Ich habe den privaten Schlüssel an einem sicheren Ort außerhalb dieses Computers gespeichert.
setup-backup-ack-control = Mir ist bewusst, dass jeder mit diesem Schlüssel das Guthaben dieser Wallet nehmen kann.
setup-backup-ack-recovery = Mir ist bewusst, dass { -brand } diesen Schlüssel nicht wiederherstellen kann. Ohne ihn bedeutet ein verlorener Computer verlorenes Guthaben.
setup-backup-create-another = Neue erstellen
setup-backup-replace-confirm = Diesen Schlüssel verwerfen und eine neue Wallet erstellen? Tun Sie das nur, wenn Sie noch nichts an sie gesendet haben.
setup-connection-title = Wie soll sich { -brand } mit { -solana } verbinden?
setup-connection-description = Sie können das später unter Einstellungen → Verbindung ändern.
setup-connection-choice =
    .aria-label = Verbindung
setup-connection-service-title = { -brand }-Dienst
setup-connection-service-badge = Kein RPC nötig
setup-connection-service-detail = Nutzt Ihr kostenloses { -brand }-Konto.
setup-connection-service-prices = Preise werden alle paar Sekunden aktualisiert
setup-connection-service-features = Copy-Trading, Wallet-Überwachung, Guthaben anderer Wallets, ältere Historie, direkte Pool-Swaps und Live-Poolpreise erfordern Ihren eigenen RPC
setup-connection-service-pause = Der Handel pausiert, wenn Sie sich abmelden oder screenerbot.io nicht erreichbar ist
setup-connection-service-privacy = Ihr privater Schlüssel verlässt diesen Computer nie: Swaps werden hier signiert, und screenerbot.io nutzt nur die öffentliche Adresse Ihrer Wallet, um Guthaben und Transaktionen zu lesen
setup-connection-own-title = Eigener RPC
setup-connection-own-badge = Am schnellsten
setup-connection-own-detail = Preise etwa zweimal pro Sekunde und alle Funktionen. Benötigt einen RPC-Endpunkt eines Anbieters.
setup-connection-signin-required = Melden Sie sich an oder erstellen Sie ein kostenloses Konto, um den { -brand }-Dienst zu nutzen.
setup-connection-reauthorize = Melden Sie sich erneut an, um Wallet-Abfragen für den { -brand }-Dienst zu erlauben.
setup-connection-sending = Transaktionen über { -brand } senden
setup-connection-sending-hint = Verfügbar, wenn Sie angemeldet sind. Ist das nicht möglich, wird Ihr RPC verwendet.
setup-wizard-account-required = Erforderlich
setup-wizard-verify-connection = Verbindung
setup-verify-service-checking = { -brand }-Dienst wird geprüft
setup-verify-service-ready = { -brand }-Dienst bereit
setup-verify-service-busy = Der { -brand }-Dienst ist gerade ausgelastet. Versuchen Sie es gleich noch einmal oder nutzen Sie Ihren eigenen RPC.
setup-verify-service-daily-limit = Ihr Konto hat das heutige Limit für den { -brand }-Dienst erreicht. Versuchen Sie es nach 00:00 UTC erneut oder nutzen Sie Ihren eigenen RPC.
setup-verify-service-unreachable = screenerbot.io ist nicht erreichbar. Prüfen Sie Ihre Internetverbindung.
setup-verify-service-version = Diese Version wird vom { -brand }-Dienst nicht mehr unterstützt. Aktualisieren Sie { -brand }, um fortzufahren.
setup-verify-service-signin = Melden Sie sich an, um den { -brand }-Dienst zu nutzen.
setup-verify-save-done-service-detail = Privater Schlüssel verschlüsselt; { -brand }-Dienst ausgewählt.
setup-action-try-again = Erneut versuchen
setup-action-use-own-rpc = Eigenen RPC verwenden
setup-action-sign-in-again = Erneut anmelden
setup-ready-title = { -brand } ist bereit
setup-fund-title = Trading-Wallet aufladen
setup-fund-waiting = Warten auf { -sol }…
setup-fund-received = { $amount } { -sol } empfangen
setup-fund-balance-unavailable = Guthaben wird gelesen… Es aktualisiert sich von selbst.
setup-fund-open-dashboard = Dashboard öffnen
