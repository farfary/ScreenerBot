## Shared

settings-duration-minutes =
    { $count ->
        [one] { $count } minuto
        [many] { $count } minutos
       *[other] { $count } minutos
    }
settings-duration-hours =
    { $count ->
        [one] { $count } hora
        [many] { $count } horas
       *[other] { $count } horas
    }

## settings_dialog.js

settings-dialog-title = Configuración
settings-dialog-close =
    .title = Cerrar (ESC)
    .aria-label = Cerrar configuración
settings-dialog-save = Guardar cambios
settings-dialog-saving = Guardando...
settings-dialog-saved = Guardado
settings-dialog-save-success = Configuración guardada correctamente
settings-dialog-save-failed = No se pudo guardar la configuración
settings-dialog-update-attention = La actualización requiere atención
settings-dialog-tab-interface = Interfaz
settings-dialog-tab-navigation = Navegación
settings-dialog-tab-startup = Inicio
settings-dialog-tab-hints = Ayudas
settings-dialog-tab-data = Datos
settings-dialog-tab-security = Seguridad
settings-dialog-tab-account = Cuenta
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Conexiones de agentes
settings-dialog-tab-updates = Actualizaciones
settings-dialog-tab-licenses = Licencias
settings-dialog-tab-about = Acerca de
settings-dialog-link-privacy = Política de privacidad
settings-dialog-link-terms = Términos del servicio

## settings_dialog.js: Startup tab

settings-startup-section-title = Comportamiento al iniciar
settings-startup-auto-start-label = Iniciar el Trader automáticamente
settings-startup-auto-start-hint = Inicia el Trader automáticamente al abrir la aplicación
settings-startup-coming-soon = Próximamente
settings-startup-default-page-label = Página predeterminada
settings-startup-default-page-hint = Página que se muestra al abrir la aplicación
settings-startup-page-dashboard = Panel
settings-startup-page-tokens = Tokens
settings-startup-page-positions = Posiciones
settings-startup-page-wallet = Billetera
settings-startup-page-config = Configuración
settings-startup-notifications-label = Mostrar notificaciones en segundo plano
settings-startup-notifications-hint = Muestra notificaciones de eventos en segundo plano

## settings_dialog.js: About tab

settings-about-tagline = Motor de trading nativo para Solana
settings-about-link-github = { -github }
settings-about-link-docs = Documentación
settings-about-link-telegram = { -telegram }
settings-about-link-website = Sitio web
settings-about-credits = Creado para traders de Solana
settings-about-copyright = © { $year } { -brand }. Todos los derechos reservados.

## interface_tab.js

settings-interface-section-appearance = Apariencia
settings-interface-theme-label = Tema
settings-interface-theme-hint = Elige tu esquema de color preferido
settings-interface-theme-dark = Oscuro
settings-interface-theme-light = Claro
settings-interface-language-label = Idioma
settings-interface-language-hint = Idioma de visualización del panel
settings-interface-logo-shape-label = Forma del logo del token
settings-interface-logo-shape-hint = Círculo recorta todos los logos; Natural conserva la silueta propia de cada imagen
settings-interface-logo-shape-circle = Círculo
settings-interface-logo-shape-natural = Natural
settings-interface-animations-label = Activar animaciones
settings-interface-animations-hint = Transiciones y efectos suaves
settings-interface-compact-label = Modo compacto
settings-interface-compact-hint = Reduce los márgenes para mostrar más contenido
settings-interface-section-data = Datos y visualización
settings-interface-refresh-label = Intervalo de actualización
settings-interface-refresh-hint = Con qué frecuencia se actualizan los datos
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } segundo
        [many] { $count } segundos
       *[other] { $count } segundos
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } minuto
        [many] { $count } minutos
       *[other] { $count } minutos
    }
settings-interface-ticker-label = Mostrar barra de cotizaciones
settings-interface-ticker-hint = Cinta de métricas en vivo en la cabecera
settings-interface-page-size-label = Tamaño de página de las tablas
settings-interface-page-size-hint = Filas por página de tabla predeterminadas
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } fila
        [many] { $count } filas
       *[other] { $count } filas
    }
settings-interface-auto-expand-label = Expandir categorías automáticamente
settings-interface-auto-expand-hint = Expande las categorías de configuración de forma predeterminada
settings-interface-hints-label = Mostrar ayudas contextuales
settings-interface-hints-hint = Muestra iconos de ayuda que explican las funciones del panel
settings-interface-featured-label = Mostrar fila de destacados
settings-interface-featured-hint = Muestra la fila de tokens destacados en Inicio y Tokens
settings-interface-section-sound = Efectos de sonido
settings-interface-sounds-label = Activar sonidos
settings-interface-sounds-hint = Señales táctiles para la navegación, los cambios de estado y los resultados

## security_tab.js

settings-security-loading = Cargando configuración de seguridad...
settings-security-load-failed = No se pudo cargar la configuración de seguridad

settings-security-type-pin4 = PIN de 4 dígitos
settings-security-type-pin6 = PIN de 6 dígitos
settings-security-type-text = Contraseña de texto
settings-security-type-unset = Sin definir

settings-security-lockscreen-title = Pantalla de bloqueo del panel
settings-security-lockscreen-description = Protege tu panel con un PIN o una contraseña. La pantalla de bloqueo aparecerá al activarse y exigirá autenticarte para continuar.
settings-security-enable-label = Activar pantalla de bloqueo
settings-security-enable-hint = Protege tu panel con autenticación por contraseña
settings-security-password-status-label = Estado de la contraseña
settings-security-password-current = Actual: { $type }
settings-security-password-none = Sin contraseña definida
settings-security-change = Cambiar
settings-security-remove = Quitar
settings-security-set-password = Definir contraseña
settings-security-auto-lock-label = Bloqueo automático por inactividad
settings-security-auto-lock-hint = Bloquea automáticamente tras un periodo sin actividad
settings-security-auto-lock-never = Nunca
settings-security-lock-blur-label = Bloquear al perder el foco de la ventana
settings-security-lock-blur-hint = Bloquea automáticamente al cambiar a otra aplicación
settings-security-quick-actions-title = Acciones rápidas
settings-security-lock-now-label = Bloquear el panel ahora
settings-security-lock-now-hint = Bloquea el panel de inmediato
settings-security-lock-now = Bloquear ahora
settings-security-lock-not-ready = No se puede bloquear: la pantalla de bloqueo no está lista
settings-security-setting-save-failed = No se pudo guardar el ajuste de seguridad

## security_tab.js: two-factor authentication

settings-security-2fa-title = Autenticación en dos pasos
settings-security-2fa-description = Añade una capa extra de seguridad con una aplicación de autenticación (Google Authenticator, Authy, etc.)
settings-security-2fa-status-label = Estado del 2FA
settings-security-2fa-status-enabled = La autenticación en dos pasos está activada
settings-security-2fa-status-none = Sin configurar
settings-security-2fa-disable = Desactivar 2FA
settings-security-2fa-enable = Activar 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = Cerrar
settings-security-password-set-title = Definir contraseña
settings-security-password-change-title = Cambiar contraseña
settings-security-password-current-label = Contraseña actual
settings-security-password-current-input =
    .placeholder = Introduce la contraseña actual
settings-security-password-type-label = Tipo de contraseña
settings-security-password-new-label = Contraseña nueva
settings-security-password-new-input =
    .placeholder = Introduce la contraseña nueva
settings-security-password-confirm-label = Confirmar contraseña
settings-security-password-confirm-input =
    .placeholder = Confirma la contraseña
settings-security-password-update = Actualizar contraseña
settings-security-placeholder-pin4 = Introduce el PIN de 4 dígitos
settings-security-placeholder-pin6 = Introduce el PIN de 6 dígitos
settings-security-placeholder-text = Introduce la contraseña
settings-security-password-required = Introduce una contraseña
settings-security-password-mismatch = Las contraseñas no coinciden
settings-security-pin4-invalid = El PIN debe tener exactamente 4 dígitos
settings-security-pin6-invalid = El PIN debe tener exactamente 6 dígitos
settings-security-text-too-short = La contraseña debe tener al menos 4 caracteres
settings-security-password-saved = Contraseña guardada
settings-security-password-save-failed = No se pudo guardar la contraseña
settings-security-password-save-failed-detail = No se pudo guardar la contraseña: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = Quitar contraseña
settings-security-remove-description = Introduce tu contraseña actual para quitar la protección de la pantalla de bloqueo.
settings-security-remove-confirm = Quitar contraseña
settings-security-current-required = Introduce tu contraseña actual
settings-security-password-removed = Contraseña eliminada
settings-security-password-remove-failed = No se pudo quitar la contraseña
settings-security-password-remove-failed-detail = No se pudo quitar la contraseña: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = Activar la autenticación en dos pasos
settings-security-2fa-password-prompt = Introduce tu contraseña para continuar:
settings-security-2fa-password-input =
    .placeholder = Introduce la contraseña
settings-security-2fa-continue = Continuar
settings-security-2fa-manual-code = Código de introducción manual:
settings-security-2fa-qr =
    .alt = Código QR de TOTP
settings-security-2fa-code-prompt = Introduce el código de 6 dígitos de tu aplicación de autenticación:
settings-security-2fa-verify-enable = Verificar y activar
settings-security-2fa-password-required = Introduce tu contraseña
settings-security-2fa-setup-failed = No se pudo configurar el 2FA
settings-security-2fa-code-invalid-length = Introduce un código de 6 dígitos
settings-security-2fa-code-invalid = Código no válido
settings-security-2fa-enabled = Autenticación en dos pasos activada
settings-security-2fa-verify-failed = No se pudo verificar el código
settings-security-2fa-disable-title = Desactivar la autenticación en dos pasos
settings-security-2fa-disable-prompt = Introduce tu contraseña para desactivar el 2FA:
settings-security-2fa-disable-failed = No se pudo desactivar el 2FA
settings-security-2fa-disabled = Autenticación en dos pasos desactivada

## agent_connections_tab.js

settings-agent-category-analysis = Análisis
settings-agent-category-portfolio = Cartera
settings-agent-category-trading = Trading
settings-agent-category-config = Configuración
settings-agent-category-system = Sistema
settings-agent-category-analysis-description = Análisis de tokens, datos de mercado y comprobaciones de seguridad.
settings-agent-category-portfolio-description = Posiciones abiertas, saldos y P&L.
settings-agent-category-trading-description = Compra, venta y cierre de posiciones con fondos reales.
settings-agent-category-config-description = Todos los ajustes del bot, incluidos los endpoints RPC. Nunca las claves de la billetera.
settings-agent-category-system-description = Estado, eventos y la parada de emergencia.
settings-agent-category-analysis-inline = análisis
settings-agent-category-portfolio-inline = cartera
settings-agent-category-trading-inline = trading
settings-agent-category-config-inline = configuración
settings-agent-category-system-inline = sistema

settings-agent-level-allow = Permitir
settings-agent-level-ask-user = Preguntar
settings-agent-level-deny = Desactivado
settings-agent-level-allow-hint = Se ejecuta de inmediato.
settings-agent-level-ask-user-hint = Espera tu aprobación en la aplicación.
settings-agent-level-deny-hint = Se rechaza y se oculta al agente.

settings-agent-preset-full = Acceso total
settings-agent-preset-ask = Preguntar antes
settings-agent-preset-read = Solo lectura
settings-agent-preset-full-description = Todo se ejecuta sin preguntar. Las claves de la billetera siguen inaccesibles.
settings-agent-preset-ask-description = Cada acción espera tu aprobación en la aplicación.
settings-agent-preset-read-description = Lecturas de análisis y de cartera. No se puede cambiar nada.
settings-agent-preset-custom = Personalizado
settings-agent-preset-group =
    .aria-label = Preajuste de permisos
settings-agent-permission-group = Permiso de { $category }

settings-agent-summary-asks-only = Limitada — pregunta por { $asking }
settings-agent-summary-off-only = Limitada — sin { $off }
settings-agent-summary-asks-and-off = Limitada — pregunta por { $asking }; sin { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP stdio genérico

settings-agent-note-placeholder = Sustituye /absolute/path/to/screenerbot por la ruta absoluta del binario de { -brand }: la aplicación en ejecución no pudo representar la ruta de su ejecutable en este sistema.
settings-agent-note-data-dir = Si ejecutas { -brand } con un directorio de datos no predeterminado, define también SCREENERBOT_DATA_DIR en el cliente (otro indicador -e / --env, o una entrada env) con la misma ruta.
settings-agent-note-codex-run = Ejecuta el comando o añade el bloque TOML a ~/.codex/config.toml ($CODEX_HOME/config.toml). Después, reinicia { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` oculta el secreto en su salida.
settings-agent-note-claude-code = { -claude } Code: ejecuta el comando y reinicia { -claude } Code. `claude mcp get screenerbot` mostrará el entorno configurado, incluido el secreto.
settings-agent-note-claude-desktop = { -claude } Desktop: combina el JSON en claude_desktop_config.json bajo `mcpServers` y reinicia la aplicación.
settings-agent-note-openclaw = Ejecuta el comando y luego usa `openclaw mcp doctor screenerbot --probe` para verificar que el servidor stdio guardado arranca y expone herramientas.
settings-agent-note-hermes = Añade esto bajo `mcp_servers` en el archivo de configuración de { -hermes } y reinicia { -hermes }.
settings-agent-note-generic = Cualquier cliente MCP que use stdio: ejecuta este comando con estos argumentos y este entorno, donde el cliente guarde su lista de servidores.
settings-agent-block-codex-command = { -codex } CLI — comando de terminal
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (alternativa)
settings-agent-block-claude-command = { -claude } Code — comando de terminal
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — comando de terminal
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Cliente MCP stdio genérico

settings-agent-name-required = Introduce un nombre para esta conexión.
settings-agent-name-too-long = El nombre debe tener { $max } caracteres o menos.
settings-agent-name-control-characters = El nombre no debe contener caracteres de control.

settings-agent-title = Conexiones de agentes
settings-agent-description = Conecta { -claude }, { -codex }, { -hermes }, { -openclaw } o cualquier cliente MCP stdio. { -brand } debe seguir en ejecución. Cada conexión tiene sus propios permisos: acceso total de forma predeterminada, limitado por conexión cuando quieras. Ninguna conexión puede leer ni cambiar la clave de tu billetera.
settings-agent-name-label = Nombre de la conexión
settings-agent-name-hint = Se muestra en la lista de abajo para distinguir las conexiones.
settings-agent-name-input =
    .placeholder = Agente de código del portátil
settings-agent-client-label = Cliente
settings-agent-client-hint = Elige la configuración que se muestra tras crear la conexión.
settings-agent-permissions-label = Permisos
settings-agent-permissions-hint = Una conexión nueva puede hacerlo todo. Limita cualquier categoría ahora o más tarde desde la lista de abajo; las claves de la billetera nunca son accesibles en ningún caso.
settings-agent-create = Crear conexión
settings-agent-issued-group =
    .aria-label = Credencial de la conexión nueva
settings-agent-issued-warning = Copia el secreto ahora. Se muestra una sola vez y no se puede recuperar: revoca y vuelve a crear la conexión si lo pierdes. { -brand } solo guarda un verificador unidireccional; tu cliente MCP guarda el texto plano en su propia configuración.
settings-agent-issued-client-id = ID de cliente
settings-agent-issued-secret = Secreto de un solo uso
settings-agent-setup-for = Configuración para
settings-agent-done = Listo
settings-agent-list-title = Conexiones
settings-agent-loading = Cargando conexiones...
settings-agent-active-count = { $count } activas
settings-agent-empty = Aún no hay conexiones. Crea una arriba para emparejar un cliente.
settings-agent-empty-active = No hay conexiones activas.
settings-agent-revoked-title = Conexiones revocadas
settings-agent-created = Creada { $time }
settings-agent-last-used = Último uso { $time }
settings-agent-never-used = Nunca usada
settings-agent-permissions-edit = Permisos
settings-agent-revoke = Revocar
settings-agent-permissions-save = Guardar permisos

settings-agent-load-failed = No se pudieron cargar las conexiones de agentes
settings-agent-list-failed = No se pudieron cargar las conexiones
settings-agent-create-failed = No se pudo crear la conexión.
settings-agent-unreachable-create = No se pudo contactar con { -brand } para crear la conexión.
settings-agent-permissions-update-failed = No se pudieron actualizar los permisos
settings-agent-permissions-updated = Permisos actualizados
settings-agent-permissions-updated-detail = Se aplica a la próxima solicitud de la conexión.
settings-agent-unreachable-save = No se pudo contactar con { -brand } para guardar
settings-agent-revoke-title = Revocar conexión
settings-agent-revoke-message = ¿Revocar “{ $label }”? El cliente dejará de funcionar en su próxima solicitud y no se podrá restaurar.
settings-agent-revoke-fallback-name = esta conexión
settings-agent-revoke-failed = No se pudo revocar la conexión
settings-agent-unreachable-revoke = No se pudo contactar con { -brand } para revocar

## telegram_tab.js

settings-telegram-loading = Cargando configuración de { -telegram }...
settings-telegram-load-failed = No se pudo cargar la configuración de { -telegram }
settings-telegram-unknown = Desconocido
settings-telegram-session-active = Activa: { $duration }
settings-telegram-sessions-empty = No hay sesiones activas
settings-telegram-session-revoke = Revocar

settings-telegram-connection-title = Conexión
settings-telegram-connection-description = Conecta tu bot de { -telegram } para recibir notificaciones y controlar { -brand } de forma remota.
settings-telegram-enable-label = Activar { -telegram }
settings-telegram-enable-hint = Activa la integración con el bot de { -telegram }
settings-telegram-token-label = Token del bot
settings-telegram-token-saved = Token guardado
settings-telegram-token-help = Obtenlo de @BotFather en { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token guardado (introduce uno nuevo para cambiarlo)
settings-telegram-token-input =
    .placeholder = Introduce el token del bot
settings-telegram-token-toggle =
    .title = Mostrar/Ocultar
settings-telegram-chat-label = ID del chat
settings-telegram-chat-connected = Conectado al chat:
settings-telegram-chat-discover-hint = Descubre tu ID de chat automáticamente
settings-telegram-chat-change =
    .title = Cambiar
settings-telegram-chat-discover = Descubrir ID del chat
settings-telegram-discovery-step-add = Añade tu bot a un grupo de { -telegram } o inicia un chat directo con él
settings-telegram-discovery-step-privacy = Para grupos: revisa @BotFather → /mybots → [tu bot] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Modo de privacidad desactivado:</strong> el bot recibe todos los mensajes del grupo<br/><strong>Modo de privacidad activado:</strong> el bot solo recibe mensajes cuando se le @menciona
settings-telegram-discovery-step-send = Envía cualquier mensaje (o @menciona a tu bot si el modo de privacidad está activado)
settings-telegram-discovery-listening = Esperando mensajes...
settings-telegram-discovery-select = Seleccionar
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Idioma de los mensajes
settings-telegram-language-hint = Idioma de los mensajes y botones del bot de { -telegram }
settings-telegram-language-follow-app = Seguir el idioma de la aplicación
settings-telegram-test-label = Probar conexión
settings-telegram-test-hint = Envía un mensaje de prueba para verificar la configuración
settings-telegram-test-send = Enviar prueba
settings-telegram-test-sending = Enviando...

settings-telegram-chat-type-private = privado
settings-telegram-chat-type-group = grupo
settings-telegram-chat-type-supergroup = supergrupo
settings-telegram-chat-type-channel = canal

settings-telegram-auth-title = Autenticación de comandos
settings-telegram-auth-description = Los comandos de { -telegram } usan el mismo 2FA que la pantalla de bloqueo del panel.
settings-telegram-auth-protected = Protegido
settings-telegram-auth-disabled = Desactivado
settings-telegram-auth-not-configured = Sin configurar
settings-telegram-auth-error = Error
settings-telegram-auth-protected-note = Los comandos están protegidos por el 2FA de la pantalla de bloqueo. Cuando las sesiones caducan, los usuarios deben enviar el código de su aplicación de autenticación con el comando <code>/login</code>.
settings-telegram-auth-disabled-note = El 2FA de la pantalla de bloqueo está configurado pero desactivado para { -telegram }. Activa “Exigir 2FA para comandos” arriba para proteger los comandos de { -telegram }.
settings-telegram-auth-missing-note = El 2FA de la pantalla de bloqueo no está configurado. Sin 2FA, las sesiones caducadas se reactivarán automáticamente sin verificación.
settings-telegram-auth-managed-in = El 2FA se gestiona en
settings-telegram-auth-configure-in = Configura el 2FA en
settings-telegram-auth-configure-suffix = para exigir verificación en los comandos de { -telegram }.
settings-telegram-security-link = Configuración de seguridad
settings-telegram-timeout-title = Caducidad de la sesión
settings-telegram-timeout-description = Cuánto tiempo permanece activa una sesión autenticada
settings-telegram-sessions-title = Sesiones activas

settings-telegram-notifications-title = Configuración de notificaciones
settings-telegram-notifications-description = Elige qué eventos generan notificaciones de { -telegram }.
settings-telegram-notify-opened-label = Posición abierta
settings-telegram-notify-opened-hint = Avisa cuando se abre una posición nueva
settings-telegram-notify-closed-label = Posición cerrada
settings-telegram-notify-closed-hint = Avisa cuando se cierra una posición
settings-telegram-notify-partial-label = Salida parcial
settings-telegram-notify-partial-hint = Avisa en las salidas parciales de posiciones
settings-telegram-notify-dca-label = DCA ejecutado
settings-telegram-notify-dca-hint = Avisa cuando se ejecutan órdenes de DCA
settings-telegram-notify-errors-label = Errores
settings-telegram-notify-errors-hint = Avisa de errores y fallos
settings-telegram-notify-startup-label = Inicio/Apagado
settings-telegram-notify-startup-hint = Avisa cuando el bot arranca o se detiene
settings-telegram-notify-filtering-label = Alertas de filtrado
settings-telegram-notify-filtering-hint = Avisa cuando tokens nuevos cumplen los criterios de filtrado
settings-telegram-notify-trades-label = Alertas de operaciones
settings-telegram-notify-trades-hint = Avisa de operaciones relevantes de los tokens en seguimiento
settings-telegram-notify-daily-label = Resumen diario
settings-telegram-notify-daily-hint = Recibe un resumen diario de la actividad de trading y del P&L

settings-telegram-features-title = Funciones
settings-telegram-features-description = Configura las capacidades del bot de { -telegram }.
settings-telegram-commands-label = Activar comandos
settings-telegram-commands-hint = Permite controlar el bot con comandos de { -telegram }
settings-telegram-require-2fa-label = Exigir 2FA para comandos
settings-telegram-require-2fa-hint = Cuando las sesiones caducan, exige el código 2FA para reactivarlas. Usa el 2FA de la pantalla de bloqueo.
settings-telegram-inline-label = Botones de acción integrados
settings-telegram-inline-hint = Muestra botones de acción en los mensajes de notificación

settings-telegram-setting-save-failed = No se pudo guardar el ajuste de { -telegram }
settings-telegram-discovery-start-failed = No se pudo iniciar la detección
settings-telegram-chat-selected = Chat seleccionado
settings-telegram-chat-select-failed = No se pudo seleccionar el chat
settings-telegram-test-sent = Mensaje de prueba enviado
settings-telegram-test-failed = Falló el mensaje de prueba
settings-telegram-session-revoked = Sesión revocada
settings-telegram-session-revoke-failed = No se pudo revocar la sesión

## licenses_tab.js

settings-licenses-title = Licencias de código abierto
settings-licenses-subtitle = { -brand } está creado con el siguiente software de código abierto
settings-licenses-footer = Los textos completos de las licencias están disponibles en el repositorio del proyecto y en el código fuente de cada dependencia.
settings-licenses-category-framework = Framework de la aplicación
settings-licenses-category-solana = Blockchain de Solana
settings-licenses-category-data = Datos y almacenamiento
settings-licenses-category-networking = Red
settings-licenses-category-cryptography = Criptografía y codificación
settings-licenses-category-assets = Recursos de interfaz
settings-licenses-desc-electron = Framework de aplicaciones de escritorio
settings-licenses-desc-tokio = Runtime asíncrono para Rust
settings-licenses-desc-axum = Framework de servidor web
settings-licenses-desc-tower = Abstracciones de servicios
settings-licenses-desc-hyper = Implementación de HTTP
settings-licenses-desc-solana-sdk = Núcleo del SDK de Solana
settings-licenses-desc-solana-client = Cliente RPC
settings-licenses-desc-solana-program = Biblioteca de programas
settings-licenses-desc-spl-token = Programa SPL Token
settings-licenses-desc-spl-token-2022 = Extensiones de Token-2022
settings-licenses-desc-spl-associated-token-account = Cuentas de token asociadas
settings-licenses-desc-sqlite = Motor de base de datos embebido
settings-licenses-desc-rusqlite = Bindings de SQLite para Rust
settings-licenses-desc-r2d2 = Pool de conexiones a la base de datos
settings-licenses-desc-serde = Framework de serialización
settings-licenses-desc-toml = Análisis de configuración
settings-licenses-desc-reqwest = Cliente HTTP
settings-licenses-desc-tokio-tungstenite = Cliente WebSocket
settings-licenses-desc-rustls = Implementación de TLS
settings-licenses-desc-blake3 = Función hash
settings-licenses-desc-sha-2 = Hash SHA-256/512
settings-licenses-desc-bs58 = Codificación Base58
settings-licenses-desc-base64 = Codificación Base64
settings-licenses-desc-lucide-icons = Biblioteca de fuente de iconos
settings-licenses-desc-inter = Fuente de la interfaz
settings-licenses-desc-jetbrains-mono = Fuente monoespaciada
settings-licenses-desc-orbitron = Fuente de títulos
settings-licenses-desc-vazirmatn = Fuente para árabe y persa
settings-licenses-desc-noto-sans-devanagari = Fuente para devanagari
settings-licenses-desc-noto-sans-sc = Fuente para chino simplificado
settings-licenses-desc-pretendard = Fuente para coreano
settings-licenses-desc-pretendard-jp = Fuente para japonés

## hints_tab.js

settings-hints-title = Ayudas contextuales
settings-hints-description = Las ayudas contextuales son los iconos de ayuda que explican las funciones del panel. Revisa todas las ayudas de abajo y restaura las que hayas ocultado con “No volver a mostrar”, una a una o todas a la vez.
settings-hints-hidden-label = Ayudas ocultas
settings-hints-hidden-summary = { $hidden } de { $total } ayudas están ocultas actualmente.
settings-hints-restore-all = Restaurar todas las ayudas
settings-hints-toggle-shown =
    .title = Mostrar esta ayuda
settings-hints-toggle-shown-title = Visible
settings-hints-toggle-hidden-title = Oculta: actívala para mostrarla
settings-hints-restore-title = Restaurar todas las ayudas
settings-hints-restore-message = ¿Volver a mostrar todas las ayudas contextuales, incluidas las que hayas ocultado?
settings-hints-restore-confirm = Restaurar todas
settings-hints-restored = Todas las ayudas restauradas

## account_tab.js

settings-account-title = Cuenta de { -brand }
settings-account-description = Gratuita y opcional. { -brand } opera, descubre mercados y muestra gráficos sin cuenta; simplemente lo hace con los proveedores públicos. El panel de abajo enumera lo que añade iniciar sesión.
settings-account-data-title = Datos de { -brand }
settings-account-data-description = Operamos un servicio compartido de datos de mercado en screenerbot.io: velas agregadas en siete marcos temporales, un registro de pools resuelto, informes de seguridad en caché e identidad de tokens normalizada. Existe para que cada instalación no quede limitada por separado por los proveedores públicos, y usarlo requiere una cuenta para que ese costo compartido tenga un responsable.
settings-account-data-fallback = Cuando no está disponible, { -brand } recurre automáticamente a los proveedores públicos. Nada se detiene; los gráficos se rellenan más despacio y tienen menos historial.
settings-account-gateway-title = Envío de transacciones
settings-account-gateway-description = Al iniciar sesión, { -brand } puede difundir tus swaps a través de screenerbot.io en lugar de tu propio RPC. Tu bot sigue construyendo y firmando cada transacción en este equipo; el servidor solo la retransmite y no puede modificar una transacción firmada sin invalidar su firma.
settings-account-gateway-label = Usar el RPC de { -brand } para enviar transacciones
settings-account-gateway-hint = Solo para el envío. Los datos de precios siempre provienen de tu propio RPC: el sondeo de pools es demasiado pesado para un endpoint compartido, por lo que nunca se envía allí.
settings-account-manage-title = Gestión de tu cuenta
settings-account-manage-description = Tu contraseña, correo electrónico, dispositivos conectados y pagos de referidos se gestionan en el sitio web. Revocar un dispositivo allí cierra su sesión en todas partes, incluida esta.
settings-account-open-dashboard = Abrir tu panel

## navigation_tab.js

settings-navigation-title = Pestañas de navegación
settings-navigation-hint = Arrastra los elementos para reordenarlos. Cambia la visibilidad con el interruptor.
settings-navigation-note = Los cambios se aplican tras guardar. Actualiza la página para ver los cambios en la barra de navegación.
settings-navigation-drag-handle =
    .title = Arrastra para reordenar
settings-navigation-defaults-failed = No se pudo cargar la navegación predeterminada
settings-navigation-reset = Navegación restablecida a los valores predeterminados

## data_tab.js

settings-data-storage-title = Almacenamiento de bases de datos
settings-data-storage-description = Resumen de todas las bases de datos que guardan tus datos de trading, posiciones e información histórica.
settings-data-stats-loading = Cargando estadísticas de las bases de datos...
settings-data-stats-load-failed = No se pudieron cargar las estadísticas de las bases de datos
settings-data-total-storage = Almacenamiento total de bases de datos
settings-data-db-tokens = Tokens
settings-data-db-transactions = Transacciones
settings-data-db-positions = Posiciones
settings-data-db-events = Eventos
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Billetera
settings-data-db-pools = Pools
settings-data-db-strategies = Estrategias
settings-data-db-actions = Acciones
settings-data-directory-label = Directorio de datos
settings-data-directory-copied = Directorio de datos
settings-data-config-path-copied = Ruta de configuración
settings-data-path-unavailable = No disponible
settings-data-path-copy-title = Haz clic para copiar la ruta
settings-data-path-copy-failed = No se pudo copiar la ruta

settings-data-config-title = Gestión de la configuración
settings-data-config-description = Exporta, importa y gestiona la configuración de tu bot. Haz copias de seguridad antes de hacer cambios importantes.
settings-data-config-export = Exportar configuración
settings-data-config-import = Importar configuración
settings-data-config-reset = Restablecer valores predeterminados
settings-data-config-location-label = Ubicación de la configuración
settings-data-config-fetch-failed = No se pudo obtener la configuración
settings-data-config-exported = Configuración exportada
settings-data-config-export-failed = No se pudo exportar la configuración: { $message }
settings-data-config-import-title = Importar configuración
settings-data-config-import-message = ¿Importar esta configuración? Se sobrescribirán los ajustes actuales. Las credenciales de la billetera se conservarán.
settings-data-config-imported = Configuración importada correctamente. Algunos cambios pueden requerir reiniciar.
settings-data-config-import-failed = No se pudo importar la configuración: { $message }
settings-data-config-reset-title = Restablecer configuración
settings-data-config-reset-message = ¿Restablecer todos los ajustes a sus valores predeterminados? Las credenciales de tu billetera se conservarán, pero el resto de los ajustes se restablecerán.
settings-data-config-reset-done = Configuración restablecida a los valores predeterminados
settings-data-config-reset-failed = No se pudo restablecer la configuración: { $message }
settings-data-unknown-error = Error desconocido

settings-data-cleanup-title = Limpieza de datos
settings-data-cleanup-description = Libera espacio en disco eliminando datos antiguos o sin uso. Estas acciones no se pueden deshacer.
settings-data-ohlcv-cleanup-label = Limpieza de datos OHLCV
settings-data-ohlcv-cleanup-hint = Elimina los datos de velas de los tokens que no han estado activos durante el tiempo indicado.
settings-data-cleanup-hours-unit = horas
settings-data-cleanup-ohlcv = Limpiar OHLCV
settings-data-cleanup-running = Limpiando...
settings-data-cleanup-hours-invalid = Valor de horas no válido
settings-data-cleanup-confirm-title = Eliminar datos OHLCV
settings-data-cleanup-confirm-message =
    ¿Eliminar los datos OHLCV de los tokens inactivos durante más de { $hours ->
        [one] { $hours } hora
        [many] { $hours } horas
       *[other] { $hours } horas
    }?
settings-data-cleanup-done =
    { $count ->
        [one] Se limpió { $count } token inactivo
        [many] Se limpiaron { $count } tokens inactivos
       *[other] Se limpiaron { $count } tokens inactivos
    }
settings-data-cleanup-failed = Falló la limpieza
settings-data-cleanup-failed-detail = Falló la limpieza: { $message }

settings-data-cache-clear-label = Vaciar toda la caché OHLCV
settings-data-cache-clear-hint = Borra todos los datos de velas en caché y vuelve a obtener desde cero cada token monitorizado. Úsalo si los gráficos se ven mal o tras una actualización de la lógica de datos.
settings-data-cache-clear = Vaciar caché OHLCV
settings-data-cache-clearing = Vaciando...
settings-data-cache-confirm-title = Vaciar toda la caché OHLCV
settings-data-cache-confirm-message = ¿Borrar todos los datos de velas en caché de todos los tokens? Los tokens monitorizados volverán a obtener su historial desde cero. Esta acción no se puede deshacer.
settings-data-candles-count =
    { $count ->
        [one] { $count } vela
        [many] { $count } velas
       *[other] { $count } velas
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
settings-data-cache-cleared = Se vaciaron { $candles } de { $tokens }; obteniendo de nuevo
settings-data-cache-clear-failed = No se pudo vaciar la caché OHLCV
settings-data-cache-clear-failed-detail = No se pudo vaciar la caché OHLCV: { $message }

settings-data-ui-cache-label = Caché del estado de la interfaz
settings-data-ui-cache-hint = Borra las preferencias de tablas, los estados de filtros y los ajustes de vista guardados.
settings-data-ui-cache-clear = Vaciar caché de la interfaz
settings-data-ui-cache-confirm-title = Borrar el estado de la interfaz
settings-data-ui-cache-confirm-message = ¿Borrar todas las preferencias de la interfaz guardadas? Se restablecerán las columnas de las tablas, los filtros y los ajustes de vista.
settings-data-ui-cache-cleared =
    { $count ->
        [one] Se borró { $count } ajuste de la interfaz en caché
        [many] Se borraron { $count } ajustes de la interfaz en caché
       *[other] Se borraron { $count } ajustes de la interfaz en caché
    }

settings-data-folder-label = Abrir carpeta de datos
settings-data-folder-hint = Abre en tu gestor de archivos la carpeta que contiene todos los datos de { -brand }.
settings-data-folder-open = Abrir carpeta
settings-data-folder-open-failed = No se pudo abrir la carpeta de datos
