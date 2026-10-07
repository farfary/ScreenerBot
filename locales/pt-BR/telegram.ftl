# Telegram bot text. Server-only: rendered by src/telegram/text.rs, never sent to the dashboard.
#
# Messages are sent as Telegram HTML. The only tags are b, i, u, s, code and pre,
# without attributes; links are built in Rust. A line break is a literal newline.
# Icons are prepended by Rust and never appear here. Copyable values (chat ids)
# arrive as arguments and are wrapped in code inside the message. Keep the
# command names (/status) and the literal ampersand placeable unchanged.

## Reply keyboard words. Icons come from ReplyCommand in src/telegram/reply.rs.

telegram-reply-status = Status
telegram-reply-balance = Saldo
telegram-reply-positions = Posições
telegram-reply-pause = Pausar
telegram-reply-resume = Retomar
telegram-reply-stop = Parar
telegram-reply-stats = Estatísticas
telegram-reply-menu = Menu
telegram-reply-help = Ajuda

## Inline keyboard buttons.

telegram-button-positions = Posições
telegram-button-balance = Saldo
telegram-button-stats = Estatísticas
telegram-button-tokens = Tokens
telegram-button-pause = Pausar
telegram-button-stop = Parar
telegram-button-settings = Configurações
telegram-button-refresh = Atualizar
telegram-button-menu = Menu
telegram-button-back = Voltar
telegram-button-back-to-menu = Voltar ao menu
telegram-button-back-to-tokens = Voltar aos tokens
telegram-button-cancel = Cancelar
telegram-button-close-all-positions = Fechar todas as posições
telegram-button-sell-percent = Vender { $percent }%
telegram-button-dca-amount = DCA { $amount }
telegram-button-blacklist = Lista negra
telegram-button-blacklist-symbol = Lista negra: { $symbol }
telegram-button-close-position = Fechar posição
telegram-button-confirm-close = Confirmar fechamento
telegram-button-confirm-close-all = Fechar TODAS as posições
telegram-button-confirm-sell = Confirmar venda de { $percent }%
telegram-button-confirm-dca = DCA { $amount } { -sol }
telegram-button-confirm-force-stop = CONFIRMAR PARADA FORÇADA
telegram-button-confirm-buy = Comprar { $amount } { -sol }
telegram-button-notifications = Notificações
telegram-button-trading = Trading
telegram-button-entry-monitor = Monitor de entrada
telegram-button-exit-monitor = Monitor de saída
telegram-button-auto-trading = Trading automático
telegram-button-force-stop = Parada forçada
telegram-button-notify-opened = Abertas
telegram-button-notify-closed = Fechadas
telegram-button-notify-partial = Parciais
telegram-button-notify-dca = DCA
telegram-button-notify-errors = Erros
telegram-button-details = Detalhes
telegram-button-position = Posição
telegram-button-sell-more = Vender mais
telegram-button-more-dca = Mais DCA
telegram-button-history = Histórico
telegram-button-status = Status
telegram-button-solscan = { -solscan }
telegram-button-dexscreener = { -dexscreener }
telegram-button-reauthenticate = Autenticar novamente
telegram-button-previous = Ant.
telegram-button-next = Próx.
telegram-button-passed = Aprovados
telegram-button-rejected = Rejeitados
telegram-button-new-24h = Novos (24h)
telegram-button-all-tokens = Todos os tokens
telegram-button-search-token = Buscar token
telegram-button-filter-stats = Estatísticas do filtro
telegram-button-refresh-stats = Atualizar estatísticas
telegram-button-view-position = Ver posição
telegram-button-buy-amount = { $amount } { -sol }

## Command router and authentication.

telegram-unknown-command =
    Comando desconhecido: { $command }

    Use /help para ver os comandos disponíveis.
telegram-session-expired =
    <b>Sessão expirada</b>

    Use /login para se autenticar novamente.
telegram-2fa-required =
    <b>2FA necessário</b>

    Informe o código de 6 dígitos do seu autenticador.
telegram-account-locked =
    <b>Conta bloqueada</b>

    Muitas tentativas malsucedidas.
    Tente novamente em { $seconds ->
        [one] { $seconds } segundo.
        [many] { $seconds } segundos.
       *[other] { $seconds } segundos.
    }
telegram-code-invalid = Informe um código válido de 6 dígitos.
telegram-authenticated =
    <b>Autenticado!</b>

    Agora você tem acesso aos comandos do bot.
telegram-wrong-code =
    <b>Código incorreto</b>

    { $remaining ->
        [one] { $remaining } tentativa restante.
        [many] { $remaining } tentativas restantes.
       *[other] { $remaining } tentativas restantes.
    }
telegram-auth-required =
    <b>Autenticação necessária</b>

    Informe sua senha para continuar.

    <i>Digite sua senha e envie.</i>
telegram-login-required =
    <b>Login necessário</b>

    Informe o código de 6 dígitos do seu autenticador:
telegram-session-activated =
    <b>Sessão ativada</b>

    O 2FA não está configurado. Sua sessão agora está ativa.

    <i>Dica: ative o 2FA nas configurações de segurança para mais proteção.</i>

## Chat discovery.

telegram-discovery-hello = Olá, { $name }!
telegram-discovery-default-name = Usuário
telegram-discovery-detected = <b>Chat detectado!</b>
telegram-discovery-details =
    Chat ID: <code>{ $chat_id }</code>
    Tipo: { $chat_type }

    Acesse o painel do { -brand } e clique neste chat para selecioná-lo.
telegram-chat-type-private = privado
telegram-chat-type-group = grupo
telegram-chat-type-supergroup = supergrupo
telegram-chat-type-channel = canal

## Menus.

telegram-menu-title =
    <b>Painel de controle</b>

    Selecione uma opção para ver informações ou controlar o bot.
telegram-menu-positions-empty =
    <b>Nenhuma posição aberta</b>

    Aguardando novas oportunidades...
telegram-menu-positions-title = <b>Posições ({ $count })</b>
telegram-menu-positions-hint = <i>Toque em uma posição para gerenciá-la.</i>
telegram-menu-settings =
    <b>Configurações</b>

    Configure as notificações e os parâmetros de trading.
telegram-settings-notifications =
    <b>Configurações de notificações</b>

    Ative/desative as notificações:
telegram-settings-trading =
    <b>Controles de trading</b>

    Ative/desative os recursos de trading:
telegram-pagination-expired = A sessão de paginação expirou.

## Status commands.

telegram-status-state-stopped = <b>PARADO</b> (parada forçada ativa)
telegram-status-state-active = <b>ATIVO</b>
telegram-status-state-paused = <b>PAUSADO</b>
telegram-status-on = Ativado
telegram-status-off = Desativado
telegram-status-body =
    <b>Status do sistema</b>

    <b>Sistema</b>
    Estado — { $state }
    Tempo ativo — { $uptime }
    Versão — v{ $version }

    <b>Trading</b>
    Entradas — { $entries }
    Saídas — { $exits }
    Posições — { $positions }
telegram-positions-empty =
    <b>Nenhuma posição aberta</b>

    Aguardando oportunidades...
telegram-positions-title = <b>Posições abertas ({ $count })</b>
telegram-positions-row =
    <b>{ $symbol }</b>
       { $pnl_sol } { -sol } ({ $pnl_pct }%)
telegram-positions-more = <i>+{ $count } mais...</i>
telegram-positions-summary =
    <b>Resumo do portfólio</b>
    Investido — { $invested } { -sol }
    P{ "&amp;" }L líquido — { $pnl } { -sol }
telegram-balance-body =
    <b>Saldo da carteira</b>

    <b>{ $sol } { -sol }</b>
    ≈ ${ $usd } USD
telegram-stats-body =
    <b>Estatísticas do dia</b>

    Posições — { $positions }
    Investido — { $invested } { -sol }
    P{ "&amp;" }L — { $pnl } { -sol }

## Trading controls.

telegram-start-ready =
    <b>{ -brand } está pronto!</b>

    O trading está <b>ativado</b>.

    Use o teclado abaixo para controlar o bot.
    Digite /help para ver os comandos disponíveis.
telegram-stop-already = <b>O trading já está desativado</b>
telegram-stop-done =
    <b>Trading desativado</b>

    Todos os monitores de trading (entradas { "&amp;" } saídas) foram parados.
    Use /pause para parar apenas as entradas.
telegram-stop-failed =
    <b>Falha ao desativar o trading</b>

    Erro: { $detail }
telegram-pause-done =
    <b>Monitor de entrada pausado</b>

    Nenhuma nova posição será aberta.
    O monitor de saída continua em execução.
telegram-pause-failed =
    <b>Falha ao pausar as entradas</b>

    Erro: { $detail }
telegram-resume-done =
    <b>Monitor de entrada retomado</b>

    Agora observando sinais de entrada.
telegram-resume-failed =
    <b>Falha ao retomar as entradas</b>

    Erro: { $detail }
telegram-force-stop-confirm =
    <b>PARADA FORÇADA</b>

    Isso interromperá imediatamente TODA a atividade de trading:
    • Nenhuma nova entrada
    • Nenhuma saída (incluindo stop loss)
    • Nenhuma operação de DCA
telegram-force-stop-warning = <b>Esta é uma ação de emergência!</b>
telegram-force-stop-question = Tem certeza?
telegram-force-stop-active =
    <b>PARADA FORÇADA ATIVADA</b>

    Todo o trading foi interrompido.

    Use /resume_trading para limpar esta trava.
telegram-resume-trading-not-stopped =
    <b>O trading não está em parada forçada</b>

    Nenhuma ação necessária.
telegram-resume-trading-done =
    <b>Trading retomado</b>

    A trava de parada forçada foi removida.
    As operações normais de trading podem ser retomadas.

## Help.

telegram-help-title = <b>Ajuda do { -brand }</b>
telegram-help-heading-dashboard = Painel
telegram-help-heading-market = Mercado
telegram-help-heading-trading = Trading
telegram-help-heading-safety = Segurança
telegram-help-heading-system = Sistema
telegram-help-commands-dashboard =
    /status — Status do sistema { "&amp;" } tempo ativo
    /stats — Desempenho do dia
    /balance — Saldo da carteira
    /positions — Posições abertas
telegram-help-commands-market =
    /tokens — Explorador de tokens
    /rejected — Tokens filtrados
telegram-help-commands-trading =
    /start — Ativar o sistema de trading
    /stop — Desativar o sistema de trading
    /pause — Pausar novas entradas
    /resume — Retomar novas entradas
    /menu — Menu interativo
telegram-help-commands-safety =
    /force_stop — <b>PARADA DE EMERGÊNCIA</b>
    /resume_trading — Limpar o status de emergência
telegram-help-commands-system =
    /update — Status da atualização { "&amp;" } instalação
    /login — Autenticação 2FA
telegram-help-tip = <i>Dica: toque em um comando para executá-lo.</i>

## The /update command.

telegram-update-up-to-date-auto =
    <b>Tudo atualizado</b>

    Executando a v{ $version }, instalada automaticamente.
telegram-update-up-to-date =
    <b>Tudo atualizado</b>

    Executando a v{ $version }.
telegram-update-check-failed =
    <b>Falha ao verificar atualizações</b>

    { $reason }
telegram-update-unreachable = Não foi possível acessar screenerbot.io.
telegram-update-installing = <b>Instalando a v{ $version }</b>
telegram-update-restarting =
    O { -brand } está reiniciando na nova versão. O trading é retomado automaticamente.
telegram-update-install-failed =
    <b>Não foi possível instalar a v{ $version }</b>

    { $detail }
telegram-update-downloaded =
    <b>A v{ $version } foi baixada</b>

    Esta versão também atualiza o app desktop, então o instalador precisa ser executado na máquina. Abra Configurações → Atualizações por lá.
telegram-update-downloading =
    <b>Baixando a v{ $version }</b>

    { $percent }% de { $size } MB.
telegram-update-available =
    <b>A v{ $version } está disponível</b>

    { $how }
    Tamanho do download: { $size } MB.

    O download é feito automaticamente; envie /update novamente quando estiver pronto.
telegram-update-how-core = Instala em segundo plano, com uma reinicialização rápida.
telegram-update-how-installer = Exige executar o instalador do desktop uma vez.

## Shared values and units. Numbers arrive formatted; only the unit words live here.

telegram-value-unknown = Desconhecido
telegram-value-na = N/D
telegram-percent-value = { $percent }%
telegram-price-native = { $price } { -sol }
telegram-amount-usd = ${ $amount }
telegram-amount-usd-thousands = ${ $amount }K
telegram-amount-usd-millions = ${ $amount }M
telegram-duration-seconds = { $seconds }s
telegram-duration-minutes = { $minutes }min
telegram-duration-minutes-seconds = { $minutes }min { $seconds }s
telegram-duration-hours = { $hours }h
telegram-duration-hours-minutes = { $hours }h { $minutes }min
telegram-duration-days = { $days }d
telegram-duration-days-hours = { $days }d { $hours }h
telegram-pnl = { $sol } { -sol } ({ $percent }%)
telegram-amount-native = { $amount } { -sol }
telegram-error-line = Erro: { $detail }
telegram-ai-reasoning =
    <b>Análise do LLM</b>
    <i>{ $reasoning }</i>

## Rows shared by several notification and position screens. Icons come from Rust.

telegram-row-entry = Entrada — { $price } { -sol }
telegram-row-exit = Saída — { $price } { -sol }
telegram-row-current = Atual — { $price } { -sol }
telegram-row-invested = Investido — { $amount } { -sol }
telegram-row-received = Recebido — { $amount } { -sol }
telegram-row-value = Valor — { $amount } { -sol }
telegram-row-total = Total — { $amount } { -sol }
telegram-row-tokens = Tokens — { $tokens }
telegram-row-duration = Duração — { $duration }
telegram-row-reason = Motivo — { $reason }
telegram-row-remaining = Restante — { $percent }%
telegram-row-pnl = P{ "&amp;" }L — { $pnl }
telegram-row-dca = DCA — #{ $count }

## Notifications.

telegram-notify-opened-title = <b>Posição aberta</b>
telegram-notify-opened-size = Tamanho — <b>{ $amount } { -sol }</b>
telegram-notify-opened-price = Preço — { $price } { -sol }
telegram-notify-opened-dex = DEX — { $dex }
telegram-notify-closed-title-profit = <b>Posição fechada</b> — Lucro
telegram-notify-closed-title-loss = <b>Posição fechada</b> — Prejuízo
telegram-notify-closed-reason-unspecified = Fechada
telegram-notify-partial-title = <b>Saída parcial</b>
telegram-notify-partial-sold = <b>${ $symbol }</b> — Vendido { $percent }%
telegram-notify-dca-title = <b>DCA #{ $count }</b>
telegram-notify-dca-added = Aporte — <b>{ $amount } { -sol }</b>
telegram-notify-dca-avg = Méd. — { $price } { -sol }
telegram-notify-unbooked-title = <b>Swap não registrado</b>
telegram-notify-unbooked-body = Um swap confirmado na blockchain ainda não está na posição. Ele é verificado de novo até ser registrado.
telegram-notify-unbooked-signature = Transação: <code>{ $signature }</code>
telegram-notify-severity-critical = <b>Erro crítico</b>
telegram-notify-severity-error = <b>Erro</b>
telegram-notify-severity-warning = <b>Alerta</b>
telegram-notify-severity-info = <b>Informação</b>
telegram-notify-alert-title = <b>Alerta de trade</b>
telegram-notify-alert-token = Token: <code>${ $symbol }</code>
telegram-notify-alert-mint = Mint: <code>{ $mint }</code>
telegram-notify-alert-bought = Ação: comprou { $amount } { -sol }
telegram-notify-alert-sold = Ação: vendeu { $amount } { -sol }
telegram-notify-alert-wallet = Carteira: <code>{ $wallet }</code>
telegram-notify-copy-header-paper = <b>{ $title }</b> (paper)
telegram-notify-copy-task = Tarefa: { $task }
telegram-notify-scheduled-completed = <b>Tarefa agendada concluída</b>
telegram-notify-scheduled-failed = <b>Tarefa agendada falhou</b>
telegram-notify-scheduled-timed-out = <b>Tarefa agendada expirou</b>
telegram-notify-scheduled-error = Erro: { $error }
telegram-notify-summary-title = <b>Resumo diário</b> — { $date }
telegram-notify-summary-performance = <b>Desempenho</b>
telegram-notify-summary-trades = Trades — { $total } ({ $wins }{ $win_icon } { $losses }{ $loss_icon })
telegram-notify-summary-win-rate = Taxa de acerto — { $percent }%
telegram-notify-summary-pnl = P{ "&amp;" }L — <b>{ $amount } { -sol }</b> { $icon }
telegram-notify-summary-open = Posições abertas — { $count }
telegram-notify-started-title = <b>{ -brand } iniciado</b>
telegram-notify-started-version = <b>Versão</b> — { $version }
telegram-notify-started-mode = <b>Modo</b> — { $mode }
telegram-notify-started-ready = Pronto para o trading!
telegram-notify-stopped-title = <b>{ -brand } parado</b>
telegram-notify-stopped-reason = <b>Motivo</b> — { $reason }
telegram-notify-stopped-goodbye = Até logo! { $icon }
telegram-notify-start-mode-normal = Normal
telegram-notify-stop-reason-graceful = Encerramento normal
telegram-notify-update-available =
    <b>Atualização v{ $version } disponível</b>

    { $how }
    Tamanho do download: { $size } MB
telegram-notify-update-how-installer = Esta versão também atualiza o app desktop, então o instalador precisa ser executado uma vez.
telegram-notify-update-ready =
    <b>Atualização v{ $version } pronta</b>

    { $how }
telegram-notify-update-ready-silent = Envie /update para aplicá-la agora, ou ela será instalada na próxima vez que o { -brand } iniciar.
telegram-notify-update-ready-installer = Abra Configurações → Atualizações para executar o instalador.
telegram-notify-update-applying =
    <b>Instalando a v{ $version }</b>

    O backend está reiniciando; o trading é retomado automaticamente.
telegram-notify-new-tokens =
    <b>Alerta de filtragem</b>

    { $count ->
        [one] { $count } novo token encontrado que atende aos seus critérios.
        [many] { $count } novos tokens encontrados que atendem aos seus critérios.
       *[other] { $count } novos tokens encontrados que atendem aos seus critérios.
    }
telegram-notify-crash =
    <b>O bot travou!</b>

    <b>Local:</b> <code>{ $location }</code>
    <b>Erro:</b> <code>{ $error }</code>
telegram-notify-crash-restart = Reinicie o bot.

## Filter results page.

telegram-filter-results-title = <b>Resultados do filtro</b> ({ $count })
telegram-filter-results-empty = <i>Nenhum token encontrado.</i>
telegram-filter-results-page = <i>Página { $page } de { $total }</i>

## Position screens.

telegram-position-not-found = Posição não encontrada
telegram-position-no-positions = Nenhuma posição para fechar
telegram-position-history-empty =
    <b>Histórico de trades</b>

    Nenhuma posição fechada ainda.
telegram-position-history-title = <b>Trades recentes</b>
telegram-position-history-more =
    <i>+{ $count } { $count ->
        [one] trade a mais
        [many] trades a mais
       *[other] trades a mais
    }...</i>
telegram-position-confirm-hint = <i>Confirme em até 30s para executar.</i>
telegram-position-confirm-close-title = <b>Fechar posição?</b>
telegram-position-confirm-close-selling = Vendendo { $tokens } tokens
telegram-position-confirm-close-estimated = Estimado — <b>{ $amount } { -sol }</b>
telegram-position-confirm-close-hint = <i>Confirme em até 30 segundos</i>
telegram-position-confirm-sell =
    <b>Confirmar venda</b>

    Token — { $symbol }
    Valor — { $percent }%
    Tokens — { $tokens }
telegram-position-confirm-dca =
    <b>Confirmar compra adicional</b>

    Token — { $symbol }
    Aporte — { $amount } { -sol }
telegram-position-confirm-close-all =
    <b>Fechar todas as posições?</b>

    Quantidade — { $count }
telegram-position-confirm-close-all-hint =
    <i>Isso venderá a mercado todas as posições abertas.
    Confirme em até 30s.</i>
telegram-position-confirm-force-stop =
    <b>PARADA FORÇADA</b>

    Isso interromperá imediatamente TODO o trading:
    • Nenhuma nova entrada
    • Nenhuma saída
    • Nenhum DCA
telegram-position-confirm-force-stop-warning = <b>Esta é uma ação de emergência.</b>
telegram-position-confirm-blacklist =
    <b>Adicionar token à lista negra?</b>

    Token — { $symbol }
    Mint — <code>{ $mint }</code>
telegram-position-confirm-blacklist-hint = <i>Isso fechará a posição e impedirá novas entradas.</i>
telegram-position-selling = Vendendo { $percent }% de { $symbol }...
telegram-position-sell-done =
    <b>Venda executada</b>

    Token — { $symbol }
    Vendido — { $percent }%
    Recebido — { $amount } { -sol }
telegram-position-sell-failed = <b>Falha na venda</b>
telegram-position-adding = Aportando { $amount } { -sol } em { $symbol }...
telegram-position-dca-done =
    <b>DCA executado</b>

    Token — { $symbol }
    Aporte — { $amount } { -sol }
telegram-position-dca-failed = <b>Falha no DCA</b>
telegram-position-closing-all = Fechando todas as posições...
telegram-position-close-all-done =
    <b>Fechamento total concluído</b>

    Fechadas — { $closed }
    Com falha — { $failed }
telegram-position-blacklisted =
    <b>Token na lista negra</b>

    Token — { $symbol }
    Status — Fechada { "&amp;" } na lista negra

## Token screens.

telegram-token-not-found = Token não encontrado
telegram-token-not-found-prefix = Token não encontrado. Tente buscar com um prefixo mais longo.
telegram-token-stats-failed = Falha ao buscar as estatísticas: { $detail }
telegram-token-list-failed = Falha ao buscar os tokens: { $detail }
telegram-token-list-empty = Nenhum token encontrado na visão <b>{ $view }</b>.
telegram-token-view-passed = Aprovados no filtro
telegram-token-view-rejected = Rejeitados
telegram-token-view-recent = Adicionados recentemente
telegram-token-view-all = Todos os tokens
telegram-token-list-title = <b>{ $name }</b> (Página { $page }/{ $total })
telegram-token-list-stats = Liq.: { $liquidity } • Preço: { $price }
telegram-token-list-hint = <i>Toque em /token_ID para ver os detalhes</i>
telegram-token-explorer =
    <b>Explorador de mercado</b>

    <b>Visão geral</b>
    Aprovados no filtro — { $passed }
    Rejeitados — { $rejected }
    Preços ativos — { $priced }
    Total descoberto — { $total }

    <i>Selecione uma categoria para navegar:</i>
telegram-token-filter-title = <b>Análise do filtro</b>
telegram-token-filter-distribution = <b>Distribuição</b>
telegram-token-filter-passed = Aprovados — { $count } ({ $percent }%)
telegram-token-filter-rejected = Rejeitados — { $count } ({ $percent }%)
telegram-token-filter-blacklisted = Na lista negra — { $count }
telegram-token-filter-coverage = <b>Cobertura</b>
telegram-token-filter-priced = Com preço do pool — { $count }
telegram-token-filter-open = Posições abertas — { $count }
telegram-token-filter-total = Total descoberto — { $count }
telegram-token-filter-updated = <b>Última atualização</b>
telegram-token-filter-time = { $time } UTC
telegram-token-filter-refresh = <i>Atualiza automaticamente a cada { $interval }</i>
telegram-token-detail-active = <b>Posição ativa</b>
telegram-token-detail-price = Preço — { $price } { -sol }
telegram-token-detail-liquidity = Liquidez — { $value }
telegram-token-detail-volume = Volume 24h — { $value }
telegram-token-detail-change = Variação 24h — { $value }
telegram-token-detail-risk = Avaliação de risco: { $score }/100
telegram-token-detail-risk-unknown = Avaliação de risco: desconhecida
telegram-token-detail-action = <i>Selecione uma ação:</i>
telegram-token-search =
    <b>Buscar no mercado</b>

    Informe o símbolo ou o endereço do mint para buscar:

    <i>Exemplo: /token_BONK ou /token_So11111</i>
telegram-token-confirm-buy =
    <b>Confirmar compra direta</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>
    Valor — { $amount } { -sol }

    <i>Confirme em até 30s para executar.</i>
telegram-token-confirm-blacklist =
    <b>Adicionar token à lista negra?</b>

    Token — ${ $symbol }
    Mint — <code>{ $mint }</code>

    <i>Isso impedirá que este token satisfaça os filtros.</i>
telegram-token-blacklisted =
    <b>Token na lista negra</b>

    Token — ${ $symbol }
    Status — Adicionado à lista negra
telegram-token-blacklist-failed = <b>Falha ao adicionar à lista negra</b>
telegram-token-buy-processing =
    <b>Processando compra...</b>

    Token — ${ $symbol }
    Valor — { $amount } { -sol }
telegram-token-buy-done =
    <b>Compra concluída</b>

    Token — ${ $symbol }
    Valor — { $amount } { -sol }

    <i>Veja os detalhes em /positions</i>
telegram-token-buy-failed =
    <b>Falha na compra</b>

    Token — ${ $symbol }
    Erro — { $detail }
