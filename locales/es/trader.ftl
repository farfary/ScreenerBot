# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Objetivo de ROI
trader-exit-type-roi-exit = Objetivo de ROI
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Anulación por tiempo
trader-exit-type-time-rule = Regla de tiempo
trader-exit-type-manual = Manual
trader-exit-type-manual-close = Manual
trader-exit-type-dca = DCA
trader-exit-type-unknown = Desconocido

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Estadísticas
trader-tab-strategy-control = Control de estrategias
trader-tab-strategies = Estrategias
trader-tab-stop-loss = Stop loss
trader-tab-trailing-stop = Trailing stop
trader-tab-roi = Take profit
trader-tab-time-rules = Reglas de tiempo
trader-tab-dca = DCA
trader-tab-settings = Configuración

## Feature status badges and their messages

trader-feature-coming-soon = Próximamente
    .message = Esta función llegará pronto y aún no está disponible.
trader-feature-beta = Beta
trader-feature-disabled = Desactivada
    .message = Esta función está desactivada actualmente.

## Status bar and trading controls

trader-status-title = Auto Trader
trader-status-loading = Cargando...
trader-status-running = En ejecución
trader-status-stopped = Detenido
trader-status-setup-required = Configuración requerida
trader-status-unavailable = Completa la configuración de billetera y RPC para usar Auto Trader
trader-toggle-on = ACTIVADO
trader-toggle-off = DESACTIVADO
trader-toggle-unavailable = NO DISPONIBLE
trader-toggle-start-failed = No se pudo iniciar el trader
trader-toggle-stop-failed = No se pudo detener el trader
trader-controls-title = Controles de trading
trader-halt-title = TRADING DETENIDO
trader-halt-reason-default = Detención forzada manual
trader-halt-resume = Reanudar
trader-monitor-entry = Monitor de entradas
trader-monitor-exit = Monitor de salidas
trader-monitor-master-off = Auto Trader desactivado
trader-loss-limit-title = Límite de pérdidas por período
trader-loss-limit-resume = Reanudar trading
trader-loss-limit-reset = Reiniciar período
trader-loss-limit-off = Desactivado
trader-loss-limit-none = No hay límite de pérdidas por período configurado
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = Se reinicia en { $hours } { $minutes }
trader-loss-limit-reached = LÍMITE ALCANZADO
trader-force-stop = Detener todo por la fuerza

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = Detención forzada del trading
    .message = Esto detendrá de inmediato TODAS las operaciones de trading. ¿Continuar?
    .confirm = Detener trading
trader-loss-limit-resume-confirm = Reanudar tras el límite de pérdidas
    .message = El límite de pérdidas del período detuvo las nuevas entradas. Al reanudar, el trader podrá abrir posiciones de nuevo antes de que el período se reinicie. ¿Continuar?
trader-loss-limit-reset-confirm = Reiniciar el período del límite de pérdidas
    .message = Esto borra la pérdida acumulada del período actual y empieza uno nuevo. ¿Continuar?

## Toasts

trader-toast-control-failed = Falló el control de Auto Trader
trader-toast-force-stop-on = Detención forzada activada
trader-toast-force-stop-failed = No se pudo activar la detención forzada
trader-toast-force-stop-cleared = Detención forzada desactivada
trader-toast-resume-failed = No se pudo reanudar el trading
trader-toast-loss-limit-reset-failed = No se pudo reiniciar el límite de pérdidas
trader-toast-entry-monitor-failed = No se pudo cambiar el monitor de entradas
trader-toast-exit-monitor-failed = No se pudo cambiar el monitor de salidas
trader-toast-load-failed = Error al cargar
    .message = No se pudo cargar la configuración del trader
trader-toast-saved = Configuración guardada
    .message = Los ajustes del trader se aplicaron correctamente
trader-toast-save-failed = Error al guardar
    .message = No se pudo guardar la configuración del trader
trader-toast-feature-enabled = Función activada
trader-toast-feature-disabled = Función desactivada
trader-toast-feature-applied = Ajuste de Auto Trader aplicado
trader-toast-strategy-enabled = Estrategia activada
    .message = La estrategia está activa
trader-toast-strategy-disabled = Estrategia desactivada
    .message = La estrategia está inactiva
trader-toast-strategy-failed = Error al actualizar
    .message = No se pudo actualizar el estado de la estrategia

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Ventana de estadísticas
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = Rendimiento realizado
trader-metric-net-pnl = P&L neto
trader-metric-win-rate = Tasa de aciertos
trader-metric-profit-factor = Factor de beneficio
trader-metric-max-drawdown = Drawdown máximo
trader-metric-capital = Capital en uso
trader-metric-avg-win-loss = Prom. ganancia / pérdida
trader-metric-closed-trades = Operaciones cerradas
trader-metric-median-hold = Tenencia mediana
trader-stats-empty = No hay operaciones cerradas en esta ventana
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = { $won } ganados · { $lost } perdidos
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } ganada
        [many] { $amount } ganadas
       *[other] { $amount } ganadas
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } perdida
        [many] { $amount } perdidas
       *[other] { $amount } perdidas
    }
# $amount is a formatted SOL amount.
trader-stats-expected = { $amount } esperados por operación
trader-stats-profit-factor-basis = Ganancia bruta ÷ pérdida bruta
trader-stats-drawdown-basis = Mayor caída realizada de pico a valle
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [one] { $used } de { $max } espacio de posición usado
        [many] { $used } de { $max } espacios de posición usados
       *[other] { $used } de { $max } espacios de posición usados
    }
trader-stats-avg-basis = Resultado promedio de una operación ganadora frente a una perdedora
trader-stats-closed =
    { $count ->
        [one] { $amount } posición cerrada
        [many] { $amount } posiciones cerradas
       *[other] { $amount } posiciones cerradas
    }
# $span is a formatted duration.
trader-stats-hold-average = { $span } en promedio
trader-stats-excluded =
    { $count ->
        [one] { $amount } ronda cerrada excluida: sin costo base completo, no hay un P&L fiable.
        [many] { $amount } rondas cerradas excluidas: sin costo base completo, no hay un P&L fiable.
       *[other] { $amount } rondas cerradas excluidas: sin costo base completo, no hay un P&L fiable.
    }

## Stats: daily P&L and extremes

trader-daily-title = P&L diario
trader-daily-subtitle = { -sol } realizados por día, con el total acumulado
trader-daily-loading = Cargando P&L diario...
trader-daily-chart = Ganancias y pérdidas realizadas diarias en { -sol }
trader-extreme-best = Mejor operación
trader-extreme-worst = Peor operación

## Stats: exit breakdown

trader-exit-title = Desglose de estrategias de salida
trader-exit-subtitle = Cómo se cerraron las posiciones y qué devolvió cada salida
trader-exit-loading = Cargando datos de salidas...
trader-exit-empty-day = No hay operaciones cerradas en las últimas 24 horas
trader-exit-empty-days =
    { $count ->
        [one] No hay operaciones cerradas en el último { $amount } día
        [many] No hay operaciones cerradas en los últimos { $amount } días
       *[other] No hay operaciones cerradas en los últimos { $amount } días
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [one] { $amount } operación · { $share } de las salidas
        [many] { $amount } operaciones · { $share } de las salidas
       *[other] { $amount } operaciones · { $share } de las salidas
    }
# $value is a formatted average percentage.
trader-exit-average = { $value } prom.

## Shared example vocabulary

trader-impact-label = Impacto:
trader-current-label = Actual:
trader-readable-label = Legible:
trader-example-how-it-works = Cómo funciona
trader-step-entry = Entrada
trader-step-initial-position = Posición inicial
trader-step-auto-exit = Salida automática
trader-step-exit = Salida
trader-step-full-exit = Salida total de la posición
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% de ganancia

## Stop loss

trader-stop-loss-title = Stop loss
trader-stop-loss-subtitle = Sale automáticamente de una posición cuando su pérdida supera tu umbral
# $threshold is the threshold as typed.
trader-stop-loss-impact = Sale cuando baja { $threshold }% desde la entrada
trader-stop-loss-hold-immediate = Inmediato
# $span is a formatted duration.
trader-stop-loss-hold-delay = Retraso de { $span }
trader-stop-loss-price-falls = El precio cae
trader-stop-loss-threshold-reached = Umbral alcanzado
trader-stop-loss-partial = Salidas parciales permitidas
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = Pérdida limitada a <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Nota:</strong> el stop loss protege de pérdidas mayores saliendo antes

## Trailing stop

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Protege automáticamente las ganancias siguiendo el precio mientras sube
# $value is the activation percentage as typed.
trader-trailing-activation-impact = Empieza a seguir con +{ $value }% de ganancia
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = Sale a -{ $value }% desde el máximo
trader-trailing-activation = Activación
trader-trailing-peak = Máximo
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% final
# $value is a formatted percentage.
trader-trailing-summary-protected = Ganancia protegida de <strong>{ $value }</strong>
# $value is a formatted percentage.
trader-trailing-summary-avoided = Pérdida evitada de <strong>{ $value }</strong> desde el máximo

## Take profit

trader-roi-title = Take profit
trader-roi-subtitle = Sale automáticamente de toda la posición cuando la ganancia alcanza tu objetivo
# $target is the target percentage as typed.
trader-roi-impact = Sale con +{ $target }% de ganancia
trader-roi-example-title = Escenario de ejemplo
trader-roi-initial-buy = Compra inicial
trader-roi-target-hit = Objetivo alcanzado
trader-roi-full-position = Posición completa
trader-roi-sold = 100% vendido
# $target is the target percentage as typed.
trader-roi-summary = Ganancia asegurada de <strong>+{ $target }%</strong>

## Time-based exit

trader-time-title = Salida por tiempo
trader-time-subtitle = Sale automáticamente de las posiciones tras un tiempo máximo de tenencia si la pérdida supera el umbral
trader-time-unit-seconds = segundos
trader-time-unit-minutes = minutos
trader-time-unit-hours = horas
trader-time-unit-days = días
# Shown before the configured duration loads.
trader-time-conversion-default = 168 horas = 7 días
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } segundo
        [many] { $amount } segundos
       *[other] { $amount } segundos
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } minuto
        [many] { $amount } minutos
       *[other] { $amount } minutos
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } hora
        [many] { $amount } horas
       *[other] { $amount } horas
    }
trader-duration-days =
    { $count ->
        [one] { $amount } día
        [many] { $amount } días
       *[other] { $amount } días
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = Sale si baja { $value }% o más tras el período de tenencia
# $day is the day number of the example.
trader-time-day = Día { $day }
trader-time-position-opened = Posición abierta
trader-time-limit = Límite de tiempo
trader-time-hold-reached = Período de tenencia alcanzado
trader-time-loss-met = Umbral de pérdida alcanzado
trader-time-note = <strong>Nota:</strong> las posiciones en ganancia o con pérdidas menores NO se cerrarán
trader-time-positions-title = Estado de las posiciones actuales
trader-time-positions-loading = Cargando posiciones...
trader-time-positions-empty = No hay posiciones abiertas
trader-time-positions-hold = Tiempo de tenencia:
trader-time-positions-roi = ROI:

## Strategy control

trader-strategy-entry-title = Estrategias de entrada
trader-strategy-entry-subtitle = Señales que pueden abrir una nueva posición.
trader-strategy-exit-title = Estrategias de salida
trader-strategy-exit-subtitle = Señales que pueden cerrar o proteger una posición abierta.
trader-strategy-active-unknown = -- activas
trader-strategy-active = { $enabled }/{ $total } activas
trader-strategy-loading = Cargando estrategias...
trader-strategy-load-failed = No se pudieron cargar las estrategias
trader-strategy-empty = No hay estrategias definidas
trader-strategy-no-description = Sin descripción.
trader-strategy-unnamed = Estrategia sin nombre
trader-strategy-priority-auto = Auto
trader-strategy-priority = Prioridad { $priority }

## Dollar-cost averaging

trader-dca-title = Promedio de costo en dólares
trader-dca-subtitle = Añade automáticamente a posiciones en pérdida para reducir tu precio de entrada promedio
trader-dca-example-title = Ejemplo de DCA
trader-dca-example = 0.01 { -sol } inicial → DCA #1: 0.005 { -sol } @ -10% → DCA #2: 0.005 { -sol } @ -10% más
trader-dca-info-title = Información de la estrategia DCA
trader-dca-info-subtitle = Consideraciones importantes para el trading con DCA
trader-dca-how-title = Cómo funciona el DCA
trader-dca-how-trigger = <strong>Disparador:</strong> la posición cae por debajo del umbral de DCA (p. ej., -10%)
trader-dca-how-action = <strong>Acción:</strong> añade más { -sol } para reducir el costo base promedio
trader-dca-how-repeat = <strong>Repetición:</strong> puede hacer DCA varias veces según el máximo configurado
trader-dca-risk-title = Advertencias de riesgo
trader-dca-risk-exposure = <strong>Mayor exposición:</strong> el DCA aumenta el capital total en riesgo por posición
trader-dca-risk-knife = <strong>Cuchillo cayendo:</strong> el DCA no ayuda si el token sigue en tendencia bajista
trader-dca-risk-cooldown = <strong>Enfriamiento:</strong> usa el enfriamiento para evitar entradas de DCA en ráfaga

## General settings

trader-sizing-title = Tamaño de posición
trader-sizing-subtitle = Controla cuánto invertir por posición
trader-timing-title = Tiempos y enfriamientos
trader-timing-subtitle = Controla el tiempo entre operaciones
trader-timing-close-cooldown = Enfriamiento tras cerrar posición
trader-timing-close-cooldown-hint = Minutos de espera antes de reabrir el mismo token
trader-timing-concurrency = Concurrencia de revisión de entradas
trader-timing-concurrency-hint = Número de tokens a revisar simultáneamente (más = más rápido, pero más CPU)
trader-timing-unit-minutes = min
trader-timing-unit-tokens = tokens
