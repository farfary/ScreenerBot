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
wallets-field-address = Dirección
wallets-field-name = Nombre de la billetera
wallets-field-notes = Notas
wallets-field-private-key = Clave privada
wallets-address-copy = Copiar dirección
wallets-modal-close =
    .aria-label = Cerrar ventana
wallets-this-wallet = esta billetera
wallets-summary-native = { -sol }
wallets-copied-address = Dirección
wallets-copied-mint = Dirección mint
wallets-copied-private-key = Clave privada

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Billetera principal
wallets-tab-secondaries = Secundarias
wallets-tab-archive = Archivo
wallets-tab-watched = Vigiladas
wallets-refresh-failed = No se pudieron actualizar las billeteras
wallets-action-failed = Falló
wallets-toast-failed = Falló: { $reason }
wallets-create-busy = Creando...
wallets-create-fallback = Falló la creación
wallets-create-done = ¡Billetera "{ $name }" creada!
wallets-import-busy = Importando...
wallets-import-failed = Falló la importación
wallets-import-done = ¡Billetera "{ $name }" importada!
wallets-archive-busy = Archivando...
wallets-archive-confirm-text = ¿Seguro que quieres archivar <strong>{ $name }</strong>?
wallets-archive-done = Billetera archivada
wallets-restore-done = Billetera restaurada
wallets-export-busy = Descifrando...
wallets-export-revealed = Clave revelada: manéjala con cuidado
wallets-delete-busy = Eliminando...
wallets-delete-confirm-text = ¿Seguro que quieres eliminar <strong>{ $name }</strong>?
wallets-delete-done = Billetera eliminada permanentemente

# wallets.html: Add Wallet dialog.
wallets-add-title = Añadir billetera
wallets-add-tab-create = Crear nueva
wallets-add-tab-import = Importar existente
wallets-create-name-input =
    .placeholder = p. ej., Billetera de trading
wallets-create-name-hint = Un nombre descriptivo para identificar esta billetera
wallets-create-notes-input =
    .placeholder = Descripción o propósito (opcional)...
wallets-create-submit = Crear billetera
wallets-import-warning-title = Advertencia de seguridad
wallets-import-warning-body = Importa claves privadas solo de fuentes de confianza. Tu clave se cifrará y se guardará de forma segura en este dispositivo.
wallets-import-name-input =
    .placeholder = p. ej., Mi billetera
wallets-import-key-input =
    .placeholder = Cadena base58 o arreglo JSON [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Mostrar u ocultar la clave privada
wallets-import-key-hint = Admite clave codificada en base58 o formato de arreglo de bytes
wallets-import-notes-input =
    .placeholder = Descripción (opcional)...
wallets-import-submit = Importar billetera

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

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = Archivar billetera
wallets-archive-note = Las billeteras archivadas no se usan en ninguna operación, pero se pueden restaurar en cualquier momento.
wallets-archive-confirm = Sí, archivar
wallets-delete-title = Eliminar billetera
wallets-delete-warning-title = ¡Esta acción no se puede deshacer!
wallets-delete-warning-body = Al eliminar esta billetera se borrarán de forma permanente ella y su clave privada cifrada de este dispositivo.
wallets-delete-confirm = Sí, eliminar

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = Importar billeteras
wallets-bulk-import-submit = Importar billeteras
wallets-bulk-step-upload = Subir archivo
wallets-bulk-step-map = Asignar columnas
wallets-bulk-step-results = Resultados
wallets-bulk-import-file-warning-body = Importa archivos solo de fuentes de confianza. Las claves privadas se cifrarán y se guardarán de forma segura en este dispositivo.
wallets-bulk-drop-title = Suelta tu archivo aquí
wallets-bulk-drop-subtitle = o haz clic para explorar
wallets-bulk-drop-formats = Admite CSV y Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Quitar archivo
wallets-bulk-map-subtitle = Asocia las columnas de tu archivo con los campos de la billetera
wallets-bulk-preview-title = Vista previa (primeras 5 filas)
wallets-bulk-summary-valid = <strong>{ $count }</strong> válidas
wallets-bulk-summary-invalid = <strong>{ $count }</strong> no válidas
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> duplicada
        [many] <strong>{ $count }</strong> duplicadas
       *[other] <strong>{ $count }</strong> duplicadas
    }
wallets-bulk-done = Listo
wallets-bulk-file-invalid = Tipo de archivo no válido. Usa archivos CSV o Excel.
wallets-bulk-preview-busy = Procesando...
wallets-bulk-preview-fallback = No se pudo procesar el archivo
wallets-bulk-preview-failed = No se pudo procesar el archivo: { $reason }
wallets-bulk-column-select = -- Seleccionar columna --
wallets-bulk-preview-empty = No se encontraron filas de datos en el archivo
wallets-bulk-preview-status = Estado
wallets-bulk-status-valid = Válida
wallets-bulk-status-duplicate = Duplicada
wallets-bulk-status-invalid = No válida
wallets-bulk-import-busy = Importando...
wallets-bulk-import-toast =
    { $count ->
        [one] { $count } billetera importada
        [many] { $count } billeteras importadas
       *[other] { $count } billeteras importadas
    }
wallets-bulk-import-error = Falló la importación: { $reason }
wallets-bulk-result-success-title = Importación exitosa
wallets-bulk-result-success-detail =
    { $count ->
        [one] { $count } billetera importada correctamente
        [many] Las { $count } billeteras se importaron correctamente
       *[other] Las { $count } billeteras se importaron correctamente
    }
wallets-bulk-result-partial-title = Éxito parcial
wallets-bulk-result-partial-detail = { $imported } importadas, { $failed } fallidas
wallets-bulk-result-failed-title = Importación fallida
wallets-bulk-result-failed-detail =
    { $count ->
        [one] { $count } billetera no se pudo importar
        [many] Las { $count } billeteras no se pudieron importar
       *[other] Las { $count } billeteras no se pudieron importar
    }
wallets-bulk-result-imported = Importadas
wallets-bulk-result-failed = Fallidas

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = Exportar billeteras
wallets-bulk-export-format = Formato
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Incluir billeteras archivadas
wallets-bulk-export-safe-title = Exportación segura
wallets-bulk-export-safe-body = Exporta solo direcciones y metadatos de las billeteras. No incluye claves privadas.
wallets-bulk-export-safe-submit = Exportar direcciones
wallets-bulk-export-or = o
wallets-bulk-export-danger-title = Exportación peligrosa
wallets-bulk-export-danger-body = Incluye las claves privadas en la exportación. Cualquiera con este archivo puede robar tus fondos.
wallets-bulk-export-danger-submit = Exportar con claves privadas
wallets-bulk-export-busy = Exportando...
wallets-bulk-export-done = Billeteras exportadas a { $filename }
wallets-bulk-export-fallback = Falló la exportación
wallets-bulk-export-error = Falló la exportación: { $reason }
wallets-bulk-confirm-title = Confirmar exportación peligrosa
wallets-bulk-confirm-warning =
    { $count ->
        [one] Vas a exportar <strong>{ $count }</strong> clave privada. ¡Es extremadamente peligroso!
        [many] Vas a exportar <strong>{ $count }</strong> claves privadas. ¡Es extremadamente peligroso!
       *[other] Vas a exportar <strong>{ $count }</strong> claves privadas. ¡Es extremadamente peligroso!
    }
wallets-bulk-confirm-risk-steal = Cualquiera con este archivo puede robar todos los fondos
wallets-bulk-confirm-risk-share = Nunca compartas este archivo con nadie
wallets-bulk-confirm-risk-delete = Elimina el archivo inmediatamente después de usarlo
wallets-bulk-confirm-prompt = Escribe la frase de abajo para confirmar
wallets-bulk-confirm-submit = Exportar claves

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Saldo
wallets-holdings-col-value = Valor ({ -sol })
wallets-holdings-col-type = Tipo
wallets-holdings-col-decimals = Decimales
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Sin tenencias de tokens
wallets-holdings-empty-message = Los tokens de esta billetera aparecerán aquí.
wallets-holdings-no-main = Sin billetera principal
wallets-holdings-main-tag = Principal
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
wallets-list-action-archive = Archivar billetera
wallets-list-action-restore = Restaurar billetera
wallets-list-action-delete = Eliminar permanentemente
wallets-list-count = Billeteras
wallets-list-search =
    .placeholder = Buscar por nombre o dirección...
wallets-list-loading-title = Cargando billeteras…
wallets-list-loading-description = Preparando la vista de billeteras seleccionada.
wallets-secondaries-empty-title = Sin billeteras secundarias
wallets-secondaries-empty-message = Crea billeteras adicionales para organizar tu actividad de trading en varias cuentas.
wallets-secondaries-add = Añadir billetera
wallets-archive-empty-title = Sin billeteras archivadas
wallets-archive-empty-message = Las billeteras que archives se guardarán aquí de forma segura para consultarlas más adelante.

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
