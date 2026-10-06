# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = Compra
transactions-type-sell = Venta
transactions-type-swap = Swap
transactions-type-sol-transfer = Transferencia de SOL
transactions-type-token-transfer = Transferencia de token
transactions-type-transfer = Transferencia
transactions-type-dust = Polvo
transactions-type-spam = Spam
transactions-type-ata-create = Cuenta abierta
transactions-type-ata-close = Renta recuperada
transactions-type-ata = Cuenta de token
transactions-type-liquidity-add = Añadir liquidez
transactions-type-liquidity-remove = Retirar liquidez
transactions-type-nft = NFT
transactions-type-program = Llamada a programa
transactions-type-compute = Cómputo
transactions-type-failed = Fallida
transactions-type-unknown = Sin clasificar

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Airdrop de spam ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = Todos los tipos
transactions-filter-transfer = Transferencias
transactions-filter-ata = Renta y cuentas
transactions-filter-liquidity = Liquidez
transactions-filter-program = Llamadas a programas

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = Entrante
transactions-direction-outgoing = Saliente
transactions-direction-internal = Interna
transactions-direction-unknown = Sin clasificar

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = Pendiente
transactions-status-confirmed = Confirmada
transactions-status-finalized = Finalizada
transactions-status-failed = Fallida
transactions-status-success = Exitosa
transactions-status-unknown = Desconocido

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = Creación
transactions-ata-operation-closure = Cierre

## Transactions page (pages/transactions.js)

transactions-toolbar-title = Historial de transacciones
transactions-search =
    .placeholder = Buscar firmas…
    .aria-label = Buscar firmas de transacciones
transactions-load-failed = No se pudieron actualizar las transacciones
transactions-summary-total = Total
transactions-summary-estimate = Estimado
transactions-summary-success = Exitosas
transactions-summary-failed = Fallidas
transactions-filter-wallet = Billetera
transactions-filter-type = Tipo
transactions-filter-direction = Dirección
transactions-filter-status = Estado
transactions-filter-all-directions = Todas las direcciones
transactions-filter-all-statuses = Todos los estados
transactions-wallet-main = Billetera principal
transactions-col-time = Hora
transactions-col-signature = Firma
transactions-col-type = Tipo
transactions-col-direction = Dirección
transactions-col-status = Estado
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Comisiones ({ -sol })
transactions-col-token = Token
transactions-col-router = Enrutador
transactions-col-instructions = Instr.

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = Copiar firma
transactions-dialog-close =
    .title = Cerrar (ESC)
transactions-dialog-tabs-label = Secciones de los detalles de la transacción
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Comisión:
transactions-dialog-loading = Cargando...
transactions-dialog-loading-details = Cargando detalles de la transacción...
transactions-dialog-load-failed = No se pudieron cargar los detalles de la transacción
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = No se pudieron cargar los detalles de la transacción: { $reason }
transactions-dialog-not-found = Transacción no encontrada
transactions-dialog-tab-overview = Resumen
transactions-dialog-tab-balances = Saldos
transactions-dialog-tab-instructions = Instrucciones
transactions-dialog-tab-logs = Registros
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Datos crudos
transactions-dialog-unknown = Desconocido
transactions-dialog-unknown-asset = Activo desconocido
transactions-dialog-unavailable = No disponible

## Transaction details dialog: overview

transactions-dialog-failed-title = La transacción falló
transactions-dialog-no-program-error = No se proporcionó ningún error del programa.
transactions-dialog-story-title = Qué ocurrió
# $router is the routing program name.
transactions-dialog-router-via = vía { $router }
transactions-dialog-flow-paid = Pagado
transactions-dialog-flow-received = Recibido
transactions-dialog-flow-from = De
transactions-dialog-flow-to = A
transactions-dialog-flow-amount = Monto
transactions-dialog-net-wallet-change = Variación neta de la billetera:
transactions-dialog-processed = Procesada en Solana
transactions-dialog-execution-title = Ejecución
transactions-dialog-metric-execution-price = Precio de ejecución
transactions-dialog-metric-effective-received = Recibido efectivo
transactions-dialog-metric-effective-spent = Gastado efectivo
transactions-dialog-metric-network-fee = Comisión de red
transactions-dialog-metric-estimated-pnl = P&L estimado
transactions-dialog-metric-net-native-change = Variación neta de { -sol }
transactions-dialog-route-title = Ruta y activos
transactions-dialog-route-router = Enrutador
transactions-dialog-route-input-asset = Activo de entrada
transactions-dialog-route-output-asset = Activo de salida
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Programa
transactions-dialog-tech-title = Detalles técnicos
transactions-dialog-tech-summary = Firma, slot y recursos
transactions-dialog-tech-signature = Firma
transactions-dialog-tech-timestamp = Marca de tiempo
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Comisión exacta
transactions-dialog-tech-accounts = Cuentas
transactions-dialog-tech-instructions = Instrucciones
transactions-dialog-tech-compute-units = Unidades de cómputo
transactions-dialog-tech-token-decimals = Decimales del token

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = Variaciones de saldo de { -sol }
transactions-dialog-balances-native-empty = Sin variaciones de saldo de { -sol }
transactions-dialog-balances-token-title = Variaciones de saldo de tokens
transactions-dialog-balances-token-empty = Sin variaciones de saldo de tokens
transactions-dialog-balances-net-native = Variación neta de { -sol }
transactions-dialog-balances-fee = Comisión de transacción
transactions-dialog-col-account = Cuenta
transactions-dialog-col-token = Token
transactions-dialog-col-mint = Dirección mint
transactions-dialog-col-pre-balance = Saldo previo
transactions-dialog-col-post-balance = Saldo posterior
transactions-dialog-col-change = Variación
transactions-dialog-col-type = Tipo
transactions-dialog-col-rent = Renta ({ -sol })
transactions-dialog-instructions-empty = No se encontraron instrucciones
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } instrucción
        [many] { $count } instrucciones
       *[other] { $count } instrucciones
    }
transactions-dialog-instruction-program-id = ID del programa
transactions-dialog-instruction-accounts = Cuentas ({ $count })
transactions-dialog-instruction-data = Datos
transactions-dialog-logs-empty = No hay registros disponibles
transactions-dialog-logs-filter = Filtrar registros...
transactions-dialog-logs-no-match = Ningún registro coincide
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } registro
        [many] { $count } registros
       *[other] { $count } registros
    }
transactions-dialog-ata-empty = No hay operaciones ATA en esta transacción
transactions-dialog-ata-summary-title = Resumen del análisis de ATA
transactions-dialog-ata-creations = Creaciones
transactions-dialog-ata-closures = Cierres
transactions-dialog-ata-rent-spent = Renta gastada
transactions-dialog-ata-rent-recovered = Renta recuperada
transactions-dialog-ata-net-rent = Impacto neto de la renta
transactions-dialog-ata-operations-title = Operaciones ATA ({ $count })
transactions-dialog-raw-copy = Copiar JSON
transactions-dialog-raw-empty = No hay datos crudos disponibles
