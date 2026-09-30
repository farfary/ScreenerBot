## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } Minute
       *[other] { $count } Minuten
    }
settings-duration-hours =
    { $count ->
        [one] { $count } Stunde
       *[other] { $count } Stunden
    }

## settings_dialog.js

settings-dialog-title = Einstellungen
settings-dialog-close =
    .title = Schließen (ESC)
    .aria-label = Einstellungen schließen
settings-dialog-save = Änderungen speichern
settings-dialog-saving = Wird gespeichert...
settings-dialog-saved = Gespeichert
settings-dialog-save-success = Einstellungen erfolgreich gespeichert
settings-dialog-save-failed = Einstellungen konnten nicht gespeichert werden
settings-dialog-update-attention = Update erfordert Aufmerksamkeit
settings-dialog-tab-interface = Oberfläche
settings-dialog-tab-navigation = Navigation
settings-dialog-tab-startup = Start
settings-dialog-tab-hints = Hinweise
settings-dialog-tab-data = Daten
settings-dialog-tab-security = Sicherheit
settings-dialog-tab-account = Konto
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Agent-Verbindungen
settings-dialog-tab-updates = Updates
settings-dialog-tab-licenses = Lizenzen
settings-dialog-tab-about = Info
settings-dialog-link-privacy = Datenschutzerklärung
settings-dialog-link-terms = Nutzungsbedingungen

## settings_dialog.js: Startup tab

settings-startup-section-title = Startverhalten
settings-startup-auto-start-label = Trader automatisch starten
settings-startup-auto-start-hint = Trader beim Start automatisch starten
settings-startup-coming-soon = Demnächst
settings-startup-default-page-label = Startseite
settings-startup-default-page-hint = Seite, die beim Öffnen der App angezeigt wird
settings-startup-page-dashboard = Dashboard
settings-startup-page-tokens = Tokens
settings-startup-page-positions = Positionen
settings-startup-page-wallet = Wallet
settings-startup-page-config = Konfiguration
settings-startup-notifications-label = Hintergrund-Benachrichtigungen anzeigen
settings-startup-notifications-hint = Benachrichtigungen für Hintergrundereignisse anzeigen

## settings_dialog.js: About tab

settings-about-logo =
    .alt = { -brand }
settings-about-tagline = Native Solana-Trading-Engine
settings-about-link-github = { -github }
settings-about-link-docs = Dokumentation
settings-about-link-telegram = { -telegram }
settings-about-link-website = Website
settings-about-credits = Entwickelt für Solana-Trader
settings-about-copyright = © { $year } { -brand }. Alle Rechte vorbehalten.

## interface_tab.js

settings-interface-section-appearance = Darstellung
settings-interface-theme-label = Design
settings-interface-theme-hint = Wählen Sie Ihr bevorzugtes Farbschema
settings-interface-theme-dark = Dunkel
settings-interface-theme-light = Hell
settings-interface-language-label = Sprache
settings-interface-language-hint = Anzeigesprache des Dashboards
settings-interface-logo-shape-label = Form der Token-Logos
settings-interface-logo-shape-hint = „Kreis“ schneidet jedes Logo rund zu; „Natürlich“ behält die Silhouette der jeweiligen Grafik bei
settings-interface-logo-shape-circle = Kreis
settings-interface-logo-shape-natural = Natürlich
settings-interface-animations-label = Animationen aktivieren
settings-interface-animations-hint = Sanfte Übergänge und Effekte
settings-interface-compact-label = Kompaktmodus
settings-interface-compact-hint = Weniger Abstände für mehr Inhalt
settings-interface-section-data = Daten und Anzeige
settings-interface-refresh-label = Aktualisierungsintervall
settings-interface-refresh-hint = Wie oft die Daten aktualisiert werden
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } Sekunde
       *[other] { $count } Sekunden
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } Minute
       *[other] { $count } Minuten
    }
settings-interface-ticker-label = Ticker-Leiste anzeigen
settings-interface-ticker-hint = Live-Kennzahlen-Ticker in der Kopfzeile
settings-interface-page-size-label = Zeilen pro Tabellenseite
settings-interface-page-size-hint = Standardanzahl der Zeilen pro Tabellenseite
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } Zeile
       *[other] { $count } Zeilen
    }
settings-interface-auto-expand-label = Kategorien automatisch ausklappen
settings-interface-auto-expand-hint = Konfigurationskategorien standardmäßig ausklappen
settings-interface-hints-label = Kontexthinweise anzeigen
settings-interface-hints-hint = Hilfe-Symbole anzeigen, die Dashboard-Funktionen erklären
settings-interface-featured-label = Empfohlene Zeile anzeigen
settings-interface-featured-hint = Zeile mit empfohlenen Tokens auf den Seiten Start und Tokens anzeigen
settings-interface-section-sound = Soundeffekte
settings-interface-sounds-label = Sounds aktivieren
settings-interface-sounds-hint = Akustische Signale für Navigation, Statuswechsel und Ergebnisse

## security_tab.js

settings-security-loading = Sicherheitseinstellungen werden geladen...
settings-security-load-failed = Sicherheitseinstellungen konnten nicht geladen werden

settings-security-type-pin4 = 4-stellige PIN
settings-security-type-pin6 = 6-stellige PIN
settings-security-type-text = Text-Passwort
settings-security-type-unset = Nicht festgelegt

settings-security-lockscreen-title = Dashboard-Sperrbildschirm
settings-security-lockscreen-description = Schützen Sie Ihr Dashboard mit einer PIN oder einem Passwort. Der Sperrbildschirm erscheint bei Auslösung und erfordert eine Authentifizierung.
settings-security-enable-label = Sperrbildschirm aktivieren
settings-security-enable-hint = Dashboard per Passwort schützen
settings-security-password-status-label = Passwortstatus
settings-security-password-current = Aktuell: { $type }
settings-security-password-none = Kein Passwort festgelegt
settings-security-change = Ändern
settings-security-remove = Entfernen
settings-security-set-password = Passwort festlegen
settings-security-auto-lock-label = Automatisch sperren bei Inaktivität
settings-security-auto-lock-hint = Nach einer Zeit ohne Aktivität automatisch sperren
settings-security-auto-lock-never = Nie
settings-security-lock-blur-label = Sperren, wenn das Fenster den Fokus verliert
settings-security-lock-blur-hint = Automatisch sperren, wenn Sie zu einer anderen Anwendung wechseln
settings-security-quick-actions-title = Schnellaktionen
settings-security-lock-now-label = Dashboard jetzt sperren
settings-security-lock-now-hint = Dashboard sofort sperren
settings-security-lock-now = Jetzt sperren
settings-security-lock-not-ready = Sperren nicht möglich – Sperrbildschirm nicht bereit
settings-security-setting-save-failed = Sicherheitseinstellung konnte nicht gespeichert werden

## security_tab.js: two-factor authentication

settings-security-2fa-title = Zwei-Faktor-Authentifizierung
settings-security-2fa-description = Zusätzliche Sicherheitsebene mit einer Authenticator-App (Google Authenticator, Authy usw.)
settings-security-2fa-status-label = 2FA-Status
settings-security-2fa-status-enabled = Zwei-Faktor-Authentifizierung ist aktiviert
settings-security-2fa-status-none = Nicht eingerichtet
settings-security-2fa-disable = 2FA deaktivieren
settings-security-2fa-enable = 2FA aktivieren

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Schließen
settings-security-password-set-title = Passwort festlegen
settings-security-password-change-title = Passwort ändern
settings-security-password-current-label = Aktuelles Passwort
settings-security-password-current-input =
    .placeholder = Aktuelles Passwort eingeben
settings-security-password-type-label = Passworttyp
settings-security-password-new-label = Neues Passwort
settings-security-password-new-input =
    .placeholder = Neues Passwort eingeben
settings-security-password-confirm-label = Passwort bestätigen
settings-security-password-confirm-input =
    .placeholder = Passwort bestätigen
settings-security-password-update = Passwort aktualisieren
settings-security-placeholder-pin4 = 4-stellige PIN eingeben
settings-security-placeholder-pin6 = 6-stellige PIN eingeben
settings-security-placeholder-text = Passwort eingeben
settings-security-password-required = Bitte geben Sie ein Passwort ein
settings-security-password-mismatch = Die Passwörter stimmen nicht überein
settings-security-pin4-invalid = Die PIN muss genau 4 Ziffern haben
settings-security-pin6-invalid = Die PIN muss genau 6 Ziffern haben
settings-security-text-too-short = Das Passwort muss mindestens 4 Zeichen lang sein
settings-security-password-saved = Passwort gespeichert
settings-security-password-save-failed = Passwort konnte nicht gespeichert werden
settings-security-password-save-failed-detail = Passwort konnte nicht gespeichert werden: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Passwort entfernen
settings-security-remove-description = Geben Sie Ihr aktuelles Passwort ein, um den Sperrbildschirm-Schutz zu entfernen.
settings-security-remove-confirm = Passwort entfernen
settings-security-current-required = Bitte geben Sie Ihr aktuelles Passwort ein
settings-security-password-removed = Passwort entfernt
settings-security-password-remove-failed = Passwort konnte nicht entfernt werden
settings-security-password-remove-failed-detail = Passwort konnte nicht entfernt werden: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Zwei-Faktor-Authentifizierung aktivieren
settings-security-2fa-password-prompt = Geben Sie Ihr Passwort ein, um fortzufahren:
settings-security-2fa-password-input =
    .placeholder = Passwort eingeben
settings-security-2fa-continue = Fortfahren
settings-security-2fa-manual-code = Code für die manuelle Eingabe:
settings-security-2fa-qr =
    .alt = TOTP-QR-Code
settings-security-2fa-code-prompt = Geben Sie den 6-stelligen Code aus Ihrer Authenticator-App ein:
settings-security-2fa-verify-enable = Bestätigen und aktivieren
settings-security-2fa-password-required = Bitte geben Sie Ihr Passwort ein
settings-security-2fa-setup-failed = 2FA konnte nicht eingerichtet werden
settings-security-2fa-code-invalid-length = Bitte geben Sie einen 6-stelligen Code ein
settings-security-2fa-code-invalid = Ungültiger Code
settings-security-2fa-enabled = Zwei-Faktor-Authentifizierung aktiviert
settings-security-2fa-verify-failed = Code konnte nicht überprüft werden
settings-security-2fa-disable-title = Zwei-Faktor-Authentifizierung deaktivieren
settings-security-2fa-disable-prompt = Geben Sie Ihr Passwort ein, um 2FA zu deaktivieren:
settings-security-2fa-disable-failed = 2FA konnte nicht deaktiviert werden
settings-security-2fa-disabled = Zwei-Faktor-Authentifizierung deaktiviert

## agent_connections_tab.js

settings-agent-category-analysis = Analyse
settings-agent-category-portfolio = Portfolio
settings-agent-category-trading = Trading
settings-agent-category-config = Konfiguration
settings-agent-category-system = System
settings-agent-category-analysis-description = Token-Analyse, Marktdaten und Sicherheitsprüfungen.
settings-agent-category-portfolio-description = Offene Positionen, Guthaben und GuV.
settings-agent-category-trading-description = Kaufen, Verkaufen und Schließen von Positionen mit echten Geldmitteln.
settings-agent-category-config-description = Alle Bot-Einstellungen, einschließlich RPC-Endpunkten. Niemals Wallet-Schlüssel.
settings-agent-category-system-description = Status, Ereignisse und der Not-Stopp.
settings-agent-category-analysis-inline = Analyse
settings-agent-category-portfolio-inline = Portfolio
settings-agent-category-trading-inline = Trading
settings-agent-category-config-inline = Konfiguration
settings-agent-category-system-inline = System

settings-agent-level-allow = Erlauben
settings-agent-level-ask-user = Fragen
settings-agent-level-deny = Aus
settings-agent-level-allow-hint = Wird sofort ausgeführt.
settings-agent-level-ask-user-hint = Wartet auf Ihre Freigabe in der App.
settings-agent-level-deny-hint = Abgelehnt und für den Agenten unsichtbar.

settings-agent-preset-full = Voller Zugriff
settings-agent-preset-ask = Erst fragen
settings-agent-preset-read = Nur lesen
settings-agent-preset-full-description = Alles wird ohne Rückfrage ausgeführt. Wallet-Schlüssel bleiben unzugänglich.
settings-agent-preset-ask-description = Jede Aktion wartet auf Ihre Freigabe in der App.
settings-agent-preset-read-description = Analyse- und Portfolio-Lesezugriffe. Nichts kann geändert werden.
settings-agent-preset-custom = Benutzerdefiniert
settings-agent-preset-group =
    .aria-label = Berechtigungsvorlage
settings-agent-permission-group = Berechtigung: { $category }

settings-agent-summary-asks-only = Eingeschränkt – fragt bei { $asking }
settings-agent-summary-off-only = Eingeschränkt – kein Zugriff auf { $off }
settings-agent-summary-asks-and-off = Eingeschränkt – fragt bei { $asking }; kein Zugriff auf { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = Generisches stdio-MCP

settings-agent-note-placeholder = Ersetzen Sie /absolute/path/to/screenerbot durch den absoluten Pfad zu Ihrer { -brand }-Binärdatei – die laufende App konnte ihren Pfad auf diesem System nicht darstellen.
settings-agent-note-data-dir = Wenn Sie { -brand } mit einem nicht standardmäßigen Datenverzeichnis ausführen, setzen Sie im Client zusätzlich SCREENERBOT_DATA_DIR auf denselben Pfad (ein weiteres -e / --env-Flag oder ein env-Eintrag).
settings-agent-note-codex-run = Führen Sie den Befehl aus oder fügen Sie den TOML-Block zu ~/.codex/config.toml ($CODEX_HOME/config.toml) hinzu. Starten Sie { -codex } anschließend neu.
settings-agent-note-codex-get = `codex mcp get screenerbot` maskiert das Secret in der Ausgabe.
settings-agent-note-claude-code = { -claude } Code: Führen Sie den Befehl aus und starten Sie { -claude } Code neu. `claude mcp get screenerbot` gibt die konfigurierte Umgebung einschließlich des Secrets aus.
settings-agent-note-claude-desktop = { -claude } Desktop: Fügen Sie das JSON in claude_desktop_config.json unter `mcpServers` ein und starten Sie die App neu.
settings-agent-note-openclaw = Führen Sie den Befehl aus und prüfen Sie dann mit `openclaw mcp doctor screenerbot --probe`, ob der gespeicherte stdio-Server startet und Tools bereitstellt.
settings-agent-note-hermes = Fügen Sie dies unter `mcp_servers` in der Konfigurationsdatei von { -hermes } ein und starten Sie { -hermes } neu.
settings-agent-note-generic = Jeder MCP-Client, der stdio unterstützt: Führen Sie diesen Befehl mit diesen Argumenten und dieser Umgebung aus, dort wo der Client seine Serverliste verwaltet.
settings-agent-block-codex-command = { -codex } CLI – Terminal-Befehl
settings-agent-block-codex-toml = { -codex } CLI – ~/.codex/config.toml (Alternative)
settings-agent-block-claude-command = { -claude } Code – Terminal-Befehl
settings-agent-block-claude-desktop = { -claude } Desktop – claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } – Terminal-Befehl
settings-agent-block-hermes = { -hermes } – mcp_servers (YAML)
settings-agent-block-generic = Generischer stdio-MCP-Client

settings-agent-name-required = Geben Sie einen Namen für diese Verbindung ein.
settings-agent-name-too-long = Der Name darf höchstens { $max } Zeichen lang sein.
settings-agent-name-control-characters = Der Name darf keine Steuerzeichen enthalten.

settings-agent-title = Agent-Verbindungen
settings-agent-description = Verbinden Sie { -claude }, { -codex }, { -hermes }, { -openclaw } oder einen beliebigen stdio-MCP-Client. { -brand } muss dabei laufen. Jede Verbindung hat eigene Berechtigungen: standardmäßig voller Zugriff, jederzeit pro Verbindung einschränkbar. Keine Verbindung kann jemals Ihren Wallet-Schlüssel lesen oder ändern.
settings-agent-name-label = Verbindungsname
settings-agent-name-hint = Wird in der Liste unten angezeigt, damit Sie Verbindungen unterscheiden können.
settings-agent-name-input =
    .placeholder = Coding-Agent auf dem Laptop
settings-agent-client-label = Client
settings-agent-client-hint = Bestimmt die Einrichtungsanleitung, die nach dem Erstellen der Verbindung angezeigt wird.
settings-agent-permissions-label = Berechtigungen
settings-agent-permissions-hint = Eine neue Verbindung kann alles. Schränken Sie jede Kategorie jetzt oder später in der Liste unten ein – Wallet-Schlüssel sind in beiden Fällen nie zugänglich.
settings-agent-create = Verbindung erstellen
settings-agent-issued-group =
    .aria-label = Zugangsdaten der neuen Verbindung
settings-agent-issued-warning = Kopieren Sie das Secret jetzt. Es wird nur einmal angezeigt und kann nicht erneut abgerufen werden – widerrufen Sie die Verbindung und erstellen Sie sie neu, falls Sie es verlieren. { -brand } speichert nur einen Einweg-Verifizierer; Ihr MCP-Client speichert den Klartext in seiner eigenen Konfiguration.
settings-agent-issued-client-id = Client-ID
settings-agent-issued-secret = Einmaliges Secret
settings-agent-setup-for = Einrichtung für
settings-agent-done = Fertig
settings-agent-list-title = Verbindungen
settings-agent-loading = Verbindungen werden geladen...
settings-agent-active-count = { $count } aktiv
settings-agent-empty = Noch keine Verbindungen. Erstellen Sie oben eine, um einen Client zu koppeln.
settings-agent-empty-active = Keine aktiven Verbindungen.
settings-agent-revoked-title = Widerrufene Verbindungen
settings-agent-created = Erstellt { $time }
settings-agent-last-used = Zuletzt verwendet { $time }
settings-agent-never-used = Nie verwendet
settings-agent-permissions-edit = Berechtigungen
settings-agent-revoke = Widerrufen
settings-agent-permissions-save = Berechtigungen speichern

settings-agent-load-failed = Agent-Verbindungen konnten nicht geladen werden
settings-agent-list-failed = Verbindungen konnten nicht geladen werden
settings-agent-create-failed = Die Verbindung konnte nicht erstellt werden.
settings-agent-unreachable-create = { -brand } war zum Erstellen der Verbindung nicht erreichbar.
settings-agent-permissions-update-failed = Berechtigungen konnten nicht aktualisiert werden
settings-agent-permissions-updated = Berechtigungen aktualisiert
settings-agent-permissions-updated-detail = Gilt ab der nächsten Anfrage der Verbindung.
settings-agent-unreachable-save = { -brand } war zum Speichern nicht erreichbar
settings-agent-revoke-title = Verbindung widerrufen
settings-agent-revoke-message = „{ $label }“ widerrufen? Der Client funktioniert ab seiner nächsten Anfrage nicht mehr und kann nicht wiederhergestellt werden.
settings-agent-revoke-fallback-name = diese Verbindung
settings-agent-revoke-failed = Die Verbindung konnte nicht widerrufen werden
settings-agent-unreachable-revoke = { -brand } war zum Widerrufen nicht erreichbar

## telegram_tab.js

settings-telegram-loading = { -telegram }-Einstellungen werden geladen...
settings-telegram-load-failed = { -telegram }-Einstellungen konnten nicht geladen werden
settings-telegram-unknown = Unbekannt
settings-telegram-session-active = Aktiv: { $duration }
settings-telegram-sessions-empty = Keine aktiven Sitzungen
settings-telegram-session-revoke = Widerrufen

settings-telegram-connection-title = Verbindung
settings-telegram-connection-description = Verbinden Sie Ihren { -telegram }-Bot, um Benachrichtigungen zu erhalten und { -brand } aus der Ferne zu steuern.
settings-telegram-enable-label = { -telegram } aktivieren
settings-telegram-enable-hint = { -telegram }-Bot-Integration aktivieren
settings-telegram-token-label = Bot-Token
settings-telegram-token-saved = Token gespeichert
settings-telegram-token-help = Erhalten Sie von @BotFather auf { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token gespeichert (neuen eingeben, um ihn zu ändern)
settings-telegram-token-input =
    .placeholder = Bot-Token eingeben
settings-telegram-token-toggle =
    .title = Anzeigen/Verbergen
settings-telegram-chat-label = Chat-ID
settings-telegram-chat-connected = Verbunden mit Chat:
settings-telegram-chat-discover-hint = Ihre Chat-ID automatisch ermitteln
settings-telegram-chat-change =
    .title = Ändern
settings-telegram-chat-discover = Chat-ID ermitteln
settings-telegram-discovery-step-add = Fügen Sie Ihren Bot zu einer { -telegram }-Gruppe hinzu oder starten Sie einen direkten Chat mit ihm
settings-telegram-discovery-step-privacy = Für Gruppen: Prüfen Sie @BotFather → /mybots → [Ihr Bot] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Privacy Mode AUS:</strong> Der Bot empfängt alle Gruppennachrichten<br/><strong>Privacy Mode AN:</strong> Der Bot empfängt nur Nachrichten mit @-Erwähnung
settings-telegram-discovery-step-send = Senden Sie eine beliebige Nachricht (oder erwähnen Sie Ihren Bot mit @, wenn Privacy Mode AN ist)
settings-telegram-discovery-listening = Warten auf Nachrichten...
settings-telegram-discovery-select = Auswählen
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Nachrichtensprache
settings-telegram-language-hint = Sprache der Nachrichten und Schaltflächen des { -telegram }-Bots
settings-telegram-language-follow-app = App-Sprache verwenden
settings-telegram-test-label = Verbindung testen
settings-telegram-test-hint = Testnachricht senden, um die Konfiguration zu prüfen
settings-telegram-test-send = Test senden
settings-telegram-test-sending = Wird gesendet...

settings-telegram-chat-type-private = privat
settings-telegram-chat-type-group = Gruppe
settings-telegram-chat-type-supergroup = Supergruppe
settings-telegram-chat-type-channel = Kanal

settings-telegram-auth-title = Befehlsauthentifizierung
settings-telegram-auth-description = { -telegram }-Befehle verwenden dieselbe 2FA wie der Dashboard-Sperrbildschirm.
settings-telegram-auth-protected = Geschützt
settings-telegram-auth-disabled = Deaktiviert
settings-telegram-auth-not-configured = Nicht eingerichtet
settings-telegram-auth-error = Fehler
settings-telegram-auth-protected-note = Befehle sind durch die 2FA des Sperrbildschirms geschützt. Wenn Sitzungen ablaufen, müssen Nutzer ihren Authenticator-Code über den Befehl <code>/login</code> angeben.
settings-telegram-auth-disabled-note = Die 2FA des Sperrbildschirms ist eingerichtet, aber für { -telegram } deaktiviert. Aktivieren Sie oben „2FA für Befehle verlangen“, um { -telegram }-Befehle zu schützen.
settings-telegram-auth-missing-note = Die 2FA des Sperrbildschirms ist nicht eingerichtet. Ohne 2FA werden abgelaufene Sitzungen ohne Verifizierung automatisch reaktiviert.
settings-telegram-auth-managed-in = 2FA wird verwaltet unter
settings-telegram-auth-configure-in = Richten Sie 2FA ein unter
settings-telegram-auth-configure-suffix = , um für { -telegram }-Befehle eine Verifizierung zu verlangen.
settings-telegram-security-link = Sicherheitseinstellungen
settings-telegram-timeout-title = Sitzungs-Timeout
settings-telegram-timeout-description = Wie lange eine authentifizierte Sitzung aktiv bleibt
settings-telegram-sessions-title = Aktive Sitzungen

settings-telegram-notifications-title = Benachrichtigungseinstellungen
settings-telegram-notifications-description = Wählen Sie, welche Ereignisse { -telegram }-Benachrichtigungen auslösen.
settings-telegram-notify-opened-label = Position eröffnet
settings-telegram-notify-opened-hint = Benachrichtigen, wenn eine neue Position eröffnet wird
settings-telegram-notify-closed-label = Position geschlossen
settings-telegram-notify-closed-hint = Benachrichtigen, wenn eine Position geschlossen wird
settings-telegram-notify-partial-label = Teilausstieg
settings-telegram-notify-partial-hint = Bei Teilausstiegen aus Positionen benachrichtigen
settings-telegram-notify-dca-label = DCA ausgeführt
settings-telegram-notify-dca-hint = Benachrichtigen, wenn DCA-Orders ausgeführt werden
settings-telegram-notify-errors-label = Fehler
settings-telegram-notify-errors-hint = Bei Fehlern und Ausfällen benachrichtigen
settings-telegram-notify-startup-label = Start/Beenden
settings-telegram-notify-startup-hint = Benachrichtigen, wenn der Bot startet oder stoppt
settings-telegram-notify-filtering-label = Filter-Meldungen
settings-telegram-notify-filtering-hint = Benachrichtigen, wenn neue Tokens die Filterkriterien bestehen
settings-telegram-notify-trades-label = Trade-Meldungen
settings-telegram-notify-trades-hint = Bei bedeutenden Trades für beobachtete Tokens benachrichtigen
settings-telegram-notify-daily-label = Tageszusammenfassung
settings-telegram-notify-daily-hint = Tägliche Zusammenfassung von Trading-Aktivität und GuV erhalten

settings-telegram-features-title = Funktionen
settings-telegram-features-description = Konfigurieren Sie die Fähigkeiten des { -telegram }-Bots.
settings-telegram-commands-label = Befehle aktivieren
settings-telegram-commands-hint = Steuerung des Bots über { -telegram }-Befehle erlauben
settings-telegram-require-2fa-label = 2FA für Befehle verlangen
settings-telegram-require-2fa-hint = Bei abgelaufenen Sitzungen einen 2FA-Code zur Reaktivierung verlangen. Nutzt die 2FA des Sperrbildschirms.
settings-telegram-inline-label = Inline-Aktionsschaltflächen
settings-telegram-inline-hint = Aktionsschaltflächen in Benachrichtigungen anzeigen

settings-telegram-setting-save-failed = { -telegram }-Einstellung konnte nicht gespeichert werden
settings-telegram-discovery-start-failed = Die Ermittlung konnte nicht gestartet werden
settings-telegram-chat-selected = Chat ausgewählt
settings-telegram-chat-select-failed = Chat konnte nicht ausgewählt werden
settings-telegram-test-sent = Testnachricht gesendet
settings-telegram-test-failed = Testnachricht fehlgeschlagen
settings-telegram-session-revoked = Sitzung widerrufen
settings-telegram-session-revoke-failed = Sitzung konnte nicht widerrufen werden

## licenses_tab.js

settings-licenses-title = Open-Source-Lizenzen
settings-licenses-subtitle = { -brand } basiert auf der folgenden Open-Source-Software
settings-licenses-footer = Die vollständigen Lizenztexte finden Sie im Projekt-Repository und im Quellcode der jeweiligen Abhängigkeit.
settings-licenses-category-framework = Anwendungs-Framework
settings-licenses-category-solana = Solana-Blockchain
settings-licenses-category-data = Daten und Speicher
settings-licenses-category-networking = Netzwerk
settings-licenses-category-cryptography = Kryptografie und Kodierung
settings-licenses-category-assets = UI-Assets
settings-licenses-desc-electron = Framework für Desktop-Anwendungen
settings-licenses-desc-tokio = Asynchrone Laufzeitumgebung für Rust
settings-licenses-desc-axum = Webserver-Framework
settings-licenses-desc-tower = Dienstabstraktionen
settings-licenses-desc-hyper = HTTP-Implementierung
settings-licenses-desc-solana-sdk = Solana-SDK-Kern
settings-licenses-desc-solana-client = RPC-Client
settings-licenses-desc-solana-program = Programmbibliothek
settings-licenses-desc-spl-token = SPL-Token-Programm
settings-licenses-desc-spl-token-2022 = Token-2022-Erweiterungen
settings-licenses-desc-spl-associated-token-account = Zugehörige Token-Konten
settings-licenses-desc-sqlite = Eingebettete Datenbank-Engine
settings-licenses-desc-rusqlite = SQLite-Bindings für Rust
settings-licenses-desc-r2d2 = Datenbank-Verbindungspool
settings-licenses-desc-serde = Serialisierungs-Framework
settings-licenses-desc-toml = Konfigurations-Parser
settings-licenses-desc-reqwest = HTTP-Client
settings-licenses-desc-tokio-tungstenite = WebSocket-Client
settings-licenses-desc-rustls = TLS-Implementierung
settings-licenses-desc-blake3 = Hashfunktion
settings-licenses-desc-sha-2 = SHA-256/512-Hashing
settings-licenses-desc-bs58 = Base58-Kodierung
settings-licenses-desc-base64 = Base64-Kodierung
settings-licenses-desc-lucide-icons = Icon-Font-Bibliothek
settings-licenses-desc-inter = Oberflächenschrift
settings-licenses-desc-jetbrains-mono = Monospace-Schrift
settings-licenses-desc-orbitron = Display-Schrift
settings-licenses-desc-vazirmatn = Schrift für Arabisch und Persisch
settings-licenses-desc-noto-sans-devanagari = Schrift für Devanagari
settings-licenses-desc-noto-sans-sc = Schrift für vereinfachtes Chinesisch
settings-licenses-desc-pretendard = Schrift für Koreanisch
settings-licenses-desc-pretendard-jp = Schrift für Japanisch

## hints_tab.js

settings-hints-title = Kontexthinweise
settings-hints-description = Kontexthinweise sind die Hilfe-Symbole, die Dashboard-Funktionen erklären. Sehen Sie sich unten alle Hinweise an und stellen Sie jene wieder her, die Sie mit „Nicht mehr anzeigen“ ausgeblendet haben – einzeln oder alle zusammen.
settings-hints-hidden-label = Ausgeblendete Hinweise
settings-hints-hidden-summary = { $hidden } von { $total } Hinweisen sind derzeit ausgeblendet.
settings-hints-restore-all = Alle Hinweise wiederherstellen
settings-hints-toggle-shown =
    .title = Diesen Hinweis anzeigen
settings-hints-toggle-shown-title = Angezeigt
settings-hints-toggle-hidden-title = Ausgeblendet – einschalten zum Anzeigen
settings-hints-restore-title = Alle Hinweise wiederherstellen
settings-hints-restore-message = Alle Kontexthinweise wieder anzeigen, auch die von Ihnen ausgeblendeten?
settings-hints-restore-confirm = Alle wiederherstellen
settings-hints-restored = Alle Hinweise wiederhergestellt

## account_tab.js

settings-account-title = { -brand }-Konto
settings-account-description = Kostenlos und optional. { -brand } handelt, erkennt Märkte und zeichnet Charts auch ohne Konto – nur eben über die öffentlichen Anbieter. Das Panel unten listet auf, was die Anmeldung zusätzlich bietet.
settings-account-data-title = { -brand }-Daten
settings-account-data-description = Wir betreiben unter screenerbot.io einen gemeinsamen Marktdaten-Dienst: gebündelte Kerzen in sieben Zeitrahmen, ein aufgelöstes Pool-Register, zwischengespeicherte Sicherheitsberichte und normalisierte Token-Identitäten. Er sorgt dafür, dass nicht jede Installation einzeln von den öffentlichen Anbietern gedrosselt wird; die Nutzung erfordert ein Konto, damit diese gemeinsamen Kosten zugeordnet werden können.
settings-account-data-fallback = Ist er nicht verfügbar, greift { -brand } automatisch auf die öffentlichen Anbieter zurück. Nichts bleibt stehen; Charts füllen sich langsamer und enthalten weniger Verlauf.
settings-account-gateway-title = Transaktionen senden
settings-account-gateway-description = Wenn Sie angemeldet sind, kann { -brand } Ihre Swaps über screenerbot.io statt über Ihren eigenen RPC senden. Ihr Bot erstellt und signiert jede Transaktion weiterhin auf diesem Rechner – der Server leitet sie nur weiter und kann eine signierte Transaktion nicht ändern, ohne ihre Signatur ungültig zu machen.
settings-account-gateway-label = { -brand }-RPC zum Senden von Transaktionen verwenden
settings-account-gateway-hint = Nur zum Senden. Preisdaten stammen immer von Ihrem eigenen RPC – das Pool-Polling ist für einen gemeinsamen Endpunkt viel zu aufwendig und wird deshalb nie dorthin gesendet.
settings-account-manage-title = Konto verwalten
settings-account-manage-description = Ihr Passwort, Ihre E-Mail-Adresse, verbundene Geräte und Empfehlungsauszahlungen werden auf der Website verwaltet. Das Widerrufen eines Geräts dort meldet es überall ab, auch dieses.
settings-account-open-dashboard = Ihr Dashboard öffnen

## navigation_tab.js

settings-navigation-title = Navigations-Tabs
settings-navigation-hint = Zum Umsortieren Elemente ziehen. Die Sichtbarkeit lässt sich mit dem Schalter umschalten.
settings-navigation-note = Änderungen gelten nach dem Speichern. Laden Sie die Seite neu, um die Aktualisierungen in der Navigationsleiste zu sehen.
settings-navigation-drag-handle =
    .title = Zum Umsortieren ziehen
settings-navigation-defaults-failed = Die Standardnavigation konnte nicht geladen werden
settings-navigation-reset = Navigation auf Standard zurückgesetzt

## data_tab.js

settings-data-storage-title = Datenbankspeicher
settings-data-storage-description = Übersicht aller Datenbanken, die Ihre Trading-Daten, Positionen und Verlaufsdaten speichern.
settings-data-stats-loading = Datenbankstatistiken werden geladen...
settings-data-stats-load-failed = Datenbankstatistiken konnten nicht geladen werden
settings-data-total-storage = Gesamter Datenbankspeicher
settings-data-db-tokens = Tokens
settings-data-db-transactions = Transaktionen
settings-data-db-positions = Positionen
settings-data-db-events = Ereignisse
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Wallet
settings-data-db-pools = Pools
settings-data-db-strategies = Strategien
settings-data-db-actions = Aktionen
settings-data-directory-label = Datenverzeichnis
settings-data-directory-copied = Datenverzeichnis
settings-data-config-path-copied = Konfigurationspfad
settings-data-path-unavailable = Nicht verfügbar
settings-data-path-copy-title = Zum Kopieren des Pfads klicken
settings-data-path-copy-failed = Pfad konnte nicht kopiert werden

settings-data-config-title = Konfigurationsverwaltung
settings-data-config-description = Exportieren, importieren und verwalten Sie Ihre Bot-Konfiguration. Legen Sie vor größeren Änderungen Sicherungen an.
settings-data-config-export = Konfiguration exportieren
settings-data-config-import = Konfiguration importieren
settings-data-config-reset = Auf Standard zurücksetzen
settings-data-config-location-label = Konfigurationsort
settings-data-config-fetch-failed = Konfiguration konnte nicht abgerufen werden
settings-data-config-exported = Konfiguration exportiert
settings-data-config-export-failed = Konfiguration konnte nicht exportiert werden: { $message }
settings-data-config-import-title = Konfiguration importieren
settings-data-config-import-message = Diese Konfiguration importieren? Die aktuellen Einstellungen werden überschrieben. Wallet-Zugangsdaten bleiben erhalten.
settings-data-config-imported = Konfiguration erfolgreich importiert. Einige Änderungen erfordern einen Neustart.
settings-data-config-import-failed = Konfiguration konnte nicht importiert werden: { $message }
settings-data-config-reset-title = Konfiguration zurücksetzen
settings-data-config-reset-message = Alle Einstellungen auf Standard zurücksetzen? Ihre Wallet-Zugangsdaten bleiben erhalten, alle anderen Einstellungen werden zurückgesetzt.
settings-data-config-reset-done = Konfiguration auf Standard zurückgesetzt
settings-data-config-reset-failed = Konfiguration konnte nicht zurückgesetzt werden: { $message }
settings-data-unknown-error = Unbekannter Fehler

settings-data-cleanup-title = Datenbereinigung
settings-data-cleanup-description = Geben Sie Speicherplatz frei, indem Sie alte oder ungenutzte Daten entfernen. Diese Aktionen können nicht rückgängig gemacht werden.
settings-data-ohlcv-cleanup-label = OHLCV-Datenbereinigung
settings-data-ohlcv-cleanup-hint = Kerzendaten von Tokens entfernen, die für die angegebene Zeit nicht aktiv waren.
settings-data-cleanup-hours-unit = Stunden
settings-data-cleanup-ohlcv = OHLCV bereinigen
settings-data-cleanup-running = Wird bereinigt...
settings-data-cleanup-hours-invalid = Ungültiger Stundenwert
settings-data-cleanup-confirm-title = OHLCV-Daten löschen
settings-data-cleanup-confirm-message =
    OHLCV-Daten von Tokens löschen, die seit mehr als { $hours ->
        [one] { $hours } Stunde
       *[other] { $hours } Stunden
    } inaktiv sind?
settings-data-cleanup-done =
    { $count ->
        [one] { $count } inaktiver Token bereinigt
       *[other] { $count } inaktive Tokens bereinigt
    }
settings-data-cleanup-failed = Bereinigung fehlgeschlagen
settings-data-cleanup-failed-detail = Bereinigung fehlgeschlagen: { $message }

settings-data-cache-clear-label = Gesamten OHLCV-Cache leeren
settings-data-cache-clear-hint = Löscht alle zwischengespeicherten Kerzendaten und lädt jeden überwachten Token von Grund auf neu. Nutzen Sie dies, wenn Charts fehlerhaft aussehen oder nach einer Änderung der Datenlogik.
settings-data-cache-clear = OHLCV-Cache leeren
settings-data-cache-clearing = Wird geleert...
settings-data-cache-confirm-title = Gesamten OHLCV-Cache leeren
settings-data-cache-confirm-message = Alle zwischengespeicherten Kerzendaten für jeden Token löschen? Überwachte Tokens laden ihren Verlauf von Grund auf neu. Dies kann nicht rückgängig gemacht werden.
settings-data-candles-count =
    { $count ->
        [one] { $count } Kerze
       *[other] { $count } Kerzen
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } Token
       *[other] { $count } Tokens
    }
settings-data-cache-cleared = { $candles } von { $tokens } geleert; Daten werden neu geladen
settings-data-cache-clear-failed = OHLCV-Cache konnte nicht geleert werden
settings-data-cache-clear-failed-detail = OHLCV-Cache konnte nicht geleert werden: { $message }

settings-data-ui-cache-label = UI-Status-Cache
settings-data-ui-cache-hint = Gespeicherte Tabelleneinstellungen, Filterstatus und Ansichtseinstellungen löschen.
settings-data-ui-cache-clear = UI-Cache leeren
settings-data-ui-cache-confirm-title = UI-Status leeren
settings-data-ui-cache-confirm-message = Alle gespeicherten UI-Einstellungen löschen? Dadurch werden Tabellenspalten, Filter und Ansichtseinstellungen zurückgesetzt.
settings-data-ui-cache-cleared =
    { $count ->
        [one] { $count } zwischengespeicherte UI-Einstellung geleert
       *[other] { $count } zwischengespeicherte UI-Einstellungen geleert
    }

settings-data-folder-label = Datenordner öffnen
settings-data-folder-hint = Öffnet den Ordner mit allen { -brand }-Daten in Ihrem Dateimanager.
settings-data-folder-open = Ordner öffnen
settings-data-folder-open-failed = Der Datenordner konnte nicht geöffnet werden
