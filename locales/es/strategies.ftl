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
strategies-editor-enabled =
    .aria-label = Estrategia activada
    .title = Estrategia activada
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
strategies-summary-period-seconds = Período: { $amount } s
strategies-summary-period-minutes = Período: { $amount } min
strategies-summary-period-hours = Período: { $amount } h

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
strategies-catalog-search-clear =
    .aria-label = Borrar búsqueda
strategies-catalog-fold-all = Contraer todo
strategies-catalog-unfold-all = Expandir todo
strategies-catalog-no-description = Sin descripción disponible

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
strategies-toast-enabled = Estrategia activada
    .message = "{ $name }" activada
strategies-toast-disabled = Estrategia desactivada
    .message = "{ $name }" desactivada
strategies-toast-toggle-failed = Error al cambiar el estado
    .message = No se pudo actualizar el estado de la estrategia
strategies-toast-load-failed = Error al cargar
    .message = No se pudieron cargar las estrategias desde el servidor
strategies-toast-load-strategy-failed = No se pudo cargar la estrategia
strategies-toast-no-strategy = No hay ninguna estrategia creada
    .message = Añade al menos una condición o haz clic en 'Nueva estrategia' para crear una primero
strategies-toast-no-conditions-save = Sin condiciones
    .message = Añade al menos una condición a la estrategia antes de guardar
strategies-toast-name-required = Nombre obligatorio
    .message = Introduce un nombre para la estrategia antes de guardar
strategies-toast-saved = Estrategia guardada
    .message = "{ $name }" guardada correctamente
strategies-toast-save-failed = Error al guardar
    .message = No se pudo guardar la estrategia en la base de datos
strategies-toast-no-strategy-validate = No hay estrategia que validar
strategies-toast-no-conditions-validate = Sin condiciones
    .message = Añade al menos una condición antes de validar
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
strategies-toast-condition-added = Condición añadida
    .message = { $name } añadida a la estrategia


## Conditions

strategies-condition-candle-size = Patrón de tamaño de vela
    .description = Detecta patrones de vela específicos: cuerpo grande, cuerpo pequeño (doji), mechas largas
strategies-condition-candle-size-param-pattern = Tipo de patrón
    .description = Patrón de vela a detectar
strategies-condition-candle-size-param-pattern-option-large-body = Cuerpo grande (movimiento fuerte)
strategies-condition-candle-size-param-pattern-option-small-body = Cuerpo pequeño (doji/indecisión)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Mecha superior larga (rechazo)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Mecha inferior larga (soporte)
strategies-condition-candle-size-param-threshold = Umbral de tamaño %
    .description = Umbral porcentual para la detección del patrón

strategies-condition-consecutive-candles = Velas consecutivas
    .description = Detecta velas verdes (alcistas) o rojas (bajistas) consecutivas con un filtro de tamaño mínimo
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
    .description = Comprueba cuánto tiempo lleva abierta una posición (para estrategias de salida: salidas por tiempo)
strategies-condition-position-holding-time-param-hours = Umbral de tiempo (horas)
    .description = Duración en horas desde que se abrió la posición
strategies-condition-position-holding-time-param-comparison = Comparación
    .description = Cómo comparar la antigüedad de la posición con el umbral
strategies-condition-position-holding-time-param-comparison-option-greater-than = Más antigua que (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = Al menos (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Más reciente que ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = Como máximo (≤)

strategies-condition-price-breakout = Ruptura de precio
    .description = Detecta cuando el precio rompe por encima de la resistencia (máximo del período) o por debajo del soporte (mínimo del período)
strategies-condition-price-breakout-param-lookback = Período de revisión
    .description = Número de velas para hallar el nivel de soporte/resistencia
strategies-condition-price-breakout-param-direction = Dirección de la ruptura
    .description = Dirección de la ruptura
strategies-condition-price-breakout-param-direction-option-upward = Alcista (ruptura de resistencia)
strategies-condition-price-breakout-param-direction-option-downward = Bajista (ruptura de soporte)
strategies-condition-price-breakout-param-confirmation = Confirmación %
    .description = Cuánto debe superar el nivel para confirmar la ruptura (evita señales falsas)

strategies-condition-price-change-percent = Variación de precio %
    .description = Comprueba si el precio varió según un umbral porcentual dentro de un período de tiempo
strategies-condition-price-change-percent-param-percentage = Umbral de variación %
    .description = Variación porcentual de precio que activa la condición (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Dirección
    .description = Dirección del movimiento del precio
strategies-condition-price-change-percent-param-direction-option-above = Ganancia (+%)
strategies-condition-price-change-percent-param-direction-option-below = Pérdida (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dentro del rango (±%)
strategies-condition-price-change-percent-param-time-value = Período de tiempo
    .description = Valor del período de revisión (1-3600 para segundos, 1-1440 para minutos, 1-720 para horas)
strategies-condition-price-change-percent-param-time-unit = Unidad de tiempo
    .description = Unidad de tiempo del período de revisión
strategies-condition-price-change-percent-param-time-unit-option-seconds = Segundos
strategies-condition-price-change-percent-param-time-unit-option-minutes = Minutos
strategies-condition-price-change-percent-param-time-unit-option-hours = Horas

strategies-condition-price-to-ma = Precio vs. media móvil
    .description = Comprueba si el precio está por encima, por debajo o dentro del rango de su media móvil simple
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
    .description = Detecta picos de volumen frente al volumen promedio (indica mayor interés)
strategies-condition-volume-spike-param-lookback = Período de revisión
    .description = Número de velas para calcular el volumen promedio
strategies-condition-volume-spike-param-multiplier = Multiplicador de volumen
    .description = Cuántas veces por encima del promedio (p. ej., 2.0 = 200% del promedio)

## Shared by every condition

strategies-condition-param-timeframe = Temporalidad
    .description = Temporalidad de las velas a analizar (usa la de la estrategia si no se define)
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
strategies-error-invalid-rule-tree = Árbol de reglas no válido: { $reason }
strategies-error-evaluation-timeout = La evaluación de la estrategia agotó el tiempo tras { $timeout } ms
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

# Indicators

strategies-error-indicator-consecutive-candles = velas consecutivas
strategies-error-indicator-moving-average = media móvil
strategies-error-indicator-price-breakout = ruptura de precio
strategies-error-indicator-price-change-lookback = período de revisión de variación de precio
strategies-error-indicator-volume-spike = pico de volumen

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Al nodo de rama le faltan condiciones
strategies-error-rule-branch-node-missing-operator = Al nodo de rama le falta el operador
strategies-error-rule-branch-node-must-have-at-least-one-child = El nodo de rama debe tener al menos un hijo
strategies-error-rule-invalid-rule-tree-structure = Estructura del árbol de reglas no válida
strategies-error-rule-leaf-node-missing-condition = Al nodo hoja le falta la condición
strategies-error-rule-not-operator-must-have-exactly-one-child = El operador NOT debe tener exactamente un hijo
