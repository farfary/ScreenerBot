# Trader page labels.

trader-exit-type-stop-loss = Stop-Loss
trader-exit-type-take-profit = Take Profit
trader-exit-type-roi = Take Profit
trader-exit-type-roi-exit = Take Profit
trader-exit-type-trailing-stop = Trailing-Stop
trader-exit-type-time-override = Zeitausstieg
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
trader-tab-take-profit = Take Profit
trader-tab-time-rules = Zeitausstieg
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
trader-setup-action = Einrichtung abschließen
trader-toggle-on = AN
trader-toggle-off = AUS
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
trader-force-stop-hint = Hält alle Monitore, Kopieraufgaben und Trades sofort an.
trader-force-stop-idle = Es läuft nichts, das ein Notstopp anhalten würde.

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
trader-stop-loss-sell-part = { $part } verkaufen
trader-stop-loss-part-of-position = Teil der Position
trader-stop-loss-sell-rest = Rest verkaufen
trader-stop-loss-still-below = Weiter unter dem Stop
trader-stop-loss-summary = Verlust begrenzt auf <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Hinweis:</strong> Der Stop-Loss schützt durch frühen Ausstieg vor größeren Verlusten

trader-trailing-title = Trailing-Stop
trader-trailing-subtitle = Gewinne automatisch sichern, indem der Stop dem steigenden Preis folgt
trader-trailing-activation-impact = Beginnt bei +{ $value } % Gewinn nachzuziehen
trader-trailing-distance-impact = Ausstieg bei -{ $value } % vom Hoch
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Gesicherter Mindestgewinn beim Aktivieren: { $value }
trader-trailing-locked-minimum-warning = Der Stop liegt beim Aktivieren auf oder unter dem Einstieg; er verkauft nur im Gewinn. Schalte den Break-Even-Stop für eine Untergrenze ein.
trader-trailing-activation = Aktivierung
trader-trailing-peak = Hoch
trader-trailing-final = +{ $value } % am Ende
trader-trailing-summary-protected = <strong>{ $value }</strong> Gewinn gesichert
trader-trailing-summary-avoided = <strong>{ $value }</strong> Verlust vom Hoch vermieden

## Break-even stop

trader-break-even-title = Break-Even-Stop
trader-break-even-subtitle = Hat eine Position Gewinn gemacht, alles verkaufen, bevor daraus ein Verlust wird
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Sobald der beste Preis seit dem letzten Kauf { $trigger } erreicht, verkauft ein Rückfall auf { $offset } alles.

trader-take-profit-title = Take Profit
trader-take-profit-subtitle = Verkauft die Position in Stufen, sobald der Gewinn jede Schwelle erreicht
trader-take-profit-initial-buy = Erstkauf
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Stufe { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Gewinnschwelle von Stufe { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Verkaufsanteil von Stufe { $step }
trader-take-profit-profit-caption = Gewinnschwelle
trader-take-profit-sell-caption = Verkaufsanteil
trader-take-profit-add-step = Stufe hinzufügen
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Stufe { $step } entfernen
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Jede Gewinnschwelle muss zwischen { $min }% und { $max }% liegen
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Jeder Anteil muss zwischen { $min }% und { $max }% liegen
trader-take-profit-error-order = Jede Stufe braucht eine höhere Gewinnschwelle als die vorherige
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Die Stufen können zusammen höchstens { $max }% verkaufen
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% der Position bleiben als Runner offen
trader-take-profit-runner-none = Die letzte Stufe verkauft den Rest der Position
trader-take-profit-runner-warning = Sind Trailing-Stop, Break-Even-Stop, beide Zeitausstiege und alle Ausstiegsstrategien aus, schließt der Runner nur per Stop-Loss oder von Hand
trader-take-profit-info = Anteile beziehen sich auf die ursprüngliche Position. Überschreitet der Preis mehrere Schwellen auf einmal, verkauft pro Prüfung eine Stufe.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = Bei +{ $profit }% { $share }% verkaufen
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Den anderen Ausstiegsregeln überlassen
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Verkauft <strong>{ $sold }%</strong> bis +{ $profit }% Gewinn

trader-time-loss-title = Verlustpositionen schließen
trader-time-subtitle = Positionen nach einer maximalen Haltedauer automatisch verlassen, wenn der Verlust die Schwelle überschreitet
trader-time-flat-title = Seitwärtspositionen schließen
trader-time-flat-subtitle = Kapital aus Positionen freigeben, die sich weder nach oben noch nach unten bewegen
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Eine Position, die { $span } oder länger gehalten wird und deren GuV innerhalb von ±{ $band } liegt, wird geschlossen.
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
trader-time-loss-hint = Der Verlust, bei dem der Zeitausstieg nach Ablauf der Haltedauer verkauft (40 = Ausstieg bei -40 %)
trader-time-day = Tag { $day }
trader-time-position-opened = Position eröffnet
trader-time-limit = Zeitlimit
trader-time-hold-reached = Haltedauer erreicht
trader-time-loss-met = Verlustschwelle erreicht
trader-time-note = <strong>Hinweis:</strong> Positionen im Gewinn oder mit kleineren Verlusten werden NICHT verlassen
trader-time-positions-title = Status der aktuellen Positionen
trader-time-positions-loading = Positionen werden geladen...
trader-time-positions-empty = Keine offenen Positionen
trader-time-positions-empty-message = Offene Positionen erscheinen hier mit Haltedauer und ROI im Vergleich zum Zeitlimit.
trader-time-positions-token = Token
trader-time-positions-hold = Haltedauer
trader-time-positions-roi = ROI
trader-time-positions-rule = Zeitausstieg
trader-time-status-off = Zeitausstiege sind aus
trader-time-status-none = Derzeit greift kein Zeitausstieg
trader-time-status-not-managed = Nicht vom Auto-Trader verwaltet
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = Im Verlust { $pnl }; schließt in { $remaining }
trader-time-status-losing-due = Im Verlust { $pnl }; schließt bei der nächsten Prüfung
# $held and $remaining are formatted durations.
trader-time-status-flat = Seitwärts seit { $held }; schließt in { $remaining }
trader-time-status-flat-due = Seitwärts seit { $held }; schließt bei der nächsten Prüfung

trader-strategy-entry-title = Einstiegsstrategien
trader-strategy-entry-subtitle = Signale, die eine neue Position eröffnen können.
trader-strategy-exit-title = Ausstiegsstrategien
trader-strategy-exit-subtitle = Signale, die eine offene Position schließen oder schützen können.
trader-strategy-active-unknown = -- aktiv
trader-strategy-active = { $enabled }/{ $total } aktiv
trader-strategy-loading = Strategien werden geladen...
trader-strategy-load-failed = Strategien konnten nicht geladen werden
trader-strategy-empty = Keine Strategien definiert
trader-strategy-unnamed = Unbenannte Strategie
trader-strategy-priority-auto = Auto
trader-strategy-priority = Priorität { $priority }

trader-dca-title = Durchschnittskosten-Strategie (DCA)
trader-dca-subtitle = Verlustpositionen automatisch aufstocken, um den durchschnittlichen Einstiegspreis zu senken
trader-dca-measure-average = Durchschnittseinstieg
trader-dca-measure-first-entry = Erster Einstieg
trader-dca-step = Nachkauf { $step }
trader-dca-drop-label = Kursrückgang für Nachkauf { $step }
trader-dca-size-label = Größe von Nachkauf { $step } als Vielfaches des ersten Kaufs
trader-dca-drop-caption = Kursrückgang
trader-dca-size-caption = Kauf × erster Kauf
trader-dca-add-step = Stufe hinzufügen
trader-dca-remove-step = Nachkauf { $step } entfernen
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Unter dem Mindesthandel von { $minimum }: dieser Nachkauf wird nicht gesendet
trader-dca-error-drop-range = Jeder Rückgang muss zwischen { $min } % und { $max } % liegen
trader-dca-error-size-range = Jede Größe muss zwischen { $min }× und { $max }× des ersten Kaufs liegen
trader-dca-error-order = Gemessen ab dem ersten Einstieg braucht jeder Nachkauf einen tieferen Rückgang als der vorherige
trader-dca-cooldown-impact = Mindestens { $span } zwischen zusätzlichen Einstiegen
trader-dca-min-hold-impact =
    { $minutes ->
        [0] Der erste Nachkauf kann direkt nach dem Einstieg folgen
       *[other] Der erste Nachkauf wartet mindestens { $span } nach dem Einstieg
    }
trader-dca-capacity = Höchstens hält diese Position: { $amount } ({ $multiple }× der erste Kauf)
trader-dca-info-title = Infos zur DCA-Strategie
trader-dca-info-subtitle = Wichtige Hinweise zum DCA-Trading
trader-dca-how-title = So funktioniert DCA
trader-dca-how-trigger = <strong>Auslöser:</strong> Der Kurs fällt unter die nächste Stufe, gemessen ab dem Durchschnitts- oder dem ersten Einstieg
trader-dca-how-action = <strong>Aktion:</strong> Mehr { -sol } nachkaufen, um die durchschnittliche Einstandsbasis zu senken
trader-dca-how-repeat = <strong>Stufen:</strong> Jeder Nachkauf rückt zur nächsten Stufe vor; sind alle Stufen genutzt, enden die Nachkäufe
trader-dca-how-pause = <strong>Pausen:</strong> Kein Nachkauf, solange der vorherige noch abgewickelt wird oder das Verlustlimit neue Einstiege sperrt
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
