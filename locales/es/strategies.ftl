# Strategies page: the strategy list, the condition editor and the condition catalog.

## Strategy list

strategies-type-entry = Entrada
strategies-type-exit = Salida
strategies-list-empty-title = Aún no hay estrategias
strategies-list-empty-hint = Crea tu primera estrategia
strategies-new = Nueva estrategia
strategies-import =
    .title = Importar estrategia
    .aria-label = Importar estrategia

# Name given to a strategy before it is saved.
strategies-new-name = Nueva estrategia

## Editor

strategies-editor-name =
    .placeholder = Nombre de la estrategia
strategies-editor-dirty =
    .title = Cambios sin guardar
strategies-action-validate = Validar
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
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } s
strategies-value-duration-minutes = { $amount } min
strategies-value-duration-hours = { $amount } h
strategies-value-duration-days = { $amount } d
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = Tu hora local: { $time }
strategies-hint-price-impact = Una operación de este tamaño mueve el precio alrededor de un { $amount } %

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
strategies-unit-utc = UTC

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
strategies-editor-order = Orden
    .title = Orden de evaluación del 1 al 100; los números menores se evalúan primero
strategies-editor-description = Descripción
strategies-editor-description-field =
    .placeholder = Qué hace esta estrategia
    .aria-label = Descripción de la estrategia
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = Todas las estrategias de entrada
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = Gestiona
strategies-editor-entry-binding-hint = Gestiona las posiciones abiertas por las estrategias de entrada marcadas. Sin ninguna marcada gestiona todas las posiciones, incluidas las manuales y las copiadas.
strategies-editor-entry-binding-empty = Aún no hay ninguna estrategia de entrada
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (eliminada)
strategies-action-export = Exportar
strategies-action-test = Probar con un token
# $token is the tested token's symbol or address.
strategies-test-title = Prueba con { $token }
# $token is the tested token's symbol or address.
strategies-test-running = Probando con { $token }…
strategies-test-verdict-pass = Se cumple
strategies-test-verdict-fail = No se cumple
strategies-test-verdict-unknown = Desconocido

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
strategies-data-clock = Hora del día

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
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = La prioridad del archivo debe ser un número entero de { $min } a { $max }
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
strategies-toast-no-strategy-test = No hay ninguna estrategia que probar
strategies-toast-test-save-first = Guarda la estrategia antes de probarla: la prueba lee las reglas guardadas
strategies-toast-test-failed = Error en la prueba
    .message = No se pudo probar la estrategia
strategies-toast-valid = La estrategia es válida
strategies-toast-invalid = La estrategia tiene errores
strategies-toast-validation-failed = Falló la validación
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

strategies-condition-transaction-velocity = Velocidad de transacciones
    .description = Se activa cuando el ritmo de transacciones de una ventana reciente es al menos o como máximo un múltiplo del ritmo de la ventana más larga que la contiene. Lee los conteos de compras y ventas del token.
strategies-condition-transaction-velocity-param-window = Ventanas
    .description = La ventana reciente y la ventana más larga con la que se compara
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 minutos vs 1 hora
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 hora vs 6 horas
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 horas vs 24 horas
strategies-condition-transaction-velocity-param-comparison = Comparación
    .description = Si el ritmo reciente debe ser al menos o como máximo el ratio
strategies-condition-transaction-velocity-param-comparison-option-above = Al menos (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = Como máximo (≤)
strategies-condition-transaction-velocity-param-ratio = Ratio de ritmo
    .description = Transacciones por segundo de la ventana reciente divididas entre las de la ventana larga
strategies-condition-transaction-velocity-param-min-transactions = Transacciones mínimas
    .description = Las ventanas recientes con menos transacciones nunca activan la condición

strategies-condition-pullback-from-high = Retroceso desde el máximo
    .description = Se activa cuando el precio en vivo está entre una caída mínima y una máxima por debajo del máximo más alto de las últimas velas.
strategies-condition-pullback-from-high-param-lookback = Período de revisión
    .description = Número de velas de las que se toma el máximo más alto
strategies-condition-pullback-from-high-param-min-drop-pct = Caída mínima %
    .description = El precio debe estar al menos así de lejos por debajo del máximo
strategies-condition-pullback-from-high-param-max-drop-pct = Caída máxima %
    .description = El precio debe estar como máximo así de lejos por debajo del máximo; debe ser mayor que la caída mínima

strategies-condition-profit-giveback = Devolución de ganancias
    .description = Se activa cuando la posición ha devuelto una parte de su mejor ganancia no realizada desde la última compra, una vez que esa ganancia alcanzó un nivel.
strategies-condition-profit-giveback-param-arm-after-profit-pct = Armar tras ganancia %
    .description = La mejor ganancia debe haber alcanzado este porcentaje del precio de entrada antes de que la condición pueda activarse
strategies-condition-profit-giveback-param-giveback-pct = Devolución %
    .description = Parte de la mejor ganancia devuelta; 100 % significa que el precio volvió al de entrada

strategies-condition-token-age = Antigüedad del token
    .description = Se activa cuando el tiempo desde la creación on-chain del token es mayor o menor que una duración. Si la hora de creación es desconocida, la condición queda sin decidir.
strategies-condition-token-age-param-comparison = Comparación
    .description = Si el token debe ser más antiguo o más reciente que la duración
strategies-condition-token-age-param-comparison-option-older-than = Más antiguo que (≥)
strategies-condition-token-age-param-comparison-option-younger-than = Más reciente que ({ "<" })
strategies-condition-token-age-param-time-value = Antigüedad
    .description = Tiempo desde la creación del token, en la unidad elegida dentro del campo (1-10000)
strategies-condition-token-age-param-time-unit = Unidad de tiempo
    .description = Unidad de la antigüedad
strategies-condition-token-age-param-time-unit-option-minutes = min
strategies-condition-token-age-param-time-unit-option-hours = h
strategies-condition-token-age-param-time-unit-option-days = d

strategies-condition-vwap = VWAP
    .description = Se activa cuando el precio en vivo está por encima o por debajo del precio medio ponderado por volumen de las últimas velas por al menos una distancia.
strategies-condition-vwap-param-lookback = Período de revisión
    .description = Número de velas que cubre la media ponderada por volumen
strategies-condition-vwap-param-position = Posición
    .description = Posición del precio respecto al VWAP
strategies-condition-vwap-param-position-option-above = Por encima del VWAP
strategies-condition-vwap-param-position-option-below = Por debajo del VWAP
strategies-condition-vwap-param-distance-pct = Distancia %
    .description = Distancia mínima al VWAP; 0 significa cualquier distancia en el lado elegido

strategies-condition-volatility-range = Rango de volatilidad
    .description = Se activa cuando el rango verdadero medio de las últimas velas, como porcentaje del precio en vivo, está por encima o por debajo de un umbral. Dos de estas en un grupo forman un rango.
strategies-condition-volatility-range-param-period = Período ATR
    .description = Número de velas sobre las que se suaviza el rango verdadero medio; la condición necesita una vela más
strategies-condition-volatility-range-param-comparison = Comparación
    .description = Si el rango debe estar por encima o por debajo del umbral
strategies-condition-volatility-range-param-comparison-option-above = Por encima
strategies-condition-volatility-range-param-comparison-option-below = Por debajo
strategies-condition-volatility-range-param-threshold-pct = Umbral de rango %
    .description = Rango verdadero medio como porcentaje del precio en vivo

strategies-condition-bollinger-bands = Bandas de Bollinger
    .description = Se activa cuando el precio en vivo está por debajo de la banda inferior o por encima de la superior, o cuando las bandas se comprimen. Las bandas se sitúan a varias desviaciones estándar del cierre medio.
strategies-condition-bollinger-bands-param-period = Período
    .description = Número de velas que cubren la media y la desviación estándar
strategies-condition-bollinger-bands-param-std-dev = Desviaciones estándar
    .description = A cuántas desviaciones estándar de la media está cada banda
strategies-condition-bollinger-bands-param-signal = Señal
    .description = Lo que deben mostrar las bandas
strategies-condition-bollinger-bands-param-signal-option-below-lower = Bajo la banda inferior
strategies-condition-bollinger-bands-param-signal-option-above-upper = Sobre la banda superior
strategies-condition-bollinger-bands-param-signal-option-squeeze = Compresión
strategies-condition-bollinger-bands-param-squeeze-width-pct = Ancho de compresión %
    .description = Para una compresión: la distancia máxima entre las bandas, como porcentaje de la media

strategies-condition-liquidity-to-size = Liquidez frente al tamaño
    .description = Se activa cuando las reservas del pool son al menos o menos que un múltiplo del tamaño en juego: el tamaño de la operación en una estrategia de entrada, el valor restante de la posición en una de salida.
strategies-condition-liquidity-to-size-param-comparison = Comparación
    .description = Si las reservas deben ser al menos o menos que el múltiplo
strategies-condition-liquidity-to-size-param-comparison-option-at-least = Al menos (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = Menos de ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = Múltiplo de liquidez
    .description = Reservas del pool divididas entre el tamaño en juego; un múltiplo N mueve el precio alrededor de 1/(N+1)

strategies-condition-position-adds = Ampliaciones de posición
    .description = Se activa cuando el número de compras añadidas tras la primera entrada, o de ventas parciales, es al menos o como máximo un número.
strategies-condition-position-adds-param-subject = Contado
    .description = Qué acciones se cuentan
strategies-condition-position-adds-param-subject-option-adds = Compras añadidas
strategies-condition-position-adds-param-subject-option-partial-sells = Ventas parciales
strategies-condition-position-adds-param-comparison = Comparación
    .description = Si el conteo debe ser al menos o como máximo el número
strategies-condition-position-adds-param-comparison-option-at-least = Al menos (≥)
strategies-condition-position-adds-param-comparison-option-at-most = Como máximo (≤)
strategies-condition-position-adds-param-count = Cantidad
    .description = Número de acciones

strategies-condition-time-of-day = Hora del día
    .description = Se activa cuando la hora actual está dentro de una ventana de horas UTC. Un inicio posterior al fin pasa por la medianoche.
strategies-condition-time-of-day-param-start-hour = Hora de inicio (UTC)
    .description = Primera hora de la ventana, de 0 a 23
strategies-condition-time-of-day-param-end-hour = Hora de fin (UTC)
    .description = Hora en que termina la ventana, de 1 a 24; esa hora queda fuera de la ventana

strategies-condition-liquidity-change-since-entry = Cambio de liquidez desde la entrada
    .description = Se activa cuando las reservas del pool han bajado o subido al menos un porcentaje desde que se abrió la posición. Sin liquidez de entrada registrada, la condición queda sin decidir.
strategies-condition-liquidity-change-since-entry-param-comparison = Comparación
    .description = Si las reservas deben haber bajado o subido
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = Bajó un
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = Subió un
strategies-condition-liquidity-change-since-entry-param-change-pct = Cambio %
    .description = Cambio de las reservas del pool desde la entrada

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
strategies-error-field-change = cambio
strategies-error-field-end-hour = hora de fin
strategies-error-field-entry-liquidity = liquidez de entrada
strategies-error-field-giveback = devolución
strategies-error-field-highest-high = máximo más alto
strategies-error-field-long-window-transactions = transacciones de la ventana larga
strategies-error-field-maximum-drop = caída máxima
strategies-error-field-minimum-drop = caída mínima
strategies-error-field-multiple = múltiplo
strategies-error-field-pool-liquidity = liquidez del pool
strategies-error-field-ratio = ratio
strategies-error-field-remaining-position-value = valor restante de la posición
strategies-error-field-signal = señal
strategies-error-field-squeeze-width = ancho de compresión
strategies-error-field-standard-deviations = desviaciones estándar
strategies-error-field-start-hour = hora de inicio
strategies-error-field-subject = conteo
strategies-error-field-token-creation-time = hora de creación del token
strategies-error-field-total-volume = volumen total
strategies-error-field-trade-size = tamaño de la operación
strategies-error-field-vwap = VWAP

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
strategies-error-data-entry-liquidity = Liquidez de entrada
strategies-error-data-entry-pool-liquidity = Liquidez del pool de entrada
strategies-error-data-real-pool-reserves = Liquidez real del pool
strategies-error-data-remaining-position-value = Valor restante de la posición
strategies-error-data-token-creation-time = Hora de creación del token
strategies-error-data-trade-size = Tamaño de la operación

# Indicators

strategies-error-indicator-consecutive-candles = velas consecutivas
strategies-error-indicator-moving-average = media móvil
strategies-error-indicator-price-breakout = ruptura de precio
strategies-error-indicator-price-change-lookback = período de revisión de variación de precio
strategies-error-indicator-volume-spike = pico de volumen
strategies-error-indicator-moving-average-cross = cruce de medias móviles
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = bandas de Bollinger
strategies-error-indicator-pullback-from-high = retroceso desde el máximo
strategies-error-indicator-volatility-range = rango de volatilidad
strategies-error-indicator-vwap = VWAP

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

strategies-readiness-history-since-met =
    { $count ->
        [one] { $count } ronda simulada cerrada desde el cambio de reglas del { $changed }, se necesitan { $needed }
        [many] { $count } rondas simuladas cerradas desde el cambio de reglas del { $changed }, se necesitan { $needed }
       *[other] { $count } rondas simuladas cerradas desde el cambio de reglas del { $changed }, se necesitan { $needed }
    }
strategies-readiness-history-since-short = { $count } de { $needed } rondas simuladas cerradas desde el cambio de reglas del { $changed }
strategies-readiness-priced = Todas las posiciones con precio
strategies-readiness-priced-ok = Todas las posiciones simuladas abiertas tienen precio de pool
strategies-readiness-priced-missing =
    { $count ->
        [one] { $count } posición simulada abierta sin precio de pool
        [many] { $count } posiciones simuladas abiertas sin precio de pool
       *[other] { $count } posiciones simuladas abiertas sin precio de pool
    }
strategies-readiness-runtime = Trading en vivo disponible
strategies-readiness-runtime-ok = La configuración y los controles de seguridad permiten operaciones en vivo

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = Todas las estrategias
strategies-view-kind = Tipo de estrategia
strategies-view-all = Todas
strategies-search-placeholder = Buscar estrategias...
strategies-unnamed = Estrategia sin nombre
strategies-col-on = Activa
strategies-col-strategy = Estrategia
strategies-col-rank = Rango
    .title = Rango de evaluación entre las estrategias de entrada activas de un modo, primero el Orden más bajo; la primera que da señal toma el token
strategies-col-mode = Modo
strategies-col-status = Estado
strategies-col-open = Abiertas
strategies-col-watching = Vigiladas
strategies-col-kind = Tipo
strategies-col-rounds = Rondas
strategies-col-closed = Cerradas
strategies-col-win-rate = Tasa de acierto
strategies-col-realized = Realizado ({ -sol })
strategies-col-manages = Gestiona
strategies-col-last-signal = Última señal
strategies-status-running = En marcha
strategies-status-not-running = Detenida
strategies-status-off = Apagada
strategies-status-setup-required = Requiere configuración
strategies-status-live-paused = Entradas en vivo en pausa
strategies-status-paper-cash = Saldo simulado insuficiente
strategies-status-slots-full = Plazas llenas
strategies-status-entry-monitor-off = Monitor de entradas apagado
strategies-status-exit-monitor-off = Monitor de salidas apagado
strategies-notice-setup-required = El trading simulado empieza tras la configuración
strategies-notice-force-stopped = Parada de emergencia activada
strategies-notice-trader-off = Auto Trader está apagado
strategies-notice-waiting = Esperando los servicios principales
strategies-notice-monitors-off = Los monitores de entradas y salidas están apagados
strategies-notice-start = Iniciar Auto Trader
# $closed and $needed count closed paper rounds; $check is the title of a readiness check.
strategies-readiness-progress = { $closed }/{ $needed } rondas
strategies-readiness-ready = Lista para activar en vivo
strategies-readiness-failing = No se cumple: { $check }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
        [one] { $book }: { $count } posición
        [many] { $book }: { $count } posiciones
       *[other] { $book }: { $count } posiciones
    }
strategies-manages-all = Todas las estrategias de entrada
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
        [one] { $first } y { $more } más
        [many] { $first } y { $more } más
       *[other] { $first } y { $more } más
    }
strategies-exit-before = Se comprueban primero
strategies-exit-hint = Estas reglas de salida se comprueban en cada posición antes de cualquier estrategia de salida
strategies-action-edit = Editar { $name }
strategies-action-delete = Eliminar { $name }
strategies-roster-stale = Última actualización

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = Cambiar las reglas de “{ $name }” en vivo
    .message = Las nuevas reglas operan con { -sol } reales desde la próxima comprobación, y la evidencia simulada de esta estrategia se reinicia desde este guardado.
strategies-live-rules-save = Guardar y operar en vivo
strategies-toast-evidence-restarted = Estrategia guardada
    .message = Las reglas de “{ $name }” cambiaron; su evidencia simulada se reinicia desde ahora

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = Activar en vivo
strategies-mode-return-paper = Volver a simulación
strategies-mode-arm-title = Activar “{ $name }” en vivo
strategies-mode-arm-lead = Las nuevas entradas de “{ $name }” operarán con swaps reales desde tu billetera de trading.
strategies-mode-arm-lead-exit = “{ $name }” venderá posiciones en vivo con swaps reales desde tu wallet de trading cuando dé señal.
strategies-mode-arm-readiness = Preparación según la cartera simulada
strategies-mode-arm-open-paper = Las posiciones simuladas abiertas siguen siendo simuladas y se cierran en simulación.
strategies-mode-arm-ack-real-native = Las nuevas entradas gastan { -sol } real de tu billetera de trading.
strategies-mode-arm-ack-real-native-exit = Las posiciones en vivo que gestiona se venden con swaps reales.
strategies-mode-arm-ack-fees = Los swaps en vivo pagan comisiones de red y deslizamiento reales; los resultados simulados no garantizan resultados en vivo.
strategies-mode-arm-ack-unready = Algunas comprobaciones de preparación no se han superado. Activar esta estrategia de todos modos.
strategies-mode-arm-unavailable = El trading en vivo no está disponible ahora; consulta la última comprobación.
strategies-mode-return-title = Volver “{ $name }” a simulación
strategies-mode-return-message = Las nuevas entradas se abren en la cartera simulada sin gastar { -sol }. Las posiciones en vivo abiertas siguen en vivo y se cierran en vivo.
strategies-mode-return-message-exit = Deja de evaluar posiciones en vivo; estas conservan sus reglas de salida.
strategies-mode-readiness-failed = No se pudo leer la preparación de la estrategia
strategies-mode-toast-armed = { $name } ahora opera en vivo
strategies-mode-toast-paper = { $name } ahora opera en simulación
strategies-mode-toast-failed = No se pudo cambiar el modo de trading
