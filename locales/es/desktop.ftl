# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = Aceptar

## Splash and loading status.

desktop-splash-starting = Iniciando { -brand }
desktop-splash-restarting = Reiniciando { -brand }
desktop-splash-recovering = Recuperando
desktop-splash-opening-dashboard = Abriendo el panel
desktop-splash-checking-dependencies = Comprobando dependencias
desktop-splash-installing-dependencies = Instalando dependencias del sistema
desktop-splash-installing-dependencies-detail = { -brand } necesita el Microsoft Visual C++ Redistributable para funcionar.
desktop-splash-resetting-wallet = Restableciendo los datos de la billetera
desktop-splash-resetting-wallet-detail = Los datos de la billetera existente se respaldan antes de borrarse.
desktop-splash-updating = Actualizando a v{ $version }
desktop-splash-updating-detail = Tu configuración y tus datos se mantienen exactamente como están.
desktop-splash-restoring = Restaurando v{ $version }
desktop-splash-restoring-detail = La actualización v{ $failed } no se inició, así que la versión anterior toma el relevo.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand } no pudo iniciarse
desktop-boot-detail-fallback = El backend se detuvo inesperadamente.
desktop-boot-remedy-label = Cómo solucionarlo
desktop-boot-log-file-label = Archivo de registro:
desktop-boot-action-reset-wallet = Restablecer datos de la billetera y reiniciar
desktop-boot-action-working = Trabajando...
desktop-boot-action-open-logs = Abrir carpeta de registros
desktop-boot-action-copy = Copiar detalles
desktop-boot-action-copied = Copiado
desktop-boot-action-quit = Salir
desktop-boot-subtitle-wallet-mismatch = Se detectó una billetera diferente
desktop-boot-subtitle-port-in-use = Un puerto de red necesario está ocupado
desktop-boot-subtitle-lock-held = { -brand } ya se está ejecutando
desktop-boot-subtitle-config-invalid = Problema de configuración
desktop-boot-subtitle-directory-setup = Problema de almacenamiento
desktop-boot-subtitle-generic = Error de inicio

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand } no pudo iniciarse
desktop-boot-error-remedy = Abre la carpeta de registros para ver qué ocurrió y reinicia la app. Si el problema continúa, contacta con soporte en t.me/screenerbotio_support.
desktop-boot-error-default = El backend se detuvo inesperadamente antes de que el panel estuviera listo.
desktop-boot-error-restore-failed = El backend actualizado falló y no se pudo restaurar la versión anterior ({ $error }).
desktop-boot-error-spawn-failed = No se pudo iniciar el programa del backend ({ $error }).
desktop-boot-error-spawn-missing = No se pudo iniciar el programa del backend. Puede que falte o que lo bloquee un software de seguridad.
desktop-boot-error-exited-running = El backend se detuvo mientras el panel estaba en ejecución (código de salida { $code }).
desktop-boot-error-exited-early = El backend se detuvo antes de que el panel estuviera listo (código de salida { $code }).
desktop-boot-error-dashboard-load = El panel no se pudo cargar ({ $description }, { $code }).
desktop-boot-error-renderer-gone = El renderizador del panel se detuvo ({ $reason }).
desktop-boot-error-unresponsive = El panel dejó de responder.
desktop-boot-error-url-failed = No se pudo cargar la URL del panel ({ $error }).
desktop-boot-error-relaunch-setup = No se pudo reiniciar el backend tras la configuración.
desktop-boot-error-relaunch-recovery = No se pudo reiniciar el backend para la recuperación.
desktop-boot-error-restart-offline = El backend no volvió a estar en línea tras el reinicio.
desktop-boot-error-recovery-offline = La recuperación terminó, pero el backend no quedó listo.
desktop-boot-error-start-timeout = El backend no terminó de iniciarse a tiempo. Esto puede ocurrir en un primer arranque lento o si otro programa bloquea la conexión.

## System tray.

desktop-tray-tooltip = { -brand } - Bot de trading de Solana
desktop-tray-show = Mostrar { -brand }
desktop-tray-open-dashboard = Abrir el panel
desktop-tray-quit = Salir de { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = Abrir carpeta de datos
desktop-menu-open-logs-folder = Abrir carpeta de registros
desktop-menu-documentation = Documentación
desktop-menu-telegram-support = Soporte en { -telegram }
desktop-menu-check-updates = Buscar actualizaciones...

## Application menu.

desktop-menu-file = Archivo
desktop-menu-edit = Edición
desktop-menu-view = Ver
desktop-menu-window = Ventana
desktop-menu-help = Ayuda
desktop-menu-reset-zoom = Restablecer zoom
desktop-menu-zoom-in = Acercar
desktop-menu-zoom-out = Alejar
desktop-menu-keyboard-shortcuts = Atajos de teclado
desktop-menu-telegram-channel = Canal de { -telegram }
desktop-menu-telegram-community = Comunidad de { -telegram }
desktop-menu-follow-x = Seguir en { -x } ({ -twitter })
desktop-menu-visit-website = Visitar el sitio web
desktop-menu-about = Acerca de { -brand }

## About dialog.

desktop-about-title = Acerca de { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Versión { $version }

    Bot avanzado de trading automático y gestión de billeteras de Solana.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = Atajos de teclado
desktop-shortcuts-message = Atajos de teclado de { -brand }
desktop-shortcuts-body-mac =
    Atajos de teclado:

    Control de la ventana:
      Cmd+M          Minimizar
      Cmd+W          Cerrar ventana
      Cmd+Q          Salir
      Cmd+Ctrl+F     Pantalla completa

    Zoom:
      Cmd++          Acercar
      Cmd+-          Alejar
      Cmd+0          Restablecer zoom

    Navegación:
      Cmd+R          Recargar el panel
      Cmd+Shift+D    Abrir carpeta de datos

    Otros:
      F1             Abrir documentación
      Cmd+Alt+I      Alternar DevTools
desktop-shortcuts-body-other =
    Atajos de teclado:

    Control de la ventana:
      Alt+F4         Salir
      F11            Pantalla completa

    Zoom:
      Ctrl++         Acercar
      Ctrl+-         Alejar
      Ctrl+0         Restablecer zoom

    Navegación:
      Ctrl+R         Recargar el panel
      Ctrl+Shift+D   Abrir carpeta de datos

    Otros:
      F1             Abrir documentación
      Ctrl+Shift+I   Alternar DevTools

## Close confirmation (Windows and Linux).

desktop-close-title = Cerrar { -brand }
desktop-close-message = ¿Qué quieres hacer?
desktop-close-detail = { -brand } puede seguir ejecutándose en segundo plano. El bot de trading seguirá monitoreando y operando mientras esté minimizado en la bandeja del sistema.
desktop-close-minimize = Minimizar a la bandeja
desktop-close-quit = Salir por completo
desktop-close-cancel = Cancelar

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = Falta una dependencia
desktop-vcredist-missing-message = Falta Visual C++ Redistributable
desktop-vcredist-missing-detail = { -brand } requiere Microsoft Visual C++ Redistributable para funcionar. ¿Quieres instalarlo ahora?
desktop-vcredist-install = Instalar y reparar
desktop-vcredist-exit = Salir
desktop-vcredist-not-found-title = Instalador no encontrado
desktop-vcredist-not-found-message = No se pudo localizar { $name } correctamente.
desktop-vcredist-done-title = Instalación completada
desktop-vcredist-done-message = Las dependencias se instalaron correctamente.
desktop-vcredist-done-detail = { -brand } se iniciará ahora.
desktop-vcredist-failed-title = Error de instalación
desktop-vcredist-failed-message = Instala Visual C++ Redistributable manualmente.
