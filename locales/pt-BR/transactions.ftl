# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = Compra
transactions-type-sell = Venda
transactions-type-swap = Swap
transactions-type-sol-transfer = Transferência de SOL
transactions-type-token-transfer = Transferência de token
transactions-type-transfer = Transferência
transactions-type-dust = Dust
transactions-type-spam = Spam
transactions-type-ata-create = Conta aberta
transactions-type-ata-close = Rent recuperado
transactions-type-ata = Conta de token
transactions-type-liquidity-add = Adicionar liquidez
transactions-type-liquidity-remove = Remover liquidez
transactions-type-nft = NFT
transactions-type-program = Chamada de programa
transactions-type-compute = Computação
transactions-type-failed = Falhou
transactions-type-unknown = Não classificada

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = Airdrop de spam ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = Todos os tipos
transactions-filter-transfer = Transferências
transactions-filter-ata = Rent e contas
transactions-filter-liquidity = Liquidez
transactions-filter-program = Chamadas de programa

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = Entrada
transactions-direction-outgoing = Saída
transactions-direction-internal = Interna
transactions-direction-unknown = Não classificada

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = Pendente
transactions-status-confirmed = Confirmada
transactions-status-finalized = Finalizada
transactions-status-failed = Falhou
transactions-status-success = Sucesso
transactions-status-unknown = Desconhecido

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = Criação
transactions-ata-operation-closure = Fechamento

## Transactions page (pages/transactions.js)

transactions-toolbar-title = Histórico de transações
transactions-search =
    .placeholder = Buscar assinaturas…
    .aria-label = Buscar assinaturas de transações
transactions-load-failed = Não foi possível atualizar as transações
transactions-setup-gate-title = As transações exigem uma carteira
transactions-summary-total = Total
transactions-summary-estimate = Estimativa
transactions-summary-success = Sucesso
transactions-summary-failed = Falhas
transactions-filter-wallet = Carteira
transactions-filter-type = Tipo
transactions-filter-direction = Direção
transactions-filter-status = Status
transactions-filter-all-directions = Todas as direções
transactions-filter-all-statuses = Todos os status
transactions-wallet-main = Carteira principal
transactions-col-time = Hora
transactions-col-signature = Assinatura
transactions-col-type = Tipo
transactions-col-direction = Direção
transactions-col-status = Status
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = Taxas ({ -sol })
transactions-col-token = Token
transactions-col-router = Roteador
transactions-col-instructions = Instr.

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = Copiar assinatura
transactions-dialog-close =
    .title = Fechar (ESC)
transactions-dialog-tabs-label = Seções dos detalhes da transação
transactions-dialog-meta-slot = Slot:
transactions-dialog-meta-fee = Taxa:
transactions-dialog-loading = Carregando...
transactions-dialog-loading-details = Carregando detalhes da transação...
transactions-dialog-load-failed = Falha ao carregar os detalhes da transação
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = Falha ao carregar os detalhes da transação: { $reason }
transactions-dialog-not-found = Transação não encontrada
transactions-dialog-tab-overview = Visão geral
transactions-dialog-tab-balances = Saldos
transactions-dialog-tab-instructions = Instruções
transactions-dialog-tab-logs = Logs
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Bruto
transactions-dialog-unknown = Desconhecido
transactions-dialog-unknown-asset = Ativo desconhecido
transactions-dialog-unavailable = Indisponível

## Transaction details dialog: overview

transactions-dialog-failed-title = Transação falhou
transactions-dialog-no-program-error = Nenhum erro de programa foi informado.
transactions-dialog-story-title = O que aconteceu
# $router is the routing program name.
transactions-dialog-router-via = via { $router }
transactions-dialog-flow-paid = Pagou
transactions-dialog-flow-received = Recebeu
transactions-dialog-flow-from = De
transactions-dialog-flow-to = Para
transactions-dialog-flow-amount = Valor
transactions-dialog-net-wallet-change = Variação líquida da carteira:
transactions-dialog-processed = Processada na Solana
transactions-dialog-execution-title = Execução
transactions-dialog-metric-execution-price = Preço de execução
transactions-dialog-metric-effective-received = Recebido efetivo
transactions-dialog-metric-effective-spent = Gasto efetivo
transactions-dialog-metric-network-fee = Taxa de rede
transactions-dialog-metric-estimated-pnl = P&L estimado
transactions-dialog-metric-net-native-change = Variação líquida de { -sol }
transactions-dialog-route-title = Rota e ativos
transactions-dialog-route-router = Roteador
transactions-dialog-route-input-asset = Ativo de entrada
transactions-dialog-route-output-asset = Ativo de saída
transactions-dialog-route-pool = Pool
transactions-dialog-route-program = Programa
transactions-dialog-tech-title = Detalhes técnicos
transactions-dialog-tech-summary = Assinatura, slot e recursos
transactions-dialog-tech-signature = Assinatura
transactions-dialog-tech-timestamp = Data e hora
transactions-dialog-tech-slot = Slot
transactions-dialog-tech-exact-fee = Taxa exata
transactions-dialog-tech-accounts = Contas
transactions-dialog-tech-instructions = Instruções
transactions-dialog-tech-compute-units = Unidades de computação
transactions-dialog-tech-token-decimals = Decimais do token

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = Variações de saldo em { -sol }
transactions-dialog-balances-native-empty = Sem variações de saldo em { -sol }
transactions-dialog-balances-token-title = Variações de saldo de tokens
transactions-dialog-balances-token-empty = Sem variações de saldo de tokens
transactions-dialog-balances-net-native = Variação líquida de { -sol }
transactions-dialog-balances-fee = Taxa da transação
transactions-dialog-col-account = Conta
transactions-dialog-col-token = Token
transactions-dialog-col-pre-balance = Saldo anterior
transactions-dialog-col-post-balance = Saldo posterior
transactions-dialog-col-change = Variação
transactions-dialog-col-type = Tipo
transactions-dialog-col-rent = Rent ({ -sol })
transactions-dialog-instructions-empty = Nenhuma instrução encontrada
transactions-dialog-instructions-count =
    { $count ->
        [one] { $count } instrução
        [many] { $count } instruções
       *[other] { $count } instruções
    }
transactions-dialog-instruction-program-id = ID do programa
transactions-dialog-instruction-accounts = Contas ({ $count })
transactions-dialog-instruction-data = Dados
transactions-dialog-logs-empty = Nenhum log disponível
transactions-dialog-logs-filter = Filtrar logs...
transactions-dialog-logs-no-match = Nenhum log correspondente
transactions-dialog-logs-count =
    { $count ->
        [one] { $count } log
        [many] { $count } logs
       *[other] { $count } logs
    }
transactions-dialog-ata-empty = Nenhuma operação de ATA nesta transação
transactions-dialog-ata-summary-title = Resumo da análise de ATA
transactions-dialog-ata-creations = Criações
transactions-dialog-ata-closures = Fechamentos
transactions-dialog-ata-rent-spent = Rent gasto
transactions-dialog-ata-rent-recovered = Rent recuperado
transactions-dialog-ata-net-rent = Impacto líquido do rent
transactions-dialog-ata-operations-title = Operações de ATA ({ $count })
transactions-dialog-raw-copy = Copiar JSON
transactions-dialog-raw-empty = Nenhum dado bruto disponível

# Empty table (scripts/pages/transactions.js)
transactions-empty = Nenhuma transação ainda
    .message = Swaps e transferências da carteira de trading aparecem aqui assim que são confirmados on-chain.
