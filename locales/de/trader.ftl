# Trader page labels.

trader-exit-type-stop-loss = Stop-Loss
trader-exit-type-take-profit = Take Profit
trader-exit-type-roi = ROI-Ziel
trader-exit-type-roi-exit = ROI-Ziel
trader-exit-type-trailing-stop = Trailing-Stop
trader-exit-type-time-override = Zeit-Override
trader-exit-type-time-rule = Zeitregel
trader-exit-type-manual = Manuell
trader-exit-type-manual-close = Manuell
trader-exit-type-dca = DCA
trader-exit-type-unknown = Unbekannt

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Statistiken
trader-tab-strategy-control = Strategiesteuerung
trader-tab-strategies = Strategien
trader-tab-stop-loss = Stop-Loss
trader-tab-trailing-stop = Trailing-Stop
trader-tab-roi = Take Profit
trader-tab-time-rules = Zeitregeln
trader-tab-dca = DCA
trader-tab-settings = Einstellungen

## Feature status badges and their messages

trader-feature-coming-soon = Demnächst
    .message = Diese Funktion kommt bald und ist noch nicht verfügbar.
trader-feature-beta = Beta
trader-feature-disabled = Deaktiviert
    .message = Diese Funktion ist derzeit deaktiviert.

## Status bar and trading controls

trader-status-title = Auto-Trader
trader-status-loading = Wird geladen...
trader-status-running = Läuft
trader-status-stopped = Gestoppt
trader-status-setup-required = Einrichtung erforderlich
trader-status-unavailable = Schließen Sie die Wallet- und RPC-Einrichtung ab, um den Auto-Trader zu nutzen
trader-toggle-on = AN
trader-toggle-off = AUS
trader-toggle-unavailable = NICHT VERFÜGBAR
trader-toggle-start-failed = Trader konnte nicht gestartet werden
trader-toggle-stop-failed = Trader konnte nicht gestoppt werden
trader-controls-title = Trading-Steuerung
trader-halt-title = TRADING ANGEHALTEN
trader-halt-reason-default = Manueller Sofortstopp
trader-halt-resume = Fortsetzen
trader-monitor-entry = Einstiegsmonitor
trader-monitor-exit = Ausstiegsmonitor
trader-monitor-master-off = Auto-Trader aus
trader-loss-limit-title = Verlustlimit pro Zeitraum
trader-loss-limit-resume = Trading fortsetzen
trader-loss-limit-reset = Zeitraum zurücksetzen
trader-loss-limit-off = Aus
trader-loss-limit-none = Kein Verlustlimit pro Zeitraum konfiguriert
trader-loss-limit-resets-in = Zurücksetzung in { $hours } { $minutes }
trader-loss-limit-reached = LIMIT ERREICHT
trader-force-stop = Alles sofort stoppen

trader-force-stop-confirm = Trading sofort stoppen
    .message = Dadurch werden ALLE Trading-Vorgänge sofort angehalten. Fortfahren?
    .confirm = Trading stoppen
trader-loss-limit-resume-confirm = Nach Verlustlimit fortsetzen
    .message = Das Verlustlimit des Zeitraums hat neue Einstiege gestoppt. Beim Fortsetzen kann der Trader wieder Positionen eröffnen, bevor der Zeitraum zurückgesetzt wird. Fortfahren?
trader-loss-limit-reset-confirm = Verlustlimit-Zeitraum zurücksetzen
    .message = Dadurch wird der aufgelaufene Verlust des aktuellen Zeitraums gelöscht und ein neuer Zeitraum begonnen. Fortfahren?

trader-toast-control-failed = Steuerung des Auto-Traders fehlgeschlagen
trader-toast-force-stop-on = Sofortstopp aktiviert
trader-toast-force-stop-failed = Sofortstopp konnte nicht aktiviert werden
trader-toast-force-stop-cleared = Sofortstopp aufgehoben
trader-toast-resume-failed = Trading konnte nicht fortgesetzt werden
trader-toast-loss-limit-reset-failed = Verlustlimit konnte nicht zurückgesetzt werden
trader-toast-entry-monitor-failed = Einstiegsmonitor konnte nicht umgeschaltet werden
trader-toast-exit-monitor-failed = Ausstiegsmonitor konnte nicht umgeschaltet werden
trader-toast-load-failed = Laden fehlgeschlagen
    .message = Trader-Konfiguration konnte nicht geladen werden
trader-toast-saved = Konfiguration gespeichert
    .message = Trader-Einstellungen erfolgreich angewendet
trader-toast-save-failed = Speichern fehlgeschlagen
    .message = Trader-Konfiguration konnte nicht gespeichert werden
trader-toast-feature-enabled = Funktion aktiviert
trader-toast-feature-disabled = Funktion deaktiviert
trader-toast-feature-applied = Auto-Trader-Einstellung angewendet
trader-toast-strategy-enabled = Strategie aktiviert
    .message = Strategie ist aktiv
trader-toast-strategy-disabled = Strategie deaktiviert
    .message = Strategie ist inaktiv
trader-toast-strategy-failed = Aktualisierung fehlgeschlagen
    .message = Strategiestatus konnte nicht aktualisiert werden

trader-stats-window =
    .aria-label = Statistikzeitraum
trader-stats-window-day = 24 Std.
trader-stats-window-week = 7 T.
trader-stats-window-month = 30 T.
trader-realized-title = Realisierte Performance
trader-metric-net-pnl = Netto-GuV
trader-metric-win-rate = Trefferquote
trader-metric-profit-factor = Profit-Faktor
trader-metric-max-drawdown = Max. Drawdown
trader-metric-capital = Eingesetztes Kapital
trader-metric-avg-win-loss = Ø Gewinn / Verlust
trader-metric-closed-trades = Geschlossene Trades
trader-metric-median-hold = Median-Haltedauer
trader-stats-empty = Keine geschlossenen Trades in diesem Zeitraum
trader-stats-won-lost = { $won } Gewinn · { $lost } Verlust
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } Gewinn
       *[other] { $amount } Gewinne
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } Verlust
       *[other] { $amount } Verluste
    }
trader-stats-expected = { $amount } erwartet pro Trade
trader-stats-profit-factor-basis = Bruttogewinn ÷ Bruttoverlust
trader-stats-drawdown-basis = Tiefster realisierter Rückgang von Hoch zu Tief
trader-stats-slots =
    { $count ->
        [one] { $used } von { $max } Positionsplatz belegt
       *[other] { $used } von { $max } Positionsplätzen belegt
    }
trader-stats-avg-basis = Durchschnittliches Ergebnis eines Gewinn- vs. Verlust-Trades
trader-stats-closed =
    { $count ->
        [one] { $amount } Position geschlossen
       *[other] { $amount } Positionen geschlossen
    }
trader-stats-hold-average = Ø { $span }
trader-stats-excluded =
    { $count ->
        [one] { $amount } geschlossene Runde ausgeschlossen – keine vollständige Einstandsbasis, daher keine belastbare GuV.
       *[other] { $amount } geschlossene Runden ausgeschlossen – keine vollständige Einstandsbasis, daher keine belastbare GuV.
    }

trader-daily-title = Tägliche GuV
trader-daily-subtitle = Realisierte { -sol } pro Tag, mit laufender Summe
trader-daily-loading = Tägliche GuV wird geladen...
trader-daily-chart = Täglicher realisierter Gewinn und Verlust in { -sol }
trader-extreme-best = Bester Trade
trader-extreme-worst = Schlechtester Trade

trader-exit-title = Aufschlüsselung der Ausstiegsstrategien
trader-exit-subtitle = Wie Positionen geschlossen wurden und was jeder Ausstieg eingebracht hat
trader-exit-loading = Ausstiegsdaten werden geladen...
trader-exit-empty-day = Keine geschlossenen Trades in den letzten 24 Stunden
trader-exit-empty-days =
    { $count ->
        [one] Keine geschlossenen Trades in den letzten { $amount } Tag
       *[other] Keine geschlossenen Trades in den letzten { $amount } Tagen
    }
trader-exit-share =
    { $count ->
        [one] { $amount } Trade · { $share } der Ausstiege
       *[other] { $amount } Trades · { $share } der Ausstiege
    }
trader-exit-average = Ø { $value }

trader-impact-label = Auswirkung:
trader-current-label = Aktuell:
trader-readable-label = Lesbar:
trader-example-how-it-works = So funktioniert es
trader-step-entry = Einstieg
trader-step-initial-position = Anfangsposition
trader-step-auto-exit = Auto-Ausstieg
trader-step-exit = Ausstieg
trader-step-full-exit = Vollständiger Positionsausstieg
trader-value-percent = { $value } %
trader-example-profit = +{ $value } % Gewinn

trader-stop-loss-title = Stop-Loss
trader-stop-loss-subtitle = Position automatisch verlassen, wenn der Verlust Ihre Schwelle überschreitet
trader-stop-loss-impact = Ausstieg bei { $threshold } % Minus gegenüber dem Einstieg
trader-stop-loss-hold-immediate = Sofort
trader-stop-loss-hold-delay = { $span } Verzögerung
trader-stop-loss-price-falls = Preis fällt
trader-stop-loss-threshold-reached = Schwelle erreicht
trader-stop-loss-partial = Teilausstiege erlaubt
trader-stop-loss-summary = Verlust begrenzt auf <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Hinweis:</strong> Der Stop-Loss schützt durch frühen Ausstieg vor größeren Verlusten

trader-trailing-title = Trailing-Stop
trader-trailing-subtitle = Gewinne automatisch sichern, indem der Stop dem steigenden Preis folgt
trader-trailing-activation-impact = Beginnt bei +{ $value } % Gewinn nachzuziehen
trader-trailing-distance-impact = Ausstieg bei -{ $value } % vom Hoch
trader-trailing-activation = Aktivierung
trader-trailing-peak = Hoch
trader-trailing-final = +{ $value } % am Ende
trader-trailing-summary-protected = <strong>{ $value }</strong> Gewinn gesichert
trader-trailing-summary-avoided = <strong>{ $value }</strong> Verlust vom Hoch vermieden

trader-roi-title = Take Profit
trader-roi-subtitle = Die gesamte Position automatisch verlassen, wenn der Gewinn Ihr Ziel erreicht
trader-roi-impact = Ausstieg bei +{ $target } % Gewinn
trader-roi-example-title = Beispielszenario
trader-roi-initial-buy = Erstkauf
trader-roi-target-hit = Ziel erreicht
trader-roi-full-position = Gesamte Position
trader-roi-sold = 100 % verkauft
trader-roi-summary = <strong>+{ $target } %</strong> Gewinn gesichert

trader-time-title = Zeitbasierter Ausstieg
trader-time-subtitle = Positionen nach einer maximalen Haltedauer automatisch verlassen, wenn der Verlust die Schwelle überschreitet
trader-time-unit-seconds = Sekunden
trader-time-unit-minutes = Minuten
trader-time-unit-hours = Stunden
trader-time-unit-days = Tage
trader-time-conversion-default = 168 Stunden = 7 Tage
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } Sekunde
       *[other] { $amount } Sekunden
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } Minute
       *[other] { $amount } Minuten
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } Stunde
       *[other] { $amount } Stunden
    }
trader-duration-days =
    { $count ->
        [one] { $amount } Tag
       *[other] { $amount } Tage
    }
trader-time-loss-impact = Ausstieg bei { $value } % Minus oder mehr nach der Haltedauer
trader-time-day = Tag { $day }
trader-time-position-opened = Position eröffnet
trader-time-limit = Zeitlimit
trader-time-hold-reached = Haltedauer erreicht
trader-time-loss-met = Verlustschwelle erreicht
trader-time-note = <strong>Hinweis:</strong> Positionen im Gewinn oder mit kleineren Verlusten werden NICHT verlassen
trader-time-positions-title = Status der aktuellen Positionen
trader-time-positions-loading = Positionen werden geladen...
trader-time-positions-empty = Keine offenen Positionen
trader-time-positions-token = Token
trader-time-positions-hold = Haltedauer
trader-time-positions-roi = ROI

trader-strategy-entry-title = Einstiegsstrategien
trader-strategy-entry-subtitle = Signale, die eine neue Position eröffnen können.
trader-strategy-exit-title = Ausstiegsstrategien
trader-strategy-exit-subtitle = Signale, die eine offene Position schließen oder schützen können.
trader-strategy-active-unknown = -- aktiv
trader-strategy-active = { $enabled }/{ $total } aktiv
trader-strategy-loading = Strategien werden geladen...
trader-strategy-load-failed = Strategien konnten nicht geladen werden
trader-strategy-empty = Keine Strategien definiert
trader-strategy-no-description = Keine Beschreibung vorhanden.
trader-strategy-unnamed = Unbenannte Strategie
trader-strategy-priority-auto = Auto
trader-strategy-priority = Priorität { $priority }

trader-dca-title = Durchschnittskosten-Strategie (DCA)
trader-dca-subtitle = Verlustpositionen automatisch aufstocken, um den durchschnittlichen Einstiegspreis zu senken
trader-dca-example-title = DCA-Beispiel
trader-dca-example = 0,01 { -sol } Anfang → DCA #1: 0,005 { -sol } @ -10 % → DCA #2: 0,005 { -sol } @ weitere -10 %
trader-dca-info-title = Infos zur DCA-Strategie
trader-dca-info-subtitle = Wichtige Hinweise zum DCA-Trading
trader-dca-how-title = So funktioniert DCA
trader-dca-how-trigger = <strong>Auslöser:</strong> Die Position fällt unter die DCA-Schwelle (z. B. -10 %)
trader-dca-how-action = <strong>Aktion:</strong> Mehr { -sol } nachkaufen, um die durchschnittliche Einstandsbasis zu senken
trader-dca-how-repeat = <strong>Wiederholung:</strong> DCA kann bis zur maximalen Anzahl mehrfach erfolgen
trader-dca-risk-title = Risikohinweise
trader-dca-risk-exposure = <strong>Höheres Engagement:</strong> DCA erhöht das insgesamt riskierte Kapital pro Position
trader-dca-risk-knife = <strong>Fallendes Messer:</strong> DCA hilft nicht, wenn der Token weiter im Abwärtstrend bleibt
trader-dca-risk-cooldown = <strong>Abkühlzeit:</strong> Nutzen Sie die Abkühlzeit, um schnell aufeinanderfolgende DCA-Einstiege zu vermeiden

trader-sizing-title = Positionsgröße
trader-sizing-subtitle = Steuern, wie viel pro Position investiert wird
trader-timing-title = Timing und Abkühlzeiten
trader-timing-subtitle = Zeitabstände zwischen Vorgängen steuern
trader-timing-close-cooldown = Abkühlzeit nach Positionsschluss
trader-timing-close-cooldown-hint = Minuten Wartezeit, bevor derselbe Token erneut eröffnet wird
trader-timing-concurrency = Parallelität der Einstiegsprüfung
trader-timing-concurrency-hint = Anzahl der Token, die gleichzeitig geprüft werden (höher = schneller, aber mehr CPU)
trader-timing-unit-minutes = Min.
trader-timing-unit-tokens = Tokens
