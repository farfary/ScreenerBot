# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

## Wallet key validation

setup-wallet-required = Geben Sie einen privaten Wallet-Schlüssel ein.
setup-wallet-json-recognized = Das 64-Byte-JSON-Schlüsselformat wurde erkannt.
setup-wallet-json-invalid = Verwenden Sie ein JSON-Array mit genau 64 Bytewerten (0–255).
setup-wallet-format-invalid = Verwenden Sie einen privaten Base58-Schlüssel oder ein 64-Byte-JSON-Array.
setup-wallet-base58-recognized = Das Base58-Schlüsselformat wurde erkannt.

## RPC endpoint validation

setup-rpc-required = Geben Sie mindestens einen RPC-Endpunkt ein.
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
setup-error-restart-timeout = Die Einrichtung ist gespeichert, aber { -brand } hat sich noch nicht wieder verbunden.

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
setup-verify-save-skipped = Nicht gespeichert
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
setup-error-gateway-failed = Die Gateway-Einstellung konnte nicht gespeichert werden.
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

setup-dialog-title = Wallet und RPC einrichten
setup-dialog-subtitle = Verbinden Sie Ihre Solana-Wallet und einen Premium-RPC-Endpunkt, um Trading und Live-On-Chain-Daten zu aktivieren. Ihr privater Schlüssel wird auf diesem Gerät verschlüsselt und verlässt es nie.
setup-dialog-close =
    .title = Schließen
    .aria-label = Schließen
setup-dialog-wallet-label = Privater Wallet-Schlüssel
setup-dialog-wallet-input =
    .placeholder = Base58-String oder JSON-Array [1,2,3,...]
setup-dialog-rpc-label = RPC-Endpunkt(e)
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (einer pro Zeile)
setup-dialog-rpc-hint = Ein Premium-Anbieter (Helius, QuickNode, Alchemy) wird dringend empfohlen — der öffentliche Solana-RPC ist ratenbegrenzt und funktioniert möglicherweise nicht.
setup-dialog-submit = Validieren und verbinden
setup-dialog-working = Wird ausgeführt…
setup-dialog-validating = Wird validiert…
setup-dialog-saving = Wird gespeichert…
setup-dialog-restarting = Neustart läuft…
setup-dialog-saved = Einrichtung gespeichert — { -brand } wird im Vollmodus neu gestartet…
setup-dialog-error-missing-fields = Geben Sie einen privaten Wallet-Schlüssel und mindestens eine RPC-URL ein.
setup-dialog-error-validation = Validierung fehlgeschlagen.
setup-dialog-error-incomplete = Die Einrichtung konnte nicht abgeschlossen werden.
setup-dialog-error-restart-helper = Der automatische Neustart-Helfer ist nicht verfügbar. Laden Sie das Dashboard in Kürze neu.
setup-dialog-error-unexpected = Unerwarteter Fehler.

## Setup wizard

setup-wizard-progress =
    .aria-label = Einrichtungsfortschritt
setup-wizard-step-credentials = Zugangsdaten
setup-wizard-step-verification = Verifizierung
setup-wizard-step-complete = Abschluss
setup-wizard-credentials-title = Zugangsdaten konfigurieren
setup-wizard-credentials-description = Verbinden Sie eine lokale Wallet und zuverlässige Solana-Mainnet-RPC-Endpunkte.
setup-wizard-wallet-toggle =
    .title = Privaten Schlüssel anzeigen
    .aria-label = Privaten Schlüssel anzeigen
setup-wizard-wallet-security-note = Wird vor dem Speichern verschlüsselt.
setup-wizard-rpc-title = RPC-Endpunkte
setup-wizard-rpc-input =
    .placeholder = Eine HTTPS-URL pro Zeile
setup-wizard-rpc-guidance = Für kontinuierliches Polling wird ein zuverlässiger Mainnet-RPC empfohlen.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = empfohlen
setup-wizard-gateway-title = Kostenloses Senden von Transaktionen
setup-wizard-gateway-hint = Verfügbar bei Anmeldung. Ihr RPC bleibt als Fallback verfügbar.
setup-wizard-account-title = { -brand }-Konto
setup-wizard-account-optional = Optional
setup-wizard-account-loading = Kontostatus wird geprüft…
setup-wizard-verify-title = Verifizieren und speichern
setup-wizard-verify-list =
    .aria-label = Verifizierungsstatus der Einrichtung
setup-wizard-verify-wallet = Wallet
setup-wizard-verify-rpc = Solana-RPC
setup-wizard-verify-save = Sichere Konfiguration
setup-wizard-complete-title = Einrichtung gespeichert
setup-wizard-reconnect = Verbindung erneut versuchen
setup-wizard-reload = Dashboard neu laden
setup-wizard-error-title = Die Einrichtung erfordert Ihre Aufmerksamkeit
setup-wizard-explore = Dashboard erkunden
setup-wizard-continue = Weiter
