# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = No se pudo obtener una cotización: revisa tu conexión e inténtalo de nuevo
# Fallback title when the quote request failed without a message.
trade-quote-error-title = No se pudo obtener una cotización
trade-quote-title = Vista previa del swap
trade-quote-refresh =
    .aria-label = Actualizar cotización
    .title = Actualizar cotización
trade-quote-idle = Elige un monto para ver una vista previa de tu swap
trade-quote-loading = Buscando la mejor ruta…
trade-quote-retry = Reintentar
trade-quote-pay = Pagas
trade-quote-receive = Recibes (estimado)
trade-quote-minimum = Mínimo garantizado
    .title = Lo mínimo que puedes recibir tras el deslizamiento máximo. El swap se revierte en lugar de ejecutarse por debajo de ese valor.
trade-quote-impact = Impacto en el precio
trade-quote-slippage = Deslizamiento máx.
trade-quote-platform-fee = Comisión de plataforma
    .title = 0.5%: apoya el desarrollo. Ya está incluida en la cotización de arriba.
trade-quote-network-fee = Comisión de red
trade-quote-route = Ruta
trade-quote-disclaimer = Los precios se actualizan en vivo desde la cadena. El swap se revierte si no puede ejecutarse por encima de tu mínimo garantizado, así que nunca recibes menos de lo mostrado.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = El impacto en el precio de { $impact } supera tu deslizamiento máximo de { $tolerance }%: este monto mueve el pool. Un monto menor se ejecuta más cerca del precio de mercado.

## Units

trade-unit-native = { -sol }
trade-unit-tokens = tokens

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = Comprar token
trade-buy-subtitle = Introduce el monto en { -sol }
trade-buy-confirm = Ejecutar compra
trade-buy-hint = Déjalo vacío para usar el valor predeterminado de la configuración
trade-sell-title = Vender posición
trade-sell-subtitle = Selecciona el porcentaje a vender
trade-sell-confirm = Ejecutar venta
trade-sell-hint = Introduce un valor entre 1 y 100
trade-sell-input = Porcentaje personalizado
    .placeholder = 1-100
trade-add-title = Añadir a la posición
trade-add-subtitle = DCA en la posición existente
trade-add-confirm = Añadir a la posición
trade-add-hint = Déjalo vacío para usar el tamaño de DCA configurado
trade-amount-input = Monto personalizado
    .placeholder = Introduce el monto en { -sol }

## Presets

trade-presets-quick-amount = Monto rápido
trade-presets-quick-sell = Venta rápida
trade-presets-match-entry = Igualar entrada
trade-presets-fixed-amount = Monto fijo
trade-preset-partial = Parcial
trade-preset-half = Mitad
trade-preset-most = Casi todo
trade-preset-full = Salida total
# $label is the preset's amount.
trade-preset-select =
    .aria-label = Seleccionar { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = Cerrar diálogo
trade-input-max = MÁX
    .aria-label = Usar el máximo
trade-slider =
    .aria-label = Control deslizante de monto
trade-context-available = Disponible
trade-context-position-size = Tamaño de la posición
trade-context-holdings = Tenencia
trade-held-badge = En tenencia
    .title = Tienes una posición abierta en este token
trade-manage-title = Gestión manual
trade-manage-description = El Auto Trader no venderá ni hará DCA en esta posición. Desmarca para que gestione las salidas.

## Slippage

trade-slippage-label = Deslizamiento
trade-slippage-presets =
    .aria-label = Valor predefinido de deslizamiento
trade-slippage-auto = Auto
trade-slippage-custom =
    .placeholder = Personalizado
    .aria-label = Porcentaje de deslizamiento personalizado
trade-slippage-note-auto = Auto (según la configuración)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = Auto ({ $pct }% según la configuración)
# $pct is the override as typed.
trade-slippage-note-override = Anulado: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = Deslizamiento alto: podrías recibir hasta { $pct }% menos de lo cotizado.
trade-impact-warning-title = Advertencia de alto impacto en el precio
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = Esta operación tiene un impacto en el precio de <strong>{ $impact }</strong>, que supera tu tolerancia de deslizamiento de <strong>{ $tolerance }%</strong>. Podrías recibir mucho menos de lo esperado.
trade-impact-warning-proceed = Continuar de todos modos

## Validation and verification

trade-error-invalid-number = Número no válido
trade-error-percentage-range = El porcentaje debe estar entre 1 y 100
trade-error-amount-positive = El monto debe ser mayor que 0
trade-error-amount-minimum = Mínimo: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = Saldo insuficiente (se necesitan { $needed } más { $reserve } para comisiones; tienes { $balance })
trade-error-position-closed = Esta posición ya no está abierta.
trade-error-verify-failed = No se pudo verificar el saldo del token
trade-error-position-missing = Posición no encontrada: puede que se haya cerrado
# $expected and $current are formatted token amounts.
trade-error-balance-changed = El saldo del token cambió. Esperado: { $expected }; ahora: { $current }. Actualiza.
trade-error-verify-network = Error de red al verificar el saldo

## Quick trade

trade-quick-buy-title = Compra rápida
trade-quick-sell-title = Venta rápida
trade-quick-subtitle = Introduce la dirección mint del token
trade-quick-mint-label = Introduce la dirección mint del token
trade-quick-mint-input =
    .placeholder = Introduce la dirección mint o busca por símbolo...
trade-quick-paste =
    .aria-label = Pegar desde el portapapeles
trade-quick-recent = Recientes:
trade-quick-fetching = Obteniendo información del token...
trade-quick-continue = Continuar
trade-quick-token-not-found = Token no encontrado
trade-quick-token-failed = No se pudo obtener el token
trade-quick-token-not-in-database = Token no encontrado en la base de datos
trade-quick-token-info-failed = No se pudo obtener la información del token
trade-quick-no-position = No se encontró ninguna posición para este token
trade-quick-no-holdings = La posición no tiene tokens restantes
trade-quick-position-failed = No se pudieron obtener los datos de la posición

## Manual trade toasts

trade-toast-no-mint = No hay dirección mint disponible
trade-toast-open-failed = No se pudo abrir el diálogo de operación
trade-toast-pending-buy = La compra sigue en curso
trade-toast-pending-add = La adición sigue en curso
trade-toast-pending-sell = La venta sigue en curso
trade-toast-pending-message = El navegador dejó de esperar; revisa la fila de la posición para ver el resultado
trade-toast-failed-buy = Compra fallida
trade-toast-failed-add = Adición a la posición fallida
trade-toast-failed-sell = Venta fallida

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = Señal de estrategia
trade-reason-manual-entry = Entrada manual
trade-reason-force-buy = Compra forzada
trade-reason-copy-buy = Compra por copia
trade-reason-dca-scheduled = DCA programado
trade-reason-take-profit = Take profit
trade-reason-stop-loss = Stop loss
trade-reason-trailing-stop = Trailing stop
trade-reason-time-override = Anulación por tiempo
trade-reason-strategy-exit = Salida de estrategia
trade-reason-llm-analysis-exit = Salida por análisis LLM
trade-reason-manual-exit = Salida manual
trade-reason-risk-management = Gestión de riesgo
trade-reason-blacklisted = En lista negra
trade-reason-force-sell = Venta forzada
trade-reason-copy-sell = Venta por copia
trade-reason-closed-externally = Cerrada externamente
trade-reason-wallet-history = Historial de la billetera
trade-reason-exit-retry-pending = Reintento de salida pendiente
trade-reason-synthetic-exit-permanent-failure = Fallo permanente de salida sintética
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (pendiente de verificación)
# $note is the operator text of a force close.
trade-reason-force-closed = Cierre forzado: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = Ningún token seleccionado
