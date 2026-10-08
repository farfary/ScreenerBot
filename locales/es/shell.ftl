# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = Abrir el inicio del panel
    .title = Inicio del panel
shell-bot-card =
    .aria-label = Cargando el estado de Auto Trader
shell-bot-label = Auto
shell-bot-status-loading = CARGANDO
shell-bot-today = Hoy
shell-explore-control =
    .aria-label = Modo Explorar. Conecta una billetera y un endpoint RPC para habilitar todas las funciones
    .title = Conecta una billetera y un endpoint RPC para habilitar el trading, los saldos y los datos en vivo de la cadena
shell-explore-title = Modo Explorar
shell-explore-detail = Billetera y RPC sin conectar
shell-explore-action = Completar configuración
shell-wallet-card =
    .aria-label = Valor de la billetera; abrir Posiciones
    .title = Valor de la billetera ({ -sol } + tokens) · abrir Posiciones
shell-wallet-worth-label = VALOR
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = Precio de { -sol } en USD: abrir gráfico
    .title = Precio de { -sol } · clic para ver el gráfico
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading; abrir Copy Trading
    .title = Copy trading · abrir Copy Trading
shell-copy-label = COPIA
shell-actions-more =
    .aria-label = Más acciones del encabezado
    .title = Más acciones
shell-actions-group =
    .aria-label = Acciones del encabezado
shell-action-search =
    .aria-label = Buscar tokens
    .title = Buscar tokens (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Tokens destacados
    .title = Tokens destacados
shell-action-notifications =
    .aria-label = Acciones y notificaciones
    .title = Acciones y notificaciones
shell-action-restart =
    .aria-label = Reiniciar la app
    .title = Reiniciar la app
shell-action-theme =
    .aria-label = Cambiar tema
    .title = Cambiar tema
shell-action-settings =
    .aria-label = Configuración
    .title = Configuración

## Ticker

shell-ticker-monitoring-segment =
    .title = Tokens monitoreados por el servicio de pools
shell-ticker-monitoring = Monitoreo:
shell-ticker-filtering-segment =
    .title = Tokens que aprobaron o no los criterios de filtrado
shell-ticker-passed = Aprobados:
shell-ticker-rejected = Rechazados:
shell-ticker-pnl-segment =
    .title = Ganancias y pérdidas realizadas hoy
shell-ticker-pnl = P&L de hoy:
shell-ticker-rpc-segment =
    .title = Llamadas RPC por minuto y tasa de éxito
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/min
shell-ticker-services-segment =
    .title = Estado de salud de los servicios en segundo plano
shell-ticker-services-loading = Servicios: <strong>Cargando</strong>

## Notification drawer

shell-notification-title = Acciones
shell-notification-mark-all-read =
    .title = Marcar todo como leído
shell-notification-clear-all =
    .title = Borrar todo
shell-notification-close =
    .aria-label = Cerrar
shell-notification-tab-all = Todas
shell-notification-tab-active = Activas
shell-notification-tab-done = Hechas
shell-notification-tab-failed = Fallidas
shell-notification-filter-type-all = Todos los tipos
shell-notification-filter-type-buy = Compra
shell-notification-filter-type-sell = Venta
shell-notification-filter-type-open = Apertura
shell-notification-filter-type-close = Cierre
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Parcial
shell-notification-filter-state-all = Todos los estados
shell-notification-filter-state-in-progress = En curso
shell-notification-filter-state-completed = Completadas
shell-notification-filter-state-failed = Fallidas
shell-notification-filter-state-cancelled = Canceladas
shell-notification-list =
    .aria-label = Notificaciones
shell-notification-empty = Aún no hay acciones
shell-notification-loading-more = Cargando más...
shell-notification-back-to-top =
    .title = Volver arriba

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = Activo
shell-status-bar-memory = Mem
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/min
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos
shell-status-bar-tokens = Tokens

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = Iniciando { -brand }
shell-splash-waiting = Esperando la respuesta del núcleo local.
shell-splash-failed = { -brand } no pudo iniciarse
shell-splash-failed-detail = Revisa el archivo de registro y reinicia la app.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = Núcleo conectado
shell-connection-waiting = Esperando al núcleo…
shell-connection-retry-now = Reintentar ahora
shell-connection-overlay-detail = No se puede acceder al núcleo. El trading está en pausa; se recuperará automáticamente.
shell-connection-restored = Conexión con el núcleo restablecida

# Source: scripts/core/header.js
shell-trader-control-failed = Falló el control del trader
shell-notification-button-unread = Acciones y notificaciones, { $count } sin leer
shell-restart-confirm-title = Reiniciar el bot
shell-restart-confirm-message =
    ¿Seguro que quieres reiniciar el bot?

    Esto hará lo siguiente:
    • Detendrá todos los servicios
    • Reiniciará el proceso
    • Tardará ~10-15 segundos

    Todas las operaciones activas se interrumpirán.
shell-restart-confirm-action = Reiniciar
shell-restart-progress = Reiniciando el bot
shell-restart-failed = Falló el reinicio
shell-restart-failed-status = Falló el reinicio: { $status }
shell-restart-helper-unavailable = El asistente de reinicio automático no está disponible. Recarga el panel en unos momentos.

# Source: scripts/core/router.js
shell-page-title-fallback = Panel
shell-page-load-failed = No se pudo cargar la página
shell-page-offline-detail = No se puede acceder al núcleo en este momento. Esta página se cargará automáticamente cuando se restablezca la conexión.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = EXPLORAR
shell-bot-state-halted = DETENIDO
shell-bot-state-off = DESACTIVADO
shell-bot-state-waiting = ESPERANDO
shell-bot-state-idle = INACTIVO
shell-bot-state-entry-paused = ENTRADAS EN PAUSA
shell-bot-state-running = EN EJECUCIÓN
shell-bot-control-explore = Auto Trader no está disponible en el Modo Explorar. Abre la configuración de billetera y RPC.
shell-bot-control-halted = La parada de emergencia está activa. Abre los controles de Auto Trader.
shell-bot-control-off = Auto Trader está desactivado. Haz clic para activarlo.
shell-bot-control-waiting = Auto Trader está activado y espera a los servicios del núcleo. Haz clic para desactivarlo.
shell-bot-control-idle = Auto Trader está activado, pero ambos monitores están desactivados. Abre los controles de Auto Trader.
shell-bot-control-entry-paused = La protección contra pérdidas pausó las entradas; las salidas pueden continuar. Abre los controles de Auto Trader.
shell-bot-control-running = Auto Trader está en ejecución. Haz clic para desactivarlo.

## Wallet and copy cards

shell-wallet-card-summary = Valor de la billetera: { $equity } { -sol } ({ $balance } { -sol } en efectivo, { $tokens } tokens); abrir Posiciones
shell-copy-running-live = { $count } en vivo
shell-copy-running-paper = { $count } simuladas
shell-copy-value-paused = En pausa
shell-copy-value-idle = Inactivo
shell-copy-sub-active = { $active } de { $total } activas

## Ticker services state

shell-ticker-services-healthy = Servicios: <strong>Saludables</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Servicios: <strong>{ $count } problema</strong>
        [many] Servicios: <strong>{ $count } problemas</strong>
       *[other] Servicios: <strong>{ $count } problemas</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = Solicitud de agente
shell-agent-request-client-fallback = Un agente vinculado
shell-agent-request-message = { $client } quiere ejecutar "{ $tool }" en { -brand }. Esta solicitud { $expiry }.
shell-agent-request-message-arguments = { $client } quiere ejecutar "{ $tool }" en { -brand }. Argumentos: { $summary }. Esta solicitud { $expiry }.
shell-agent-request-expires-minutes = caduca en { $minutes } min
shell-agent-request-expires-seconds = caduca en { $seconds } s
shell-agent-request-approve = Aprobar
shell-agent-request-deny = Denegar

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } copiado
shell-toast-copy-failed = Falló la copia
shell-toast-still-running = Sigue en curso: revisa el centro de notificaciones
shell-toast-dismiss =
    .aria-label = Descartar
shell-confirm-title = Confirmar acción
shell-confirm-message = ¿Estás seguro?

# Source: scripts/core/global_chat.js
shell-assistant-label = Asistente
shell-assistant-dialog =
    .aria-label = Asistente

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = Activo
shell-status-bar-trading-inactive = Inactivo

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } cancelada
shell-action-swap-buy-live = Comprando
shell-action-swap-buy-done = Comprado
shell-action-swap-buy-failed = Compra fallida
shell-action-swap-sell-live = Vendiendo
shell-action-swap-sell-done = Vendido
shell-action-swap-sell-failed = Venta fallida
shell-action-position-open-live = Abriendo posición
shell-action-position-open-done = Abierta
shell-action-position-open-failed = Apertura fallida
shell-action-position-close-live = Cerrando posición
shell-action-position-close-done = Cerrada
shell-action-position-close-failed = Cierre fallido
shell-action-position-dca-live = Añadiendo a la posición
shell-action-position-dca-done = Añadido a
shell-action-position-dca-failed = Adición fallida
shell-action-partial-exit-live = Salida parcial
shell-action-partial-exit-done = Salida parcial
shell-action-partial-exit-failed = Salida parcial fallida
shell-action-manual-order-live = Colocando orden
shell-action-manual-order-done = Orden colocada
shell-action-manual-order-failed = Orden fallida
shell-action-trade-live = Operación
shell-action-trade-done = Operación realizada
shell-action-trade-failed = Operación fallida
shell-action-via-router = { $action } vía { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = evitando { $venue }
shell-action-cost-guard-avoiding-cost = evitando { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = evitando una plataforma
shell-action-cost-guard-avoiding-unnamed-cost = evitando una plataforma · { $cost }
shell-action-cost-guard-avoided = { $outcome } · se evitaron { $cost } de renta en { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · se evitaron { $cost } de renta de plataforma
shell-action-exit-full = Salida total
shell-action-exit-percent = Salida del { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = ¿Cerrar { -brand }?
shell-exit-description = Elige cómo quieres cerrar la aplicación
shell-exit-minimize = Minimizar a la bandeja
shell-exit-minimize-detail = Sigue funcionando en segundo plano
shell-exit-quit = Salir de la app
shell-exit-quit-detail = Cierra por completo y detiene todos los servicios

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = Guardar imagen
shell-lightbox-close =
    .title = Cerrar (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = Claro
shell-theme-dark = Oscuro
shell-theme-switch-to-light = Cambiar al tema claro
shell-theme-switch-to-dark = Cambiar al tema oscuro
