# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = Sem decimais no banco de dados
filtering-reject-token-too-new = Token muito novo
filtering-reject-cooldown-filtered = Bloqueado por cooldown
filtering-reject-dex-data-missing = Dados do { -dexscreener } ausentes
filtering-reject-gecko-data-missing = Dados do { -geckoterminal } ausentes
filtering-reject-rug-data-missing = Dados do { -rugcheck } ausentes
filtering-reject-onchain-numeric-symbol = Símbolo apenas numérico (golpe)
filtering-reject-onchain-empty-symbol = Símbolo vazio (golpe)
filtering-reject-onchain-suspicious-symbol = Símbolo suspeito (golpe)
filtering-reject-onchain-known-scam-authority = Autoridade de golpe conhecida
filtering-reject-onchain-immutable-with-freeze = Imutável + autoridade de congelamento (golpe)
filtering-reject-onchain-high-risk-score = Pontuação de risco on-chain alta
filtering-reject-dex-empty-name = Nome vazio
filtering-reject-dex-empty-symbol = Símbolo vazio
filtering-reject-dex-empty-logo = URL do logo vazia
filtering-reject-dex-empty-website = URL do site vazia
filtering-reject-dex-txn-5m = Poucas transações em 5min
filtering-reject-dex-txn-1h = Poucas transações em 1h
filtering-reject-dex-zero-liq = Liquidez zero
filtering-reject-dex-liq-low = Liquidez muito baixa
filtering-reject-dex-liq-high = Liquidez muito alta
filtering-reject-dex-mcap-low = Market cap muito baixo
filtering-reject-dex-mcap-high = Market cap muito alto
filtering-reject-dex-vol-low = Volume muito baixo
filtering-reject-dex-vol-missing = Volume ausente
filtering-reject-dex-fdv-low = FDV muito baixo
filtering-reject-dex-fdv-high = FDV muito alto
filtering-reject-dex-vol5m-low = Volume de 5min muito baixo
filtering-reject-dex-vol5m-missing = Volume de 5min ausente
filtering-reject-dex-vol1h-low = Volume de 1h muito baixo
filtering-reject-dex-vol1h-missing = Volume de 1h ausente
filtering-reject-dex-vol6h-low = Volume de 6h muito baixo
filtering-reject-dex-vol6h-missing = Volume de 6h ausente
filtering-reject-dex-price-change-5m-low = Variação de preço em 5min muito baixa
filtering-reject-dex-price-change-5m-high = Variação de preço em 5min muito alta
filtering-reject-dex-price-change-low = Variação de preço muito baixa
filtering-reject-dex-price-change-high = Variação de preço muito alta
filtering-reject-dex-price-change-6h-low = Variação de preço em 6h muito baixa
filtering-reject-dex-price-change-6h-high = Variação de preço em 6h muito alta
filtering-reject-dex-price-change-24h-low = Variação de preço em 24h muito baixa
filtering-reject-dex-price-change-24h-high = Variação de preço em 24h muito alta
filtering-reject-gecko-liq-low = Liquidez muito baixa
filtering-reject-gecko-liq-high = Liquidez muito alta
filtering-reject-gecko-mcap-low = Market cap muito baixo
filtering-reject-gecko-mcap-high = Market cap muito alto
filtering-reject-gecko-vol5m-low = Volume de 5min muito baixo
filtering-reject-gecko-vol5m-missing = Volume de 5min ausente
filtering-reject-gecko-vol1h-low = Volume de 1h muito baixo
filtering-reject-gecko-vol1h-missing = Volume de 1h ausente
filtering-reject-gecko-vol24h-low = Volume de 24h muito baixo
filtering-reject-gecko-vol24h-missing = Volume de 24h ausente
filtering-reject-gecko-price-change-5m-low = Variação de preço em 5min muito baixa
filtering-reject-gecko-price-change-5m-high = Variação de preço em 5min muito alta
filtering-reject-gecko-price-change-1h-low = Variação de preço em 1h muito baixa
filtering-reject-gecko-price-change-1h-high = Variação de preço em 1h muito alta
filtering-reject-gecko-price-change-24h-low = Variação de preço em 24h muito baixa
filtering-reject-gecko-price-change-24h-high = Variação de preço em 24h muito alta
filtering-reject-gecko-pool-count-low = Quantidade de pools muito baixa
filtering-reject-gecko-pool-count-high = Quantidade de pools muito alta
filtering-reject-gecko-pool-count-missing = Quantidade de pools ausente
filtering-reject-gecko-reserve-low = Reserva muito baixa
filtering-reject-gecko-reserve-missing = Reserva ausente
filtering-reject-rug-rugged = Token com rug pull
filtering-reject-rug-score = Pontuação de risco muito alta
filtering-reject-rug-level-danger = Nível de risco perigoso
filtering-reject-rug-mint-authority = Autoridade de mint presente
filtering-reject-rug-freeze-authority = Autoridade de congelamento presente
filtering-reject-rug-top-holder = % do maior holder muito alta
filtering-reject-rug-top3-holders = % dos 3 maiores holders muito alta
filtering-reject-rug-min-holders = Holders insuficientes
filtering-reject-rug-insider-count = Insiders demais entre os holders
filtering-reject-rug-insider-pct = % de insiders muito alta
filtering-reject-rug-creator-pct = Saldo do criador muito alto
filtering-reject-rug-transfer-fee-present = Taxa de transferência presente
filtering-reject-rug-transfer-fee-high = Taxa de transferência muito alta
filtering-reject-rug-graph-insiders = Insiders no grafo em excesso
filtering-reject-rug-lp-providers-low = Provedores de LP muito poucos
filtering-reject-rug-lp-providers-missing = Provedores de LP ausentes
filtering-reject-rug-lp-lock-low = Bloqueio de LP muito baixo
filtering-reject-rug-lp-lock-missing = Bloqueio de LP ausente
filtering-reject-llm-analysis-rejected = Análise por LLM rejeitada: { $reason } ({ $confidence }% de confiança, { $provider })
filtering-reject-llm-analysis-rejected-generic = Análise por LLM rejeitada
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV ausente
filtering-reject-dex-price-change-5m-missing = Variação de preço em 5min ausente
filtering-reject-dex-price-change-missing = Variação de preço ausente
filtering-reject-dex-price-change-6h-missing = Variação de preço em 6h ausente
filtering-reject-dex-price-change-24h-missing = Variação de preço em 24h ausente
filtering-reject-gecko-liq-missing = Liquidez ausente
filtering-reject-gecko-mcap-missing = Market cap ausente
filtering-reject-gecko-price-change-5m-missing = Variação de preço em 5min ausente
filtering-reject-gecko-price-change-1h-missing = Variação de preço em 1h ausente
filtering-reject-gecko-price-change-24h-missing = Variação de preço em 24h ausente
filtering-reject-rug-transfer-fee-missing = Dados da taxa de transferência ausentes

# Rejection categories used to group reasons.
filtering-reject-category-security = Problemas de segurança
filtering-reject-category-distribution = Distribuição de holders
filtering-reject-category-liquidity-lock = Problemas de bloqueio de LP
filtering-reject-category-fees = Taxas de transferência
filtering-reject-category-liquidity = Liquidez
filtering-reject-category-volume = Volume de negociação
filtering-reject-category-market-cap = Market cap/FDV
filtering-reject-category-price-action = Movimento de preço
filtering-reject-category-activity = Atividade de negociação
filtering-reject-category-data-quality = Dados ausentes
filtering-reject-category-timing = Filtros de tempo
filtering-reject-category-market = Dados de mercado
filtering-reject-category-other = Outros

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = Status
filtering-tab-analytics = Análises
filtering-tab-explorer = Explorador
filtering-source-core = Core
filtering-source-onchain = On-chain
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = Análise por LLM

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = Tudo
filtering-range-all-time = Todo o período
filtering-range-custom = Personalizado
filtering-range-now = Agora
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = Salvando alterações...
filtering-footer-refreshing = Atualizando snapshot...
filtering-footer-unsaved = Alterações não salvas pendentes
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = Salvo { $time }
filtering-footer-in-sync = Configuração sincronizada

## Info bar and status metrics

filtering-info-total = Total
filtering-info-priced = Com preço
filtering-info-passed = Aprovados
filtering-info-positions = Posições
filtering-info-blacklisted = Na lista negra
filtering-info-cache = Cache
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count } ({ $share })
filtering-refresh-building = Construindo…
filtering-refresh-never = Nunca

filtering-status-loading = Carregando estatísticas...
filtering-status-total = Total de tokens
filtering-status-total-detail = No cache de filtragem
filtering-status-total-detail-building = Snapshot em construção: as contagens aparecem na próxima atualização
filtering-status-priced = Com preço
filtering-status-priced-detail = { $share } têm preço
filtering-status-passed = Aprovados nos filtros
filtering-status-passed-detail = { $share } aprovados
filtering-status-positions = Posições abertas
filtering-status-positions-detail = Trades ativos
filtering-status-blacklisted = Na lista negra
filtering-status-blacklisted-detail = Tokens sinalizados
filtering-status-ohlcv = Com OHLCV
filtering-status-ohlcv-detail = Dados históricos
filtering-status-refresh = Última atualização
filtering-status-refresh-building = Primeiro snapshot em andamento
filtering-status-refresh-none = Nenhuma atualização ainda
filtering-status-no-rejections = Nenhum dado de rejeição disponível

## Analytics

filtering-analytics-loading = Carregando análises de { $range }…
filtering-analytics-scanned = Total analisado
# $time is a relative time such as "5m ago".
filtering-analytics-updated = Atualizado { $time }
filtering-analytics-passed = Tokens aprovados
filtering-analytics-pass-rate = <strong>{ $share }</strong> de aprovação
filtering-analytics-rejected = Tokens rejeitados
filtering-analytics-rejection-rate = <strong>{ $share }</strong> de rejeição
filtering-analytics-by-category = Rejeição por categoria
filtering-analytics-by-source = Rejeição por fonte
filtering-analytics-no-category = Sem dados de categoria
filtering-analytics-no-source = Sem dados de fonte
filtering-analytics-top-reasons = Principais motivos de rejeição
filtering-analytics-no-data = Nenhum dado disponível
filtering-analytics-column-reason = Motivo
filtering-analytics-column-category = Categoria
filtering-analytics-column-count = Contagem
filtering-analytics-column-share = %
filtering-analytics-column-impact = Impacto
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
        [one] { $amount } token
        [many] { $amount } tokens
       *[other] { $amount } tokens
    }

## Explorer

filtering-explorer-top-reasons = Principais motivos
filtering-explorer-recent = Rejeições recentes
filtering-explorer-none = Sem dados
filtering-explorer-none-recent = Nada recente
filtering-explorer-search =
    .placeholder = Buscar motivos...
filtering-explorer-overview = Visão geral
filtering-explorer-no-match = Nenhum motivo correspondente
filtering-explorer-column-token = Token
filtering-explorer-column-source = Fonte
filtering-explorer-column-time = Hora
filtering-explorer-page = Página { $page }
filtering-explorer-no-results = Sem resultados
filtering-explorer-empty = Nenhum token encontrado
filtering-explorer-empty-filtered = Nenhum token encontrado com este filtro
filtering-explorer-load-failed = Falha ao carregar tokens

## Configuration panels

filtering-config-loading = Carregando configuração…
# $query is the text typed in the filter box.
filtering-config-no-match = Nenhum parâmetro corresponde a “{ $query }”
filtering-config-no-parameters = Esta fonte não expõe parâmetros
# $source is the source name.
filtering-source-off = O filtro de { $source } está desativado: estes parâmetros não são avaliados.
filtering-toolbar-filter =
    .placeholder = Filtrar parâmetros
    .aria-label = Filtrar parâmetros
filtering-toolbar-clear =
    .aria-label = Limpar filtro
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
        [one] { $amount } parâmetro
        [many] { $amount } parâmetros
       *[other] { $amount } parâmetros
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
        [one] { $visible } de { $total } parâmetro
        [many] { $visible } de { $total } parâmetros
       *[other] { $visible } de { $total } parâmetros
    }
filtering-group-enable =
    .aria-label = Ativar verificações de { $group }
filtering-field-min = Mín.
filtering-field-max = Máx.
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = { $label } mínimo
filtering-field-max-aria =
    .aria-label = { $label } máximo
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = Restaurar o padrão ({ $default })
    .aria-label = Restaurar { $label } para o padrão

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = Configuração salva
    .message = Configurações de filtragem salvas e snapshot atualizado
filtering-toast-save-failed = Falha ao salvar
    .message = Falha ao salvar a configuração de filtragem
filtering-toast-reset = Alterações descartadas
    .message = Configuração restaurada ao último estado salvo
filtering-toast-refresh-failed = Falha na atualização
    .message = Falha ao atualizar o snapshot de filtragem
filtering-toast-exported = Configuração exportada
    .message = Configurações de filtragem salvas em arquivo
filtering-toast-imported = Configuração importada
    .message = Configurações de filtragem carregadas do arquivo
filtering-toast-import-failed = Falha na importação
    .message = Falha ao importar a configuração: formato de arquivo inválido
filtering-toast-load-failed = Falha ao carregar
    .message = Falha ao carregar a configuração de filtragem
filtering-toast-range-missing = Selecione as datas de início e de fim
filtering-toast-range-order = O horário de início deve ser anterior ao de fim
filtering-toast-range-future = O horário de fim não pode estar no futuro
