# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = Ferramentas
tools-category-wallet = Carteira
tools-category-token = Token
tools-category-single-token = Token único
tools-category-utilities = Utilitários
tools-sidebar-hint = Selecione uma ferramenta para começar
tools-placeholder-title = Selecione uma ferramenta
tools-placeholder-subtitle = Escolha uma ferramenta na barra lateral para começar
tools-placeholder-hint-wallets = As ferramentas de carteira ajudam a gerenciar suas carteiras Solana
tools-placeholder-hint-secure = Todas as operações são protegidas e reversíveis sempre que possível

tools-status-ready = Pronta para uso
tools-status-coming = Em breve
tools-status-beta = Beta - pode ter bugs
tools-status-disabled = Desativada no momento
tools-status-badge-coming = Em breve
tools-status-badge-beta = Beta
tools-toast-coming-soon = Esta ferramenta chegará em breve
tools-toast-disabled = Esta ferramenta está desativada no momento
tools-setup-gate-title = Esta ferramenta exige uma carteira

## Tool names.

tools-tool-wallet-cleanup-title = Limpeza de carteira
tools-tool-wallet-cleanup-summary = Fechar ATAs vazias
tools-tool-wallet-cleanup-description = Feche Associated Token Accounts vazias para recuperar { -sol }
tools-tool-burn-tokens-title = Queimar tokens
tools-tool-burn-tokens-summary = Destruir tokens permanentemente
tools-tool-burn-tokens-description = Destrua tokens da sua carteira de forma permanente
tools-tool-token-analyzer-title = Analisador de tokens
tools-tool-token-analyzer-summary = Análise profunda de tokens
tools-tool-token-analyzer-description = Análise profunda de qualquer token Solana, com insights multidimensionais
tools-tool-create-token-title = Criar token
tools-tool-create-token-summary = Publicar novo token SPL
tools-tool-create-token-description = Publique um novo token SPL na Solana
tools-tool-trade-watcher-title = Monitor de Trades
tools-tool-trade-watcher-summary = Monitore trades e aja automaticamente
tools-tool-trade-watcher-description = Monitore os trades de um token e dispare ações automáticas de compra/venda
tools-tool-token-watch-title = Monitor de Holders
tools-tool-token-watch-summary = Acompanhe novos holders do token
tools-tool-token-watch-description = Acompanhe e monitore novos holders de um token em tempo real
tools-tool-buy-multi-wallets-title = Compra múltipla
tools-tool-buy-multi-wallets-summary = Coordene compras entre carteiras
tools-tool-buy-multi-wallets-description = Execute ordens de compra coordenadas em várias carteiras, com valores aleatórios
tools-tool-sell-multi-wallets-title = Venda múltipla
tools-tool-sell-multi-wallets-summary = Coordene vendas entre carteiras
tools-tool-sell-multi-wallets-description = Execute ordens de venda coordenadas em várias carteiras, com consolidação de { -sol }
tools-tool-wallet-consolidation-title = Consolidação de carteiras
tools-tool-wallet-consolidation-nav-title = Consolidação
tools-tool-wallet-consolidation-summary = Consolide os fundos das carteiras
tools-tool-wallet-consolidation-description = Consolide { -sol } e tokens das subcarteiras de volta na carteira principal
tools-tool-airdrop-checker-title = Verificador de airdrops
tools-tool-airdrop-checker-summary = Verifique airdrops pendentes
tools-tool-airdrop-checker-description = Verifique airdrops pendentes e recompensas disponíveis para resgate
tools-tool-wallet-generator-title = Gerador de carteiras
tools-tool-wallet-generator-summary = Gere novos pares de chaves
tools-tool-wallet-generator-description = Gere novos pares de chaves Solana com segurança

## Shared by the tools

tools-validation-mint-required = Informe o endereço do mint do token
tools-validation-mint-format = Formato de endereço do mint inválido
tools-validation-mint-invalid = Informe um endereço de mint válido

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = Detalhes do token
tools-create-token-name-label = Nome do token
tools-create-token-name-input =
    .placeholder = Meu token
tools-create-token-symbol-label = Símbolo
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = Decimais
tools-create-token-supply-label = Supply inicial
tools-create-token-description-label = Descrição
tools-create-token-description-input =
    .placeholder = Descrição do token...
tools-create-token-image-title = Imagem do token
tools-create-token-image-drop = Solte a imagem aqui ou clique para enviar
tools-create-token-image-hint = Recomendado: PNG 512x512
tools-create-token-action-preview = Prévia
tools-create-token-action-create = Criar token

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = Carregando configurações...
tools-holder-watch-saved = Configurações do Monitor de Holders salvas
tools-holder-watch-save-failed = Falha ao salvar as configurações
tools-holder-watch-save-error = Erro ao salvar as configurações
tools-holder-watch-settings-title = Configurações do Monitor de Holders
tools-holder-watch-enabled-label = Ativar monitoramento de holders
tools-holder-watch-interval-label = Intervalo de verificação
tools-holder-watch-interval-hint = Com que frequência verificar o número de holders (10-3600s)
tools-holder-watch-max-tokens-label = Máx. de tokens monitorados
tools-holder-watch-max-tokens-hint = Máximo de tokens monitorados simultaneamente
tools-holder-watch-notify-new-label = Notificar novos holders
tools-holder-watch-notify-drop-label = Notificar queda de holders
tools-holder-watch-min-change-label = Variação mín. de holders
tools-holder-watch-min-change-hint = Variação mínima de holders para disparar a notificação
tools-holder-watch-drop-percent-label = Limite de queda de holders
tools-holder-watch-drop-percent-hint = Queda percentual que dispara o alerta
tools-holder-watch-action-save = Salvar configurações
tools-holder-watch-tokens-title = Tokens monitorados
tools-holder-watch-token-input =
    .placeholder = Informe o endereço do mint do token...
tools-holder-watch-empty = Nenhum token sendo monitorado
tools-holder-watch-empty-hint = Adicione um endereço de mint acima para começar a monitorar
tools-holder-watch-coming-soon = O monitoramento de tokens chegará em breve

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = Analisar token
tools-analyzer-mint-input =
    .placeholder = Cole o endereço do mint do token...
tools-analyzer-action-analyze = Analisar
tools-analyzer-action-analyzing = Analisando...
tools-analyzer-action-copy-report = Copiar relatório
tools-analyzer-loading = Analisando token...
tools-analyzer-failed = Falha ao analisar o token
tools-analyzer-empty = Informe o endereço do mint de um token para analisar
tools-analyzer-empty-hint = Obtenha insights completos sobre qualquer token Solana
tools-analyzer-tab-overview = Visão geral
tools-analyzer-tab-security = Segurança
tools-analyzer-tab-market = Mercado
tools-analyzer-tab-liquidity = Liquidez
tools-analyzer-unknown-token = Token desconhecido

tools-analyzer-favorite-add =
    .title = Adicionar aos favoritos
    .aria-label = Adicionar aos favoritos
tools-analyzer-favorite-already = Já está nos favoritos
tools-analyzer-favorite-added = { $symbol } adicionado aos favoritos
tools-analyzer-favorite-failed = Falha ao adicionar aos favoritos
tools-analyzer-blacklist-add =
    .title = Adicionar à lista negra
    .aria-label = Adicionar à lista negra
tools-analyzer-blacklist-title = Adicionar token à lista negra
tools-analyzer-blacklist-message = Adicionar { $symbol } à lista negra? Este token será excluído das negociações.
tools-analyzer-blacklist-confirm = Adicionar à lista negra
tools-analyzer-blacklisted = Na lista negra
tools-analyzer-blacklist-done = { $symbol } adicionado à lista negra
tools-analyzer-blacklist-failed = Falha ao adicionar o token à lista negra

tools-analyzer-card-quick-stats = Resumo rápido
tools-analyzer-card-market-summary = Resumo do mercado
tools-analyzer-card-token-info = Informações do token
tools-analyzer-stat-holders = Holders
tools-analyzer-stat-decimals = Decimais
tools-analyzer-stat-safety-score = Pontuação de segurança
tools-analyzer-stat-pools = Pools
tools-analyzer-stat-volume-24h = Volume 24h
tools-analyzer-stat-change-24h = Variação 24h
tools-analyzer-stat-market-cap = Market cap
tools-analyzer-stat-liquidity = Liquidez
tools-analyzer-info-mint = Endereço do mint
tools-analyzer-info-description = Descrição
tools-analyzer-info-supply = Supply

tools-analyzer-security-empty = Nenhum dado de segurança disponível
tools-analyzer-security-empty-hint = A análise de segurança não está disponível para este token
tools-analyzer-card-safety-score = Pontuação de segurança
tools-analyzer-score-good = Boa
tools-analyzer-score-moderate = Moderada
tools-analyzer-score-risky = Arriscada
tools-analyzer-raw-score = Pontuação de risco bruta: { $score }
tools-analyzer-card-authorities = Autoridades do token
tools-analyzer-authority-mint = Autoridade de mint
tools-analyzer-authority-freeze = Autoridade de congelamento
tools-analyzer-authority-transfer-fee = Taxa de transferência
tools-analyzer-authority-mutable = Mutável
tools-analyzer-authority-active = Ativa
tools-analyzer-authority-revoked = Revogada
tools-analyzer-card-holder-concentration = Concentração de holders
tools-analyzer-top-holders = em poder dos 10 maiores holders
tools-analyzer-risks-title = Riscos de segurança ({ $count })
tools-analyzer-risks-title-none = Riscos de segurança
tools-analyzer-risks-none = Nenhum risco de segurança detectado

tools-analyzer-market-empty = Nenhum dado de mercado disponível
tools-analyzer-market-empty-hint = Os dados de mercado não estão disponíveis para este token
tools-analyzer-card-price = Preço atual
tools-analyzer-card-price-changes = Variações de preço
tools-analyzer-card-volume = Volume de negociação
tools-analyzer-card-transactions = Transações em 24h
tools-analyzer-card-valuation = Valuation
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = Volume 1h
tools-analyzer-stat-volume-6h = Volume 6h
tools-analyzer-stat-fdv = Valor totalmente diluído
tools-analyzer-txn-buys = Compras
tools-analyzer-txn-sells = Vendas

tools-analyzer-liquidity-empty = Nenhum dado de liquidez disponível
tools-analyzer-liquidity-empty-hint = Nenhum pool encontrado para este token
tools-analyzer-card-total-liquidity = Liquidez total
tools-analyzer-card-pools = Pools
tools-analyzer-active-pools =
    { $count ->
        [one] Pool ativo
        [many] Pools ativos
       *[other] Pools ativos
    }
tools-analyzer-card-pool-details = Detalhes do pool
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = Liquidez ({ -sol })
tools-analyzer-pools-column-status = Status
tools-analyzer-pool-primary = Principal

tools-analyzer-report-empty = Nenhuma análise para copiar
tools-analyzer-report-label = Relatório de análise
tools-analyzer-report-title = Relatório de análise do token
tools-analyzer-report-token = Token: { $symbol } ({ $name })
tools-analyzer-report-mint = Mint: { $mint }
tools-analyzer-report-price = Preço: { $sol }
tools-analyzer-report-price-with-usd = Preço: { $sol } ({ $usd })
tools-analyzer-report-security = Segurança:
tools-analyzer-report-safety-score = - Pontuação de segurança: { $score }/100
tools-analyzer-report-mint-authority = - Autoridade de mint: { $state }
tools-analyzer-report-freeze-authority = - Autoridade de congelamento: { $state }
tools-analyzer-report-risks = - Riscos: { $count }
tools-analyzer-report-market = Mercado:
tools-analyzer-report-volume = - Volume 24h: { $amount }
tools-analyzer-report-change = - Variação 24h: { $amount }
tools-analyzer-report-market-cap = - Market cap: { $amount }
tools-analyzer-report-liquidity = Liquidez:
tools-analyzer-report-liquidity-total = - Total: { $amount }
tools-analyzer-report-pools = - Pools: { $count }
tools-analyzer-report-generated = Gerado em: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = Comprar na venda
tools-watch-type-sell-on-buy = Vender na compra
tools-watch-type-notify = Notificar
tools-watch-type-notify-only = Somente notificar

tools-trade-watcher-setup-title = Configurar monitoramento
tools-trade-watcher-mint-label = Endereço do mint do token
tools-trade-watcher-mint-input =
    .placeholder = Informe o endereço do mint do token...
tools-trade-watcher-action-search-pools = Buscar pools
tools-trade-watcher-pool-label = Pool selecionado
tools-trade-watcher-pool-none = Nenhum pool selecionado
tools-trade-watcher-pool-clear =
    .title = Limpar pool
tools-trade-watcher-pool-selected = Pool selecionado: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = Tipo de monitoramento
tools-trade-watcher-type-hint = Comprar na venda: compra automaticamente quando alguém vende. Vender na compra: vende automaticamente quando alguém compra.
tools-trade-watcher-trigger-label = Valor de gatilho
tools-trade-watcher-trigger-hint = Tamanho mínimo do trade em { -sol } para disparar a ação
tools-trade-watcher-action-amount-label = Valor da ação
tools-trade-watcher-action-amount-hint = Valor a comprar/vender quando disparado
tools-trade-watcher-slippage-label = Slippage
tools-trade-watcher-slippage-hint = Slippage máximo aceitável nos trades
tools-trade-watcher-active-title = Monitoramentos ativos
tools-trade-watcher-empty = Nenhum monitoramento ativo
tools-trade-watcher-empty-hint = Configure um monitoramento acima e clique em "Iniciar monitoramento" para começar
tools-trade-watcher-action-start = Iniciar monitoramento
tools-trade-watcher-action-starting = Iniciando...
tools-trade-watcher-action-stop-all = Parar todos
tools-trade-watcher-action-stopping = Parando...
tools-trade-watcher-started = Monitoramento iniciado para { $token }...
tools-trade-watcher-start-failed = Falha ao iniciar o monitoramento
tools-trade-watcher-stopped = Monitoramento parado
tools-trade-watcher-stop-failed = Falha ao parar o monitoramento
tools-trade-watcher-stopped-all = Todos os monitoramentos foram parados
tools-trade-watcher-stop-all-failed = Falha ao parar os monitoramentos
tools-trade-watcher-load-failed = Falha ao carregar os monitoramentos
tools-trade-watcher-column-token = Token
tools-trade-watcher-column-type = Tipo
tools-trade-watcher-column-trigger = Gatilho ({ -sol })
tools-trade-watcher-column-action = Ação ({ -sol })
tools-trade-watcher-column-triggered = Disparos
tools-trade-watcher-stop-watch =
    .title = Parar monitoramento

## Results returned by the tools backend.

tools-burn-failure-native-asset = Não é possível queimar { -sol }
tools-burn-failure-open-position = Não é possível queimar tokens de posições abertas
tools-burn-failure-account-not-found = Conta de token não encontrada
tools-burn-failure-zero-balance = O saldo do token já é zero
tools-burn-failure-transaction = Falha na transação
tools-burn-warning-open-position = Não é possível queimar tokens de posições abertas
tools-burn-warning-closed-position = Resquício de posição fechada
tools-burn-warning-worth = Vale ~{ $amount } { -sol }
tools-multi-buy-warning-insufficient = Saldo insuficiente. Necessário: { $needed } { -sol }; disponível: { $have } { -sol }
tools-multi-buy-warning-over-limit = O total de { -sol } necessário ({ $needed }) excede o limite ({ $limit })
tools-multi-sell-warning-no-wallets = Nenhuma carteira secundária encontrada
tools-multi-sell-warning-no-balance = Nenhuma carteira tem saldo do token
tools-multi-op-buy-failed = Falha na compra
tools-multi-op-sell-failed = Falha na venda
tools-multi-op-transfer-failed = Falha na transferência
tools-multi-op-balance-failed = Falha ao obter o saldo
tools-multi-op-mint-invalid = Endereço do mint inválido
tools-multi-buy-session-failed = Falha na compra múltipla
tools-multi-sell-session-failed = Falha na venda múltipla
tools-multi-session-aborted = Operação abortada pelo usuário

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = Escanear carteira
tools-wallet-action-scanning = Escaneando...
tools-wallet-scan-failed = Falha no escaneamento: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
        [one] Selecionada: { $count } carteira
        [many] Selecionadas: { $count } carteiras
       *[other] Selecionadas: { $count } carteiras
    }
tools-wallet-transfer-failed = Falha na transferência: { $reason }
tools-wallet-cleanup-failed = Falha na limpeza: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = Resultados do escaneamento
tools-wallet-cleanup-stat-empty = ATAs vazias
tools-wallet-cleanup-stat-reclaimable = { -sol } recuperável
tools-wallet-cleanup-stat-failed = Com falha (em cache)
tools-wallet-cleanup-prompt = Clique em "Escanear carteira" para encontrar ATAs vazias
tools-wallet-cleanup-prompt-hint = Isso verificará todas as contas de token da sua carteira
tools-wallet-cleanup-action-cleanup = Limpar tudo
tools-wallet-cleanup-action-cleaning = Limpando...
tools-wallet-cleanup-scanning = Escaneando carteira...
tools-wallet-cleanup-found =
    { $count ->
        [one] { $count } ATA vazia encontrada, no valor de ~{ $amount }
        [many] { $count } ATAs vazias encontradas, no valor de ~{ $amount }
       *[other] { $count } ATAs vazias encontradas, no valor de ~{ $amount }
    }
tools-wallet-cleanup-clean = Nenhuma ATA vazia encontrada - a carteira está limpa!
tools-wallet-cleanup-scan-failed = Falha ao escanear as ATAs
tools-wallet-cleanup-done =
    { $count ->
        [one] { $count } ATA limpa
        [many] { $count } ATAs limpas
       *[other] { $count } ATAs limpas
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = Queimar tokens
tools-burn-info-title = O que é queimar?
tools-burn-info-body = Queimar destrói tokens de forma permanente, tornando-os irrecuperáveis. Depois de queimar, execute a Limpeza de carteira para fechar ATAs vazias e recuperar ~0.002 { -sol } de rent por token.
tools-burn-stat-total = Total de tokens
tools-burn-stat-selected = Selecionados
tools-burn-stat-rent = Rent recuperável
tools-burn-prompt = Clique em "Escanear carteira" para encontrar tokens
tools-burn-scanning = Escaneando tokens da carteira...
tools-burn-scan-failed = Falha ao escanear os tokens
tools-burn-empty = Nenhum token encontrado na carteira
tools-burn-action-burn = Queimar selecionados ({ $count })
tools-burn-action-burning = Queimando...
tools-burn-cannot-burn = Não pode ser queimado
tools-burn-no-value = Sem valor

tools-burn-category-open-position = Posições abertas
tools-burn-category-has-value = Com valor
tools-burn-category-closed-position = Posições fechadas
tools-burn-category-zero-liquidity = Liquidez zero
tools-burn-category-hint-open-position = Não é possível queimar tokens de posições abertas
tools-burn-category-hint-has-value = Considere vender em vez de queimar
tools-burn-category-hint-closed-position = Resquícios de trades encerrados
tools-burn-category-hint-zero-liquidity = Seguro para queimar - sem valor de mercado

tools-burn-confirm-title = Confirmar queima
tools-burn-confirm-message =
    { $count ->
        [one] Tem certeza de que deseja queimar <strong>{ $count }</strong> token?
        [many] Tem certeza de que deseja queimar <strong>{ $count }</strong> tokens?
       *[other] Tem certeza de que deseja queimar <strong>{ $count }</strong> tokens?
    }
tools-burn-confirm-value = Valor total estimado: <strong>{ $amount }</strong>
tools-burn-confirm-continue = Continuar
tools-burn-final-title = Aviso final
tools-burn-final-headline = Esta ação é IRREVERSÍVEL!
tools-burn-final-message =
    { $count ->
        [one] O seguinte { $count } token será destruído permanentemente e não poderá ser recuperado em nenhuma circunstância.
        [many] Os seguintes { $count } tokens serão destruídos permanentemente e não poderão ser recuperados em nenhuma circunstância.
       *[other] Os seguintes { $count } tokens serão destruídos permanentemente e não poderão ser recuperados em nenhuma circunstância.
    }
tools-burn-final-confirm = Sim, queimar tokens
tools-burn-toast-burned =
    { $total ->
        [one] { $successful }/{ $total } token queimado. Execute a Limpeza de carteira para recuperar ~{ $amount }
        [many] { $successful }/{ $total } tokens queimados. Execute a Limpeza de carteira para recuperar ~{ $amount }
       *[other] { $successful }/{ $total } tokens queimados. Execute a Limpeza de carteira para recuperar ~{ $amount }
    }
tools-burn-toast-failed =
    { $count ->
        [one] { $count } token falhou ao ser queimado
        [many] { $count } tokens falharam ao ser queimados
       *[other] { $count } tokens falharam ao ser queimados
    }
tools-burn-failed = Falha ao queimar: { $reason }
tools-burn-failures-title =
    { $count ->
        [one] { $count } token não pôde ser queimado
        [many] { $count } tokens não puderam ser queimados
       *[other] { $count } tokens não puderam ser queimados
    }
tools-burn-failure-unknown = Nenhum motivo foi informado

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = Sobre
tools-airdrop-about-body = Verifique airdrops pendentes, recompensas disponíveis para resgate e alocações não resgatadas nos principais protocolos Solana.
tools-airdrop-list-title = Airdrops disponíveis
tools-airdrop-prompt = Clique em "Verificar airdrops" para buscar resgates disponíveis
tools-airdrop-action-check = Verificar airdrops
tools-airdrop-action-claim-all = Resgatar tudo

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = Opções do gerador
tools-generator-warning-title = Guarde suas chaves privadas com segurança!
tools-generator-warning-body = Os pares de chaves são criados localmente e nunca são transmitidos. Sempre faça backup das suas chaves em um local seguro.
tools-generator-count-label = Número de carteiras
tools-generator-vanity-label = Endereço personalizado (começa com caracteres específicos)
tools-generator-prefix-label = Prefixo
tools-generator-prefix-input =
    .placeholder = ex.: SOL
tools-generator-prefix-hint = Prefixos mais longos levam exponencialmente mais tempo para gerar
tools-generator-list-title = Carteiras geradas
tools-generator-empty = Nenhuma carteira gerada ainda
tools-generator-action-generate = Gerar
tools-generator-action-generating = Gerando...
tools-generator-count-invalid = Informe um número entre 1 e 10
tools-generator-no-keypairs = Nenhum par de chaves retornado
tools-generator-generated =
    { $count ->
        [one] { $count } carteira gerada
        [many] { $count } carteiras geradas
       *[other] { $count } carteiras geradas
    }
tools-generator-failed = Falha ao gerar as carteiras: { $reason }
tools-generator-copy-public-key =
    .title = Copiar chave pública
tools-generator-copy-private-key =
    .title = Copiar chave privada
tools-generator-remove =
    .title = Remover da lista
tools-generator-reveal =
    .title = Revelar chave privada
tools-generator-public-key-label = Chave pública:
tools-generator-private-key-label = Chave privada:
tools-generator-public-key-name = Chave pública
tools-generator-private-key-copied = Chave privada copiada
tools-generator-private-key-warning = Quem tiver esta chave controla a carteira
tools-generator-export-empty = Nenhuma carteira para exportar
tools-generator-exported = Carteiras exportadas - guarde com segurança

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = Resumo
tools-consolidation-stat-wallets = Subcarteiras
tools-consolidation-stat-native = Total de { -sol }
tools-consolidation-stat-tokens = Tipos de token
tools-consolidation-stat-rent = Rent recuperável
tools-consolidation-wallets-title = Carteiras
tools-consolidation-loading-wallets = Carregando carteiras...
tools-consolidation-loading-data = Carregando dados da carteira...
tools-consolidation-action-transfer-native = Transferir { -sol }
tools-consolidation-action-transfer-tokens = Transferir todos os tokens
tools-consolidation-action-cleanup = Limpar ATAs
tools-consolidation-action-transferring = Transferindo...
tools-consolidation-column-name = Nome
tools-consolidation-column-native = Saldo ({ -sol })
tools-consolidation-column-tokens = Tokens
tools-consolidation-column-atas = ATAs vazias
tools-consolidation-empty = Nenhuma subcarteira encontrada
tools-consolidation-empty-hint = Crie subcarteiras usando a Compra múltipla para começar
tools-consolidation-load-failed = Falha ao carregar: { $reason }
tools-consolidation-select-prompt = Selecione as carteiras para consolidar
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
        [one] { $tokens } token
        [many] { $tokens } tokens
       *[other] { $tokens } tokens
    } | { $atas ->
        [one] { $atas } ATA vazia
        [many] { $atas } ATAs vazias
       *[other] { $atas } ATAs vazias
    }
tools-consolidation-transferred-native = { $amount } transferidos para a carteira principal
tools-consolidation-transferred-tokens =
    { $count ->
        [one] { $count } token transferido para a carteira principal
        [many] { $count } tokens transferidos para a carteira principal
       *[other] { $count } tokens transferidos para a carteira principal
    }
tools-consolidation-cleaned =
    { $count ->
        [one] { $count } ATA fechada, { $amount } recuperados
        [many] { $count } ATAs fechadas, { $amount } recuperados
       *[other] { $count } ATAs fechadas, { $amount } recuperados
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = Token
tools-multi-mint-label = Endereço do mint do token
tools-multi-mint-input =
    .placeholder = Cole o endereço do mint do token...
tools-multi-execution-title = Configurações de execução
tools-multi-delay-min-label = Atraso mín.
tools-unit-native = { -sol }
tools-unit-seconds = s
tools-unit-ms = ms
tools-multi-delay-max-label = Atraso máx.
tools-multi-concurrency-label = Concorrência
tools-multi-concurrency-sequential = { $count } (sequencial)
tools-multi-concurrency-parallel = { $count } em paralelo
tools-multi-slippage-label = Slippage
tools-multi-router-label = Roteador
tools-multi-router-auto = Auto (melhor rota)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = Pool direto
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = Progresso
tools-multi-progress-preparing = Preparando...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = Carteira
tools-multi-column-route = Rota
tools-multi-column-status = Status
tools-multi-op-completed = Concluída
tools-multi-op-failed = Falhou
tools-multi-action-stop = Parar
tools-multi-action-loading = Carregando...
tools-multi-start-failed = Falha ao iniciar: { $reason }

tools-multi-state-pending = Pendente
tools-multi-state-funding = Financiando
tools-multi-state-executing = Executando
tools-multi-state-consolidating = Consolidando
tools-multi-state-completed = Concluída
tools-multi-state-failed = Falhou
tools-multi-state-aborted = Abortada

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = O token que você quer comprar em várias carteiras
tools-multi-buy-wallets-title = Configurações das carteiras
tools-multi-buy-wallet-count-label = Quantidade de carteiras
tools-multi-buy-wallet-count-option =
    { $count ->
        [one] { $count } carteira
        [many] { $count } carteiras
       *[other] { $count } carteiras
    }
tools-multi-buy-wallet-count-hint = Número de subcarteiras a usar
tools-multi-buy-buffer-label = Reserva de { -sol } por carteira
tools-multi-buy-buffer-hint = Reservado para taxas (mín. 0.015 { -sol })
tools-multi-buy-amounts-title = Configurações de valores
tools-multi-buy-min-label = Mín. de { -sol } por carteira
tools-multi-buy-min-hint = Valor mínimo de compra
tools-multi-buy-max-label = Máx. de { -sol } por carteira
tools-multi-buy-max-hint = Valor máximo de compra
tools-multi-buy-limit-label = Limite total de { -sol } (opcional)
tools-multi-buy-limit-hint = Gasto total máximo
tools-multi-buy-preview-title = Prévia
tools-multi-buy-preview-create = Carteiras a criar
tools-multi-buy-preview-amount = Valor por carteira
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = Total de { -sol } necessário
tools-multi-buy-preview-balance = Saldo principal
tools-multi-buy-action-preview = Prévia
tools-multi-buy-action-start = Iniciar compra múltipla
tools-multi-buy-executing = Executando compras...
tools-multi-buy-column-spent = Gasto ({ -sol })
tools-multi-buy-column-tokens = Tokens
tools-multi-buy-preview-failed = Falha na prévia: { $reason }
tools-multi-buy-started = Compra múltipla iniciada
tools-multi-buy-stopped = Compra múltipla parada
tools-multi-buy-completed = Compra múltipla concluída! { $successful }/{ $total } com sucesso

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = Informe o endereço de um token para buscar as carteiras que o possuem
tools-multi-sell-action-scan = Escanear
tools-multi-sell-settings-title = Configurações de venda
tools-multi-sell-percent-label = Porcentagem de venda
tools-multi-sell-percent-hint = % dos tokens a vender por carteira
tools-multi-sell-min-fee-label = Mín. de { -sol } para taxa
tools-multi-sell-min-fee-hint = Mínimo de { -sol } necessário para a taxa da transação
tools-multi-sell-topup-label = Recarregar automaticamente se necessário
tools-multi-sell-topup-hint = Transfere { -sol } da carteira principal se a subcarteira tiver saldo insuficiente
tools-multi-sell-post-title = Ações após a venda
tools-multi-sell-consolidate-label = Consolidar { -sol } na carteira principal
tools-multi-sell-consolidate-hint = Transfere todo o { -sol } das subcarteiras de volta para a carteira principal
tools-multi-sell-close-atas-label = Fechar ATAs do token após a venda
tools-multi-sell-close-atas-hint = Recupere ~0.002 { -sol } por ATA
tools-multi-sell-wallets-title = Carteiras com o token
tools-multi-sell-empty = Nenhuma subcarteira possui este token
tools-multi-sell-column-tokens = Tokens
tools-multi-sell-column-native = Saldo ({ -sol })
tools-multi-sell-column-topup = Precisa de recarga
tools-multi-sell-none-selected = Nenhuma carteira selecionada
tools-multi-sell-select-required = Selecione pelo menos uma carteira
tools-multi-sell-action-start = Iniciar venda múltipla
tools-multi-sell-executing = Executando vendas...
tools-multi-sell-column-sold = Tokens vendidos
tools-multi-sell-column-received = Recebido ({ -sol })
tools-multi-sell-started = Venda múltipla iniciada
tools-multi-sell-stopped = Venda múltipla parada
tools-multi-sell-completed = Venda múltipla concluída! { $amount } recebidos

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = Favoritos
tools-favorites-saved = Favoritos salvos
tools-favorites-save-current = Salvar atual
tools-favorites-empty = Nenhum favorito salvo ainda
tools-favorites-no-label = Sem rótulo
tools-favorites-uses = { $count }x
tools-favorites-remove = Remover
tools-favorites-loaded = Favorito carregado: { $name }
tools-favorites-default-name = Config
tools-favorites-mint-required = Informe primeiro o endereço do mint do token
tools-favorites-add-title = Adicionar favorito
tools-favorites-add-message = Informe um rótulo para este favorito
tools-favorites-add-placeholder = Rótulo (opcional)...
tools-favorites-saved-toast = Salvo nos favoritos
tools-favorites-save-failed = Falha ao salvar o favorito
tools-favorites-remove-title = Remover favorito
tools-favorites-remove-message = Remover este favorito?
tools-favorites-removed-toast = Favorito removido
tools-favorites-remove-failed = Falha ao remover o favorito
