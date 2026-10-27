# Strategies page: the strategy list, the condition editor and the condition catalog.

## Strategy list

strategies-filter-all = Todas
strategies-filter-entry = Entrada
strategies-filter-exit = Salida
strategies-type-entry = Entrada
strategies-type-exit = Salida
strategies-list-empty-title = Aún no hay estrategias
strategies-list-empty-hint = Crea tu primera estrategia
strategies-new = Nueva estrategia
strategies-import =
    .title = Importar estrategia
    .aria-label = Importar estrategia
strategies-item-enable =
    .title = Activar
strategies-item-disable =
    .title = Desactivar

# Name given to a strategy before it is saved.
strategies-new-name = Nueva estrategia

## Editor

strategies-editor-name =
    .placeholder = Nombre de la estrategia
strategies-editor-dirty =
    .title = Cambios sin guardar
strategies-action-validate = Validar
strategies-editor-empty = Selecciona una estrategia para editarla o crea una nueva
strategies-conditions-empty-title = Aún no hay condiciones
strategies-conditions-empty-hint = Usa "{ strategies-add-condition }" para empezar a construir
strategies-add-condition = Añadir condición
strategies-modal-close =
    .aria-label = Cerrar
strategies-card-move-up =
    .title = Subir
strategies-card-move-down =
    .title = Bajar
strategies-card-duplicate =
    .title = Duplicar
strategies-card-delete =
    .title = Eliminar
# $name is the condition name.
strategies-card-delete-confirm = Quitar condición
    .message = ¿Quitar «{ $name }» de esta estrategia?

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = Sin parámetros
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Ajuste de la estrategia ({ $value })
strategies-summary-period-seconds = Período de revisión: { $amount } s
strategies-summary-period-minutes = Período de revisión: { $amount } min
strategies-summary-period-hours = Período de revisión: { $amount } h

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } hora
        [many] { $amount } horas
       *[other] { $amount } horas
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } vela
        [many] { $amount } velas
       *[other] { $amount } velas
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = h
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = Buscar condiciones...
    .aria-label = Buscar condiciones
strategies-catalog-search-clear =
    .aria-label = Borrar búsqueda
strategies-catalog-no-description = Sin descripción disponible

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Activa
strategies-editor-timeframe = Temporalidad predeterminada
    .title = Temporalidad de las velas para las condiciones que no definen la suya
strategies-editor-priority = Prioridad
    .title = Orden de evaluación del 1 al 100; los números menores se evalúan primero
strategies-editor-description = Descripción
strategies-editor-description-field =
    .placeholder = Qué hace esta estrategia
    .aria-label = Descripción de la estrategia
strategies-action-export = Exportar

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Condiciones que deben cumplirse: { $mode }
strategies-group-match = Deben cumplirse: { $mode }
strategies-match-all = todas
strategies-match-any = al menos una
strategies-match-mode =
    .aria-label = Todas o al menos una condición
strategies-invert = Invertir
    .title = Invertir: se cumple cuando esto no coincide
strategies-chip-off = Desactivado
strategies-chip-inverted = Invertido
strategies-row-switch =
    .title = Desactivar para conservarlo guardado pero sin evaluarlo
    .aria-label = Evaluado por esta estrategia
strategies-row-expand =
    .aria-label = Mostrar u ocultar los parámetros
strategies-add-group = Añadir grupo
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Quitar grupo
    .message =
        { $count ->
            [one] ¿Quitar este grupo y su condición?
            [many] ¿Quitar este grupo y sus { $count } condiciones?
           *[other] ¿Quitar este grupo y sus { $count } condiciones?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } problema por resolver: { $first }
        [many] { $count } problemas por resolver: { $first }
       *[other] { $count } problemas por resolver: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Añade al menos una condición activada
strategies-problem-group-empty = Un grupo necesita al menos una condición activada
strategies-problem-not-applicable = Esta condición no se puede usar en este tipo de estrategia
strategies-problem-unknown-condition = Tipo de condición desconocido: { $type }
strategies-problem-too-deep = Las reglas admiten como máximo { $max } niveles de anidación; un grupo invertido cuenta como un nivel más
strategies-problem-required = Introduce un número
strategies-problem-range = Debe estar entre { $min } y { $max }
strategies-problem-invalid-option = Elige una de las opciones disponibles

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Todas
strategies-catalog-categories =
    .aria-label = Categorías
strategies-catalog-empty = Ninguna condición coincide con la búsqueda
strategies-catalog-exit-only = Solo estrategias de salida
strategies-catalog-entry-only = Solo estrategias de entrada
strategies-catalog-candles-note = { $description } Solo lee velas.
strategies-data-candles = Velas
strategies-data-pool = Pool
strategies-data-position = Posición
strategies-data-market = Actividad del mercado

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = trades
strategies-value-trades =
    { $count ->
        [one] { $amount } trade
        [many] { $amount } trades
       *[other] { $amount } trades
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Demasiados niveles
    .message = Las reglas admiten como máximo { $max } niveles de anidación
strategies-toast-priority-invalid = Prioridad no válida
    .message = Introduce un número entero del 1 al 100
strategies-toast-exported = Estrategia exportada
# $name is the strategy name.
strategies-unsaved-confirm = ¿Descartar los cambios sin guardar?
    .message = «{ $name }» tiene cambios sin guardar.
strategies-unsaved-discard = Descartar
strategies-import-error-json = El archivo no es un JSON válido
strategies-import-error-shape = El archivo no describe una estrategia
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = El archivo usa una condición que esta versión no tiene: { $type }
strategies-import-error-not-applicable = { $name } no se puede usar en este tipo de estrategia

## New strategy dialog

strategies-create-title = Crear nueva estrategia
strategies-create-prompt = Elige el tipo de estrategia que quieres crear:
strategies-create-entry-name = Estrategia de entrada
strategies-create-entry-description = Define las condiciones para comprar un token
strategies-create-exit-name = Estrategia de salida
strategies-create-exit-description = Define las condiciones para vender un token

## Delete dialog

strategies-delete-title = Eliminar estrategia
# $name is the strategy name.
strategies-delete-message = ¿Eliminar la estrategia "{ $name }"? Esta acción no se puede deshacer.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Corrige los errores de validación antes de guardar
strategies-toast-load-failed = Error al cargar
    .message = No se pudieron cargar las estrategias desde el servidor
strategies-toast-load-strategy-failed = No se pudo cargar la estrategia
strategies-toast-no-strategy = No hay ninguna estrategia creada
    .message = Añade al menos una condición o haz clic en 'Nueva estrategia' para crear una primero
strategies-toast-name-required = Nombre obligatorio
    .message = Introduce un nombre para la estrategia antes de guardar
strategies-toast-saved = Estrategia guardada
    .message = "{ $name }" guardada correctamente
strategies-toast-save-failed = Error al guardar
    .message = No se pudo guardar la estrategia en la base de datos
strategies-toast-no-strategy-validate = No hay estrategia que validar
strategies-toast-valid = La estrategia es válida
strategies-toast-invalid = La estrategia tiene errores
strategies-toast-validation-failed = Falló la validación
strategies-toast-item-enabled = Estrategia activada
strategies-toast-item-disabled = Estrategia desactivada
strategies-toast-item-toggle-failed = No se pudo cambiar el estado de la estrategia
strategies-toast-deleted = Estrategia eliminada
    .message = "{ $name }" eliminada correctamente
strategies-toast-delete-failed = Error al eliminar
    .message = No se pudo eliminar la estrategia de la base de datos
strategies-toast-imported = Estrategia importada
strategies-toast-import-failed = No se pudo importar la estrategia
strategies-toast-unknown-condition = Condición desconocida
    .message = No se encontró el tipo de condición
strategies-toast-create-first = Crea una estrategia primero
    .message = Haz clic en 'Nueva estrategia' para crear una antes de añadir condiciones

## Conditions

strategies-condition-candle-size = Patrón de tamaño de vela
    .description = Detecta patrones de vela específicos: cuerpo grande, cuerpo pequeño (doji), mechas largas.
strategies-condition-candle-size-param-pattern = Tipo de patrón
    .description = Patrón de vela a detectar
strategies-condition-candle-size-param-pattern-option-large-body = Cuerpo grande (movimiento fuerte)
strategies-condition-candle-size-param-pattern-option-small-body = Cuerpo pequeño (doji/indecisión)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Mecha superior larga (rechazo)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Mecha inferior larga (soporte)
strategies-condition-candle-size-param-threshold = Umbral de tamaño %
    .description = Umbral porcentual para la detección del patrón

strategies-condition-consecutive-candles = Velas consecutivas
    .description = Detecta velas verdes (alcistas) o rojas (bajistas) consecutivas con un filtro de tamaño mínimo.
strategies-condition-consecutive-candles-param-count = Número de velas
    .description = Cantidad de velas consecutivas requeridas
strategies-condition-consecutive-candles-param-direction = Dirección de las velas
    .description = Color/dirección de las velas consecutivas
strategies-condition-consecutive-candles-param-direction-option-green = Verde (alcista)
strategies-condition-consecutive-candles-param-direction-option-red = Roja (bajista)
strategies-condition-consecutive-candles-param-minimum-change = Cambio mínimo %
    .description = Cambio mínimo en % de cada vela (filtra el ruido)

strategies-condition-liquidity-level = Nivel de liquidez del pool
    .description = Comprueba la liquidez del pool en { -sol } (Entrada: asegura liquidez suficiente; Salida: detecta el drenaje de liquidez)
strategies-condition-liquidity-level-param-threshold = Umbral de liquidez ({ -sol })
    .description = Nivel de liquidez del pool en { -sol }
strategies-condition-liquidity-level-param-comparison = Comparación
    .description = Cómo comparar la liquidez del pool con el umbral
strategies-condition-liquidity-level-param-comparison-option-greater-than = Mayor que (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Mayor o igual (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Menor que ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Menor o igual (≤)

strategies-condition-position-holding-time = Tiempo de tenencia de la posición
    .description = Comprueba cuánto tiempo lleva abierta una posición
strategies-condition-position-holding-time-param-hours = Umbral de tiempo (horas)
    .description = Duración en horas desde que se abrió la posición
strategies-condition-position-holding-time-param-comparison = Comparación
    .description = Cómo comparar la antigüedad de la posición con el umbral
strategies-condition-position-holding-time-param-comparison-option-greater-than = Más antigua que (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Al menos (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Más reciente que ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Como máximo (≤)

strategies-condition-price-breakout = Ruptura de precio
    .description = Detecta cuando el precio rompe por encima de la resistencia (máximo del período) o por debajo del soporte (mínimo del período).
strategies-condition-price-breakout-param-lookback = Período de revisión
    .description = Número de velas para hallar el nivel de soporte/resistencia
strategies-condition-price-breakout-param-direction = Dirección de la ruptura
    .description = Dirección de la ruptura
strategies-condition-price-breakout-param-direction-option-upward = Alcista (ruptura de resistencia)
strategies-condition-price-breakout-param-direction-option-downward = Bajista (ruptura de soporte)
strategies-condition-price-breakout-param-confirmation = Confirmación %
    .description = Cuánto debe superar el nivel para confirmar la ruptura (evita señales falsas)

strategies-condition-price-change-percent = Variación de precio %
    .description = Comprueba si el precio varió según un umbral porcentual dentro de un período de tiempo.
strategies-condition-price-change-percent-param-percentage = Umbral de variación %
    .description = Variación porcentual de precio que activa la condición (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Dirección
    .description = Dirección del movimiento del precio
strategies-condition-price-change-percent-param-direction-option-above = Ganancia (+%)
strategies-condition-price-change-percent-param-direction-option-below = Pérdida (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dentro del rango (±%)
strategies-condition-price-change-percent-param-time-value = Período de revisión
    .description = Hasta dónde se mide el cambio, en la unidad elegida dentro del campo (1-3600 s, 1-1440 min, 1-720 h)
strategies-condition-price-change-percent-param-time-unit = Unidad de tiempo
    .description = Unidad de tiempo del período de revisión
strategies-condition-price-change-percent-param-time-unit-option-seconds = s
strategies-condition-price-change-percent-param-time-unit-option-minutes = min
strategies-condition-price-change-percent-param-time-unit-option-hours = h

strategies-condition-price-to-ma = Precio vs. media móvil
    .description = Comprueba si el precio está por encima, por debajo o dentro del rango de su media móvil simple.
strategies-condition-price-to-ma-param-period = Período de la MA
    .description = Número de velas para calcular la media móvil
strategies-condition-price-to-ma-param-position = Posición
    .description = Posición del precio respecto a la MA
strategies-condition-price-to-ma-param-position-option-above = Por encima de la MA
strategies-condition-price-to-ma-param-position-option-below = Por debajo de la MA
strategies-condition-price-to-ma-param-position-option-within = Dentro del rango
strategies-condition-price-to-ma-param-distance = Distancia %
    .description = Distancia mínima a la MA (para ENCIMA/DEBAJO) o rango máximo (para DENTRO)

strategies-condition-volume-spike = Pico de volumen
    .description = Detecta picos de volumen frente al volumen promedio (indica mayor interés).
strategies-condition-volume-spike-param-lookback = Período de revisión
    .description = Número de velas para calcular el volumen promedio
strategies-condition-volume-spike-param-multiplier = Multiplicador de volumen
    .description = Cuántas veces por encima del promedio (p. ej., 2.0 = 200% del promedio)

strategies-condition-rsi = RSI (índice de fuerza relativa)
    .description = Se activa cuando el RSI, con suavizado de Wilder sobre velas cerradas, está por encima o por debajo de un umbral.
strategies-condition-rsi-param-period = Período del RSI
    .description = Número de velas sobre las que se calcula el RSI; la condición necesita una vela más que este valor
strategies-condition-rsi-param-comparison = Comparación
    .description = Si el RSI debe estar por debajo o por encima del umbral
strategies-condition-rsi-param-comparison-option-below = Por debajo (sobrevendido)
strategies-condition-rsi-param-comparison-option-above = Por encima (sobrecomprado)
strategies-condition-rsi-param-threshold = Umbral del RSI
    .description = Nivel de RSI de 1 a 99 (30 y 70 son los niveles habituales de sobreventa y sobrecompra)

strategies-condition-ma-cross = Cruce de medias móviles
    .description = Se activa cuando una media móvil rápida cruza una lenta dentro de las últimas velas.
strategies-condition-ma-cross-param-ma-type = Tipo de media
    .description = Las medias exponenciales dan más peso a las velas recientes; las simples pesan todas las velas por igual
strategies-condition-ma-cross-param-ma-type-option-ema = Exponencial (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Simple (SMA)
strategies-condition-ma-cross-param-fast-period = Período rápido
    .description = Número de velas de la media rápida
strategies-condition-ma-cross-param-slow-period = Período lento
    .description = Número de velas de la media lenta; debe ser mayor que el período rápido
strategies-condition-ma-cross-param-direction = Dirección del cruce
    .description = En qué sentido la media rápida cruza la lenta
strategies-condition-ma-cross-param-direction-option-bullish = Alcista (la rápida cruza hacia arriba)
strategies-condition-ma-cross-param-direction-option-bearish = Bajista (la rápida cruza hacia abajo)
strategies-condition-ma-cross-param-within-candles = Cruce en las últimas velas
    .description = Cuántas de las últimas velas pueden contener el cruce; 1 significa solo la última vela

strategies-condition-position-profit = Beneficio de la posición
    .description = Se activa cuando el beneficio no realizado de la posición abierta está por encima o por debajo de un porcentaje del precio de entrada, medido con el precio del pool.
strategies-condition-position-profit-param-comparison = Comparación
    .description = Si el beneficio debe ser como mínimo o como máximo el umbral
strategies-condition-position-profit-param-comparison-option-above = Al menos (≥)
strategies-condition-position-profit-param-comparison-option-below = Como máximo (≤)
strategies-condition-position-profit-param-threshold-pct = Umbral de beneficio %
    .description = Beneficio no realizado como porcentaje del precio de entrada; los valores negativos son pérdidas

strategies-condition-drawdown-from-peak = Caída desde el máximo
    .description = Se activa cuando el precio del pool ha caído un porcentaje desde el precio más alto visto desde la última compra, una vez que ese máximo estuvo lo bastante por encima del precio de entrada.
strategies-condition-drawdown-from-peak-param-drop-pct = Caída desde el máximo %
    .description = Cuánto por debajo del máximo debe estar el precio actual
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Activar tras beneficio %
    .description = El máximo debe haber estado al menos este porcentaje por encima del precio de entrada antes de que la condición pueda activarse; 0 significa siempre activa

strategies-condition-buy-pressure = Presión compradora
    .description = Se activa cuando las compras suponen al menos una parte de las transacciones de una ventana de tiempo y la ventana tiene suficientes transacciones. Lee los recuentos de compras y ventas del token.
strategies-condition-buy-pressure-param-window = Ventana de tiempo
    .description = Período que cubren los recuentos de compras y ventas
strategies-condition-buy-pressure-param-window-option-5m = 5 minutos
strategies-condition-buy-pressure-param-window-option-1h = 1 hora
strategies-condition-buy-pressure-param-window-option-6h = 6 horas
strategies-condition-buy-pressure-param-window-option-24h = 24 horas
strategies-condition-buy-pressure-param-min-buy-share-pct = Cuota mínima de compras %
    .description = Compras como porcentaje de todas las transacciones de la ventana
strategies-condition-buy-pressure-param-min-transactions = Transacciones mínimas
    .description = Las ventanas con menos transacciones nunca activan la condición

## Shared by every condition

strategies-condition-param-timeframe = Temporalidad
    .description = Tamaño de vela: la duración de cada vela que lee la condición (la temporalidad de la estrategia si no se define)
strategies-condition-timeframe-option-1m = 1 minuto
strategies-condition-timeframe-option-5m = 5 minutos
strategies-condition-timeframe-option-15m = 15 minutos
strategies-condition-timeframe-option-1h = 1 hora
strategies-condition-timeframe-option-4h = 4 horas
strategies-condition-timeframe-option-12h = 12 horas
strategies-condition-timeframe-option-1d = 1 día

## Condition categories

strategies-condition-category-price-analysis = Análisis de precio
strategies-condition-category-candle-patterns = Patrones de velas
strategies-condition-category-technical-indicators = Indicadores técnicos
strategies-condition-category-market-context = Contexto de mercado
strategies-condition-category-position-performance = Posición y rendimiento
strategies-condition-category-volume-analysis = Análisis de volumen

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = Falta el parámetro { $field }
strategies-error-parameter-type = El parámetro { $field } debe ser { $expected }
strategies-error-invalid-value = "{ $value }" no es un valor válido para { $field }
strategies-error-missing-data = { $data } no está disponible
strategies-error-no-candle-data = La temporalidad { $timeframe } no tiene datos de velas
strategies-error-insufficient-history = Historial insuficiente para { $indicator }: { $available } s disponibles, { $required } s necesarios
strategies-error-insufficient-candles = Velas insuficientes para { $indicator }: hay { $available }, se necesitan { $required }
strategies-error-stale-candle-data = Los datos de velas de { $timeframe } están desactualizados: su antigüedad de { $age } s supera { $max } s
strategies-error-stale-market-data = Los recuentos de transacciones de { $window } están desactualizados: su antigüedad de { $age } s supera { $max } s
strategies-error-unusable-data = El valor "{ $value }" de { $field } no se puede usar
strategies-error-invalid-rule-tree = Árbol de reglas no válido: { $reason }
strategies-error-evaluation-timeout = La evaluación de la estrategia agotó el tiempo tras { $timeout } ms
strategies-error-condition-not-applicable = { $condition } no se puede usar en estrategias de tipo { $strategy }
strategies-error-invalid-rules = No se pudieron leer las reglas: { $reason }

# Parameter names

strategies-error-field-average-volume = volumen promedio
strategies-error-field-candle-open = apertura de la vela
strategies-error-field-comparison = comparación
strategies-error-field-condition-type = tipo de condición
strategies-error-field-confirmation = confirmación
strategies-error-field-count = cantidad
strategies-error-field-current-price = precio actual
strategies-error-field-direction = dirección
strategies-error-field-distance = distancia
strategies-error-field-hours = horas
strategies-error-field-lookback = período de revisión
strategies-error-field-minimum-change = cambio mínimo
strategies-error-field-multiplier = multiplicador
strategies-error-field-pattern = patrón
strategies-error-field-percentage = porcentaje
strategies-error-field-period = período
strategies-error-field-position = posición
strategies-error-field-threshold = umbral
strategies-error-field-time-unit = unidad de tiempo
strategies-error-field-time-value = valor de tiempo
strategies-error-field-timeframe = temporalidad
strategies-error-field-arm-after-profit = activación tras beneficio
strategies-error-field-drop-percentage = porcentaje de caída
strategies-error-field-entry-price = precio de entrada
strategies-error-field-fast-period = período rápido
strategies-error-field-minimum-buy-share = cuota mínima de compras
strategies-error-field-minimum-transactions = transacciones mínimas
strategies-error-field-moving-average-type = tipo de media
strategies-error-field-moving-average = media móvil
strategies-error-field-reference-candle-close = cierre de la vela de referencia
strategies-error-field-peak-price = precio máximo
strategies-error-field-slow-period = período lento
strategies-error-field-window = ventana
strategies-error-field-within-candles = velas del cruce

# Expected parameter types

strategies-error-expected-boolean = un booleano
strategies-error-expected-number = un número
strategies-error-expected-string = una cadena de texto

# Missing context data

strategies-error-data-current-price = El precio actual
strategies-error-data-liquidity-data = Los datos de liquidez
strategies-error-data-market-data = Los datos de mercado
strategies-error-data-ohlcv-data = Los datos OHLCV
strategies-error-data-position-data = Los datos de la posición
strategies-error-data-peak-price = El precio máximo
strategies-error-data-transaction-counts = Los recuentos de transacciones

# Indicators

strategies-error-indicator-consecutive-candles = velas consecutivas
strategies-error-indicator-moving-average = media móvil
strategies-error-indicator-price-breakout = ruptura de precio
strategies-error-indicator-price-change-lookback = período de revisión de variación de precio
strategies-error-indicator-volume-spike = pico de volumen
strategies-error-indicator-moving-average-cross = cruce de medias móviles
strategies-error-indicator-rsi = RSI

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Al nodo de rama le faltan condiciones
strategies-error-rule-branch-node-missing-operator = Al nodo de rama le falta el operador
strategies-error-rule-branch-node-must-have-at-least-one-child = El nodo de rama debe tener al menos un hijo
strategies-error-rule-invalid-rule-tree-structure = Estructura del árbol de reglas no válida
strategies-error-rule-leaf-node-missing-condition = Al nodo hoja le falta la condición
strategies-error-rule-not-operator-must-have-exactly-one-child = El operador NOT debe tener exactamente un hijo
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Un grupo necesita al menos una condición activada
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = El operador NOT no puede tener un hijo desactivado
strategies-error-rule-rules-nest-deeper-than-four-levels = Las reglas no pueden anidarse más de cuatro niveles
strategies-error-rule-the-root-group-cannot-be-switched-off = El grupo raíz no se puede desactivar
