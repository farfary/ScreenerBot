copy-skip-not-buy-swap = A atividade da carteira não foi uma compra
copy-skip-task-disabled = A tarefa está pausada
copy-skip-mode-transition-required = O modo de execução deve ser alterado separadamente
copy-skip-live-confirmation-required = A execução real precisa de confirmação
copy-skip-unsupported-sizing-mode = O modo de dimensionamento ainda não é compatível
copy-skip-self-copy = A carteira é uma das suas
copy-skip-target-below-minimum = Trade da carteira abaixo do mínimo
copy-skip-target-above-maximum = Trade da carteira acima do máximo
copy-skip-already-bought = Este token já foi comprado (comprar uma vez)
copy-skip-blacklisted = Token bloqueado pelos controles de risco
copy-skip-filter-required = O token não passou na filtragem
copy-skip-budget-exhausted = O orçamento da tarefa acabou
copy-skip-token-cap-reached = Limite por token atingido
copy-skip-below-minimum-size = Tamanho da cópia muito pequeno
copy-skip-invalid-sizing = O dimensionamento da tarefa é inválido
copy-skip-invalid-slippage = O slippage da tarefa é inválido
copy-skip-invalid-exit-policy = As regras de saída da tarefa são inválidas
copy-skip-invalid-price = Sem preço de mercado utilizável
copy-skip-not-sell-swap = A atividade da carteira não foi uma venda
copy-skip-exit-mode-disabled = Venda da carteira ignorada: a tarefa vende pelas próprias regras
copy-skip-force-stopped = O trading está em parada forçada
copy-skip-copy-position-not-found = Nenhuma posição pertence a esta tarefa
copy-skip-position-user-only = A posição é gerenciada por você
copy-skip-position-management-mismatch = A posição não acompanha mais as vendas copiadas
copy-skip-latency-kill-switch = Pausada automaticamente: trades detectados tarde demais
copy-skip-claim-reconciled-abandoned = Envio real interrompido encerrado sem nova tentativa
copy-skip-stale-observation = Reprocessado após inatividade, antigo demais para copiar
copy-skip-unknown-observation-time = O trade reprocessado não tem horário de bloco
copy-skip-entry-blocked = Entrada bloqueada

copy-entry-block-force-stopped = O trading está em parada forçada
copy-entry-block-loss-limit = O limite de perda bloqueia novas entradas
copy-entry-block-connectivity = Os serviços necessários estão indisponíveis
copy-entry-block-position-limit = Limite de posições abertas atingido
copy-entry-block-already-open = Já existe uma posição aberta
copy-entry-block-reentry-cooldown = Cooldown de reentrada no token
copy-entry-block-open-cooldown = Cooldown global de entradas
copy-entry-block-entry-reserved = Outra entrada está sendo processada
copy-entry-block-blacklisted = Token bloqueado pelos controles de risco
copy-entry-block-check-failed = Uma verificação de segurança não pôde ser concluída

copy-pause-user = Pausada por você
copy-pause-latency-kill-switch = Pausada automaticamente: os trades chegaram { $average }s atrasados em média (limite de { $threshold }s)
copy-pause-watch-detached = Pausada automaticamente: a carteira não está mais sendo monitorada
copy-pause-watch-budget-exceeded = Pausada: esta carteira atingiu o limite de { $limit } assinaturas por verificação do monitoramento antes de se atualizar
copy-pause-helius-unavailable = Pausada: as verificações de carteira do { -helius } falharam
copy-pause-watch-processing-failed = Pausada: não foi possível processar a atividade da carteira
copy-pause-unspecified = Pausada

copy-pause-short-user = por você
copy-pause-short-latency-kill-switch = lenta demais
copy-pause-short-watch-detached = monitoramento perdido
copy-pause-short-watch-budget-exceeded = limite do monitoramento
copy-pause-short-helius-unavailable = provedor do monitoramento
copy-pause-short-watch-processing-failed = processamento do monitoramento
copy-state-paused = Pausada
copy-state-paused-reason = Pausada · { $reason }

copy-readiness-history = Histórico em Paper
copy-readiness-history-met =
    { $count ->
        [one] { $count } rodada fechada em Paper, { $needed } necessárias
        [many] { $count } rodadas fechadas em Paper, { $needed } necessárias
       *[other] { $count } rodadas fechadas em Paper, { $needed } necessárias
    }
copy-readiness-history-short = { $count } de { $needed } rodadas fechadas em Paper
copy-readiness-profit = Lucrativa em Paper
copy-readiness-profit-detail =
    { $count ->
        [one] { $realized } { -sol } realizados em { $count } rodada, { $wins } vencedoras
        [many] { $realized } { -sol } realizados em { $count } rodadas, { $wins } vencedoras
       *[other] { $realized } { -sol } realizados em { $count } rodadas, { $wins } vencedoras
    }
copy-readiness-latency = Trades detectados a tempo
copy-readiness-latency-detail = chegada p95 de { $p95 }s, limite de { $limit }s
copy-readiness-latency-none = Ainda sem amostras de chegada
copy-readiness-priced = Todas as posses com preço
copy-readiness-priced-ok = Toda posse aberta em Paper tem preço de pool
copy-readiness-priced-missing =
    { $count ->
        [one] { $count } posse aberta sem preço de pool
        [many] { $count } posses abertas sem preço de pool
       *[other] { $count } posses abertas sem preço de pool
    }
copy-readiness-runtime = Execução real disponível
copy-readiness-runtime-ok = A configuração e as travas de segurança permitem cópias reais

copy-live-block-setup-incomplete = Conclua antes a configuração da carteira e do RPC
copy-live-block-force-stop = A parada de emergência está ativa
copy-live-block-copy-trading-disabled = O processamento de cópias está pausado globalmente
copy-live-block-unavailable = A execução real está indisponível

copy-state-system-paused = Pausada globalmente
copy-state-force-stopped = Parada forçada
copy-state-entries-blocked = Entradas bloqueadas
copy-state-running-live = Em execução
copy-state-running-paper = Em execução
copy-mode-paper = Paper
copy-mode-live = Real
copy-exit-mode-buy-only = Minhas regras de saída
copy-exit-mode-mirror = Espelhar vendas da carteira
copy-exit-mode-hybrid = Vendas da carteira e minhas regras
copy-exit-target-sell = Carteira vendeu
copy-exit-stop-loss = Stop loss
copy-exit-trailing-stop = Trailing stop
copy-exit-take-profit = Take profit
copy-exit-time-override = Regra de tempo
copy-exit-manual = Fechada manualmente

copy-request-failed = Falha na requisição
copy-keep-paused = Manter pausada
copy-paused-suffix = · pausada
copy-mode-paused = { $mode } · pausada
copy-task-ref = “{ $name }” ({ $mode })
copy-metric-realized-pnl = P&L realizado
copy-metric-unrealized-pnl = P&L não realizado
copy-metric-win-rate = Taxa de acerto
copy-metric-budget-spent = Orçamento gasto
copy-metric-median-arrival = Chegada mediana
copy-metric-open-holdings = Posses abertas
copy-record-won-lost = { $won } ganhas · { $lost } perdidas
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = Execuções
copy-kind-exits = Saídas
copy-kind-skips = Ignoradas
copy-kind-errors = Erros
copy-field-per-trade-cap = Limite por trade
copy-field-per-token-cap = Limite por token
copy-field-total-budget = Orçamento total
copy-field-slippage = Slippage
copy-rules-wallet-sells-only = Somente vendas da carteira
copy-filter-copy-setting-required = Configuração da cópia (obrigatório)
copy-filter-copy-setting-not-required = Configuração da cópia (não obrigatório)
copy-count-closed-rounds =
    { $count ->
        [one] { $count } rodada fechada
        [many] { $count } rodadas fechadas
       *[other] { $count } rodadas fechadas
    }
copy-count-open-holdings =
    { $count ->
        [one] { $count } posse aberta
        [many] { $count } posses abertas
       *[other] { $count } posses abertas
    }
copy-unrealized-partial =
    { $priced ->
        [one] { $priced } posse com preço · { $unpriced } sem preço
        [many] { $priced } posses com preço · { $unpriced } sem preço
       *[other] { $priced } posses com preço · { $unpriced } sem preço
    }
copy-unrealized-unpriced =
    { $count ->
        [one] { $count } posse sem preço
        [many] { $count } posses sem preço
       *[other] { $count } posses sem preço
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = Tudo
copy-range-label =
    .aria-label = Intervalo de datas

copy-page-title = Copy Trading
copy-page-beta = Beta
copy-strip-loading = Carregando
copy-strip-unavailable = Indisponível
copy-strip-pause-all = Pausar tudo
copy-strip-resume = Retomar processamento
copy-strip-settings = Configurações
copy-strip-add-wallet = Adicionar carteira
copy-strip-paused-globally = Pausado globalmente · sem novas cópias, as saídas continuam
copy-strip-force-stopped = Parada forçada · nada é copiado
copy-strip-loss-limit = Limite de perda · novas entradas bloqueadas, as saídas continuam
copy-strip-idle-paused =
    { $count ->
        [one] Ocioso · { $count } tarefa pausada
        [many] Ocioso · { $count } tarefas pausadas
       *[other] Ocioso · { $count } tarefas pausadas
    }
copy-strip-idle-empty = Ocioso · nenhuma tarefa ainda
copy-strip-processing = Processando · { $paper } em Paper
copy-strip-processing-live = Processando · { $live } reais · { $paper } em Paper
copy-figures-label =
    .aria-label = Totais do copy trading
copy-figure-marked-at-pool = Avaliado ao preço do pool
copy-figure-across-tasks = Em todas as tarefas
copy-figure-budget-lifetime = Gasto acumulado das tarefas ativas
copy-figure-budget-none = Nenhuma tarefa ativa
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
        [one] { $count } trade
        [many] { $count } trades
       *[other] { $count } trades
    }
copy-figure-arrival-none = Sem amostras das tarefas ativas

copy-load-failed = Não foi possível carregar o copy trading: { $error }
copy-resume-all-title = Retomar processamento de cópias
copy-resume-all-message =
    { $count ->
        [one] { $count } tarefa real enviará swaps reais quando a carteira dela operar novamente.
        [many] { $count } tarefas reais enviarão swaps reais quando as carteiras delas operarem novamente.
       *[other] { $count } tarefas reais enviarão swaps reais quando as carteiras delas operarem novamente.
    }
copy-toast-resumed-all = Processamento de cópias retomado
copy-toast-paused-all = Todo o processamento de cópias foi pausado
copy-toast-global-failed = Não foi possível alterar o processamento de cópias

copy-onboarding-title = Copie as carteiras em que você confia, comprovando-as antes em Paper
copy-onboarding-body = Toda tarefa começa em Paper: os trades da carteira-alvo são simulados ao preço do pool, com o seu slippage e as taxas, e suas regras de saída rodam no livro de Paper. Arme o modo real por carteira quando os resultados em Paper justificarem.
copy-onboarding-add = Adicionar sua primeira carteira
copy-onboarding-observe = Observar
copy-onboarding-observe-detail = Detecte os swaps da carteira sem gastar { -sol }.
copy-onboarding-evaluate = Avaliar
copy-onboarding-evaluate-detail = Analise o P&L em Paper, a taxa de acerto, as cópias ignoradas, a velocidade de detecção e o slippage.
copy-onboarding-arm = Armar
copy-onboarding-arm-detail = Passe nas verificações de prontidão e ative os swaps reais.

copy-list-label =
    .aria-label = Carteiras copiadas
copy-list-title = Carteiras
copy-list-compare = Comparar
copy-list-sort-label = Ordenar carteiras
copy-list-count = { $active } ativas · { $total } no total
copy-sort-pnl = P&L
copy-sort-state = Estado
copy-sort-name = Nome
copy-compare-label =
    .aria-label = Comparar carteiras

copy-dialog-close =
    .aria-label = Fechar
copy-editor-title-add = Adicionar carteira
copy-editor-sub-add = Novas tarefas começam em Paper
copy-arm-title = Armar cópia real
copy-arm-sub = Swaps reais a partir da sua carteira
copy-arm-keep-paper = Manter Paper
copy-arm-confirm = Armar modo real
copy-profile-title = Perfil da carteira
copy-profile-sub = O que este bot já viu da carteira

copy-settings-title = Configurações do copy trading
copy-settings-subtitle = Política global para todas as tarefas
copy-settings-filter-warning = Com a configuração de filtragem padrão, isso rejeita quase todos os tokens e nada é copiado. Deixe desativado, a menos que seus filtros aprovem os tokens que suas carteiras negociam.
copy-settings-unit-seconds = segundos
copy-settings-unit-trades = trades
copy-settings-unit-tasks = tarefas
copy-settings-unit-closed-rounds = rodadas fechadas
copy-settings-save = Salvar configurações
copy-settings-load-failed = Não foi possível carregar as configurações de cópia
copy-settings-saved = Configurações do copy trading salvas

copy-tab-overview = Visão geral
copy-tab-holdings = Posses
copy-tab-activity = Atividade
copy-tab-rules = Regras
copy-tab-execution = Execução
copy-tabs-label = Visões da tarefa
copy-workspace-select = Selecione uma carteira para abrir o espaço de trabalho dela.
copy-workspace-loading = Carregando tarefa…
copy-workspace-load-failed = Não foi possível carregar esta tarefa: { $error }

copy-state-detail-paper = Em execução em Paper · os trades são simulados, nada é gasto
copy-state-detail-live = Em execução real · os trades da carteira são copiados com swaps reais
copy-state-detail-system-paused = Em espera · o processamento de cópias está pausado globalmente, as saídas continuam
copy-state-detail-entries-blocked = Entradas bloqueadas pelo limite de perda · as saídas continuam
copy-state-detail-force-stopped = Parada forçada · nada é copiado

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = Retomar mantém o mesmo limite, então ela pausa de novo enquanto os trades ainda chegarem tarde. Verifique o stream de RPC ou aumente o limite de chegada em Configurações.
copy-paused-resume-detached = Retomar volta a monitorar a carteira.
copy-paused-holdings-rules =
    { $count ->
        [one] As regras de saída ainda fecham a { $count } posse aberta dela.
        [many] As regras de saída ainda fecham as { $count } posses abertas dela.
       *[other] As regras de saída ainda fecham as { $count } posses abertas dela.
    }
copy-paused-holdings-mirror =
    { $count ->
        [one] As vendas da carteira ainda fecham a { $count } posse aberta dela.
        [many] As vendas da carteira ainda fecham as { $count } posses abertas dela.
       *[other] As vendas da carteira ainda fecham as { $count } posses abertas dela.
    }
copy-paused-holdings-hybrid =
    { $count ->
        [one] As vendas da carteira e as regras de saída ainda fecham a { $count } posse aberta dela.
        [many] As vendas da carteira e as regras de saída ainda fecham as { $count } posses abertas dela.
       *[other] As vendas da carteira e as regras de saída ainda fecham as { $count } posses abertas dela.
    }

copy-watch-state-catching-up = Monitoramento da carteira: atualizando. Verificando esta carteira pelo { -helius }.
copy-watch-state-watching = Monitoramento da carteira: monitorando. Verificando esta carteira pelo { -helius }.
copy-watch-last-check = Última verificação { $ago }.
copy-watch-recovery-active = Monitoramento da carteira ativo
copy-watch-recovery-catching-up = O monitoramento da carteira está se atualizando
copy-watch-recovery-still-paused = A tarefa de cópia continua pausada. Retome a cópia quando quiser.
copy-watch-recovery-title = Restaurar monitoramento da carteira
copy-watch-recovery-processing-failed = Não foi possível processar a atividade da carteira. O progresso salvo foi preservado. Tente novamente após resolver o problema.
copy-watch-recovery-provider-failed = As verificações do { -helius } falharam. O progresso salvo foi preservado. Tente novamente quando o provedor estiver disponível.
copy-watch-recovery-budget-intro = Esta carteira tem mais atividade do que o monitoramento atual consegue verificar. Escolha como continuar.
copy-watch-approve = Tentar se atualizar usando o { -helius }
copy-watch-approve-help = Continua a partir do progresso salvo. Pode usar mais créditos do { -helius } e ainda pode ficar para trás.
copy-watch-approve-unavailable = A atualização pelo { -helius } está indisponível. Configure um endpoint de RPC do { -helius } ativado para continuar sem pular atividade não verificada.
copy-watch-no-provider = Nenhum provedor de atualização é compatível com este monitoramento.
copy-watch-budget-label = Assinaturas verificadas por verificação
copy-watch-budget-hint = Ou pule a atividade não verificada e retome a partir de agora. Escolha de { $min } a { $max } assinaturas por verificação; um limite maior pode usar mais chamadas de RPC.
copy-watch-ack = Entendo que a atividade perdida não será copiada.
copy-watch-toast-range = Escolha entre { $min } e { $max } assinaturas por consulta, em passos de { $step } assinaturas
copy-watch-toast-ack = Confirme que as assinaturas desde a última verificação concluída serão puladas
copy-watch-resumed = Monitoramento da carteira retomado a partir de agora; a tarefa de cópia continua pausada
copy-watch-resume-failed = Não foi possível retomar o monitoramento da carteira
copy-watch-retry-started = Nova tentativa do monitoramento iniciada a partir do progresso salvo; a tarefa de cópia continua pausada
copy-watch-retry-failed = Não foi possível tentar novamente o monitoramento da carteira
copy-watch-approve-title = Permitir atualização pelo { -helius } para esta carteira
copy-watch-approve-message = O { -helius } pode verificar as transações bem-sucedidas da Solana a partir do progresso salvo, sem pular o intervalo não verificado. Atualmente ele cobra 10 créditos a cada 100 transações completas retornadas, arredondando para cima, com mínimo de 10 créditos por requisição. Uma verificação pode fazer várias requisições; o uso e o preço do provedor podem variar. A cópia continua pausada até você retomá-la separadamente.
copy-watch-approve-confirm = Permitir para esta carteira
copy-watch-approved = Monitoramento da carteira iniciado a partir do progresso salvo; a tarefa de cópia continua pausada
copy-watch-restore-failed = Não foi possível restaurar o monitoramento da carteira

copy-action-pause = Pausar
copy-action-resume = Retomar
copy-action-resume-copy = Retomar cópia
copy-action-resume-from-now = Retomar a partir de agora
copy-action-retry-watch = Tentar monitoramento novamente
copy-action-return-paper = Voltar para Paper
copy-action-edit-rules = Editar regras
copy-action-clone = Clonar
copy-action-profile = Perfil da carteira
copy-resume-live-title = Retomar cópia real
copy-resume-live-message = “{ $name }” enviará swaps reais da sua carteira quando esta carteira operar novamente.
copy-resume-live-confirm = Retomar modo real
copy-task-resumed = Tarefa retomada
copy-task-paused = Tarefa pausada
copy-task-state-failed = Não foi possível alterar o estado da tarefa
copy-return-paper-message = As novas cópias de “{ $name }” voltarão a ser simuladas, sem gastar { -sol }.
copy-return-paper-cancel = Manter modo real
copy-task-returned-paper = Tarefa devolvida ao Paper
copy-mode-change-failed = Não foi possível alterar o modo de execução
copy-delete-title = Excluir tarefa de cópia
copy-delete-message = Excluir “{ $name }”? As decisões e os resultados em Paper dela serão removidos e a carteira deixará de ser monitorada para esta tarefa.
copy-delete-confirm = Excluir tarefa
copy-delete-cancel = Manter tarefa
copy-task-deleted = Tarefa de cópia excluída
copy-task-delete-failed = Não foi possível excluir a tarefa de cópia

copy-overview-results = Resultados
copy-analytics-load-failed = Não foi possível carregar as análises: { $error }
copy-analytics-loading = Carregando análises…
copy-exit-bucket =
    { $count ->
        [one] { $count } venda · { $pnl }
        [many] { $count } vendas · { $pnl }
       *[other] { $count } vendas · { $pnl }
    }
copy-overview-average-win = Ganho médio
copy-overview-average-loss = Perda média { $amount }
copy-overview-profit-factor = Fator de lucro
copy-overview-profit-factor-note = Ganhos brutos ÷ perdas brutas
copy-overview-average-hold = Tempo médio de posse
copy-overview-average-hold-note = Da entrada à saída
copy-overview-best-round = Melhor rodada
copy-overview-worst-round = Pior { $amount }
copy-overview-curve-title = P&L acumulado
copy-overview-exits-title = Vendas por saída
copy-overview-skips-title = Por que os trades foram ignorados
copy-book-title-live = Livro real
copy-book-title-paper = Livro de Paper
copy-book-all-time = Todo o período
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
        [one] compra
        [many] compras
       *[other] compras
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
        [one] saída pelas suas regras
        [many] saídas pelas suas regras
       *[other] saídas pelas suas regras
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
        [one] venda da carteira
        [many] vendas da carteira
       *[other] vendas da carteira
    }
copy-book-manual-closes = <strong>{ $count }</strong> fechadas manualmente
copy-book-skipped = <strong>{ $count }</strong> ignoradas
copy-book-failed = <strong>{ $count }</strong> com falha
copy-book-closed = { $count } fechadas
copy-book-budget-note = Gasto { $mode } de { $total } · { $remaining } restantes
copy-check-passed = aprovada
copy-check-not-passed = não aprovada
copy-readiness-title = Antes de ir para o modo real
copy-readiness-live-note = Esta tarefa opera em modo real. Volte para Paper pelo cabeçalho acima.
copy-readiness-all-pass = Todas as verificações foram aprovadas.
copy-readiness-needs-review = Armar exige uma revisão explícita do que não está pronto.
copy-readiness-arm = Revisar e armar modo real

copy-rules-title = Regras em vigor
copy-rules-size-ratio = { $pct } do trade da carteira
copy-rules-size-fixed = { $amount } por cópia
copy-rules-target-any = Qualquer tamanho
copy-rules-target-min = No mínimo { $amount }
copy-rules-target-max = No máximo { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = Personalizada na tarefa · Trader { $value }
copy-rules-source-default = Padrão do Trader
copy-rules-not-used = Não usada: as vendas da carteira decidem
copy-rules-col-rule = Regra
copy-rules-col-applies = Aplica-se
copy-rules-col-source = Origem
copy-rules-budget-note = { $spent } gastos em { $mode } · { $remaining } restantes
copy-rules-token-copies =
    { $count ->
        [one] Cerca de { $count } cópia completa de um token
        [many] Cerca de { $count } cópias completas de um token
       *[other] Cerca de { $count } cópias completas de um token
    }
copy-rules-sizing = Dimensionamento
copy-rules-copy-size = Tamanho da cópia
copy-rules-entry-filters = Filtros de entrada
copy-rules-target-size = Tamanho do trade da carteira
copy-rules-repeat-buys = Compras repetidas
copy-rules-repeat-first-only = Somente a primeira compra de cada token
copy-rules-repeat-every = Toda compra, até o limite por token
copy-rules-filter-pass = Aprovação na filtragem
copy-rules-filter-required = Obrigatória
copy-rules-filter-not-required = Não obrigatória
copy-rules-filter-task-override = Personalizada na tarefa
copy-rules-exits = Saídas
copy-rules-exits-inactive = As posses só são vendidas quando a carteira vende; as regras abaixo não rodam neste modo.

copy-rule-status = Status
copy-rule-on = Ativada
copy-rule-off = Desativada
copy-rule-unit-seconds = segundos
copy-rule-unit-minutes = minutos
copy-rule-stop-loss-threshold = Vende com uma perda de
copy-rule-stop-loss-min-hold = Não antes de manter por
copy-rule-no-minimum = Sem mínimo
copy-rule-partial-exits = Saídas parciais
copy-rule-partial-allowed = Permitidas
copy-rule-partial-full-only = Somente saída total
copy-rule-partial-size = Tamanho da saída parcial
copy-rule-trailing-activation = Arma com um ganho de
copy-rule-trailing-distance = Vende abaixo do pico em
copy-rule-take-profit-target = Vende com um ganho de
copy-rule-time-duration = Verifica após manter por
copy-rule-time-threshold = Vende enquanto o P&L estiver igual ou abaixo de
copy-preset-inherit = Padrões do Trader
copy-preset-conservative = Conservador
copy-preset-balanced = Equilibrado
copy-preset-aggressive = Agressivo
copy-preset-custom = Personalizado
copy-validate-stop-loss = O stop loss deve ser maior que 0% e no máximo 100%.
copy-validate-partial-size = O tamanho da saída parcial deve estar entre 0% e 100%.
copy-validate-min-hold = O tempo mínimo de posse deve ser um número inteiro de segundos.
copy-validate-trailing-activation = A ativação do trailing deve ser maior que 0% e no máximo 100%.
copy-validate-trailing-distance = A distância do trailing deve ser maior que 0% e no máximo 100%.
copy-validate-take-profit = O take profit deve ser maior que 0%.
copy-validate-time-duration = A regra de tempo precisa de uma duração maior que zero.
copy-validate-time-threshold = O limite da regra de tempo é uma perda: use 0% ou um número negativo.
copy-warning-mirror = Somente as vendas da carteira fecham as posses: nenhum stop loss as protege, e um token que a carteira nunca vende continua em posse.
copy-warning-no-rules = Nenhuma regra de saída está ativada e as vendas da carteira são ignoradas: as posses nunca são vendidas.
copy-warning-no-stop-loss = Nenhum stop loss se aplica: um token em queda é mantido até outra regra ou a carteira vender.
copy-warning-stop-delay = O stop loss espera { $hold } após cada compra: um token que cai mais rápido fecha bem além de { $threshold }.
copy-warning-take-profit-cost = O take profit em { $target } não cobre a venda ({ $slippage } de slippage e { $fee } de taxa de swap), então fecha rodadas com prejuízo.
copy-warning-trailing-distance = A distância do trailing é igual ou maior que o ganho de ativação, então um trailing armado pode vender abaixo da entrada.

copy-execution-title = Qualidade de execução
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = Qualquer
copy-execution-limit-on =
    { $count ->
        [one] Pausa acima de { $limit } em média ao longo de { $count } trade
        [many] Pausa acima de { $limit } em média ao longo de { $count } trades
       *[other] Pausa acima de { $limit } em média ao longo de { $count } trades
    }
copy-execution-limit-off = Kill switch desativado
copy-execution-arrival-samples =
    { $count ->
        [one] { $count } trade visto em tempo real
        [many] { $count } trades vistos em tempo real
       *[other] { $count } trades vistos em tempo real
    }
copy-execution-p95 = Chegada p95
copy-execution-median-slippage = Slippage mediano
copy-execution-slippage-samples =
    { $count ->
        [one] { $count } execução medida
        [many] { $count } execuções medidas
       *[other] { $count } execuções medidas
    }
copy-execution-worst-slippage = Pior slippage
copy-execution-average-slippage = Média { $amount }
copy-execution-delay-title = Atraso de detecção
copy-execution-delay-note = Tempo entre o bloco da carteira e este bot ver o trade. Reprocessamentos após inatividade são excluídos.
copy-execution-delay-limit = Barras acima do limite de chegada de { $limit } ficam em âmbar.
copy-execution-fastest = Mais rápido
copy-execution-average = Média
copy-execution-slowest = Mais lento
copy-execution-fill-title = Execução em relação à carteira
copy-execution-fill-note = Positivo significa pior que a carteira: pagou mais em uma compra, recebeu menos em uma venda espelhada. Uma execução em Paper de um token sem preço de pool é precificada ao trade da própria carteira, então não mede nada e fica de fora.
copy-execution-samples = Amostras
copy-execution-median = Mediana
copy-execution-worst = Pior
copy-execution-decisions = Decisões no intervalo

copy-compare-title = Comparar carteiras
copy-compare-back = Voltar à carteira
copy-compare-load-failed = Não foi possível carregar a comparação: { $error }
copy-compare-loading = Carregando comparação…
copy-compare-empty = Nenhuma tarefa para comparar.
copy-compare-curve-title = P&L realizado acumulado
copy-table-wallet = Carteira
copy-table-mode = Modo
copy-table-rounds = Rodadas
copy-table-realized = Realizado
copy-table-profit-factor = Fator de lucro
copy-table-average-hold = Tempo méd. de posse
copy-table-median-slippage = Slippage mediano

copy-chart-curve-label = P&L acumulado { $amount } { -sol }
copy-chart-compare-label = P&L acumulado por tarefa
copy-chart-empty-curve = Nenhuma rodada fechada neste intervalo ainda.
copy-chart-empty-bars = Nada registrado neste intervalo.
copy-chart-empty-histogram = Nenhuma amostra de chegada neste intervalo.
copy-chart-empty-compare = Nenhuma rodada fechada para comparar neste intervalo.
copy-chart-histogram-title = { $count } de { $total }

copy-profile-copy = Copiar esta carteira
copy-profile-copy-other = Copiar com outras regras
copy-profile-loading = Carregando perfil da carteira…
copy-profile-watch-title = Monitoramento
copy-profile-watched = Monitorada
copy-profile-watch-resume-hint = Retomar uma tarefa volta a monitorá-la
copy-profile-watch-add-hint = Adicionar uma tarefa inicia o monitoramento
copy-profile-stream = Stream
copy-profile-subscribed = Inscrita
copy-profile-not-subscribed = Não inscrita
copy-profile-sources =
    { $count ->
        [one] { $count } origem
        [many] { $count } origens
       *[other] { $count } origens
    }
copy-profile-last-activity = Última atividade
copy-profile-last-error = Último erro
copy-profile-own-wallet = Esta é uma das suas carteiras; copiá-la é recusado.
copy-profile-observed-title = Trades observados
copy-profile-observed-none = Nenhum trade desta carteira neste bot ainda. Uma tarefa em Paper a observa sem gastar { -sol }.
copy-profile-swaps-seen = Swaps vistos
copy-profile-swaps-seen-note = Swaps distintos da carteira em todas as suas tarefas
copy-profile-buys-sells = Compras / vendas
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = Tokens negociados
copy-profile-first-seen = Visto pela primeira vez
copy-profile-last-seen = Visto pela última vez
copy-profile-tasks-title = Suas tarefas nesta carteira
copy-table-task = Tarefa

copy-arm-acks-left =
    { $count ->
        [one] { $count } confirmação restante para marcar
        [many] { $count } confirmações restantes para marcar
       *[other] { $count } confirmações restantes para marcar
    }
copy-arm-readiness-title = Prontidão segundo o livro de Paper
copy-arm-exposure-title = Exposição
copy-arm-per-copy = Por cópia
copy-arm-budget-left-value = { $left } de { $total } { -sol }
copy-arm-budget-left = Orçamento real restante
copy-arm-budget-left-note = O gasto em Paper é contado à parte e não o consome
copy-arm-exits = Saídas
copy-arm-stop-note = Não antes de manter por { $hold }: uma queda mais rápida fecha mais baixo
copy-arm-shared = Esta carteira também é copiada por { $tasks }: cada tarefa copia os trades dela com o próprio orçamento.
copy-arm-unavailable = A execução real está indisponível no momento; veja a última verificação.
copy-arm-ack-real-native = { -sol } de verdade: esta tarefa pode gastar até { $budget } { -sol } da sua carteira, no máximo { $trade } { -sol } por cópia.
copy-arm-ack-fees = As cópias reais pagam taxas de rede e slippage de verdade; os resultados em Paper não garantem resultados reais.
copy-arm-ack-unready = Algumas verificações de prontidão não foram aprovadas. Armar esta tarefa mesmo assim.
copy-arm-lead = “{ $name }” copiará os trades desta carteira com swaps reais da sua carteira.
copy-arm-confirmation-missing = Não foi possível carregar a confirmação do modo real
copy-arm-armed = Cópia real armada
copy-arm-failed = Não foi possível armar a cópia real

copy-holdings-title = Posses
copy-holdings-view-label = Visão das posses
copy-holdings-view-open = Abertas ({ $count })
copy-holdings-view-closed = Rodadas fechadas ({ $count })
copy-holdings-reset = Redefinir livro de Paper
copy-holdings-live-note = As cópias reais são posições reais.
copy-holdings-open-positions = Posições abertas
copy-holdings-token-details = Abrir detalhes do token
copy-holdings-opened = Aberta { $time }
copy-holdings-no-pool-price = Sem preço de pool
copy-holdings-close = Fechar
copy-holdings-write-off = Baixar
copy-holdings-activity = Atividade
copy-holdings-no-exit-rule = Sem regra de saída
copy-holdings-watch-stop = Stop { $level }
copy-holdings-watch-stop-until = Stop { $level } em { $span }
copy-holdings-watch-take = Take { $level }
copy-holdings-watch-trail = Trail { $level }
copy-holdings-watch-trail-arms = Trail arma em { $level }
copy-holdings-watch-time = Tempo ≤ { $level }
copy-holdings-watch-time-until = Tempo ≤ { $level } em { $span }
copy-holdings-watch-wallet-sells = Vendas da carteira
copy-holdings-empty = Nenhuma posse aberta em Paper. As compras copiadas da carteira aparecem aqui.
copy-holdings-col-token = Token
copy-holdings-col-cost = Custo
copy-holdings-col-entry = Entrada
copy-holdings-col-mark = Marcação
copy-holdings-col-peak = Pico
copy-holdings-col-pnl = P&L
copy-holdings-col-exit-rules = Regras de saída
copy-holdings-col-held = Em posse
copy-holdings-col-actions = Ações
copy-holdings-col-invested = Investido
copy-holdings-col-proceeds = Recebido
copy-holdings-col-exit = Saída
copy-holdings-col-closed = Fechada
copy-holdings-price-note = Os preços são em SOL por token. A entrada inclui o slippage e as taxas da compra; o pico e os níveis de saída são relativos a ela, então uma posse abre com o pico abaixo da entrada. Passe o mouse sobre um valor para ver o preço do pool.
copy-holdings-paused-rules = Pausada: sem novas cópias. Suas regras de saída ainda fecham estas posses.
copy-holdings-paused-mirror = Pausada: sem novas cópias. As vendas da carteira ainda fecham estas posses.
copy-holdings-paused-hybrid = Pausada: sem novas cópias. As vendas da carteira e suas regras de saída ainda fecham estas posses.
copy-holdings-closed-load-failed = Não foi possível carregar as rodadas fechadas: { $error }
copy-holdings-closed-loading = Carregando rodadas fechadas…
copy-holdings-closed-empty = Nenhuma rodada fechada ainda.
copy-holdings-closed-latest = Últimas { $shown } de { $total } rodadas.
copy-holdings-close-title = Fechar posse em Paper
copy-holdings-close-message = Vender { $token } no livro de Paper ao preço do pool ({ $price }), com o slippage e as taxas da tarefa.
copy-holdings-close-confirm = Fechar posse
copy-holdings-write-off-title = Baixar posse em Paper
copy-holdings-write-off-message = { $token } não tem preço de pool para vender. Baixar fecha a posse em zero e registra o custo de { $cost } como perda.
copy-holdings-keep = Manter
copy-holdings-written-off = { $token } baixado
copy-holdings-closed = { $token } fechado
copy-holdings-written-off-detail = Fechada com recebimento zero
copy-holdings-sold-at = Vendido a { $price }
copy-holdings-close-failed = Não foi possível fechar a posse
copy-holdings-reset-message = Recomeçar “{ $name }”: as posses, o gasto, as execuções, as saídas e as ignoradas em Paper serão removidos. As regras e a carteira permanecem.
copy-holdings-reset-cancel = Manter histórico
copy-holdings-reset-done = Livro de Paper redefinido
copy-holdings-reset-detail =
    { $count ->
        [one] { $count } decisão removida
        [many] { $count } decisões removidas
       *[other] { $count } decisões removidas
    }
copy-holdings-reset-failed = Não foi possível redefinir o livro de Paper

copy-activity-title = Atividade
copy-activity-filter-label = Filtro de atividade
copy-filter-all = Todas
copy-outcome-paper-filled = Compra em Paper
copy-outcome-live-submitted = Compra real enviada
copy-outcome-live-confirmed = Compra real confirmada
copy-outcome-live-failed = Compra real com falha
copy-outcome-paper-sell-observed = Venda em Paper · a carteira vendeu
copy-outcome-live-sell-submitted = Venda real enviada
copy-outcome-live-sell-failed = Venda real com falha
copy-outcome-skipped = Ignorada
copy-activity-decision = Decisão
copy-activity-paper-exit = Saída em Paper · { $rule }
copy-activity-filled = { $input } a { $price } · a carteira comprou { $target }
copy-activity-filled-slippage = { $input } a { $price } · a carteira comprou { $target } · slippage { $slippage }
copy-activity-filled-unpriced = { $input } a { $price } · precificada no trade da carteira, sem preço de pool
copy-activity-live-sized = { $sized } · a carteira comprou { $target }
copy-activity-sell-nothing = A carteira vendeu { $amount } · nada em posse para vender
copy-activity-written-off = Baixada em zero: sem preço de pool
copy-activity-sold = { $tokens } tokens por { $proceeds } a { $price }
copy-activity-full-close = Fechamento total
copy-activity-partial-exit = Saída de { $pct }
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = mínimo de { $amount }
copy-activity-skip-maximum = máximo de { $value }
copy-activity-skip-stale = { $arrival } de atraso, limite de { $limit }
copy-activity-skip-latency = média de { $average }, limite de { $limit }
copy-activity-arrival-replayed = Reprocessado { $span } após o bloco
copy-activity-arrival-seen = Visto { $span } após o bloco
copy-activity-link-wallet-tx = Tx da carteira
copy-activity-link-own-tx = Sua tx
copy-activity-only-token = Somente este token
copy-activity-skipped-group = Ignoradas ×{ $count }
copy-activity-group-detail =
    { $tokens ->
        [one] { $tokens } token · desde { $since }
        [many] { $tokens } tokens · desde { $since }
       *[other] { $tokens } tokens · desde { $since }
    }
copy-activity-mint-filter =
    .placeholder = Mint do token
    .aria-label = Filtrar por mint do token
copy-activity-clear = Limpar
copy-activity-load-failed = Não foi possível carregar a atividade: { $error }
copy-activity-loading = Carregando atividade…
copy-activity-no-match = Nada corresponde a este filtro.
copy-activity-empty = Nenhuma decisão ainda. Execuções, saídas e ignoradas aparecem aqui conforme a carteira opera.
copy-activity-load-older = Carregar anteriores
copy-activity-start = Início do histórico
copy-activity-older-failed = Não foi possível carregar a atividade anterior

copy-step-wallet = Carteira
copy-step-sizing = Dimensionamento
copy-step-entry = Filtros de entrada
copy-step-exits = Saídas
copy-step-review = Revisão
copy-editor-title-edit = Editar { $name }
copy-editor-title-clone = Clonar { $name }
copy-editor-sub-edit = Tarefa { $mode } · as alterações valem para as próximas decisões
copy-editor-sub-clone = Mesmas regras, livro de Paper vazio, começa em Paper
copy-editor-save-edit = Salvar alterações
copy-editor-save-clone = Criar clone
copy-editor-save-create = Criar tarefa em Paper
copy-editor-clone-suffix = (cópia)
copy-editor-discard-edit = Descartar alterações
copy-editor-discard-create = Descartar esta tarefa
copy-editor-discard-edit-message = Suas alterações em “{ $name }” não foram salvas.
copy-editor-discard-create-message = A carteira e as regras informadas até agora não foram salvas.
copy-editor-discard-confirm = Descartar
copy-editor-keep-editing = Continuar editando
copy-editor-toast-updated = Tarefa atualizada
copy-editor-toast-clone = Clone criado
copy-editor-toast-created = Tarefa em Paper criada
copy-unit-native = { -sol }
copy-editor-any = Qualquer
copy-editor-duplicate = Já copiada por { $tasks }. Esta tarefa copia os mesmos trades de novo, com regras e orçamento próprios.
copy-editor-wallet = Carteira
copy-editor-wallet-identity = A carteira de uma tarefa é a sua identidade. Para copiar outra carteira com estas regras, clone a tarefa.
copy-editor-address-label = Endereço da carteira
copy-editor-address-placeholder = Endereço de carteira Solana
copy-editor-address-help-clone = Mesmas regras com o livro de Paper vazio. Mantenha esta carteira para testar outras regras nela ou informe outra.
copy-editor-address-help-create = A carteira cujas compras (e, se você escolher, vendas) esta tarefa copia.
copy-editor-name-label = Nome <em>opcional</em>
copy-editor-name-placeholder = ex.: Rotador rápido
copy-editor-enabled-title = Processar os trades da carteira
copy-editor-enabled-help = Desativado mantém a tarefa pausada até você retomá-la.
copy-editor-note-live = Esta tarefa opera em modo real: as alterações valem para as próximas cópias reais.
copy-editor-note-paper = As tarefas rodam em Paper até você armá-las: os trades são simulados ao preço do pool e nada é gasto.
copy-editor-copy-size = Tamanho da cópia
copy-editor-sizing-fixed = Valor fixo
copy-editor-sizing-ratio = Parte do trade da carteira
copy-editor-amount-fixed = Valor por cópia
copy-editor-amount-ratio = Parte de cada trade
copy-editor-amount-help-fixed = Gasto em cada compra copiada, no mínimo { $minimum }.
copy-editor-amount-help-ratio = Da compra da própria carteira, até o limite por trade.
copy-editor-help-trade-cap = Nenhuma cópia individual gasta mais que isso.
copy-editor-help-token-cap = Total gasto em um token.
copy-editor-help-budget = Tudo o que esta tarefa pode gastar ao longo da vida; Paper e modo real contam cada um o próprio gasto.
copy-editor-preview-title = Quanto custa uma cópia
copy-editor-preview-empty = Informe o dimensionamento para ver quanto custa uma cópia.
copy-editor-preview-example = A carteira compra { $target } → você copia <strong>{ $copy }</strong>
copy-editor-preview-once = Um token recebe uma única cópia de { $size }, pois cada token é comprado uma vez
copy-editor-preview-token-cap =
    { $count ->
        [one] Um token recebe no máximo { $count } cópia de { $size }
        [many] Um token recebe no máximo { $count } cópias de { $size }
       *[other] Um token recebe no máximo { $count } cópias de { $size }
    }
copy-editor-preview-summary-exact = { $perToken }; o orçamento cobre cerca de { $count } delas. Taxas de rede e de prioridade são adicionais.
copy-editor-preview-summary-minimum = { $perToken }; o orçamento cobre pelo menos { $count } delas. Taxas de rede e de prioridade são adicionais.
copy-editor-target-min = Menor trade da carteira copiado
copy-editor-target-min-help = Ignora as compras menores da carteira. Deixe vazio para não ter mínimo.
copy-editor-target-max = Maior trade da carteira copiado
copy-editor-target-max-help = Ignora as compras maiores da carteira. Deixe vazio para não ter máximo.
copy-editor-buy-once-title = Comprar cada token uma vez
copy-editor-buy-once-help = Copia apenas a primeira compra de um token pela carteira; as compras seguintes dele são ignoradas.
copy-editor-filter-require = Exigir
copy-editor-filter-skip = Não exigir
copy-editor-filter-help = Exige que o token passe pelo seu pipeline de filtragem antes de ser copiado.
copy-editor-filter-warning = Com a configuração de filtragem padrão, quase todos os tokens falham, então uma tarefa que exige aprovação não copia nada. Exija somente quando seus filtros aprovarem os tokens que esta carteira negocia.
copy-editor-exit-both = Ambos
copy-editor-exit-help-buy-only = Suas regras abaixo vendem todas as posses; as vendas da carteira são ignoradas.
copy-editor-exit-help-hybrid = O que vier primeiro: a carteira vende ou uma das suas regras dispara.
copy-editor-exit-help-mirror = As posses só são vendidas quando a carteira vende. Suas regras de saída não rodam.
copy-editor-who-sells = Quem vende
copy-editor-preset = Predefinição
copy-editor-preset-help = Uma predefinição preenche todas as regras abaixo; ajuste qualquer uma depois.
copy-editor-mirror-note = Estas regras não rodam enquanto as vendas da carteira decidem. Elas valem se você mudar para { $mine } ou { $both }.
copy-editor-rule-inherit = Padrão do Trader
copy-editor-inherit-value = Padrão do Trader ({ $value })
copy-editor-rule-aria = Configuração de { $rule }
copy-editor-rule-empty-uses = Vazio usa o padrão do Trader: { $value }
copy-editor-rule-follows = Segue o Trader: { $summary }
copy-editor-rule-follows-plain = Segue a configuração do Trader.
copy-editor-rule-off-note = Desativada nesta tarefa, independentemente do que o Trader usa.
copy-editor-unnamed = Tarefa sem nome
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = processa trades depois de salva
copy-editor-review-paused = salva pausada
copy-editor-error-address = Informe um endereço de carteira Solana válido.
copy-editor-error-sizing = Todo valor de dimensionamento deve ser maior que zero.
copy-editor-error-min-copy = Uma cópia deve ser de pelo menos { $minimum }: aumente o valor por cópia.
copy-editor-error-min-cap = Uma cópia deve ser de pelo menos { $minimum }: aumente o limite por trade.
copy-editor-error-trade-cap = O limite por trade não pode exceder o limite por token.
copy-editor-error-token-cap = O limite por token não pode exceder o orçamento total.
copy-editor-error-slippage = O slippage deve estar entre { $min } e { $max }.
copy-editor-error-target-limits = Os limites de trade da carteira devem ser zero ou mais.
copy-editor-error-target-order = O menor trade da carteira não pode exceder o maior.

copy-notice-task-unnamed = Tarefa #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = Compra copiada em Paper
copy-notice-title-paper-sell = Venda copiada em Paper
copy-notice-title-paper-closed = Posse em Paper fechada
copy-notice-title-paper-exit = Saída em Paper: { $rule }
copy-notice-title-live-buy-submitted = Compra copiada real enviada
copy-notice-title-live-buy-confirmed = Compra copiada real confirmada
copy-notice-title-live-buy-failed = Compra copiada real com falha
copy-notice-title-live-sell-submitted = Venda copiada real enviada
copy-notice-title-live-sell-failed = Venda copiada real com falha
copy-notice-title-auto-paused = Tarefa de cópia pausada automaticamente
copy-notice-detail-bought = Comprado por { $amount } { -sol }
copy-notice-detail-sold = Vendido por { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = { $percent }% da posse
copy-notice-detail-full-close = Fechamento total
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = Falha no swap
