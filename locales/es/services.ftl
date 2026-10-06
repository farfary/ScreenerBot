services-health-component-unavailable = Componente { $component } no disponible
services-health-unavailable = Estado de salud no disponible
services-health-pools-not-running = El servicio de pools no se está ejecutando
services-health-events-db-uninitialized = Base de datos de eventos sin inicializar
services-health-sol-price-not-running = El servicio de precio de SOL no se está ejecutando
services-health-sol-price-stale = Los datos de precio de SOL están desactualizados ({ $seconds } s de antigüedad)
services-health-sol-price-no-data = Aún no hay datos de precio de SOL
services-health-telegram-discovery = Modo descubrimiento
services-health-telegram-disconnected = Desconectado
services-health-wallet-watch-polling-only = Detección funcionando solo con sondeo
services-health-assistant-tasks-disabled = Desactivado en la configuración
services-health-connectivity-critical-unhealthy = Endpoints críticos con problemas: { $endpoints }
services-health-filtering-snapshot-stale = La instantánea de filtrado tiene { $seconds } s de antigüedad

services-status-healthy = Saludable
services-status-starting = Iniciando
services-status-degraded = Degradado
services-status-unhealthy = Con problemas
services-status-stopping = Deteniendo
services-status-disabled = Desactivado
services-status-unknown = Desconocido

services-name-account = Cuenta
services-name-assistant-scheduled-tasks = Tareas programadas del Asistente
services-name-ata-cleanup = Limpieza de cuentas de token
services-name-connectivity = Conectividad
services-name-copy-trading = Copy trading
services-name-events = Eventos
services-name-filtering = Filtrado
services-name-llm-analysis = Análisis LLM
services-name-ohlcv = OHLCV
services-name-pool-pricing = Precios de pools
services-name-pools = Pools
services-name-positions = Posiciones
services-name-referral = Referidos
services-name-rpc-stats = Estadísticas de RPC
services-name-sol-price = Precio de { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Tokens
services-name-trader = Trader
services-name-transactions = Transacciones
services-name-update-check = Comprobación de actualizaciones
services-name-wallet = Billetera
services-name-wallet-watch = Seguimiento de billeteras
services-name-webserver = Servidor web

services-loading = Cargando servicios...
services-load-failed = Error al cargar los servicios
services-load-failed-description = Esperando la respuesta del backend. Reintentaremos automáticamente.
services-refresh-failed = No se pudieron actualizar los servicios
services-search-placeholder = Buscar servicios...
services-summary-total = Total
services-summary-alerts = Alertas
services-summary-alerts-tooltip = { $degraded } degradados / { $unhealthy } con problemas
services-filter-status = Estado
services-filter-all-statuses = Todos los estados
services-filter-all-services = Todos los servicios
services-filter-enabled-only = Solo activados
services-filter-disabled-only = Solo desactivados
services-col-service = Servicio
services-col-health = Salud
services-col-priority = Prioridad
services-col-uptime = Tiempo activo
services-col-activity = Actividad
services-col-last-cycle = Último ciclo
services-col-avg-cycle = Ciclo prom.
services-col-avg-poll = Sondeo prom.
services-col-cycle-rate = Tasa de ciclos
services-col-tasks = Tareas
services-col-ops = Ops/s
services-col-errors = Errores
services-col-dependencies = Dependencias
services-dependencies-none = Ninguna
services-activity-busy = { $percent } ocupado
services-activity-polls =
    { $count ->
        [one] { $count } sondeo
        [many] { $count } sondeos
       *[other] { $count } sondeos
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } tarea
        [many] { $count } tareas
       *[other] { $count } tareas
    }
    Último: { $last }
    Prom.: { $avg }
    Sondeo: { $poll }
    Inactivo: { $idle }
    Sondeos totales: { $polls }
services-tasks-none = Sin tareas instrumentadas
