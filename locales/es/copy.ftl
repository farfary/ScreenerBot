copy-skip-not-buy-swap = La actividad de la billetera no fue una compra
copy-skip-task-disabled = La tarea está en pausa
copy-skip-mode-transition-required = El modo de ejecución debe cambiarse por separado
copy-skip-live-confirmation-required = La ejecución en vivo necesita confirmación
copy-skip-unsupported-sizing-mode = El modo de tamaño aún no es compatible
copy-skip-self-copy = La billetera es una de las tuyas
copy-skip-target-below-minimum = Operación de la billetera por debajo del mínimo
copy-skip-target-above-maximum = Operación de la billetera por encima del máximo
copy-skip-already-bought = Este token ya se compró (comprar una vez)
copy-skip-blacklisted = El token está bloqueado por los controles de riesgo
copy-skip-filter-required = El token no pasó el filtrado
copy-skip-budget-exhausted = Se agotó el presupuesto de la tarea
copy-skip-token-cap-reached = Se alcanzó el límite por token
copy-skip-below-minimum-size = Tamaño de copia demasiado pequeño
copy-skip-invalid-sizing = El tamaño de la tarea no es válido
copy-skip-invalid-slippage = El deslizamiento de la tarea no es válido
copy-skip-invalid-exit-policy = Las reglas de salida de la tarea no son válidas
copy-skip-invalid-price = No hay un precio de mercado utilizable
copy-skip-not-sell-swap = La actividad de la billetera no fue una venta
copy-skip-exit-mode-disabled = Venta de la billetera ignorada: la tarea vende con sus propias reglas
copy-skip-force-stopped = El trading está detenido a la fuerza
copy-skip-copy-position-not-found = Ninguna posición pertenece a esta tarea
copy-skip-position-user-only = La posición la gestionas tú
copy-skip-position-management-mismatch = La posición ya no sigue las ventas copiadas
copy-skip-latency-kill-switch = Pausa automática: operaciones detectadas demasiado tarde
copy-skip-claim-reconciled-abandoned = Envío en vivo interrumpido, cerrado sin reintento
copy-skip-stale-observation = Reproducida tras una interrupción, demasiado antigua para copiar
copy-skip-unknown-observation-time = La operación reproducida no tiene hora de bloque
copy-skip-entry-blocked = Entrada bloqueada

copy-entry-block-force-stopped = El trading está detenido a la fuerza
copy-entry-block-loss-limit = El límite de pérdidas bloquea nuevas entradas
copy-entry-block-connectivity = Los servicios necesarios no están disponibles
copy-entry-block-position-limit = Se alcanzó el límite de posiciones abiertas
copy-entry-block-already-open = Ya hay una posición abierta
copy-entry-block-reentry-cooldown = Enfriamiento de reentrada del token
copy-entry-block-open-cooldown = Enfriamiento global de entradas
copy-entry-block-entry-reserved = Se está procesando otra entrada
copy-entry-block-blacklisted = El token está bloqueado por los controles de riesgo
copy-entry-block-check-failed = No se pudo completar una comprobación de seguridad

copy-pause-user = Pausada por ti
copy-pause-latency-kill-switch = Pausa automática: las operaciones llegaron con { $average } s de retraso de media (límite { $threshold } s)
copy-pause-watch-detached = Pausa automática: la billetera ya no está en seguimiento
copy-pause-watch-budget-exceeded = Pausada: esta billetera alcanzó su límite de { $limit } firmas por comprobación de seguimiento antes de ponerse al día
copy-pause-helius-unavailable = Pausada: fallaron las comprobaciones de la billetera con { -helius }
copy-pause-watch-processing-failed = Pausada: no se pudo procesar la actividad de la billetera
copy-pause-unspecified = En pausa

copy-pause-short-user = por ti
copy-pause-short-latency-kill-switch = demasiado lenta
copy-pause-short-watch-detached = seguimiento perdido
copy-pause-short-watch-budget-exceeded = límite de seguimiento
copy-pause-short-helius-unavailable = proveedor de seguimiento
copy-pause-short-watch-processing-failed = proceso de seguimiento
copy-state-paused = En pausa
copy-state-paused-reason = En pausa · { $reason }

copy-readiness-history = Historial simulado
copy-readiness-history-met =
    { $count ->
        [one] { $count } ronda simulada cerrada, se necesitan { $needed }
        [many] { $count } rondas simuladas cerradas, se necesitan { $needed }
       *[other] { $count } rondas simuladas cerradas, se necesitan { $needed }
    }
copy-readiness-history-short = { $count } de { $needed } rondas simuladas cerradas
copy-readiness-profit = Rentable en simulación
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realizados en { $count } ronda, { $wins } ganadas
        [many] { $realized } { -sol } realizados en { $count } rondas, { $wins } ganadas
       *[other] { $realized } { -sol } realizados en { $count } rondas, { $wins } ganadas
    }
copy-readiness-latency = Operaciones detectadas a tiempo
copy-readiness-latency-detail = llegada p95 { $p95 } s, límite { $limit } s
copy-readiness-latency-none = Aún no hay muestras de llegada
copy-readiness-priced = Todas las tenencias con precio
copy-readiness-priced-ok = Todas las tenencias simuladas abiertas tienen precio de pool
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } tenencia abierta sin precio de pool
        [many] { $count } tenencias abiertas sin precio de pool
       *[other] { $count } tenencias abiertas sin precio de pool
    }
copy-readiness-runtime = Ejecución en vivo disponible
copy-readiness-runtime-ok = La configuración y los controles de seguridad permiten copias en vivo

copy-live-block-setup-incomplete = Termina primero la configuración de la billetera y del RPC
copy-live-block-force-stop = La parada de emergencia está activada
copy-live-block-copy-trading-disabled = El procesamiento de copias está en pausa global
copy-live-block-unavailable = La ejecución en vivo no está disponible

## Task state, mode and exit labels.

copy-state-system-paused = En pausa global
copy-state-force-stopped = Detención forzada
copy-state-entries-blocked = Entradas bloqueadas
copy-state-running-live = En ejecución
copy-state-running-paper = En ejecución
copy-mode-paper = Simulado
copy-mode-live = En vivo
copy-exit-mode-buy-only = Mis reglas de salida
copy-exit-mode-mirror = Replicar ventas de la billetera
copy-exit-mode-hybrid = Ventas de la billetera y mis reglas
copy-exit-target-sell = Vendió la billetera
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Regla de tiempo
copy-exit-manual = Cerrada a mano

## Shared wording

copy-request-failed = Solicitud fallida
copy-keep-paused = Mantener en pausa
copy-paused-suffix = · en pausa
copy-mode-paused = { $mode } · en pausa
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = P&L realizado
copy-metric-unrealized-pnl = P&L no realizado
copy-metric-win-rate = Tasa de aciertos
copy-metric-budget-spent = Presupuesto gastado
copy-metric-median-arrival = Llegada mediana
copy-metric-open-holdings = Tenencias abiertas
copy-record-won-lost = { $won } ganadas · { $lost } perdidas
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Ejecuciones
copy-kind-exits = Salidas
copy-kind-skips = Omisiones
copy-kind-errors = Errores
copy-field-per-trade-cap = Límite por operación
copy-field-per-token-cap = Límite por token
copy-field-total-budget = Presupuesto total
copy-field-slippage = Deslizamiento
copy-rules-wallet-sells-only = Solo ventas de la billetera
copy-filter-copy-setting-required = Ajuste de copia (obligatorio)
copy-filter-copy-setting-not-required = Ajuste de copia (no obligatorio)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } ronda cerrada
        [many] { $count } rondas cerradas
       *[other] { $count } rondas cerradas
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } tenencia abierta
        [many] { $count } tenencias abiertas
       *[other] { $count } tenencias abiertas
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } tenencia con precio · { $unpriced } sin precio
        [many] { $priced } tenencias con precio · { $unpriced } sin precio
       *[other] { $priced } tenencias con precio · { $unpriced } sin precio
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } tenencia sin precio
        [many] { $count } tenencias sin precio
       *[other] { $count } tenencias sin precio
    }
copy-range-24h = 24 h
copy-range-7d = 7 d
copy-range-30d = 30 d
copy-range-all = Todo
copy-range-label =
    .aria-label = Rango de fechas

## Page strip

copy-page-title = Copy Trading
copy-page-beta = Beta
copy-strip-loading = Cargando
copy-strip-unavailable = No disponible
copy-strip-setup-required = Configuración requerida · el copy trading necesita billetera y RPC
copy-strip-pause-all = Pausar todo
copy-strip-resume = Reanudar procesamiento
copy-strip-settings = Configuración
copy-strip-add-wallet = Añadir billetera
copy-strip-paused-globally = En pausa global · sin copias nuevas, las salidas siguen activas
copy-strip-force-stopped = Detención forzada · no se copia nada
copy-strip-loss-limit = Límite de pérdidas · nuevas entradas bloqueadas, las salidas siguen activas
copy-strip-idle-paused =
    { $count ->
        [one] Inactivo · { $count } tarea en pausa
        [many] Inactivo · { $count } tareas en pausa
       *[other] Inactivo · { $count } tareas en pausa
    }
copy-strip-idle-empty = Inactivo · aún sin tareas
copy-strip-processing = Procesando · { $paper } simuladas
copy-strip-processing-live = Procesando · { $live } en vivo · { $paper } simuladas
copy-figures-label =
    .aria-label = Totales de copy trading
copy-figure-marked-at-pool = Valorado al precio del pool
copy-figure-across-tasks = En todas las tareas
copy-figure-budget-lifetime = Gasto acumulado de las tareas activas
copy-figure-budget-none = Sin tareas activas
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } operación
        [many] { $count } operaciones
       *[other] { $count } operaciones
    }
copy-figure-arrival-none = Sin muestras de las tareas activas

## Page frame

copy-load-failed = No se pudo cargar el copy trading: { $error }
copy-resume-all-title = Reanudar el procesamiento de copias
copy-resume-all-message =
    { $count ->
        [one] { $count } tarea en vivo enviará swaps reales cuando su billetera vuelva a operar.
        [many] { $count } tareas en vivo enviarán swaps reales cuando sus billeteras vuelvan a operar.
       *[other] { $count } tareas en vivo enviarán swaps reales cuando sus billeteras vuelvan a operar.
    }
copy-toast-resumed-all = Procesamiento de copias reanudado
copy-toast-paused-all = Todo el procesamiento de copias en pausa
copy-toast-global-failed = No se pudo cambiar el procesamiento de copias

## Onboarding

copy-onboarding-title = Copia las billeteras en las que confías, pruébalas primero en simulación
copy-onboarding-body = Toda tarea empieza en simulación: las operaciones objetivo se simulan al precio del pool con tu deslizamiento y tus comisiones, y tus reglas de salida se aplican sobre la cartera simulada. Activa el modo en vivo por billetera cuando sus resultados simulados lo justifiquen.
copy-onboarding-add = Añade tu primera billetera
copy-setup-gate-title = El copy trading necesita una billetera
copy-onboarding-observe = Observar
copy-onboarding-observe-detail = Detecta los swaps de la billetera sin gastar { -sol }.
copy-onboarding-evaluate = Evaluar
copy-onboarding-evaluate-detail = Revisa el P&L simulado, la tasa de aciertos, las omisiones, la velocidad de detección y el deslizamiento.
copy-onboarding-arm = Activar
copy-onboarding-arm-detail = Supera las comprobaciones de preparación y activa los swaps reales.

## Wallet list

copy-list-label =
    .aria-label = Billeteras copiadas
copy-list-title = Billeteras
copy-list-compare = Comparar
copy-list-sort-label = Ordenar billeteras
copy-list-count = { $active } activas · { $total } en total
copy-sort-pnl = P&L
copy-sort-state = Estado
copy-sort-name = Nombre
copy-compare-label =
    .aria-label = Comparar billeteras

## Dialog chrome

copy-dialog-close =
    .aria-label = Cerrar
copy-editor-title-add = Añadir billetera
copy-editor-sub-add = Las tareas nuevas empiezan en simulación
copy-arm-title = Activar copia en vivo
copy-arm-sub = Swaps reales desde tu billetera
copy-arm-keep-paper = Mantener simulación
copy-arm-confirm = Activar en vivo
copy-profile-title = Perfil de la billetera
copy-profile-sub = Lo que este bot ha visto de la billetera

## Settings dialog

copy-settings-title = Configuración de copy trading
copy-settings-subtitle = Política global para todas las tareas
copy-settings-filter-warning = Con la configuración de filtrado predeterminada, esto rechaza casi todos los tokens, así que no se copia nada. Déjalo desactivado salvo que tus filtros aprueben los tokens que operan tus billeteras.
copy-settings-unit-seconds = segundos
copy-settings-unit-trades = operaciones
copy-settings-unit-tasks = tareas
copy-settings-unit-closed-rounds = rondas cerradas
copy-settings-save = Guardar configuración
copy-settings-load-failed = No se pudo cargar la configuración de copias
copy-settings-saved = Configuración de copy trading guardada

## Workspace

copy-tab-overview = Resumen
copy-tab-holdings = Tenencias
copy-tab-activity = Actividad
copy-tab-rules = Reglas
copy-tab-execution = Ejecución
copy-tabs-label = Vistas de la tarea
copy-workspace-select = Selecciona una billetera para abrir su espacio de trabajo.
copy-workspace-loading = Cargando tarea…
copy-workspace-load-failed = No se pudo cargar esta tarea: { $error }

copy-state-detail-paper = En ejecución en simulación · las operaciones se simulan, no se gasta nada
copy-state-detail-live = En ejecución en vivo · las operaciones de la billetera se copian con swaps reales
copy-state-detail-system-paused = En espera · el procesamiento de copias está en pausa global, las salidas siguen activas
copy-state-detail-entries-blocked = Entradas bloqueadas por el límite de pérdidas · las salidas siguen activas
copy-state-detail-force-stopped = Detención forzada · no se copia nada

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Al reanudar se mantiene el mismo límite, así que volverá a pausarse mientras las operaciones sigan llegando tarde. Revisa el flujo RPC o sube el límite de llegada en Configuración.
copy-paused-resume-detached = Al reanudar se vuelve a seguir la billetera.
copy-paused-holdings-rules =
    { $count ->
        [one] Sus reglas de salida siguen cerrando su { $count } tenencia abierta.
        [many] Sus reglas de salida siguen cerrando sus { $count } tenencias abiertas.
       *[other] Sus reglas de salida siguen cerrando sus { $count } tenencias abiertas.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] Las ventas de la billetera siguen cerrando su { $count } tenencia abierta.
        [many] Las ventas de la billetera siguen cerrando sus { $count } tenencias abiertas.
       *[other] Las ventas de la billetera siguen cerrando sus { $count } tenencias abiertas.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] Las ventas de la billetera y sus reglas de salida siguen cerrando su { $count } tenencia abierta.
        [many] Las ventas de la billetera y sus reglas de salida siguen cerrando sus { $count } tenencias abiertas.
       *[other] Las ventas de la billetera y sus reglas de salida siguen cerrando sus { $count } tenencias abiertas.
    }

copy-watch-state-catching-up = Seguimiento de la billetera: poniéndose al día. Comprobando esta billetera mediante { -helius }.
copy-watch-state-watching = Seguimiento de la billetera: activo. Comprobando esta billetera mediante { -helius }.
copy-watch-last-check = Última comprobación { $ago }.
copy-watch-recovery-active = Seguimiento de la billetera activo
copy-watch-recovery-catching-up = El seguimiento de la billetera se está poniendo al día
copy-watch-recovery-still-paused = La tarea de copia sigue en pausa. Reanuda la copia cuando estés listo.
copy-watch-recovery-title = Restaurar el seguimiento de la billetera
copy-watch-recovery-processing-failed = No se pudo procesar la actividad de la billetera. El progreso guardado se conserva. Reintenta cuando se resuelva el problema.
copy-watch-recovery-provider-failed = Fallaron las comprobaciones de { -helius }. El progreso guardado se conserva. Reintenta cuando el proveedor esté disponible.
copy-watch-recovery-budget-intro = Esta billetera tiene más actividad de la que su seguimiento actual puede comprobar. Elige cómo continuar.
copy-watch-approve = Intentar ponerse al día con { -helius }
copy-watch-approve-help = Continúa desde el progreso guardado. Puede consumir más créditos de { -helius } y aun así quedarse atrás.
copy-watch-approve-unavailable = La puesta al día con { -helius } no está disponible. Configura un endpoint RPC de { -helius } activado para continuar sin omitir actividad sin comprobar.
copy-watch-no-provider = Este seguimiento no admite ningún proveedor de puesta al día.
copy-watch-budget-label = Firmas comprobadas por comprobación
copy-watch-budget-hint = O bien omite la actividad sin comprobar y reanuda desde ahora. Elige entre { $min } y { $max } firmas por comprobación; un límite mayor puede usar más llamadas RPC.
copy-watch-ack = Entiendo que la actividad omitida no se copiará.
copy-watch-toast-range = Elige entre { $min } y { $max } firmas por sondeo en pasos de { $step } firmas
copy-watch-toast-ack = Confirma que se omitirán las firmas posteriores a la última comprobación completada
copy-watch-resumed = Seguimiento de la billetera reanudado desde ahora; la tarea de copia sigue en pausa
copy-watch-resume-failed = No se pudo reanudar el seguimiento de la billetera
copy-watch-retry-started = Reintento del seguimiento de la billetera iniciado desde el progreso guardado; la tarea de copia sigue en pausa
copy-watch-retry-failed = No se pudo reintentar el seguimiento de la billetera
copy-watch-approve-title = Permitir la puesta al día con { -helius } para esta billetera
copy-watch-approve-message = { -helius } puede comprobar las transacciones correctas de Solana desde el progreso guardado sin omitir el intervalo sin comprobar. Actualmente cobra 10 créditos por cada 100 transacciones completas devueltas, redondeando hacia arriba, con un mínimo de 10 créditos por solicitud. Una comprobación puede hacer varias solicitudes; el uso y los precios del proveedor pueden variar. La copia sigue en pausa hasta que la reanudes por separado.
copy-watch-approve-confirm = Permitir para esta billetera
copy-watch-approved = Seguimiento de la billetera iniciado desde el progreso guardado; la tarea de copia sigue en pausa
copy-watch-restore-failed = No se pudo restaurar el seguimiento de la billetera

copy-action-pause = Pausar
copy-action-resume = Reanudar
copy-action-resume-copy = Reanudar copia
copy-action-resume-from-now = Reanudar desde ahora
copy-action-retry-watch = Reintentar seguimiento de la billetera
copy-action-return-paper = Volver a simulación
copy-action-edit-rules = Editar reglas
copy-action-clone = Clonar
copy-action-profile = Perfil de la billetera
copy-resume-live-title = Reanudar copia en vivo
copy-resume-live-message = “{ $name }” enviará swaps reales desde tu billetera cuando esta billetera vuelva a operar.
copy-resume-live-confirm = Reanudar en vivo
copy-task-resumed = Tarea reanudada
copy-task-paused = Tarea en pausa
copy-task-state-failed = No se pudo cambiar el estado de la tarea
copy-return-paper-message = Las nuevas copias de “{ $name }” volverán a simularse, sin gastar { -sol }.
copy-return-paper-cancel = Mantener en vivo
copy-task-returned-paper = La tarea volvió a simulación
copy-mode-change-failed = No se pudo cambiar el modo de ejecución
copy-delete-title = Eliminar tarea de copia
copy-delete-message = ¿Eliminar “{ $name }”? Se borran sus decisiones y resultados simulados, y la billetera deja de seguirse para esta tarea.
copy-delete-confirm = Eliminar tarea
copy-delete-cancel = Conservar tarea
copy-task-deleted = Tarea de copia eliminada
copy-task-delete-failed = No se pudo eliminar la tarea de copia

## Overview tab

copy-overview-results = Resultados
copy-analytics-load-failed = No se pudieron cargar las analíticas: { $error }
copy-analytics-loading = Cargando analíticas…
copy-exit-bucket =
    { $count ->
        [one] { $count } venta · { $pnl }
        [many] { $count } ventas · { $pnl }
       *[other] { $count } ventas · { $pnl }
    }
copy-overview-average-win = Ganancia promedio
copy-overview-average-loss = Pérdida promedio { $amount }
copy-overview-profit-factor = Factor de beneficio
copy-overview-profit-factor-note = Ganancias brutas ÷ pérdidas brutas
copy-overview-average-hold = Tenencia promedio
copy-overview-average-hold-note = De la entrada a la salida
copy-overview-best-round = Mejor ronda
copy-overview-worst-round = Peor { $amount }
copy-overview-curve-title = P&L acumulado
copy-overview-exits-title = Ventas por salida
copy-overview-skips-title = Por qué se omitieron operaciones
copy-book-title-live = Cartera en vivo
copy-book-title-paper = Cartera simulada
copy-book-all-time = Todo el historial
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] compra
        [many] compras
       *[other] compras
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] salida por tus reglas
        [many] salidas por tus reglas
       *[other] salidas por tus reglas
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] venta de la billetera
        [many] ventas de la billetera
       *[other] ventas de la billetera
    }
copy-book-manual-closes = <strong>{ $count }</strong> cerradas a mano
copy-book-skipped = <strong>{ $count }</strong> omitidas
copy-book-failed = <strong>{ $count }</strong> fallidas
copy-book-closed = { $count } cerradas
copy-book-budget-note = Gasto { $mode } de { $total } · { $remaining } restantes
copy-check-passed = superada
copy-check-not-passed = no superada
copy-readiness-title = Antes de pasar a vivo
copy-readiness-live-note = Esta tarea opera en vivo. Devuélvela a simulación desde la cabecera de arriba.
copy-readiness-all-pass = Se superan todas las comprobaciones.
copy-readiness-needs-review = Para activarla hay que revisar de forma explícita lo que no está listo.
copy-readiness-arm = Revisar y activar en vivo

## Rules tab and review

copy-rules-title = Reglas en vigor
copy-rules-size-ratio = { $pct } de la operación de la billetera
copy-rules-size-fixed = { $amount } por copia
copy-rules-target-any = Cualquier tamaño
copy-rules-target-min = Al menos { $amount }
copy-rules-target-max = Como máximo { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Ajuste de la tarea · Trader { $value }
copy-rules-source-default = Valor predeterminado del Trader
copy-rules-not-used = No se usa: deciden las ventas de la billetera
copy-rules-col-rule = Regla
copy-rules-col-applies = Se aplica
copy-rules-col-source = Origen
copy-rules-budget-note = { $spent } gastados en { $mode } · { $remaining } restantes
copy-rules-token-copies =
    { $count ->
        [one] Unas { $count } copia completa de un token
        [many] Unas { $count } copias completas de un token
       *[other] Unas { $count } copias completas de un token
    }
copy-rules-sizing = Tamaño
copy-rules-copy-size = Tamaño de copia
copy-rules-entry-filters = Filtros de entrada
copy-rules-target-size = Tamaño de la operación de la billetera
copy-rules-repeat-buys = Compras repetidas
copy-rules-repeat-first-only = Solo la primera compra de cada token
copy-rules-repeat-every = Todas las compras, hasta el límite por token
copy-rules-filter-pass = Aprobar el filtrado
copy-rules-filter-required = Obligatorio
copy-rules-filter-not-required = No obligatorio
copy-rules-filter-task-override = Ajuste de la tarea
copy-rules-exits = Salidas
copy-rules-exits-inactive = Las tenencias solo se venden cuando vende la billetera; las reglas de abajo no se aplican en este modo.

## Exit rules

copy-rule-status = Estado
copy-rule-on = Activado
copy-rule-off = Desactivado
copy-rule-unit-seconds = segundos
copy-rule-unit-minutes = minutos
copy-rule-stop-loss-threshold = Vende con una pérdida de
copy-rule-stop-loss-min-hold = No antes de mantener
copy-rule-no-minimum = Sin mínimo
copy-rule-partial-exits = Salidas parciales
copy-rule-partial-allowed = Permitidas
copy-rule-partial-full-only = Solo salida completa
copy-rule-partial-size = Tamaño de la salida parcial
copy-rule-trailing-activation = Se activa con una ganancia de
copy-rule-trailing-distance = Vende por debajo del máximo en
copy-rule-take-profit-target = Vende con una ganancia de
copy-rule-time-duration = Comprueba tras mantener
copy-rule-time-threshold = Vende mientras el P&L esté en o por debajo de
copy-preset-inherit = Valores del Trader
copy-preset-conservative = Conservador
copy-preset-balanced = Equilibrado
copy-preset-aggressive = Agresivo
copy-preset-custom = Personalizado
copy-validate-stop-loss = El stop loss debe ser mayor que 0 % y como máximo 100 %.
copy-validate-partial-size = El tamaño de la salida parcial debe estar entre 0 % y 100 %.
copy-validate-min-hold = La tenencia mínima debe ser un número entero de segundos.
copy-validate-trailing-activation = La activación del trailing debe ser mayor que 0 % y como máximo 100 %.
copy-validate-trailing-distance = La distancia del trailing debe ser mayor que 0 % y como máximo 100 %.
copy-validate-take-profit = El take profit debe ser mayor que 0 %.
copy-validate-time-duration = La regla de tiempo necesita una duración mayor que cero.
copy-validate-time-threshold = El umbral de la regla de tiempo es una pérdida: usa 0 % o un número negativo.
copy-warning-mirror = Solo las ventas de la billetera cierran las tenencias: ningún stop loss las protege, y un token que la billetera nunca vende se queda en cartera.
copy-warning-no-rules = Ninguna regla de salida está activa y las ventas de la billetera se ignoran: las tenencias nunca se venden.
copy-warning-no-stop-loss = No se aplica ningún stop loss: un token a la baja se mantiene hasta que otra regla o la billetera venda.
copy-warning-stop-delay = El stop loss espera { $hold } tras cada compra: un token que caiga más rápido se cierra bastante más allá de { $threshold }.
copy-warning-take-profit-cost = El take profit en { $target } no cubre la venta ({ $slippage } de deslizamiento y { $fee } de comisión de swap), así que cierra rondas con pérdida.
copy-warning-trailing-distance = La distancia del trailing es al menos igual a su ganancia de activación, así que un trailing activado puede vender por debajo de la entrada.

## Execution tab

copy-execution-title = Calidad de ejecución
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Cualquiera
copy-execution-limit-on =
    { $count ->
        [one] Pausa por encima de { $limit } de media en { $count } operación
        [many] Pausa por encima de { $limit } de media en { $count } operaciones
       *[other] Pausa por encima de { $limit } de media en { $count } operaciones
    }
copy-execution-limit-off = Interruptor de emergencia desactivado
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } operación vista en el momento
        [many] { $count } operaciones vistas en el momento
       *[other] { $count } operaciones vistas en el momento
    }
copy-execution-p95 = Llegada p95
copy-execution-median-slippage = Deslizamiento mediano
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } ejecución medida
        [many] { $count } ejecuciones medidas
       *[other] { $count } ejecuciones medidas
    }
copy-execution-worst-slippage = Peor deslizamiento
copy-execution-average-slippage = Promedio { $amount }
copy-execution-delay-title = Retraso de detección
copy-execution-delay-note = Tiempo desde el bloque de la billetera hasta que este bot ve la operación. Se excluyen las reproducciones tras una interrupción.
copy-execution-delay-limit = Las barras que superan el límite de llegada de { $limit } se muestran en ámbar.
copy-execution-fastest = Más rápida
copy-execution-average = Promedio
copy-execution-slowest = Más lenta
copy-execution-fill-title = Ejecución frente a la billetera
copy-execution-fill-note = Positivo significa peor que la billetera: pagaste más en una compra o recibiste menos en una venta replicada. Una ejecución simulada de un token sin precio de pool se valora a la operación de la propia billetera, así que no mide nada y se excluye.
copy-execution-samples = Muestras
copy-execution-median = Mediana
copy-execution-worst = Peor
copy-execution-decisions = Decisiones en el rango

## Compare view

copy-compare-title = Comparar billeteras
copy-compare-back = Volver a la billetera
copy-compare-load-failed = No se pudo cargar la comparación: { $error }
copy-compare-loading = Cargando comparación…
copy-compare-empty = No hay tareas que comparar.
copy-compare-empty-message = Añade una tarea de copia para comparar sus resultados con las demás.
copy-compare-curve-title = P&L realizado acumulado
copy-table-wallet = Billetera
copy-table-mode = Modo
copy-table-rounds = Rondas
copy-table-realized = Realizado
copy-table-profit-factor = Factor de beneficio
copy-table-average-hold = Tenencia prom.
copy-table-median-slippage = Deslizamiento mediano

## Charts

copy-chart-curve-label = P&L acumulado { $amount } { -sol }
copy-chart-compare-label = P&L acumulado por tarea
copy-chart-empty-curve = Aún no hay rondas cerradas en este rango.
copy-chart-empty-bars = No hay nada registrado en este rango.
copy-chart-empty-histogram = No hay muestras de llegada en este rango.
copy-chart-empty-compare = No hay rondas cerradas que comparar en este rango.
copy-chart-histogram-title = { $count } de { $total }

## Wallet profile

copy-profile-copy = Copiar esta billetera
copy-profile-copy-other = Copiar con otras reglas
copy-profile-loading = Cargando perfil de la billetera…
copy-profile-watch-title = Seguimiento
copy-profile-watched = En seguimiento
copy-profile-watch-resume-hint = Al reanudar una tarea se vuelve a seguir
copy-profile-watch-add-hint = Al añadir una tarea se empieza a seguir
copy-profile-stream = Flujo
copy-profile-subscribed = Suscrita
copy-profile-not-subscribed = No suscrita
copy-profile-sources =
    { $count ->
        [one] { $count } fuente
        [many] { $count } fuentes
       *[other] { $count } fuentes
    }
copy-profile-last-activity = Última actividad
copy-profile-last-error = Último error
copy-profile-own-wallet = Esta es una de tus propias billeteras; no se puede copiar.
copy-profile-observed-title = Operaciones observadas
copy-profile-observed-none = Este bot aún no ha visto operaciones de esta billetera. Una tarea simulada la observa sin gastar { -sol }.
copy-profile-swaps-seen = Swaps vistos
copy-profile-swaps-seen-note = Swaps distintos de la billetera en todas tus tareas
copy-profile-buys-sells = Compras / ventas
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Tokens operados
copy-profile-first-seen = Vista por primera vez
copy-profile-last-seen = Vista por última vez
copy-profile-tasks-title = Tus tareas sobre esta billetera
copy-table-task = Tarea

## Arm live dialog

copy-arm-acks-left =
    { $count ->
        [one] Queda { $count } confirmación por marcar
        [many] Quedan { $count } confirmaciones por marcar
       *[other] Quedan { $count } confirmaciones por marcar
    }
copy-arm-readiness-title = Preparación según la cartera simulada
copy-arm-exposure-title = Exposición
copy-arm-per-copy = Por copia
copy-arm-budget-left-value = { $left } de { $total } { -sol }
copy-arm-budget-left = Presupuesto en vivo restante
copy-arm-budget-left-note = El gasto simulado se cuenta aparte y no lo consume
copy-arm-exits = Salidas
copy-arm-stop-note = No antes de mantener { $hold }: una caída más rápida cierra más abajo
copy-arm-shared = Esta billetera también la copian { $tasks }: cada tarea copia sus operaciones con su propio presupuesto.
copy-arm-unavailable = La ejecución en vivo no está disponible ahora; consulta la última comprobación.
copy-arm-ack-real-native = { -sol } real: esta tarea puede gastar hasta { $budget } { -sol } de tu billetera, como máximo { $trade } { -sol } por copia.
copy-arm-ack-fees = Las copias en vivo pagan comisiones de red y deslizamiento reales; los resultados simulados no garantizan resultados en vivo.
copy-arm-ack-unready = Algunas comprobaciones de preparación no se han superado. Activar esta tarea de todos modos.
copy-arm-lead = “{ $name }” copiará las operaciones de esta billetera con swaps reales desde tu billetera.
copy-arm-confirmation-missing = No se pudo cargar la confirmación en vivo
copy-arm-armed = Copia en vivo activada
copy-arm-failed = No se pudo activar la copia en vivo

## Holdings tab

copy-holdings-title = Tenencias
copy-holdings-view-label = Vista de tenencias
copy-holdings-view-open = Abiertas ({ $count })
copy-holdings-view-closed = Rondas cerradas ({ $count })
copy-holdings-reset = Restablecer cartera simulada
copy-holdings-live-note = Las copias en vivo son posiciones reales.
copy-holdings-open-positions = Posiciones abiertas
copy-holdings-token-details = Abrir detalles del token
copy-holdings-opened = Abierta { $time }
copy-holdings-no-pool-price = Sin precio de pool
copy-holdings-close = Cerrar
copy-holdings-write-off = Dar de baja
copy-holdings-activity = Actividad
copy-holdings-no-exit-rule = Sin regla de salida
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } en { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = El trail se activa en { $level }
copy-holdings-watch-time = Tiempo ≤ { $level }
copy-holdings-watch-time-until = Tiempo ≤ { $level } en { $span }
copy-holdings-watch-wallet-sells = Ventas de la billetera
copy-holdings-empty = No hay tenencias simuladas abiertas. Las compras copiadas de la billetera aparecen aquí.
copy-holdings-col-token = Token
copy-holdings-col-cost = Costo
copy-holdings-col-entry = Entrada
copy-holdings-col-mark = Precio actual
copy-holdings-col-peak = Máximo
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Reglas de salida
copy-holdings-col-held = Tenencia
copy-holdings-col-actions = Acciones
copy-holdings-col-invested = Invertido
copy-holdings-col-proceeds = Ingresos
copy-holdings-col-exit = Salida
copy-holdings-col-closed = Cerrada
copy-holdings-price-note = Los precios son { -sol } por token. La entrada incluye el deslizamiento y las comisiones de la compra; el máximo y los niveles de salida son relativos a ella, así que una tenencia se abre con su máximo por debajo de la entrada. Pasa el cursor por uno para ver su precio de pool.
copy-holdings-paused-rules = En pausa: sin copias nuevas. Tus reglas de salida siguen cerrando estas tenencias.
copy-holdings-paused-mirror = En pausa: sin copias nuevas. Las ventas de la billetera siguen cerrando estas tenencias.
copy-holdings-paused-hybrid = En pausa: sin copias nuevas. Las ventas de la billetera y tus reglas de salida siguen cerrando estas tenencias.
copy-holdings-closed-load-failed = No se pudieron cargar las rondas cerradas: { $error }
copy-holdings-closed-loading = Cargando rondas cerradas…
copy-holdings-closed-empty = Aún no hay rondas cerradas.
copy-holdings-closed-latest = Últimas { $shown } de { $total } rondas.
copy-holdings-close-title = Cerrar tenencia simulada
copy-holdings-close-message = Vende { $token } en la cartera simulada al precio del pool ({ $price }) con el deslizamiento y las comisiones de la tarea.
copy-holdings-close-confirm = Cerrar tenencia
copy-holdings-write-off-title = Dar de baja tenencia simulada
copy-holdings-write-off-message = { $token } no tiene precio de pool al que vender. Al darlo de baja se cierra a cero y su costo de { $cost } se registra como pérdida.
copy-holdings-keep = Conservar
copy-holdings-written-off = { $token } dado de baja
copy-holdings-closed = { $token } cerrado
copy-holdings-written-off-detail = Cerrada con ingresos cero
copy-holdings-sold-at = Vendido a { $price }
copy-holdings-close-failed = No se pudo cerrar la tenencia
copy-holdings-reset-message = Empezar de nuevo “{ $name }”: se eliminan sus tenencias simuladas, gasto, ejecuciones, salidas y omisiones. Las reglas y la billetera se conservan.
copy-holdings-reset-cancel = Conservar historial
copy-holdings-reset-done = Cartera simulada restablecida
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } decisión eliminada
        [many] { $count } decisiones eliminadas
       *[other] { $count } decisiones eliminadas
    }
copy-holdings-reset-failed = No se pudo restablecer la cartera simulada

## Activity tab

copy-activity-title = Actividad
copy-activity-filter-label = Filtro de actividad
copy-filter-all = Todas
copy-outcome-paper-filled = Compra simulada
copy-outcome-live-submitted = Compra en vivo enviada
copy-outcome-live-confirmed = Compra en vivo confirmada
copy-outcome-live-failed = Compra en vivo fallida
copy-outcome-paper-sell-observed = Venta simulada · vendió la billetera
copy-outcome-live-sell-submitted = Venta en vivo enviada
copy-outcome-live-sell-failed = Venta en vivo fallida
copy-outcome-skipped = Omitida
copy-activity-decision = Decisión
copy-activity-paper-exit = Salida simulada · { $rule }
copy-activity-filled = { $input } a { $price } · la billetera compró { $target }
copy-activity-filled-slippage = { $input } a { $price } · la billetera compró { $target } · deslizamiento { $slippage }
copy-activity-filled-unpriced = { $input } a { $price } · valorada a la operación de la billetera, sin precio de pool
copy-activity-live-sized = { $sized } · la billetera compró { $target }
copy-activity-sell-nothing = La billetera vendió { $amount } · nada en cartera que vender
copy-activity-written-off = Dada de baja a cero: sin precio de pool
copy-activity-sold = { $tokens } tokens por { $proceeds } a { $price }
copy-activity-full-close = Cierre completo
copy-activity-partial-exit = Salida de { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = mínimo { $amount }
copy-activity-skip-maximum = máximo { $value }
copy-activity-skip-stale = { $arrival } de retraso, límite { $limit }
copy-activity-skip-latency = { $average } de media, límite { $limit }
copy-activity-arrival-replayed = Reproducida { $span } después del bloque
copy-activity-arrival-seen = Vista { $span } después del bloque
copy-activity-link-wallet-tx = Tx de la billetera
copy-activity-link-own-tx = Tu tx
copy-activity-only-token = Solo este token
copy-activity-skipped-group = Omitidas ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } token · desde { $since }
        [many] { $tokens } tokens · desde { $since }
       *[other] { $tokens } tokens · desde { $since }
    }
copy-activity-mint-filter =
    .placeholder = Mint del token
    .aria-label = Filtrar por mint del token
copy-activity-clear = Limpiar
copy-activity-load-failed = No se pudo cargar la actividad: { $error }
copy-activity-loading = Cargando actividad…
copy-activity-no-match = Nada coincide con este filtro.
copy-activity-empty = Aún no hay decisiones. Las ejecuciones, salidas y omisiones aparecen aquí a medida que opera la billetera.
copy-activity-load-older = Cargar anteriores
copy-activity-start = Inicio del historial
copy-activity-older-failed = No se pudo cargar la actividad anterior

## Task editor

copy-step-wallet = Billetera
copy-step-sizing = Tamaño
copy-step-entry = Filtros de entrada
copy-step-exits = Salidas
copy-step-review = Revisión
copy-editor-title-edit = Editar { $name }
copy-editor-title-clone = Clonar { $name }
copy-editor-sub-edit = Tarea { $mode } · los cambios se aplican a sus próximas decisiones
copy-editor-sub-clone = Mismas reglas, cartera simulada vacía, empieza en simulación
copy-editor-save-edit = Guardar cambios
copy-editor-save-clone = Crear clon
copy-editor-save-create = Crear tarea simulada
copy-editor-clone-suffix = (copia)
copy-editor-discard-edit = Descartar cambios
copy-editor-discard-create = Descartar esta tarea
copy-editor-discard-edit-message = Tus cambios en “{ $name }” no se han guardado.
copy-editor-discard-create-message = La billetera y las reglas introducidas hasta ahora no se han guardado.
copy-editor-discard-confirm = Descartar
copy-editor-keep-editing = Seguir editando
copy-editor-toast-updated = Tarea actualizada
copy-editor-toast-clone = Clon creado
copy-editor-toast-created = Tarea simulada creada
copy-unit-native = { -sol }
copy-editor-any = Cualquiera
copy-editor-duplicate = Ya la copian { $tasks }. Esta tarea copia las mismas operaciones otra vez, con sus propias reglas y presupuesto.
copy-editor-wallet = Billetera
copy-editor-wallet-identity = La billetera de una tarea es su identidad. Para copiar otra billetera con estas reglas, clona la tarea.
copy-editor-address-label = Dirección de la billetera
copy-editor-address-placeholder = Dirección de billetera de Solana
copy-editor-address-help-clone = Mismas reglas con una cartera simulada vacía. Mantén esta billetera para probar otras reglas con ella, o introduce otra.
copy-editor-address-help-create = La billetera cuyas compras (y, si lo eliges, ventas) copia esta tarea.
copy-editor-name-label = Nombre <em>opcional</em>
copy-editor-name-placeholder = p. ej., Rotador rápido
copy-editor-enabled-title = Procesar las operaciones de la billetera
copy-editor-enabled-help = Desactivado mantiene la tarea en pausa hasta que la reanudes.
copy-editor-note-live = Esta tarea está en vivo: los cambios se aplican a sus próximas copias reales.
copy-editor-note-paper = Las tareas se ejecutan en simulación hasta que las actives: las operaciones se simulan al precio del pool y no se gasta nada.
copy-editor-copy-size = Tamaño de copia
copy-editor-sizing-fixed = Importe fijo
copy-editor-sizing-ratio = Proporción de la operación de la billetera
copy-editor-amount-fixed = Importe por copia
copy-editor-amount-ratio = Proporción de cada operación
copy-editor-amount-help-fixed = Se gasta en cada compra copiada, como mínimo { $minimum }.
copy-editor-amount-help-ratio = De la propia compra de la billetera, hasta el límite por operación.
copy-editor-help-trade-cap = Ninguna copia individual gasta más.
copy-editor-help-token-cap = Total gastado en un token.
copy-editor-help-budget = Todo lo que esta tarea puede gastar a lo largo de su vida; simulación y vivo cuentan cada uno su propio gasto.
copy-editor-preview-title = Lo que cuesta una copia
copy-editor-preview-empty = Introduce el tamaño para ver lo que cuesta una copia.
copy-editor-preview-example = La billetera compra { $target } → tú copias <strong>{ $copy }</strong>
copy-editor-preview-once = Un token admite una sola copia de { $size }, ya que cada token se compra una vez
copy-editor-preview-token-cap =
    { $count ->
        [one] Un token admite como máximo { $count } copia de { $size }
        [many] Un token admite como máximo { $count } copias de { $size }
       *[other] Un token admite como máximo { $count } copias de { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; el presupuesto cubre unas { $count } de ellas. Las comisiones de red y de prioridad se suman.
copy-editor-preview-summary-minimum = { $perToken }; el presupuesto cubre al menos { $count } de ellas. Las comisiones de red y de prioridad se suman.
copy-editor-target-min = Operación mínima de la billetera copiada
copy-editor-target-min-help = Ignora las compras más pequeñas de la billetera. Déjalo vacío para no tener mínimo.
copy-editor-target-max = Operación máxima de la billetera copiada
copy-editor-target-max-help = Ignora las compras más grandes de la billetera. Déjalo vacío para no tener máximo.
copy-editor-buy-once-title = Comprar cada token una vez
copy-editor-buy-once-help = Copia solo la primera compra de un token de la billetera; las siguientes se omiten.
copy-editor-filter-require = Exigir
copy-editor-filter-skip = No exigir
copy-editor-filter-help = Exige que un token supere tu pipeline de filtrado antes de copiarlo.
copy-editor-filter-warning = Con la configuración de filtrado predeterminada casi todos los tokens fallan, así que una tarea que exige aprobarlo no copia nada. Exígelo solo cuando tus filtros aprueben los tokens que opera esta billetera.
copy-editor-exit-both = Ambas
copy-editor-exit-help-buy-only = Tus reglas de abajo venden cada tenencia; las ventas de la billetera se ignoran.
copy-editor-exit-help-hybrid = Lo que ocurra primero: vende la billetera o se activa una de tus reglas.
copy-editor-exit-help-mirror = Las tenencias solo se venden cuando vende la billetera. Tus reglas de salida no se aplican.
copy-editor-who-sells = Quién vende
copy-editor-preset = Preajuste
copy-editor-preset-help = Un preajuste rellena todas las reglas de abajo; puedes ajustar cualquiera después.
copy-editor-mirror-note = Estas reglas no se aplican mientras deciden las ventas de la billetera. Se aplican si cambias a { $mine } o { $both }.
copy-editor-rule-inherit = Valor del Trader
copy-editor-inherit-value = Valor del Trader ({ $value })
copy-editor-rule-aria = Ajuste de { $rule }
copy-editor-rule-empty-uses = Vacío usa el valor del Trader: { $value }
copy-editor-rule-follows = Sigue al Trader: { $summary }
copy-editor-rule-follows-plain = Sigue el ajuste del Trader.
copy-editor-rule-follows-own = Sigue el interruptor del Trader con los valores de esta tarea: { $summary }
copy-editor-rule-off-note = Desactivada para esta tarea, sea cual sea el ajuste del Trader.
copy-task-unnamed = Tarea sin nombre
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = procesa operaciones al guardarse
copy-editor-review-paused = se guarda en pausa
copy-editor-error-address = Introduce una dirección de billetera de Solana válida.
copy-editor-error-sizing = Todos los valores de tamaño deben ser mayores que cero.
copy-editor-error-min-copy = Una copia debe ser de al menos { $minimum }: aumenta el importe por copia.
copy-editor-error-min-cap = Una copia debe ser de al menos { $minimum }: aumenta el límite por operación.
copy-editor-error-trade-cap = El límite por operación no puede superar el límite por token.
copy-editor-error-token-cap = El límite por token no puede superar el presupuesto total.
copy-editor-error-slippage = El deslizamiento debe estar entre { $min } y { $max }.
copy-editor-error-target-limits = Los límites de la operación de la billetera deben ser cero o más.
copy-editor-error-target-order = La operación mínima de la billetera no puede superar la máxima.

## Copy notices

copy-notice-task-unnamed = Tarea n.º { $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Compra simulada copiada
copy-notice-title-paper-sell = Venta simulada copiada
copy-notice-title-paper-closed = Tenencia simulada cerrada
copy-notice-title-paper-exit = Salida simulada: { $rule }
copy-notice-title-live-buy-submitted = Compra en vivo copiada enviada
copy-notice-title-live-buy-confirmed = Compra en vivo copiada confirmada
copy-notice-title-live-buy-failed = Compra en vivo copiada fallida
copy-notice-title-live-sell-submitted = Venta en vivo copiada enviada
copy-notice-title-live-sell-failed = Venta en vivo copiada fallida
copy-notice-title-auto-paused = Tarea de copia en pausa automática
copy-notice-detail-bought = Comprado por { $amount } { -sol }
copy-notice-detail-sold = Vendido por { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent } % de la tenencia
copy-notice-detail-full-close = Cierre completo
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Falló el swap
