# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = Posición creada


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = Abiertas
positions-status-closed = Cerradas
positions-status-archived = Archivadas
positions-origin-copy = Copia
positions-origin-manual = Manual
positions-origin-wallet = Billetera
positions-origin-copy-link =
    .title = Abrir la tarea de copia que abrió esta posición
positions-holding-frozen = Congelado
    .title = La autoridad mint congeló esta cuenta de token: el saldo no se puede transferir ni vender
positions-toolbar-total = Total
positions-toolbar-delete-all = Eliminar todo
positions-search-placeholder = Buscar por símbolo o mint...
positions-filter-origin = Origen
positions-filter-origin-all = Todos los orígenes
positions-filter-origin-auto = Auto Trader
positions-filter-origin-copy = Copy Trading
positions-delete-all-tooltip = Eliminar permanentemente todas las posiciones archivadas

## Columns

positions-column-token = Token
positions-column-archived-at = Archivada
positions-column-entry-time = Hora de entrada
positions-column-exit-time = Hora de salida
positions-column-avg-entry = Entrada prom. ({ -sol })
positions-column-avg-exit = Salida prom. ({ -sol })
positions-column-current-price = Actual ({ -sol })
positions-column-total-invested = Total invertido
positions-column-proceeds = Ingresos
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Tamaño
positions-column-dca = DCA
positions-column-exits = Salidas
positions-column-unrealized-pnl = P&L no realizado
positions-column-unrealized-percent = No realizado %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = Sin costo base en el historial de esta billetera (airdrop, ejecución cotizada en USD o swap sin tramo en SOL)
positions-unknown-history = Esta ronda no concuerda con el saldo en la cadena
positions-dca-count =
    { $count ->
        [one] { $count } DCA
        [many] { $count } DCA
       *[other] { $count } DCA
    }
positions-exit-count =
    { $count ->
        [one] { $count } salida
        [many] { $count } salidas
       *[other] { $count } salidas
    }

## Row actions

positions-action-add =
    .title = Añadir a la posición (DCA)
    .aria-label = Añadir a la posición
positions-action-sell =
    .title = Vender (total o parcial en %)
    .aria-label = Vender posición
positions-action-sell-frozen = Congelado por la autoridad mint: esta tenencia no se puede vender
positions-action-remove =
    .title = Quitar (archivar o eliminar)
    .aria-label = Quitar posición
positions-action-restore =
    .title = Restaurar a Abiertas/Cerradas
    .aria-label = Restaurar posición
positions-action-delete =
    .title = Eliminar permanentemente
    .aria-label = Eliminar permanentemente
positions-action-in-progress = En curso…

## Live state of a row

positions-caption-buying = Comprando
# $step is the label of the current action step.
positions-caption-buying-step = Comprando · { $step }
positions-caption-selling = Vendiendo
positions-caption-selling-step = Vendiendo · { $step }
positions-caption-closing = Cerrando
positions-caption-failed = Fallida
# $error is the failure text of the action.
positions-caption-failed-detail = Fallida · { $error }
positions-step-adding = Añadiendo
positions-pending-buying = Comprando…
positions-pending-buy-failed = Compra fallida

## Messages and confirmations

positions-load-failed = No se pudieron actualizar las posiciones
positions-toast-not-found = No se encontraron los datos de la posición
positions-toast-deleted = Posición eliminada
positions-toast-archived = Posición archivada
positions-toast-restored = Posición restaurada
positions-action-failed = La acción falló
positions-delete-title = Eliminar posición permanentemente
# $symbol is the token symbol.
positions-delete-message = ¿Eliminar { $symbol } de forma permanente? Se borrarán la posición y su historial de la base de datos, y no se puede deshacer. Tus transacciones y los datos del token no se ven afectados.
positions-delete-confirm = Eliminar permanentemente
positions-delete-all-title = Eliminar todas las posiciones archivadas
positions-delete-all-message =
    { $count ->
        [one] ¿Eliminar permanentemente { $count } posición archivada? No se puede deshacer. Las transacciones y los datos de tokens no se ven afectados.
        [many] ¿Eliminar permanentemente { $count } posiciones archivadas? No se puede deshacer. Las transacciones y los datos de tokens no se ven afectados.
       *[other] ¿Eliminar permanentemente { $count } posiciones archivadas? No se puede deshacer. Las transacciones y los datos de tokens no se ven afectados.
    }
positions-delete-all-message-empty = ¿Eliminar permanentemente todas las posiciones archivadas? No se puede deshacer.
positions-delete-all-confirm = Eliminar todo
positions-delete-all-done =
    { $count ->
        [one] { $count } posición archivada eliminada
        [many] { $count } posiciones archivadas eliminadas
       *[other] { $count } posiciones archivadas eliminadas
    }
positions-delete-all-failed = No se pudieron eliminar las posiciones archivadas

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = Quitar posición
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>Esta posición sigue abierta.</strong> El bot mantiene este token. Quitarla libera el espacio de operación y detiene el seguimiento, pero <strong>no</strong> vende. Vende primero si quieres recuperar tus { -sol }.
positions-remove-modes =
    .aria-label = Modo de eliminación
positions-remove-archive = Archivar
positions-remove-recommended = Recomendado
positions-remove-archive-description = La oculta en la pestaña Archivadas. Reversible en cualquier momento: no se vende nada y todas las operaciones quedan registradas.
positions-remove-delete = Eliminar permanentemente
positions-remove-delete-description = Borra esta posición y todo su historial de la base de datos.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = Esto elimina de forma permanente la posición y su historial. <strong>No se puede deshacer.</strong> Tus transacciones y los datos del token no se ven afectados.
positions-remove-confirm-archive = Archivar posición

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = Gestión de la posición cambiada a { $mode }
positions-details-load-failed = No se pudieron cargar los detalles de la posición
positions-details-mint-label = Dirección mint
positions-details-management-failed = No se pudo actualizar la gestión de la posición
positions-details-favorite-add =
    .title = Añadir a favoritos
    .aria-label = Añadir a favoritos
positions-details-favorite-remove =
    .title = Quitar de favoritos
    .aria-label = Quitar de favoritos
positions-details-view-solscan =
    .title = Ver en { -solscan }
    .aria-label = Ver token en { -solscan }
positions-details-close =
    .title = Cerrar (Esc)
    .aria-label = Cerrar
positions-details-chart-section =
    .aria-label = Gráfico de precio
positions-details-loading-chart = Cargando gráfico...
positions-details-activity-section =
    .aria-label = Actividad
positions-details-activity-title = Actividad
positions-details-split-handle =
    .aria-label = Cambiar tamaño del gráfico y la actividad
positions-details-activity-pane =
    .aria-label = Panel de actividad
positions-details-activity-expand =
    .title = Expandir actividad
    .aria-label = Expandir actividad
positions-details-summary-section =
    .aria-label = Resumen de la posición
positions-details-loading = Cargando posición...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = Auto Trader
positions-management-user-only = Solo usuario
positions-management-copy-task = Tarea de copia
positions-management-hybrid = Híbrida
positions-pane-show-chart = Mostrar gráfico
positions-pane-show-activity = Mostrar actividad
positions-pane-restore-activity = Restaurar actividad
positions-pane-expand-chart =
    .title = Expandir gráfico
    .aria-label = Expandir gráfico

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = Riesgo bajo
positions-risk-medium = Riesgo medio
positions-risk-high = Riesgo alto
positions-risk-unknown = Riesgo desconocido
positions-busy-buying = Compra en curso…
positions-busy-selling = Venta en curso…
positions-busy-closing = Cierre en curso…
positions-header-avg-entry = Entrada prom.
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [one] { $count } compra
        [many] { $count } compras
       *[other] { $count } compras
    }
positions-header-exit-price = Precio de salida
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = cerrada { $ago }
positions-header-realized-pnl = P&L realizado
positions-header-usd-note = USD al precio actual de { -sol }
positions-header-returned = Recuperado
# $amount is the formatted SOL amount invested.
positions-header-of-invested = de { $amount } invertidos
positions-header-price = Precio
positions-header-last-price = Último precio
positions-header-pool-ago = pool · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = P&L no realizado
positions-header-pnl-last-price = P&L al último precio
positions-header-value = Valor
positions-header-last-value = Último valor
positions-header-invested = { $amount } invertidos
positions-header-origin-hint = Cómo se abrió esta posición
positions-header-risk-hint = Puntuación de { -rugcheck }: cuanto más baja, más segura
positions-header-frozen = Congelado
    .title = La autoridad mint congeló esta tenencia
positions-header-managed-by = Gestionada por
positions-header-management-select =
    .aria-label = Gestión de la posición

## Entry origin shown in the header badge

positions-origin-unknown = desconocido
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = Copiada · tarea { $task }
positions-origin-manual-entry = Entrada manual
positions-origin-wallet-entry = Entrada de billetera
# $strategy is the strategy id.
positions-origin-auto-strategy = Auto · { $strategy }
positions-origin-auto-entry = Entrada automática

## Swaps that are submitted and not yet booked

positions-pending-adding = Añadiendo
positions-pending-adding-amount = Añadiendo { $amount }
positions-pending-selling = Vendiendo
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = Vendiendo { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · confirmando
    .title = Enviado y en espera de confirmación en la cadena. Las cifras se actualizan cuando se verifique.

## Trade controls

positions-trade-add = Añadir
    .title = Añadir a la posición
positions-trade-sell = Vender
    .title = Vender parte de la posición
positions-trade-close = Cerrar posición
    .title = Vender todo y cerrar
positions-trade-token = Detalles del token
    .title = Abrir detalles del token

## Favorites

positions-favorite-token-fallback = Token
# $symbol is the token symbol.
positions-favorite-added = { $symbol } añadido a favoritos
positions-favorite-removed = { $symbol } quitado de favoritos
positions-favorite-add-failed = No se pudo añadir el favorito
positions-favorite-remove-failed = No se pudo quitar el favorito
positions-favorite-update-failed = No se pudieron actualizar los favoritos

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = Posición
positions-summary-price-path = Trayectoria del precio
positions-summary-network-fees = Comisiones de red
positions-summary-risk = Riesgo
positions-summary-market = Mercado
positions-summary-market-now = Mercado ahora
positions-summary-links = Enlaces
positions-fact-tokens-fallback = tokens
positions-fact-bought = Comprado
positions-fact-holding = Tenencia
positions-fact-sold = Vendido
positions-fact-realized = Realizado
positions-fact-opened = Abierta
positions-fact-closed = Cerrada
positions-fact-reason = Motivo
positions-fact-archived = Archivada
positions-fact-entry = Entrada
positions-fact-exit = Salida
positions-fact-total = Total
positions-fact-verified = Verificado en la cadena
positions-fact-confirming = Confirmando
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } de lo comprado
positions-fact-share-of-invested = { $percent } de lo invertido
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 entrada
        [one] 1 entrada + { $count } adición
        [many] 1 entrada + { $count } adiciones
       *[other] 1 entrada + { $count } adiciones
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } salida parcial · { $returned } recuperados
        [many] { $count } salidas parciales · { $returned } recuperados
       *[other] { $count } salidas parciales · { $returned } recuperados
    }
# $age is the elapsed time of the hold.
positions-fact-held = mantenida { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } vs. entrada
positions-fact-exit-vs-peak = Salida vs. máximo
positions-fact-now-vs-peak = Ahora vs. máximo
positions-fact-entry-range = Rango de entrada
positions-range-low = Mínimo
positions-range-peak = Máximo
positions-range-now = Ahora
positions-range-label-exit = Precio de entrada y de salida entre el mínimo y el máximo
positions-range-label-now = Precio de entrada y actual entre el mínimo y el máximo
positions-fact-mint-authority = Autoridad mint
positions-fact-freeze-authority = Autoridad de congelación
positions-fact-active = Activa
positions-fact-pool = Pool
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = { $amount } { -sol } de liquidez
positions-fact-market-cap = Capitalización de mercado
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Liquidez
positions-fact-volume-24h = Volumen 24h
positions-fact-price-change = Variación de precio
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holders
positions-link-website = Sitio web
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = No se pudo cargar la actividad
positions-activity-loading = Cargando actividad...
positions-activity-empty = Aún no ha ocurrido nada con este token en esta billetera
positions-activity-filter-empty = Ninguna actividad coincide con este filtro
positions-activity-round-count =
    { $count ->
        [one] { $count } ronda
        [many] { $count } rondas
       *[other] { $count } rondas
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } evento
        [many] { $count } eventos
       *[other] { $count } eventos
    }
positions-activity-pending-count = { $count } pendientes
positions-activity-failed-count = { $count } fallidos
positions-filter-all = Todo
positions-filter-trades = Operaciones
positions-filter-buys = Compras
positions-filter-sells = Ventas
positions-filter-wallet = Billetera
positions-filter-issues = Problemas
positions-activity-filters =
    .aria-label = Filtrar actividad
positions-activity-totals =
    .aria-label = Todas las rondas de este token
positions-activity-realized-all = Realizado, todas las rondas
positions-activity-invested = Invertido
positions-activity-returned = Recuperado
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = Abierta { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = Posición { $index }
positions-activity-this-position = Esta posición
positions-activity-dates-unavailable = Fechas no disponibles
positions-activity-wallet-title = Transacciones de la billetera
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [one] Fuera de cualquier posición · { $range } · { $count } evento
        [many] Fuera de cualquier posición · { $range } · { $count } eventos
       *[other] Fuera de cualquier posición · { $range } · { $count } eventos
    }
positions-details-signature-label = Firma

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = Posición abierta
positions-state-closing = Posición cerrándose
positions-state-closed = Posición cerrada
positions-state-exit-pending = Salida de posición pendiente
positions-state-exit-failed = Salida de posición fallida
positions-state-phantom = Posición fantasma
positions-state-reconciling = Posición en conciliación

## Activity events

positions-event-kind-entry = Entrada
positions-event-kind-dca = Adición
positions-event-kind-partial-exit = Salida parcial
positions-event-kind-exit = Salida
positions-event-kind-buy = Compra de billetera
positions-event-kind-sell = Venta de billetera
positions-event-kind-transfer = Transferencia
positions-event-kind-ata = Cuenta de token
positions-event-kind-other = Transacción
positions-event-state-pending = Pendiente
positions-event-state-failed = Fallida
positions-event-state-synthetic = Sintética
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = Fallida: { $error }
positions-event-tokens-fallback = tokens
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = Compra enviada por { $amount }
positions-event-entry-for = Compra de { $amount } por { $sol }
positions-event-entry = Compra de { $amount }
positions-event-dca-submitted = Adición enviada por { $amount }
positions-event-dca-for = Adición de { $amount } por { $sol }
positions-event-dca = Adición de { $amount }
positions-event-partial-exit-submitted-percent = Salida parcial de { $percent } enviada por { $amount }
positions-event-partial-exit-submitted = Salida parcial enviada por { $amount }
positions-event-sold-percent-for = Venta de { $amount } ({ $percent }) por { $sol }
positions-event-sold-percent = Venta de { $amount } ({ $percent })
positions-event-sold-for = Venta de { $amount } por { $sol }
positions-event-sold = Venta de { $amount }
positions-event-exit-submitted = Salida total de la posición enviada
positions-event-exit-for = Cerrada con { $amount } vendidos por { $sol }
positions-event-exit-closed = Posición cerrada
positions-event-wallet-bought = La billetera compró { $amount } en otro lugar
positions-event-wallet-sold = La billetera vendió { $amount } en otro lugar
positions-event-received = Recibido: { $amount }
positions-event-sent = Enviado: { $amount }
positions-event-transferred = Transferido: { $amount }
positions-event-ata = Actividad de cuenta de token
positions-event-wallet-transaction = Transacción de billetera con { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / token
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = { $amount } de variación en la billetera
positions-event-after-title = Posición tras este evento
positions-event-capital-invested = Capital invertido
positions-event-average-entry = Entrada promedio
positions-event-transfers-title = Transferencias de tokens
positions-event-transfer-amount = Monto
positions-event-transfer-mint = Mint
positions-event-transfer-from = De
positions-event-transfer-to = A
positions-event-no-signature = Sin firma en la cadena
positions-event-click-to-copy = Clic para copiar
positions-event-solscan = { -solscan }
positions-event-token-amount = Monto de tokens
positions-event-trade-price = Precio de la operación
positions-event-sol-amount = Monto en { -sol }
positions-event-cost-basis = Costo base
positions-event-usd-value = Valor en USD
positions-event-network-fee = Comisión de red
positions-event-router = Enrutador
positions-event-slot = Slot
positions-event-chain-status = Estado en la cadena
positions-event-transaction-type = Tipo de transacción
positions-event-direction = Dirección
positions-event-wallet-sol-change = Variación de { -sol } en la billetera
positions-event-instructions = Instrucciones
positions-event-compute-units = Unidades de cómputo
positions-event-accounts = Cuentas
positions-event-record-id = ID de registro
positions-event-time-unavailable = Hora no disponible
positions-event-details = Detalles
positions-event-hide-details = Ocultar detalles

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = Velas
positions-chart-type-line = Línea
positions-chart-type-area = Área
positions-chart-type-group =
    .aria-label = Tipo de gráfico
positions-chart-overlays-group =
    .aria-label = Superposiciones del gráfico
positions-chart-ema = EMA
    .title = Medias móviles exponenciales, 9 y 21
positions-chart-fit = Ajustar
    .title = Encuadrar la vida de esta posición
positions-chart-timeframes-group =
    .aria-label = Temporalidad
positions-chart-pane-group =
    .aria-label = Panel del gráfico
positions-chart-unavailable = Motor de gráficos no disponible
positions-chart-collecting = Recopilando datos del gráfico…
positions-chart-no-data = Aún no hay datos de gráfico para este token
positions-chart-avg-entry = Entrada prom.
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Entrada prom.
positions-chart-legend-avg-entry-off-scale = Entrada prom. (fuera de escala)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } evento sin vela en esta temporalidad
        [many] { $count } eventos sin vela en esta temporalidad
       *[other] { $count } eventos sin vela en esta temporalidad
    }
positions-chart-level = Nivel
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } está por encima de esta vista
positions-chart-level-below = { $label } { $price } está por debajo de esta vista
positions-chart-scale-hint = Arrastra el eje de precio para escalar hasta él
positions-chart-pnl-at-bar = P&L @ barra
positions-chart-click-to-locate = Clic para ubicar
