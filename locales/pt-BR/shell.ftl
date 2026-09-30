# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = Abrir a página inicial do painel
    .title = Página inicial do painel
shell-bot-card =
    .aria-label = Carregando status do Trader automático
shell-bot-label = Auto
shell-bot-status-loading = CARREGANDO
shell-bot-today = Hoje
shell-explore-control =
    .aria-label = Modo Explorar. Conecte uma carteira e um endpoint RPC para habilitar todos os recursos
    .title = Conecte uma carteira e um endpoint RPC para habilitar trading, saldos e dados on-chain ao vivo
shell-explore-title = Modo Explorar
shell-explore-detail = Carteira e RPC não conectados
shell-explore-action = Concluir configuração
shell-wallet-card =
    .aria-label = Valor da carteira; abrir Posições
    .title = Valor da carteira ({ -sol } + tokens) · abrir Posições
shell-wallet-worth-label = VALOR
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = TKN
shell-sol-price-card =
    .aria-label = Preço do { -sol } em USD: abrir gráfico
    .title = Preço do { -sol } · clique para ver o gráfico
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = Copy trading; abrir Copy Trading
    .title = Copy trading · abrir Copy Trading
shell-copy-label = CÓPIA
shell-actions-more =
    .aria-label = Mais ações do cabeçalho
    .title = Mais ações
shell-actions-group =
    .aria-label = Ações do cabeçalho
shell-action-search =
    .aria-label = Buscar tokens
    .title = Buscar tokens (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = Tokens em destaque
    .title = Tokens em destaque
shell-action-notifications =
    .aria-label = Ações e notificações
    .title = Ações e notificações
shell-action-restart =
    .aria-label = Reiniciar app
    .title = Reiniciar app
shell-action-theme =
    .aria-label = Alternar tema
    .title = Alternar tema
shell-action-settings =
    .aria-label = Configurações
    .title = Configurações

## Ticker

shell-ticker-monitoring-segment =
    .title = Tokens monitorados pelo serviço de pools
shell-ticker-monitoring = Monitorando:
shell-ticker-filtering-segment =
    .title = Tokens que passaram/falharam nos critérios de filtragem
shell-ticker-passed = Aprovados:
shell-ticker-rejected = Rejeitados:
shell-ticker-pnl-segment =
    .title = Lucro e prejuízo realizado hoje
shell-ticker-pnl = P&L de hoje:
shell-ticker-rpc-segment =
    .title = Chamadas RPC por minuto e taxa de sucesso
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/min
shell-ticker-services-segment =
    .title = Status de saúde dos serviços em segundo plano
shell-ticker-services-loading = Serviços: <strong>Carregando</strong>

## Notification drawer

shell-notification-title = Ações
shell-notification-mark-all-read =
    .title = Marcar tudo como lido
shell-notification-clear-all =
    .title = Limpar tudo
shell-notification-close =
    .aria-label = Fechar
shell-notification-tab-all = Todas
shell-notification-tab-active = Ativas
shell-notification-tab-done = Concluídas
shell-notification-tab-failed = Com falha
shell-notification-filter-type-all = Todos os tipos
shell-notification-filter-type-buy = Compra
shell-notification-filter-type-sell = Venda
shell-notification-filter-type-open = Abertura
shell-notification-filter-type-close = Fechamento
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = Parcial
shell-notification-filter-state-all = Todos os estados
shell-notification-filter-state-in-progress = Em andamento
shell-notification-filter-state-completed = Concluído
shell-notification-filter-state-failed = Falhou
shell-notification-filter-state-cancelled = Cancelado
shell-notification-list =
    .aria-label = Notificações
shell-notification-empty = Nenhuma ação ainda
shell-notification-loading-more = Carregando mais...
shell-notification-back-to-top =
    .title = Voltar ao topo

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = Ativo
shell-status-bar-memory = Mem
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/min
shell-status-bar-trading = Trading
shell-status-bar-positions = Pos
shell-status-bar-tokens = Tokens

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = Iniciando o { -brand }
shell-splash-waiting = Aguardando o core local responder.
shell-splash-failed = Não foi possível iniciar o { -brand }
shell-splash-failed-detail = Verifique o arquivo de log e reinicie o app.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = Core conectado
shell-connection-waiting = Aguardando o core…
shell-connection-retry-now = Tentar agora
shell-connection-overlay-detail = O core está inacessível. O trading está pausado; a conexão será recuperada automaticamente.
shell-connection-restored = Conexão com o core restaurada

# Source: scripts/core/header.js
shell-trader-control-failed = Falha no controle do trader
shell-notification-button-unread = Ações e notificações, { $count } não lidas
shell-restart-confirm-title = Reiniciar bot
shell-restart-confirm-message =
    Tem certeza de que deseja reiniciar o bot?

    Isso vai:
    • Parar todos os serviços
    • Reiniciar o processo
    • Levar ~10-15 segundos

    Todas as operações ativas serão interrompidas.
shell-restart-confirm-action = Reiniciar
shell-restart-progress = Reiniciando o bot
shell-restart-failed = Falha ao reiniciar
shell-restart-failed-status = Falha ao reiniciar: { $status }
shell-restart-helper-unavailable = O auxiliar de reinicialização automática está indisponível. Recarregue o painel em instantes.

# Source: scripts/core/router.js
shell-page-title-fallback = Painel
shell-page-load-failed = Falha ao carregar a página
shell-page-offline-detail = O core está inacessível no momento. Esta página será carregada automaticamente quando a conexão voltar.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = EXPLORAR
shell-bot-state-halted = INTERROMPIDO
shell-bot-state-off = DESATIVADO
shell-bot-state-waiting = AGUARDANDO
shell-bot-state-idle = OCIOSO
shell-bot-state-entry-paused = ENTRADAS PAUSADAS
shell-bot-state-running = EM EXECUÇÃO
shell-bot-control-explore = Trader automático indisponível no Modo Explorar. Abra a configuração de carteira e RPC.
shell-bot-control-halted = A parada de emergência está ativa. Abra os controles do Trader automático.
shell-bot-control-off = O Trader automático está desativado. Clique para ativá-lo.
shell-bot-control-waiting = O Trader automático está ativado e aguardando os serviços do core. Clique para desativá-lo.
shell-bot-control-idle = O Trader automático está ativado, mas os dois monitores estão desativados. Abra os controles do Trader automático.
shell-bot-control-entry-paused = A proteção contra perdas pausou as entradas; as saídas podem continuar. Abra os controles do Trader automático.
shell-bot-control-running = O Trader automático está em execução. Clique para desativá-lo.

## Wallet and copy cards

shell-wallet-card-summary = Valor da carteira: { $equity } { -sol } ({ $balance } { -sol } em caixa, { $tokens } em tokens); abrir Posições
shell-copy-running-live = { $count } reais
shell-copy-running-paper = { $count } simuladas
shell-copy-value-paused = Pausado
shell-copy-value-idle = Ocioso
shell-copy-sub-active = { $active } de { $total } ativas

## Ticker services state

shell-ticker-services-healthy = Serviços: <strong>Saudáveis</strong>
shell-ticker-services-issues =
    { $count ->
        [one] Serviços: <strong>{ $count } problema</strong>
        [many] Serviços: <strong>{ $count } problemas</strong>
       *[other] Serviços: <strong>{ $count } problemas</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = Solicitação de agente
shell-agent-request-client-fallback = Um agente pareado
shell-agent-request-message = { $client } quer executar "{ $tool }" no { -brand }. Esta solicitação { $expiry }.
shell-agent-request-message-arguments = { $client } quer executar "{ $tool }" no { -brand }. Argumentos: { $summary }. Esta solicitação { $expiry }.
shell-agent-request-expires-minutes = expira em { $minutes }min
shell-agent-request-expires-seconds = expira em { $seconds }s
shell-agent-request-approve = Aprovar
shell-agent-request-deny = Negar

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } copiado
shell-toast-copy-failed = Falha ao copiar
shell-toast-still-running = Ainda em execução: confira a central de notificações
shell-toast-dismiss =
    .aria-label = Dispensar
shell-confirm-title = Confirmar ação
shell-confirm-message = Tem certeza?
shell-address-open-solscan = — abrir no { -solscan }
shell-address-copy = Copiar endereço

# Source: scripts/core/global_chat.js
shell-assistant-label = Assistente
shell-assistant-dialog =
    .aria-label = Assistente

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = Ativo
shell-status-bar-trading-inactive = Inativo

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } cancelado
shell-action-swap-buy-live = Comprando
shell-action-swap-buy-done = Comprado
shell-action-swap-buy-failed = Falha na compra
shell-action-swap-sell-live = Vendendo
shell-action-swap-sell-done = Vendido
shell-action-swap-sell-failed = Falha na venda
shell-action-position-open-live = Abrindo posição
shell-action-position-open-done = Aberta
shell-action-position-open-failed = Falha ao abrir
shell-action-position-close-live = Fechando posição
shell-action-position-close-done = Fechada
shell-action-position-close-failed = Falha ao fechar
shell-action-position-dca-live = Fazendo aporte na posição
shell-action-position-dca-done = Aporte feito em
shell-action-position-dca-failed = Falha no aporte
shell-action-partial-exit-live = Saída parcial
shell-action-partial-exit-done = Saída parcial
shell-action-partial-exit-failed = Falha na saída parcial
shell-action-manual-order-live = Enviando ordem
shell-action-manual-order-done = Ordem enviada
shell-action-manual-order-failed = Falha na ordem
shell-action-trade-live = Trade
shell-action-trade-done = Trade concluído
shell-action-trade-failed = Falha no trade
shell-action-via-router = { $action } via { $router }
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = evitando { $venue }
shell-action-cost-guard-avoiding-cost = evitando { $venue } · { $cost }
shell-action-cost-guard-avoiding-unnamed = evitando uma plataforma
shell-action-cost-guard-avoiding-unnamed-cost = evitando uma plataforma · { $cost }
shell-action-cost-guard-avoided = { $outcome } · evitou { $cost } de aluguel em { $venue }
shell-action-cost-guard-avoided-unnamed = { $outcome } · evitou { $cost } de aluguel de plataforma
shell-action-exit-full = Saída total
shell-action-exit-percent = Saída de { $percent }

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = Fechar o { -brand }?
shell-exit-description = Escolha como deseja fechar o aplicativo
shell-exit-minimize = Minimizar para a bandeja
shell-exit-minimize-detail = Continuar em segundo plano
shell-exit-quit = Sair do app
shell-exit-quit-detail = Fechar por completo e parar todos os serviços

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = Salvar imagem
shell-lightbox-close =
    .title = Fechar (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = Claro
shell-theme-dark = Escuro
shell-theme-switch-to-light = Mudar para o tema claro
shell-theme-switch-to-dark = Mudar para o tema escuro
