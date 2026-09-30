# Results of system and configuration operations.

system-result-config-differs = Die Konfiguration im Arbeitsspeicher weicht von der Version auf dem Datenträger ab
system-result-config-matches = Die Konfiguration im Arbeitsspeicher stimmt mit der Version auf dem Datenträger überein

system-result-config-imported =
    { $count ->
        [one] { $count } Abschnitt erfolgreich importiert
       *[other] { $count } Abschnitte erfolgreich importiert
    }
system-result-config-imported-with-warnings =
    { $count ->
        [one] { $count } Abschnitt
       *[other] { $count } Abschnitte
    } importiert, { $warnings ->
        [one] { $warnings } Warnung
       *[other] { $warnings } Warnungen
    }: { $details }

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = Einstellungen durchsuchen...
system-config-export-title =
    .title = Konfiguration in Datei exportieren
system-config-import-title =
    .title = Konfiguration aus Datei importieren
system-config-reload = Vom Datenträger neu laden
system-config-reset-defaults = Auf Standard zurücksetzen
system-config-select-section = Konfigurationsabschnitt auswählen
system-config-select-section-details = Wählen Sie einen Konfigurationsabschnitt, um Details anzuzeigen.
system-config-no-metadata = Keine Metadaten für <code>{ $section }</code>
system-config-technical-settings = Technische Einstellungen
system-config-expand-title = Alle Abschnitte und alle verschachtelten Unterkonfigurationen aufklappen
system-config-collapse-title = Alle Abschnitte und alle verschachtelten Unterkonfigurationen zuklappen
system-config-toolbar-no-changes = Keine Änderungen im Abschnitt
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> Änderung im Abschnitt
       *[other] <strong>{ $count }</strong> Änderungen im Abschnitt
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> Änderung insgesamt
       *[other] <strong>{ $count }</strong> Änderungen insgesamt
    }

## Config page: state banner

system-config-loading = Konfiguration wird geladen…
system-config-refreshing = Konfiguration wird aktualisiert…
system-config-saving-title = Änderungen werden gespeichert…
system-config-saving-detail = Konfiguration wird aktualisiert
system-config-validation-issues = <strong>Validierungsprobleme erkannt.</strong> Bitte prüfen Sie die markierten Felder.

## Config page: section header and category chips

system-config-save-changes = Änderungen speichern
system-config-saving = Wird gespeichert…
system-config-compare = Mit Datenträger vergleichen
system-config-revert-section = Abschnitt zurücksetzen
system-config-summary-critical = { $count } kritisch
system-config-summary-performance = { $count } Performance
system-config-summary-pending =
    { $count ->
        [one] { $count } ausstehende Änderung
       *[other] { $count } ausstehende Änderungen
    }
system-config-summary-none = Keine Metadaten-Zusammenfassung
system-config-fields-count =
    { $count ->
        [one] { $count } Feld
       *[other] { $count } Felder
    }
system-config-chip-pending = { $fields } · { $pending } ausstehend
system-config-chip-visible = { $visible } von { $fields }

## Config page: field rows

system-config-field-unit = Einheit: { $unit }
system-config-field-default = Standard: { $value }
system-config-field-reset = Auf Standard zurücksetzen
system-config-array-invalid-title = Ungültiger Array-Eintrag
system-config-json-invalid-title = Ungültiges JSON
system-config-list-separator = { ", " }
system-config-array-invalid-integer =
    { $count ->
        [one] Zeile { $lines } muss eine gültige Ganzzahl sein.
       *[other] Zeilen { $lines } müssen gültige Ganzzahlen sein.
    }
system-config-array-invalid-number =
    { $count ->
        [one] Zeile { $lines } muss eine gültige Zahl sein.
       *[other] Zeilen { $lines } müssen gültige Zahlen sein.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] Zeile { $lines } muss ein gültiger Boolean sein.
       *[other] Zeilen { $lines } müssen gültige Booleans sein.
    }
system-config-array-invalid-value =
    { $count ->
        [one] Zeile { $lines } muss ein gültiger Wert sein.
       *[other] Zeilen { $lines } müssen gültige Werte sein.
    }

## Config page: Telegram actions

system-config-telegram-actions = Aktionen
system-config-telegram-test-title = Verbindung testen
system-config-telegram-test-description = Eine Testnachricht senden, um zu prüfen, ob Ihre { -telegram }-Konfiguration funktioniert
system-config-telegram-send-test = Testnachricht senden
system-config-telegram-sending = Wird gesendet...
system-config-telegram-configure-token-title = Zuerst Bot-Token konfigurieren
system-config-telegram-configure-token-status = Konfigurieren Sie oben den Bot-Token, um Tests zu aktivieren
system-config-telegram-test-sent-status = Testnachricht erfolgreich gesendet! Prüfen Sie Ihr { -telegram }.
system-config-telegram-test-sent = { -telegram }-Testnachricht gesendet
system-config-telegram-test-failed = Testnachricht konnte nicht gesendet werden
system-config-telegram-auth-title = Bot-Authentifizierung
system-config-telegram-totp-title = Zwei-Faktor-Authentifizierung (TOTP)
system-config-telegram-totp-configured = Konfiguriert
system-config-telegram-totp-not-configured = Nicht konfiguriert
system-config-telegram-totp-active = Die Zwei-Faktor-Authentifizierung ist aktiv. Abgelaufene { -telegram }-Sitzungen erfordern einen TOTP-Code aus Ihrer Authenticator-App.
system-config-telegram-totp-inactive = Aktivieren Sie die Zwei-Faktor-Authentifizierung in den Sicherheitseinstellungen, um { -telegram }-Befehle zu schützen.
system-config-telegram-totp-note = TOTP wird mit dem Dashboard-Sperrbildschirm geteilt. Konfigurieren Sie es in den Sicherheitseinstellungen.
system-config-telegram-require-2fa = 2FA für Befehle verlangen
system-config-telegram-save-rejected = Speichern abgelehnt ({ $status })
system-config-telegram-save-failed = { -telegram }-Einstellung konnte nicht gespeichert werden

## Config page: operations

system-config-saved = Konfiguration gespeichert
system-config-save-failed = Konfiguration konnte nicht gespeichert werden
system-config-reloaded = Konfiguration vom Datenträger neu geladen
system-config-reload-failed = Konfiguration konnte nicht neu geladen werden
system-config-diff-title = Konfigurationsunterschiede
system-config-diff-console = In die Browser-Konsole geschrieben
system-config-diff-failed = Unterschiede konnten nicht berechnet werden
system-config-reset-title = Konfiguration zurücksetzen
system-config-reset-message =
    Dadurch wird die gesamte Konfiguration auf die eingebetteten Standardwerte zurückgesetzt. Alle aktuellen Einstellungen gehen verloren.

    Diese Aktion kann nicht rückgängig gemacht werden.
system-config-reset-done-title = Konfiguration zurückgesetzt
system-config-reset-done-message = Alle Einstellungen auf Standardwerte zurückgesetzt
system-config-reset-failed = Konfiguration konnte nicht zurückgesetzt werden
system-config-load-failed = Konfiguration konnte nicht geladen werden
system-config-metadata-failed = Konfigurationsmetadaten konnten nicht geladen werden

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = Schließen
system-config-select-none = Keine auswählen
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } Änderung
       *[other] { $count } Änderungen
    }
system-config-sections-count =
    { $count ->
        [one] { $count } Abschnitt
       *[other] { $count } Abschnitte
    }

## Import and export dialogs: section descriptions.

system-config-section-hint-rpc = RPC-Endpunkte und Verbindungseinstellungen
system-config-section-hint-trader = Trading-Regeln und Automatisierung
system-config-section-hint-positions = Einstellungen zur Positionsverwaltung
system-config-section-hint-filtering = Token-Filterregeln und Schwellenwerte
system-config-section-hint-swaps = Einstellungen zur Swap-Ausführung
system-config-section-hint-tokens = Token-Erkennung und Datenquellen
system-config-section-hint-sol-price = Konfiguration des { -sol }-Preisdienstes
system-config-section-hint-events = Einstellungen zur Ereignisaufzeichnung
system-config-section-hint-services = Einstellungen für Hintergrunddienste
system-config-section-hint-monitoring = Konfiguration der Systemüberwachung
system-config-section-hint-ohlcv = Einstellungen für Kerzendaten
system-config-section-hint-gui = Dashboard- und Oberflächeneinstellungen
system-config-section-hint-telegram = { -telegram }-Bot-Konfiguration

## Export dialog

system-config-export-dialog-title = Konfiguration exportieren
system-config-export-intro = Wählen Sie, welche Konfigurationsabschnitte exportiert werden sollen. Die exportierte Datei kann später importiert werden, um Einstellungen wiederherzustellen oder zu teilen.
system-config-export-sections = Abschnitte
system-config-export-timestamp = Export-Zeitstempel einschließen
system-config-sections-selected =
    { $count ->
        [one] { $count } Abschnitt ausgewählt
       *[other] { $count } Abschnitte ausgewählt
    }
system-config-exporting = Wird exportiert...
system-config-export-invalid-response = Ungültige Antwort vom Server
system-config-exported-title = Konfiguration exportiert
system-config-exported-message =
    { $count ->
        [one] { $count } Abschnitt exportiert
       *[other] { $count } Abschnitte exportiert
    }
system-config-export-failed-title = Export fehlgeschlagen
system-config-export-failed = Konfiguration konnte nicht exportiert werden

## Import dialog

system-config-import-dialog-title = Konfiguration importieren
system-config-import-upload-intro = Laden Sie eine zuvor exportierte Konfigurationsdatei hoch. Sie können eine Vorschau ansehen und auswählen, welche Abschnitte importiert werden.
system-config-import-dropzone-title = Konfigurationsdatei hier ablegen
system-config-import-dropzone-hint = oder zum Durchsuchen klicken
system-config-import-analyzing = Konfiguration wird analysiert...
system-config-import-preview = Vorschau
system-config-import-preview-intro = Prüfen Sie die Konfigurationsabschnitte unten. Wählen Sie aus, welche Abschnitte importiert werden.
system-config-import-sections = Abschnitte in der Datei
system-config-import-select-valid = Alle gültigen auswählen
system-config-import-merge-label = Mit vorhandener zusammenführen
system-config-import-merge-hint = Nur in der Datei vorhandene Felder aktualisieren. Nicht angehakt = ganze Abschnitte ersetzen.
system-config-import-save-label = Auf Datenträger speichern
system-config-import-save-hint = Änderungen nach dem Import in config.toml sichern
system-config-import-selected = Auswahl importieren
system-config-import-warnings =
    { $count ->
        [one] { $count } Warnung
       *[other] { $count } Warnungen
    }
system-config-import-warning-unknown-section = Unbekannter Abschnitt „{ $section }“ wird ignoriert
system-config-import-warning-sensitive-field = Der Import von { $field } kann Authentifizierungseinstellungen überschreiben
system-config-import-section-error = { $detail }
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Nicht in der Datei
system-config-import-status-invalid = Ungültige Konfiguration
system-config-import-status-unchanged = Keine Änderungen
system-config-import-not-included = Nicht in der Datei enthalten
system-config-import-show-changes = Änderungen anzeigen
system-config-import-hide-changes = Änderungen ausblenden
system-config-import-value-current = Aktueller Wert
system-config-import-value-new = Neuer Wert
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } weitere Änderung
       *[other] +{ $count } weitere Änderungen
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } Element
       *[other] { $count } Elemente
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } Schlüssel
       *[other] { $count } Schlüssel
    }{ "}" }
system-config-importing = Wird importiert...
system-config-import-failed = Import fehlgeschlagen
system-config-import-invalid-file-title = Ungültige Datei
system-config-import-invalid-file = Konfigurationsdatei konnte nicht gelesen werden
system-config-imported-title = Konfiguration importiert
system-config-imported-message =
    { $count ->
        [one] { $count } Abschnitt importiert
       *[other] { $count } Abschnitte importiert
    }
system-config-import-failed-title = Import fehlgeschlagen
system-config-import-failed-message = Konfiguration konnte nicht importiert werden
