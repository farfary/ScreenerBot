startup-wallet-mismatch-title = La billetera cambió
startup-wallet-mismatch-detail =
    La billetera de tu configuración no coincide con la billetera registrada en el historial local de este equipo.

    Billetera actual: { $current }
    Billetera anterior: { $stored }

    Datos locales afectados: { $systems }

    Esto suele ocurrir tras importar otra clave privada o restaurar otra configuración. El trading, las posiciones y el historial pertenecen a la billetera anterior y deben borrarse antes de que la nueva billetera pueda iniciar con seguridad.
startup-wallet-mismatch-systems-default = Transacciones, Posiciones, Historial de la billetera
startup-wallet-mismatch-remedy =
    Borra el historial local de la billetera anterior para continuar (antes se hace una copia de seguridad automática de tus bases de datos):

      - En la app: elige "{ $action }" abajo.
      - Desde una terminal: ejecuta  screenerbot --clean-wallet-data

    Los fondos en la cadena no se ven afectados; solo se restablece el historial local de operaciones y posiciones de este equipo. Las copias de seguridad se guardan en:
      { $path }
startup-recovery-reset-wallet = Restablecer datos de la billetera y reiniciar

startup-port-in-use-title = El puerto de red está ocupado
startup-port-in-use-detail = El puerto del panel { $address } ya está en uso.
startup-port-in-use-remedy = Otro programa está usando el puerto que necesita { -brand }. Cierra ese programa o cambia el puerto del servidor web en Configuración y vuelve a iniciar { -brand }.

startup-lock-held-title = { -brand } ya se está ejecutando
startup-lock-held-detail = Ya hay otra copia de { -brand } ejecutándose en este equipo, por lo que no se puede iniciar una segunda.
startup-lock-held-remedy = Cambia a la ventana que ya está abierta. Si no ves ninguna, cierra cualquier proceso de { -brand } en segundo plano y vuelve a intentarlo. Si el problema persiste después de reiniciar el equipo, el archivo de bloqueo puede estar obsoleto y se puede eliminar de la carpeta de datos (.screenerbot.lock).

startup-config-invalid-title = No se pudo leer la configuración
startup-config-parse-detail = No se pudo analizar config.toml: { $detail }
startup-config-load-parse-detail = Error al cargar la configuración: no se pudo analizar config.toml: { $detail }
startup-config-parse-remedy = No se pudo leer tu archivo de configuración. Restaura una copia de seguridad desde la carpeta de datos o restablece la configuración a los valores predeterminados y vuelve a configurar tu billetera y tu RPC.
startup-config-load-parse-remedy = Restaura una configuración válida o completa la configuración inicial de nuevo.
startup-option-invalid-title = Opción de inicio no válida
startup-option-invalid-remedy = Una opción de línea de comandos no es válida. Inicia { -brand } sin esa opción o corrígela e inténtalo de nuevo.

startup-storage-upgrade-title = No se pudieron actualizar tus datos
startup-storage-upgrade-detail =
    { -brand } no pudo actualizar { $database } a esta versión y se detuvo antes de modificarla. Tus datos no se modificaron.

    Causa:
    { $error }
startup-storage-upgrade-remedy = Copia los detalles y envíalos junto con el archivo de registro al soporte en t.me/screenerbotio_support. No edites, muevas ni elimines la base de datos: { -brand } la abrirá de nuevo cuando se instale una corrección.

startup-wallet-replacement-incomplete-title = No se pudo completar el reemplazo de la billetera
startup-wallet-replacement-incomplete-detail =
    { -brand } se detuvo mientras cambiaba a la nueva billetera de trading. No se eliminó nada: la clave de la billetera anterior sigue en la bóveda de billeteras y su historial local sigue en la carpeta de copias de seguridad.

    Causa:
    { $error }
startup-wallet-replacement-incomplete-remedy = Inicia { -brand } de nuevo para reanudar el reemplazo. Si este mensaje vuelve a aparecer, copia los detalles y envíalos junto con el archivo de registro al soporte en t.me/screenerbotio_support. No edites, muevas ni elimines wallet_replacement.json ni la carpeta de copias de seguridad.
startup-wallet-reset-refused-title = Los datos de la billetera no se restablecieron
startup-wallet-reset-refused-open-positions = La billetera anterior aún tiene posiciones abiertas ({ $count }), así que se conservó su historial local.
startup-wallet-reset-refused-trade-in-flight = Había una operación en curso, así que se conservó el historial local de la billetera.
startup-wallet-reset-refused-tool-in-flight = Había una herramienta de billetera en ejecución, así que se conservó el historial local de la billetera.
startup-wallet-reset-refused-positions-loading = Las posiciones abiertas aún se estaban cargando, así que se conservó el historial local de la billetera.
startup-wallet-reset-refused-positions-unreadable = No se pudo leer el almacén de posiciones, así que no se pueden descartar posiciones abiertas y se conservó el historial local de la billetera.
startup-wallet-reset-refused-remedy = No se movió nada. Inicia { -brand } con la billetera anterior, cierra sus posiciones abiertas y luego restablece de nuevo los datos de la billetera.

startup-generic-title = No se pudo iniciar { -brand }
startup-generic-remedy = Revisa el archivo de registro para ver los detalles y reinicia la app. Si el problema persiste, contacta con soporte en t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Error al crear los directorios necesarios: { $error }
startup-failure-config-load = Error al cargar la configuración: { $error }
startup-failure-actions-sync = Error al sincronizar las acciones desde la base de datos: { $error }
startup-failure-analysis-init = Error al inicializar el motor de análisis: { $error }
startup-failure-assistant-init = Error al inicializar el motor de chat del Asistente: { $error }
startup-failure-wallets-init = Error al inicializar las billeteras: { $error }
startup-failure-wallet-validation = Error al validar la coherencia de la billetera: { $error }
