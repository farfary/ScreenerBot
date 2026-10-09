# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = La configuración en memoria difiere de la versión en disco
system-result-config-matches = La configuración en memoria coincide con la versión en disco

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    { $count ->
        [one] Se importó { $count } sección correctamente
        [many] Se importaron { $count } secciones correctamente
       *[other] Se importaron { $count } secciones correctamente
    }
system-result-config-imported-with-warnings =
    { $count ->
        [one] Se importó { $count } sección
        [many] Se importaron { $count } secciones
       *[other] Se importaron { $count } secciones
    } con { $warnings ->
        [one] { $warnings } advertencia
        [many] { $warnings } advertencias
       *[other] { $warnings } advertencias
    }: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = Buscar ajustes...
system-config-export-title =
    .title = Exportar configuración a un archivo
system-config-import-title =
    .title = Importar configuración desde un archivo
system-config-reload = Recargar desde el disco
system-config-reset-defaults = Restablecer valores predeterminados
system-config-select-section = Selecciona una sección de configuración
system-config-select-section-details = Selecciona una sección de configuración para ver los detalles.
system-config-no-metadata = Sin metadatos para <code>{ $section }</code>
system-config-technical-settings = Ajustes técnicos
system-config-expand-title = Expandir todas las secciones y subconfiguraciones anidadas
system-config-collapse-title = Contraer todas las secciones y subconfiguraciones anidadas
system-config-toolbar-no-changes = Sin cambios en la sección
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> cambio en la sección
        [many] <strong>{ $count }</strong> cambios en la sección
       *[other] <strong>{ $count }</strong> cambios en la sección
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> cambio en total
        [many] <strong>{ $count }</strong> cambios en total
       *[other] <strong>{ $count }</strong> cambios en total
    }

## Config page: state banner

system-config-loading = Cargando configuración…
system-config-refreshing = Actualizando configuración…
system-config-saving-title = Guardando cambios…
system-config-saving-detail = Actualizando la configuración
system-config-validation-issues = <strong>Se detectaron problemas de validación.</strong> Revisa los campos resaltados.

## Config page: section header and category chips

system-config-save-changes = Guardar cambios
system-config-saving = Guardando…
system-config-compare = Comparar con el disco
system-config-revert-section = Revertir sección
system-config-summary-critical = { $count } críticos
system-config-summary-performance = { $count } de rendimiento
system-config-summary-pending =
    { $count ->
        [one] { $count } cambio pendiente
        [many] { $count } cambios pendientes
       *[other] { $count } cambios pendientes
    }
system-config-summary-none = Sin resumen de metadatos
system-config-fields-count =
    { $count ->
        [one] { $count } campo
        [many] { $count } campos
       *[other] { $count } campos
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · { $pending } pendientes
system-config-chip-visible = { $visible } de { $fields }

## Config page: field rows

system-config-field-default = Predeterminado: { $value }
system-config-field-reset = Restablecer al valor predeterminado
system-config-array-invalid-title = Entrada de arreglo no válida
system-config-json-invalid-title = JSON no válido
system-config-list-separator = { ", " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [one] La línea { $lines } debe ser un entero válido.
        [many] Las líneas { $lines } deben ser enteros válidos.
       *[other] Las líneas { $lines } deben ser enteros válidos.
    }
system-config-array-invalid-number =
    { $count ->
        [one] La línea { $lines } debe ser un número válido.
        [many] Las líneas { $lines } deben ser números válidos.
       *[other] Las líneas { $lines } deben ser números válidos.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] La línea { $lines } debe ser un booleano válido.
        [many] Las líneas { $lines } deben ser booleanos válidos.
       *[other] Las líneas { $lines } deben ser booleanos válidos.
    }
system-config-array-invalid-value =
    { $count ->
        [one] La línea { $lines } debe ser un valor válido.
        [many] Las líneas { $lines } deben ser valores válidos.
       *[other] Las líneas { $lines } deben ser valores válidos.
    }

## Config page: Telegram actions

system-config-telegram-actions = Acciones
system-config-telegram-test-title = Probar conexión
system-config-telegram-test-description = Envía un mensaje de prueba para verificar que tu configuración de { -telegram } funciona
system-config-telegram-send-test = Enviar mensaje de prueba
system-config-telegram-sending = Enviando...
system-config-telegram-configure-token-title = Configura primero el token del bot
system-config-telegram-configure-token-status = Configura el token del bot arriba para habilitar la prueba
system-config-telegram-test-sent-status = ¡Mensaje de prueba enviado correctamente! Revisa tu { -telegram }.
system-config-telegram-test-sent = Mensaje de prueba de { -telegram } enviado
system-config-telegram-test-failed = No se pudo enviar el mensaje de prueba
system-config-telegram-auth-title = Autenticación del bot
system-config-telegram-totp-title = Autenticación en dos pasos (TOTP)
system-config-telegram-totp-configured = Configurada
system-config-telegram-totp-not-configured = No configurada
system-config-telegram-totp-active = La autenticación en dos pasos está activa. Las sesiones de { -telegram } caducadas requieren un código TOTP de tu app de autenticación.
system-config-telegram-totp-inactive = Activa la autenticación en dos pasos en los ajustes de Seguridad para proteger los comandos de { -telegram }.
system-config-telegram-totp-note = El TOTP se comparte con la pantalla de bloqueo del panel. Configúralo en los ajustes de Seguridad.
system-config-telegram-require-2fa = Exigir 2FA para los comandos
# $status is the HTTP status code.
system-config-telegram-save-rejected = Guardado rechazado ({ $status })
system-config-telegram-save-failed = No se pudo guardar el ajuste de { -telegram }

## Config page: operations

system-config-saved = Configuración guardada
system-config-save-failed = No se pudo guardar la configuración
system-config-reloaded = Configuración recargada desde el disco
system-config-reload-failed = No se pudo recargar la configuración
system-config-diff-title = Diferencias de configuración
system-config-diff-console = Escrito en la consola del navegador
system-config-diff-failed = No se pudieron calcular las diferencias
system-config-reset-title = Restablecer configuración
system-config-reset-message =
    Esto restablecerá toda la configuración a los valores predeterminados incluidos. Se perderán todos los ajustes actuales.

    Esta acción no se puede deshacer.
system-config-reset-done-title = Configuración restablecida
system-config-reset-done-message = Todos los ajustes se restauraron a sus valores predeterminados
system-config-reset-failed = No se pudo restablecer la configuración
system-config-load-failed = No se pudo cargar la configuración
system-config-metadata-failed = No se pudieron cargar los metadatos de la configuración

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = Cerrar
system-config-select-none = Deseleccionar todo
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } cambio
        [many] { $count } cambios
       *[other] { $count } cambios
    }
system-config-sections-count =
    { $count ->
        [one] { $count } sección
        [many] { $count } secciones
       *[other] { $count } secciones
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-chains = Activación de cadenas, endpoints RPC y enrutamiento de swaps
system-config-section-hint-trader = Reglas de trading y automatización
system-config-section-hint-positions = Ajustes de gestión de posiciones
system-config-section-hint-filtering = Reglas y umbrales de filtrado de tokens
system-config-section-hint-tokens = Descubrimiento de tokens y fuentes de datos
system-config-section-hint-events = Ajustes de registro de eventos
system-config-section-hint-services = Ajustes de los servicios en segundo plano
system-config-section-hint-monitoring = Configuración de monitoreo del sistema
system-config-section-hint-ohlcv = Ajustes de datos de velas
system-config-section-hint-gui = Ajustes del panel y de la interfaz
system-config-section-hint-telegram = Configuración del bot de { -telegram }

## Export dialog

system-config-export-dialog-title = Exportar configuración
system-config-export-intro = Selecciona qué secciones de la configuración exportar. El archivo exportado se puede importar más tarde para restaurar o compartir los ajustes.
system-config-export-sections = Secciones
system-config-export-timestamp = Incluir marca de tiempo de la exportación
system-config-sections-selected =
    { $count ->
        [one] { $count } sección seleccionada
        [many] { $count } secciones seleccionadas
       *[other] { $count } secciones seleccionadas
    }
system-config-exporting = Exportando...
system-config-export-invalid-response = Respuesta no válida del servidor
system-config-exported-title = Configuración exportada
system-config-exported-message =
    { $count ->
        [one] Se exportó { $count } sección
        [many] Se exportaron { $count } secciones
       *[other] Se exportaron { $count } secciones
    }
system-config-export-failed-title = Error al exportar
system-config-export-failed = No se pudo exportar la configuración

## Import dialog

system-config-import-dialog-title = Importar configuración
system-config-import-upload-intro = Sube un archivo de configuración exportado anteriormente. Podrás previsualizarlo y elegir qué secciones importar.
system-config-import-dropzone-title = Suelta aquí el archivo de configuración
system-config-import-dropzone-hint = o haz clic para explorar
system-config-import-analyzing = Analizando configuración...
system-config-import-preview = Vista previa
system-config-import-preview-intro = Revisa las secciones de configuración de abajo. Elige qué secciones importar.
system-config-import-sections = Secciones del archivo
system-config-import-select-valid = Seleccionar todas las válidas
system-config-import-merge-label = Combinar con lo existente
system-config-import-merge-hint = Solo actualiza los campos presentes en el archivo. Sin marcar = reemplaza secciones completas.
system-config-import-save-label = Guardar en el disco
system-config-import-save-hint = Conserva los cambios en config.toml tras importar
system-config-import-selected = Importar seleccionadas
system-config-import-warnings =
    { $count ->
        [one] { $count } advertencia
        [many] { $count } advertencias
       *[other] { $count } advertencias
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = Se ignorará la sección desconocida "{ $section }"
system-config-import-warning-sensitive-field = Importar { $field } puede sobrescribir los ajustes de autenticación
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = No está en el archivo
system-config-import-status-invalid = Configuración no válida
system-config-import-status-unchanged = Sin cambios
system-config-import-not-included = No incluida en el archivo
system-config-import-show-changes = Mostrar cambios
system-config-import-hide-changes = Ocultar cambios
system-config-import-value-current = Valor actual
system-config-import-value-new = Valor nuevo
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } cambio más
        [many] +{ $count } cambios más
       *[other] +{ $count } cambios más
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } elemento
        [many] { $count } elementos
       *[other] { $count } elementos
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } clave
        [many] { $count } claves
       *[other] { $count } claves
    }{ "}" }
system-config-importing = Importando...
system-config-import-failed = Falló la importación
system-config-import-invalid-file-title = Archivo no válido
system-config-import-invalid-file = No se pudo analizar el archivo de configuración
system-config-imported-title = Configuración importada
system-config-imported-message =
    { $count ->
        [one] Se importó { $count } sección
        [many] Se importaron { $count } secciones
       *[other] Se importaron { $count } secciones
    }
system-config-import-failed-title = Error al importar
system-config-import-failed-message = No se pudo importar la configuración
