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
setup-error-restart-timeout = La configuración está guardada, pero { -brand } aún no se ha reconectado.

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
setup-verify-save-skipped = No guardada
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
setup-error-gateway-failed = No se pudo guardar la preferencia de gateway.
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

setup-dialog-title = Configurar billetera y RPC
setup-dialog-subtitle = Conecta tu billetera de Solana y un endpoint RPC premium para habilitar el trading y los datos en vivo de la cadena. Tu clave privada se cifra en este dispositivo y nunca sale de él.
setup-dialog-close =
    .title = Cerrar
    .aria-label = Cerrar
setup-dialog-wallet-label = Clave privada de la billetera
setup-dialog-wallet-input =
    .placeholder = Cadena base58 o arreglo JSON [1,2,3,...]
setup-dialog-rpc-label = Endpoint(s) RPC
setup-dialog-rpc-input =
    .placeholder = https://tu-endpoint... (uno por línea)
setup-dialog-rpc-hint = Se recomienda encarecidamente un proveedor premium (Helius, QuickNode, Alchemy): el RPC público de Solana tiene límite de velocidad y puede no funcionar.
setup-dialog-submit = Validar y conectar
setup-dialog-working = Trabajando…
setup-dialog-validating = Validando…
setup-dialog-saving = Guardando…
setup-dialog-restarting = Reiniciando…
setup-dialog-saved = Configuración guardada: reiniciando { -brand } en modo completo…
setup-dialog-error-missing-fields = Introduce la clave privada de una billetera y al menos una URL de RPC.
setup-dialog-error-validation = La validación falló.
setup-dialog-error-incomplete = No se pudo completar la configuración.
setup-dialog-error-restart-helper = El asistente de reinicio automático no está disponible. Recarga el panel en unos momentos.
setup-dialog-error-unexpected = Error inesperado.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Progreso de la configuración
setup-wizard-step-credentials = Credenciales
setup-wizard-step-verification = Verificación
setup-wizard-step-complete = Completado
setup-wizard-credentials-title = Configurar credenciales
setup-wizard-credentials-description = Conecta una billetera local y endpoints RPC de Solana mainnet fiables.
setup-wizard-wallet-toggle =
    .title = Mostrar clave privada
    .aria-label = Mostrar clave privada
setup-wizard-wallet-security-note = Se cifra antes de guardarse.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Una URL HTTPS por línea
setup-wizard-rpc-guidance = Se recomienda un RPC de mainnet fiable para el sondeo continuo.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recomendado
setup-wizard-gateway-title = Envío de transacciones gratis
setup-wizard-gateway-hint = Disponible al iniciar sesión. Tu RPC sigue disponible como respaldo.
setup-wizard-account-title = Cuenta de { -brand }
setup-wizard-account-optional = Opcional
setup-wizard-account-loading = Comprobando el estado de la cuenta…
setup-wizard-verify-title = Verificar y guardar
setup-wizard-verify-list =
    .aria-label = Estado de la verificación de la configuración
setup-wizard-verify-wallet = Billetera
setup-wizard-verify-rpc = RPC de Solana
setup-wizard-verify-save = Configuración segura
setup-wizard-complete-title = Configuración guardada
setup-wizard-reconnect = Reintentar conexión
setup-wizard-reload = Recargar el panel
setup-wizard-error-title = La configuración requiere atención
setup-wizard-explore = Explorar el panel
setup-wizard-continue = Continuar
