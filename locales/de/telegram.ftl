## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Status
telegram-reply-balance = Guthaben
telegram-reply-positions = Positionen
telegram-reply-pause = Pause
telegram-reply-resume = Fortsetzen
telegram-reply-stop = Stopp
telegram-reply-stats = Statistik
telegram-reply-menu = Menü
telegram-reply-help = Hilfe

## Inline keyboard buttons.

telegram-button-positions = Positionen
telegram-button-balance = Guthaben
telegram-button-stats = Statistik
telegram-button-tokens = Tokens
telegram-button-pause = Pause
telegram-button-stop = Stopp
telegram-button-settings = Einstellungen
telegram-button-refresh = Aktualisieren
telegram-button-menu = Menü
telegram-button-back = Zurück
telegram-button-back-to-menu = Zurück zum Menü
telegram-button-back-to-tokens = Zurück zu Tokens
telegram-button-cancel = Abbrechen
telegram-button-close-all-positions = Alle Positionen schließen
telegram-button-sell-percent = { $percent }% verkaufen
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Blacklist
telegram-button-blacklist-symbol = { $symbol } auf Blacklist
telegram-button-close-position = Position schließen
telegram-button-confirm-close = Schließen bestätigen
telegram-button-confirm-close-all = ALLE Positionen schließen
telegram-button-confirm-sell = { $percent }% Verkauf bestätigen
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = SOFORTSTOPP BESTÄTIGEN
telegram-button-confirm-buy = { $amount } { -sol } kaufen
telegram-button-notifications = Benachrichtigungen
telegram-button-trading = Trading
telegram-button-entry-monitor = Einstiegsmonitor
telegram-button-exit-monitor = Ausstiegsmonitor
telegram-button-auto-trading = Auto-Trading
telegram-button-force-stop = Sofortstopp
telegram-button-notify-opened = Eröffnet
telegram-button-notify-closed = Geschlossen
telegram-button-notify-partial = Teilweise
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Fehler
telegram-button-details = Details
telegram-button-position = Position
telegram-button-sell-more = Mehr verkaufen
telegram-button-more-dca = Mehr DCA
telegram-button-history = Verlauf
telegram-button-status = Status
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Neu authentifizieren
telegram-button-previous = Zurück
telegram-button-next = Weiter
telegram-button-passed = Bestanden
telegram-button-rejected = Abgelehnt
telegram-button-new-24h = Neu (24 Std.)
telegram-button-all-tokens = Alle Tokens
telegram-button-search-token = Token suchen
telegram-button-filter-stats = Filterstatistik
telegram-button-refresh-stats = Statistik aktualisieren
telegram-button-view-position = Position ansehen
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Unbekannter Befehl: { $command }

    Mit /help sehen Sie die verfügbaren Befehle.
telegram-session-expired =
    <b>Sitzung abgelaufen</b>

    Mit /login authentifizieren Sie sich erneut.
telegram-2fa-required =
    <b>2FA erforderlich</b>

    Bitte geben Sie Ihren 6-stelligen Authenticator-Code ein.
telegram-account-locked =
    <b>Konto gesperrt</b>

    Zu viele fehlgeschlagene Versuche.
    Versuchen Sie es erneut in { $seconds ->
        [one] { $seconds } Sekunde.
       *[other] { $seconds } Sekunden.
    }
telegram-code-invalid = Bitte geben Sie einen gültigen 6-stelligen Code ein.
telegram-authenticated =
    <b>Authentifiziert!</b>

    Sie haben nun Zugriff auf die Bot-Befehle.
telegram-wrong-code =
    <b>Falscher Code</b>

    { $remaining ->
        [one] { $remaining } Versuch übrig.
       *[other] { $remaining } Versuche übrig.
    }
telegram-auth-required =
    <b>Authentifizierung erforderlich</b>

    Bitte geben Sie zum Fortfahren Ihr Passwort ein.

    <i>Passwort eingeben und senden.</i>
telegram-login-required =
    <b>Anmeldung erforderlich</b>

    Bitte geben Sie Ihren 6-stelligen Authenticator-Code ein:
telegram-session-activated =
    <b>Sitzung aktiviert</b>

    2FA ist nicht eingerichtet. Ihre Sitzung ist jetzt aktiv.

    <i>Tipp: Aktivieren Sie 2FA in den Sicherheitseinstellungen für mehr Sicherheit.</i>

## Chat discovery.

telegram-discovery-hello = Hallo { $name }!
telegram-discovery-default-name = Nutzer
telegram-discovery-detected = <b>Chat erkannt!</b>
telegram-discovery-details =
    Chat-ID: <code>{ $chat_id }</code>
    Typ: { $chat_type }

    Öffnen Sie das { -brand }-Dashboard und klicken Sie auf diesen Chat, um ihn auszuwählen.
telegram-chat-type-private = privat
telegram-chat-type-group = Gruppe
telegram-chat-type-supergroup = Supergruppe
telegram-chat-type-channel = Kanal

## Menus.

telegram-menu-title =
    <b>Kontrollzentrum</b>

    Wählen Sie eine Option, um Informationen anzuzeigen oder den Bot zu steuern.
telegram-menu-positions-empty =
    <b>Keine offenen Positionen</b>

    Warte auf neue Gelegenheiten...
telegram-menu-positions-title = <b>Positionen ({ $count })</b>
telegram-menu-positions-hint = <i>Tippen Sie auf eine Position, um sie zu verwalten.</i>
telegram-menu-settings =
    <b>Einstellungen</b>

    Benachrichtigungen und Trading-Parameter konfigurieren.
telegram-settings-notifications =
    <b>Benachrichtigungseinstellungen</b>

    Benachrichtigungen ein-/ausschalten:
telegram-settings-trading =
    <b>Trading-Steuerung</b>

    Trading-Funktionen ein-/ausschalten:
telegram-pagination-expired = Die Seitensitzung ist abgelaufen.

## Status commands.

telegram-status-state-stopped = <b>GESTOPPT</b> (Sofortstopp aktiv)
telegram-status-state-active = <b>AKTIV</b>
telegram-status-state-paused = <b>PAUSIERT</b>
telegram-status-on = AN
telegram-status-off = AUS
telegram-status-body =
    <b>Systemstatus</b>

    <b>System</b>
    Zustand — { $state }
    Laufzeit — { $uptime }
    Version — v{ $version }

    <b>Trading</b>
    Einstiege — { $entries }
    Ausstiege — { $exits }
    Positionen — { $positions }
telegram-positions-empty =
    <b>Keine offenen Positionen</b>

    Warte auf Gelegenheiten...
telegram-positions-title = <b>Offene Positionen ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } weitere...</i>
telegram-positions-summary =
    <b>Portfolio-Übersicht</b>
    Investiert — { $invested } { -sol }
    Netto-GuV — { $pnl } { -sol }
telegram-balance-body =
    <b>Wallet-Guthaben</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Tagesstatistik</b>

    Positionen — { $positions }
    Investiert — { $invested } { -sol }
    GuV — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } ist bereit!</b>

    Trading ist <b>aktiviert</b>.

    Steuern Sie den Bot mit der Tastatur unten.
    Mit /help sehen Sie die verfügbaren Befehle.
telegram-stop-already = <b>Trading ist bereits deaktiviert</b>
telegram-stop-done =
    <b>Trading deaktiviert</b>

    Alle Trading-Monitore (Einstiege { "&amp;" } Ausstiege) sind gestoppt.
    Mit /pause stoppen Sie nur Einstiege.
telegram-stop-failed =
    <b>Trading konnte nicht deaktiviert werden</b>

    Fehler: { $detail }
telegram-pause-done =
    <b>Einstiegsmonitor pausiert</b>

    Es werden keine neuen Positionen eröffnet.
    Der Ausstiegsmonitor läuft weiter.
telegram-pause-failed =
    <b>Einstiege konnten nicht pausiert werden</b>

    Fehler: { $detail }
telegram-resume-done =
    <b>Einstiegsmonitor fortgesetzt</b>

    Es wird wieder auf Einstiegssignale geachtet.
telegram-resume-failed =
    <b>Einstiege konnten nicht fortgesetzt werden</b>

    Fehler: { $detail }
telegram-force-stop-confirm =
    <b>SOFORTSTOPP</b>

    Dadurch wird SÄMTLICHE Trading-Aktivität sofort angehalten:
    • Keine neuen Einstiege
    • Keine Ausstiege (auch keine Stop-Loss)
    • Keine DCA-Vorgänge
telegram-force-stop-warning = <b>Dies ist eine Notfallmaßnahme!</b>
telegram-force-stop-question = Sind Sie sicher?
telegram-force-stop-active =
    <b>SOFORTSTOPP AKTIVIERT</b>

    Das gesamte Trading wurde angehalten.

    Mit /resume_trading heben Sie diesen Zustand auf.
telegram-resume-trading-not-stopped =
    <b>Trading ist nicht per Sofortstopp angehalten</b>

    Keine Aktion erforderlich.
telegram-resume-trading-done =
    <b>Trading fortgesetzt</b>

    Der Sofortstopp wurde aufgehoben.
    Der normale Trading-Betrieb kann nun weiterlaufen.

## Help.

telegram-help-title = <b>{ -brand }-Hilfe</b>
telegram-help-heading-dashboard = Dashboard
telegram-help-heading-market = Markt
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Sicherheit
telegram-help-heading-system = System
telegram-help-commands-dashboard =
    /status — Systemstatus { "&amp;" } Laufzeit
    /stats — Tagesperformance
    /balance — Wallet-Guthaben
    /positions — Offene Positionen
telegram-help-commands-market =
    /tokens — Token-Explorer
    /rejected — Gefilterte Tokens
telegram-help-commands-trading =
    /start — Trading-System aktivieren
    /stop — Trading-System deaktivieren
    /pause — Neue Einstiege pausieren
    /resume — Neue Einstiege fortsetzen
    /menu — Interaktives Menü
telegram-help-commands-safety =
    /force_stop — <b>NOTHALT</b>
    /resume_trading — Notfallstatus aufheben
telegram-help-commands-system =
    /update — Update-Status { "&amp;" } Installation
    /login — 2FA-Authentifizierung
telegram-help-tip = <i>Tipp: Tippen Sie auf einen Befehl, um ihn auszuführen.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Auf dem neuesten Stand</b>

    v{ $version } läuft, automatisch installiert.
telegram-update-up-to-date =
    <b>Auf dem neuesten Stand</b>

    v{ $version } läuft.
telegram-update-check-failed =
    <b>Update-Prüfung fehlgeschlagen</b>

    { $reason }
telegram-update-unreachable = screenerbot.io war nicht erreichbar.
telegram-update-installing = <b>v{ $version } wird installiert</b>
telegram-update-restarting =
    { -brand } startet mit der neuen Version neu. Das Trading wird automatisch fortgesetzt.
telegram-update-install-failed =
    <b>v{ $version } konnte nicht installiert werden</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } ist heruntergeladen</b>

    Dieses Release aktualisiert auch die Desktop-App, daher muss deren Installer auf dem Rechner ausgeführt werden. Öffnen Sie dort Einstellungen → Updates.
telegram-update-downloading =
    <b>v{ $version } wird heruntergeladen</b>

    { $percent }% von { $size } MB.
telegram-update-available =
    <b>v{ $version } ist verfügbar</b>

    { $how }
    Downloadgröße: { $size } MB.

    Der Download startet automatisch; senden Sie /update erneut, sobald er bereit ist.
telegram-update-how-core = Wird still mit kurzem Neustart installiert.
telegram-update-how-installer = Der Desktop-Installer muss einmal ausgeführt werden.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Unbekannt
telegram-value-na = k. A.
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds } s
telegram-duration-minutes = { $minutes } Min.
telegram-duration-minutes-seconds = { $minutes } Min. { $seconds } s
telegram-duration-hours = { $hours } Std.
telegram-duration-hours-minutes = { $hours } Std. { $minutes } Min.
telegram-duration-days = { $days } T.
telegram-duration-days-hours = { $days } T. { $hours } Std.
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = Fehler: { $detail }
telegram-ai-reasoning =
    <b>LLM-Analyse</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Einstieg — { $price } { -sol }
telegram-row-exit = Ausstieg — { $price } { -sol }
telegram-row-current = Aktuell — { $price } { -sol }
telegram-row-invested = Investiert — { $amount } { -sol }
telegram-row-received = Erhalten — { $amount } { -sol }
telegram-row-value = Wert — { $amount } { -sol }
telegram-row-total = Gesamt — { $amount } { -sol }
telegram-row-tokens = Tokens — { $tokens }
telegram-row-duration = Dauer — { $duration }
telegram-row-reason = Grund — { $reason }
telegram-row-remaining = Verbleibend — { $percent }%
telegram-row-pnl = GuV — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Position eröffnet</b>
telegram-notify-opened-size = Größe — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Preis — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Position geschlossen</b> — Gewinn
telegram-notify-closed-title-loss = <b>Position geschlossen</b> — Verlust
telegram-notify-closed-reason-unspecified = Geschlossen
telegram-notify-partial-title = <b>Teilausstieg</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — { $percent }% verkauft
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Hinzugefügt — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Ø — { $price } { -sol }
telegram-notify-severity-critical = <b>Kritischer Fehler</b>
telegram-notify-severity-error = <b>Fehler</b>
telegram-notify-severity-warning = <b>Warnung</b>
telegram-notify-severity-info = <b>Info</b>
telegram-notify-alert-title = <b>Trade-Alarm</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Aktion: { $amount } { -sol } gekauft
telegram-notify-alert-sold = Aktion: { $amount } { -sol } verkauft
telegram-notify-alert-wallet = Wallet: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (Paper)
telegram-notify-copy-task = Aufgabe: { $task }
telegram-notify-scheduled-completed = <b>Geplante Aufgabe abgeschlossen</b>
telegram-notify-scheduled-failed = <b>Geplante Aufgabe fehlgeschlagen</b>
telegram-notify-scheduled-timed-out = <b>Zeitüberschreitung bei geplanter Aufgabe</b>
telegram-notify-scheduled-error = Fehler: { $error }
telegram-notify-summary-title = <b>Tageszusammenfassung</b> — { $date }
telegram-notify-summary-performance = <b>Performance</b>
telegram-notify-summary-trades = Trades — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Gewinnquote — { $percent }%
telegram-notify-summary-pnl = GuV — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Offene Positionen — { $count }
telegram-notify-started-title = <b>{ -brand } gestartet</b>
telegram-notify-started-version = <b>Version</b> — { $version }
telegram-notify-started-mode = <b>Modus</b> — { $mode }
telegram-notify-started-ready = Bereit fürs Trading!
telegram-notify-stopped-title = <b>{ -brand } gestoppt</b>
telegram-notify-stopped-reason = <b>Grund</b> — { $reason }
telegram-notify-stopped-goodbye = Auf Wiedersehen! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Ordnungsgemäßes Herunterfahren
telegram-notify-update-available =
    <b>Update v{ $version } verfügbar</b>

    { $how }
    Downloadgröße: { $size } MB
telegram-notify-update-how-installer = Dieses Release aktualisiert auch die Desktop-App, daher muss deren Installer einmal ausgeführt werden.
telegram-notify-update-ready =
    <b>Update v{ $version } bereit</b>

    { $how }
telegram-notify-update-ready-silent = Senden Sie /update, um es jetzt anzuwenden, oder es wird beim nächsten Start von { -brand } installiert.
telegram-notify-update-ready-installer = Öffnen Sie Einstellungen → Updates, um den Installer auszuführen.
telegram-notify-update-applying =
    <b>v{ $version } wird installiert</b>

    Das Backend startet neu; das Trading wird automatisch fortgesetzt.
telegram-notify-new-tokens =
    <b>Filter-Alarm</b>

    { $count ->
        [one] { $count } neuer Token gefunden, der Ihren Kriterien entspricht.
       *[other] { $count } neue Tokens gefunden, die Ihren Kriterien entsprechen.
    }
telegram-notify-crash =
    <b>Bot abgestürzt!</b>

    <b>Ort:</b> <code>{ $location }</code>
    <b>Fehler:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Bitte starten Sie den Bot neu.

## Filter results page.

telegram-filter-results-title = <b>Filterergebnisse</b> ({ $count })
telegram-filter-results-empty = <i>Keine Tokens gefunden.</i>
telegram-filter-results-page = <i>Seite { $page } von { $total }</i>

## Position screens.

telegram-position-not-found = Position nicht gefunden
telegram-position-no-positions = Keine Positionen zum Schließen
telegram-position-history-empty =
    <b>Trade-Verlauf</b>

    Noch keine geschlossenen Positionen.
telegram-position-history-title = <b>Letzte Trades</b>
telegram-position-history-more = <i>+{ $count } weitere Trades...</i>
telegram-position-confirm-hint = <i>Bestätigen Sie innerhalb von 30 s, um auszuführen.</i>
telegram-position-confirm-close-title = <b>Position schließen?</b>
telegram-position-confirm-close-selling = { $tokens } Tokens werden verkauft
telegram-position-confirm-close-estimated = Geschätzt — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Bestätigen Sie innerhalb von 30 Sekunden</i>
telegram-position-confirm-sell =
    <b>Verkauf bestätigen</b>

    Token — { $symbol }
    Menge — { $percent }%
    Tokens — { $tokens }
telegram-position-confirm-dca =
    <b>Nachkauf bestätigen</b>

    Token — { $symbol }
    Zukauf — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Alle Positionen schließen?</b>

    Anzahl — { $count }
telegram-position-confirm-close-all-hint =
    <i>Dadurch werden alle offenen Positionen zum Marktpreis verkauft.
    Bestätigen Sie innerhalb von 30 s.</i>
telegram-position-confirm-force-stop =
    <b>SOFORTSTOPP</b>

    Dadurch wird SÄMTLICHES Trading sofort angehalten:
    • Keine neuen Einstiege
    • Keine Ausstiege
    • Kein DCA
telegram-position-confirm-force-stop-warning = <b>Dies ist eine Notfallmaßnahme.</b>
telegram-position-confirm-blacklist =
    <b>Token auf die Blacklist setzen?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Dadurch wird die Position geschlossen und künftige Einstiege werden verhindert.</i>
telegram-position-selling = { $percent }% von { $symbol } werden verkauft...
telegram-position-sell-done =
    <b>Verkauf ausgeführt</b>

    Token — { $symbol }
    Verkauft — { $percent }%
    Erhalten — { $amount } { -sol }
telegram-position-sell-failed = <b>Verkauf fehlgeschlagen</b>
telegram-position-adding = { $amount } { -sol } werden zu { $symbol } hinzugefügt...
telegram-position-dca-done =
    <b>DCA ausgeführt</b>

    Token — { $symbol }
    Hinzugefügt — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA fehlgeschlagen</b>
telegram-position-closing-all = Alle Positionen werden geschlossen...
telegram-position-close-all-done =
    <b>Alle schließen abgeschlossen</b>

    Geschlossen — { $closed }
    Fehlgeschlagen — { $failed }
telegram-position-blacklisted =
    <b>Token auf Blacklist</b>

    Token — { $symbol }
    Status — Geschlossen { "&amp;" } auf Blacklist

## Token screens.

telegram-token-not-found = Token nicht gefunden
telegram-token-not-found-prefix = Token nicht gefunden. Versuchen Sie die Suche mit einem längeren Präfix.
telegram-token-stats-failed = Statistik konnte nicht abgerufen werden: { $detail }
telegram-token-list-failed = Tokens konnten nicht abgerufen werden: { $detail }
telegram-token-list-empty = Keine Tokens in der Ansicht <b>{ $view }</b> gefunden.
telegram-token-view-passed = Filter bestanden
telegram-token-view-rejected = Abgelehnt
telegram-token-view-recent = Zuletzt hinzugefügt
telegram-token-view-all = Alle Tokens
telegram-token-list-title = <b>{ $name }</b> (Seite { $page }/{ $total })
telegram-token-list-stats = Liq.: { $liquidity } • Preis: { $price }
telegram-token-list-hint = <i>Tippen Sie auf /token_ID für Details</i>
telegram-token-explorer =
    <b>Markt-Explorer</b>

    <b>Übersicht</b>
    Filter bestanden — { $passed }
    Abgelehnt — { $rejected }
    Aktive Preise — { $priced }
    Insgesamt entdeckt — { $total }

    <i>Wählen Sie eine Kategorie zum Durchsuchen:</i>
telegram-token-filter-title = <b>Filteranalyse</b>
telegram-token-filter-distribution = <b>Verteilung</b>
telegram-token-filter-passed = Bestanden — { $count } ({ $percent }%)
telegram-token-filter-rejected = Abgelehnt — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Auf Blacklist — { $count }
telegram-token-filter-coverage = <b>Abdeckung</b>
telegram-token-filter-priced = Mit Pool-Preis — { $count }
telegram-token-filter-open = Offene Positionen — { $count }
telegram-token-filter-total = Insgesamt entdeckt — { $count }
telegram-token-filter-updated = <b>Zuletzt aktualisiert</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Aktualisiert sich automatisch alle { $interval }</i>
telegram-token-detail-active = <b>Aktive Position</b>
telegram-token-detail-price = Preis — { $price } { -sol }
telegram-token-detail-liquidity = Liquidität — { $value }
telegram-token-detail-volume = 24-Std.-Volumen — { $value }
telegram-token-detail-change = 24-Std.-Änderung — { $value }
telegram-token-detail-risk = Risikobewertung: { $score }/100
telegram-token-detail-risk-unknown = Risikobewertung: Unbekannt
telegram-token-detail-action = <i>Aktion wählen:</i>
telegram-token-search =
    <b>Markt durchsuchen</b>

    Geben Sie Symbol oder Mint-Adresse zur Suche ein:

    <i>Beispiel: /token_BONK oder /token_So11111</i>
telegram-token-confirm-buy =
    <b>Direktkauf bestätigen</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Betrag — { $amount } { -sol }

    <i>Bestätigen Sie innerhalb von 30 s, um auszuführen.</i>
telegram-token-confirm-blacklist =
    <b>Token auf die Blacklist setzen?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Dadurch kann dieser Token Filter nicht mehr erfüllen.</i>
telegram-token-blacklisted =
    <b>Token auf Blacklist</b>

    Token — ${ $symbol }
    Status — Zur Blacklist hinzugefügt
telegram-token-blacklist-failed = <b>Blacklist fehlgeschlagen</b>
telegram-token-buy-processing =
    <b>Kauf wird verarbeitet...</b>

    Token — ${ $symbol }
    Betrag — { $amount } { -sol }
telegram-token-buy-done =
    <b>Kauf erfolgreich</b>

    Token — ${ $symbol }
    Betrag — { $amount } { -sol }

    <i>Details unter /positions</i>
telegram-token-buy-failed =
    <b>Kauf fehlgeschlagen</b>

    Token — ${ $symbol }
    Fehler — { $detail }
