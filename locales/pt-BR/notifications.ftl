notifications-action-swap-buy = Compra
notifications-action-swap-sell = Venda
notifications-action-position-open = Abertura
notifications-action-position-close = Fechamento
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = Saída parcial
notifications-action-manual-order = Manual
notifications-action-unknown = Ação

actions-step-evaluate = Avaliando
actions-step-validate = Validando
actions-step-quote = Obtendo cotação
actions-step-swap = Executando swap
actions-step-verify = Verificando
actions-step-unknown = Processando
actions-step-evaluate-short = Avaliando
actions-step-validate-short = Checando
actions-step-quote-short = Cotação
actions-step-swap-short = Swap
actions-step-verify-short = Confirmando
actions-step-unknown-short = Em andamento

actions-failure-recorded = { $message }
actions-failure-unknown = Erro desconhecido
actions-failure-interrupted = Interrompido pela reinicialização do aplicativo
actions-failure-validation = Falha na validação
actions-failure-quote = Falha na cotação
actions-failure-swap = Falha no swap
actions-failure-trade = Falha no trade
actions-failure-entry = Falha na entrada
actions-failure-entry-withdrawn = Entrada retirada
actions-failure-exit = Falha na saída
actions-failure-dca = Falha no DCA
actions-failure-verification-expired = Verificação expirada: a transação nunca foi confirmada
actions-failure-verification-gave-up = Verificação abandonada
actions-failure-transaction-failed = A transação falhou na blockchain
actions-failure-sell-transaction-failed = A transação de venda falhou na blockchain
actions-failure-dca-verification-failed = Falha na verificação do DCA

notifications-empty-all = Nenhuma ação
notifications-empty-active = Nenhuma ação ativa
notifications-empty-completed = Nenhuma ação concluída
notifications-empty-failed = Nenhuma ação com falha
notifications-source-auto = Auto
notifications-source-manual = Manual
notifications-cancelled = Cancelada
notifications-load-failed = Falha ao carregar
notifications-clear-title = Limpar notificações
notifications-clear-message = Dispensar todas as notificações desta lista? Elas continuam no histórico de concluídas/com falha.
notifications-clear-failed = Falha ao limpar as notificações
notifications-stream-lag-title = O fluxo de ações ficou atrasado
notifications-stream-lag-missed =
    { $count ->
        [one] { $count } atualização perdida
        [many] { $count } atualizações perdidas
       *[other] { $count } atualizações perdidas
    } — atualizando
notifications-stream-lag-refreshing = Atualizando
notifications-sync-failed = Não foi possível atualizar as ações

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = Hoje
notifications-day-yesterday = Ontem
notifications-source-copy = Copy trading
notifications-steps = Etapas
notifications-detail-transaction = Transação
notifications-detail-fill-price = Preço de execução
    .title = Valor em { -sol } deste swap dividido pela quantidade de tokens
notifications-detail-fee = Taxa de rede
notifications-detail-pnl = P&L da posição
notifications-detail-route = Rota
notifications-detail-duration = Duração
notifications-detail-reason = Motivo
notifications-view-token = Ver token
notifications-view-position = Ver posição
notifications-view-transaction = Ver transação
notifications-unrecorded = Não registrado na posição
notifications-unrecorded-detail = Este swap não foi registrado na posição e pode não ter sido executado. Verifique a transação.
