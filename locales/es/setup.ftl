# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = Introduce la clave privada de una billetera.
setup-wallet-json-recognized = Formato de clave JSON de 64 bytes reconocido.
setup-wallet-json-invalid = Usa un arreglo JSON con exactamente 64 valores de byte (0–255).
setup-wallet-format-invalid = Usa una clave privada en base58 o un arreglo JSON de 64 bytes.
setup-wallet-base58-recognized = Formato de clave base58 reconocido.

## RPC endpoint validation

setup-rpc-required = Introduce al menos un endpoint RPC.
setup-dialog-rpc-optional-note = Opcional: deja el campo vacío para mantener los endpoints RPC ya configurados.
setup-rpc-too-many = Usa como máximo 10 endpoints RPC.
setup-rpc-url-invalid = Todos los endpoints deben ser URL HTTPS válidas.
setup-rpc-url-credentials = Las URL de RPC no pueden incluir nombres de usuario ni contraseñas.
setup-rpc-url-fragment = Las URL de RPC no pueden incluir fragmentos.
setup-rpc-public-endpoint = El RPC público de Solana no admite sondeo continuo.
setup-rpc-private-host = Los endpoints RPC no pueden usar hosts locales ni de red privada.
setup-rpc-duplicate = Quita los endpoints RPC duplicados.
setup-rpc-ready =
    { $count ->
        [one] { $count } endpoint HTTPS listo para probar.
        [many] { $count } endpoints HTTPS listos para probar.
       *[other] { $count } endpoints HTTPS listos para probar.
    }

## Verification results

setup-wallet-verified = Billetera verificada
setup-wallet-unverified = No se pudo verificar la billetera
setup-wallet-address-detail = Dirección { $address }
setup-wallet-format-hint = Revisa el formato de la clave privada.
setup-rpc-none-working = Ningún RPC de mainnet funciona
setup-rpc-health-failed = Ningún endpoint superó las comprobaciones de estado de mainnet.
setup-rpc-partial = { $working } funcionan; { $failed } no disponibles
setup-rpc-verified =
    { $count ->
        [one] { $count } endpoint de mainnet verificado
        [many] { $count } endpoints de mainnet verificados
       *[other] { $count } endpoints de mainnet verificados
    }
setup-rpc-fastest = Más rápido: { $url } ({ $latency } ms).
setup-error-request-failed = La solicitud falló ({ $status })
setup-error-restart-timeout = { -brand } aún no se ha reconectado.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = Analizando la clave privada
setup-verify-wallet-parsing-detail = Comprobando la clave y derivando su dirección pública.
setup-verify-wallet-waiting = Esperando para validar
setup-verify-rpc-testing = Probando Solana mainnet
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Comprobando { $count } endpoint.
        [many] Comprobando { $count } endpoints.
       *[other] Comprobando { $count } endpoints.
    }
setup-verify-rpc-waiting = Esperando para probar los endpoints
setup-verify-save-waiting = Esperando para guardar
setup-verify-save-running = Cifrando y guardando
setup-verify-save-running-detail = Escribiendo la configuración verificada en este dispositivo.
setup-verify-save-done = Configuración guardada
setup-verify-save-done-detail = Clave privada cifrada; endpoints RPC funcionales almacenados.
setup-verify-save-failed = No se pudo guardar la configuración
setup-verify-save-skipped = No guardada. Corrige el problema de arriba y vuelve a verificar.
setup-verify-request-failed = La solicitud de verificación falló
setup-verify-summary-checking = Comprobando tu billetera y las conexiones a Solana mainnet.
setup-verify-summary-running = Verificando exactamente las credenciales que introdujiste.
setup-verify-summary-saving = Credenciales verificadas. Guardando de forma segura.
setup-verify-summary-failed = Revisa el problema y vuelve a verificar.

## Errors

setup-error-credentials-failed = La verificación de credenciales falló.
setup-error-save-failed = No se pudo guardar la configuración.
setup-error-verify-failed = La verificación falló.
setup-error-explore-failed = No se pudo iniciar el Modo Explorar.
setup-error-gateway-failed = No se pudo guardar el ajuste de envío de transacciones.
setup-action-review-credentials = Revisar credenciales

## Completion

setup-explore-opening = Abriendo el Modo Explorar…
setup-complete-restarting = Reiniciando { -brand } con tu configuración verificada.
setup-complete-finishing = Terminando el reinicio…
setup-complete-ready = { -brand } está listo. Abriendo el panel…
setup-complete-stored = Tu configuración verificada está guardada de forma segura en este dispositivo.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = Mostrar clave privada
setup-wallet-hide-key = Ocultar clave privada
setup-wallet-copy =
    .aria-label = Copiar dirección de la billetera
    .title = Copiar dirección de la billetera
setup-wallet-copy-done =
    .aria-label = Dirección de la billetera copiada
    .title = Copiado
setup-wallet-copy-failed =
    .aria-label = No se pudo copiar la dirección de la billetera
    .title = Falló la copia

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-close =
    .title = Cerrar
    .aria-label = Cerrar
setup-dialog-wallet-label = Clave privada de la billetera
setup-dialog-wallet-input =
    .placeholder = Cadena base58 o arreglo JSON [1,2,3,...]
setup-dialog-working = Trabajando…
setup-dialog-validating = Validando…
setup-dialog-saving = Guardando…
setup-dialog-restarting = Reiniciando…
setup-dialog-error-validation = La validación falló.
setup-dialog-error-incomplete = No se pudo completar la configuración.
setup-dialog-error-restart-helper = El asistente de reinicio automático no está disponible. Recarga el panel en unos momentos.
setup-dialog-error-unexpected = Error inesperado.
setup-dialog-replace-title = Reemplazar billetera de trading
setup-dialog-replace-subtitle = Opera con otra billetera. Su clave privada se cifra en este dispositivo y nunca sale de él.
setup-dialog-replace-submit = Validar y reemplazar
setup-dialog-replace-saved = Reemplazo preparado: reiniciando { -brand } con la nueva billetera…
setup-dialog-replace-confirm-intro = Al reemplazar la billetera de trading:
setup-dialog-replace-confirm-stop = Se detienen el trader y el copy trading. Vuelve a activarlos después del reinicio.
setup-dialog-replace-confirm-backup = Las posiciones, transacciones e historial de la billetera actual se mueven a una carpeta de copia de seguridad.
setup-dialog-replace-confirm-keep = La clave de la billetera actual se conserva en Otras billeteras, donde aún puedes exportarla.
setup-dialog-replace-confirm-funds = No se mueven fondos: los tokens y el { -sol } permanecen en la billetera actual.
setup-dialog-replace-confirm-rpc = Sustituye los endpoints RPC configurados por los que introduzcas; con el campo vacío, se mantienen los actuales.
setup-dialog-replace-confirm-restart = { -brand } se reinicia con la nueva billetera.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Progreso de la configuración
setup-wizard-wallet-security-note = Se cifra antes de guardarse.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Una URL HTTPS por línea
setup-wizard-rpc-guidance = Usa un proveedor fiable de mainnet; los endpoints públicos no son compatibles.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recomendado
setup-wizard-account-title = Cuenta de { -brand }
setup-wizard-account-optional = Opcional
setup-wizard-account-loading = Comprobando el estado de la cuenta…
setup-wizard-verify-title = Verificar y guardar
setup-wizard-verify-list =
    .aria-label = Estado de la verificación de la configuración
setup-wizard-verify-wallet = Billetera
setup-wizard-verify-save = Configuración segura
setup-wizard-complete-title = Configuración guardada
setup-wizard-reconnect = Reintentar conexión
setup-wizard-reload = Recargar el panel
setup-wizard-error-title = La configuración requiere atención
setup-wizard-explore = Explorar el panel
setup-wizard-continue = Continuar

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Wallet
setup-wizard-step-connection = Conexión
setup-wizard-step-verify = Verificar
setup-wizard-step-ready = Listo
setup-wallet-title = Configura tu wallet de trading
setup-wallet-description = La wallet con la que opera { -brand }. Su clave se cifra en este ordenador y nunca sale de él.
setup-wallet-source =
    .aria-label = Origen de la wallet
setup-wallet-create-title = Crear una wallet nueva
setup-wallet-create-detail = { -brand } crea una wallet nueva. Guardas su clave y luego le envías { -sol }.
setup-wallet-import-title = Importar una wallet
setup-wallet-import-detail = Usa una clave privada que ya tengas (base58 o un array JSON de 64 números).
setup-wallet-creating = Creando tu wallet…
setup-wallet-create-failed = No se pudo crear la wallet. Inténtalo de nuevo.
setup-backup-title = Guarda tu clave privada
setup-backup-description = Quien tenga esta clave controla la wallet. { -brand } no puede recuperarla.
setup-backup-address = Dirección
setup-backup-key = Clave privada
setup-backup-copy-address = Copiar dirección
setup-backup-download = Descargar
setup-backup-copied = Copiada. Vacía el portapapeles después de guardarla.
setup-backup-machine-warning = La copia cifrada que guarda { -brand } solo se abre en este ordenador. Si el ordenador se pierde o se sustituye, solo la clave que guardes ahora restaura la wallet. Guárdala sin conexión, por ejemplo en papel o en un gestor de contraseñas.
setup-backup-ack-saved = Guardé la clave privada en un lugar seguro, fuera de este ordenador.
setup-backup-ack-control = Entiendo que cualquiera con esta clave puede llevarse los fondos de esta wallet.
setup-backup-ack-recovery = Entiendo que { -brand } no puede recuperar esta clave. Sin ella, perder el ordenador significa perder los fondos.
setup-backup-create-another = Crear otra
setup-backup-replace-confirm = ¿Descartar esta clave y crear una wallet nueva? Hazlo solo si no le has enviado nada.
setup-connection-title = ¿Cómo debe conectarse { -brand } a { -solana }?
setup-connection-description = Puedes cambiarlo más tarde en Configuración → Conexión.
setup-connection-choice =
    .aria-label = Conexión
setup-connection-service-title = Servicio de { -brand }
setup-connection-service-badge = No necesita RPC
setup-connection-service-detail = Usa tu cuenta gratuita de { -brand }.
setup-connection-service-prices = Los precios se actualizan cada pocos segundos
setup-connection-service-features = El copy trading, la vigilancia de wallets, los saldos de otras wallets, el historial antiguo, los swaps directos en pools y los precios de pool en vivo necesitan tu propio RPC
setup-connection-service-pause = El trading se pausa si cierras sesión o si no se puede acceder a screenerbot.io
setup-connection-service-privacy = Tu clave privada nunca sale de este equipo: los swaps se firman aquí y screenerbot.io solo usa la dirección pública de tu billetera para leer sus saldos y transacciones
setup-connection-own-title = Tu propio RPC
setup-connection-own-badge = El más rápido
setup-connection-own-detail = Precios unas dos veces por segundo y todas las funciones. Requiere un endpoint RPC de un proveedor.
setup-connection-signin-required = Inicia sesión o crea una cuenta gratuita para usar el servicio de { -brand }.
setup-connection-reauthorize = Vuelve a iniciar sesión para permitir las consultas de la wallet del servicio de { -brand }.
setup-connection-sending = Enviar transacciones a través de { -brand }
setup-connection-sending-hint = Disponible con la sesión iniciada. Si no es posible, se usa tu RPC.
setup-wizard-account-required = Necesaria
setup-wizard-verify-connection = Conexión
setup-verify-service-checking = Comprobando el servicio de { -brand }
setup-verify-service-ready = Servicio de { -brand } listo
setup-verify-service-busy = El servicio de { -brand } está ocupado ahora mismo. Inténtalo de nuevo en un momento o usa tu propio RPC.
setup-verify-service-daily-limit = Tu cuenta alcanzó el límite de hoy del servicio de { -brand }. Inténtalo de nuevo después de las 00:00 UTC o usa tu propio RPC.
setup-verify-service-unreachable = No se puede acceder a screenerbot.io. Comprueba tu conexión a internet.
setup-verify-service-version = El servicio de { -brand } ya no admite esta versión. Actualiza { -brand } para continuar.
setup-verify-service-signin = Inicia sesión para usar el servicio de { -brand }.
setup-verify-save-done-service-detail = Clave privada cifrada; servicio de { -brand } seleccionado.
setup-action-try-again = Intentar de nuevo
setup-action-use-own-rpc = Usar mi propio RPC
setup-action-sign-in-again = Volver a iniciar sesión
setup-ready-title = { -brand } está listo
setup-fund-title = Añade fondos a tu wallet de trading
setup-fund-waiting = Esperando { -sol }…
setup-fund-received = Recibido { $amount } { -sol }
setup-fund-balance-unavailable = Leyendo el saldo… Se actualiza solo.
setup-fund-open-dashboard = Abrir el panel
