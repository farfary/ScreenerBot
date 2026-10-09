# Token data source status. Ids come from build_source_status in
# src/webserver/routes/tokens/source_status.rs. $label is a provider name and is
# not translated.

tokens-result-source-live = Dados de mercado ao vivo
tokens-result-source-unavailable = { $label } indisponível — tentando novamente
tokens-result-source-not-listed = Não listado no { $label }
tokens-result-security-available = Relatório de segurança disponível
tokens-result-security-missing = Sem relatório do { -rugcheck }
tokens-result-chart-available = Dados do gráfico disponíveis
tokens-result-chart-missing = Ainda sem dados do gráfico

# Token details dialog: shared tab states (ui/token_details/state_handling.js)

tokens-state-error-title = Não foi possível carregar os dados
tokens-state-offline = Você parece estar offline.
tokens-state-request-failed = A solicitação falhou após várias tentativas.
tokens-state-waiting = Aguardando dados…

# Token details dialog: chart (ui/token_details/chart_tab.js)

tokens-chart-marker-entry = Entrada
tokens-chart-level-stop-loss = Stop loss
tokens-chart-level-take-profit = Take profit

# Token details dialog: transactions tab (ui/token_details/transactions_tab.js)

tokens-transactions-loading = Carregando transações…
tokens-transactions-empty-title = Nenhuma transação
tokens-transactions-empty-history = Não há histórico de transações de carteira disponível para este token.
tokens-transactions-empty-data = Não há dados de transações disponíveis para este token.
tokens-transactions-error-title = Não foi possível carregar as transações
tokens-transactions-error-message = O histórico de transações está temporariamente indisponível.
tokens-transactions-activity-title = Atividade em 24h
tokens-transactions-activity-subtitle = Transações de carteiras por hora
tokens-transactions-metric-total = Total
tokens-transactions-metric-buys = Compras
tokens-transactions-metric-sells = Vendas
tokens-transactions-recent-title = Transações recentes
tokens-transactions-shown = { $count } exibidas
tokens-transactions-column-time = Hora
tokens-transactions-column-type = Tipo
tokens-transactions-column-price = Preço ({ -sol })
tokens-transactions-column-total = Total ({ -sol })
tokens-transactions-chart-missing = Biblioteca de gráficos ausente
tokens-transactions-view-solscan = Ver transação no { -solscan }

# Token details dialog: positions tab (ui/token_details/positions_tab.js)

tokens-positions-empty-title = Nenhuma posição
tokens-positions-no-token = Nenhum token selecionado.
tokens-positions-empty-message = Ainda não há posição neste token. Use Comprar para abrir uma.
tokens-positions-loading = Carregando posição…
tokens-positions-from-wallet-history = Do histórico da carteira
tokens-positions-frozen = Congelado — não pode ser vendido
tokens-positions-no-cost-basis = Sem custo base
tokens-positions-history-incomplete = Histórico incompleto
tokens-positions-dca-count = DCA { $count }
tokens-positions-exit-count = Saídas { $count }
tokens-positions-fact-avg-entry = Entrada méd.
tokens-positions-fact-current = Atual
tokens-positions-fact-tokens = Tokens
tokens-positions-fact-opened = Aberta em
tokens-positions-fact-exit-price = Preço de saída
tokens-positions-fact-native-received = { -sol } recebido
tokens-positions-fact-closed-reason = Motivo do fechamento
tokens-positions-fact-target-min = Meta de lucro mín.
tokens-positions-fact-target-max = Meta de lucro máx.
tokens-positions-fact-highest = Preço mais alto
tokens-positions-fact-lowest = Preço mais baixo
tokens-positions-section-range = Metas e faixa
tokens-positions-section-market = Mercado e posses
tokens-positions-kicker = Posição
tokens-positions-fallback-symbol = Token
tokens-positions-realized-pnl = P&L realizado
tokens-positions-unrealized-pnl = P&L não realizado
tokens-positions-size = Tamanho

# Token details dialog: security tab (ui/token_details/security_tab.js)
# Risk names and descriptions come from the RugCheck report and render as sent.

tokens-security-analysis-pending = Análise do { -rugcheck } em andamento...
tokens-security-analyzing = Analisando segurança…
tokens-security-pulse-title = Pulso de segurança
tokens-security-pending-caption = Os sinais de risco ainda estão sendo coletados.
tokens-security-control-title = Controle do token
tokens-security-control-meta = Status das autoridades
tokens-security-updated = Atualizado { $time }
tokens-security-score-caption = Pontuação de risco do token normalizada em 100.
tokens-security-score-label = Pontuação
tokens-security-rugged = Rug pull
tokens-security-grade-analyzing = Analisando
tokens-security-grade-shielded = Blindado
tokens-security-grade-safe = Seguro
tokens-security-grade-caution = Cautela
tokens-security-grade-vulnerable = Vulnerável
tokens-security-grade-unknown = Desconhecido
tokens-security-metric-token-type = Tipo de token
tokens-security-metric-total-holders = Total de holders
tokens-security-metric-lp-providers = Provedores de LP
tokens-security-metric-graph-insiders = Insiders no grafo
tokens-security-insiders-detected = Detectados ({ $count })
tokens-security-insiders-clean = Limpo
tokens-security-authority-mint = Mint
tokens-security-authority-freeze = Congelamento
tokens-security-authority-immutable = Imutável
tokens-security-authority-mutable = Mutável
tokens-security-authority-revoked = Revogada
tokens-security-authority-active = Ativa
tokens-security-holder-health-title = Saúde dos holders
tokens-security-holders-unique = únicos
tokens-security-creator-share = Participação do criador
tokens-security-gauge-top-10 = Top 10
tokens-security-concentration-unknown = Desconhecida
tokens-security-concentration-critical = Crítica
tokens-security-concentration-high = Alta
tokens-security-concentration-moderate = Moderada
tokens-security-concentration-healthy = Saudável
tokens-security-transfer-title = Taxa de transferência
tokens-security-transfer-no-fee = Sem taxa
tokens-security-transfer-fee-percentage = Porcentagem da taxa
tokens-security-transfer-max-fee = Valor máximo da taxa
tokens-security-transfer-authority = Autoridade da taxa
tokens-security-transfer-note = Uma taxa de { $percent } é cobrada em cada transferência.
tokens-security-transfer-none = Nenhuma taxa de transferência detectada.
tokens-security-risks-title = Riscos de segurança
tokens-security-risks-none = Nenhum risco de segurança detectado.
tokens-security-risk-fallback-name = Sinal de segurança
tokens-security-risks-critical = { $count } crítico(s)
tokens-security-risks-warnings =
    { $count ->
        [one] { $count } alerta
        [many] { $count } alertas
       *[other] { $count } alertas
    }
tokens-security-risks-info = { $count } informativo(s)
tokens-security-risks-incidents =
    { $count ->
        [one] { $count } incidente encontrado
        [many] { $count } incidentes encontrados
       *[other] { $count } incidentes encontrados
    }
tokens-security-top-holders-title = Maiores holders
tokens-security-top-holders-concentration = { $percent } de concentração
tokens-security-insider = Insider

# Token details dialog: overview tab (ui/token_details/overview_tab.js)
# 5M/1H/6H/24H period codes and the chart timeframe buttons are id codes shared
# with the chart and stay as sent.

tokens-overview-chart-checking = Verificando dados…
tokens-overview-banner-open = Abrir banner do token
tokens-overview-headline-label = Principais métricas de mercado
tokens-overview-price = Preço
tokens-overview-market-cap = Market cap
tokens-overview-liquidity = Liquidez
tokens-overview-volume = Volume
tokens-overview-no-tags = Sem tags
tokens-overview-info-title = Informações do token
tokens-overview-profile = Perfil publicado
tokens-overview-fact-mint = Mint
tokens-overview-fact-decimals = Decimais
tokens-overview-fact-age = Idade
tokens-overview-fact-dex = DEX
tokens-overview-fact-holders = Holders
tokens-overview-fact-top-10 = Top 10
tokens-overview-tags = Tags
tokens-overview-liquidity-title = Liquidez e mercado
tokens-overview-fact-fdv = FDV
tokens-overview-fact-pool-native = Pool { -sol }
tokens-overview-fact-pool-token = Pool Token
tokens-overview-pool = Pool
tokens-overview-pulse-title = Pulso do mercado
tokens-overview-activity-title = Atividade de transações
tokens-overview-buy-share = { $percent } compras
tokens-overview-buy-sell-ratio = { $ratio } C/V
tokens-overview-buys-24h = Compras 24H
tokens-overview-sells-24h = Vendas 24H
tokens-overview-net-flow = Fluxo líquido
tokens-overview-total-24h = Total 24H
tokens-overview-average-24h = Méd. 24H
tokens-overview-spike-5m = Pico 5M
tokens-overview-rate-per-hour = { $amount }/h
tokens-overview-rate-per-minute = { $amount }/min
tokens-overview-spike-factor = { $factor }×
tokens-overview-flow-counts = Compras: { $buys } ({ $buyPercent }), Vendas: { $sells } ({ $sellPercent }), Total: { $total }
tokens-overview-flow-no-data = Sem dados de transações

# Token details dialog: pools tab (ui/token_details/pools_links_tab.js)
# DEX names in pool data render as sent.

tokens-pools-empty-title = Nenhum pool
tokens-pools-empty-message = Nenhum pool de liquidez foi detectado para este token.
tokens-pools-unknown = Desconhecido
tokens-pools-unknown-dex = DEX desconhecida
tokens-pools-liquidity = Liquidez
tokens-pools-volume-24h = Volume 24h
tokens-pools-base-role = Papel base
tokens-pools-quote-role = Papel de cotação
tokens-pools-canonical = Canônico
tokens-pools-summary-title = Resumo dos pools
tokens-pools-breakdown-title = Detalhamento por DEX
tokens-pools-all-title = Todos os pools
tokens-pools-updated = Atualizado
tokens-pools-role-base = Base
tokens-pools-role-quote = Cotação
tokens-pools-role-unknown = Desconhecido
tokens-pools-reserves = Contas de reserva
tokens-pools-no-reserves = Nenhuma conta de reserva
tokens-pools-address-pool = Pool
    .title = Copiar pool
tokens-pools-address-base = Mint base
    .title = Copiar mint base
tokens-pools-address-quote = Mint de cotação
    .title = Copiar mint de cotação
    .title = Copiar mint pareado

# Token details dialog: links tab (ui/token_details/pools_links_tab.js)

tokens-links-empty = Não há site oficial nem links de redes sociais disponíveis para este token.
tokens-links-info-title = Informações do token
tokens-links-mint-address = Endereço do mint
tokens-links-data-source = Fonte dos dados
tokens-links-security = Segurança
tokens-links-profile-title = Perfil do token
tokens-links-profile-published-title = Conteúdo do perfil publicado
tokens-links-profile-published-note = Mídia, descrição e links oficiais são conteúdo de perfil pago, revisado antes da publicação. Isso não verifica a propriedade nem a segurança do token.
tokens-links-profile-create-note = Adicione um logo revisado, a descrição do projeto e links oficiais ao perfil público deste token.
tokens-links-profile-update-hint = Atualize o perfil deste token em screenerbot.io
tokens-links-profile-create-hint = Crie um perfil de token em screenerbot.io
tokens-links-profile-update = Atualizar perfil
tokens-links-profile-create = Criar perfil
tokens-links-media-title = Recursos de mídia
tokens-links-media-fallback-symbol = Token
tokens-links-media-logo = Logo
tokens-links-media-banner = Banner
tokens-links-media-banner-alt = Banner de { $symbol }
tokens-links-media-open = Abrir imagem
tokens-links-description-title = Descrição
tokens-links-explorers-title = Exploradores e análises
tokens-links-websites-title = Sites oficiais
tokens-links-socials-title = Redes sociais
tokens-links-explorer-solana-explorer = { -solana-explorer }
tokens-links-explorer-geckoterminal = { -geckoterminal }
tokens-links-explorer-dextools = { -dextools }
tokens-links-explorer-coingecko = { -coingecko }
tokens-links-explorer-jupiter-swap = { -jupiter } Swap
tokens-links-social-twitter = { -twitter } / { -x }
tokens-links-social-x = { -x } ({ -twitter })
tokens-links-social-telegram = { -telegram }
tokens-links-social-discord = { -discord }
tokens-links-social-medium = { -medium }
tokens-links-social-github = { -github }
tokens-links-social-youtube = { -youtube }
tokens-links-social-reddit = { -reddit }
tokens-links-social-facebook = { -facebook }
tokens-links-social-instagram = { -instagram }
tokens-links-social-linkedin = { -linkedin }
tokens-links-social-tiktok = { -tiktok }
tokens-links-social-fallback = Rede social

# Token details dialog: frame, header and data sources (ui/token_details_dialog.js)

tokens-dialog-tab-overview = Visão geral
tokens-dialog-tab-security = Segurança
tokens-dialog-tab-positions = Posições
tokens-dialog-tab-pools = Pools
tokens-dialog-tab-links = Links
tokens-dialog-tab-transactions = Txs
tokens-dialog-sections = Seções dos detalhes do token
tokens-dialog-close =
    .title = Fechar (ESC)
    .aria-label = Fechar detalhes do token
tokens-dialog-unknown-symbol = Desconhecido
tokens-dialog-unknown-name = Token desconhecido
tokens-dialog-market-summary = Resumo do mercado
tokens-dialog-price-loading = Carregando preço
tokens-dialog-unit-native = { -sol }
tokens-dialog-market-metrics = Métricas de mercado
tokens-dialog-metric-market-cap = Market cap
tokens-dialog-metric-volume-24h = Volume 24h
tokens-dialog-change-24h = Variação em 24 horas { $change }
tokens-dialog-buy = Comprar
    .title = Comprar este token
tokens-dialog-sell = Vender
    .title = Vender posição
tokens-dialog-sell-unavailable = Nenhuma posição aberta para vender
tokens-dialog-details = Detalhes
tokens-dialog-sources = Fontes
tokens-dialog-sources-status = Status das fontes de dados
tokens-dialog-updated-label = Atualizado
tokens-dialog-just-now = Agora mesmo
tokens-dialog-updated-at = Atualizado { $time }
tokens-dialog-updated-unavailable = Horário da atualização indisponível
tokens-dialog-error-title = Não foi possível carregar os dados do token
tokens-dialog-waiting-token = Aguardando dados do token…
tokens-dialog-loading-overview = Carregando visão geral…
tokens-dialog-loading-security = Carregando segurança…
tokens-dialog-loading-pools = Carregando pools…
tokens-dialog-loading-links = Carregando links…
tokens-dialog-chart-still-checking = Ainda sem dados do gráfico — continuando a verificar…
tokens-dialog-no-data = Nenhum dado disponível
tokens-dialog-source-token = Token
tokens-dialog-source-market = Mercado
tokens-dialog-source-security = Segurança
tokens-dialog-source-chart = Gráfico
tokens-dialog-status-pending = Aguardando
tokens-dialog-status-loading = Carregando
tokens-dialog-status-ready = Pronto
tokens-dialog-status-unavailable = Indisponível
tokens-dialog-status-cached = Em cache
tokens-dialog-source-summary = { $source }: { $status }
tokens-dialog-badge-pool-price = Preço do pool
tokens-dialog-badge-pool-price-hint = Preço do pool on-chain em tempo real
tokens-dialog-badge-api-price = Preço da API
tokens-dialog-badge-api-price-hint = Preço dos dados de mercado em cache (API)
tokens-dialog-badge-profile = Perfil publicado
    .title = Conteúdo de perfil pago, revisado para publicação; não é uma auditoria nem uma verificação de propriedade.
tokens-dialog-badge-low-risk-hint = Risco baixo segundo a pontuação atual do { -rugcheck }; não é verificação de identidade.
tokens-dialog-badge-immutable = Imutável
tokens-dialog-badge-mutable = Mutável
tokens-dialog-badge-position = Posição
tokens-dialog-badge-blacklisted = Na lista negra

# Tokens page: sub-tabs (scripts/pages/tokens/constants.js)
# Ids are the view values of /api/tokens/list.

tokens-view-favorites = Favoritos
tokens-view-pool = Serviço de pools
tokens-view-no-market = Sem dados de mercado
tokens-view-all = Todos os tokens
tokens-view-passed = Aprovados
tokens-view-rejected = Rejeitados
tokens-view-blacklisted = Na lista negra
tokens-view-positions = Posições
tokens-view-recent = Recentes
tokens-view-ohlcv = Dados OHLCV
# Empty token table per view (TOKEN_VIEW_EMPTY_LABELS)
tokens-view-pool-empty = Nenhum token com preço ainda
    .message = Os tokens aparecem aqui quando passam na filtragem e o preço do pool é calculado.
tokens-view-no-market-empty = Nenhum token sem dados de mercado
    .message = Os tokens aparecem aqui enquanto as fontes de dados de mercado ainda não os listaram.
tokens-view-all-empty = Nenhum token descoberto ainda
    .message = Todo token encontrado pela descoberta aparece aqui, seja qual for o resultado da filtragem.
tokens-view-passed-empty = Nenhum token passou na filtragem
    .message = Os tokens que passam em todos os filtros ativos aparecem aqui. Revise a página Filtragem se isto continuar vazio.
tokens-view-rejected-empty = Nenhum token rejeitado
    .message = Os tokens que falham em um filtro aparecem aqui com o motivo.
tokens-view-blacklisted-empty = Nenhum token na lista negra
    .message = Os tokens excluídos do trading, por você ou pelas verificações de segurança, aparecem aqui.
tokens-view-positions-empty = Nenhum token em posições
    .message = Os tokens mantidos em posições abertas aparecem aqui.
tokens-view-recent-empty = Nenhum token recente
    .message = Os tokens recém-descobertos aparecem aqui conforme são encontrados.
tokens-ohlcv-empty = Nenhum dado de gráfico ainda
    .message = Os tokens aparecem aqui assim que seus candles começam a ser coletados.

# Tokens page: token cell (scripts/pages/tokens/formatters.js)

tokens-cell-logo-enlarge = Clique para ampliar
tokens-boost-title = Impulsionado { $boosts } no screenerbot.io
tokens-cell-action-add =
    .title = Aportar na posição (DCA)
    .aria-label = Aportar na posição
tokens-cell-action-sell =
    .title = Vender (total ou % parcial)
    .aria-label = Vender token
tokens-cell-action-buy =
    .title = Comprar posição
    .aria-label = Comprar token
tokens-cell-external-links =
    .title = Links externos
    .aria-label = Links externos

# Tokens page: table states shared by the token lists (scripts/pages/tokens/*.js)

tokens-table-loading-title = Carregando tokens…
tokens-table-loading-description = Preparando a visão de tokens selecionada.
tokens-table-retry-hint = Troque de aba ou tente novamente.
tokens-filter-all = Todos

# Tokens page: favorites (scripts/pages/tokens/favorites.js)

tokens-favorites-load-failed-title = Não foi possível carregar os favoritos
tokens-favorites-load-failed-toast = Não foi possível carregar os favoritos
tokens-favorites-total = Total de favoritos
tokens-favorites-empty-title = Nenhum favorito ainda
    .message = Marque um token com estrela em qualquer lista para mantê-lo aqui.

# Tokens page: OHLCV data view (scripts/pages/tokens/ohlcv.js)
# Status ids come from /api/ohlcv/tokens; priority ids are Priority::as_str in src/ohlcvs/types.rs.

tokens-column-token = Token
tokens-column-status = Status
tokens-ohlcv-delete =
    .title = Excluir dados OHLCV
    .aria-label = Excluir dados OHLCV
tokens-ohlcv-status-active = Ativo
tokens-ohlcv-status-inactive = Inativo
tokens-ohlcv-priority-critical = Crítica
tokens-ohlcv-priority-high = Alta
tokens-ohlcv-priority-medium = Média
tokens-ohlcv-priority-low = Baixa
tokens-ohlcv-column-priority = Prioridade
tokens-ohlcv-column-backfill = Backfill
tokens-ohlcv-column-data-span = Período dos dados
tokens-ohlcv-column-gaps = Gaps
tokens-ohlcv-column-pools = Pools
tokens-ohlcv-column-last-fetch = Última busca
tokens-ohlcv-timeframe-complete = { $timeframe }: concluído
tokens-ohlcv-timeframe-pending = { $timeframe }: pendente
tokens-ohlcv-load-failed-title = Não foi possível carregar os dados OHLCV
tokens-ohlcv-load-failed-toast = Não foi possível carregar os dados OHLCV
tokens-ohlcv-total = Total de tokens
tokens-ohlcv-active = Ativos
tokens-ohlcv-db-size = Tamanho do BD
tokens-ohlcv-cleanup = Limpar inativos
tokens-ohlcv-delete-title = Excluir dados OHLCV
tokens-ohlcv-delete-token-message = Excluir todos os dados OHLCV de { $token }?
tokens-ohlcv-delete-done =
    Excluídos: { $candles ->
        [one] { $candles } candle
        [many] { $candles } candles
       *[other] { $candles } candles
    }, { $pools ->
        [one] { $pools } pool
        [many] { $pools } pools
       *[other] { $pools } pools
    }
tokens-ohlcv-delete-failed = Falha ao excluir os dados OHLCV
tokens-ohlcv-cleanup-title = Excluir tokens inativos
tokens-ohlcv-cleanup-message = Exclui tokens inativos há mais do que o número de horas informado
tokens-ohlcv-cleanup-placeholder = Horas...
tokens-ohlcv-cleanup-invalid = Informe um número positivo
tokens-ohlcv-cleanup-done =
    Limpeza concluída: { $count ->
        [one] { $count } token inativo
        [many] { $count } tokens inativos
       *[other] { $count } tokens inativos
    }
tokens-ohlcv-cleanup-failed = Falha ao limpar os dados OHLCV

# Tokens page: token lists (scripts/pages/tokens.js)
# The list statuses shown in the Status column come from row flags, not ids.

tokens-summary-total = Total
tokens-summary-pool-priced = Com preço do pool
tokens-summary-positions = Posições
tokens-summary-blacklisted = Na lista negra
tokens-search-placeholder = Buscar por símbolo ou mint...
tokens-table-waiting-title = Ainda carregando tokens...
tokens-table-waiting-description = Aguardando resposta do backend. Tentaremos novamente automaticamente.
tokens-load-failed-toast = Não foi possível carregar os tokens
tokens-row-data-missing = Dados do token não encontrados
tokens-column-price-sol = Preço ({ -sol })
tokens-column-liquidity = Liquidez
tokens-column-volume-24h = Vol. 24h
tokens-column-fdv = FDV
tokens-column-market-cap = Market cap
tokens-column-change-1h = 1h
tokens-column-change-24h = 24h
tokens-column-txns-5m = Txs 5m
tokens-column-txns-1h = Txs 1h
tokens-column-txns-6h = Txs 6h
tokens-column-txns-24h = Txs 24h
tokens-column-risk-score = Pontuação de risco
tokens-column-reject-reason = Motivo da rejeição
tokens-column-blacklist-reason = Motivo da lista negra
tokens-column-updated = Atualizado
tokens-column-birth = Criação
tokens-column-first-seen = Visto pela 1ª vez
tokens-badge-price = Preço
tokens-badge-ohlcv = OHLCV
tokens-badge-position = Posição
tokens-badge-blacklisted = Na lista negra
tokens-badge-blacklisted-title = Token na lista negra
tokens-badge-blacklisted-reasons = Na lista negra: { $reasons }
tokens-links-menu-copy-mint = Copiar mint
tokens-links-copy-failed = Falha ao copiar o mint
tokens-lightbox-token-age = Idade do token

# Global search dialog (scripts/ui/search_dialog.js)

tokens-search-placeholder-dialog = Buscar nome, símbolo ou mint...
tokens-search-input-label = Buscar tokens
tokens-search-results-label = Resultados da busca
tokens-search-tip-nav = navegar
tokens-search-tip-open = abrir
tokens-search-tip-close = fechar
tokens-search-failed = Falha na busca
tokens-search-error = Erro: { $message }
tokens-search-clear =
    .title = Limpar busca
    .aria-label = Limpar busca
tokens-search-recent = Recentes
tokens-search-recent-label = Buscas recentes
tokens-search-lists-label = Listas de tokens
tokens-search-tab-trending = Em alta
tokens-search-kinds = Nome · símbolo · mint
tokens-search-empty-trending = Os tokens em alta aparecem assim que o bot precificar seus primeiros pools.
tokens-search-empty-positions = Nenhuma posição aberta no momento.
tokens-search-empty-favorites = Marque um token com estrela e ele ficará aqui para a próxima busca.
tokens-search-empty-boosted = Nenhum token está impulsionado no momento.
tokens-search-list-failed = Não foi possível carregar esta lista.
tokens-search-searching = Buscando nos mercados…
# $count is the number of tokens found.
tokens-search-result-count =
    { $count ->
        [one] { $count } resultado
        [many] { $count } resultados
       *[other] { $count } resultados
    }
tokens-search-order = Melhor correspondência primeiro, depois volume 24h
tokens-search-metric-mc = MC
    .title = Market cap
tokens-search-metric-fdv = FDV
    .title = Avaliação totalmente diluída
tokens-search-metric-liq = Liq
    .title = Liquidez
tokens-search-metric-vol = Vol
    .title = Volume 24h
tokens-search-more =
    .title = Mais ações
    .aria-label = Mais ações
# $query is the text the user typed.
tokens-search-no-match = Nenhum token corresponde a “{ $query }”.

# Featured dialog (scripts/ui/featured_dialog.js)
# Category and source ids are those of CATEGORIES; provider names are terms.

tokens-featured-category-boosted = Impulsionados
tokens-featured-category-jupiter-organic = Mais orgânicos da { -jupiter }
tokens-featured-category-jupiter-traded = Mais negociados da { -jupiter }
tokens-featured-category-dexscreener-trending = Em alta no { -dexscreener }
tokens-featured-source-jupiter = { -jupiter }
tokens-featured-source-dexscreener = { -dexscreener }
tokens-featured-note-boosted = Promovidos por suas equipes
tokens-featured-security-risky = Arriscado
tokens-featured-load-failed = Falha ao carregar os destaques
tokens-featured-network-error = Erro de rede: { $message }
tokens-featured-title = Em destaque
tokens-featured-subtitle = Primeiro os tokens impulsionados, depois os que estão em alta na Solana
tokens-featured-boost = Impulsionar um token
tokens-featured-close =
    .title = Fechar (ESC)
tokens-featured-loading = Carregando destaques e tokens em alta...
tokens-featured-error-hint = Verifique a conexão ou tente novamente
tokens-featured-empty = Nenhum token disponível no momento
tokens-featured-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
tokens-featured-stat-market-cap = Market cap
tokens-featured-stat-liquidity = Liquidez
tokens-featured-stat-volume = Vol. 24H
tokens-featured-stat-holders = Holders
tokens-featured-stat-txns = Txs 24H
tokens-featured-buy = Comprar
    .title = Comprar { $symbol }
tokens-featured-security-score = Pontuação de segurança: { $score }/100
tokens-featured-social-website = Site
tokens-featured-social-twitter = { -twitter }

# Featured row (scripts/ui/featured_row.js)

tokens-featured-row-view-all = Todos
    .title = Abrir a visão completa de Em destaque
tokens-featured-row-scroll-start =
    .aria-label = Mostrar tokens anteriores
tokens-featured-row-scroll-end =
    .aria-label = Mostrar mais tokens
tokens-featured-row-empty = Nenhum token em destaque
tokens-featured-row-title = { $name } ({ $symbol })
tokens-featured-row-boosted-title = { $name } ({ $symbol }) — impulsionado { $boosts }

# Pool selector dialog (scripts/ui/pool_selector.js)

tokens-pool-selector-title = Selecionar pool
tokens-pool-selector-loading = Carregando pools...
tokens-pool-selector-empty = Nenhum pool encontrado para este token
tokens-pool-selector-load-failed = Falha ao carregar os pools: { $message }
tokens-pool-selector-count =
    { $count ->
        [one] { $count } pool encontrado
        [many] { $count } pools encontrados
       *[other] { $count } pools encontrados
    }
tokens-pool-selector-liquidity = { $amount } liq.
    .title = Liquidez
tokens-pool-selector-volume = { $amount } 24h
    .title = Volume 24h

# Token identity chips and address rows (scripts/ui/token_identity.js)

tokens-identity-unknown-asset = Ativo desconhecido
tokens-identity-copy-address =
    .title = Copiar endereço
    .aria-label = Copiar endereço
tokens-identity-copy-signature =
    .title = Copiar assinatura
    .aria-label = Copiar assinatura

tokens-rugcheck-risk-single-holder-ownership = Um único holder domina
    .description = Um único holder possui grande parte do supply do token.
tokens-rugcheck-risk-low-liquidity = Liquidez baixa
    .description = O pool do token tem pouca liquidez.
tokens-rugcheck-risk-few-lp-providers = Poucos provedores de LP
    .description = Apenas poucos usuários fornecem liquidez.
tokens-rugcheck-risk-high-holder-concentration = Alta concentração de holders
    .description = Os 10 maiores holders possuem mais de 50% do supply do token.
tokens-rugcheck-risk-top-10-holders-high-ownership = Alta participação do top 10
    .description = Os 10 maiores holders possuem mais de 70% do supply do token.
tokens-rugcheck-risk-high-ownership = Alta participação
    .description = Os maiores holders possuem mais de 80% do supply do token.
tokens-rugcheck-risk-creator-rug-history = Criador com histórico de rug pull
    .description = O criador tem histórico de rug pull em tokens.
tokens-rugcheck-risk-large-lp-unlocked = Grande parte da LP desbloqueada
    .description = Grande parte dos tokens LP está desbloqueada, permitindo que o dono remova a liquidez a qualquer momento.
tokens-rugcheck-risk-mutable-metadata = Metadados mutáveis
    .description = O dono pode alterar os metadados do token.
tokens-rugcheck-risk-few-holders = Poucos holders
    .description = Poucas carteiras possuem o token.
tokens-rugcheck-risk-copycat-token = Token imitador
    .description = Este token usa o símbolo de um token verificado.
tokens-rugcheck-risk-fee-config-enabled = Taxas configuráveis
    .description = O dono pode alterar as taxas a qualquer momento.
tokens-rugcheck-risk-high-holder-correlation = Alta correlação de holders
    .description = Os maiores holders possuem quantidades parecidas do supply.
tokens-rugcheck-risk-freeze-authority-enabled = Autoridade de congelamento ativa
    .description = Os tokens podem ser congelados e impedidos de negociar.
tokens-rugcheck-risk-mint-authority-enabled = Autoridade de mint ativa
    .description = O dono pode emitir mais tokens.
tokens-rugcheck-risk-missing-file-metadata = Arquivo de metadados ausente
    .description = Nenhum arquivo de metadados está associado a este token.
tokens-rugcheck-risk-high-market-cap-per-holder = Market cap alto por holder
    .description = O market cap é muito alto em relação ao número de holders.
tokens-rugcheck-risk-symbol-mismatch = Símbolo divergente
    .description = O símbolo do token não corresponde ao seu arquivo de metadados.
tokens-rugcheck-risk-name-mismatch = Nome divergente
    .description = O nome do token não corresponde ao seu arquivo de metadados.
tokens-rugcheck-risk-permanent-control-enabled = Controle permanente ativado
    .description = O criador do token pode controlar todos os tokens permanentemente.
tokens-rugcheck-risk-missing-metadata = Metadados ausentes
    .description = Nenhum metadado foi encontrado para este token.
tokens-rugcheck-risk-lp-unlock-soon = Desbloqueio de LP em breve
    .description = Os tokens LP serão desbloqueados em breve, permitindo que o dono remova a liquidez.
tokens-rugcheck-risk-lp-vault-unlocked = Cofre de LP desbloqueado
    .description = Os tokens LP do cofre podem ser resgatados.
tokens-rugcheck-risk-mint-authority-locked = Autoridade de mint bloqueada
    .description = A emissão de novos tokens está bloqueada.
tokens-rugcheck-risk-high-transfer-fee = Taxa de transferência alta
    .description = Cada transferência deste token paga uma taxa alta.
