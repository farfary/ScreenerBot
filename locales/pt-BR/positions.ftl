# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = Posição criada


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = Abertas
positions-status-closed = Fechadas
positions-status-archived = Arquivadas
positions-origin-copy = Cópia
positions-origin-manual = Manual
positions-origin-wallet = Carteira
positions-origin-copy-link =
    .title = Abrir a tarefa de cópia que abriu esta posição
positions-holding-frozen = Congelado
    .title = A autoridade do mint congelou esta conta de token: o saldo não pode ser transferido nem vendido
positions-toolbar-total = Total
positions-toolbar-delete-all = Excluir tudo
positions-search-placeholder = Buscar por símbolo ou mint...
positions-filter-origin = Origem
positions-filter-origin-all = Todas as origens
positions-filter-origin-auto = Trader automático
positions-filter-origin-copy = Copy trading
positions-delete-all-tooltip = Excluir permanentemente todas as posições arquivadas

## Columns

positions-column-token = Token
positions-column-archived-at = Arquivada
positions-column-entry-time = Hora de entrada
positions-column-exit-time = Hora de saída
positions-column-avg-entry = Entrada méd. ({ -sol })
positions-column-avg-exit = Saída méd. ({ -sol })
positions-column-current-price = Atual ({ -sol })
positions-column-total-invested = Total investido
positions-column-proceeds = Retorno
positions-column-pnl = P&L
positions-column-pnl-percent = P&L %
positions-column-size = Tamanho
positions-column-dca = DCA
positions-column-exits = Saídas
positions-column-unrealized-pnl = P&L não realizado
positions-column-unrealized-percent = Não realizado %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = Sem custo base no histórico desta carteira (airdrop, execução cotada em USD ou swap sem perna em SOL)
positions-unknown-history = Esta rodada não bate com o saldo on-chain
positions-dca-count =
    { $count ->
        [one] { $count } DCA
        [many] { $count } DCAs
       *[other] { $count } DCAs
    }
positions-exit-count =
    { $count ->
        [one] { $count } saída
        [many] { $count } saídas
       *[other] { $count } saídas
    }

## Row actions

positions-action-add =
    .title = Fazer aporte na posição (DCA)
    .aria-label = Fazer aporte na posição
positions-action-sell =
    .title = Vender (total ou parcial em %)
    .aria-label = Vender posição
positions-action-sell-frozen = Congelada pela autoridade do mint: esta posse não pode ser vendida
positions-action-remove =
    .title = Remover (arquivar ou excluir)
    .aria-label = Remover posição
positions-action-restore =
    .title = Restaurar para Abertas/Fechadas
    .aria-label = Restaurar posição
positions-action-delete =
    .title = Excluir permanentemente
    .aria-label = Excluir permanentemente
positions-action-in-progress = Em andamento…

## Live state of a row

positions-caption-buying = Comprando
# $step is the label of the current action step.
positions-caption-buying-step = Comprando · { $step }
positions-caption-selling = Vendendo
positions-caption-selling-step = Vendendo · { $step }
positions-caption-closing = Fechando
positions-caption-failed = Falhou
# $error is the failure text of the action.
positions-caption-failed-detail = Falhou · { $error }
positions-step-adding = Aportando
positions-pending-buying = Comprando…
positions-pending-buy-failed = Falha na compra

## Messages and confirmations

positions-load-failed = Não foi possível atualizar as posições
positions-toast-not-found = Dados da posição não encontrados
positions-toast-deleted = Posição excluída
positions-toast-archived = Posição arquivada
positions-toast-restored = Posição restaurada
positions-action-failed = Falha na ação
positions-delete-title = Excluir posição permanentemente
# $symbol is the token symbol.
positions-delete-message = Excluir { $symbol } permanentemente? Isso remove a posição e o histórico dela do banco de dados e não pode ser desfeito. Suas transações e os dados do token não são afetados.
positions-delete-confirm = Excluir permanentemente
positions-delete-all-title = Excluir todas as posições arquivadas
positions-delete-all-message =
    { $count ->
        [one] Excluir permanentemente { $count } posição arquivada? Isso não pode ser desfeito. Transações e dados de tokens não são afetados.
        [many] Excluir permanentemente todas as { $count } posições arquivadas? Isso não pode ser desfeito. Transações e dados de tokens não são afetados.
       *[other] Excluir permanentemente todas as { $count } posições arquivadas? Isso não pode ser desfeito. Transações e dados de tokens não são afetados.
    }
positions-delete-all-message-empty = Excluir permanentemente todas as posições arquivadas? Isso não pode ser desfeito.
positions-delete-all-confirm = Excluir tudo
positions-delete-all-done =
    { $count ->
        [one] { $count } posição arquivada excluída
        [many] { $count } posições arquivadas excluídas
       *[other] { $count } posições arquivadas excluídas
    }
positions-delete-all-failed = Falha ao excluir as posições arquivadas

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = Remover posição
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>Esta posição ainda está aberta.</strong> O bot está segurando este token. Remover libera a vaga de trade e para o acompanhamento, mas <strong>não</strong> vende. Venda antes se quiser receber seus { -sol } de volta.
positions-remove-modes =
    .aria-label = Modo de remoção
positions-remove-archive = Arquivar
positions-remove-recommended = Recomendado
positions-remove-archive-description = Move para a aba Arquivadas. Reversível a qualquer momento: nada é vendido e todos os trades continuam registrados.
positions-remove-delete = Excluir permanentemente
positions-remove-delete-description = Apaga esta posição e todo o histórico dela do banco de dados.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = Isso remove permanentemente a posição e o histórico dela. <strong>Isso não pode ser desfeito.</strong> Suas transações e os dados do token não são afetados.
positions-remove-confirm-archive = Arquivar posição

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = Gestão da posição definida como { $mode }
positions-details-load-failed = Falha ao carregar os detalhes da posição
positions-details-mint-label = Endereço do mint
positions-details-management-failed = Falha ao atualizar a gestão da posição
positions-details-favorite-add =
    .title = Adicionar aos favoritos
    .aria-label = Adicionar aos favoritos
positions-details-favorite-remove =
    .title = Remover dos favoritos
    .aria-label = Remover dos favoritos
positions-details-view-solscan =
    .title = Ver no { -solscan }
    .aria-label = Ver token no { -solscan }
positions-details-close =
    .title = Fechar (Esc)
    .aria-label = Fechar
positions-details-chart-section =
    .aria-label = Gráfico de preço
positions-details-loading-chart = Carregando gráfico...
positions-details-activity-section =
    .aria-label = Atividade
positions-details-activity-title = Atividade
positions-details-split-handle =
    .aria-label = Redimensionar gráfico e atividade
positions-details-activity-pane =
    .aria-label = Painel de atividade
positions-details-activity-expand =
    .title = Expandir atividade
    .aria-label = Expandir atividade
positions-details-summary-section =
    .aria-label = Resumo da posição
positions-details-loading = Carregando posição...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = Trader automático
positions-management-user-only = Somente usuário
positions-management-copy-task = Tarefa de cópia
positions-management-hybrid = Híbrida
positions-pane-show-chart = Mostrar gráfico
positions-pane-show-activity = Mostrar atividade
positions-pane-restore-activity = Restaurar atividade
positions-pane-expand-chart =
    .title = Expandir gráfico
    .aria-label = Expandir gráfico

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = Risco baixo
positions-risk-medium = Risco médio
positions-risk-high = Risco alto
positions-risk-unknown = Risco desconhecido
positions-busy-buying = Compra em andamento…
positions-busy-selling = Venda em andamento…
positions-busy-closing = Fechamento em andamento…
positions-header-avg-entry = Entrada méd.
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
        [one] { $count } compra
        [many] { $count } compras
       *[other] { $count } compras
    }
positions-header-exit-price = Preço de saída
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = fechada { $ago }
positions-header-realized-pnl = P&L realizado
positions-header-usd-note = USD pela cotação atual do { -sol }
positions-header-returned = Retornado
# $amount is the formatted SOL amount invested.
positions-header-of-invested = de { $amount } investidos
positions-header-price = Preço
positions-header-last-price = Último preço
positions-header-pool-ago = pool · { $ago }
positions-header-unrealized-pnl = P&L não realizado
positions-header-pnl-last-price = P&L no último preço
positions-header-value = Valor
positions-header-last-value = Último valor
positions-header-invested = { $amount } investidos
positions-header-origin-hint = Como esta posição foi aberta
positions-header-risk-hint = Pontuação do { -rugcheck }: quanto menor, mais seguro
positions-header-frozen = Congelado
    .title = A autoridade do mint congelou esta posse
positions-header-managed-by = Gerenciada por
positions-header-management-select =
    .aria-label = Gestão da posição

## Entry origin shown in the header badge

positions-origin-unknown = desconhecida
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = Copiada · tarefa { $task }
positions-origin-manual-entry = Entrada manual
positions-origin-wallet-entry = Entrada pela carteira
# $strategy is the strategy id.
positions-origin-auto-strategy = Auto · { $strategy }
positions-origin-auto-entry = Entrada automática

## Swaps that are submitted and not yet booked

positions-pending-adding = Aportando
positions-pending-adding-amount = Aportando { $amount }
positions-pending-selling = Vendendo
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = Vendendo { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · confirmando
    .title = Enviado e aguardando confirmação on-chain. Os valores são atualizados assim que for verificado.

## Trade controls

positions-trade-add = Aportar
    .title = Fazer aporte na posição
positions-trade-sell = Vender
    .title = Vender parte da posição
positions-trade-close = Fechar posição
    .title = Vender tudo e fechar
positions-trade-token = Detalhes do token
    .title = Abrir detalhes do token

## Favorites

positions-favorite-token-fallback = Token
# $symbol is the token symbol.
positions-favorite-added = { $symbol } adicionado aos favoritos
positions-favorite-removed = { $symbol } removido dos favoritos
positions-favorite-add-failed = Falha ao adicionar favorito
positions-favorite-remove-failed = Falha ao remover favorito
positions-favorite-update-failed = Falha ao atualizar favoritos

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = Posição
positions-summary-price-path = Trajetória do preço
positions-summary-network-fees = Taxas de rede
positions-summary-risk = Risco
positions-summary-market = Mercado
positions-summary-market-now = Mercado agora
positions-summary-links = Links
positions-fact-tokens-fallback = tokens
positions-fact-bought = Comprado
positions-fact-holding = Em posse
positions-fact-sold = Vendido
positions-fact-realized = Realizado
positions-fact-opened = Abertura
positions-fact-closed = Fechamento
positions-fact-reason = Motivo
positions-fact-archived = Arquivada
positions-fact-entry = Entrada
positions-fact-exit = Saída
positions-fact-total = Total
positions-fact-verified = Verificado on-chain
positions-fact-confirming = Confirmando
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = { $percent } do comprado
positions-fact-share-of-invested = { $percent } do investido
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 entrada
        [one] 1 entrada + { $count } aporte
        [many] 1 entrada + { $count } aportes
       *[other] 1 entrada + { $count } aportes
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
        [one] { $count } saída parcial · { $returned } de volta
        [many] { $count } saídas parciais · { $returned } de volta
       *[other] { $count } saídas parciais · { $returned } de volta
    }
# $age is the elapsed time of the hold.
positions-fact-held = mantida por { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = { $percent } vs entrada
positions-fact-exit-vs-peak = Saída vs pico
positions-fact-now-vs-peak = Agora vs pico
positions-fact-entry-range = Faixa de entrada
positions-range-low = Mínima
positions-range-peak = Pico
positions-range-now = Agora
positions-range-label-exit = Preço de entrada e de saída entre a mínima e o pico
positions-range-label-now = Preço de entrada e atual entre a mínima e o pico
positions-fact-mint-authority = Autoridade de mint
positions-fact-freeze-authority = Autoridade de congelamento
positions-fact-active = Ativa
positions-fact-pool = Pool
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = { $amount } { -sol } de liquidez
positions-fact-market-cap = Market cap
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = Liquidez
positions-fact-volume-24h = Volume 24h
positions-fact-price-change = Variação de preço
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = Holders
positions-link-website = Site
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = Não foi possível carregar a atividade
positions-activity-loading = Carregando atividade...
positions-activity-empty = Ainda não aconteceu nada com este token nesta carteira
positions-activity-filter-empty = Nenhuma atividade corresponde a este filtro
positions-activity-round-count =
    { $count ->
        [one] { $count } rodada
        [many] { $count } rodadas
       *[other] { $count } rodadas
    }
positions-activity-event-count =
    { $count ->
        [one] { $count } evento
        [many] { $count } eventos
       *[other] { $count } eventos
    }
positions-activity-pending-count = { $count } pendentes
positions-activity-failed-count = { $count } com falha
positions-filter-all = Tudo
positions-filter-trades = Trades
positions-filter-buys = Compras
positions-filter-sells = Vendas
positions-filter-wallet = Carteira
positions-filter-issues = Problemas
positions-activity-filters =
    .aria-label = Filtrar atividade
positions-activity-totals =
    .aria-label = Todas as rodadas deste token
positions-activity-realized-all = Realizado, todas as rodadas
positions-activity-invested = Investido
positions-activity-returned = Retornado
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = Aberta { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = Posição { $index }
positions-activity-this-position = Esta posição
positions-activity-dates-unavailable = Datas indisponíveis
positions-activity-wallet-title = Transações da carteira
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
        [one] Fora de qualquer posição · { $range } · { $count } evento
        [many] Fora de qualquer posição · { $range } · { $count } eventos
       *[other] Fora de qualquer posição · { $range } · { $count } eventos
    }
positions-details-signature-label = Assinatura

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = Posição aberta
positions-state-closing = Posição fechando
positions-state-closed = Posição fechada
positions-state-exit-pending = Saída da posição pendente
positions-state-exit-failed = Falha na saída da posição
positions-state-phantom = Posição fantasma
positions-state-reconciling = Posição em conciliação

## Activity events

positions-event-kind-entry = Entrada
positions-event-kind-dca = Aporte
positions-event-kind-partial-exit = Saída parcial
positions-event-kind-exit = Saída
positions-event-kind-buy = Compra da carteira
positions-event-kind-sell = Venda da carteira
positions-event-kind-transfer = Transferência
positions-event-kind-ata = Conta de token
positions-event-kind-other = Transação
positions-event-state-pending = Pendente
positions-event-state-failed = Falhou
positions-event-state-synthetic = Sintético
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = Falhou: { $error }
positions-event-tokens-fallback = tokens
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = Compra enviada de { $amount }
positions-event-entry-for = Comprou { $amount } por { $sol }
positions-event-entry = Comprou { $amount }
positions-event-dca-submitted = Aporte enviado de { $amount }
positions-event-dca-for = Aportou { $amount } por { $sol }
positions-event-dca = Aportou { $amount }
positions-event-partial-exit-submitted-percent = Saída parcial de { $percent } enviada para { $amount }
positions-event-partial-exit-submitted = Saída parcial enviada para { $amount }
positions-event-sold-percent-for = Vendeu { $amount } ({ $percent }) por { $sol }
positions-event-sold-percent = Vendeu { $amount } ({ $percent })
positions-event-sold-for = Vendeu { $amount } por { $sol }
positions-event-sold = Vendeu { $amount }
positions-event-exit-submitted = Saída total da posição enviada
positions-event-exit-for = Fechada com { $amount } vendidos por { $sol }
positions-event-exit-closed = Posição fechada
positions-event-wallet-bought = A carteira comprou { $amount } em outro lugar
positions-event-wallet-sold = A carteira vendeu { $amount } em outro lugar
positions-event-received = Recebeu { $amount }
positions-event-sent = Enviou { $amount }
positions-event-transferred = Transferiu { $amount }
positions-event-ata = Atividade da conta de token
positions-event-wallet-transaction = Transação da carteira envolvendo { $amount }
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / token
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = { $amount } de variação na carteira
positions-event-after-title = Posição após este evento
positions-event-capital-invested = Capital investido
positions-event-average-entry = Entrada média
positions-event-transfers-title = Transferências de token
positions-event-transfer-amount = Valor
positions-event-transfer-mint = Mint
positions-event-transfer-from = De
positions-event-transfer-to = Para
positions-event-no-signature = Sem assinatura on-chain
positions-event-click-to-copy = Clique para copiar
positions-event-solscan = { -solscan }
positions-event-token-amount = Quantidade de tokens
positions-event-trade-price = Preço do trade
positions-event-sol-amount = Valor em { -sol }
positions-event-cost-basis = Custo base
positions-event-usd-value = Valor em USD
positions-event-network-fee = Taxa de rede
positions-event-router = Roteador
positions-event-slot = Slot
positions-event-chain-status = Status na chain
positions-event-transaction-type = Tipo de transação
positions-event-direction = Direção
positions-event-wallet-sol-change = Variação de { -sol } da carteira
positions-event-instructions = Instruções
positions-event-compute-units = Unidades de computação
positions-event-accounts = Contas
positions-event-record-id = ID do registro
positions-event-time-unavailable = Horário indisponível
positions-event-details = Detalhes
positions-event-hide-details = Ocultar detalhes

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = Candles
positions-chart-type-line = Linha
positions-chart-type-area = Área
positions-chart-type-group =
    .aria-label = Tipo de gráfico
positions-chart-overlays-group =
    .aria-label = Sobreposições do gráfico
positions-chart-ema = EMA
    .title = Médias móveis exponenciais, 9 e 21
positions-chart-fit = Ajustar
    .title = Enquadrar todo o ciclo de vida desta posição
positions-chart-timeframes-group =
    .aria-label = Timeframe
positions-chart-pane-group =
    .aria-label = Painel do gráfico
positions-chart-unavailable = Motor de gráficos indisponível
positions-chart-collecting = Coletando dados do gráfico…
positions-chart-no-data = Ainda não há dados de gráfico para este token
positions-chart-avg-entry = Entrada méd.
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = Entrada méd.
positions-chart-legend-avg-entry-off-scale = Entrada méd. (fora da escala)
positions-chart-dropped-events =
    { $count ->
        [one] { $count } evento sem candle neste timeframe
        [many] { $count } eventos sem candle neste timeframe
       *[other] { $count } eventos sem candle neste timeframe
    }
positions-chart-level = Nível
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } está acima desta visão
positions-chart-level-below = { $label } { $price } está abaixo desta visão
positions-chart-scale-hint = Arraste o eixo de preço para ampliar a escala até ele
positions-chart-pnl-at-bar = P&L @ barra
positions-chart-click-to-locate = Clique para localizar
