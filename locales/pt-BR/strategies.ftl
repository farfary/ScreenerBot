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

# Card summary: up to three "label: value" entries.
strategies-summary-param = { $label }: { $value }
strategies-summary-parts =
    { $count ->
        [1] { $first }
        [2] { $first }, { $second }
       *[3] { $first }, { $second }, { $third }
    }
strategies-summary-none = Sem parâmetros
strategies-summary-period-seconds = Período: { $amount } s
strategies-summary-period-minutes = Período: { $amount } min
strategies-summary-period-hours = Período: { $amount } h

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
strategies-catalog-search-clear =
    .aria-label = Limpar busca
strategies-catalog-fold-all = Recolher tudo
strategies-catalog-unfold-all = Expandir tudo
strategies-catalog-no-description = Nenhuma descrição disponível

## New strategy dialog

strategies-create-title = Criar nova estratégia
strategies-create-prompt = Escolha o tipo de estratégia que deseja criar:
strategies-create-entry-name = Estratégia de entrada
strategies-create-entry-description = Defina as condições para COMPRAR um token
strategies-create-exit-name = Estratégia de saída
strategies-create-exit-description = Defina as condições para VENDER um token

## Delete dialog

strategies-delete-title = Excluir estratégia
# $name is the strategy name.
strategies-delete-message = Excluir a estratégia "{ $name }"? Esta ação não pode ser desfeita.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = Corrija os erros de validação antes de salvar
strategies-toast-enabled = Estratégia ativada
    .message = "{ $name }" ativada
strategies-toast-disabled = Estratégia desativada
    .message = "{ $name }" desativada
strategies-toast-toggle-failed = Falha ao alternar
    .message = Falha ao atualizar o status da estratégia
strategies-toast-load-failed = Falha ao carregar
    .message = Falha ao carregar as estratégias do servidor
strategies-toast-created = Nova estratégia
    .message =
        { $type ->
            [EXIT] Nova estratégia de saída criada
           *[ENTRY] Nova estratégia de entrada criada
        }
strategies-toast-load-strategy-failed = Falha ao carregar a estratégia
strategies-toast-no-strategy = Nenhuma estratégia criada
    .message = Adicione ao menos uma condição ou clique em "Nova estratégia" para criar uma estratégia primeiro
strategies-toast-no-conditions-save = Sem condições
    .message = Adicione ao menos uma condição à estratégia antes de salvar
strategies-toast-name-required = Nome obrigatório
    .message = Informe um nome para a estratégia antes de salvar
strategies-toast-saved = Estratégia salva
    .message = "{ $name }" salva com sucesso
strategies-toast-save-failed = Falha ao salvar
    .message = Falha ao salvar a estratégia no banco de dados
strategies-toast-no-strategy-validate = Nenhuma estratégia para validar
strategies-toast-no-conditions-validate = Sem condições
    .message = Adicione ao menos uma condição antes de validar
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
strategies-toast-condition-added = Condição adicionada
    .message = { $name } adicionada à estratégia


## Conditions

strategies-condition-candle-size = Padrão de tamanho do candle
    .description = Detecta padrões específicos de candle: corpo grande, corpo pequeno (doji), pavios longos
strategies-condition-candle-size-param-pattern = Tipo de padrão
    .description = Padrão de candle a detectar
strategies-condition-candle-size-param-pattern-option-large-body = Corpo grande (movimento forte)
strategies-condition-candle-size-param-pattern-option-small-body = Corpo pequeno (doji/indecisão)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = Pavio superior longo (rejeição)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = Pavio inferior longo (suporte)
strategies-condition-candle-size-param-threshold = Limite de tamanho %
    .description = Limite percentual para detecção do padrão

strategies-condition-consecutive-candles = Candles consecutivos
    .description = Detecta candles verdes (de alta) ou vermelhos (de baixa) consecutivos, com filtro de tamanho mínimo
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
    .description = Verifica há quanto tempo uma posição é mantida (para estratégias de saída: saídas por tempo)
strategies-condition-position-holding-time-param-hours = Limite de tempo (horas)
    .description = Duração em horas desde a abertura da posição
strategies-condition-position-holding-time-param-comparison = Comparação
    .description = Como comparar a idade da posição com o limite
strategies-condition-position-holding-time-param-comparison-option-greater-than = Mais antiga que (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = No mínimo (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = Mais recente que ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = No máximo (≤)

strategies-condition-price-breakout = Rompimento de preço
    .description = Detecta o preço rompendo acima da resistência (máxima do período) ou abaixo do suporte (mínima do período)
strategies-condition-price-breakout-param-lookback = Janela de análise
    .description = Número de candles para encontrar o nível de suporte/resistência
strategies-condition-price-breakout-param-direction = Direção do rompimento
    .description = Direção do rompimento
strategies-condition-price-breakout-param-direction-option-upward = Para cima (rompe resistência)
strategies-condition-price-breakout-param-direction-option-downward = Para baixo (rompe suporte)
strategies-condition-price-breakout-param-confirmation = Confirmação %
    .description = Quanto além do nível para confirmar o rompimento (evita sinais falsos)

strategies-condition-price-change-percent = Variação de preço %
    .description = Verifica se o preço variou por um limite percentual dentro de um período
strategies-condition-price-change-percent-param-percentage = Limite de variação %
    .description = Variação percentual de preço que aciona a condição (0.1-1000%)
strategies-condition-price-change-percent-param-direction = Direção
    .description = Direção do movimento do preço
strategies-condition-price-change-percent-param-direction-option-above = Alta (+%)
strategies-condition-price-change-percent-param-direction-option-below = Queda (-%)
strategies-condition-price-change-percent-param-direction-option-within = Dentro da faixa (±%)
strategies-condition-price-change-percent-param-time-value = Período
    .description = Valor da janela de análise (1-3600 para segundos, 1-1440 para minutos, 1-720 para horas)
strategies-condition-price-change-percent-param-time-unit = Unidade de tempo
    .description = Unidade de tempo da janela de análise
strategies-condition-price-change-percent-param-time-unit-option-seconds = Segundos
strategies-condition-price-change-percent-param-time-unit-option-minutes = Minutos
strategies-condition-price-change-percent-param-time-unit-option-hours = Horas

strategies-condition-price-to-ma = Preço vs média móvel
    .description = Verifica se o preço está acima, abaixo ou dentro da faixa da sua média móvel simples
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
    .description = Detecta picos de volume em relação ao volume médio (indica maior interesse)
strategies-condition-volume-spike-param-lookback = Janela de análise
    .description = Número de candles para calcular o volume médio
strategies-condition-volume-spike-param-multiplier = Multiplicador de volume
    .description = Quantas vezes acima da média (ex.: 2.0 = 200% da média)

## Shared by every condition

strategies-condition-param-timeframe = Timeframe
    .description = Timeframe dos candles a analisar (usa o timeframe da estratégia se não definido)
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
strategies-error-invalid-rule-tree = Árvore de regras inválida: { $reason }
strategies-error-evaluation-timeout = A avaliação da estratégia expirou após { $timeout } ms
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

# Indicators

strategies-error-indicator-consecutive-candles = candles consecutivos
strategies-error-indicator-moving-average = média móvel
strategies-error-indicator-price-breakout = rompimento de preço
strategies-error-indicator-price-change-lookback = janela de variação de preço
strategies-error-indicator-volume-spike = pico de volume

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = Nó de ramificação sem condições
strategies-error-rule-branch-node-missing-operator = Nó de ramificação sem operador
strategies-error-rule-branch-node-must-have-at-least-one-child = O nó de ramificação deve ter ao menos um filho
strategies-error-rule-invalid-rule-tree-structure = Estrutura de árvore de regras inválida
strategies-error-rule-leaf-node-missing-condition = Nó folha sem condição
strategies-error-rule-not-operator-must-have-exactly-one-child = O operador NOT deve ter exatamente um filho
