# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = Stop loss
trader-exit-type-take-profit = Take profit
trader-exit-type-roi = Take profit
trader-exit-type-roi-exit = Take profit
trader-exit-type-trailing-stop = Trailing stop
trader-exit-type-time-override = Saída por tempo
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
trader-tab-take-profit = Take profit
trader-tab-time-rules = Saída por tempo
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
trader-setup-action = Concluir configuração
trader-toggle-on = ATIVADO
trader-toggle-off = DESATIVADO
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
trader-force-stop-hint = Interrompe de uma vez todos os monitores, tarefas de cópia e trades.
trader-force-stop-idle = Nada está em execução para uma parada forçada interromper.

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
trader-toast-strategy-armed = { $name } agora opera no modo real
trader-toast-strategy-paper = { $name } agora opera em Paper
trader-toast-strategy-mode-failed = Não foi possível alterar o modo de trading

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = Janela de estatísticas
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-stats-book =
    .aria-label = Livro de trading
trader-paper-run-since = Rodada em Paper desde { $date }
trader-paper-start = Início
trader-paper-cash = Caixa
trader-paper-open = Aberto
trader-paper-unrealized = Não realizado
trader-paper-reset = Reiniciar rodada em Paper
trader-paper-reset-title = Reiniciar a rodada em Paper?
trader-paper-reset-message = Cada posição em Paper aberta fecha pelo preço atual e uma nova rodada começa com o saldo inicial configurado. As rodadas anteriores ficam no histórico.
trader-paper-reset-done = Uma nova rodada em Paper começou
trader-paper-reset-failed = Não foi possível reiniciar a rodada em Paper
trader-paper-account-failed = Não foi possível carregar a rodada em Paper
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
# $threshold is the threshold as typed.
trader-stop-loss-impact = Sai quando cair { $threshold }% da entrada
trader-stop-loss-hold-immediate = Imediato
# $span is a formatted duration.
trader-stop-loss-hold-delay = Atraso de { $span }
trader-stop-loss-price-falls = Preço cai
trader-stop-loss-threshold-reached = Limite atingido
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = Vender { $part }
trader-stop-loss-part-of-position = Parte da posição
trader-stop-loss-sell-rest = Vender o restante
trader-stop-loss-still-below = Ainda abaixo do stop
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = Perda limitada a <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>Nota:</strong> o stop loss protege contra perdas maiores saindo cedo

## Trailing stop

trader-trailing-title = Trailing stop
trader-trailing-subtitle = Protege lucros automaticamente acompanhando o preço enquanto ele sobe
# $value is the activation percentage as typed.
trader-trailing-activation-impact = Começa a acompanhar com +{ $value }% de lucro
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = Sai a -{ $value }% do pico
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = Mínimo garantido ao ativar: { $value }
trader-trailing-locked-minimum-warning = O stop fica no preço de entrada ou abaixo ao ativar; ele só vende no lucro. Ative o stop no ponto de equilíbrio para ter um piso.
trader-trailing-activation = Ativação
trader-trailing-peak = Pico
# $value is a formatted percentage.
trader-trailing-final = +{ $value }% final
# $value is a formatted percentage.
trader-trailing-summary-protected = <strong>{ $value }</strong> de lucro protegido
# $value is a formatted percentage.
trader-trailing-summary-avoided = <strong>{ $value }</strong> de perda evitada desde o pico
# $step is the band's position in the list, from 1.
trader-trailing-band = Faixa { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = Lucro máximo da faixa { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = Distância da faixa { $step }
trader-trailing-band-profit-caption = A partir do lucro máximo
trader-trailing-band-distance-caption = Acompanhar a
trader-trailing-add-band = Adicionar faixa
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = Remover faixa { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = O lucro máximo de cada faixa deve estar entre { $min }% e { $max }%
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = A distância de cada faixa deve estar entre { $min }% e { $max }%
trader-trailing-band-error-order = Cada faixa precisa de um lucro máximo maior que a ativação e que a faixa anterior
trader-trailing-band-error-distance = Cada faixa deve acompanhar a uma distância menor que seu lucro máximo

## Break-even stop

trader-break-even-title = Stop no ponto de equilíbrio
trader-break-even-subtitle = Depois que a posição entra no lucro, venda tudo antes que vire prejuízo
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = Assim que o melhor preço desde a última compra chega a { $trigger }, uma volta a { $offset } vende tudo.

## Take profit

trader-take-profit-title = Take profit
trader-take-profit-subtitle = Vende a posição em etapas conforme o lucro atinge cada nível
trader-take-profit-initial-buy = Compra inicial
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = Etapa { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = Nível de lucro da etapa { $step }
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = Parte a vender da etapa { $step }
trader-take-profit-profit-caption = Nível de lucro
trader-take-profit-sell-caption = Parte a vender
trader-take-profit-add-step = Adicionar etapa
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = Remover etapa { $step }
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = Cada nível de lucro deve ficar entre { $min }% e { $max }%
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = Cada parte deve ficar entre { $min }% e { $max }%
trader-take-profit-error-order = Cada etapa precisa de um nível de lucro maior que o anterior
# $max is the largest total share, the whole position.
trader-take-profit-error-total = As etapas juntas podem vender no máximo { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = { $runner }% da posição ficam abertos como runner
trader-take-profit-runner-none = A última etapa vende o restante da posição
trader-take-profit-runner-warning = Com o trailing stop, o stop no ponto de equilíbrio, as duas saídas por tempo e todas as estratégias de saída desligados, o runner só fecha pelo stop loss ou manualmente
trader-take-profit-info = As partes se referem à posição inicial. Se o preço passar vários níveis de uma vez, uma etapa é vendida por verificação.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = Em +{ $profit }%, vender { $share }%
trader-take-profit-example-runner = Runner
trader-take-profit-example-runner-detail = Fica com as outras regras de saída
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = Vende <strong>{ $sold }%</strong> até +{ $profit }% de lucro

## Time-based exit

trader-time-loss-title = Fechar posições no prejuízo
trader-time-subtitle = Sai automaticamente das posições após um tempo máximo de posse se a perda passar do limite
trader-time-flat-title = Fechar posições estagnadas
trader-time-flat-subtitle = Libere o capital de posições que não sobem nem descem
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = Uma posição mantida por { $span } ou mais com P&L dentro de ±{ $band } é fechada.
trader-time-unit-seconds = segundos
trader-time-unit-minutes = minutos
trader-time-unit-hours = horas
trader-time-unit-days = dias
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
trader-time-loss-hint = A perda na qual a saída por tempo vende depois do tempo de retenção (40 = saída em -40%)
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
trader-time-positions-empty-message = As posições abertas aparecem aqui com o tempo de retenção e o ROI em relação ao limite de tempo.
trader-time-positions-token = Token
trader-time-positions-hold = Tempo de posse
trader-time-positions-roi = ROI
trader-time-positions-rule = Saída por tempo
trader-time-status-off = As saídas por tempo estão desligadas
trader-time-status-none = Nenhuma saída por tempo se aplica agora
trader-time-status-not-managed = Não gerenciada pelo Trader automático
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = No prejuízo { $pnl }; fecha em { $remaining }
trader-time-status-losing-due = No prejuízo { $pnl }; fecha na próxima verificação
# $held and $remaining are formatted durations.
trader-time-status-flat = Estagnada há { $held }; fecha em { $remaining }
trader-time-status-flat-due = Estagnada há { $held }; fecha na próxima verificação

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
trader-strategy-unnamed = Estratégia sem nome
trader-strategy-priority-auto = Auto
trader-strategy-priority = Prioridade { $priority }
trader-strategy-mode-arm = Armar modo real
trader-strategy-mode-return-paper = Voltar ao Paper
trader-strategy-arm-title = Armar “{ $name }” no modo real
trader-strategy-arm-lead = As novas entradas de “{ $name }” vão operar com swaps reais da sua carteira de trading.
trader-strategy-arm-readiness = Prontidão segundo o livro de Paper
trader-strategy-arm-open-paper = As posições em Paper abertas continuam em Paper e terminam em Paper.
trader-strategy-arm-ack-real-native = As novas entradas gastam { -sol } real da sua carteira de trading.
trader-strategy-arm-ack-fees = Os swaps reais pagam taxas de rede e slippage de verdade; os resultados em Paper não garantem resultados reais.
trader-strategy-arm-ack-unready = Algumas verificações de prontidão não foram aprovadas. Armar esta estratégia mesmo assim.
trader-strategy-arm-unavailable = O trading real está indisponível no momento; veja a última verificação.
trader-strategy-return-title = Voltar “{ $name }” ao Paper
trader-strategy-return-message = As novas entradas abrem no livro de Paper sem gastar { -sol }. As posições reais abertas continuam reais e terminam no modo real.
trader-strategy-readiness-failed = Não foi possível ler a prontidão da estratégia
trader-paper-explore-title = O trading em Paper começa após a configuração

## Dollar-cost averaging

trader-dca-title = Dollar-cost averaging (DCA)
trader-dca-subtitle = Faz aportes automáticos em posições no prejuízo para reduzir o preço médio de entrada
trader-dca-measure-average = Entrada média
trader-dca-measure-first-entry = Primeira entrada
trader-dca-step = Aporte { $step }
trader-dca-drop-label = Queda de preço do aporte { $step }
trader-dca-size-label = Tamanho do aporte { $step }, vezes a primeira compra
trader-dca-drop-caption = Queda de preço
trader-dca-size-caption = Compra × primeira compra
trader-dca-add-step = Adicionar nível
trader-dca-remove-step = Remover aporte { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = Abaixo da operação mínima de { $minimum }: este aporte não é enviado
trader-dca-error-drop-range = Cada queda deve estar entre { $min }% e { $max }%
trader-dca-error-size-range = Cada tamanho deve estar entre { $min }× e { $max }× a primeira compra
trader-dca-error-order = Medido a partir da primeira entrada, cada aporte precisa de uma queda mais funda que o anterior
trader-dca-cooldown-impact = Pelo menos { $span } entre entradas extras
trader-dca-min-hold-none = O primeiro aporte pode vir logo após a entrada
trader-dca-min-hold-impact = O primeiro aporte espera pelo menos { $span } após a entrada
trader-dca-capacity = O máximo que esta posição pode ter: { $amount } ({ $multiple }× a primeira compra)
trader-dca-info-title = Sobre a estratégia de DCA
trader-dca-info-subtitle = Pontos importantes ao usar DCA
trader-dca-how-title = Como o DCA funciona
trader-dca-how-trigger = <strong>Gatilho:</strong> o preço cai abaixo do próximo nível, medido a partir da entrada média ou da primeira
trader-dca-how-action = <strong>Ação:</strong> aporta mais { -sol } para reduzir o custo base médio
trader-dca-how-repeat = <strong>Escada:</strong> cada aporte passa ao próximo nível; os aportes param quando todos os níveis são usados
trader-dca-how-pause = <strong>Pausas:</strong> nenhum aporte enquanto o anterior está sendo liquidado ou enquanto o limite de perdas bloqueia novas entradas
trader-dca-risk-title = Avisos de risco
trader-dca-risk-exposure = <strong>Maior exposição:</strong> o DCA aumenta o capital total em risco por posição
trader-dca-risk-knife = <strong>Faca caindo:</strong> o DCA não ajuda se o token continuar em tendência de queda
trader-dca-risk-cooldown = <strong>Cooldown:</strong> use o cooldown para evitar entradas de DCA em sequência rápida

## General settings

trader-sizing-title = Tamanho da posição
trader-sizing-subtitle = Controle quanto investir por posição
trader-timing-title = Tempo e cooldowns
trader-timing-subtitle = Controle o intervalo entre operações
trader-timing-close-cooldown = Cooldown após fechar posição
trader-timing-close-cooldown-hint = Minutos de espera antes de reabrir o mesmo token
trader-timing-concurrency = Simultaneidade da checagem de entrada
trader-timing-concurrency-hint = Número de tokens verificados ao mesmo tempo (maior = mais rápido, porém mais CPU)
trader-timing-unit-minutes = min
trader-timing-unit-tokens = tokens
