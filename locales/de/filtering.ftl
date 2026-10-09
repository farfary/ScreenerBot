# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = Keine Dezimalstellen in der Datenbank
filtering-reject-token-too-new = Token zu neu
filtering-reject-cooldown-filtered = Durch Abkühlzeit gefiltert
filtering-reject-dex-data-missing = { -dexscreener }-Daten fehlen
filtering-reject-gecko-data-missing = { -geckoterminal }-Daten fehlen
filtering-reject-rug-data-missing = { -rugcheck }-Daten fehlen
filtering-reject-onchain-numeric-symbol = Rein numerisches Symbol (Scam)
filtering-reject-onchain-empty-symbol = Leeres Symbol (Scam)
filtering-reject-onchain-suspicious-symbol = Verdächtiges Symbol (Scam)
filtering-reject-onchain-known-scam-authority = Bekannte Scam-Berechtigung
filtering-reject-onchain-immutable-with-freeze = Unveränderlich + Freeze-Berechtigung (Scam)
filtering-reject-onchain-high-risk-score = Hoher On-Chain-Risikoscore
filtering-reject-dex-empty-name = Leerer Name
filtering-reject-dex-empty-symbol = Leeres Symbol
filtering-reject-dex-empty-logo = Leere Logo-URL
filtering-reject-dex-empty-website = Leere Website-URL
filtering-reject-dex-txn-5m = Wenige Transaktionen (5 Min.)
filtering-reject-dex-txn-1h = Wenige Transaktionen (1 Std.)
filtering-reject-dex-zero-liq = Keine Liquidität
filtering-reject-dex-liq-low = Liquidität zu niedrig
filtering-reject-dex-liq-high = Liquidität zu hoch
filtering-reject-dex-mcap-low = Marktkapitalisierung zu niedrig
filtering-reject-dex-mcap-high = Marktkapitalisierung zu hoch
filtering-reject-dex-vol-low = Volumen zu niedrig
filtering-reject-dex-vol-missing = Volumen fehlt
filtering-reject-dex-fdv-low = FDV zu niedrig
filtering-reject-dex-fdv-high = FDV zu hoch
filtering-reject-dex-vol5m-low = 5-Min.-Volumen zu niedrig
filtering-reject-dex-vol5m-missing = 5-Min.-Volumen fehlt
filtering-reject-dex-vol1h-low = 1-Std.-Volumen zu niedrig
filtering-reject-dex-vol1h-missing = 1-Std.-Volumen fehlt
filtering-reject-dex-vol6h-low = 6-Std.-Volumen zu niedrig
filtering-reject-dex-vol6h-missing = 6-Std.-Volumen fehlt
filtering-reject-dex-price-change-5m-low = 5-Min.-Preisänderung zu niedrig
filtering-reject-dex-price-change-5m-high = 5-Min.-Preisänderung zu hoch
filtering-reject-dex-price-change-low = Preisänderung zu niedrig
filtering-reject-dex-price-change-high = Preisänderung zu hoch
filtering-reject-dex-price-change-6h-low = 6-Std.-Preisänderung zu niedrig
filtering-reject-dex-price-change-6h-high = 6-Std.-Preisänderung zu hoch
filtering-reject-dex-price-change-24h-low = 24-Std.-Preisänderung zu niedrig
filtering-reject-dex-price-change-24h-high = 24-Std.-Preisänderung zu hoch
filtering-reject-gecko-liq-low = Liquidität zu niedrig
filtering-reject-gecko-liq-high = Liquidität zu hoch
filtering-reject-gecko-mcap-low = Marktkapitalisierung zu niedrig
filtering-reject-gecko-mcap-high = Marktkapitalisierung zu hoch
filtering-reject-gecko-vol5m-low = 5-Min.-Volumen zu niedrig
filtering-reject-gecko-vol5m-missing = 5-Min.-Volumen fehlt
filtering-reject-gecko-vol1h-low = 1-Std.-Volumen zu niedrig
filtering-reject-gecko-vol1h-missing = 1-Std.-Volumen fehlt
filtering-reject-gecko-vol24h-low = 24-Std.-Volumen zu niedrig
filtering-reject-gecko-vol24h-missing = 24-Std.-Volumen fehlt
filtering-reject-gecko-price-change-5m-low = 5-Min.-Preisänderung zu niedrig
filtering-reject-gecko-price-change-5m-high = 5-Min.-Preisänderung zu hoch
filtering-reject-gecko-price-change-1h-low = 1-Std.-Preisänderung zu niedrig
filtering-reject-gecko-price-change-1h-high = 1-Std.-Preisänderung zu hoch
filtering-reject-gecko-price-change-24h-low = 24-Std.-Preisänderung zu niedrig
filtering-reject-gecko-price-change-24h-high = 24-Std.-Preisänderung zu hoch
filtering-reject-gecko-pool-count-low = Pool-Anzahl zu niedrig
filtering-reject-gecko-pool-count-high = Pool-Anzahl zu hoch
filtering-reject-gecko-pool-count-missing = Pool-Anzahl fehlt
filtering-reject-gecko-reserve-low = Reserve zu niedrig
filtering-reject-gecko-reserve-missing = Reserve fehlt
filtering-reject-rug-rugged = Token wurde gerugged
filtering-reject-rug-score = Risikoscore zu hoch
filtering-reject-rug-level-danger = Risikostufe „Gefahr“
filtering-reject-rug-mint-authority = Mint-Berechtigung vorhanden
filtering-reject-rug-freeze-authority = Freeze-Berechtigung vorhanden
filtering-reject-rug-top-holder = Anteil des größten Holders zu hoch
filtering-reject-rug-top3-holders = Anteil der Top-3-Holder zu hoch
filtering-reject-rug-min-holders = Zu wenige Holder
filtering-reject-rug-insider-count = Zu viele Insider-Holder
filtering-reject-rug-insider-pct = Insider-Anteil zu hoch
filtering-reject-rug-creator-pct = Creator-Guthaben zu hoch
filtering-reject-rug-transfer-fee-present = Transfergebühr vorhanden
filtering-reject-rug-transfer-fee-high = Transfergebühr zu hoch
filtering-reject-rug-graph-insiders = Graph-Insider zu hoch
filtering-reject-rug-lp-providers-low = LP-Anbieter zu wenige
filtering-reject-rug-lp-providers-missing = LP-Anbieter fehlen
filtering-reject-rug-lp-lock-low = LP-Sperre zu niedrig
filtering-reject-rug-lp-lock-missing = LP-Sperre fehlt
filtering-reject-llm-analysis-rejected = LLM-Analyse abgelehnt: { $reason } ({ $confidence } % Vertrauen, { $provider })
filtering-reject-llm-analysis-rejected-generic = LLM-Analyse abgelehnt
filtering-reject-unknown = { $code }

filtering-reject-dex-fdv-missing = FDV fehlt
filtering-reject-dex-price-change-5m-missing = 5-Min.-Preisänderung fehlt
filtering-reject-dex-price-change-missing = Preisänderung fehlt
filtering-reject-dex-price-change-6h-missing = 6-Std.-Preisänderung fehlt
filtering-reject-dex-price-change-24h-missing = 24-Std.-Preisänderung fehlt
filtering-reject-gecko-liq-missing = Liquidität fehlt
filtering-reject-gecko-mcap-missing = Marktkapitalisierung fehlt
filtering-reject-gecko-price-change-5m-missing = 5-Min.-Preisänderung fehlt
filtering-reject-gecko-price-change-1h-missing = 1-Std.-Preisänderung fehlt
filtering-reject-gecko-price-change-24h-missing = 24-Std.-Preisänderung fehlt
filtering-reject-rug-transfer-fee-missing = Daten zur Transfergebühr fehlen

filtering-reject-category-security = Sicherheitsprobleme
filtering-reject-category-distribution = Holder-Verteilung
filtering-reject-category-liquidity-lock = LP-Sperren-Probleme
filtering-reject-category-fees = Transfergebühren
filtering-reject-category-liquidity = Liquidität
filtering-reject-category-volume = Handelsvolumen
filtering-reject-category-market-cap = Marktkapitalisierung/FDV
filtering-reject-category-price-action = Preisbewegung
filtering-reject-category-activity = Handelsaktivität
filtering-reject-category-data-quality = Fehlende Daten
filtering-reject-category-timing = Zeitfilter
filtering-reject-category-market = Marktdaten
filtering-reject-category-other = Sonstige

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = Status
filtering-tab-analytics = Analysen
filtering-tab-explorer = Explorer
filtering-source-core = Kern
filtering-source-onchain = On-Chain
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = LLM-Analyse

## Time range

filtering-range-1h = 1 Std.
filtering-range-6h = 6 Std.
filtering-range-24h = 24 Std.
filtering-range-7d = 7 T.
filtering-range-all = Alle
filtering-range-all-time = Gesamter Zeitraum
filtering-range-custom = Benutzerdefiniert
filtering-range-now = Jetzt
filtering-range-span = { $start } → { $end }
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = Änderungen werden gespeichert...
filtering-footer-refreshing = Snapshot wird aktualisiert...
filtering-footer-unsaved = Ungespeicherte Änderungen ausstehend
filtering-footer-last-saved = Zuletzt gespeichert { $time }
filtering-footer-in-sync = Konfiguration synchron

## Info bar and status metrics

filtering-info-total = Gesamt
filtering-info-priced = Mit Preis
filtering-info-passed = Bestanden
filtering-info-positions = Positionen
filtering-info-blacklisted = Auf der Blacklist
filtering-info-cache = Cache
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = Wird erstellt…
filtering-refresh-never = Nie

filtering-status-loading = Statistiken werden geladen...
filtering-status-total = Token gesamt
filtering-status-total-detail = Im Filter-Cache
filtering-status-total-detail-building = Snapshot wird erstellt – Zahlen erscheinen bei der nächsten Aktualisierung
filtering-status-priced = Mit Preis
filtering-status-priced-detail = { $share } haben einen Preis
filtering-status-passed = Filter bestanden
filtering-status-passed-detail = { $share } bestanden
filtering-status-positions = Offene Positionen
filtering-status-positions-detail = Aktive Trades
filtering-status-blacklisted = Auf der Blacklist
filtering-status-blacklisted-detail = Markierte Token
filtering-status-ohlcv = Mit OHLCV
filtering-status-ohlcv-detail = Historische Daten
filtering-status-refresh = Letzte Aktualisierung
filtering-status-refresh-building = Erster Snapshot wird erstellt
filtering-status-refresh-none = Noch keine Aktualisierung
filtering-status-no-rejections = Keine Ablehnungsdaten verfügbar

## Analytics

filtering-analytics-loading = Analysen für { $range } werden geladen…
filtering-analytics-scanned = Insgesamt gescannt
filtering-analytics-updated = Aktualisiert { $time }
filtering-analytics-passed = Bestandene Token
filtering-analytics-pass-rate = <strong>{ $share }</strong> Bestehensquote
filtering-analytics-rejected = Abgelehnte Token
filtering-analytics-rejection-rate = <strong>{ $share }</strong> Ablehnungsquote
filtering-analytics-by-category = Ablehnungen nach Kategorie
filtering-analytics-by-source = Ablehnungen nach Quelle
filtering-analytics-no-category = Keine Kategoriedaten
filtering-analytics-no-source = Keine Quellendaten
filtering-analytics-top-reasons = Häufigste Ablehnungsgründe
filtering-analytics-no-data = Keine Daten verfügbar
filtering-analytics-column-reason = Grund
filtering-analytics-column-category = Kategorie
filtering-analytics-column-count = Anzahl
filtering-analytics-column-share = %
filtering-analytics-column-impact = Auswirkung
filtering-tokens-count =
    { $count ->
        [one] { $amount } Token
       *[other] { $amount } Token
    }

## Explorer

filtering-explorer-top-reasons = Häufigste Gründe
filtering-explorer-recent = Letzte Ablehnungen
filtering-explorer-none = Keine Daten
filtering-explorer-none-recent = Keine aktuellen
filtering-explorer-search =
    .placeholder = Gründe durchsuchen...
filtering-explorer-overview = Übersicht
filtering-explorer-no-match = Keine passenden Gründe
filtering-explorer-column-token = Token
filtering-explorer-column-source = Quelle
filtering-explorer-column-time = Zeit
filtering-explorer-page = Seite { $page }
filtering-explorer-no-results = Keine Ergebnisse
filtering-explorer-empty = Keine Token gefunden
filtering-explorer-empty-filtered = Keine Token gefunden, die dem Filter entsprechen
filtering-explorer-load-failed = Token konnten nicht geladen werden

## Configuration panels

filtering-config-loading = Konfiguration wird geladen…
filtering-config-no-match = Kein Parameter passt zu „{ $query }“
filtering-config-no-parameters = Diese Quelle stellt keine Parameter bereit
filtering-source-off = Die Filterung für { $source } ist aus – diese Parameter werden nicht ausgewertet.
filtering-toolbar-filter =
    .placeholder = Parameter filtern
    .aria-label = Parameter filtern
filtering-toolbar-clear =
    .aria-label = Filter löschen
filtering-parameter-count =
    { $count ->
        [one] { $amount } Parameter
       *[other] { $amount } Parameter
    }
filtering-parameter-count-filtered =
    { $count ->
        [one] { $visible } von { $total } Parameter
       *[other] { $visible } von { $total } Parametern
    }
filtering-group-enable =
    .aria-label = { $group }-Prüfungen aktivieren
filtering-field-min = Min.
filtering-field-max = Max.
filtering-field-min-aria =
    .aria-label = Minimum { $label }
filtering-field-max-aria =
    .aria-label = Maximum { $label }
filtering-field-reset =
    .title = Auf Standard zurücksetzen ({ $default })
    .aria-label = { $label } auf Standard zurücksetzen

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = Konfiguration gespeichert
    .message = Filtereinstellungen gespeichert und Snapshot aktualisiert
filtering-toast-save-failed = Speichern fehlgeschlagen
    .message = Filterkonfiguration konnte nicht gespeichert werden
filtering-toast-reset = Änderungen zurückgesetzt
    .message = Konfiguration auf den zuletzt gespeicherten Stand zurückgesetzt
filtering-toast-refresh-failed = Aktualisierung fehlgeschlagen
    .message = Filter-Snapshot konnte nicht aktualisiert werden
filtering-toast-exported = Konfiguration exportiert
    .message = Filtereinstellungen in Datei gespeichert
filtering-toast-imported = Konfiguration importiert
    .message = Filtereinstellungen aus Datei geladen
filtering-toast-import-failed = Import fehlgeschlagen
    .message = Konfiguration konnte nicht importiert werden – ungültiges Dateiformat
filtering-toast-load-failed = Laden fehlgeschlagen
    .message = Filterkonfiguration konnte nicht geladen werden
filtering-toast-range-missing = Bitte wählen Sie Start- und Enddatum aus
filtering-toast-range-order = Die Startzeit muss vor der Endzeit liegen
filtering-toast-range-future = Die Endzeit darf nicht in der Zukunft liegen
