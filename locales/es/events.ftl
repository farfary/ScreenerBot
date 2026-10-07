# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = Evento OHLCV: { $subtype }
events-filtering-default = Evento de filtrado: { $subtype }
events-trader-default = Evento del trader: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = Tarea '{ $name }' completada
events-task-failed = Tarea '{ $name }' fallida
events-task-timed-out = Tarea '{ $name }' agotó el tiempo

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = Tarea completada
events-subtype-task-failed = Tarea fallida
events-subtype-task-timed-out = Tarea con tiempo agotado

# Shown when an event carries no display text.
events-message-none = Sin mensaje

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = No se pudo limpiar la caché de OHLCV
events-ohlcv-gap-cleanup-failed = No se pudieron limpiar los registros de huecos rellenados
events-ohlcv-gap-fill-failed = Error al rellenar el hueco de { $mint }
events-ohlcv-backfill-scheduled = Relleno histórico multitemporalidad programado para { $mint } mediante { $pool }
events-ohlcv-fetch-failed = No se pudo obtener OHLCV de { $mint } mediante { $pool }: { $error }
events-ohlcv-gap-detection-failed = Falló la detección de huecos de { $mint } mediante { $pool }
events-ohlcv-fetch-success = { $count } puntos OHLCV almacenados para { $mint }
events-ohlcv-retention-backfill-failed = Falló el relleno de retención de { $mint } mediante { $pool }
events-ohlcv-empty-fetch = Obtención de OHLCV vacía para { $mint } mediante { $pool }
events-ohlcv-pool-discovery-failed = Falló el descubrimiento de pools de { $mint }
events-ohlcv-pool-discovery-success = Pools descubiertos para { $mint }
events-ohlcv-process-token-error = Error al procesar { $mint }: { $error }
events-ohlcv-rate-limit-hit = Se activó el límite de velocidad al procesar { $mint }
events-ohlcv-pool-unavailable = No hay pools saludables disponibles para { $mint }; se aplaza
events-ohlcv-token-missing = Faltaba el token { $mint } durante el procesamiento
events-monitors-stopped = Monitores de trading automático detenidos
events-monitors-starting = Monitores de trading automático iniciándose
events-entry-monitor-started = Monitor de oportunidades de entrada iniciado
events-exit-monitor-started = Monitor de salidas y posiciones iniciado
events-trader-service-stopped = Servicio del trader detenido correctamente
events-trader-service-stopping = Apagado del servicio del trader iniciado
events-trader-service-started = Servicio del trader totalmente inicializado y en ejecución
events-trader-auto-trading-error = El trading automático encontró un error
events-trader-trading-enabled = El trading está habilitado y activo
events-trader-trading-disabled = El trading está desactivado en la configuración
events-trader-service-initializing = Iniciando la inicialización del servicio del trader
events-connectivity-monitoring-stopped = Monitoreo de conectividad detenido
events-connectivity-monitoring-started = Monitoreo de conectividad iniciado (intervalo={ $seconds }s)
events-connectivity-service-initialized = Servicio de conectividad inicializado con { $count } monitores
events-connectivity-critical-unhealthy = { $count } endpoint(s) críticos no saludables: el sistema debería pausar las operaciones
events-connectivity-endpoint-recovered = El endpoint pasó de { $from } a saludable
events-position-entry-not-landed = La compra de { $symbol } no llegó a la cadena; su posición se eliminó
events-position-fill-after-force-close = Una operación de { $symbol } llegó a la cadena después del cierre forzado de su posición; se registró y la posición se recalculó

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = Swap
events-category-transaction = Transacción
events-category-pool = Pool
events-category-position = Posición
events-category-token = Token
events-category-wallet = Billetera
events-category-trader = Trader
events-category-entry = Entrada
events-category-system = Sistema
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Seguridad
events-category-connectivity = Conectividad
events-category-filtering = Filtrado
events-category-scheduled-task = Tarea programada
events-category-learner = Aprendizaje
events-category-other = Otros

events-loading = Cargando eventos...
events-load-failed = No se pudieron cargar los eventos
events-load-failed-description = Esperando la respuesta del backend. Reintentaremos automáticamente.
events-load-error = No se pudieron cargar los eventos
events-search-placeholder = Buscar eventos...
events-summary-total = Total
events-filter-category = Categoría
events-filter-all-categories = Todas las categorías
events-filter-all-severities = Todas las gravedades
events-col-time = Hora
events-col-category = Categoría
events-col-type = Tipo
events-col-severity = Gravedad
events-col-message = Mensaje
events-col-token = Token
events-col-details = Detalles
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count } más

## Event details dialog (ui/events_dialog.js)

events-dialog-title = Detalles del evento
events-dialog-close =
    .aria-label = Cerrar diálogo
events-dialog-payload = Carga útil
events-dialog-copy = Copiar detalles
events-dialog-copy-title =
    .title = Copiar todos los detalles del evento
events-dialog-copy-done = ¡Copiado!
events-dialog-copy-failed = Falló
events-dialog-not-available = N/D
# $category is the category label; shown when an event has no message.
events-dialog-category-event = Evento de { $category }
events-dialog-field-id = ID del evento
events-dialog-field-severity = Gravedad
events-dialog-field-category = Categoría
events-dialog-field-subtype = Subtipo
events-dialog-field-mint = Mint del token
events-dialog-field-reference = Referencia
events-dialog-field-time = Hora del evento
events-dialog-field-age = Antigüedad
events-dialog-field-created = Creado
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = DETALLES DEL EVENTO
events-dialog-export-message = MENSAJE
events-dialog-export-payload = CARGA ÚTIL
events-dialog-export-line = { $label }: { $value }
