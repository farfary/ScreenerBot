# Update system text.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = Die automatische Installation ist deaktiviert. Das Update ist bereit und wird angewendet, wenn Sie es wünschen.
updates-defer-trading-active = Eine Position, ein Trade oder ein Tool-Vorgang ist aktiv, daher wird der Neustart verschoben. Das Update wird automatisch angewendet, sobald die App inaktiv ist.
updates-defer-needs-installer = Diese Version aktualisiert auch die Desktop-Shell, daher muss das Installationsprogramm einmal ausgeführt werden.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

## Progress and outcome of update actions

updates-download-started = Update v{ $version } wird heruntergeladen...
updates-apply-started = Das Update wird installiert. { -brand } startet neu und verbindet sich automatisch wieder.
updates-install-opened = Das verifizierte Update-Installationsprogramm wurde geöffnet. Schließen Sie die Installation des Betriebssystems ab.

updates-installer-toast-title = Installationsprogramm geöffnet
updates-installer-toast-message = { -brand } wird jetzt sauber beendet.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = Status
updates-tab-release-notes = Versionshinweise
updates-tab-preferences = Einstellungen
updates-tab-sections = Update-Bereiche
updates-checking-installation = Diese Installation wird geprüft...

updates-phase-idle-headline = Bereit zur Update-Suche
updates-phase-idle-detail = { -brand } v{ $version } ist installiert.
updates-phase-up-to-date-headline = Sie sind auf dem neuesten Stand
updates-phase-up-to-date-detail = { -brand } v{ $version } ist die neueste Version.
updates-phase-checking-headline = Suche nach Updates
updates-phase-checking-detail = Die neueste veröffentlichte Version wird gesucht.
updates-phase-available-headline = Version { $version } ist verfügbar
updates-phase-downloading-headline = v{ $version } wird heruntergeladen
updates-phase-verifying-headline = v{ $version } wird verifiziert
updates-phase-verifying-detail = Der Download wird anhand seiner veröffentlichten Prüfsumme geprüft.
updates-phase-ready-to-apply-headline = Version { $version } ist bereit
updates-phase-ready-to-apply-detail = Das Update kann jetzt mit einem kurzen Neustart oder automatisch beim nächsten Start installiert werden.
updates-phase-ready-to-install-headline = Version { $version } ist bereit
updates-phase-ready-to-install-detail = Das Desktop-Installationsprogramm ist bereit, dieses Update abzuschließen.
updates-phase-applying-headline = Update wird installiert
updates-phase-applying-detail = { -brand } startet mit der neuen Version neu.
updates-phase-applied-headline = Auf v{ $version } aktualisiert
updates-phase-applied-detail = Das Update wurde installiert. Es ist nichts weiter erforderlich.
updates-phase-failed-headline = Das Update wurde nicht abgeschlossen
updates-phase-failed-detail = Versuchen Sie das Update erneut.
updates-phase-check-failed-headline = Update-Suche fehlgeschlagen
updates-phase-check-failed-detail = Der Release-Dienst war nicht erreichbar.
updates-status-unavailable-headline = Update-Status nicht verfügbar
updates-phase-unrecognized-detail = Der gemeldete Update-Status wird nicht erkannt.
updates-status-load-failed-detail = Der Installationsstatus konnte nicht geladen werden.

updates-kind-core = Kern-Update · { $size } · kurzer Neustart
updates-kind-full = Desktop-Update · { $size } · Installationsprogramm erforderlich
updates-size-unknown = unbekannte Größe

updates-action-check-now = Jetzt prüfen
updates-action-check-again = Erneut prüfen
updates-action-try-again = Erneut versuchen
updates-action-download = Update herunterladen
updates-action-restart = Zum Aktualisieren neu starten
updates-action-open-installer = Installationsprogramm öffnen

updates-busy-checking = Wird geprüft...
updates-busy-resuming = Download wird fortgesetzt...
updates-busy-starting-download = Download wird gestartet...
updates-busy-restarting = Neustart läuft...
updates-busy-opening-installer = Installationsprogramm wird geöffnet...

updates-progress-downloading = Update wird heruntergeladen
updates-progress-verifying = Update wird verifiziert
updates-progress-transferred = { $done } von { $total }
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Installationsdetails
updates-detail-installed-version = Installierte Version
updates-detail-system = System
updates-detail-last-checked = Zuletzt geprüft
updates-detail-never = Nie
updates-detail-available-version = Verfügbare Version
updates-detail-download-size = Download-Größe

updates-version-installed = Installiert
updates-version-available = Verfügbar

updates-notes-highlights = Highlights
updates-notes-empty-title = Noch keine Versionshinweise
updates-notes-empty-error = Der Versionsverlauf konnte nicht geladen werden. Prüfen Sie die Verbindung und versuchen Sie es erneut.
updates-notes-empty-none = Versionshinweise erscheinen hier, sobald eine Version veröffentlicht wurde.
updates-notes-history-notice = Angezeigt wird, was diese Installation bereits kennt — der Versionsverlauf konnte nicht geladen werden.
updates-release-empty = Für diese Version wurden keine Änderungen aufgeführt.
updates-release-changes =
    { $count ->
        [one] { $count } Änderung
       *[other] { $count } Änderungen
    }

updates-preferences-unavailable-title = Update-Einstellungen nicht verfügbar
updates-preferences-unavailable-detail = Die Update-Konfiguration konnte nicht geladen werden.
updates-preference-fallback-name = Update-Einstellung
updates-preference-save-failed = { $preference } konnte nicht gespeichert werden

updates-request-failed = Anfrage fehlgeschlagen
updates-check-request-failed = Update-Suche fehlgeschlagen
updates-resume-failed = Update-Download konnte nicht fortgesetzt werden
updates-download-failed = Update-Download konnte nicht gestartet werden
updates-apply-failed = Update konnte nicht installiert werden
updates-install-failed = Update-Installationsprogramm konnte nicht geöffnet werden
updates-apply-confirm-title = v{ $version } installieren
updates-apply-confirm-message = { -brand } startet mit der neuen Version neu. Das Trading pausiert für einige Sekunden und wird automatisch fortgesetzt; offene Positionen bleiben unberührt.
updates-install-confirm-title = Installationsprogramm ausführen
updates-install-confirm-message = Das verifizierte Installationsprogramm wird geöffnet und { -brand } wird sauber beendet. Schließen Sie die Installation ab und öffnen Sie { -brand } danach erneut.

updates-version-number = v{ $version }
