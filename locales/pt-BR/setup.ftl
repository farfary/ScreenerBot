# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = Informe a chave privada da carteira.
setup-wallet-json-recognized = Formato de chave JSON de 64 bytes reconhecido.
setup-wallet-json-invalid = Use um array JSON com exatamente 64 valores de byte (0–255).
setup-wallet-format-invalid = Use uma chave privada em base58 ou um array JSON de 64 bytes.
setup-wallet-base58-recognized = Formato de chave base58 reconhecido.

## RPC endpoint validation

setup-rpc-required = Informe pelo menos um endpoint RPC.
setup-rpc-too-many = Use no máximo 10 endpoints RPC.
setup-rpc-url-invalid = Todo endpoint deve ser uma URL HTTPS válida.
setup-rpc-url-credentials = As URLs de RPC não podem conter usuário ou senha.
setup-rpc-url-fragment = As URLs de RPC não podem conter fragmentos.
setup-rpc-public-endpoint = O RPC público da Solana não comporta consultas periódicas contínuas.
setup-rpc-private-host = Os endpoints RPC não podem usar hosts locais ou de rede privada.
setup-rpc-duplicate = Remova os endpoints RPC duplicados.
setup-rpc-ready =
    { $count ->
        [one] { $count } endpoint HTTPS pronto para teste.
        [many] { $count } endpoints HTTPS prontos para teste.
       *[other] { $count } endpoints HTTPS prontos para teste.
    }

## Verification results

setup-wallet-verified = Carteira verificada
setup-wallet-unverified = Não foi possível verificar a carteira
setup-wallet-address-detail = Endereço { $address }
setup-wallet-format-hint = Verifique o formato da chave privada.
setup-rpc-none-working = Nenhum RPC da mainnet funcionando
setup-rpc-health-failed = Nenhum endpoint passou nas verificações de saúde da mainnet.
setup-rpc-partial = { $working } funcionando; { $failed } indisponíveis
setup-rpc-verified =
    { $count ->
        [one] { $count } endpoint da mainnet verificado
        [many] { $count } endpoints da mainnet verificados
       *[other] { $count } endpoints da mainnet verificados
    }
setup-rpc-fastest = Mais rápido: { $url } ({ $latency } ms).
setup-error-request-failed = Falha na requisição ({ $status })
setup-error-restart-timeout = A configuração foi salva, mas o { -brand } ainda não reconectou.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = Interpretando a chave privada
setup-verify-wallet-parsing-detail = Verificando a chave e derivando o endereço público.
setup-verify-wallet-waiting = Aguardando validação
setup-verify-rpc-testing = Testando a mainnet da Solana
setup-verify-rpc-testing-detail =
    { $count ->
        [one] Verificando { $count } endpoint.
        [many] Verificando { $count } endpoints.
       *[other] Verificando { $count } endpoints.
    }
setup-verify-rpc-waiting = Aguardando teste dos endpoints
setup-verify-save-waiting = Aguardando para salvar
setup-verify-save-running = Criptografando e salvando
setup-verify-save-running-detail = Gravando a configuração verificada neste dispositivo.
setup-verify-save-done = Configuração salva
setup-verify-save-done-detail = Chave privada criptografada; endpoints RPC funcionais armazenados.
setup-verify-save-failed = Não foi possível salvar a configuração
setup-verify-save-skipped = Não salva
setup-verify-request-failed = Falha na requisição de verificação
setup-verify-summary-checking = Verificando sua carteira e as conexões com a mainnet da Solana.
setup-verify-summary-running = Verificando exatamente as credenciais que você informou.
setup-verify-summary-saving = Credenciais verificadas. Salvando com segurança.
setup-verify-summary-failed = Revise o problema e verifique novamente.

## Errors

setup-error-credentials-failed = Falha na verificação das credenciais.
setup-error-save-failed = Não foi possível salvar a configuração.
setup-error-verify-failed = Falha na verificação.
setup-error-explore-failed = Não foi possível iniciar o Modo Explorar.
setup-error-gateway-failed = Não foi possível salvar a preferência de gateway.
setup-action-review-credentials = Revisar credenciais

## Completion

setup-explore-opening = Abrindo o Modo Explorar…
setup-complete-restarting = Reiniciando o { -brand } com a sua configuração verificada.
setup-complete-finishing = Concluindo a reinicialização…
setup-complete-ready = O { -brand } está pronto. Abrindo o painel…
setup-complete-stored = Sua configuração verificada está armazenada com segurança neste dispositivo.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = Mostrar chave privada
setup-wallet-hide-key = Ocultar chave privada
setup-wallet-copy =
    .aria-label = Copiar endereço da carteira
    .title = Copiar endereço da carteira
setup-wallet-copy-done =
    .aria-label = Endereço da carteira copiado
    .title = Copiado
setup-wallet-copy-failed =
    .aria-label = Não foi possível copiar o endereço da carteira
    .title = Falha ao copiar

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = Configurar carteira e RPC
setup-dialog-subtitle = Conecte sua carteira Solana e um endpoint RPC premium para habilitar o trading e os dados on-chain ao vivo. Sua chave privada é criptografada neste dispositivo e nunca sai dele.
setup-dialog-close =
    .title = Fechar
    .aria-label = Fechar
setup-dialog-wallet-label = Chave privada da carteira
setup-dialog-wallet-input =
    .placeholder = Texto em base58 ou array JSON [1,2,3,...]
setup-dialog-rpc-label = Endpoint(s) RPC
setup-dialog-rpc-input =
    .placeholder = https://seu-endpoint... (um por linha)
setup-dialog-rpc-hint = Recomendamos fortemente um provedor premium (Helius, QuickNode, Alchemy): o RPC público da Solana tem limite de requisições e pode não funcionar.
setup-dialog-submit = Validar e conectar
setup-dialog-working = Processando…
setup-dialog-validating = Validando…
setup-dialog-saving = Salvando…
setup-dialog-restarting = Reiniciando…
setup-dialog-saved = Configuração salva: reiniciando o { -brand } em modo completo…
setup-dialog-error-missing-fields = Informe a chave privada da carteira e pelo menos uma URL de RPC.
setup-dialog-error-validation = Falha na validação.
setup-dialog-error-incomplete = Não foi possível concluir a configuração.
setup-dialog-error-restart-helper = O auxiliar de reinicialização automática está indisponível. Recarregue o painel em instantes.
setup-dialog-error-unexpected = Erro inesperado.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Progresso da configuração
setup-wizard-step-credentials = Credenciais
setup-wizard-step-verification = Verificação
setup-wizard-step-complete = Concluído
setup-wizard-credentials-title = Configurar credenciais
setup-wizard-credentials-description = Conecte uma carteira local e endpoints RPC confiáveis da mainnet da Solana.
setup-wizard-wallet-toggle =
    .title = Mostrar chave privada
    .aria-label = Mostrar chave privada
setup-wizard-wallet-security-note = Criptografada antes de ser salva.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Uma URL HTTPS por linha
setup-wizard-rpc-guidance = Recomendamos um RPC confiável da mainnet para consultas periódicas contínuas.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recomendado
setup-wizard-gateway-title = Envio de transações gratuito
setup-wizard-gateway-hint = Disponível ao entrar na conta. Seu RPC continua disponível como reserva.
setup-wizard-account-title = Conta { -brand }
setup-wizard-account-optional = Opcional
setup-wizard-account-loading = Verificando o status da conta…
setup-wizard-verify-title = Verificar e salvar
setup-wizard-verify-list =
    .aria-label = Status da verificação da configuração
setup-wizard-verify-wallet = Carteira
setup-wizard-verify-rpc = RPC da Solana
setup-wizard-verify-save = Configuração segura
setup-wizard-complete-title = Configuração salva
setup-wizard-reconnect = Tentar reconectar
setup-wizard-reload = Recarregar painel
setup-wizard-error-title = A configuração precisa de atenção
setup-wizard-explore = Explorar o painel
setup-wizard-continue = Continuar
