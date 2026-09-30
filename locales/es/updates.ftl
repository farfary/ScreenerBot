# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = La instalación automática está desactivada. La actualización está lista y se aplicará cuando tú decidas.
updates-defer-trading-active = Hay una posición, una operación o una herramienta en curso, así que el reinicio se aplaza. La actualización se aplica automáticamente cuando la app esté inactiva.
updates-defer-needs-installer = Esta versión también actualiza el shell de escritorio, por lo que el instalador debe ejecutarse una vez.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = Descargando la actualización v{ $version }...
updates-apply-started = Instalando la actualización. { -brand } se reinicia y se reconecta automáticamente.
updates-install-opened = Se abrió el instalador de la actualización verificada. Completa el instalador del sistema operativo.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = Instalador abierto
updates-installer-toast-message = { -brand } se cerrará correctamente ahora.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = Estado
updates-tab-release-notes = Notas de la versión
updates-tab-preferences = Preferencias
updates-tab-sections = Secciones de actualización
updates-checking-installation = Comprobando esta instalación...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = Listo para buscar actualizaciones
updates-phase-idle-detail = { -brand } v{ $version } está instalado.
updates-phase-up-to-date-headline = Estás al día
updates-phase-up-to-date-detail = { -brand } v{ $version } es la última versión.
updates-phase-checking-headline = Buscando actualizaciones
updates-phase-checking-detail = Buscando la última versión publicada.
updates-phase-available-headline = La versión { $version } está disponible
updates-phase-downloading-headline = Descargando v{ $version }
updates-phase-verifying-headline = Verificando v{ $version }
updates-phase-verifying-detail = Comprobando la descarga con su suma de verificación publicada.
updates-phase-ready-to-apply-headline = La versión { $version } está lista
updates-phase-ready-to-apply-detail = La actualización se puede instalar ahora con un reinicio breve, o automáticamente en el próximo inicio.
updates-phase-ready-to-install-headline = La versión { $version } está lista
updates-phase-ready-to-install-detail = El instalador de escritorio está listo para terminar esta actualización.
updates-phase-applying-headline = Instalando actualización
updates-phase-applying-detail = { -brand } se está reiniciando con la nueva versión.
updates-phase-applied-headline = Actualizado a v{ $version }
updates-phase-applied-detail = La actualización se instaló. No hace falta nada más.
updates-phase-failed-headline = La actualización no terminó
updates-phase-failed-detail = Inténtalo de nuevo.
updates-phase-check-failed-headline = No se pudo buscar actualizaciones
updates-phase-check-failed-detail = No se pudo acceder al servicio de versiones.
updates-status-unavailable-headline = El estado de la actualización no está disponible
updates-phase-unrecognized-detail = El estado de actualización informado no se reconoce.
updates-status-load-failed-detail = No se pudo cargar el estado de la instalación.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = Actualización del núcleo · { $size } · reinicio breve
updates-kind-full = Actualización de escritorio · { $size } · requiere instalador
updates-size-unknown = tamaño desconocido

updates-action-check-now = Buscar ahora
updates-action-check-again = Buscar de nuevo
updates-action-try-again = Reintentar
updates-action-download = Descargar actualización
updates-action-restart = Reiniciar para actualizar
updates-action-open-installer = Abrir instalador

updates-busy-checking = Buscando...
updates-busy-resuming = Reanudando descarga...
updates-busy-starting-download = Iniciando descarga...
updates-busy-restarting = Reiniciando...
updates-busy-opening-installer = Abriendo instalador...

updates-progress-downloading = Descargando actualización
updates-progress-verifying = Verificando actualización
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } de { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Detalles de la instalación
updates-detail-installed-version = Versión instalada
updates-detail-system = Sistema
updates-detail-last-checked = Última comprobación
updates-detail-never = Nunca
updates-detail-available-version = Versión disponible
updates-detail-download-size = Tamaño de la descarga

updates-version-installed = Instalada
updates-version-available = Disponible

updates-notes-highlights = Novedades
updates-notes-empty-title = Aún no hay notas de la versión
updates-notes-empty-error = No se pudo cargar el historial de versiones. Revisa la conexión e inténtalo de nuevo.
updates-notes-empty-none = Las notas de la versión aparecerán aquí cuando se publique una versión.
updates-notes-history-notice = Se muestra lo que esta instalación ya conoce: no se pudo cargar el historial de versiones.
updates-release-empty = No se indicaron cambios para esta versión.
updates-release-changes =
    { $count ->
        [one] { $count } cambio
        [many] { $count } cambios
       *[other] { $count } cambios
    }

updates-preferences-unavailable-title = Las preferencias de actualización no están disponibles
updates-preferences-unavailable-detail = No se pudo cargar la configuración de actualizaciones.
updates-preference-fallback-name = preferencia de actualización
updates-preference-save-failed = No se pudo guardar { $preference }

updates-request-failed = La solicitud falló
updates-check-request-failed = No se pudo buscar actualizaciones
updates-resume-failed = No se pudo reanudar la descarga de la actualización
updates-download-failed = No se pudo iniciar la descarga de la actualización
updates-apply-failed = No se pudo instalar la actualización
updates-install-failed = No se pudo abrir el instalador de la actualización
updates-apply-confirm-title = Instalar v{ $version }
updates-apply-confirm-message = { -brand } se reinicia con la nueva versión. El trading se detiene unos segundos y se reanuda automáticamente; las posiciones abiertas no se tocan.
updates-install-confirm-title = Ejecutar el instalador
updates-install-confirm-message = Se abre el instalador verificado y { -brand } se cierra correctamente. Completa el instalador y vuelve a abrir { -brand }.

# A release version as displayed.
updates-version-number = v{ $version }
