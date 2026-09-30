hints-category-tokens = Tokens
hints-category-positions = Posiciones
hints-category-filtering = Filtrado
hints-category-trader = Trader automático
hints-category-services = Servicios
hints-category-wallet = Billetera
hints-category-wallets = Billeteras
hints-category-tools = Herramientas
hints-category-config = Configuración
hints-category-config-telegram = { -telegram }
hints-category-token-details = Detalles del token
hints-category-ui = Interfaz

hints-tokens-pool-service-title = Tokens del servicio de pools
hints-tokens-pool-service-content =
    Los tokens que se muestran aquí tienen:

    • **Todos los criterios de filtrado superados** — comprobaciones de liquidez, volumen, antigüedad y seguridad
    • **Pools de liquidez de SOL válidos** — compatibles con nuestros decodificadores de DEX (Raydium, Orca, Meteora, etc.)
    • **Cálculo de precio exitoso** — precios calculados directamente a partir de las reservas del pool en la cadena

    Es la lista de tokens más fiable para operar, ya que los precios provienen de los datos reales del pool y no de APIs externas.

    Haz clic en cualquier token para ver información detallada y gestionar su estado en la lista negra.
hints-tokens-no-market-title = Sin datos de mercado
hints-tokens-no-market-content =
    Tokens descubiertos en la cadena que no tienen datos de mercado de { -dexscreener } ni de { -geckoterminal }.

    Motivos habituales:
    • **Tokens muy nuevos** — aún no indexados por los agregadores
    • **Poco volumen de trading** — por debajo de los umbrales de los agregadores
    • **Pares no listados** — operan en DEX que los agregadores no siguen

    Estos tokens aún pueden tener pools válidos y se pueden operar, pero carecen de métricas de mercado externas.
hints-tokens-all-title = Todos los tokens
hints-tokens-all-content =
    Base de datos completa de los tokens descubiertos, sin importar su estado de filtrado.

    Incluye:
    • Tokens que superaron el filtrado
    • Tokens que fueron rechazados
    • Tokens sin datos de mercado
    • Tokens en la lista negra

    Usa esta vista para investigar o para encontrar tokens que pudieron quedar filtrados.
hints-tokens-passed-title = Filtrado aprobado
hints-tokens-passed-content =
    Tokens que superaron todos los criterios de filtrado activos.

    Las comprobaciones del filtrado incluyen:
    • **Liquidez** — umbral mínimo de liquidez en SOL
    • **Volumen** — requisitos de volumen de trading en 24 h
    • **Antigüedad del token** — tiempo mínimo desde su creación
    • **Seguridad** — límites de la puntuación de riesgo de { -rugcheck }
    • **Capitalización de mercado** — filtros opcionales de FDV/MC

    Configura los filtros en la página **Filtrado**.
hints-tokens-rejected-title = Tokens rechazados
hints-tokens-rejected-content =
    Tokens que no cumplieron uno o más criterios de filtrado.

    Cada token muestra el motivo concreto del rechazo:
    • Qué filtro falló
    • El valor real frente al umbral requerido
    • Cuándo se hizo la comprobación

    Revisa los tokens rechazados para ajustar tu configuración de filtros.
hints-tokens-blacklisted-title = Tokens en la lista negra
hints-tokens-blacklisted-content =
    Tokens excluidos del trading de forma permanente.

    Los motivos de la lista negra incluyen:
    • **Lista negra manual** — tokens que has bloqueado explícitamente
    • **Riesgos de seguridad** — indicadores de rug pull detectados
    • **Umbral de pérdida** — se superaron los límites de pérdidas configurados
    • **Transacciones fallidas** — fallos repetidos de swap

    Los tokens en la lista negra nunca aparecen en las listas de aprobados ni se consideran para el trading automático.
hints-tokens-positions-title = Tokens con posición
hints-tokens-positions-content =
    Tokens que tienes actualmente en posiciones abiertas.

    Muestra datos en tiempo real de tus tenencias activas:
    • Precio actual a partir de las reservas del pool
    • P&L no realizado
    • Tamaño de la posición y precio de entrada
    • Tiempo en posesión

    Haz clic en cualquier token para gestionar la posición en detalle.
hints-tokens-recent-title = Descubiertos recientemente
hints-tokens-recent-content =
    Tokens recién descubiertos, ordenados por fecha de descubrimiento.

    Útil para:
    • Detectar nuevos lanzamientos de tokens
    • Monitorear liquidez reciente
    • Oportunidades de entrada temprana

    Nota: los tokens nuevos pueden no tener al principio datos de mercado completos.
hints-tokens-ohlcv-title = Gestión de datos OHLCV
hints-tokens-ohlcv-content =
    Consulta y gestiona los datos OHLCV (velas) almacenados para los tokens.

    Muestra:
    • **Número de velas** — total de puntos de datos almacenados
    • **Progreso del relleno histórico** — estado de finalización por temporalidad
    • **Rango de datos** — cobertura temporal en horas
    • **Número de pools** — pools de liquidez seguidos
    • **Estado** — monitoreo activo o inactivo

    Acciones:
    • **Eliminar** — quita todos los datos OHLCV de un token
    • **Limpiar** — elimina en bloque los datos de tokens inactivos

    Los datos OHLCV se conservan de forma permanente y nunca se eliminan automáticamente.

hints-positions-overview-title = Resumen de posiciones
hints-positions-overview-content =
    Tus tenencias de tokens y posiciones de trading actuales.

    Métricas clave:
    • **Precio de entrada** — precio medio pagado (incluido el DCA)
    • **Precio actual** — precio en vivo a partir de las reservas del pool
    • **P&L** — ganancia o pérdida no realizada en SOL y en %
    • **Tamaño** — cantidad total de tokens en posesión

    Haz clic en cualquier posición para ver las opciones de gestión detalladas.
hints-positions-dca-title = DCA (promedio de costo en dólares)
hints-positions-dca-content =
    El DCA permite añadir a posiciones existentes a distintos precios.

    Cuando se activa el DCA:
    • Se compran tokens adicionales
    • El precio de entrada se recalcula como promedio ponderado
    • El tamaño de la posición aumenta
    • El contador de entradas se incrementa

    Configura las reglas de DCA en los ajustes del **Trader automático**.
hints-positions-partial-exit-title = Salida parcial
hints-positions-partial-exit-content =
    Vende una parte de tu posición y conserva el resto.

    Ventajas:
    • Asegura parte de las ganancias sin perder la exposición
    • Reduce el tamaño de la posición sin cerrarla del todo
    • Permite implementar escalones de take profit

    Cada salida parcial se registra por separado para un seguimiento preciso del P&L.
hints-positions-management-title = Gestión de posiciones
hints-positions-management-content =
    La gestión define qué automatización puede actuar sobre una posición:

    • Trader automático: salidas de seguridad, salidas por política y DCA automático
    • Solo usuario: sin acciones automáticas
    • Tarea de copia: salidas de seguridad y ventas de copia
    • Híbrida: salidas de seguridad, salidas por política y ventas de copia

    Tú vendes o añades a la posición por tu cuenta. Las compras manuales usan por defecto la gestión manual para que el bot no pueda vender un token que compraste a propósito. Desactívala para devolver la posición al trader automático.

hints-filtering-overview-title = Filtrado de tokens
hints-filtering-overview-content =
    El filtrado determina qué tokens pueden operarse.

    Los tokens deben superar **todos los criterios activados** para aparecer en la lista de aprobados:
    • Métricas de { -dexscreener } (liquidez, volumen, etc.)
    • Métricas de { -geckoterminal } (capitalización de mercado, FDV)
    • Análisis de seguridad de { -rugcheck }
    • Filtros meta (antigüedad del token, etc.)

    Los criterios desactivados se omiten por completo.
hints-filtering-dexscreener-title = Filtros de { -dexscreener }
hints-filtering-dexscreener-content =
    Filtros basados en los datos de mercado de { -dexscreener }:

    • **Liquidez** — liquidez mínima en USD en los pools
    • **Volumen 24 h** — volumen de trading mínimo
    • **Transacciones** — umbrales de actividad (compras/ventas)
    • **Cambio de precio** — filtros de volatilidad

    Los datos de { -dexscreener } se actualizan cada pocos minutos.
hints-filtering-geckoterminal-title = Filtros de { -geckoterminal }
hints-filtering-geckoterminal-content =
    Filtros basados en los datos de mercado de { -geckoterminal }:

    • **Capitalización de mercado** — capitalización mínima
    • **FDV** — límites de valoración totalmente diluida
    • **Ratio de reservas** — indicadores de salud del pool

    { -geckoterminal } suele tener datos de tokens más nuevos.
hints-filtering-rugcheck-title = Filtros de seguridad
hints-filtering-rugcheck-content =
    Análisis de seguridad de { -rugcheck }.xyz:

    • **Puntuación de riesgo** — calificación de riesgo global (0-100)
    • **Autoridad de mint** — ¿se pueden crear más tokens?
    • **Autoridad de congelación** — ¿se pueden congelar las transferencias?
    • **Principales holders** — riesgo de concentración

    Las puntuaciones de riesgo más altas indican más posibles señales de alerta.
hints-filtering-meta-title = Filtros meta
hints-filtering-meta-content =
    Criterios de filtrado adicionales:

    • **Antigüedad del token** — tiempo mínimo desde la creación del token
    • **Antigüedad del pool** — tiempo mínimo desde la creación del pool
    • **Tiene sitio web** — exige sitio web o enlaces sociales
    • **Tiene redes sociales** — exige Twitter/{ -telegram }

    Ayudan a descartar tokens muy nuevos o sospechosos.

hints-trader-overview-title = Trader automático
hints-trader-overview-content =
    Motor de trading automatizado que monitorea tokens y ejecuta operaciones.

    Componentes:
    • **Monitor de entradas** — busca oportunidades de compra
    • **Monitor de salidas** — gestiona las ventas y los take profit
    • **Monitor de DCA** — gestiona el promedio de posiciones
    • **Controles de riesgo** — límites de pérdidas y barreras de seguridad

    Inicia o detén el trading desde el panel de control.
hints-trader-entry-title = Monitor de entradas
hints-trader-entry-content =
    Vigila los tokens filtrados en busca de señales de entrada.

    La evaluación de entrada comprueba que:
    • El token supere el filtrado actual
    • No tenga ya una posición
    • No esté en la lista negra
    • No se superen los límites de posiciones
    • Se cumplan las condiciones de la estrategia (si está configurada)

    Configura el tamaño y los límites de entrada en Configuración.
hints-trader-exit-title = Monitor de salidas
hints-trader-exit-content =
    Monitorea las posiciones abiertas en busca de señales de salida.

    Disparadores de salida:
    • **Take profit** — se alcanzó el precio objetivo
    • **Stop loss** — se superó la pérdida máxima
    • **Trailing stop** — el precio retrocedió desde su máximo
    • **Salida de estrategia** — se cumplen condiciones personalizadas
    • **Por tiempo** — duración máxima de tenencia

    Configura los umbrales en Configuración.

hints-services-overview-title = Servicios del sistema
hints-services-overview-content =
    Servicios en segundo plano que hacen funcionar { -brand }.

    Estados de los servicios:
    • **En ejecución** (verde) — funciona con normalidad
    • **Iniciando** (amarillo) — se está inicializando
    • **Detenido** (rojo) — no se está ejecutando
    • **Error** (advertencia) — falló, puede reiniciarse solo

    Los servicios tienen dependencias y se inician en orden.
hints-services-health-title = Salud de los servicios
hints-services-health-content =
    Los indicadores de salud muestran el estado del servicio:

    • **Tiempo activo** — tiempo desde el último inicio
    • **Tareas** — operaciones activas en segundo plano
    • **Errores** — número de errores recientes
    • **Métricas** — datos de rendimiento (si están disponibles)

    Los servicios críticos afectan a la capacidad de operar.

hints-wallet-overview-title = Resumen de la billetera
hints-wallet-overview-content =
    Estado de tu billetera de Solana conectada.

    Muestra:
    • **Saldo de SOL** — SOL nativo para comisiones y trading
    • **Tenencias de tokens** — tokens SPL con sus valores
    • **Cambio 24 h** — variación del valor de la cartera
    • **Historial** — instantáneas del saldo a lo largo del tiempo

    Los saldos se actualizan cada minuto.
hints-wallet-tokens-title = Saldos de tokens
hints-wallet-tokens-content =
    Tokens SPL que tienes en tu billetera.

    Muestra:
    • Símbolo y nombre del token
    • Cantidad en posesión
    • Valor actual en SOL/USD
    • Precio del pool o de los datos de mercado

    Las cuentas de token vacías se pueden limpiar en Configuración.

hints-wallets-main-title = Billetera principal
hints-wallets-main-content =
    La billetera principal que se usa en todas las operaciones de trading.

    • **Trading automático** — las operaciones de entrada y salida se ejecutan desde esta billetera
    • **Saldo visible** — se muestra en el encabezado y en el panel
    • **Tenencias de tokens** — tokens SPL que posee esta billetera

    Cambia la billetera principal seleccionando "Establecer como principal" en cualquier billetera secundaria.
hints-wallets-secondary-title = Billeteras secundarias
hints-wallets-secondary-content =
    Billeteras adicionales para operaciones con varias billeteras.

    • **Trading con varias billeteras** — coordina compras y ventas entre billeteras
    • **Separación de la cartera** — organiza por estrategia o propósito
    • **Saldos independientes** — cada billetera tiene su propio SOL y sus tokens

    El trading automático no usa las billeteras secundarias a menos que se configure explícitamente.

hints-tools-wallet-cleanup-title = Herramienta de limpieza de billetera
hints-tools-wallet-cleanup-content =
    { "*" }*Recupera SOL de las cuentas de token vacías**

    { "*" }*¿Qué son las ATA?**
    Las Associated Token Accounts (ATA) son cuentas de Solana que guardan tus tokens. Cada token con el que interactúas crea una ATA que requiere ~0.002 SOL de renta.

    { "*" }*¿Por qué limpiar las ATA vacías?**
    • Recuperar la renta (~0.002 SOL por ATA)
    • Los traders activos pueden acumular cientos de ATA vacías
    • 100 ATA vacías = ~0.2 SOL recuperables

    { "*" }*Cómo funciona:**
    • Escanea tu billetera en busca de ATA con saldo cero
    • Muestra el total de SOL recuperable
    • Cierra las cuentas vacías para recuperar la renta

    { "*" }*Limpieza automática:**
    Cuando está activada, escanea y cierra automáticamente las ATA vacías cada 5 minutos en segundo plano.

    { "*" }*Importante:**
    • Solo cierra cuentas con saldo exactamente 0
    • Los cierres fallidos se guardan en caché para evitar reintentos repetidos
    • Las billeteras grandes pueden requerir varias pasadas de limpieza
hints-tools-burn-tokens-title = Herramienta para quemar tokens
hints-tools-burn-tokens-content =
    { "*" }*Destruye tokens de forma permanente**

    Quemar tokens los elimina de forma permanente de tu billetera y de la circulación.

    { "*" }*Qué ocurre al quemar:**
    • Los tokens se envían a una dirección de quema (irrecuperable)
    • El saldo del token pasa a cero
    • Después se puede cerrar la ATA con la limpieza de billetera para recuperar ~0.002 SOL de renta

    { "*" }*Categorías de tokens:**
    • **Posiciones abiertas** - No se pueden quemar (operaciones activas)
    • **Posiciones cerradas** - Restos de operaciones pasadas
    • **Con valor** - Tokens con liquidez (considera venderlos)
    • **Sin liquidez** - Tokens sin valor o residuales (seguros de quemar)

    { "*" }*Advertencia:** Esta acción es **irreversible**. Los tokens quemados no se pueden recuperar bajo ninguna circunstancia.

    { "*" }*Después de quemar:** Ejecuta la limpieza de billetera para cerrar las ATA vacías y recuperar la renta en SOL.
hints-tools-wallet-generator-title = Herramienta generadora de billeteras
hints-tools-wallet-generator-content =
    { "*" }*Genera nuevos pares de claves de Solana**

    Crea billeteras nuevas de forma segura en tu dispositivo.

    { "*" }*Funciones:**
    • Genera pares de claves criptográficamente seguros
    • Prefijo de dirección personalizada opcional (p. ej., "SOL...")
    • Exportación en base58 o como matriz JSON

    { "*" }*Seguridad:**
    • Las claves se generan localmente
    • Nunca se transmiten por la red
    • Haz siempre una copia de seguridad segura de las claves
hints-tools-multi-buy-title = Herramienta de compra múltiple
hints-tools-multi-buy-content =
    { "*" }*Coordina compras entre varias billeteras**

    Ejecuta órdenes de compra en varias subbilleteras con montos aleatorios para simular una actividad de compra orgánica.

    { "*" }*Cómo funciona:**
    1. Crea o usa subbilleteras existentes
    2. Distribuye SOL de la billetera principal a las subbilleteras
    3. Ejecuta órdenes de compra con montos y retrasos aleatorios
    4. Cada billetera compra de forma independiente con firmas únicas

    { "*" }*Ajustes de billeteras:**
    • **Número de billeteras** — cantidad de subbilleteras a usar (2-10)
    • **Reserva de SOL** — SOL reservado por billetera para comisiones (~0.015)

    { "*" }*Ajustes de montos:**
    • **SOL mín./máx.** — rango de montos de compra por billetera
    • **Límite total** — tope opcional del total de SOL a gastar

    { "*" }*Ajustes de ejecución:**
    • **Retraso** — retraso aleatorio entre transacciones
    • **Concurrencia** — ejecución en paralelo (1 = secuencial)
    • **Deslizamiento** — deslizamiento máximo aceptable
    • **Enrutador** — enrutamiento del swap (Auto, Jupiter, Raydium)

    { "*" }*Importante:**
    • Requiere suficiente SOL en la billetera principal
    • Las compras fallidas se registran pero no detienen la sesión
    • Las subbilleteras se pueden reutilizar entre sesiones
hints-tools-multi-sell-title = Herramienta de venta múltiple
hints-tools-multi-sell-content =
    { "*" }*Coordina ventas entre varias billeteras**

    Vende tokens de todas las subbilleteras que tengan un token concreto, con consolidación automática del SOL.

    { "*" }*Cómo funciona:**
    1. Escanea las subbilleteras en busca de saldos del token
    2. Opcionalmente recarga las billeteras con poco SOL para comisiones
    3. Ejecuta órdenes de venta con un porcentaje configurable
    4. Consolida lo obtenido de vuelta en la billetera principal

    { "*" }*Ajustes de venta:**
    • **% de venta** — porcentaje de tokens a vender (100% por defecto)
    • **SOL mín. para comisión** — SOL mínimo necesario para la transacción
    • **Recarga automática** — transfiere SOL desde la principal si hace falta

    { "*" }*Acciones posteriores a la venta:**
    • **Consolidar SOL** — transfiere todo el SOL de vuelta a la billetera principal
    • **Cerrar ATA** — cierra las cuentas de token para recuperar la renta (~0.002 SOL cada una)

    { "*" }*Ajustes de ejecución:**
    • **Retraso** — retraso aleatorio entre transacciones
    • **Concurrencia** — ejecución en paralelo
    • **Deslizamiento** — deslizamiento máximo aceptable
    • **Enrutador** — preferencia de enrutamiento del swap

    { "*" }*Consejos:**
    • La vista previa muestra todas las billeteras que tienen el token
    • Deselecciona las billeteras desde las que no quieras vender
    • La consolidación se realiza cuando terminan todas las ventas
hints-tools-trade-watcher-title = Herramienta de seguimiento de operaciones
hints-tools-trade-watcher-content =
    { "*" }*Monitorea operaciones y activa acciones automáticas**

    Observa la actividad de trading de un token y reacciona automáticamente cuando se producen operaciones.

    { "*" }*Tipos de seguimiento:**
    • **Comprar al vender** — compra automáticamente cuando alguien vende (aprovecha las caídas)
    • **Vender al comprar** — vende automáticamente cuando alguien compra (sigue al mercado)
    • **Solo notificar** — recibe alertas sin realizar ninguna acción

    { "*" }*Cómo funciona:**
    1. Introduce la dirección mint de un token
    2. Haz clic en "Buscar pools" para encontrar los pools de liquidez disponibles
    3. Selecciona un pool para monitorear (obligatorio para acciones de compra o venta)
    4. Define el monto de activación (tamaño mínimo de operación al que reaccionar)
    5. Define el monto de la acción (cuánto SOL comprar o vender)
    6. Inicia el seguimiento

    { "*" }*Requisitos:**
    • Dirección mint de token válida
    • Selección de un pool (para acciones de compra o venta)
    • Saldo de SOL suficiente para los montos de las acciones

    { "*" }*Integración con { -telegram }:**
    Configura { -telegram } en Configuración → { -telegram } para recibir notificaciones instantáneas cuando se activen los seguimientos.
hints-tools-wallet-consolidation-title = Herramienta de consolidación de billeteras
hints-tools-wallet-consolidation-content =
    { "*" }*Gestiona y consolida los fondos de las subbilleteras**

    Consulta todas las subbilleteras y consolida SOL, tokens y la renta de las ATA en tu billetera principal.

    { "*" }*El resumen muestra:**
    • **Subbilleteras** — número total de subbilleteras creadas
    • **SOL total** — saldo de SOL combinado de todas las subbilleteras
    • **Tipos de token** — número de tokens distintos en posesión
    • **Renta recuperable** — SOL bloqueado en ATA vacías

    { "*" }*Acciones:**
    • **Transferir SOL** — mueve todo el SOL de las billeteras seleccionadas a la principal
    • **Transferir tokens** — mueve todos los tokens a la billetera principal
    • **Limpiar ATA** — cierra las cuentas de token vacías para recuperar la renta

    { "*" }*Información de la tabla:**
    • Casilla para seleccionar billeteras en operaciones por lotes
    • Nombre, dirección, saldo de SOL, número de tokens, ATA vacías
    • Las billeteras vacías se atenúan para identificarlas fácilmente

    { "*" }*Consejos:**
    • Úsala después de una venta múltiple para recoger el SOL restante
    • Limpia las ATA con regularidad para recuperar la renta
    • Las billeteras vacías se pueden reutilizar en operaciones futuras

hints-config-overview-title = Configuración
hints-config-overview-content =
    Ajustes globales de { -brand }.

    Categorías:
    • **Trader** — reglas de entrada y salida, tamaño de las posiciones
    • **Filtrado** — umbrales de los filtros de tokens
    • **Swaps** — ajustes de enrutamiento y deslizamiento
    • **RPC** — configuración de nodos
    • **Servicios** — ajustes de los servicios en segundo plano

    Los cambios se aplican de inmediato (recarga en caliente).
hints-config-telegram-title = Notificaciones de { -telegram }
hints-config-telegram-content =
    { "*" }*Recibe alertas de trading al instante por { -telegram }**

    Recibe avisos de operaciones, posiciones y eventos importantes directamente en { -telegram }.

    { "*" }*Pasos de configuración:**

    1. **Crea un bot:**
       • Abre { -telegram } y escribe a @BotFather
       • Envía /newbot y sigue las indicaciones
       • Copia el token del bot (tiene este aspecto: 123456:ABC-DEF...)

    2. **Obtén tu ID de chat:**
       • Escribe a @userinfobot o @getidsbot
       • Copia el ID numérico que te devuelva

    3. **Configura en { -brand }:**
       • Activa el interruptor de notificaciones
       • Pega el token del bot y el ID de chat
       • Haz clic en "Probar conexión" para verificar

    { "*" }*Qué recibirás:**
    • Confirmaciones de ejecución de operaciones
    • Actualizaciones de posiciones (entrada/salida)
    • Alertas del seguimiento de operaciones
    • Notificaciones de error

    { "*" }*Privacidad:**
    Los mensajes se envían directamente desde { -brand } a tu bot de { -telegram }, sin servidores de terceros de por medio.
hints-config-telegram-password-title = Contraseña de autenticación del bot
hints-config-telegram-password-content =
    { "*" }*Protege tu bot de { -telegram } con una contraseña**

    Cuando interactúes con tu bot de { -telegram } de { -brand }, tendrás que autenticarte con esta contraseña antes de ejecutar comandos sensibles.

    { "*" }*¿Por qué establecer una contraseña?**
    • Impide que usuarios no autorizados controlen tu bot
    • Es obligatoria para ejecutar comandos de trading por { -telegram }
    • Debe tener al menos 8 caracteres

    { "*" }*Cómo funciona:**
    1. Establece una contraseña aquí, en el panel
    2. Cuando envíes un comando de trading a tu bot, te pedirá autenticación
    3. Introduce tu contraseña para verificar tu identidad
    4. Opcionalmente, activa 2FA para mayor seguridad

    { "*" }*Nota:** La contraseña se guarda como un hash SHA256 seguro; nunca almacenamos el texto sin cifrar.
hints-config-telegram-totp-title = Autenticación de dos factores (2FA)
hints-config-telegram-totp-content =
    { "*" }*Añade una capa extra de seguridad con 2FA TOTP**

    La autenticación de dos factores usa contraseñas de un solo uso basadas en el tiempo (TOTP) de apps como Google Authenticator, Authy o 1Password.

    { "*" }*¿Por qué activar 2FA?**
    • Aunque alguien conozca tu contraseña, no podrá acceder a tu bot sin el código
    • Los códigos de 6 dígitos cambian cada 30 segundos
    • Funciona sin conexión una vez configurado

    { "*" }*Proceso de configuración:**
    1. Haz clic en "Activar 2FA" e introduce tu contraseña
    2. Escanea el código QR con tu app de autenticación
    3. Introduce el código de 6 dígitos para verificar la configuración

    { "*" }*Apps compatibles:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Cualquier app compatible con TOTP

    { "*" }*Importante:** Guarda tu clave secreta en un lugar seguro. Si pierdes el acceso a tu app de autenticación, tendrás que desactivar 2FA desde este panel.

hints-token-details-chart-title = Gráfico de precio (OHLCV)
hints-token-details-chart-content =
    { "*" }*Importante:** Este gráfico muestra **datos OHLCV en caché** para la evaluación de estrategias, *no* el precio de ejecución en vivo.

    { "*" }*¿Por qué datos en caché?**
    • **Propósito:** Los usan las estrategias e indicadores automatizados (p. ej., RSI, MA).
    • **Frescura:** Las actualizaciones dependen de la prioridad del token (posiciones abiertas = actualizaciones más rápidas).
    • **Fuente:** Agregados de { -dexscreener }/{ -geckoterminal }, no del RPC directo de la cadena.

    { "*" }*La realidad del precio en los DEX:**
    En DeFi, los tokens se operan en **varios pools** (Raydium, Orca, Meteora). Cada pool tiene un precio propio según la profundidad de liquidez y las operaciones recientes.
    • **Precio del gráfico:** Un promedio o agregado entre mercados.
    • **Precio del swap:** La tasa concreta que obtienes de la mejor ruta en el momento exacto de la operación.

    { "*" }Es normal que haya pequeñas diferencias entre este gráfico y tu precio de ejecución final.*

    { "*" }*Estado:** "Esperando datos" significa que los procesos en segundo plano están obteniendo velas nuevas.
hints-token-details-token-info-title = Información del token
hints-token-details-token-info-content =
    Metadatos básicos del token de fuentes on-chain y de mercado.

        • **Mint** — dirección única del token en Solana (haz clic para copiar)
        • **Decimales** — precisión del token (normalmente 6-9)
        • **Antigüedad** — tiempo desde que se creó el pool o token principal
        • **DEX** — plataforma de trading principal de este token
        • **Holders** — billeteras únicas que poseen el token
        • **Top 10 posee** — % en manos de las 10 billeteras principales

        Un mayor número de holders y una menor concentración suelen indicar una distribución más sana.
hints-token-details-liquidity-title = Liquidez y datos de mercado
hints-token-details-liquidity-content =
    Métricas de mercado del pool de SOL con mayor liquidez.

        • **FDV** — precio × suministro total (precio del agregador)
        • **Liquidez** — valor en USD de las reservas del pool
        • **SOL del pool** / **Token del pool** — reservas en vivo que fijan el precio del pool

        { "*" }*Por qué importa:**
        • Más liquidez = menos deslizamiento
        • Los pools poco profundos pueden moverse con operaciones pequeñas
        • Las reservas del pool fijan directamente el precio de ejecución del swap

        Los datos se actualizan periódicamente desde { -dexscreener }/{ -geckoterminal } y con lecturas de pools en la cadena.
hints-token-details-market-pulse-title = Pulso del mercado
hints-token-details-market-pulse-content =
    El movimiento del precio y el volumen de trading en USD comparten la misma línea temporal **5M / 1H / 6H / 24H**, de modo que se pueden comparar directamente el impulso y la participación.

    { "*" }*Interpretación:**
    • **Precio** — variación porcentual derivada del agregador, no el precio de ejecución en vivo del pool.
    • **Volumen alto** — más interés, descubrimiento de precio más eficiente y salidas más fáciles.
    • **Volumen bajo** — más deslizamiento, diferenciales más amplios y salidas grandes más difíciles.
    • **Volumen alto + poca liquidez** — mayor volatilidad y riesgo de ejecución.

    Los datos de mercado se agregan de los principales DEX mediante { -dexscreener }/{ -geckoterminal }, por lo que el cambio de precio puede diferir del precio actual del pool en la cadena.
hints-token-details-activity-title = Actividad de transacciones (recuentos)
hints-token-details-activity-content =
    Analiza el **número de operaciones** (compras frente a ventas) en varias temporalidades. Esto revela la intención de los traders sin importar el tamaño de la operación.

    { "*" }*Desglose de métricas:**
    • **Temporalidades:** ventanas de 5M, 1H, 6H, 24H.
    • **Barras:** Proporción visual entre compras (verde) y ventas (rojo).
    • **Ritmo:** Operaciones por minuto (p. ej., "12.5/min"). Ritmos más altos = actividad viral.
    • **Recuentos:** Número exacto de compras y ventas y su porcentaje.

    { "*" }*Métricas resumen:**
    • **% de compras 24H:** >50% es alcista (más compradores), { "<" }50% es bajista (más vendedores).
    • **Flujo neto:** Total de compras menos ventas. Positivo = acumulación.
    • **Pico 5M:** Cuánto más rápido se opera *ahora mismo* frente al promedio de 1H.
      • **>1.0x:** Interés en aceleración.
      • **>3.0x:** Ruptura viral o evento de pánico.
      • **{ "<" }1.0x:** Enfriándose.

    { "*" }*Consejo de estrategia:** Un "% de compras" alto con un "factor de pico" alto suele indicar una buena entrada en ruptura.
hints-token-details-security-title = Análisis de seguridad
hints-token-details-security-content =
    Evaluación de riesgo de { -rugcheck }.xyz y análisis en la cadena.

    { "*" }*Puntuación de seguridad (0-100):**
    Las puntuaciones más altas indican tokens más seguros. Los factores incluyen:
    • Permisos de autoridad (mint/congelación)
    • Concentración de holders
    • Estado de bloqueo del LP
    • Patrones de riesgo conocidos

    { "*" }*Indicadores de riesgo clave:**
    • **Autoridad de mint** — puede crear tokens nuevos (riesgo de inflación)
    • **Autoridad de congelación** — puede congelar cuentas de token
    • **% del holder principal** — riesgo de concentración
    • **Proveedores de LP** — número de proveedores de liquidez

    Verifica siempre la seguridad antes de operar con montos importantes.
hints-token-details-pools-title = Pools de liquidez
hints-token-details-pools-content =
    Todos los pools de liquidez descubiertos para este token.

    { "*" }*Por qué importan varios pools:**
    • Cada pool tiene distinta liquidez y precio
    • Los enrutadores de swap encuentran la mejor ruta entre pools
    • El precio puede variar entre un 1 y un 5% entre pools

    { "*" }*Información del pool:**
    • **DEX** — qué exchange aloja el pool
    • **Liquidez** — valor en USD de las reservas del pool
    • **Volumen** — actividad de trading reciente
    • **Precio** — precio actual del pool

    El servicio de pools calcula los precios a partir del par de SOL con mayor liquidez.

hints-ui-featured-title = Destacados
hints-ui-featured-content =
    Primero los tokens impulsados y después los proyectos en tendencia de Jupiter y { -dexscreener }.

    { "*" }*Qué verás:**
    • Tokens impulsados — sus equipos pagaron por promocionarlos — fijados al principio y marcados en dorado
    • Después, tokens en tendencia de los tableros de descubrimiento
    • Haz clic en cualquier token para abrir sus detalles completos

    { "*" }*Impulsar un token:**
    Un impulso compra visibilidad, nunca es una recomendación. Las filas impulsadas se marcan en dorado
    en todas partes donde aparecen, incluida tu tabla de tokens, para que siempre sepas cuál es cuál. Impulsa un token en
    { "*" }*screenerbot.io/boost**.

    { "*" }*Desactivar la fila:**
    Ocúltala en **Configuración → Interfaz → Mostrar fila de destacados**. La acción del encabezado sigue abriendo la
    vista completa de Destacados.

hints-trigger =
    .aria-label = Ayuda: { $title }
hints-popover-close =
    .aria-label = Cerrar
hints-popover-learn-more = Más información
hints-popover-dismiss = No volver a mostrar
