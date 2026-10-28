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
setup-dialog-rpc-optional-note = Opcional: deixe o campo vazio para manter os endpoints RPC já configurados.
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
setup-error-restart-timeout = O { -brand } ainda não reconectou.

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
setup-verify-save-skipped = Não salva. Corrija o problema acima e verifique novamente.
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
setup-error-gateway-failed = Não foi possível salvar a configuração de envio de transações.
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
setup-dialog-submit = Validar e conectar
setup-dialog-working = Processando…
setup-dialog-validating = Validando…
setup-dialog-saving = Salvando…
setup-dialog-restarting = Reiniciando…
setup-dialog-saved = Configuração salva: reiniciando o { -brand } em modo completo…
setup-dialog-error-validation = Falha na validação.
setup-dialog-error-incomplete = Não foi possível concluir a configuração.
setup-dialog-error-restart-helper = O auxiliar de reinicialização automática está indisponível. Recarregue o painel em instantes.
setup-dialog-error-unexpected = Erro inesperado.
setup-dialog-replace-title = Substituir carteira de trading
setup-dialog-replace-subtitle = Opere com outra carteira. A chave privada dela é criptografada neste dispositivo e nunca sai dele.
setup-dialog-replace-submit = Validar e substituir
setup-dialog-replace-saved = Substituição preparada: reiniciando o { -brand } com a nova carteira…
setup-dialog-replace-confirm-intro = Ao substituir a carteira de trading:
setup-dialog-replace-confirm-stop = O trader e o copy trading são parados. Ative-os novamente após a reinicialização.
setup-dialog-replace-confirm-backup = As posições, transações e o histórico da carteira atual são movidos para uma pasta de backup.
setup-dialog-replace-confirm-keep = A chave da carteira atual fica em Outras carteiras, onde você ainda pode exportá-la.
setup-dialog-replace-confirm-funds = Nenhum fundo é movido: os tokens e o { -sol } permanecem na carteira atual.
setup-dialog-replace-confirm-rpc = Substitui os endpoints RPC configurados pelos que você informar; com o campo vazio, os endpoints atuais permanecem.
setup-dialog-replace-confirm-restart = O { -brand } reinicia com a nova carteira.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = Progresso da configuração
setup-wizard-wallet-security-note = Criptografada antes de ser salva.
setup-wizard-rpc-title = Endpoints RPC
setup-wizard-rpc-input =
    .placeholder = Uma URL HTTPS por linha
setup-wizard-rpc-guidance = Use um provedor confiável da mainnet; endpoints públicos não são suportados.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = recomendado
setup-wizard-account-title = Conta { -brand }
setup-wizard-account-optional = Opcional
setup-wizard-account-loading = Verificando o status da conta…
setup-wizard-verify-title = Verificar e salvar
setup-wizard-verify-list =
    .aria-label = Status da verificação da configuração
setup-wizard-verify-wallet = Carteira
setup-wizard-verify-save = Configuração segura
setup-wizard-complete-title = Configuração salva
setup-wizard-reconnect = Tentar reconectar
setup-wizard-reload = Recarregar painel
setup-wizard-error-title = A configuração precisa de atenção
setup-wizard-explore = Explorar o painel
setup-wizard-continue = Continuar

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = Carteira
setup-wizard-step-connection = Conexão
setup-wizard-step-verify = Verificar
setup-wizard-step-ready = Pronto
setup-wallet-title = Configure sua carteira de trading
setup-wallet-description = A carteira com a qual o { -brand } opera. A chave dela é criptografada neste computador e nunca sai dele.
setup-wallet-source =
    .aria-label = Origem da carteira
setup-wallet-create-title = Criar uma carteira nova
setup-wallet-create-detail = O { -brand } cria uma carteira nova. Você salva a chave dela e depois envia { -sol } para ela.
setup-wallet-import-title = Importar uma carteira
setup-wallet-import-detail = Use uma chave privada que você já tem (base58 ou um array JSON de 64 números).
setup-wallet-creating = Criando sua carteira…
setup-wallet-create-failed = Não foi possível criar a carteira. Tente de novo.
setup-backup-title = Salve sua chave privada
setup-backup-description = Quem tiver esta chave controla a carteira. O { -brand } não consegue recuperá-la.
setup-backup-address = Endereço
setup-backup-key = Chave privada
setup-backup-copy-address = Copiar endereço
setup-backup-download = Baixar
setup-backup-copied = Copiado. Limpe a área de transferência depois de salvar.
setup-backup-machine-warning = A cópia criptografada que o { -brand } guarda só abre neste computador. Se o computador for perdido ou substituído, só a chave que você salvar agora restaura a carteira. Guarde-a offline, por exemplo em papel ou em um gerenciador de senhas.
setup-backup-ack-saved = Salvei a chave privada em um lugar seguro, fora deste computador.
setup-backup-ack-control = Entendo que qualquer pessoa com esta chave pode levar os fundos desta carteira.
setup-backup-ack-recovery = Entendo que o { -brand } não consegue recuperar esta chave. Sem ela, perder o computador significa perder os fundos.
setup-backup-create-another = Criar outra
setup-backup-replace-confirm = Descartar esta chave e criar uma carteira nova? Faça isso só se você ainda não enviou nada para ela.
setup-connection-title = Como o { -brand } deve se conectar à { -solana }?
setup-connection-description = Você pode mudar isso depois em Configurações → Conexão.
setup-connection-choice =
    .aria-label = Conexão
setup-connection-service-title = Serviço do { -brand }
setup-connection-service-badge = Sem necessidade de RPC
setup-connection-service-detail = Usa sua conta gratuita do { -brand }.
setup-connection-service-prices = Os preços são atualizados a cada poucos segundos
setup-connection-service-features = Copy trading, monitoramento de carteiras, saldos de outras carteiras, histórico antigo, swaps diretos em pools e preços de pool ao vivo precisam do seu próprio RPC
setup-connection-service-pause = O trading pausa se você sair da conta ou se screenerbot.io ficar inacessível
setup-connection-service-privacy = Sua chave privada nunca sai deste computador: os swaps são assinados aqui, e o screenerbot.io usa só o endereço público da sua carteira para ler os saldos e as transações
setup-connection-own-title = Seu próprio RPC
setup-connection-own-badge = Mais rápido
setup-connection-own-detail = Preços cerca de duas vezes por segundo e todos os recursos. Precisa de um endpoint RPC de um provedor.
setup-connection-signin-required = Entre ou crie uma conta gratuita para usar o serviço do { -brand }.
setup-connection-reauthorize = Entre de novo para permitir leituras da carteira pelo serviço do { -brand }.
setup-connection-sending = Enviar transações pelo { -brand }
setup-connection-sending-hint = Disponível quando você está conectado. Quando não for possível, seu RPC é usado.
setup-wizard-account-required = Obrigatória
setup-wizard-verify-connection = Conexão
setup-verify-service-checking = Verificando o serviço do { -brand }
setup-verify-service-ready = Serviço do { -brand } pronto
setup-verify-service-busy = O serviço do { -brand } está ocupado agora. Tente de novo em instantes ou use seu próprio RPC.
setup-verify-service-daily-limit = Sua conta atingiu o limite de hoje do serviço do { -brand }. Tente de novo depois das 00:00 UTC ou use seu próprio RPC.
setup-verify-service-unreachable = screenerbot.io está inacessível. Verifique sua conexão com a internet.
setup-verify-service-version = Esta versão não é mais suportada pelo serviço do { -brand }. Atualize o { -brand } para continuar.
setup-verify-service-signin = Entre para usar o serviço do { -brand }.
setup-verify-save-done-service-detail = Chave privada criptografada; serviço do { -brand } selecionado.
setup-action-try-again = Tentar de novo
setup-action-use-own-rpc = Usar meu próprio RPC
setup-action-sign-in-again = Entrar de novo
setup-ready-title = O { -brand } está pronto
setup-fund-title = Adicione fundos à sua carteira de trading
setup-fund-waiting = Aguardando { -sol }…
setup-fund-received = Recebido { $amount } { -sol }
setup-fund-balance-unavailable = Lendo o saldo… Ele se atualiza sozinho.
setup-fund-open-dashboard = Abrir o painel
