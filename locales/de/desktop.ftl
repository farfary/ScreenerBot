# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen.

## Actions shared by dialogs.

desktop-action-ok = OK

## Splash and loading status.

desktop-splash-starting = { -brand } wird gestartet
desktop-splash-restarting = { -brand } wird neu gestartet
desktop-splash-recovering = Wiederherstellung läuft
desktop-splash-opening-dashboard = Dashboard wird geöffnet
desktop-splash-checking-dependencies = Abhängigkeiten werden geprüft
desktop-splash-installing-dependencies = Systemabhängigkeiten werden installiert
desktop-splash-installing-dependencies-detail = { -brand } benötigt das Microsoft Visual C++ Redistributable, um zu laufen.
desktop-splash-resetting-wallet = Wallet-Daten werden zurückgesetzt
desktop-splash-resetting-wallet-detail = Die vorhandenen Wallet-Daten werden vor dem Löschen gesichert.
desktop-splash-updating = Update auf v{ $version }
desktop-splash-updating-detail = Ihre Einstellungen und Daten bleiben unverändert.
desktop-splash-restoring = v{ $version } wird wiederhergestellt
desktop-splash-restoring-detail = Update v{ $failed } wurde nicht gestartet, daher übernimmt die vorherige Version.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand } konnte nicht gestartet werden
desktop-boot-detail-fallback = Das Backend wurde unerwartet beendet.
desktop-boot-remedy-label = So beheben Sie das Problem
desktop-boot-log-file-label = Protokolldatei:
desktop-boot-action-reset-wallet = Wallet-Daten zurücksetzen und neu starten
desktop-boot-action-working = Wird ausgeführt...
desktop-boot-action-open-logs = Protokollordner öffnen
desktop-boot-action-copy = Details kopieren
desktop-boot-action-copied = Kopiert
desktop-boot-action-quit = Beenden
desktop-boot-subtitle-wallet-mismatch = Es wurde eine andere Wallet erkannt
desktop-boot-subtitle-port-in-use = Ein benötigter Netzwerkport ist belegt
desktop-boot-subtitle-lock-held = { -brand } läuft bereits
desktop-boot-subtitle-config-invalid = Konfigurationsproblem
desktop-boot-subtitle-directory-setup = Speicherproblem
desktop-boot-subtitle-storage-upgrade = Problem beim Datenbank-Upgrade
desktop-boot-subtitle-generic = Startfehler

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand } konnte nicht gestartet werden
desktop-boot-error-remedy = Öffnen Sie den Protokollordner, um zu sehen, was passiert ist, und starten Sie die App neu. Wenn das Problem weiterhin besteht, wenden Sie sich an den Support unter t.me/screenerbotio_support.
desktop-boot-error-default = Das Backend wurde unerwartet beendet, bevor das Dashboard bereit war.
desktop-boot-error-restore-failed = Das aktualisierte Backend ist fehlgeschlagen und die vorherige Version konnte nicht wiederhergestellt werden ({ $error }).
desktop-boot-error-spawn-failed = Das Backend-Programm konnte nicht gestartet werden ({ $error }).
desktop-boot-error-spawn-missing = Das Backend-Programm konnte nicht gestartet werden. Möglicherweise fehlt es oder wird von einer Sicherheitssoftware blockiert.
desktop-boot-error-exited-running = Das Backend wurde beendet, während das Dashboard lief (Exit-Code { $code }).
desktop-boot-error-exited-early = Das Backend wurde beendet, bevor das Dashboard bereit war (Exit-Code { $code }).
desktop-boot-error-dashboard-load = Das Dashboard konnte nicht geladen werden ({ $description }, { $code }).
desktop-boot-error-renderer-gone = Der Dashboard-Renderer wurde beendet ({ $reason }).
desktop-boot-error-unresponsive = Das Dashboard reagiert nicht mehr.
desktop-boot-error-url-failed = Die Dashboard-URL konnte nicht geladen werden ({ $error }).
desktop-boot-error-relaunch-setup = Das Backend konnte nach der Einrichtung nicht neu gestartet werden.
desktop-boot-error-relaunch-recovery = Das Backend konnte für die Wiederherstellung nicht neu gestartet werden.
desktop-boot-error-restart-offline = Das Backend ist nach dem Neustart nicht wieder online gegangen.
desktop-boot-error-recovery-offline = Die Wiederherstellung wurde abgeschlossen, aber das Backend wurde nicht bereit.
desktop-boot-error-start-timeout = Das Backend hat den Start nicht rechtzeitig abgeschlossen. Das kann beim ersten, langsamen Start passieren oder wenn ein anderes Programm die Verbindung blockiert.

## System tray.

desktop-tray-tooltip = { -brand } - Solana Trading Bot
desktop-tray-show = { -brand } anzeigen
desktop-tray-open-dashboard = Dashboard öffnen
desktop-tray-quit = { -brand } beenden

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = Datenordner öffnen
desktop-menu-open-logs-folder = Protokollordner öffnen
desktop-menu-documentation = Dokumentation
desktop-menu-telegram-support = { -telegram }-Support
desktop-menu-check-updates = Nach Updates suchen...

## Application menu.

desktop-menu-file = Datei
desktop-menu-edit = Bearbeiten
desktop-menu-view = Ansicht
desktop-menu-window = Fenster
desktop-menu-help = Hilfe
desktop-menu-reset-zoom = Zoom zurücksetzen
desktop-menu-zoom-in = Vergrößern
desktop-menu-zoom-out = Verkleinern
desktop-menu-keyboard-shortcuts = Tastenkürzel
desktop-menu-telegram-channel = { -telegram }-Kanal
desktop-menu-telegram-community = { -telegram }-Community
desktop-menu-follow-x = Auf { -x } ({ -twitter }) folgen
desktop-menu-visit-website = Website besuchen
desktop-menu-about = Über { -brand }

## About dialog.

desktop-about-title = Über { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Version { $version }

    Erweiterte Solana-Wallet-Verwaltung und Auto-Trading-Bot.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = Tastenkürzel
desktop-shortcuts-message = { -brand }-Tastenkürzel
desktop-shortcuts-body-mac =
    Tastenkürzel:

    Fenstersteuerung:
      Cmd+M          Minimieren
      Cmd+W          Fenster schließen
      Cmd+Q          Beenden
      Cmd+Ctrl+F     Vollbild umschalten

    Zoom:
      Cmd++          Vergrößern
      Cmd+-          Verkleinern
      Cmd+0          Zoom zurücksetzen

    Navigation:
      Cmd+R          Dashboard neu laden
      Cmd+Shift+D    Datenordner öffnen

    Sonstiges:
      F1             Dokumentation öffnen
      Cmd+Alt+I      DevTools umschalten
desktop-shortcuts-body-other =
    Tastenkürzel:

    Fenstersteuerung:
      Alt+F4         Beenden
      F11            Vollbild umschalten

    Zoom:
      Ctrl++         Vergrößern
      Ctrl+-         Verkleinern
      Ctrl+0         Zoom zurücksetzen

    Navigation:
      Ctrl+R         Dashboard neu laden
      Ctrl+Shift+D   Datenordner öffnen

    Sonstiges:
      F1             Dokumentation öffnen
      Ctrl+Shift+I   DevTools umschalten

## Close confirmation (Windows and Linux).

desktop-close-title = { -brand } schließen
desktop-close-message = Was möchten Sie tun?
desktop-close-detail = { -brand } kann im Hintergrund weiterlaufen. Der Trading-Bot überwacht und handelt weiter, während er im Infobereich minimiert ist.
desktop-close-minimize = In den Infobereich minimieren
desktop-close-quit = Vollständig beenden
desktop-close-cancel = Abbrechen

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = Fehlende Abhängigkeit
desktop-vcredist-missing-message = Visual C++ Redistributable fehlt
desktop-vcredist-missing-detail = { -brand } benötigt das Microsoft Visual C++ Redistributable, um zu laufen. Möchten Sie es jetzt installieren?
desktop-vcredist-install = Installieren und beheben
desktop-vcredist-exit = Beenden
desktop-vcredist-not-found-title = Installationsprogramm nicht gefunden
desktop-vcredist-not-found-message = { $name } konnte nicht korrekt gefunden werden.
desktop-vcredist-done-title = Installation abgeschlossen
desktop-vcredist-done-message = Abhängigkeiten erfolgreich installiert.
desktop-vcredist-done-detail = { -brand } wird jetzt gestartet.
desktop-vcredist-failed-title = Installation fehlgeschlagen
desktop-vcredist-failed-message = Bitte installieren Sie das Visual C++ Redistributable manuell.
