# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = Não foi possível obter uma cotação: verifique sua conexão e tente novamente
# Fallback title when the quote request failed without a message.
trade-quote-error-title = Não foi possível obter uma cotação
trade-quote-title = Prévia do swap
trade-quote-refresh =
    .aria-label = Atualizar cotação
    .title = Atualizar cotação
trade-quote-idle = Escolha um valor para ver a prévia do swap
trade-quote-loading = Buscando a melhor rota…
trade-quote-retry = Tentar novamente
trade-quote-pay = Você paga
trade-quote-receive = Você recebe (estimado)
trade-quote-minimum = Mínimo garantido
    .title = O mínimo que você pode receber após o slippage máximo. O swap é revertido em vez de executar abaixo desse valor.
trade-quote-impact = Impacto no preço
trade-quote-slippage = Slippage máx.
trade-quote-platform-fee = Taxa da plataforma
    .title = 0.5%: apoia o desenvolvimento. Já incluída na cotação acima.
trade-quote-network-fee = Taxa de rede
trade-quote-route = Rota
trade-quote-disclaimer = Os preços são atualizados ao vivo a partir da chain. O swap é revertido se não puder ser executado acima do seu mínimo garantido, então você nunca recebe menos do que o exibido.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = O impacto no preço de { $impact } está acima do seu slippage máximo de { $tolerance }%: este valor movimenta o pool. Um valor menor é executado mais perto do preço de mercado.

## Units

trade-unit-sol = { -sol }
trade-unit-tokens = tokens

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = Comprar token
trade-buy-subtitle = Informe o valor em { -sol }
trade-buy-confirm = Executar compra
trade-buy-hint = Deixe vazio para usar o padrão da configuração
trade-sell-title = Vender posição
trade-sell-subtitle = Selecione a porcentagem de venda
trade-sell-confirm = Executar venda
trade-sell-hint = Informe um valor entre 1 e 100
trade-sell-input = Porcentagem personalizada
    .placeholder = 1-100
trade-add-title = Aportar na posição
trade-add-subtitle = Faça DCA em uma posição existente
trade-add-confirm = Aportar
trade-add-hint = Deixe vazio para usar o tamanho de DCA configurado
trade-amount-input = Valor personalizado
    .placeholder = Informe o valor em { -sol }

## Presets

trade-presets-quick-amount = Valor rápido
trade-presets-quick-sell = Venda rápida
trade-presets-match-entry = Igualar entrada
trade-presets-fixed-amount = Valor fixo
trade-preset-partial = Parcial
trade-preset-half = Metade
trade-preset-most = Maior parte
trade-preset-full = Saída total
# $label is the preset's amount.
trade-preset-select =
    .aria-label = Selecionar { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = Fechar janela
trade-input-max = MÁX
    .aria-label = Usar o máximo
trade-slider =
    .aria-label = Controle deslizante de valor
trade-context-available = Disponível
trade-context-position-size = Tamanho da posição
trade-context-holdings = Posse
trade-held-badge = Em posse
    .title = Você tem uma posição aberta neste token
trade-manage-title = Gestão manual
trade-manage-description = O Trader automático não vai vender nem fazer DCA nesta posição. Desmarque para deixá-lo gerenciar as saídas.

## Slippage

trade-slippage-label = Slippage
trade-slippage-presets =
    .aria-label = Predefinição de slippage
trade-slippage-auto = Auto
trade-slippage-custom =
    .placeholder = Personalizado
    .aria-label = Porcentagem de slippage personalizada
trade-slippage-note-auto = Auto (das configurações)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = Auto ({ $pct }% das configurações)
# $pct is the override as typed.
trade-slippage-note-override = Substituição: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = Slippage alto: você pode receber até { $pct }% a menos que o cotado.
trade-impact-warning-title = Aviso de alto impacto no preço
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = Este trade tem um impacto no preço de <strong>{ $impact }</strong>, que excede a sua tolerância de slippage de <strong>{ $tolerance }%</strong>. Você pode receber bem menos do que o esperado.
trade-impact-warning-proceed = Continuar mesmo assim

## Validation and verification

trade-error-invalid-number = Número inválido
trade-error-percentage-range = A porcentagem deve estar entre 1 e 100
trade-error-amount-positive = O valor deve ser maior que 0
trade-error-amount-minimum = Mínimo: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = Saldo insuficiente (necessário { $needed } mais { $reserve } para taxas; saldo atual { $balance })
trade-error-position-closed = Esta posição não está mais aberta.
trade-error-verify-failed = Não foi possível verificar o saldo do token
trade-error-position-missing = Posição não encontrada: ela pode ter sido fechada
# $expected and $current are formatted token amounts.
trade-error-balance-changed = O saldo do token mudou. Esperado { $expected }, agora { $current }. Atualize a página.
trade-error-verify-network = Erro de rede ao verificar o saldo

## Quick trade

trade-quick-buy-title = Compra rápida
trade-quick-sell-title = Venda rápida
trade-quick-subtitle = Informe o endereço do mint do token
trade-quick-mint-label = Informe o endereço do mint do token
trade-quick-mint-input =
    .placeholder = Informe o endereço do mint ou busque por símbolo...
trade-quick-paste =
    .aria-label = Colar da área de transferência
trade-quick-recent = Recentes:
trade-quick-fetching = Buscando informações do token...
trade-quick-continue = Continuar
trade-quick-token-not-found = Token não encontrado
trade-quick-token-failed = Falha ao buscar o token
trade-quick-token-not-in-database = Token não encontrado no banco de dados
trade-quick-token-info-failed = Falha ao buscar as informações do token
trade-quick-no-position = Nenhuma posição encontrada para este token
trade-quick-no-holdings = A posição não tem tokens restantes
trade-quick-position-failed = Falha ao buscar os dados da posição

## Manual trade toasts

trade-toast-no-mint = Nenhum endereço de mint disponível
trade-toast-open-failed = Não foi possível abrir a janela de trade
trade-toast-pending-buy = Compra ainda em andamento
trade-toast-pending-add = Aporte ainda em andamento
trade-toast-pending-sell = Venda ainda em andamento
trade-toast-pending-message = O navegador parou de aguardar; acompanhe o resultado na linha da posição
trade-toast-failed-buy = Falha na compra
trade-toast-failed-add = Falha no aporte na posição
trade-toast-failed-sell = Falha na venda

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = Sinal da estratégia
trade-reason-manual-entry = Entrada manual
trade-reason-force-buy = Compra forçada
trade-reason-copy-buy = Compra de cópia
trade-reason-dca-scheduled = DCA agendado
trade-reason-take-profit = Take profit
trade-reason-stop-loss = Stop loss
trade-reason-trailing-stop = Trailing stop
trade-reason-time-override = Substituição por tempo
trade-reason-strategy-exit = Saída da estratégia
trade-reason-llm-analysis-exit = Saída por análise de LLM
trade-reason-manual-exit = Saída manual
trade-reason-risk-management = Gestão de risco
trade-reason-blacklisted = Na lista negra
trade-reason-force-sell = Venda forçada
trade-reason-copy-sell = Venda de cópia
trade-reason-closed-externally = Fechada externamente
trade-reason-wallet-history = Histórico da carteira
trade-reason-exit-retry-pending = Nova tentativa de saída pendente
trade-reason-synthetic-exit-permanent-failure = Falha permanente de saída sintética
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (verificação pendente)
# $note is the operator text of a force close.
trade-reason-force-closed = Fechamento forçado: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = Nenhum token selecionado
