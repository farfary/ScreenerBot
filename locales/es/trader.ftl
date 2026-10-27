# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Take profit
trader-exit-type-roi-exit = Take profit
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Salida por tiempo
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
trader-tab-take-profit = Take profit
trader-tab-time-rules = Salida por tiempo
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
trader-setup-action = Completar configuración
trader-toggle-on = ACTIVADO
trader-toggle-off = DESACTIVADO
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
trader-force-stop-hint = Detiene a la vez todos los monitores, tareas de copia y operaciones.
trader-force-stop-idle = No hay nada en marcha que una detención forzada pudiera parar.

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
trader-toast-strategy-armed = { $name } ahora opera en vivo
trader-toast-strategy-paper = { $name } ahora opera en simulación
trader-toast-strategy-mode-failed = No se pudo cambiar el modo de trading

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Ventana de estadísticas
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-stats-book =
    .aria-label = Cartera de trading
trader-paper-run-since = Sesión simulada desde { $date }
trader-paper-start = Inicio
trader-paper-cash = Efectivo
trader-paper-open = Abierto
trader-paper-unrealized = No realizado
trader-paper-reset = Reiniciar sesión simulada
trader-paper-reset-title = ¿Reiniciar la sesión simulada?
trader-paper-reset-run-message = Cada posición simulada abierta, incluidas las tenencias simuladas de las tareas de copia, se cierra a su precio actual y empieza una nueva sesión con el saldo inicial configurado. Las sesiones anteriores quedan en el historial.
trader-paper-reset-done = Empezó una nueva sesión simulada
trader-paper-reset-failed = No se pudo reiniciar la sesión simulada
trader-paper-account-failed = No se pudo cargar la sesión simulada
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
trader-weekly-title = P&L semanal
trader-weekly-subtitle = { -sol } realizados por semana ISO, con el total acumulado
trader-weekly-chart = Ganancias y pérdidas realizadas semanales en { -sol }
trader-extreme-best = Mejor operación
trader-extreme-worst = Peor operación

## Stats: exit breakdown

trader-exit-title = Desglose de estrategias de salida
trader-exit-subtitle = Cómo se cerraron las posiciones y qué devolvió cada salida
trader-exit-loading = Cargando datos de salidas...
trader-exit-empty-day = No hay operaciones cerradas en las últimas 24 horas
trader-exit-empty-all = Aún no hay operaciones cerradas
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
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = Vender { $part }
trader-stop-loss-part-of-position = Parte de la posición
trader-stop-loss-sell-rest = Vender el resto
trader-stop-loss-still-below = Sigue por debajo del stop
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
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Mínimo asegurado al activarse: { $value }
trader-trailing-locked-minimum-warning = El stop queda en la entrada o por debajo al activarse; solo vende con ganancia. Activa el stop en punto de equilibrio para tener un piso.
trader-trailing-activation = Activación
trader-trailing-peak = Máximo
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% final
# $value is a formatted percentage.
trader-trailing-summary-protected = Ganancia protegida de <strong>{ $value }</strong>
# $value is a formatted percentage.
trader-trailing-summary-avoided = Pérdida evitada de <strong>{ $value }</strong> desde el máximo
# $step is the band's position in the list, from 1.
trader-trailing-band = Banda { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Ganancia máxima de la banda { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Distancia de la banda { $step }
trader-trailing-band-profit-caption = Desde ganancia máxima
trader-trailing-band-distance-caption = Seguir a
trader-trailing-add-band = Añadir banda
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Quitar banda { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = La ganancia máxima de cada banda debe estar entre { $min }% y { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = La distancia de cada banda debe estar entre { $min }% y { $max }%
trader-trailing-band-error-order = Cada banda necesita una ganancia máxima mayor que la activación y que la banda anterior
trader-trailing-band-error-distance = Cada banda debe seguir a una distancia menor que su ganancia máxima

## Break-even stop

trader-break-even-title = Stop en punto de equilibrio
trader-break-even-subtitle = Cuando una posición ya entró en ganancia, véndela entera antes de que pase a pérdida
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Cuando el mejor precio desde la última compra llega a { $trigger }, una caída de vuelta a { $offset } lo vende todo.

## Take profit

trader-take-profit-title = Take profit
trader-take-profit-subtitle = Vende la posición por tramos a medida que la ganancia alcanza cada nivel
trader-take-profit-initial-buy = Compra inicial
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Tramo { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Nivel de ganancia del tramo { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Parte a vender del tramo { $step }
trader-take-profit-profit-caption = Nivel de ganancia
trader-take-profit-sell-caption = Parte a vender
trader-take-profit-add-step = Añadir tramo
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Quitar tramo { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Cada nivel de ganancia debe estar entre { $min }% y { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Cada parte debe estar entre { $min }% y { $max }%
trader-take-profit-error-order = Cada tramo necesita un nivel de ganancia mayor que el anterior
# $max is the largest total share, the whole position.
trader-take-profit-error-total = Los tramos juntos pueden vender como máximo { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = El { $runner }% de la posición queda abierto como runner
trader-take-profit-runner-none = El último tramo vende el resto de la posición
trader-take-profit-runner-warning = Con el trailing stop, el stop en punto de equilibrio, ambas salidas por tiempo y todas las estrategias de salida desactivados, el runner solo se cierra por el stop loss o a mano
trader-take-profit-info = Las partes se refieren a la posición inicial. Si el precio supera varios niveles a la vez, se vende un tramo por comprobación.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = A +{ $profit }%, vender { $share }%
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Queda en manos de las demás reglas de salida
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Vende <strong>{ $sold }%</strong> hasta +{ $profit }% de ganancia

## Time-based exit

trader-time-loss-title = Cerrar posiciones en pérdida
trader-time-subtitle = Sale automáticamente de las posiciones tras un tiempo máximo de tenencia si la pérdida supera el umbral
trader-time-flat-title = Cerrar posiciones estancadas
trader-time-flat-subtitle = Libera el capital de posiciones que no se mueven, ni al alza ni a la baja
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Una posición mantenida { $span } o más con su P&L dentro de ±{ $band } se cierra.
trader-time-unit-seconds = segundos
trader-time-unit-minutes = minutos
trader-time-unit-hours = horas
trader-time-unit-days = días
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
trader-time-loss-hint = La pérdida a la que la salida por tiempo vende una vez cumplido el tiempo de tenencia (40 = salida a -40%)
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
trader-time-positions-empty-message = Las posiciones abiertas aparecen aquí con su tiempo de tenencia y ROI frente al límite de tiempo.
trader-time-positions-token = Token
trader-time-positions-hold = Tiempo de tenencia
trader-time-positions-roi = ROI
trader-time-positions-rule = Salida por tiempo
trader-time-status-off = Las salidas por tiempo están desactivadas
trader-time-status-none = Ahora no aplica ninguna salida por tiempo
trader-time-status-not-managed = No gestionada por Auto Trader
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = En pérdida { $pnl }; se cierra en { $remaining }
trader-time-status-losing-due = En pérdida { $pnl }; se cierra en la próxima revisión
# $held and $remaining are formatted durations.
trader-time-status-flat = Estancada desde hace { $held }; se cierra en { $remaining }
trader-time-status-flat-due = Estancada desde hace { $held }; se cierra en la próxima revisión

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
trader-strategy-unnamed = Estrategia sin nombre
trader-strategy-priority-auto = Auto
trader-strategy-priority = Prioridad { $priority }
trader-strategy-mode-arm = Activar en vivo
trader-strategy-mode-return-paper = Volver a simulación
trader-strategy-arm-title = Activar “{ $name }” en vivo
trader-strategy-arm-lead = Las nuevas entradas de “{ $name }” operarán con swaps reales desde tu billetera de trading.
trader-strategy-arm-readiness = Preparación según la cartera simulada
trader-strategy-arm-open-paper = Las posiciones simuladas abiertas siguen siendo simuladas y se cierran en simulación.
trader-strategy-arm-ack-real-native = Las nuevas entradas gastan { -sol } real de tu billetera de trading.
trader-strategy-arm-ack-fees = Los swaps en vivo pagan comisiones de red y deslizamiento reales; los resultados simulados no garantizan resultados en vivo.
trader-strategy-arm-ack-unready = Algunas comprobaciones de preparación no se han superado. Activar esta estrategia de todos modos.
trader-strategy-arm-unavailable = El trading en vivo no está disponible ahora; consulta la última comprobación.
trader-strategy-return-title = Volver “{ $name }” a simulación
trader-strategy-return-message = Las nuevas entradas se abren en la cartera simulada sin gastar { -sol }. Las posiciones en vivo abiertas siguen en vivo y se cierran en vivo.
trader-strategy-readiness-failed = No se pudo leer la preparación de la estrategia
trader-paper-explore-title = El trading simulado empieza tras la configuración

## Dollar-cost averaging

trader-dca-title = Promedio de costo en dólares
trader-dca-subtitle = Añade automáticamente a posiciones en pérdida para reducir tu precio de entrada promedio
trader-dca-measure-average = Entrada media
trader-dca-measure-first-entry = Primera entrada
trader-dca-step = Añadido { $step }
trader-dca-drop-label = Caída de precio del añadido { $step }
trader-dca-size-label = Tamaño del añadido { $step }, veces la primera compra
trader-dca-drop-caption = Caída de precio
trader-dca-size-caption = Compra × primera compra
trader-dca-add-step = Añadir nivel
trader-dca-remove-step = Quitar añadido { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Por debajo de la operación mínima de { $minimum }: este añadido no se envía
trader-dca-error-drop-range = Cada caída debe estar entre { $min }% y { $max }%
trader-dca-error-size-range = Cada tamaño debe estar entre { $min }× y { $max }× la primera compra
trader-dca-error-order = Medido desde la primera entrada, cada añadido necesita una caída más profunda que el anterior
trader-dca-cooldown-impact = Al menos { $span } entre entradas extra
trader-dca-min-hold-none = El primer añadido puede llegar justo después de la entrada
trader-dca-min-hold-impact = El primer añadido espera al menos { $span } tras la entrada
trader-dca-capacity = Lo máximo que puede tener esta posición: { $amount } ({ $multiple }× la primera compra)
trader-dca-info-title = Información de la estrategia DCA
trader-dca-info-subtitle = Consideraciones importantes para el trading con DCA
trader-dca-how-title = Cómo funciona el DCA
trader-dca-how-trigger = <strong>Disparador:</strong> el precio cae por debajo del siguiente nivel, medido desde la entrada media o la primera
trader-dca-how-action = <strong>Acción:</strong> añade más { -sol } para reducir el costo base promedio
trader-dca-how-repeat = <strong>Escalera:</strong> cada añadido pasa al siguiente nivel; los añadidos paran cuando se usan todos los niveles
trader-dca-how-pause = <strong>Pausas:</strong> ningún añadido mientras el anterior se liquida o mientras el límite de pérdidas bloquea nuevas entradas
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
