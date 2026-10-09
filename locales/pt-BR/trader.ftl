# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Meta de ROI
trader-exit-type-roi-exit = Meta de ROI
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Substituição por tempo
trader-exit-type-time-rule = Regra de tempo
trader-exit-type-manual = Manual
trader-exit-type-manual-close = Manual
trader-exit-type-dca = DCA
trader-exit-type-unknown = Desconhecido

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = Estatísticas
trader-tab-strategy-control = Controle de estratégias
trader-tab-strategies = Estratégias
trader-tab-stop-loss = Stop loss
trader-tab-trailing-stop = Trailing stop
trader-tab-roi = Take profit
trader-tab-time-rules = Regras de tempo
trader-tab-dca = DCA
trader-tab-settings = Configurações

## Feature status badges and their messages

trader-feature-coming-soon = Em breve
    .message = Este recurso chegará em breve e ainda não está disponível.
trader-feature-beta = Beta
trader-feature-disabled = Desativado
    .message = Este recurso está desativado no momento.

## Status bar and trading controls

trader-status-title = Trader automático
trader-status-loading = Carregando...
trader-status-running = Em execução
trader-status-stopped = Parado
trader-status-setup-required = Configuração necessária
trader-status-unavailable = Conclua a configuração de carteira e RPC para usar o Trader automático
trader-toggle-on = ATIVADO
trader-toggle-off = DESATIVADO
trader-toggle-unavailable = INDISPONÍVEL
trader-toggle-start-failed = Falha ao iniciar o trader
trader-toggle-stop-failed = Falha ao parar o trader
trader-controls-title = Controles de trading
trader-halt-title = TRADING INTERROMPIDO
trader-halt-reason-default = Parada forçada manual
trader-halt-resume = Retomar
trader-monitor-entry = Monitor de entrada
trader-monitor-exit = Monitor de saída
trader-monitor-master-off = Trader automático desativado
trader-loss-limit-title = Limite de perda do período
trader-loss-limit-resume = Retomar trading
trader-loss-limit-reset = Reiniciar período
trader-loss-limit-off = Desativado
trader-loss-limit-none = Nenhum limite de perda do período configurado
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = Reinicia em { $hours } { $minutes }
trader-loss-limit-reached = LIMITE ATINGIDO
trader-force-stop = Forçar parada de tudo

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = Forçar parada do trading
    .message = Isso interrompe imediatamente TODAS as operações de trading. Continuar?
    .confirm = Parar trading
trader-loss-limit-resume-confirm = Retomar após o limite de perda
    .message = O limite de perda do período bloqueou novas entradas. Retomar permite que o trader abra posições de novo antes do período reiniciar. Continuar?
trader-loss-limit-reset-confirm = Reiniciar período do limite de perda
    .message = Isso zera a perda acumulada do período atual e inicia um novo. Continuar?

## Toasts

trader-toast-control-failed = Falha no controle do Trader automático
trader-toast-force-stop-on = Parada forçada ativada
trader-toast-force-stop-failed = Não foi possível ativar a parada forçada
trader-toast-force-stop-cleared = Parada forçada desfeita
trader-toast-resume-failed = Não foi possível retomar o trading
trader-toast-loss-limit-reset-failed = Não foi possível reiniciar o limite de perda
trader-toast-entry-monitor-failed = Não foi possível alternar o monitor de entrada
trader-toast-exit-monitor-failed = Não foi possível alternar o monitor de saída
trader-toast-load-failed = Falha ao carregar
    .message = Falha ao carregar a configuração do trader
trader-toast-saved = Configuração salva
    .message = Configurações do trader aplicadas com sucesso
trader-toast-save-failed = Falha ao salvar
    .message = Falha ao salvar a configuração do trader
trader-toast-feature-enabled = Recurso ativado
trader-toast-feature-disabled = Recurso desativado
trader-toast-feature-applied = Configuração do Trader automático aplicada
trader-toast-strategy-enabled = Estratégia ativada
    .message = A estratégia está ativa
trader-toast-strategy-disabled = Estratégia desativada
    .message = A estratégia está inativa
trader-toast-strategy-failed = Falha na atualização
    .message = Falha ao atualizar o status da estratégia

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Janela de estatísticas
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = Desempenho realizado
trader-metric-net-pnl = P&L líquido
trader-metric-win-rate = Taxa de acerto
trader-metric-profit-factor = Fator de lucro
trader-metric-max-drawdown = Drawdown máx.
trader-metric-capital = Capital em uso
trader-metric-avg-win-loss = Ganho / perda méd.
trader-metric-closed-trades = Trades fechados
trader-metric-median-hold = Posse mediana
trader-stats-empty = Nenhum trade fechado nesta janela
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = { $won } ganhos · { $lost } perdidos
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
        [one] { $amount } ganho
        [many] { $amount } ganhos
       *[other] { $amount } ganhos
    }
trader-stats-losses =
    { $count ->
        [one] { $amount } perda
        [many] { $amount } perdas
       *[other] { $amount } perdas
    }
# $amount is a formatted SOL amount.
trader-stats-expected = { $amount } esperados por trade
trader-stats-profit-factor-basis = Ganho bruto ÷ perda bruta
trader-stats-drawdown-basis = Maior queda realizada do pico ao fundo
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
        [one] { $used } de { $max } vaga de posição em uso
        [many] { $used } de { $max } vagas de posição em uso
       *[other] { $used } de { $max } vagas de posição em uso
    }
trader-stats-avg-basis = Resultado médio de um trade vencedor vs perdedor
trader-stats-closed =
    { $count ->
        [one] { $amount } posição fechada
        [many] { $amount } posições fechadas
       *[other] { $amount } posições fechadas
    }
# $span is a formatted duration.
trader-stats-hold-average = { $span } em média
trader-stats-excluded =
    { $count ->
        [one] { $amount } rodada fechada excluída: sem custo base completo, portanto sem P&L confiável.
        [many] { $amount } rodadas fechadas excluídas: sem custo base completo, portanto sem P&L confiável.
       *[other] { $amount } rodadas fechadas excluídas: sem custo base completo, portanto sem P&L confiável.
    }

## Stats: daily P&L and extremes

trader-daily-title = P&L diário
trader-daily-subtitle = { -sol } realizado por dia, com o total acumulado
trader-daily-loading = Carregando P&L diário...
trader-daily-chart = Lucro e prejuízo realizado por dia em { -sol }
trader-extreme-best = Melhor trade
trader-extreme-worst = Pior trade

## Stats: exit breakdown

trader-exit-title = Detalhamento das estratégias de saída
trader-exit-subtitle = Como as posições foram fechadas e o retorno de cada saída
trader-exit-loading = Carregando dados de saída...
trader-exit-empty-day = Nenhum trade fechado nas últimas 24 horas
trader-exit-empty-days =
    { $count ->
        [one] Nenhum trade fechado no último { $amount } dia
        [many] Nenhum trade fechado nos últimos { $amount } dias
       *[other] Nenhum trade fechado nos últimos { $amount } dias
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
        [one] { $amount } trade · { $share } das saídas
        [many] { $amount } trades · { $share } das saídas
       *[other] { $amount } trades · { $share } das saídas
    }
# $value is a formatted average percentage.
trader-exit-average = { $value } méd.

## Shared example vocabulary

trader-impact-label = Impacto:
trader-current-label = Atual:
trader-readable-label = Legível:
trader-example-how-it-works = Como funciona
trader-step-entry = Entrada
trader-step-initial-position = Posição inicial
trader-step-auto-exit = Saída automática
trader-step-exit = Saída
trader-step-full-exit = Saída total da posição
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = +{ $value }% de lucro

## Stop loss

trader-stop-loss-title = Stop loss
trader-stop-loss-subtitle = Sai automaticamente de uma posição quando a perda passa do seu limite
trader-stop-loss-threshold-badge = Limite de perda
trader-stop-loss-hold-badge = Atraso opcional
# $threshold is the threshold as typed.
trader-stop-loss-impact = Sai quando cair { $threshold }% da entrada
trader-stop-loss-hold-immediate = Imediato
# $span is a formatted duration.
trader-stop-loss-hold-delay = Atraso de { $span }
trader-stop-loss-price-falls = Preço cai
trader-stop-loss-threshold-reached = Limite atingido
trader-stop-loss-partial = Saídas parciais permitidas
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = Perda limitada a <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Nota:</strong> o stop loss protege contra perdas maiores saindo cedo

## Trailing stop

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Protege lucros automaticamente acompanhando o preço enquanto ele sobe
trader-trailing-activation-badge = Quando iniciar
trader-trailing-distance-badge = Margem de segurança
# $value is the activation percentage as typed.
trader-trailing-activation-impact = Começa a acompanhar com +{ $value }% de lucro
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = Sai a -{ $value }% do pico
trader-trailing-activation = Ativação
trader-trailing-peak = Pico
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% final
# $value is a formatted percentage.
trader-trailing-summary-protected = <strong>{ $value }</strong> de lucro protegido
# $value is a formatted percentage.
trader-trailing-summary-avoided = <strong>{ $value }</strong> de perda evitada desde o pico

## Take profit

trader-roi-title = Take profit
trader-roi-subtitle = Sai automaticamente de toda a posição quando o lucro atinge a sua meta
trader-roi-target-badge = Meta única
# $target is the target percentage as typed.
trader-roi-impact = Sai com +{ $target }% de lucro
trader-roi-example-title = Cenário de exemplo
trader-roi-initial-buy = Compra inicial
trader-roi-target-hit = Meta atingida
trader-roi-full-position = Posição total
trader-roi-sold = 100% vendido
# $target is the target percentage as typed.
trader-roi-summary = <strong>+{ $target }%</strong> de lucro garantido

## Time-based exit

trader-time-title = Saída por tempo
trader-time-subtitle = Sai automaticamente das posições após um tempo máximo de posse se a perda passar do limite
trader-time-hold-badge = Gatilho de tempo
trader-time-loss-badge = Filtro de perda
trader-time-unit-seconds = segundos
trader-time-unit-minutes = minutos
trader-time-unit-hours = horas
trader-time-unit-days = dias
# Shown before the configured duration loads.
trader-time-conversion-default = 168 horas = 7 dias
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
        [one] { $amount } segundo
        [many] { $amount } segundos
       *[other] { $amount } segundos
    }
trader-duration-minutes =
    { $count ->
        [one] { $amount } minuto
        [many] { $amount } minutos
       *[other] { $amount } minutos
    }
trader-duration-hours =
    { $count ->
        [one] { $amount } hora
        [many] { $amount } horas
       *[other] { $amount } horas
    }
trader-duration-days =
    { $count ->
        [one] { $amount } dia
        [many] { $amount } dias
       *[other] { $amount } dias
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = Sai se estiver em queda de { $value }% ou mais após o período de posse
# $day is the day number of the example.
trader-time-day = Dia { $day }
trader-time-position-opened = Posição aberta
trader-time-limit = Limite de tempo
trader-time-hold-reached = Período de posse atingido
trader-time-loss-met = Limite de perda atingido
trader-time-note = <strong>Nota:</strong> posições com lucro ou com perdas menores NÃO serão encerradas
trader-time-positions-title = Status das posições atuais
trader-time-positions-loading = Carregando posições...
trader-time-positions-empty = Nenhuma posição aberta
trader-time-positions-hold = Tempo de posse:
trader-time-positions-roi = ROI:

## Strategy control

trader-strategy-entry-title = Estratégias de entrada
trader-strategy-entry-subtitle = Sinais que podem abrir uma nova posição.
trader-strategy-exit-title = Estratégias de saída
trader-strategy-exit-subtitle = Sinais que podem fechar ou proteger uma posição aberta.
trader-strategy-active-unknown = -- ativas
trader-strategy-active = { $enabled }/{ $total } ativas
trader-strategy-loading = Carregando estratégias...
trader-strategy-load-failed = Não foi possível carregar as estratégias
trader-strategy-empty = Nenhuma estratégia definida
trader-strategy-no-description = Sem descrição.
trader-strategy-unnamed = Estratégia sem nome
trader-strategy-type-unknown = Estratégia
trader-strategy-priority-auto = Auto
trader-strategy-priority = Prioridade { $priority }

## Dollar-cost averaging

trader-dca-title = Dollar-cost averaging (DCA)
trader-dca-subtitle = Faz aportes automáticos em posições no prejuízo para reduzir o preço médio de entrada
trader-dca-threshold-badge = Gatilho de entrada
trader-dca-example-title = Exemplo de DCA
trader-dca-example = 0.01 { -sol } inicial → DCA #1: 0.005 { -sol } @ -10% → DCA #2: mais 0.005 { -sol } @ -10%
trader-dca-info-title = Sobre a estratégia de DCA
trader-dca-info-subtitle = Pontos importantes ao usar DCA
trader-dca-how-title = Como o DCA funciona
trader-dca-how-trigger = <strong>Gatilho:</strong> a posição cai abaixo do limite de DCA (ex.: -10%)
trader-dca-how-action = <strong>Ação:</strong> aporta mais { -sol } para reduzir o custo base médio
trader-dca-how-repeat = <strong>Repetição:</strong> pode fazer DCA várias vezes, conforme a contagem máxima
trader-dca-risk-title = Avisos de risco
trader-dca-risk-exposure = <strong>Maior exposição:</strong> o DCA aumenta o capital total em risco por posição
trader-dca-risk-knife = <strong>Faca caindo:</strong> o DCA não ajuda se o token continuar em tendência de queda
trader-dca-risk-cooldown = <strong>Cooldown:</strong> use o cooldown para evitar entradas de DCA em sequência rápida

## General settings

trader-sizing-title = Tamanho da posição
trader-sizing-subtitle = Controle quanto investir por posição
trader-sizing-positions-badge = Controle de risco
trader-sizing-trade-size-badge = Por posição
trader-timing-title = Tempo e cooldowns
trader-timing-subtitle = Controle o intervalo entre operações
trader-timing-close-cooldown = Cooldown após fechar posição
trader-timing-close-cooldown-hint = Minutos de espera antes de reabrir o mesmo token
trader-timing-concurrency = Simultaneidade da checagem de entrada
trader-timing-concurrency-hint = Número de tokens verificados ao mesmo tempo (maior = mais rápido, porém mais CPU)
trader-timing-unit-minutes = min
trader-timing-unit-tokens = tokens
trader-timing-intervals = Intervalos dos monitores
trader-timing-intervals-badge = Somente leitura
trader-timing-intervals-hint = Definidos no código (não editáveis pela interface)
trader-timing-intervals-value = <strong>Monitor de entrada:</strong> 30s | <strong>Monitor de saída:</strong> 5s
