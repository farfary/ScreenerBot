# Source: templates/pages/onboarding.html

## Slide: welcome

onboarding-welcome-title = Te damos la bienvenida a { -brand }
onboarding-welcome-description = Tu compañero de trading en Solana con enfoque local, construido en Rust para una velocidad nativa. Descubre tokens, analiza mercados y controla el trading desde tu propio equipo.
onboarding-welcome-free-title = Gratis y de código disponible
onboarding-welcome-free-description = Sin suscripciones ni muros de pago. Inspecciona el código publicado en { -github }.
onboarding-welcome-custody-title = Autocustodia
onboarding-welcome-custody-description = Claves privadas cifradas en reposo y nunca transmitidas a ningún lugar.
onboarding-welcome-engine-title = Motor siempre activo
onboarding-welcome-engine-description = Servicios orquestados con comprobaciones de estado y control ordenado del ciclo de vida.
onboarding-welcome-realtime-title = Datos en la cadena en tiempo real
onboarding-welcome-realtime-description = Cálculos directos de las reservas de los pools, no instantáneas de API con retraso.

## Slide: discover

onboarding-discover-title = Descubre y filtra
onboarding-discover-description = Analiza tres fuentes de datos en busca de nuevos pares de Solana, decodifica más de 12 tipos de pools de DEX en la cadena y pasa cada token por reglas configurables de calidad y seguridad.
onboarding-discover-dex-title = Descubrimiento multi-DEX
onboarding-discover-dex-description = Feeds de { -dexscreener }, { -geckoterminal } y Raydium para pares nuevos de { -sol }.
onboarding-discover-scanner-title = Escáner inteligente de tokens
onboarding-discover-scanner-description = Liquidez, volumen, antigüedad del token, distribución de holders y reglas de { -rugcheck }.
onboarding-discover-intelligence-title = Inteligencia de tokens
onboarding-discover-intelligence-description = Datos de mercado, seguridad y lista negra unificados en una sola vista.
onboarding-discover-price-action-title = Seguimiento de la acción del precio
onboarding-discover-price-action-description = Siete temporalidades de velas con detección de huecos y señales de momentum.

## Slide: trade

onboarding-trade-title = Opera con inteligencia
onboarding-trade-description = Trading automático con un sistema de prioridad de salidas de seis niveles. Haz DCA en tus posiciones, define trailing stops, crea árboles de estrategias o opera manualmente con un clic.
onboarding-trade-auto-title = Trading automático
onboarding-trade-auto-description = Evaluadores de entrada y salida, rondas de DCA, salidas parciales y trailing stop loss.
onboarding-trade-strategy-title = Motor de estrategias
onboarding-trade-strategy-description = Árboles de condiciones que combinan señales de precio, volumen y tiempo.
onboarding-trade-routing-title = Enrutamiento al mejor precio
onboarding-trade-routing-description = Cotizaciones simultáneas de cada enrutador habilitado: gana la mejor ruta.
onboarding-trade-safety-title = Controles de seguridad
onboarding-trade-safety-description = Parada de emergencia, límites de pérdidas por período e interruptores de monitor independientes.

## Slide: connect

onboarding-connect-title = Mantente conectado
onboarding-connect-description = Monitorea tu portafolio desde cualquier lugar. Un Asistente respaldado por nueve proveedores de LLM, alertas de { -telegram } con trading integrado y un registro de eventos con búsqueda.
onboarding-connect-assistant-title = Asistente
onboarding-connect-assistant-description = Análisis por chat con llamadas a herramientas para operaciones, configuración y portafolio.
onboarding-connect-telegram-title = Integración con { -telegram }
onboarding-connect-telegram-description = Notificaciones, comandos integrados y sesiones protegidas con 2FA desde tu teléfono.
onboarding-connect-wallets-title = Seguimiento de varias billeteras
onboarding-connect-wallets-description = Todas tus billeteras de Solana y tus tenencias de tokens en un solo panel.
onboarding-connect-events-title = Flujo de eventos en vivo
onboarding-connect-events-description = Cada operación, swap y evento del sistema se registra con categoría y gravedad.

## Slide: data

onboarding-data-title = Datos de { -brand }
onboarding-data-description = Operamos un servicio de datos de mercado compartido para que cada instalación no tenga su propio límite de velocidad con los proveedores públicos. Es gratis con una cuenta de { -brand }, y { -brand } funciona sin ella.
onboarding-data-candles-title = Historial de velas compartido
onboarding-data-candles-description = Siete temporalidades de historial compartido, con años de profundidad, servidas desde una sola caché.
onboarding-data-pools-title = Pools resueltos y seguridad
onboarding-data-pools-description = Un registro central de pools e informes de { -rugcheck } en caché, ya obtenidos.
onboarding-data-signin-title = Inicia sesión para usarlos
onboarding-data-signin-description = Sin una cuenta, estos datos no están disponibles y se usan los proveedores públicos.
onboarding-data-reading-title = Solo lectura
onboarding-data-reading-description = Vemos qué tokens consultas. Nunca una clave, un saldo, una posición ni una operación.

## Slide: privacy

onboarding-privacy-title = Tus claves, tus datos
onboarding-privacy-description = Tu configuración, tus claves y tu historial de trading permanecen en este equipo. A continuación, elige el Modo Explorar para descubrir sin credenciales, o vincula una billetera y un RPC para habilitar el bot completo, e inicia sesión allí si quieres los datos de { -brand }.
onboarding-privacy-local-title = Arquitectura local primero
onboarding-privacy-local-description = Configuración, analíticas y bases de datos almacenadas en tu equipo.
onboarding-privacy-wallet-title = Billetera cifrada
onboarding-privacy-wallet-description = Tu clave privada se cifra en reposo y nunca se transmite.
onboarding-privacy-security-title = Seguridad del panel
onboarding-privacy-security-description = Bloqueo con contraseña, TOTP en dos pasos y protección por tiempo de inactividad de la sesión.
onboarding-privacy-config-title = Configuración flexible
onboarding-privacy-config-description = La mayoría de los ajustes se pueden cambiar desde el panel después de la configuración.

## Footer

onboarding-setup-shortcut =
    .aria-label = Ir directamente a la configuración de billetera y RPC o elegir el Modo Explorar
onboarding-setup-shortcut-label = Ir a la configuración
onboarding-progress-dot =
    .aria-label = Ir a la diapositiva { $number }
onboarding-action-continue-to-setup = Continuar con la configuración
