# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = Generada
wallets-type-imported = Importada
wallets-type-migrated = Migrada

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = Pausado por ti
wallets-watch-disabled-signature-budget = Pausado: se alcanzó el límite de { $limit } firmas por revisión antes de ponerse al día
wallets-watch-disabled-unknown = Pausado: no se pudo leer el motivo de seguridad guardado del seguimiento
wallets-watch-disabled-helius-unavailable = Pausado: el proveedor de alta actividad no está disponible; cursor conservado
wallets-watch-disabled-processing-failed = Pausado: no se pudo procesar la actividad de la billetera; cursor conservado

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = El proveedor de alta actividad no está disponible; seguimiento en pausa
wallets-watch-error-provider-repeated-failure = Las revisiones de { -helius } fallaron repetidamente; seguimiento en pausa
wallets-watch-error-processing-repeated-failure = El procesamiento de la actividad de la billetera falló repetidamente; seguimiento en pausa
wallets-watch-error-position-unreadable = El seguimiento no pudo leer su posición guardada; reintentando
wallets-watch-error-provider-check-failed = Falló la revisión del proveedor de alta actividad; reintentando
wallets-watch-error-decode-failed = No se pudo decodificar una transacción de alta actividad; cursor conservado
wallets-watch-error-processing-failed = No se pudo procesar la actividad de la billetera; reintentando
wallets-watch-error-position-save-failed = El seguimiento no pudo guardar su posición; reintentando

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = Pausado por ti.
wallets-watch-reason-signature-budget = Esta billetera tiene más actividad de la que su seguimiento actual puede revisar.
wallets-watch-reason-helius-unavailable = Fallaron las revisiones de { -helius }. El progreso guardado se conserva.
wallets-watch-reason-processing-failed = No se pudo procesar la actividad de la billetera. El progreso guardado se conserva.

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = Cerrar ventana
wallets-summary-native = { -sol }
wallets-copied-private-key = Clave privada

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Billetera principal
wallets-tab-secondaries = Secundarias
wallets-tab-archive = Archivo
wallets-tab-watched = Vigiladas
wallets-refresh-failed = No se pudieron actualizar las billeteras
wallets-action-failed = Falló
wallets-toast-failed = Falló: { $reason }
wallets-export-busy = Descifrando...
wallets-export-revealed = Clave revelada: manéjala con cuidado

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = Vigilar billetera
wallets-watch-add-address = Dirección de la billetera
wallets-watch-add-address-input =
    .placeholder = Dirección de Solana
wallets-watch-add-address-hint = Registra la actividad en la cadena de la billetera y envía alertas de operaciones según tu configuración de { -telegram }.
wallets-watch-add-label = Etiqueta
wallets-watch-add-label-input =
    .placeholder = Nombre (opcional)
wallets-watch-add-submit = Añadir seguimiento

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = Opciones de seguimiento de billetera
wallets-watch-budget-title-restore = Restaurar seguimiento de billetera
wallets-watch-budget-close =
    .aria-label = Cerrar
wallets-watch-budget-label-signatures = Firmas revisadas por revisión
wallets-watch-budget-label-transactions = Transacciones completas exitosas revisadas por revisión
wallets-watch-budget-hint-signatures = Límite actual: { $limit }. Elige entre 500 y 5,000 firmas por revisión, en pasos de 100.
wallets-watch-budget-hint-transactions = Límite actual: { $limit }. Elige entre 500 y 5,000 transacciones exitosas por revisión, en pasos de 100.
wallets-watch-budget-error-range = Elige entre 500 y 5,000 registros por revisión, en pasos de 100 registros.
wallets-watch-budget-error-ack = Confirma que se omitirán las firmas posteriores a la última revisión completada.
wallets-watch-budget-save-failed = No se pudo guardar el límite de seguimiento.
wallets-watch-budget-save = Guardar límite
wallets-watch-budget-resume = Reanudar desde ahora
wallets-watch-budget-resume-notice = Esta billetera alcanzó su límite de revisión antes de ponerse al día. Reanudar desde ahora empieza en la actividad más reciente de la billetera; la actividad posterior a la última revisión completada no se copiará.
wallets-watch-budget-resume-tasks = Las tareas de copia siguen en pausa hasta que reanudes cada una en Copy Trading.
wallets-watch-budget-resume-ack = Entiendo que la actividad omitida no se copiará.
wallets-watch-budget-resumed = Seguimiento reanudado desde el punto actual de la billetera
wallets-watch-budget-updated = Límite de seguimiento de billetera actualizado
wallets-watch-helius-allow = Permitir la puesta al día con { -helius } si es necesario
wallets-watch-helius-try = Intentar ponerse al día con { -helius }
wallets-watch-helius-stop = Detener la puesta al día con { -helius } para esta billetera
wallets-watch-helius-description-approved = La puesta al día con { -helius } está permitida para esta billetera. Si la desactivas, se vuelve a las revisiones estándar, que pueden quedarse atrás en una billetera con mucha actividad.
wallets-watch-helius-description-available = { -helius } puede revisar transacciones exitosas de Solana desde la posición guardada sin omitir el intervalo sin revisar. Puede consumir más créditos del proveedor y aun así quedarse atrás.
wallets-watch-helius-description-unavailable = La puesta al día con { -helius } no está disponible. Configura un endpoint RPC de { -helius } habilitado para usarla.
wallets-watch-helius-description-unsupported = Ningún proveedor de puesta al día es compatible con este seguimiento. Reanudar desde ahora está disponible si el seguimiento alcanza su límite.
wallets-watch-helius-allow-title = Permitir la puesta al día con { -helius } para esta billetera
wallets-watch-helius-allow-message = { -helius } puede revisar transacciones exitosas de Solana desde la posición guardada sin omitir el intervalo sin revisar. Actualmente cobra 10 créditos por cada 100 transacciones completas devueltas, redondeado hacia arriba, con un mínimo de 10 créditos por solicitud. Una revisión puede hacer varias solicitudes; el uso y los precios del proveedor pueden variar. Las tareas de copia siguen en pausa hasta que se reanuden por separado.
wallets-watch-helius-allow-confirm = Permitir para esta billetera
wallets-watch-helius-stop-message = Esta billetera volverá a las revisiones estándar. Una billetera con mucha actividad puede alcanzar su límite de seguimiento y volver a pausarse. Las demás billeteras y tu configuración RPC de { -helius } no cambian.
wallets-watch-helius-stop-confirm = Detener para esta billetera
wallets-watch-helius-stop-keep = Mantener permitido
wallets-watch-helius-restored = Seguimiento restaurado desde el progreso guardado; las tareas de copia siguen en pausa
wallets-watch-helius-allowed = Puesta al día con { -helius } permitida para esta billetera cuando sea necesario
wallets-watch-helius-stopped = Puesta al día con { -helius } detenida para esta billetera
wallets-watch-helius-update-failed = No se pudo actualizar el ajuste de puesta al día de la billetera

# wallets.html: Export Private Key dialog.
wallets-export-title = Exportar clave privada
wallets-export-warning-title = Advertencia de seguridad crítica
wallets-export-warning-body = Nunca compartas tu clave privada con nadie. Cualquiera con acceso a esta clave puede robar todos los fondos de esta billetera.
wallets-export-key-label = Clave privada (Base58)
wallets-export-copy =
    .title = Copiar al portapapeles
    .aria-label = Copiar al portapapeles
wallets-export-reveal = Revelar clave

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Saldo
wallets-holdings-col-value = Valor ({ -sol })
wallets-holdings-col-type = Tipo
wallets-holdings-col-decimals = Decimales
wallets-holdings-empty-title = Sin tenencias de tokens
wallets-holdings-unreadable-title = Tenencias desconocidas
wallets-holdings-empty-message = Los tokens de esta billetera aparecerán aquí.
wallets-funding-title = Fondea la billetera de trading para empezar
wallets-funding-message = Esta billetera no tiene { -sol }. Envía { -sol } a esta dirección desde un exchange u otra billetera; cada compra gasta su importe más las comisiones de red.
wallets-holdings-no-main = Sin billetera principal
wallets-holdings-main-title = Billetera principal
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Último uso
wallets-holdings-never = Nunca
wallets-holdings-search =
    .placeholder = Buscar por símbolo o mint...
wallets-holdings-export = Exportar clave
wallets-holdings-export-tooltip = Exportar la clave privada de esta billetera
wallets-list-col-name = Nombre
wallets-list-col-balance = Saldo ({ -sol })
wallets-list-col-type = Tipo
wallets-list-col-created = Creada
wallets-list-col-actions = Acciones
wallets-list-action-export = Exportar clave privada
wallets-list-count = Billeteras
wallets-list-search =
    .placeholder = Buscar por nombre o dirección...
wallets-list-loading-title = Cargando billeteras…
wallets-list-loading-description = Preparando la vista de billeteras seleccionada.
wallets-list-kept-empty-message = Las billeteras de versiones anteriores aparecen aquí para que aún puedas exportar sus claves.
wallets-secondaries-empty-title = Sin billeteras secundarias
wallets-archive-empty-title = Sin billeteras archivadas

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = Billetera
wallets-watched-col-status = Estado
wallets-watched-col-progress = Progreso guardado
wallets-watched-col-last-check = Última revisión
wallets-watched-unlabelled = Billetera sin etiqueta
wallets-watched-generic-name = billetera
wallets-watched-not-synced = Aún sin sincronizar
wallets-watched-not-checked = Aún sin revisar
wallets-watched-action-copy = Copiar operaciones
    .title = Abrir esta billetera en Copy Trading
wallets-watched-action-restore = Restaurar seguimiento
wallets-watched-action-options = Opciones de seguimiento
wallets-watched-action-retry = Reintentar seguimiento
wallets-watched-action-pause = Pausar
wallets-watched-action-enable = Activar
wallets-watched-action-remove =
    .title = Quitar
    .aria-label = Quitar { $name }
wallets-watch-state-paused = En pausa
wallets-watch-state-catching-up = Poniéndose al día
wallets-watch-state-watching = Vigilando
wallets-watch-state-streaming = En streaming
wallets-watch-state-polling = En sondeo
wallets-watched-detail-helius = Revisando con { -helius } para esta billetera.
wallets-watched-empty-title = Sin direcciones vigiladas
wallets-watched-empty-message = Usa Vigilar billetera para registrar la actividad en la cadena de una billetera pública.
wallets-watched-count = Vigiladas
wallets-watched-search =
    .placeholder = Buscar billeteras vigiladas...
wallets-watched-add = Vigilar billetera
wallets-watched-refresh = Actualizar billeteras vigiladas
wallets-watched-loading-title = Cargando billeteras vigiladas...
wallets-watched-loading-description = Obteniendo los objetivos de observación.
wallets-watched-load-error-title = No se pudieron cargar las direcciones vigiladas
wallets-watched-load-error-description = Usa actualizar para intentarlo de nuevo.
wallets-watched-address-invalid = Introduce una dirección de billetera de Solana válida.
wallets-watched-added = Seguimiento de billetera añadido
wallets-watched-duplicate = Esa billetera ya está vigilada.
wallets-watched-add-failed = No se pudo añadir el seguimiento de la billetera.
wallets-watched-retried = Seguimiento de billetera restaurado con su cursor guardado
wallets-watched-paused = Seguimiento de billetera en pausa
wallets-watched-enabled = Seguimiento de billetera activado
wallets-watched-removed = Seguimiento de billetera eliminado
wallets-watched-update-failed = No se pudo actualizar el seguimiento de la billetera
wallets-setup-gate-title = Las billeteras requieren configuración
