# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = Sin decimales en la base de datos
filtering-reject-token-too-new = Token demasiado nuevo
filtering-reject-cooldown-filtered = Filtrado por enfriamiento
filtering-reject-dex-data-missing = Faltan datos de { -dexscreener }
filtering-reject-gecko-data-missing = Faltan datos de { -geckoterminal }
filtering-reject-rug-data-missing = Faltan datos de { -rugcheck }
filtering-reject-onchain-numeric-symbol = Símbolo solo numérico (estafa)
filtering-reject-onchain-empty-symbol = Símbolo vacío (estafa)
filtering-reject-onchain-suspicious-symbol = Símbolo sospechoso (estafa)
filtering-reject-onchain-known-scam-authority = Autoridad de estafa conocida
filtering-reject-onchain-immutable-with-freeze = Inmutable + autoridad de congelación (estafa)
filtering-reject-onchain-high-risk-score = Puntuación de riesgo alta en la cadena
filtering-reject-dex-empty-name = Nombre vacío
filtering-reject-dex-empty-symbol = Símbolo vacío
filtering-reject-dex-empty-logo = URL del logo vacía
filtering-reject-dex-empty-website = URL del sitio web vacía
filtering-reject-dex-txn-5m = Pocas transacciones en 5m
filtering-reject-dex-txn-1h = Pocas transacciones en 1h
filtering-reject-dex-zero-liq = Liquidez cero
filtering-reject-dex-liq-low = Liquidez demasiado baja
filtering-reject-dex-liq-high = Liquidez demasiado alta
filtering-reject-dex-mcap-low = Capitalización de mercado demasiado baja
filtering-reject-dex-mcap-high = Capitalización de mercado demasiado alta
filtering-reject-dex-vol-low = Volumen demasiado bajo
filtering-reject-dex-vol-missing = Falta el volumen
filtering-reject-dex-fdv-low = FDV demasiado baja
filtering-reject-dex-fdv-high = FDV demasiado alta
filtering-reject-dex-vol5m-low = Volumen de 5m demasiado bajo
filtering-reject-dex-vol5m-missing = Falta el volumen de 5m
filtering-reject-dex-vol1h-low = Volumen de 1h demasiado bajo
filtering-reject-dex-vol1h-missing = Falta el volumen de 1h
filtering-reject-dex-vol6h-low = Volumen de 6h demasiado bajo
filtering-reject-dex-vol6h-missing = Falta el volumen de 6h
filtering-reject-dex-price-change-5m-low = Variación de precio de 5m demasiado baja
filtering-reject-dex-price-change-5m-high = Variación de precio de 5m demasiado alta
filtering-reject-dex-price-change-low = Variación de precio demasiado baja
filtering-reject-dex-price-change-high = Variación de precio demasiado alta
filtering-reject-dex-price-change-6h-low = Variación de precio de 6h demasiado baja
filtering-reject-dex-price-change-6h-high = Variación de precio de 6h demasiado alta
filtering-reject-dex-price-change-24h-low = Variación de precio de 24h demasiado baja
filtering-reject-dex-price-change-24h-high = Variación de precio de 24h demasiado alta
filtering-reject-gecko-liq-low = Liquidez demasiado baja
filtering-reject-gecko-liq-high = Liquidez demasiado alta
filtering-reject-gecko-mcap-low = Capitalización de mercado demasiado baja
filtering-reject-gecko-mcap-high = Capitalización de mercado demasiado alta
filtering-reject-gecko-vol5m-low = Volumen de 5m demasiado bajo
filtering-reject-gecko-vol5m-missing = Falta el volumen de 5m
filtering-reject-gecko-vol1h-low = Volumen de 1h demasiado bajo
filtering-reject-gecko-vol1h-missing = Falta el volumen de 1h
filtering-reject-gecko-vol24h-low = Volumen de 24h demasiado bajo
filtering-reject-gecko-vol24h-missing = Falta el volumen de 24h
filtering-reject-gecko-price-change-5m-low = Variación de precio de 5m demasiado baja
filtering-reject-gecko-price-change-5m-high = Variación de precio de 5m demasiado alta
filtering-reject-gecko-price-change-1h-low = Variación de precio de 1h demasiado baja
filtering-reject-gecko-price-change-1h-high = Variación de precio de 1h demasiado alta
filtering-reject-gecko-price-change-24h-low = Variación de precio de 24h demasiado baja
filtering-reject-gecko-price-change-24h-high = Variación de precio de 24h demasiado alta
filtering-reject-gecko-pool-count-low = Número de pools demasiado bajo
filtering-reject-gecko-pool-count-high = Número de pools demasiado alto
filtering-reject-gecko-pool-count-missing = Falta el número de pools
filtering-reject-gecko-reserve-low = Reserva demasiado baja
filtering-reject-gecko-reserve-missing = Falta la reserva
filtering-reject-rug-rugged = Token con rug pull
filtering-reject-rug-score = Puntuación de riesgo demasiado alta
filtering-reject-rug-level-danger = Nivel de riesgo peligroso
filtering-reject-rug-mint-authority = Autoridad mint presente
filtering-reject-rug-freeze-authority = Autoridad de congelación presente
filtering-reject-rug-top-holder = % del mayor holder demasiado alto
filtering-reject-rug-top3-holders = % de los 3 mayores holders demasiado alto
filtering-reject-rug-min-holders = Holders insuficientes
filtering-reject-rug-insider-count = Demasiados holders insiders
filtering-reject-rug-insider-pct = % de insiders demasiado alto
filtering-reject-rug-creator-pct = Saldo del creador demasiado alto
filtering-reject-rug-transfer-fee-present = Comisión de transferencia presente
filtering-reject-rug-transfer-fee-high = Comisión de transferencia demasiado alta
filtering-reject-rug-graph-insiders = Insiders del grafo demasiado altos
filtering-reject-rug-lp-providers-low = Proveedores de LP demasiado bajos
filtering-reject-rug-lp-providers-missing = Faltan proveedores de LP
filtering-reject-rug-lp-lock-low = Bloqueo de LP demasiado bajo
filtering-reject-rug-lp-lock-missing = Falta el bloqueo de LP
filtering-reject-llm-analysis-rejected = Análisis LLM rechazado: { $reason } ({ $confidence }% de confianza, { $provider })
filtering-reject-llm-analysis-rejected-generic = Análisis LLM rechazado
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = Falta la FDV
filtering-reject-dex-price-change-5m-missing = Falta la variación de precio de 5m
filtering-reject-dex-price-change-missing = Falta la variación de precio
filtering-reject-dex-price-change-6h-missing = Falta la variación de precio de 6h
filtering-reject-dex-price-change-24h-missing = Falta la variación de precio de 24h
filtering-reject-gecko-liq-missing = Falta la liquidez
filtering-reject-gecko-mcap-missing = Falta la capitalización de mercado
filtering-reject-gecko-price-change-5m-missing = Falta la variación de precio de 5m
filtering-reject-gecko-price-change-1h-missing = Falta la variación de precio de 1h
filtering-reject-gecko-price-change-24h-missing = Falta la variación de precio de 24h
filtering-reject-rug-transfer-fee-missing = Faltan datos de la comisión de transferencia

# Rejection categories used to group reasons.
filtering-reject-category-security = Problemas de seguridad
filtering-reject-category-distribution = Distribución de holders
filtering-reject-category-liquidity-lock = Problemas de bloqueo de LP
filtering-reject-category-fees = Comisiones de transferencia
filtering-reject-category-liquidity = Liquidez
filtering-reject-category-volume = Volumen de trading
filtering-reject-category-market-cap = Capitalización de mercado/FDV
filtering-reject-category-price-action = Movimiento de precio
filtering-reject-category-activity = Actividad de trading
filtering-reject-category-data-quality = Datos faltantes
filtering-reject-category-timing = Filtros de tiempo
filtering-reject-category-market = Datos de mercado
filtering-reject-category-other = Otros

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = Estado
filtering-tab-analytics = Analíticas
filtering-tab-explorer = Explorador
filtering-source-core = Núcleo
filtering-source-onchain = En la cadena
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = Análisis LLM

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = Todo
filtering-range-all-time = Todo el tiempo
filtering-range-custom = Personalizado
filtering-range-now = Ahora
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = Guardando cambios...
filtering-footer-refreshing = Actualizando instantánea...
filtering-footer-unsaved = Cambios sin guardar pendientes
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = Último guardado { $time }
filtering-footer-in-sync = Configuración sincronizada

## Info bar and status metrics

filtering-info-total = Total:
filtering-info-priced = Con precio:
filtering-info-passed = Aprobados:
filtering-info-positions = Posiciones:
filtering-info-blacklisted = En lista negra:
filtering-info-cache = Caché:
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = Creando…
filtering-refresh-never = Nunca

filtering-status-loading = Cargando estadísticas...
filtering-status-total = Total de tokens
filtering-status-total-detail = En la caché de filtrado
filtering-status-total-detail-building = Creando instantánea: los recuentos aparecerán en la próxima actualización
filtering-status-priced = Con precio
filtering-status-priced-detail = { $share } tienen precio
filtering-status-passed = Filtros aprobados
filtering-status-passed-detail = { $share } aprobados
filtering-status-positions = Posiciones abiertas
filtering-status-positions-detail = Operaciones activas
filtering-status-blacklisted = En lista negra
filtering-status-blacklisted-detail = Tokens marcados
filtering-status-ohlcv = Con OHLCV
filtering-status-ohlcv-detail = Datos históricos
filtering-status-refresh = Última actualización
filtering-status-refresh-building = Primera instantánea en curso
filtering-status-refresh-none = Aún sin actualizar
filtering-status-no-rejections = No hay datos de rechazos disponibles

## Analytics

filtering-analytics-loading = Cargando analíticas de { $range }…
filtering-analytics-scanned = Total analizado
# $time is a relative time such as "5m ago".
filtering-analytics-updated = Actualizado { $time }
filtering-analytics-passed = Tokens aprobados
filtering-analytics-pass-rate = <strong>{ $share }</strong> de aprobación
filtering-analytics-rejected = Tokens rechazados
filtering-analytics-rejection-rate = <strong>{ $share }</strong> de rechazo
filtering-analytics-by-category = Rechazos por categoría
filtering-analytics-by-source = Rechazos por fuente
filtering-analytics-no-category = Sin datos de categorías
filtering-analytics-no-source = Sin datos de fuentes
filtering-analytics-top-reasons = Principales motivos de rechazo
filtering-analytics-no-data = No hay datos disponibles
filtering-analytics-column-reason = Motivo
filtering-analytics-column-category = Categoría
filtering-analytics-column-count = Cantidad
filtering-analytics-column-share = %
filtering-analytics-column-impact = Impacto
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [one] { $amount } token
        [many] { $amount } tokens
       *[other] { $amount } tokens
    }

## Explorer

filtering-explorer-top-reasons = Principales motivos
filtering-explorer-recent = Rechazos recientes
filtering-explorer-none = Sin datos
filtering-explorer-none-recent = Sin recientes
filtering-explorer-search =
    .placeholder = Buscar motivos...
filtering-explorer-overview = Resumen
filtering-explorer-no-match = Ningún motivo coincide
filtering-explorer-column-token = Token
filtering-explorer-column-source = Fuente
filtering-explorer-column-time = Hora
filtering-explorer-page = Página { $page }
filtering-explorer-no-results = Sin resultados
filtering-explorer-empty = No se encontraron tokens
filtering-explorer-empty-filtered = No se encontraron tokens que coincidan con el filtro
filtering-explorer-load-failed = No se pudieron cargar los tokens

## Configuration panels

filtering-config-loading = Cargando configuración…
# $query is the text typed in the filter box.
filtering-config-no-match = Ningún parámetro coincide con «{ $query }»
filtering-config-no-parameters = Esta fuente no expone parámetros
# $source is the source name.
filtering-source-off = El filtrado de { $source } está desactivado: estos parámetros no se evalúan.
filtering-toolbar-filter =
    .placeholder = Filtrar parámetros
    .aria-label = Filtrar parámetros
filtering-toolbar-clear =
    .aria-label = Borrar filtro
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [one] { $amount } parámetro
        [many] { $amount } parámetros
       *[other] { $amount } parámetros
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [one] { $visible } de { $total } parámetro
        [many] { $visible } de { $total } parámetros
       *[other] { $visible } de { $total } parámetros
    }
filtering-group-enable =
    .aria-label = Activar las comprobaciones de { $group }
filtering-field-min = Mín.
filtering-field-max = Máx.
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = { $label } mínimo
filtering-field-max-aria =
    .aria-label = { $label } máximo
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = Restablecer al valor predeterminado ({ $default })
    .aria-label = Restablecer { $label } al valor predeterminado

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = Configuración guardada
    .message = Ajustes de filtrado guardados e instantánea actualizada
filtering-toast-save-failed = Error al guardar
    .message = No se pudo guardar la configuración de filtrado
filtering-toast-reset = Cambios restablecidos
    .message = Configuración restaurada al último estado guardado
filtering-toast-refresh-failed = Error al actualizar
    .message = No se pudo actualizar la instantánea de filtrado
filtering-toast-exported = Configuración exportada
    .message = Ajustes de filtrado guardados en un archivo
filtering-toast-imported = Configuración importada
    .message = Ajustes de filtrado cargados desde un archivo
filtering-toast-import-failed = Error al importar
    .message = No se pudo importar la configuración: formato de archivo no válido
filtering-toast-load-failed = Error al cargar
    .message = No se pudo cargar la configuración de filtrado
filtering-toast-range-missing = Selecciona las fechas de inicio y de fin
filtering-toast-range-order = La hora de inicio debe ser anterior a la de fin
filtering-toast-range-future = La hora de fin no puede estar en el futuro
