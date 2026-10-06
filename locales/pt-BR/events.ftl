# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = Evento de OHLCV: { $subtype }
events-filtering-default = Evento de filtragem: { $subtype }
events-trader-default = Evento do trader: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = Tarefa '{ $name }' concluída
events-task-failed = Tarefa '{ $name }' falhou
events-task-timed-out = Tarefa '{ $name }' expirou

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = Tarefa concluída
events-subtype-task-failed = Tarefa falhou
events-subtype-task-timed-out = Tarefa expirou

# Shown when an event carries no display text.
events-message-none = Sem mensagem

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = Falha ao limpar o cache de OHLCV
events-ohlcv-gap-cleanup-failed = Falha ao limpar os registros de gaps preenchidos
events-ohlcv-gap-fill-failed = Erro ao preencher gap de { $mint }
events-ohlcv-backfill-scheduled = Backfill multi-timeframe agendado para { $mint } via { $pool }
events-ohlcv-fetch-failed = Falha ao buscar OHLCV de { $mint } via { $pool }: { $error }
events-ohlcv-gap-detection-failed = Falha na detecção de gaps de { $mint } via { $pool }
events-ohlcv-fetch-success = { $count } pontos de OHLCV armazenados para { $mint }
events-ohlcv-retention-backfill-failed = Falha no backfill de retenção de { $mint } via { $pool }
events-ohlcv-empty-fetch = Busca de OHLCV vazia para { $mint } via { $pool }
events-ohlcv-pool-discovery-failed = Falha na descoberta de pools de { $mint }
events-ohlcv-pool-discovery-success = Pools descobertos para { $mint }
events-ohlcv-process-token-error = Erro ao processar { $mint }: { $error }
events-ohlcv-rate-limit-hit = Limite de requisições atingido ao processar { $mint }
events-ohlcv-pool-unavailable = Nenhum pool saudável disponível para { $mint }; adiando
events-ohlcv-token-missing = O token { $mint } estava ausente durante o processamento
events-monitors-stopped = Monitores de trading automático parados
events-monitors-starting = Monitores de trading automático iniciando
events-entry-monitor-started = Monitor de oportunidades de entrada iniciado
events-exit-monitor-started = Monitor de saída/posições iniciado
events-trader-service-stopped = Serviço do trader encerrado normalmente
events-trader-service-stopping = Encerramento do serviço do trader iniciado
events-trader-service-started = Serviço do trader totalmente inicializado e em execução
events-trader-auto-trading-error = O trading automático encontrou um erro
events-trader-trading-enabled = O trading está ativado e em operação
events-trader-trading-disabled = O trading está desativado na configuração
events-trader-service-initializing = Inicialização do serviço do trader em andamento
events-connectivity-monitoring-stopped = Monitoramento de conectividade parado
events-connectivity-monitoring-started = Monitoramento de conectividade iniciado (intervalo={ $seconds }s)
events-connectivity-service-initialized = Serviço de conectividade inicializado com { $count } monitores
events-connectivity-critical-unhealthy = { $count } endpoint(s) crítico(s) com falha - o sistema deve pausar as operações
events-connectivity-endpoint-recovered = Endpoint recuperado de { $from } para saudável
events-position-entry-not-landed = A compra de { $symbol } não chegou à blockchain; a posição foi removida

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = Swap
events-category-transaction = Transação
events-category-pool = Pool
events-category-position = Posição
events-category-token = Token
events-category-wallet = Carteira
events-category-trader = Trader
events-category-entry = Entrada
events-category-system = Sistema
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = Segurança
events-category-connectivity = Conectividade
events-category-filtering = Filtragem
events-category-scheduled-task = Tarefa agendada
events-category-learner = Learner
events-category-other = Outros

events-loading = Carregando eventos...
events-load-failed = Falha ao carregar eventos
events-load-failed-description = Aguardando resposta do backend. Vamos tentar novamente automaticamente.
events-load-error = Não foi possível carregar os eventos
events-search-placeholder = Buscar eventos...
events-summary-total = Total
events-filter-category = Categoria
events-filter-all-categories = Todas as categorias
events-filter-all-severities = Todas as gravidades
events-col-time = Hora
events-col-category = Categoria
events-col-type = Tipo
events-col-severity = Gravidade
events-col-message = Mensagem
events-col-token = Token
events-col-details = Detalhes
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count } a mais

## Event details dialog (ui/events_dialog.js)

events-dialog-title = Detalhes do evento
events-dialog-close =
    .aria-label = Fechar janela
events-dialog-payload = Payload
events-dialog-copy = Copiar detalhes
events-dialog-copy-title =
    .title = Copiar todos os detalhes do evento
events-dialog-copy-done = Copiado!
events-dialog-copy-failed = Falhou
events-dialog-not-available = N/D
# $category is the category label; shown when an event has no message.
events-dialog-category-event = Evento de { $category }
events-dialog-field-id = ID do evento
events-dialog-field-severity = Gravidade
events-dialog-field-category = Categoria
events-dialog-field-subtype = Subtipo
events-dialog-field-mint = Mint do token
events-dialog-field-reference = Referência
events-dialog-field-time = Hora do evento
events-dialog-field-age = Idade
events-dialog-field-created = Criado
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = DETALHES DO EVENTO
events-dialog-export-message = MENSAGEM
events-dialog-export-payload = PAYLOAD
events-dialog-export-line = { $label }: { $value }
