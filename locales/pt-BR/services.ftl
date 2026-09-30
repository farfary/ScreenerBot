# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

services-health-component-unavailable = Componente { $component } indisponível
services-health-unavailable = Status de saúde indisponível
services-health-pools-not-running = O serviço de pools não está em execução
services-health-events-db-uninitialized = Banco de dados de eventos não inicializado
services-health-sol-price-not-running = O serviço de preço do SOL não está em execução
services-health-sol-price-stale = Os dados de preço do SOL estão desatualizados ({ $seconds }s de idade)
services-health-sol-price-no-data = Ainda não há dados de preço do SOL disponíveis
services-health-telegram-discovery = Modo de descoberta
services-health-telegram-disconnected = Desconectado
services-health-wallet-watch-polling-only = Detecção funcionando apenas por consulta periódica
services-health-assistant-tasks-disabled = Desativado na configuração
services-health-connectivity-critical-unhealthy = Endpoints críticos com problemas: { $endpoints }
services-health-filtering-snapshot-stale = O snapshot de filtragem tem { $seconds }s de idade

## Services page (pages/services.js)

services-status-healthy = Saudável
services-status-starting = Iniciando
services-status-degraded = Degradado
services-status-unhealthy = Com problemas
services-status-stopping = Parando
services-status-disabled = Desativado
services-status-unknown = Desconhecido

services-name-account = Conta
services-name-assistant-scheduled-tasks = Tarefas agendadas do Assistente
services-name-ata-cleanup = Limpeza de contas de token
services-name-connectivity = Conectividade
services-name-copy-trading = Copy trading
services-name-events = Eventos
services-name-filtering = Filtragem
services-name-llm-analysis = Análise por LLM
services-name-ohlcv = OHLCV
services-name-pool-analyzer = Analisador de pools
services-name-pool-calculator = Calculadora de pools
services-name-pool-discovery = Descoberta de pools
services-name-pool-fetcher = Coletor de pools
services-name-pools = Pools
services-name-positions = Posições
services-name-referral = Indicações
services-name-rpc-stats = Estatísticas de RPC
services-name-sol-price = Preço do { -sol }
services-name-telegram = { -telegram }
services-name-tokens = Tokens
services-name-trader = Trader
services-name-transactions = Transações
services-name-update-check = Verificação de atualizações
services-name-wallet = Carteira
services-name-wallet-watch = Monitoramento de carteiras
services-name-webserver = Servidor web

services-loading = Carregando serviços...
services-load-failed = Falha ao carregar os serviços
services-load-failed-description = Aguardando resposta do backend. Tentaremos novamente automaticamente.
services-refresh-failed = Não foi possível atualizar os serviços
services-search-placeholder = Buscar serviços...
services-summary-total = Total
services-summary-alerts = Alertas
services-summary-alerts-tooltip = { $degraded } degradados / { $unhealthy } com problemas
services-filter-status = Status
services-filter-all-statuses = Todos os status
services-filter-all-services = Todos os serviços
services-filter-enabled-only = Somente ativados
services-filter-disabled-only = Somente desativados
services-col-service = Serviço
services-col-health = Saúde
services-col-priority = Prioridade
services-col-uptime = Tempo ativo
services-col-activity = Atividade
services-col-last-cycle = Último ciclo
services-col-avg-cycle = Ciclo méd.
services-col-avg-poll = Consulta méd.
services-col-cycle-rate = Taxa de ciclos
services-col-tasks = Tarefas
services-col-ops = Ops/s
services-col-errors = Erros
services-col-dependencies = Dependências
services-dependencies-none = Nenhuma
services-activity-busy = { $percent } ocupado
services-activity-polls =
    { $count ->
        [one] { $count } consulta
        [many] { $count } consultas
       *[other] { $count } consultas
    }
services-tasks-tooltip =
    { $count ->
        [one] { $count } tarefa
        [many] { $count } tarefas
       *[other] { $count } tarefas
    }
    Última: { $last }
    Méd.: { $avg }
    Consulta: { $poll }
    Ociosa: { $idle }
    Total de consultas: { $polls }
services-tasks-none = Nenhuma tarefa instrumentada
