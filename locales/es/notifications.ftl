notifications-action-swap-buy = Compra
notifications-action-swap-sell = Venta
notifications-action-position-open = Apertura
notifications-action-position-close = Cierre
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Salida parcial
notifications-action-manual-order = Manual
notifications-action-unknown = Acción

actions-step-evaluate = Evaluando
actions-step-validate = Validando
actions-step-quote = Obteniendo cotización
actions-step-swap = Ejecutando swap
actions-step-verify = Verificando
actions-step-unknown = Procesando
actions-step-evaluate-short = Evaluando
actions-step-validate-short = Comprobando
actions-step-quote-short = Cotización
actions-step-swap-short = Swap
actions-step-verify-short = Confirmando
actions-step-unknown-short = En curso

actions-failure-recorded = { $message }
actions-failure-unknown = Error desconocido
actions-failure-interrupted = Interrumpida por el reinicio de la aplicación
actions-failure-validation = Falló la validación
actions-failure-quote = Falló la cotización
actions-failure-swap = Falló el swap
actions-failure-trade = Falló la operación
actions-failure-entry = Falló la entrada
actions-failure-exit = Falló la salida
actions-failure-dca = Falló el DCA
actions-failure-verification-expired = Verificación caducada: la transacción nunca se confirmó
actions-failure-verification-gave-up = Se abandonó la verificación
actions-failure-transaction-failed = La transacción falló en la cadena
actions-failure-sell-transaction-failed = La transacción de venta falló en la cadena
actions-failure-dca-verification-failed = Falló la verificación del DCA

notifications-empty-all = Sin acciones
notifications-empty-active = Sin acciones activas
notifications-empty-completed = Sin acciones completadas
notifications-empty-failed = Sin acciones fallidas
notifications-source-auto = Auto
notifications-source-manual = Manual
notifications-state-locked = El estado lo controla la pestaña
notifications-cancelled = Cancelada
notifications-dismiss = Descartar
notifications-dismiss-failed = No se pudo descartar la notificación
notifications-load-failed = No se pudo cargar
notifications-mark-read-failed = No se pudieron marcar las notificaciones como leídas
notifications-clear-title = Borrar notificaciones
notifications-clear-message = ¿Descartar todas las notificaciones de esta lista? Seguirán en el historial de Completadas/Fallidas.
notifications-clear-failed = No se pudieron borrar las notificaciones
notifications-stream-lag-title = El flujo de acciones se retrasó
notifications-stream-lag-missed =
    { $count ->
        [one] Se omitió { $count } actualización
        [many] Se omitieron { $count } actualizaciones
       *[other] Se omitieron { $count } actualizaciones
    } — actualizando
notifications-stream-lag-refreshing = Actualizando
notifications-sync-failed = No se pudieron actualizar las acciones
