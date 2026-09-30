# Wallet page labels.

wallets-type-generated = Generiert
wallets-type-imported = Importiert
wallets-type-migrated = Migriert

wallets-watch-disabled-user = Von Ihnen pausiert
wallets-watch-disabled-signature-budget = Pausiert: Das Prüflimit von { $limit } Signaturen wurde erreicht, bevor der Rückstand aufgeholt war
wallets-watch-disabled-unknown = Pausiert: Der gespeicherte Sicherheitsgrund der Überwachung konnte nicht gelesen werden
wallets-watch-disabled-helius-unavailable = Pausiert: Der Anbieter für hohe Aktivität ist nicht verfügbar; Cursor bleibt erhalten
wallets-watch-disabled-processing-failed = Pausiert: Wallet-Aktivität konnte nicht verarbeitet werden; Cursor bleibt erhalten

wallets-watch-error-provider-unavailable = Anbieter für hohe Aktivität ist nicht verfügbar; Überwachung pausiert
wallets-watch-error-provider-repeated-failure = { -helius }-Prüfungen sind wiederholt fehlgeschlagen; Überwachung pausiert
wallets-watch-error-processing-repeated-failure = Die Verarbeitung der Wallet-Aktivität ist wiederholt fehlgeschlagen; Überwachung pausiert
wallets-watch-error-position-unreadable = Die Wallet-Überwachung konnte ihre gespeicherte Position nicht lesen; neuer Versuch läuft
wallets-watch-error-provider-check-failed = Prüfung beim Anbieter für hohe Aktivität fehlgeschlagen; neuer Versuch läuft
wallets-watch-error-decode-failed = Transaktion mit hoher Aktivität konnte nicht dekodiert werden; Cursor bleibt erhalten
wallets-watch-error-processing-failed = Wallet-Aktivität konnte nicht verarbeitet werden; neuer Versuch läuft
wallets-watch-error-position-save-failed = Die Wallet-Überwachung konnte ihre Position nicht speichern; neuer Versuch läuft

wallets-watch-reason-user = Von Ihnen pausiert.
wallets-watch-reason-signature-budget = Diese Wallet hat mehr Aktivität, als die aktuelle Überwachung prüfen kann.
wallets-watch-reason-helius-unavailable = { -helius }-Prüfungen sind fehlgeschlagen. Der gespeicherte Fortschritt bleibt erhalten.
wallets-watch-reason-processing-failed = Wallet-Aktivität konnte nicht verarbeitet werden. Der gespeicherte Fortschritt bleibt erhalten.

wallets-field-address = Adresse
wallets-field-name = Wallet-Name
wallets-field-notes = Notizen
wallets-field-private-key = Privater Schlüssel
wallets-address-copy = Adresse kopieren
wallets-modal-close =
    .aria-label = Dialog schließen
wallets-this-wallet = diese Wallet
wallets-summary-sol = { -sol }
wallets-copied-address = Adresse
wallets-copied-mint = Mint-Adresse
wallets-copied-private-key = Privater Schlüssel

wallets-tab-main = Haupt-Wallet
wallets-tab-secondaries = Sekundäre
wallets-tab-archive = Archiv
wallets-tab-watched = Überwacht
wallets-refresh-failed = Wallets konnten nicht aktualisiert werden
wallets-action-failed = Fehlgeschlagen
wallets-toast-failed = Fehlgeschlagen: { $reason }
wallets-create-busy = Wird erstellt...
wallets-create-fallback = Erstellung fehlgeschlagen
wallets-create-done = Wallet „{ $name }“ erstellt!
wallets-import-busy = Wird importiert...
wallets-import-failed = Import fehlgeschlagen
wallets-import-done = Wallet „{ $name }“ importiert!
wallets-archive-busy = Wird archiviert...
wallets-archive-confirm-text = Möchten Sie <strong>{ $name }</strong> wirklich archivieren?
wallets-archive-done = Wallet archiviert
wallets-restore-done = Wallet wiederhergestellt
wallets-export-busy = Wird entschlüsselt...
wallets-export-revealed = Schlüssel angezeigt – bitte sorgfältig behandeln
wallets-delete-busy = Wird gelöscht...
wallets-delete-confirm-text = Möchten Sie <strong>{ $name }</strong> wirklich löschen?
wallets-delete-done = Wallet endgültig gelöscht

wallets-add-title = Wallet hinzufügen
wallets-add-tab-create = Neu erstellen
wallets-add-tab-import = Vorhandene importieren
wallets-create-name-input =
    .placeholder = z. B. Trading-Wallet
wallets-create-name-hint = Ein einprägsamer Name zur Identifizierung dieser Wallet
wallets-create-notes-input =
    .placeholder = Optionale Beschreibung oder Zweck...
wallets-create-submit = Wallet erstellen
wallets-import-warning-title = Sicherheitswarnung
wallets-import-warning-body = Importieren Sie private Schlüssel nur aus vertrauenswürdigen Quellen. Ihr Schlüssel wird verschlüsselt und sicher auf diesem Gerät gespeichert.
wallets-import-name-input =
    .placeholder = z. B. Meine Wallet
wallets-import-key-input =
    .placeholder = Base58-String oder JSON-Array [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Sichtbarkeit des privaten Schlüssels umschalten
wallets-import-key-hint = Unterstützt Base58-kodierte Schlüssel oder das Byte-Array-Format
wallets-import-notes-input =
    .placeholder = Optionale Beschreibung...
wallets-import-submit = Wallet importieren

wallets-watch-add-title = Wallet überwachen
wallets-watch-add-address = Wallet-Adresse
wallets-watch-add-address-input =
    .placeholder = Solana-Adresse
wallets-watch-add-address-hint = Zeichnet die On-Chain-Aktivität der Wallet auf und sendet Trade-Benachrichtigungen über Ihre { -telegram }-Einstellungen.
wallets-watch-add-label = Bezeichnung
wallets-watch-add-label-input =
    .placeholder = Optionaler Name
wallets-watch-add-submit = Überwachung hinzufügen

wallets-watch-budget-title-options = Optionen der Wallet-Überwachung
wallets-watch-budget-title-restore = Wallet-Überwachung wiederherstellen
wallets-watch-budget-close =
    .aria-label = Schließen
wallets-watch-budget-label-signatures = Geprüfte Signaturen pro Prüfung
wallets-watch-budget-label-transactions = Geprüfte erfolgreiche vollständige Transaktionen pro Prüfung
wallets-watch-budget-hint-signatures = Aktuelles Limit: { $limit }. Wählen Sie 500–5.000 Signaturen pro Prüfung in Schritten von 100.
wallets-watch-budget-hint-transactions = Aktuelles Limit: { $limit }. Wählen Sie 500–5.000 erfolgreiche Transaktionen pro Prüfung in Schritten von 100.
wallets-watch-budget-error-range = Wählen Sie zwischen 500 und 5.000 Datensätzen pro Prüfung in Schritten von 100.
wallets-watch-budget-error-ack = Bestätigen Sie, dass Signaturen seit der letzten abgeschlossenen Prüfung übersprungen werden.
wallets-watch-budget-save-failed = Überwachungslimit konnte nicht gespeichert werden.
wallets-watch-budget-save = Limit speichern
wallets-watch-budget-resume = Ab jetzt fortsetzen
wallets-watch-budget-resume-notice = Diese Wallet hat ihr Prüflimit erreicht, bevor der Rückstand aufgeholt war. „Ab jetzt fortsetzen“ beginnt bei der neuesten Wallet-Aktivität; Aktivität seit der letzten abgeschlossenen Prüfung wird nicht kopiert.
wallets-watch-budget-resume-tasks = Copy-Aufgaben bleiben pausiert, bis Sie jede Aufgabe im Copy-Trading fortsetzen.
wallets-watch-budget-resume-ack = Ich verstehe, dass verpasste Aktivität nicht kopiert wird.
wallets-watch-budget-resumed = Überwachung ab dem aktuellen Wallet-Stand fortgesetzt
wallets-watch-budget-updated = Limit der Wallet-Überwachung aktualisiert
wallets-watch-helius-allow = { -helius }-Aufholen bei Bedarf erlauben
wallets-watch-helius-try = Aufholen über { -helius } versuchen
wallets-watch-helius-stop = { -helius }-Aufholen für diese Wallet beenden
wallets-watch-helius-description-approved = { -helius }-Aufholen ist für diese Wallet erlaubt. Wenn Sie es ausschalten, wird wieder mit Standardprüfungen gearbeitet, die bei einer stark genutzten Wallet zurückfallen können.
wallets-watch-helius-description-available = { -helius } kann erfolgreiche Solana-Transaktionen ab der gespeicherten Position prüfen, ohne das ungeprüfte Intervall zu überspringen. Das kann mehr Anbieter-Credits verbrauchen und dennoch zurückfallen.
wallets-watch-helius-description-unavailable = { -helius }-Aufholen ist nicht verfügbar. Konfigurieren Sie einen aktivierten { -helius }-RPC-Endpunkt, um es zu nutzen.
wallets-watch-helius-description-unsupported = Für diese Überwachung wird kein Anbieter zum Aufholen unterstützt. „Ab jetzt fortsetzen“ ist verfügbar, wenn die Überwachung ihr Limit erreicht.
wallets-watch-helius-allow-title = { -helius }-Aufholen für diese Wallet erlauben
wallets-watch-helius-allow-message = { -helius } kann erfolgreiche Solana-Transaktionen ab der gespeicherten Position prüfen, ohne das ungeprüfte Intervall zu überspringen. Derzeit werden 10 Credits pro 100 zurückgegebene vollständige Transaktionen berechnet, aufgerundet, mit mindestens 10 Credits pro Anfrage. Eine Prüfung kann mehrere Anfragen auslösen; Verbrauch und Anbieterpreise können abweichen. Copy-Aufgaben bleiben pausiert, bis sie separat fortgesetzt werden.
wallets-watch-helius-allow-confirm = Für diese Wallet erlauben
wallets-watch-helius-stop-message = Diese Wallet kehrt zu Standardprüfungen zurück. Eine stark genutzte Wallet kann ihr Überwachungslimit erreichen und erneut pausieren. Andere Wallets und Ihre { -helius }-RPC-Konfiguration bleiben unverändert.
wallets-watch-helius-stop-confirm = Für diese Wallet beenden
wallets-watch-helius-stop-keep = Erlaubt lassen
wallets-watch-helius-restored = Überwachung aus gespeichertem Fortschritt wiederhergestellt; Copy-Aufgaben bleiben pausiert
wallets-watch-helius-allowed = { -helius }-Aufholen für diese Wallet bei Bedarf erlaubt
wallets-watch-helius-stopped = { -helius }-Aufholen für diese Wallet beendet
wallets-watch-helius-update-failed = Einstellung zum Aufholen der Wallet konnte nicht aktualisiert werden

wallets-export-title = Privaten Schlüssel exportieren
wallets-export-warning-title = Kritische Sicherheitswarnung
wallets-export-warning-body = Geben Sie Ihren privaten Schlüssel niemals an Dritte weiter. Jeder mit Zugriff auf diesen Schlüssel kann alle Gelder dieser Wallet stehlen.
wallets-export-key-label = Privater Schlüssel (Base58)
wallets-export-copy =
    .title = In die Zwischenablage kopieren
    .aria-label = In die Zwischenablage kopieren
wallets-export-reveal = Schlüssel anzeigen

wallets-archive-title = Wallet archivieren
wallets-archive-note = Archivierte Wallets werden in keinem Vorgang verwendet, können aber jederzeit wiederhergestellt werden.
wallets-archive-confirm = Ja, archivieren
wallets-delete-title = Wallet löschen
wallets-delete-warning-title = Diese Aktion kann nicht rückgängig gemacht werden!
wallets-delete-warning-body = Beim Löschen werden diese Wallet und ihr verschlüsselter privater Schlüssel endgültig von diesem Gerät entfernt.
wallets-delete-confirm = Ja, löschen

wallets-bulk-import-title = Wallets importieren
wallets-bulk-import-submit = Wallets importieren
wallets-bulk-step-upload = Datei hochladen
wallets-bulk-step-map = Spalten zuordnen
wallets-bulk-step-results = Ergebnisse
wallets-bulk-import-file-warning-body = Importieren Sie Dateien nur aus vertrauenswürdigen Quellen. Private Schlüssel werden verschlüsselt und sicher auf diesem Gerät gespeichert.
wallets-bulk-drop-title = Datei hier ablegen
wallets-bulk-drop-subtitle = oder zum Durchsuchen klicken
wallets-bulk-drop-formats = Unterstützt CSV und Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Datei entfernen
wallets-bulk-map-subtitle = Ordnen Sie die Spalten Ihrer Datei den Wallet-Feldern zu
wallets-bulk-preview-title = Vorschau (erste 5 Zeilen)
wallets-bulk-summary-valid = <strong>{ $count }</strong> gültig
wallets-bulk-summary-invalid = <strong>{ $count }</strong> ungültig
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> Duplikat
       *[other] <strong>{ $count }</strong> Duplikate
    }
wallets-bulk-done = Fertig
wallets-bulk-file-invalid = Ungültiger Dateityp. Bitte verwenden Sie CSV- oder Excel-Dateien.
wallets-bulk-preview-busy = Wird verarbeitet...
wallets-bulk-preview-fallback = Datei konnte nicht verarbeitet werden
wallets-bulk-preview-failed = Datei konnte nicht verarbeitet werden: { $reason }
wallets-bulk-column-select = -- Spalte auswählen --
wallets-bulk-preview-empty = Keine Datenzeilen in der Datei gefunden
wallets-bulk-preview-status = Status
wallets-bulk-status-valid = Gültig
wallets-bulk-status-duplicate = Duplikat
wallets-bulk-status-invalid = Ungültig
wallets-bulk-import-busy = Wird importiert...
wallets-bulk-import-toast =
    { $count ->
        [one] { $count } Wallet importiert
       *[other] { $count } Wallets importiert
    }
wallets-bulk-import-error = Import fehlgeschlagen: { $reason }
wallets-bulk-result-success-title = Import erfolgreich
wallets-bulk-result-success-detail =
    { $count ->
        [one] Die { $count } Wallet wurde erfolgreich importiert
       *[other] Alle { $count } Wallets wurden erfolgreich importiert
    }
wallets-bulk-result-partial-title = Teilweise erfolgreich
wallets-bulk-result-partial-detail = { $imported } importiert, { $failed } fehlgeschlagen
wallets-bulk-result-failed-title = Import fehlgeschlagen
wallets-bulk-result-failed-detail =
    { $count ->
        [one] Der Import der { $count } Wallet ist fehlgeschlagen
       *[other] Der Import aller { $count } Wallets ist fehlgeschlagen
    }
wallets-bulk-result-imported = Importiert
wallets-bulk-result-failed = Fehlgeschlagen

wallets-bulk-export-title = Wallets exportieren
wallets-bulk-export-format = Format
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Archivierte Wallets einschließen
wallets-bulk-export-safe-title = Sicherer Export
wallets-bulk-export-safe-body = Exportiert nur Wallet-Adressen und Metadaten. Keine privaten Schlüssel enthalten.
wallets-bulk-export-safe-submit = Adressen exportieren
wallets-bulk-export-or = oder
wallets-bulk-export-danger-title = Gefährlicher Export
wallets-bulk-export-danger-body = Private Schlüssel in den Export aufnehmen. Jeder mit dieser Datei kann Ihre Gelder stehlen.
wallets-bulk-export-danger-submit = Mit privaten Schlüsseln exportieren
wallets-bulk-export-busy = Wird exportiert...
wallets-bulk-export-done = Wallets nach { $filename } exportiert
wallets-bulk-export-fallback = Export fehlgeschlagen
wallets-bulk-export-error = Export fehlgeschlagen: { $reason }
wallets-bulk-confirm-title = Gefährlichen Export bestätigen
wallets-bulk-confirm-warning =
    { $count ->
        [one] Sie sind dabei, <strong>{ $count }</strong> privaten Schlüssel zu exportieren. Das ist äußerst gefährlich!
       *[other] Sie sind dabei, <strong>{ $count }</strong> private Schlüssel zu exportieren. Das ist äußerst gefährlich!
    }
wallets-bulk-confirm-risk-steal = Jeder mit dieser Datei kann alle Gelder stehlen
wallets-bulk-confirm-risk-share = Geben Sie diese Datei niemals an Dritte weiter
wallets-bulk-confirm-risk-delete = Löschen Sie die Datei sofort nach der Verwendung
wallets-bulk-confirm-prompt = Geben Sie zur Bestätigung die untenstehende Phrase ein
wallets-bulk-confirm-submit = Schlüssel exportieren

wallets-holdings-col-token = Token
wallets-holdings-col-balance = Guthaben
wallets-holdings-col-value = Wert ({ -sol })
wallets-holdings-col-type = Typ
wallets-holdings-col-decimals = Dezimalstellen
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Keine Token-Bestände
wallets-holdings-empty-message = Von dieser Wallet gehaltene Token erscheinen hier.
wallets-holdings-no-main = Keine Haupt-Wallet
wallets-holdings-main-tag = Haupt
wallets-holdings-main-title = Haupt-Wallet
wallets-holdings-tokens = Token
wallets-holdings-last-used = Zuletzt verwendet
wallets-holdings-never = Nie
wallets-holdings-search =
    .placeholder = Nach Symbol oder Mint suchen...
wallets-holdings-export = Schlüssel exportieren
wallets-holdings-export-tooltip = Privaten Schlüssel dieser Wallet exportieren
wallets-list-col-name = Name
wallets-list-col-balance = Guthaben ({ -sol })
wallets-list-col-type = Typ
wallets-list-col-created = Erstellt
wallets-list-col-actions = Aktionen
wallets-list-action-export = Privaten Schlüssel exportieren
wallets-list-action-archive = Wallet archivieren
wallets-list-action-restore = Wallet wiederherstellen
wallets-list-action-delete = Endgültig löschen
wallets-list-count = Wallets
wallets-list-search =
    .placeholder = Nach Name oder Adresse suchen...
wallets-list-loading-title = Wallets werden geladen…
wallets-list-loading-description = Die ausgewählte Wallet-Ansicht wird vorbereitet.
wallets-secondaries-empty-title = Keine sekundären Wallets
wallets-secondaries-empty-message = Erstellen Sie weitere Wallets, um Ihre Trading-Aktivitäten auf mehrere Konten zu verteilen.
wallets-secondaries-add = Wallet hinzufügen
wallets-archive-empty-title = Keine archivierten Wallets
wallets-archive-empty-message = Von Ihnen archivierte Wallets werden hier sicher zur späteren Verwendung aufbewahrt.

wallets-watched-col-wallet = Wallet
wallets-watched-col-status = Status
wallets-watched-col-progress = Gespeicherter Fortschritt
wallets-watched-col-last-check = Letzte Prüfung
wallets-watched-unlabelled = Unbenannte Wallet
wallets-watched-generic-name = Wallet
wallets-watched-not-synced = Noch nicht synchronisiert
wallets-watched-not-checked = Noch nicht geprüft
wallets-watched-action-copy = Trade kopieren
    .title = Diese Wallet im Copy-Trading öffnen
wallets-watched-action-restore = Überwachung wiederherstellen
wallets-watched-action-options = Überwachungsoptionen
wallets-watched-action-retry = Überwachung wiederholen
wallets-watched-action-pause = Pausieren
wallets-watched-action-enable = Aktivieren
wallets-watched-action-remove =
    .title = Entfernen
    .aria-label = { $name } entfernen
wallets-watch-state-paused = Pausiert
wallets-watch-state-catching-up = Holt auf
wallets-watch-state-watching = Überwacht
wallets-watch-state-streaming = Streaming
wallets-watch-state-polling = Polling
wallets-watched-detail-helius = Prüfung für diese Wallet über { -helius }.
wallets-watched-empty-title = Keine überwachten Adressen
wallets-watched-empty-message = Mit „Wallet überwachen“ zeichnen Sie die On-Chain-Aktivität einer öffentlichen Wallet auf.
wallets-watched-count = Überwacht
wallets-watched-search =
    .placeholder = Überwachte Wallets durchsuchen...
wallets-watched-add = Wallet überwachen
wallets-watched-refresh = Überwachte Wallets aktualisieren
wallets-watched-loading-title = Überwachte Wallets werden geladen...
wallets-watched-loading-description = Beobachtungsziele werden abgerufen.
wallets-watched-load-error-title = Überwachte Adressen konnten nicht geladen werden
wallets-watched-load-error-description = Versuchen Sie es mit Aktualisieren erneut.
wallets-watched-address-invalid = Geben Sie eine gültige Solana-Wallet-Adresse ein.
wallets-watched-added = Wallet-Überwachung hinzugefügt
wallets-watched-duplicate = Diese Wallet wird bereits überwacht.
wallets-watched-add-failed = Wallet-Überwachung konnte nicht hinzugefügt werden.
wallets-watched-retried = Wallet-Überwachung mit gespeichertem Cursor wiederhergestellt
wallets-watched-paused = Wallet-Überwachung pausiert
wallets-watched-enabled = Wallet-Überwachung aktiviert
wallets-watched-removed = Wallet-Überwachung entfernt
wallets-watched-update-failed = Wallet-Überwachung konnte nicht aktualisiert werden
