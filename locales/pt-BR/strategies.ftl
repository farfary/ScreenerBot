# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = Todas
strategies-filter-entry = Entrada
strategies-filter-exit = Saída
strategies-type-entry = Entrada
strategies-type-exit = Saída
strategies-list-empty-title = Nenhuma estratégia ainda
strategies-list-empty-hint = Crie sua primeira estratégia
strategies-new = Nova estratégia
strategies-import =
    .title = Importar estratégia
    .aria-label = Importar estratégia
strategies-item-enable =
    .title = Ativar
strategies-item-disable =
    .title = Desativar

# Name given to a strategy before it is saved.
strategies-new-name = Nova estratégia

## Editor

strategies-editor-name =
    .placeholder = Nome da estratégia
strategies-editor-dirty =
    .title = Alterações não salvas
strategies-action-validate = Validar
strategies-editor-empty = Selecione uma estratégia para editar ou crie uma nova
strategies-conditions-empty-title = Nenhuma condição ainda
strategies-conditions-empty-hint = Use "{ strategies-add-condition }" para começar a montar
strategies-add-condition = Adicionar condição
strategies-modal-close =
    .aria-label = Fechar
strategies-card-move-up =
    .title = Mover para cima
strategies-card-move-down =
    .title = Mover para baixo
strategies-card-duplicate =
    .title = Duplicar
strategies-card-delete =
    .title = Excluir
# $name is the condition name.
strategies-card-delete-confirm = Remover condição
    .message = Remover "{ $name }" desta estratégia?

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = Sem parâmetros
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = Configuração da estratégia ({ $value })
strategies-summary-period-seconds = Janela de análise: { $amount } s
strategies-summary-period-minutes = Janela de análise: { $amount } min
strategies-summary-period-hours = Janela de análise: { $amount } h

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
        [one] { $amount } hora
        [many] { $amount } horas
       *[other] { $amount } horas
    }
strategies-value-candles =
    { $count ->
        [one] { $amount } candle
        [many] { $amount } candles
       *[other] { $amount } candles
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = h
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = Buscar condições...
    .aria-label = Buscar condições
strategies-catalog-search-clear =
    .aria-label = Limpar busca
strategies-catalog-no-description = Nenhuma descrição disponível

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = Ativa
strategies-editor-timeframe = Timeframe padrão
    .title = Timeframe dos candles para as condições que não definem o seu
strategies-editor-priority = Prioridade
    .title = Ordem de avaliação de 1 a 100; números menores são avaliados primeiro
strategies-editor-description = Descrição
strategies-editor-description-field =
    .placeholder = O que esta estratégia faz
    .aria-label = Descrição da estratégia
strategies-action-export = Exportar

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = Condições que precisam ser atendidas: { $mode }
strategies-group-match = Precisam ser atendidas: { $mode }
strategies-match-all = todas
strategies-match-any = pelo menos uma
strategies-match-mode =
    .aria-label = Todas ou pelo menos uma condição
strategies-invert = Inverter
    .title = Inverter: verdadeiro quando isto não corresponde
strategies-chip-off = Desativado
strategies-chip-inverted = Invertido
strategies-row-switch =
    .title = Desative para manter salvo sem avaliar
    .aria-label = Avaliado por esta estratégia
strategies-row-expand =
    .aria-label = Mostrar ou ocultar os parâmetros
strategies-add-group = Adicionar grupo
# $count is the number of conditions in the group.
strategies-group-delete-confirm = Remover grupo
    .message =
        { $count ->
            [one] Remover este grupo e a condição dele?
            [many] Remover este grupo e as { $count } condições dele?
           *[other] Remover este grupo e as { $count } condições dele?
        }

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems =
    { $count ->
        [one] { $count } problema para corrigir: { $first }
        [many] { $count } problemas para corrigir: { $first }
       *[other] { $count } problemas para corrigir: { $first }
    }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = Adicione pelo menos uma condição ativada
strategies-problem-group-empty = Um grupo precisa de pelo menos uma condição ativada
strategies-problem-not-applicable = Esta condição não pode ser usada neste tipo de estratégia
strategies-problem-unknown-condition = Tipo de condição desconhecido: { $type }
strategies-problem-too-deep = As regras aceitam no máximo { $max } níveis de aninhamento; um grupo invertido conta como um nível a mais
strategies-problem-required = Informe um número
strategies-problem-range = Deve estar entre { $min } e { $max }
strategies-problem-invalid-option = Escolha uma das opções disponíveis

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = Todas
strategies-catalog-categories =
    .aria-label = Categorias
strategies-catalog-empty = Nenhuma condição corresponde à busca
strategies-catalog-exit-only = Somente estratégias de saída
strategies-catalog-entry-only = Somente estratégias de entrada
strategies-catalog-candles-note = { $description } Lê apenas candles.
strategies-data-candles = Candles
strategies-data-pool = Pool
strategies-data-position = Posição
strategies-data-market = Atividade do mercado

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = trades
strategies-value-trades =
    { $count ->
        [one] { $amount } trade
        [many] { $amount } trades
       *[other] { $amount } trades
    }

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = Níveis demais
    .message = As regras aceitam no máximo { $max } níveis de aninhamento
strategies-toast-priority-invalid = Prioridade inválida
    .message = Informe um número inteiro de 1 a 100
strategies-toast-exported = Estratégia exportada
# $name is the strategy name.
strategies-unsaved-confirm = Descartar as alterações não salvas?
    .message = "{ $name }" tem alterações que não foram salvas.
strategies-unsaved-discard = Descartar
strategies-import-error-json = O arquivo não é um JSON válido
strategies-import-error-shape = O arquivo não descreve uma estratégia
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = O arquivo usa uma condição que esta versão não tem: { $type }
strategies-import-error-not-applicable = { $name } não pode ser usada neste tipo de estratégia

## New strategy dialog

strategies-create-title = Criar nova estratégia
strategies-create-prompt = Escolha o tipo de estratégia que deseja criar:
strategies-create-entry-name = Estratégia de entrada
strategies-create-entry-description = Defina as condições para comprar um token
strategies-create-exit-name = Estratégia de saída
strategies-create-exit-description = Defina as condições para vender um token

## Delete dialog

strategies-delete-title = Excluir estratégia
# $name is the strategy name.
strategies-delete-message = Excluir a estratégia "{ $name }"? Esta ação não pode ser desfeita.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Corrija os erros de validação antes de salvar
strategies-toast-load-failed = Falha ao carregar
    .message = Falha ao carregar as estratégias do servidor
strategies-toast-load-strategy-failed = Falha ao carregar a estratégia
strategies-toast-no-strategy = Nenhuma estratégia criada
    .message = Adicione ao menos uma condição ou clique em "Nova estratégia" para criar uma estratégia primeiro
strategies-toast-name-required = Nome obrigatório
    .message = Informe um nome para a estratégia antes de salvar
strategies-toast-saved = Estratégia salva
    .message = "{ $name }" salva com sucesso
strategies-toast-save-failed = Falha ao salvar
    .message = Falha ao salvar a estratégia no banco de dados
strategies-toast-no-strategy-validate = Nenhuma estratégia para validar
strategies-toast-valid = A estratégia é válida
strategies-toast-invalid = A estratégia tem erros
strategies-toast-validation-failed = Falha na validação
strategies-toast-item-enabled = Estratégia ativada
strategies-toast-item-disabled = Estratégia desativada
strategies-toast-item-toggle-failed = Falha ao alternar a estratégia
strategies-toast-deleted = Estratégia excluída
    .message = "{ $name }" removida com sucesso
strategies-toast-delete-failed = Falha ao excluir
    .message = Falha ao excluir a estratégia do banco de dados
strategies-toast-imported = Estratégia importada
strategies-toast-import-failed = Falha ao importar a estratégia
strategies-toast-unknown-condition = Condição desconhecida
    .message = Tipo de condição não encontrado
strategies-toast-create-first = Crie uma estratégia primeiro
    .message = Clique em "Nova estratégia" para criar uma estratégia antes de adicionar condições

## Conditions

strategies-condition-candle-size = Padrão de tamanho do candle
    .description = Detecta padrões específicos de candle: corpo grande, corpo pequeno (doji), pavios longos.
strategies-condition-candle-size-param-pattern = Tipo de padrão
    .description = Padrão de candle a detectar
strategies-condition-candle-size-param-pattern-option-large-body = Corpo grande (movimento forte)
strategies-condition-candle-size-param-pattern-option-small-body = Corpo pequeno (doji/indecisão)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Pavio superior longo (rejeição)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Pavio inferior longo (suporte)
strategies-condition-candle-size-param-threshold = Limite de tamanho %
    .description = Limite percentual para detecção do padrão

strategies-condition-consecutive-candles = Candles consecutivos
    .description = Detecta candles verdes (de alta) ou vermelhos (de baixa) consecutivos, com filtro de tamanho mínimo.
strategies-condition-consecutive-candles-param-count = Quantidade de candles
    .description = Número de candles consecutivos exigido
strategies-condition-consecutive-candles-param-direction = Direção do candle
    .description = Cor/direção dos candles consecutivos
strategies-condition-consecutive-candles-param-direction-option-green = Verde (alta)
strategies-condition-consecutive-candles-param-direction-option-red = Vermelho (baixa)
strategies-condition-consecutive-candles-param-minimum-change = Variação mínima %
    .description = Variação mínima em % de cada candle (filtra ruído)

strategies-condition-liquidity-level = Nível de liquidez do pool
    .description = Verifica a liquidez do pool em { -sol } (Entrada: garantir liquidez suficiente; Saída: detectar drenagem de liquidez)
strategies-condition-liquidity-level-param-threshold = Limite de liquidez ({ -sol })
    .description = Nível de liquidez do pool em { -sol }
strategies-condition-liquidity-level-param-comparison = Comparação
    .description = Como comparar a liquidez do pool com o limite
strategies-condition-liquidity-level-param-comparison-option-greater-than = Maior que (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = Maior ou igual (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = Menor que ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = Menor ou igual (≤)

strategies-condition-position-holding-time = Tempo de posse da posição
    .description = Verifica há quanto tempo uma posição é mantida
strategies-condition-position-holding-time-param-hours = Limite de tempo (horas)
    .description = Duração em horas desde a abertura da posição
strategies-condition-position-holding-time-param-comparison = Comparação
    .description = Como comparar a idade da posição com o limite
strategies-condition-position-holding-time-param-comparison-option-greater-than = Mais antiga que (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = No mínimo (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Mais recente que ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = No máximo (≤)

strategies-condition-price-breakout = Rompimento de preço
    .description = Detecta o preço rompendo acima da resistência (máxima do período) ou abaixo do suporte (mínima do período).
strategies-condition-price-breakout-param-lookback = Janela de análise
    .description = Número de candles para encontrar o nível de suporte/resistência
strategies-condition-price-breakout-param-direction = Direção do rompimento
    .description = Direção do rompimento
strategies-condition-price-breakout-param-direction-option-upward = Para cima (rompe resistência)
strategies-condition-price-breakout-param-direction-option-downward = Para baixo (rompe suporte)
strategies-condition-price-breakout-param-confirmation = Confirmação %
    .description = Quanto além do nível para confirmar o rompimento (evita sinais falsos)

strategies-condition-price-change-percent = Variação de preço %
    .description = Verifica se o preço variou por um limite percentual dentro de um período.
strategies-condition-price-change-percent-param-percentage = Limite de variação %
    .description = Variação percentual de preço que aciona a condição (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Direção
    .description = Direção do movimento do preço
strategies-condition-price-change-percent-param-direction-option-above = Alta (+%)
strategies-condition-price-change-percent-param-direction-option-below = Queda (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dentro da faixa (±%)
strategies-condition-price-change-percent-param-time-value = Janela de análise
    .description = Até quando a variação é medida, na unidade escolhida dentro do campo (1-3600 s, 1-1440 min, 1-720 h)
strategies-condition-price-change-percent-param-time-unit = Unidade de tempo
    .description = Unidade de tempo da janela de análise
strategies-condition-price-change-percent-param-time-unit-option-seconds = s
strategies-condition-price-change-percent-param-time-unit-option-minutes = min
strategies-condition-price-change-percent-param-time-unit-option-hours = h

strategies-condition-price-to-ma = Preço vs média móvel
    .description = Verifica se o preço está acima, abaixo ou dentro da faixa da sua média móvel simples.
strategies-condition-price-to-ma-param-period = Período da MM
    .description = Número de candles para o cálculo da média móvel
strategies-condition-price-to-ma-param-position = Posição
    .description = Posição do preço em relação à MM
strategies-condition-price-to-ma-param-position-option-above = Acima da MM
strategies-condition-price-to-ma-param-position-option-below = Abaixo da MM
strategies-condition-price-to-ma-param-position-option-within = Dentro da faixa
strategies-condition-price-to-ma-param-distance = Distância %
    .description = Distância mínima da MM (para ACIMA/ABAIXO) ou faixa máxima (para DENTRO)

strategies-condition-volume-spike = Pico de volume
    .description = Detecta picos de volume em relação ao volume médio (indica maior interesse).
strategies-condition-volume-spike-param-lookback = Janela de análise
    .description = Número de candles para calcular o volume médio
strategies-condition-volume-spike-param-multiplier = Multiplicador de volume
    .description = Quantas vezes acima da média (ex.: 2.0 = 200% da média)

strategies-condition-rsi = RSI (Índice de Força Relativa)
    .description = Dispara quando o RSI, com a suavização de Wilder sobre candles fechados, está acima ou abaixo de um limite.
strategies-condition-rsi-param-period = Período do RSI
    .description = Número de candles sobre os quais o RSI é calculado; a condição precisa de um candle a mais que este valor
strategies-condition-rsi-param-comparison = Comparação
    .description = Se o RSI deve estar abaixo ou acima do limite
strategies-condition-rsi-param-comparison-option-below = Abaixo (sobrevendido)
strategies-condition-rsi-param-comparison-option-above = Acima (sobrecomprado)
strategies-condition-rsi-param-threshold = Limite do RSI
    .description = Nível de RSI de 1 a 99 (30 e 70 são os níveis usuais de sobrevenda e sobrecompra)

strategies-condition-ma-cross = Cruzamento de médias móveis
    .description = Dispara quando uma média móvel rápida cruza uma lenta dentro dos últimos candles.
strategies-condition-ma-cross-param-ma-type = Tipo de média
    .description = Médias exponenciais dão mais peso aos candles recentes; médias simples pesam todos os candles igualmente
strategies-condition-ma-cross-param-ma-type-option-ema = Exponencial (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = Simples (SMA)
strategies-condition-ma-cross-param-fast-period = Período rápido
    .description = Número de candles da média rápida
strategies-condition-ma-cross-param-slow-period = Período lento
    .description = Número de candles da média lenta; deve ser maior que o período rápido
strategies-condition-ma-cross-param-direction = Direção do cruzamento
    .description = Em que sentido a média rápida cruza a lenta
strategies-condition-ma-cross-param-direction-option-bullish = Altista (a rápida cruza para cima)
strategies-condition-ma-cross-param-direction-option-bearish = Baixista (a rápida cruza para baixo)
strategies-condition-ma-cross-param-within-candles = Cruzamento nos últimos candles
    .description = Quantos dos últimos candles podem conter o cruzamento; 1 significa apenas o último candle

strategies-condition-position-profit = Lucro da posição
    .description = Dispara quando o lucro não realizado da posição aberta está acima ou abaixo de uma porcentagem do preço de entrada, medido pelo preço do pool.
strategies-condition-position-profit-param-comparison = Comparação
    .description = Se o lucro deve ser no mínimo ou no máximo o limite
strategies-condition-position-profit-param-comparison-option-above = No mínimo (≥)
strategies-condition-position-profit-param-comparison-option-below = No máximo (≤)
strategies-condition-position-profit-param-threshold-pct = Limite de lucro %
    .description = Lucro não realizado como porcentagem do preço de entrada; valores negativos são prejuízos

strategies-condition-drawdown-from-peak = Queda desde o pico
    .description = Dispara quando o preço do pool caiu uma porcentagem desde o maior preço visto após a última compra, depois que esse pico esteve alto o bastante acima do preço de entrada.
strategies-condition-drawdown-from-peak-param-drop-pct = Queda desde o pico %
    .description = Quanto abaixo do pico o preço atual precisa estar
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = Armar após lucro %
    .description = O pico precisa ter ficado pelo menos esta porcentagem acima do preço de entrada antes de a condição poder disparar; 0 significa sempre armada

strategies-condition-buy-pressure = Pressão de compra
    .description = Dispara quando as compras representam pelo menos uma parcela das transações de uma janela de tempo e a janela tem transações suficientes. Lê as contagens de compras e vendas do token.
strategies-condition-buy-pressure-param-window = Janela de tempo
    .description = Período coberto pelas contagens de compras e vendas
strategies-condition-buy-pressure-param-window-option-5m = 5 minutos
strategies-condition-buy-pressure-param-window-option-1h = 1 hora
strategies-condition-buy-pressure-param-window-option-6h = 6 horas
strategies-condition-buy-pressure-param-window-option-24h = 24 horas
strategies-condition-buy-pressure-param-min-buy-share-pct = Parcela mínima de compras %
    .description = Compras como porcentagem de todas as transações da janela
strategies-condition-buy-pressure-param-min-transactions = Transações mínimas
    .description = Janelas com menos transações nunca disparam a condição

## Shared by every condition

strategies-condition-param-timeframe = Timeframe
    .description = Tamanho do candle: a duração de cada candle que a condição lê (o timeframe da estratégia se não definido)
strategies-condition-timeframe-option-1m = 1 minuto
strategies-condition-timeframe-option-5m = 5 minutos
strategies-condition-timeframe-option-15m = 15 minutos
strategies-condition-timeframe-option-1h = 1 hora
strategies-condition-timeframe-option-4h = 4 horas
strategies-condition-timeframe-option-12h = 12 horas
strategies-condition-timeframe-option-1d = 1 dia

## Condition categories

strategies-condition-category-price-analysis = Análise de preço
strategies-condition-category-candle-patterns = Padrões de candle
strategies-condition-category-technical-indicators = Indicadores técnicos
strategies-condition-category-market-context = Contexto de mercado
strategies-condition-category-position-performance = Posição e desempenho
strategies-condition-category-volume-analysis = Análise de volume

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = O parâmetro { $field } está ausente
strategies-error-parameter-type = O parâmetro { $field } deve ser { $expected }
strategies-error-invalid-value = "{ $value }" não é um valor válido para { $field }
strategies-error-missing-data = { $data } não está disponível
strategies-error-no-candle-data = O timeframe { $timeframe } não tem dados de candle
strategies-error-insufficient-history = Histórico insuficiente para { $indicator }: { $available } s disponíveis, { $required } s necessários
strategies-error-insufficient-candles = Candles insuficientes para { $indicator }: há { $available }, são necessários { $required }
strategies-error-stale-candle-data = Os dados de candle de { $timeframe } estão desatualizados: a idade de { $age } s excede { $max } s
strategies-error-stale-market-data = As contagens de transações de { $window } estão desatualizadas: a idade de { $age } s excede { $max } s
strategies-error-unusable-data = O valor "{ $value }" de { $field } não pode ser usado
strategies-error-invalid-rule-tree = Árvore de regras inválida: { $reason }
strategies-error-evaluation-timeout = A avaliação da estratégia expirou após { $timeout } ms
strategies-error-condition-not-applicable = { $condition } não pode ser usado em estratégias do tipo { $strategy }
strategies-error-invalid-rules = Não foi possível ler as regras: { $reason }

# Parameter names

strategies-error-field-average-volume = volume médio
strategies-error-field-candle-open = abertura do candle
strategies-error-field-comparison = comparação
strategies-error-field-condition-type = tipo de condição
strategies-error-field-confirmation = confirmação
strategies-error-field-count = quantidade
strategies-error-field-current-price = preço atual
strategies-error-field-direction = direção
strategies-error-field-distance = distância
strategies-error-field-hours = horas
strategies-error-field-lookback = janela de análise
strategies-error-field-minimum-change = variação mínima
strategies-error-field-multiplier = multiplicador
strategies-error-field-pattern = padrão
strategies-error-field-percentage = porcentagem
strategies-error-field-period = período
strategies-error-field-position = posição
strategies-error-field-threshold = limite
strategies-error-field-time-unit = unidade de tempo
strategies-error-field-time-value = valor de tempo
strategies-error-field-timeframe = timeframe
strategies-error-field-arm-after-profit = armar após lucro
strategies-error-field-drop-percentage = porcentagem de queda
strategies-error-field-entry-price = preço de entrada
strategies-error-field-fast-period = período rápido
strategies-error-field-minimum-buy-share = parcela mínima de compras
strategies-error-field-minimum-transactions = transações mínimas
strategies-error-field-moving-average-type = tipo de média
strategies-error-field-moving-average = média móvel
strategies-error-field-reference-candle-close = fechamento do candle de referência
strategies-error-field-peak-price = preço de pico
strategies-error-field-slow-period = período lento
strategies-error-field-window = janela
strategies-error-field-within-candles = candles do cruzamento

# Expected parameter types

strategies-error-expected-boolean = um booleano
strategies-error-expected-number = um número
strategies-error-expected-string = um texto

# Missing context data

strategies-error-data-current-price = Preço atual
strategies-error-data-liquidity-data = Dados de liquidez
strategies-error-data-market-data = Dados de mercado
strategies-error-data-ohlcv-data = Dados OHLCV
strategies-error-data-position-data = Dados da posição
strategies-error-data-peak-price = Preço de pico
strategies-error-data-transaction-counts = Contagens de transações

# Indicators

strategies-error-indicator-consecutive-candles = candles consecutivos
strategies-error-indicator-moving-average = média móvel
strategies-error-indicator-price-breakout = rompimento de preço
strategies-error-indicator-price-change-lookback = janela de variação de preço
strategies-error-indicator-volume-spike = pico de volume
strategies-error-indicator-moving-average-cross = cruzamento de médias móveis
strategies-error-indicator-rsi = RSI

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Nó de ramificação sem condições
strategies-error-rule-branch-node-missing-operator = Nó de ramificação sem operador
strategies-error-rule-branch-node-must-have-at-least-one-child = O nó de ramificação deve ter ao menos um filho
strategies-error-rule-invalid-rule-tree-structure = Estrutura de árvore de regras inválida
strategies-error-rule-leaf-node-missing-condition = Nó folha sem condição
strategies-error-rule-not-operator-must-have-exactly-one-child = O operador NOT deve ter exatamente um filho
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = Um grupo precisa de pelo menos uma condição ativada
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = O operador NOT não pode ter um filho desativado
strategies-error-rule-rules-nest-deeper-than-four-levels = As regras não podem se aninhar em mais de quatro níveis
strategies-error-rule-the-root-group-cannot-be-switched-off = O grupo raiz não pode ser desativado
