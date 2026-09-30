telegram-reply-status = Estado
telegram-reply-balance = Saldo
telegram-reply-positions = Posiciones
telegram-reply-pause = Pausar
telegram-reply-resume = Reanudar
telegram-reply-stop = Detener
telegram-reply-stats = Estadísticas
telegram-reply-menu = Menú
telegram-reply-help = Ayuda

telegram-button-positions = Posiciones
telegram-button-balance = Saldo
telegram-button-stats = Estadísticas
telegram-button-tokens = Tokens
telegram-button-pause = Pausar
telegram-button-stop = Detener
telegram-button-settings = Configuración
telegram-button-refresh = Actualizar
telegram-button-menu = Menú
telegram-button-back = Atrás
telegram-button-back-to-menu = Volver al menú
telegram-button-back-to-tokens = Volver a tokens
telegram-button-cancel = Cancelar
telegram-button-close-all-positions = Cerrar todas las posiciones
telegram-button-sell-percent = Vender { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Lista negra
telegram-button-blacklist-symbol = Añadir { $symbol } a la lista negra
telegram-button-close-position = Cerrar posición
telegram-button-confirm-close = Confirmar cierre
telegram-button-confirm-close-all = Cerrar TODAS las posiciones
telegram-button-confirm-sell = Confirmar venta de { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = CONFIRMAR DETENCIÓN FORZADA
telegram-button-confirm-buy = Comprar { $amount } { -sol }
telegram-button-notifications = Notificaciones
telegram-button-trading = Trading
telegram-button-entry-monitor = Monitor de entradas
telegram-button-exit-monitor = Monitor de salidas
telegram-button-auto-trading = Trading automático
telegram-button-force-stop = Detención forzada
telegram-button-notify-opened = Abiertas
telegram-button-notify-closed = Cerradas
telegram-button-notify-partial = Parciales
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Errores
telegram-button-details = Detalles
telegram-button-position = Posición
telegram-button-sell-more = Vender más
telegram-button-more-dca = Más DCA
telegram-button-history = Historial
telegram-button-status = Estado
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Volver a autenticar
telegram-button-previous = Ant.
telegram-button-next = Sig.
telegram-button-passed = Aprobados
telegram-button-rejected = Rechazados
telegram-button-new-24h = Nuevos (24 h)
telegram-button-all-tokens = Todos los tokens
telegram-button-search-token = Buscar token
telegram-button-filter-stats = Estadísticas de filtros
telegram-button-refresh-stats = Actualizar estadísticas
telegram-button-view-position = Ver posición
telegram-button-buy-amount = { $amount } { -sol }

telegram-unknown-command =
    Comando desconocido: { $command }

    Usa /help para ver los comandos disponibles.
telegram-session-expired =
    <b>Sesión caducada</b>

    Usa /login para autenticarte de nuevo.
telegram-2fa-required =
    <b>Se requiere 2FA</b>

    Introduce tu código de 6 dígitos del autenticador.
telegram-account-locked =
    <b>Cuenta bloqueada</b>

    Demasiados intentos fallidos.
    Inténtalo de nuevo en { $seconds ->
        [one] { $seconds } segundo.
        [many] { $seconds } segundos.
       *[other] { $seconds } segundos.
    }
telegram-code-invalid = Introduce un código válido de 6 dígitos.
telegram-authenticated =
    <b>¡Autenticado!</b>

    Ya tienes acceso a los comandos del bot.
telegram-wrong-code =
    <b>Código incorrecto</b>

    { $remaining ->
        [one] Queda { $remaining } intento.
        [many] Quedan { $remaining } intentos.
       *[other] Quedan { $remaining } intentos.
    }
telegram-auth-required =
    <b>Se requiere autenticación</b>

    Introduce tu contraseña para continuar.

    <i>Escribe tu contraseña y envíala.</i>
telegram-login-required =
    <b>Se requiere inicio de sesión</b>

    Introduce tu código de 6 dígitos del autenticador:
telegram-session-activated =
    <b>Sesión activada</b>

    2FA no está configurado. Tu sesión ya está activa.

    <i>Consejo: activa 2FA en los ajustes de seguridad para mayor protección.</i>

telegram-discovery-hello = ¡Hola, { $name }!
telegram-discovery-default-name = usuario
telegram-discovery-detected = <b>¡Chat detectado!</b>
telegram-discovery-details =
    ID del chat: <code>{ $chat_id }</code>
    Tipo: { $chat_type }

    Ve al panel de { -brand } y haz clic en este chat para seleccionarlo.
telegram-chat-type-private = privado
telegram-chat-type-group = grupo
telegram-chat-type-supergroup = supergrupo
telegram-chat-type-channel = canal

telegram-menu-title =
    <b>Panel de control</b>

    Selecciona una opción para ver información o controlar el bot.
telegram-menu-positions-empty =
    <b>Sin posiciones abiertas</b>

    Esperando nuevas oportunidades...
telegram-menu-positions-title = <b>Posiciones ({ $count })</b>
telegram-menu-positions-hint = <i>Toca una posición para gestionarla.</i>
telegram-menu-settings =
    <b>Configuración</b>

    Configura las notificaciones y los parámetros de trading.
telegram-settings-notifications =
    <b>Configuración de notificaciones</b>

    Activa o desactiva las notificaciones:
telegram-settings-trading =
    <b>Controles de trading</b>

    Activa o desactiva las funciones de trading:
telegram-pagination-expired = La sesión de paginación ha caducado.

telegram-status-state-stopped = <b>DETENIDO</b> (detención forzada activa)
telegram-status-state-active = <b>ACTIVO</b>
telegram-status-state-paused = <b>EN PAUSA</b>
telegram-status-on = ACTIVADO
telegram-status-off = DESACTIVADO
telegram-status-body =
    <b>Estado del sistema</b>

    <b>Sistema</b>
    Estado — { $state }
    Tiempo activo — { $uptime }
    Versión — v{ $version }

    <b>Trading</b>
    Entradas — { $entries }
    Salidas — { $exits }
    Posiciones — { $positions }
telegram-positions-empty =
    <b>Sin posiciones abiertas</b>

    Esperando oportunidades...
telegram-positions-title = <b>Posiciones abiertas ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } más...</i>
telegram-positions-summary =
    <b>Resumen de la cartera</b>
    Invertido — { $invested } { -sol }
    P{ "&amp;" }L neto — { $pnl } { -sol }
telegram-balance-body =
    <b>Saldo de la billetera</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Estadísticas diarias</b>

    Posiciones — { $positions }
    Invertido — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

telegram-start-ready =
    <b>¡{ -brand } está listo!</b>

    El trading está <b>activado</b>.

    Usa el teclado de abajo para controlar el bot.
    Escribe /help para ver los comandos disponibles.
telegram-stop-already = <b>El trading ya está desactivado</b>
telegram-stop-done =
    <b>Trading desactivado</b>

    Todos los monitores de trading (entradas { "&amp;" } salidas) están detenidos.
    Usa /pause para detener solo las entradas.
telegram-stop-failed =
    <b>No se pudo desactivar el trading</b>

    Error: { $detail }
telegram-pause-done =
    <b>Monitor de entradas en pausa</b>

    No se abrirán nuevas posiciones.
    El monitor de salidas sigue funcionando.
telegram-pause-failed =
    <b>No se pudieron pausar las entradas</b>

    Error: { $detail }
telegram-resume-done =
    <b>Monitor de entradas reanudado</b>

    Buscando señales de entrada de nuevo.
telegram-resume-failed =
    <b>No se pudieron reanudar las entradas</b>

    Error: { $detail }
telegram-force-stop-confirm =
    <b>DETENCIÓN FORZADA</b>

    Esto detendrá de inmediato TODA la actividad de trading:
    • Sin nuevas entradas
    • Sin salidas (incluidos los stop loss)
    • Sin operaciones DCA
telegram-force-stop-warning = <b>¡Esta es una acción de emergencia!</b>
telegram-force-stop-question = ¿Seguro que quieres continuar?
telegram-force-stop-active =
    <b>DETENCIÓN FORZADA ACTIVADA</b>

    Todo el trading se ha detenido.

    Usa /resume_trading para quitar este indicador.
telegram-resume-trading-not-stopped =
    <b>El trading no está en detención forzada</b>

    No se necesita ninguna acción.
telegram-resume-trading-done =
    <b>Trading reanudado</b>

    El indicador de detención forzada se ha quitado.
    Las operaciones de trading normales pueden continuar.

telegram-help-title = <b>Ayuda de { -brand }</b>
telegram-help-heading-dashboard = Panel
telegram-help-heading-market = Mercado
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Seguridad
telegram-help-heading-system = Sistema
telegram-help-commands-dashboard =
    /status — Estado del sistema { "&amp;" } tiempo activo
    /stats — Rendimiento diario
    /balance — Saldo de la billetera
    /positions — Posiciones abiertas
telegram-help-commands-market =
    /tokens — Explorador de tokens
    /rejected — Tokens filtrados
telegram-help-commands-trading =
    /start — Activar el sistema de trading
    /stop — Desactivar el sistema de trading
    /pause — Pausar nuevas entradas
    /resume — Reanudar nuevas entradas
    /menu — Menú interactivo
telegram-help-commands-safety =
    /force_stop — <b>PARADA DE EMERGENCIA</b>
    /resume_trading — Quitar el estado de emergencia
telegram-help-commands-system =
    /update — Estado de actualización { "&amp;" } instalación
    /login — Autenticación 2FA
telegram-help-tip = <i>Consejo: toca un comando para ejecutarlo.</i>

telegram-update-up-to-date-auto =
    <b>Estás al día</b>

    Ejecutando v{ $version }, instalada automáticamente.
telegram-update-up-to-date =
    <b>Estás al día</b>

    Ejecutando v{ $version }.
telegram-update-check-failed =
    <b>Falló la comprobación de actualizaciones</b>

    { $reason }
telegram-update-unreachable = No se pudo conectar con screenerbot.io.
telegram-update-installing = <b>Instalando v{ $version }</b>
telegram-update-restarting =
    { -brand } se está reiniciando con la nueva versión. El trading se reanuda automáticamente.
telegram-update-install-failed =
    <b>No se pudo instalar v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>v{ $version } está descargada</b>

    Esta versión también actualiza la app de escritorio, por lo que su instalador debe ejecutarse en el equipo. Abre Configuración → Actualizaciones allí.
telegram-update-downloading =
    <b>Descargando v{ $version }</b>

    { $percent }% de { $size } MB.
telegram-update-available =
    <b>v{ $version } está disponible</b>

    { $how }
    Tamaño de descarga: { $size } MB.

    Se descarga sola; envía /update de nuevo cuando esté lista.
telegram-update-how-core = Se instala en segundo plano con un breve reinicio.
telegram-update-how-installer = Requiere ejecutar una vez el instalador de escritorio.

telegram-value-unknown = Desconocido
telegram-value-na = N/D
telegram-percent-value = { $percent }%
telegram-price-sol = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }s
telegram-duration-minutes = { $minutes }min
telegram-duration-minutes-seconds = { $minutes }min { $seconds }s
telegram-duration-hours = { $hours }h
telegram-duration-hours-minutes = { $hours }h { $minutes }min
telegram-duration-days = { $days }d
telegram-duration-days-hours = { $days }d { $hours }h
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-sol = { $amount } { -sol }
telegram-error-line = Error: { $detail }
telegram-ai-reasoning =
    <b>Análisis LLM</b>
    <i>{ $reasoning }</i>

telegram-row-entry = Entrada — { $price } { -sol }
telegram-row-exit = Salida — { $price } { -sol }
telegram-row-current = Actual — { $price } { -sol }
telegram-row-invested = Invertido — { $amount } { -sol }
telegram-row-received = Recibido — { $amount } { -sol }
telegram-row-value = Valor — { $amount } { -sol }
telegram-row-total = Total — { $amount } { -sol }
telegram-row-tokens = Tokens — { $tokens }
telegram-row-duration = Duración — { $duration }
telegram-row-reason = Motivo — { $reason }
telegram-row-remaining = Restante — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

telegram-notify-opened-title = <b>Posición abierta</b>
telegram-notify-opened-size = Tamaño — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Precio — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Posición cerrada</b> — Ganancia
telegram-notify-closed-title-loss = <b>Posición cerrada</b> — Pérdida
telegram-notify-closed-reason-unspecified = Cerrada
telegram-notify-partial-title = <b>Salida parcial</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Vendido { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Añadido — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Prom. — { $price } { -sol }
telegram-notify-severity-critical = <b>Error crítico</b>
telegram-notify-severity-error = <b>Error</b>
telegram-notify-severity-warning = <b>Advertencia</b>
telegram-notify-severity-info = <b>Información</b>
telegram-notify-alert-title = <b>Alerta de operación</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Acción: compra de { $amount } { -sol }
telegram-notify-alert-sold = Acción: venta de { $amount } { -sol }
telegram-notify-alert-wallet = Billetera: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (simulado)
telegram-notify-copy-task = Tarea: { $task }
telegram-notify-scheduled-completed = <b>Tarea programada completada</b>
telegram-notify-scheduled-failed = <b>Tarea programada fallida</b>
telegram-notify-scheduled-timed-out = <b>Tarea programada agotó el tiempo</b>
telegram-notify-scheduled-error = Error: { $error }
telegram-notify-summary-title = <b>Resumen diario</b> — { $date }
telegram-notify-summary-performance = <b>Rendimiento</b>
telegram-notify-summary-trades = Operaciones — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Tasa de aciertos — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Posiciones abiertas — { $count }
telegram-notify-started-title = <b>{ -brand } iniciado</b>
telegram-notify-started-version = <b>Versión</b> — { $version }
telegram-notify-started-mode = <b>Modo</b> — { $mode }
telegram-notify-started-ready = ¡Listo para operar!
telegram-notify-stopped-title = <b>{ -brand } detenido</b>
telegram-notify-stopped-reason = <b>Motivo</b> — { $reason }
telegram-notify-stopped-goodbye = ¡Hasta pronto! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Apagado ordenado
telegram-notify-update-available =
    <b>Actualización v{ $version } disponible</b>

    { $how }
    Tamaño de descarga: { $size } MB
telegram-notify-update-how-installer = Esta versión también actualiza la app de escritorio, por lo que su instalador debe ejecutarse una vez.
telegram-notify-update-ready =
    <b>Actualización v{ $version } lista</b>

    { $how }
telegram-notify-update-ready-silent = Envía /update para aplicarla ahora, o se instalará la próxima vez que se inicie { -brand }.
telegram-notify-update-ready-installer = Abre Configuración → Actualizaciones para ejecutar el instalador.
telegram-notify-update-applying =
    <b>Instalando v{ $version }</b>

    El backend se está reiniciando; el trading se reanuda automáticamente.
telegram-notify-new-tokens =
    <b>Alerta de filtrado</b>

    { $count ->
        [one] Se encontró { $count } token nuevo que cumple tus criterios.
        [many] Se encontraron { $count } tokens nuevos que cumplen tus criterios.
       *[other] Se encontraron { $count } tokens nuevos que cumplen tus criterios.
    }
telegram-notify-crash =
    <b>¡El bot falló!</b>

    <b>Ubicación:</b> <code>{ $location }</code>
    <b>Error:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Reinicia el bot.

telegram-filter-results-title = <b>Resultados del filtro</b> ({ $count })
telegram-filter-results-empty = <i>No se encontraron tokens.</i>
telegram-filter-results-page = <i>Página { $page } de { $total }</i>

telegram-position-not-found = Posición no encontrada
telegram-position-no-positions = No hay posiciones que cerrar
telegram-position-history-empty =
    <b>Historial de operaciones</b>

    Aún no hay posiciones cerradas.
telegram-position-history-title = <b>Operaciones recientes</b>
telegram-position-history-more =
    <i>+{ $count } { $count ->
        [one] operación más
        [many] operaciones más
       *[other] operaciones más
    }...</i>
telegram-position-confirm-hint = <i>Confirma en 30 s para ejecutar.</i>
telegram-position-confirm-close-title = <b>¿Cerrar posición?</b>
telegram-position-confirm-close-selling = Vendiendo { $tokens } tokens
telegram-position-confirm-close-estimated = Estimado — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Confirma en 30 segundos</i>
telegram-position-confirm-sell =
    <b>Confirmar venta</b>

    Token — { $symbol }
    Cantidad — { $percent }%
    Tokens — { $tokens }
telegram-position-confirm-dca =
    <b>Confirmar compra adicional</b>

    Token — { $symbol }
    Adición — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>¿Cerrar todas las posiciones?</b>

    Cantidad — { $count }
telegram-position-confirm-close-all-hint =
    <i>Se venderán a mercado todas las posiciones abiertas.
    Confirma en 30 s.</i>
telegram-position-confirm-force-stop =
    <b>DETENCIÓN FORZADA</b>

    Esto detendrá de inmediato TODO el trading:
    • Sin nuevas entradas
    • Sin salidas
    • Sin DCA
telegram-position-confirm-force-stop-warning = <b>Esta es una acción de emergencia.</b>
telegram-position-confirm-blacklist =
    <b>¿Añadir token a la lista negra?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Se cerrará la posición y se impedirán futuras entradas.</i>
telegram-position-selling = Vendiendo { $percent }% de { $symbol }...
telegram-position-sell-done =
    <b>Venta ejecutada</b>

    Token — { $symbol }
    Vendido — { $percent }%
    Recibido — { $amount } { -sol }
telegram-position-sell-failed = <b>Venta fallida</b>
telegram-position-adding = Añadiendo { $amount } { -sol } a { $symbol }...
telegram-position-dca-done =
    <b>DCA ejecutado</b>

    Token — { $symbol }
    Añadido — { $amount } { -sol }
telegram-position-dca-failed = <b>DCA fallido</b>
telegram-position-closing-all = Cerrando todas las posiciones...
telegram-position-close-all-done =
    <b>Cierre total completado</b>

    Cerradas — { $closed }
    Fallidas — { $failed }
telegram-position-blacklisted =
    <b>Token en la lista negra</b>

    Token — { $symbol }
    Estado — Cerrada { "&amp;" } en lista negra

telegram-token-not-found = Token no encontrado
telegram-token-not-found-prefix = Token no encontrado. Prueba a buscar con un prefijo más largo.
telegram-token-stats-failed = Error al obtener las estadísticas: { $detail }
telegram-token-list-failed = Error al obtener los tokens: { $detail }
telegram-token-list-empty = No se encontraron tokens en la vista <b>{ $view }</b>.
telegram-token-view-passed = Filtro aprobado
telegram-token-view-rejected = Rechazados
telegram-token-view-recent = Añadidos recientemente
telegram-token-view-all = Todos los tokens
telegram-token-list-title = <b>{ $name }</b> (Página { $page }/{ $total })
telegram-token-list-stats = Liq.: { $liquidity } • Precio: { $price }
telegram-token-list-hint = <i>Toca /token_ID para ver los detalles</i>
telegram-token-explorer =
    <b>Explorador de mercado</b>

    <b>Resumen</b>
    Filtro aprobado — { $passed }
    Rechazados — { $rejected }
    Precios activos — { $priced }
    Total descubierto — { $total }

    <i>Selecciona una categoría para explorar:</i>
telegram-token-filter-title = <b>Análisis de filtros</b>
telegram-token-filter-distribution = <b>Distribución</b>
telegram-token-filter-passed = Aprobados — { $count } ({ $percent }%)
telegram-token-filter-rejected = Rechazados — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = En lista negra — { $count }
telegram-token-filter-coverage = <b>Cobertura</b>
telegram-token-filter-priced = Con precio de pool — { $count }
telegram-token-filter-open = Posiciones abiertas — { $count }
telegram-token-filter-total = Total descubierto — { $count }
telegram-token-filter-updated = <b>Última actualización</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Se actualiza automáticamente cada { $interval }</i>
telegram-token-detail-active = <b>Posición activa</b>
telegram-token-detail-price = Precio — { $price } { -sol }
telegram-token-detail-liquidity = Liquidez — { $value }
telegram-token-detail-volume = Volumen 24 h — { $value }
telegram-token-detail-change = Cambio 24 h — { $value }
telegram-token-detail-risk = Evaluación de riesgo: { $score }/100
telegram-token-detail-risk-unknown = Evaluación de riesgo: desconocida
telegram-token-detail-action = <i>Selecciona una acción:</i>
telegram-token-search =
    <b>Buscar en el mercado</b>

    Introduce el símbolo o la dirección mint para buscar:

    <i>Ejemplo: /token_BONK o /token_So11111</i>
telegram-token-confirm-buy =
    <b>Confirmar compra directa</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Cantidad — { $amount } { -sol }

    <i>Confirma en 30 s para ejecutar.</i>
telegram-token-confirm-blacklist =
    <b>¿Añadir token a la lista negra?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Se impedirá que este token cumpla los filtros.</i>
telegram-token-blacklisted =
    <b>Token en la lista negra</b>

    Token — ${ $symbol }
    Estado — Añadido a la lista negra
telegram-token-blacklist-failed = <b>Error al añadir a la lista negra</b>
telegram-token-buy-processing =
    <b>Procesando compra...</b>

    Token — ${ $symbol }
    Cantidad — { $amount } { -sol }
telegram-token-buy-done =
    <b>Compra exitosa</b>

    Token — ${ $symbol }
    Cantidad — { $amount } { -sol }

    <i>Consulta los detalles en /positions</i>
telegram-token-buy-failed =
    <b>Compra fallida</b>

    Token — ${ $symbol }
    Error — { $detail }
