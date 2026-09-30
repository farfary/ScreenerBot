# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-tokens = Tokens
hints-category-positions = Posições
hints-category-filtering = Filtragem
hints-category-trader = Trader automático
hints-category-services = Serviços
hints-category-wallet = Carteira
hints-category-wallets = Carteiras
hints-category-tools = Ferramentas
hints-category-config = Configurações
hints-category-config-telegram = { -telegram }
hints-category-token-details = Detalhes do token
hints-category-ui = Interface

## tokens

hints-tokens-pool-service-title = Tokens do serviço de pools
hints-tokens-pool-service-content =
    Os tokens exibidos aqui têm:

    • **Aprovação em todos os critérios de filtragem** — liquidez, volume, idade e verificações de segurança
    • **Pools de liquidez SOL válidos** — compatíveis com nossos decodificadores de DEX (Raydium, Orca, Meteora etc.)
    • **Cálculo de preço bem-sucedido** — preços calculados diretamente das reservas on-chain do pool

    Esta é a lista de tokens mais confiável para trading, pois os preços vêm dos dados reais do pool, não de APIs externas.

    Clique em qualquer token para ver detalhes e gerenciar o status na lista negra.
hints-tokens-no-market-title = Sem dados de mercado
hints-tokens-no-market-content =
    Tokens descobertos on-chain, mas sem dados de mercado do { -dexscreener } ou do { -geckoterminal }.

    Motivos comuns:
    • **Tokens muito novos** — ainda não indexados pelos agregadores
    • **Baixo volume de negociação** — abaixo dos limites dos agregadores
    • **Pares não listados** — negociados em DEXs não monitoradas pelos agregadores

    Esses tokens ainda podem ter pools válidos e ser negociados, mas não têm métricas de mercado externas.
hints-tokens-all-title = Todos os tokens
hints-tokens-all-content =
    Banco de dados completo dos tokens descobertos, independentemente do status de filtragem.

    Inclui:
    • Tokens aprovados na filtragem
    • Tokens rejeitados
    • Tokens sem dados de mercado
    • Tokens na lista negra

    Use esta visão para pesquisar ou encontrar tokens que possam ter sido filtrados.
hints-tokens-passed-title = Aprovados na filtragem
hints-tokens-passed-content =
    Tokens que passaram por todos os critérios de filtragem ativos.

    As verificações de filtragem incluem:
    • **Liquidez** — limite mínimo de liquidez em SOL
    • **Volume** — requisitos de volume de negociação em 24h
    • **Idade do token** — tempo mínimo desde a criação
    • **Segurança** — limites da pontuação de risco do { -rugcheck }
    • **Market cap** — filtros opcionais de FDV/MC

    Configure os filtros na página **Filtragem**.
hints-tokens-rejected-title = Tokens rejeitados
hints-tokens-rejected-content =
    Tokens que falharam em um ou mais critérios de filtragem.

    Cada token mostra o motivo específico da rejeição:
    • Qual filtro falhou
    • O valor real versus o limite exigido
    • Quando a verificação ocorreu

    Revise os tokens rejeitados para ajustar suas configurações de filtro.
hints-tokens-blacklisted-title = Tokens na lista negra
hints-tokens-blacklisted-content =
    Tokens excluídos permanentemente das negociações.

    Os motivos da lista negra incluem:
    • **Lista negra manual** — tokens que você bloqueou explicitamente
    • **Riscos de segurança** — indicadores de rug pull detectados
    • **Limite de perda** — limites de perda configurados excedidos
    • **Transações com falha** — falhas repetidas de swap

    Tokens na lista negra nunca aparecem nas listas de aprovados nem são considerados no trading automático.
hints-tokens-positions-title = Tokens em posição
hints-tokens-positions-content =
    Tokens mantidos atualmente em posições abertas.

    Mostra dados em tempo real das suas posses ativas:
    • Preço atual a partir das reservas do pool
    • P&L não realizado
    • Tamanho da posição e preço de entrada
    • Tempo de posse

    Clique em qualquer token para gerenciar a posição em detalhes.
hints-tokens-recent-title = Descobertos recentemente
hints-tokens-recent-content =
    Tokens recém-descobertos, ordenados pelo momento da descoberta.

    Útil para:
    • Identificar novos lançamentos de tokens
    • Monitorar liquidez recém-criada
    • Oportunidades de entrada antecipada

    Observação: tokens novos podem não ter dados de mercado completos no início.
hints-tokens-ohlcv-title = Gerenciamento de dados OHLCV
hints-tokens-ohlcv-content =
    Veja e gerencie os dados OHLCV (candles) armazenados dos tokens.

    Mostra:
    • **Quantidade de candles** — total de pontos de dados armazenados
    • **Progresso do backfill** — status de conclusão por timeframe
    • **Período dos dados** — cobertura de tempo em horas
    • **Quantidade de pools** — pools de liquidez monitorados
    • **Status** — monitoramento ativo ou inativo

    Ações:
    • **Excluir** — remove todos os dados OHLCV de um token
    • **Limpeza** — remove em massa os dados de tokens inativos

    Os dados OHLCV são preservados permanentemente e nunca são excluídos automaticamente.

## positions

hints-positions-overview-title = Visão geral das posições
hints-positions-overview-content =
    Suas posses de tokens e posições de trading atuais.

    Principais métricas:
    • **Preço de entrada** — preço médio pago (incluindo DCA)
    • **Preço atual** — preço ao vivo a partir das reservas do pool
    • **P&L** — lucro/prejuízo não realizado em SOL e %
    • **Tamanho** — quantidade total de tokens em posse

    Clique em qualquer posição para ver as opções de gerenciamento.
hints-positions-dca-title = DCA (custo médio)
hints-positions-dca-content =
    O DCA permite aportar em posições existentes a preços diferentes.

    Quando o DCA é acionado:
    • Tokens adicionais são comprados
    • O preço de entrada é recalculado como média ponderada
    • O tamanho da posição aumenta
    • A contagem de entradas aumenta

    Configure as regras de DCA nas configurações do **Trader automático**.
hints-positions-partial-exit-title = Saída parcial
hints-positions-partial-exit-content =
    Venda uma parte da sua posição e mantenha o restante.

    Vantagens:
    • Realize parte do lucro sem sair da exposição
    • Reduza o tamanho da posição sem fechá-la por completo
    • Implemente escadas de take profit

    Cada saída parcial é registrada separadamente para um acompanhamento preciso do P&L.
hints-positions-management-title = Gerenciamento da posição
hints-positions-management-content =
    O gerenciamento define qual automação pode atuar em uma posição:

    • Trader automático: saídas de segurança, saídas por política e DCA automático
    • Somente usuário: nenhuma ação automática
    • Tarefa de cópia: saídas de segurança e vendas de cópia
    • Híbrido: saídas de segurança, saídas por política e vendas de cópia

    Você mesmo vende ou aporta nela. Compras manuais usam gerenciamento manual por padrão, para que o bot não venda um token que você comprou de propósito. Desative para devolver a posição ao trader automático.

## filtering

hints-filtering-overview-title = Filtragem de tokens
hints-filtering-overview-content =
    A filtragem determina quais tokens são elegíveis para negociação.

    Os tokens precisam passar por **todos os critérios ativados** para aparecer na lista de aprovados:
    • Métricas do { -dexscreener } (liquidez, volume etc.)
    • Métricas do { -geckoterminal } (market cap, FDV)
    • Análise de segurança do { -rugcheck }
    • Filtros meta (idade do token etc.)

    Critérios desativados são ignorados por completo.
hints-filtering-dexscreener-title = Filtros do { -dexscreener }
hints-filtering-dexscreener-content =
    Filtros baseados nos dados de mercado do { -dexscreener }:

    • **Liquidez** — liquidez mínima em USD nos pools
    • **Volume 24h** — volume mínimo de negociação
    • **Transações** — limites de atividade (compras/vendas)
    • **Variação de preço** — filtros de volatilidade

    Os dados do { -dexscreener } são atualizados a cada poucos minutos.
hints-filtering-geckoterminal-title = Filtros do { -geckoterminal }
hints-filtering-geckoterminal-content =
    Filtros baseados nos dados de mercado do { -geckoterminal }:

    • **Market cap** — capitalização de mercado mínima
    • **FDV** — limites de valuation totalmente diluído
    • **Proporção de reservas** — indicadores de saúde do pool

    O { -geckoterminal } costuma ter dados de tokens mais novos.
hints-filtering-rugcheck-title = Filtros de segurança
hints-filtering-rugcheck-content =
    Análise de segurança do { -rugcheck }.xyz:

    • **Pontuação de risco** — classificação geral de risco (0-100)
    • **Autoridade de mint** — novos tokens podem ser criados?
    • **Autoridade de congelamento** — transferências podem ser congeladas?
    • **Maiores holders** — risco de concentração

    Pontuações de risco mais altas indicam mais possíveis sinais de alerta.
hints-filtering-meta-title = Filtros meta
hints-filtering-meta-content =
    Critérios de filtragem adicionais:

    • **Idade do token** — tempo mínimo desde a criação do token
    • **Idade do pool** — tempo mínimo desde a criação do pool
    • **Tem site** — exige links de site/redes sociais
    • **Tem redes sociais** — exige { -twitter }/{ -telegram }

    Ajudam a filtrar tokens muito novos ou suspeitos.

## trader

hints-trader-overview-title = Trader automático
hints-trader-overview-content =
    Motor de trading automatizado que monitora tokens e executa trades.

    Componentes:
    • **Monitor de entrada** — observa oportunidades de compra
    • **Monitor de saída** — gerencia vendas e take profits
    • **Monitor de DCA** — cuida do preço médio das posições
    • **Controles de risco** — limites de perda e travas de segurança

    Inicie/pare o trading no painel de controle.
hints-trader-entry-title = Monitor de entrada
hints-trader-entry-content =
    Observa tokens filtrados em busca de sinais de entrada.

    A avaliação de entrada verifica:
    • O token passa pela filtragem atual
    • Ainda não está em uma posição
    • Não está na lista negra
    • Limites de posições não excedidos
    • Condições da estratégia atendidas (se configurada)

    Configure o tamanho e os limites de entrada em Configurações.
hints-trader-exit-title = Monitor de saída
hints-trader-exit-content =
    Monitora posições abertas em busca de sinais de saída.

    Gatilhos de saída:
    • **Take profit** — meta de preço atingida
    • **Stop loss** — perda máxima excedida
    • **Trailing stop** — preço recuou desde o pico
    • **Saída da estratégia** — condições personalizadas atendidas
    • **Por tempo** — duração máxima de posse

    Configure os limites em Configurações.

## services

hints-services-overview-title = Serviços do sistema
hints-services-overview-content =
    Serviços em segundo plano que fazem o { -brand } funcionar.

    Estados dos serviços:
    • **Em execução** (verde) — funcionando normalmente
    • **Iniciando** (amarelo) — inicializando
    • **Parado** (vermelho) — não está em execução
    • **Erro** (alerta) — falhou, pode reiniciar automaticamente

    Os serviços têm dependências e iniciam em ordem.
hints-services-health-title = Saúde dos serviços
hints-services-health-content =
    Os indicadores de saúde mostram o status do serviço:

    • **Tempo ativo** — tempo desde o último início
    • **Tarefas** — operações ativas em segundo plano
    • **Erros** — contagem de erros recentes
    • **Métricas** — dados de desempenho (se disponíveis)

    Serviços críticos afetam a capacidade de trading.

## wallet

hints-wallet-overview-title = Visão geral da carteira
hints-wallet-overview-content =
    Status da sua carteira Solana conectada.

    Exibe:
    • **Saldo em SOL** — SOL nativo para taxas e trading
    • **Posses de tokens** — tokens SPL com seus valores
    • **Variação 24h** — variação do valor do portfólio
    • **Histórico** — snapshots do saldo ao longo do tempo

    Os saldos são atualizados a cada minuto.
hints-wallet-tokens-title = Saldos de tokens
hints-wallet-tokens-content =
    Tokens SPL mantidos na sua carteira.

    Mostra:
    • Símbolo e nome do token
    • Quantidade em posse
    • Valor atual em SOL/USD
    • Preço do pool ou dos dados de mercado

    Contas de token vazias podem ser limpas em Configurações.

## wallets

hints-wallets-main-title = Carteira principal
hints-wallets-main-content =
    A carteira principal usada em todas as operações de trading.

    • **Trading automático** — os trades de entrada/saída são executados a partir desta carteira
    • **Exibição do saldo** — mostrado no cabeçalho e no painel
    • **Posses de tokens** — tokens SPL mantidos por esta carteira

    Para trocar a carteira principal, selecione "Definir como principal" em qualquer carteira secundária.
hints-wallets-secondary-title = Carteiras secundárias
hints-wallets-secondary-content =
    Carteiras adicionais para operações com várias carteiras.

    • **Trading com várias carteiras** — coordene compras/vendas entre carteiras
    • **Separação de portfólio** — organize por estratégia ou finalidade
    • **Saldos independentes** — cada carteira tem seu próprio SOL/tokens

    Carteiras secundárias não são usadas pelo trading automático, a menos que sejam configuradas explicitamente.

## tools

hints-tools-wallet-cleanup-title = Ferramenta de limpeza de carteira
hints-tools-wallet-cleanup-content =
    { "*" }*Recupere SOL de contas de token vazias**

    { "*" }*O que são ATAs?**
    Associated Token Accounts (ATAs) são contas Solana que guardam seus tokens. Cada token com o qual você interage cria uma ATA que exige ~0.002 SOL de rent.

    { "*" }*Por que limpar ATAs vazias?**
    • Recupere o rent (~0.002 SOL por ATA)
    • Traders ativos podem acumular centenas de ATAs vazias
    • 100 ATAs vazias = ~0.2 SOL recuperáveis

    { "*" }*Como funciona:**
    • Escaneia sua carteira em busca de ATAs com saldo zero
    • Mostra o total de SOL recuperável
    • Fecha as contas vazias para recuperar o rent

    { "*" }*Limpeza automática:**
    Quando ativada, escaneia e fecha ATAs vazias automaticamente a cada 5 minutos em segundo plano.

    { "*" }*Importante:**
    • Só fecha contas com saldo exatamente 0
    • Fechamentos com falha ficam em cache para evitar novas tentativas repetidas
    • Carteiras grandes podem exigir várias passagens de limpeza
hints-tools-burn-tokens-title = Ferramenta de queima de tokens
hints-tools-burn-tokens-content =
    { "*" }*Destrua tokens permanentemente**

    Queimar tokens os remove permanentemente da sua carteira e da circulação.

    { "*" }*O que acontece ao queimar:**
    • Os tokens são enviados a um endereço de queima (irrecuperável)
    • O saldo do token passa a ser zero
    • A ATA pode então ser fechada pela Limpeza de carteira para recuperar ~0.002 SOL de rent

    { "*" }*Categorias de tokens:**
    • **Posições abertas** - Não podem ser queimados (trades ativos)
    • **Posições fechadas** - Resquícios de trades anteriores
    • **Com valor** - Tokens com liquidez (considere vender em vez de queimar)
    • **Liquidez zero** - Tokens sem valor/poeira (seguros para queimar)

    { "*" }*Aviso:** Esta ação é **irreversível**. Tokens queimados não podem ser recuperados em nenhuma circunstância.

    { "*" }*Depois de queimar:** Execute a Limpeza de carteira para fechar ATAs vazias e recuperar o rent em SOL.
hints-tools-wallet-generator-title = Ferramenta de gerador de carteiras
hints-tools-wallet-generator-content =
    { "*" }*Gere novos pares de chaves Solana**

    Crie novas carteiras com segurança no seu dispositivo.

    { "*" }*Recursos:**
    • Gera pares de chaves criptograficamente seguros
    • Prefixo opcional de endereço personalizado (ex.: "SOL...")
    • Exportação em base58 ou array JSON

    { "*" }*Segurança:**
    • As chaves são geradas localmente
    • Nunca são transmitidas pela rede
    • Sempre faça backup das chaves com segurança
hints-tools-multi-buy-title = Ferramenta de compra múltipla
hints-tools-multi-buy-content =
    { "*" }*Coordene compras em várias carteiras**

    Execute ordens de compra em várias subcarteiras com valores aleatórios para simular uma atividade de compra orgânica.

    { "*" }*Como funciona:**
    1. Cria ou usa subcarteiras existentes
    2. Distribui SOL da carteira principal para as subcarteiras
    3. Executa ordens de compra com valores e atrasos aleatórios
    4. Cada carteira compra de forma independente, com assinaturas únicas

    { "*" }*Configurações das carteiras:**
    • **Quantidade de carteiras** — número de subcarteiras a usar (2-10)
    • **Reserva de SOL** — SOL reservado por carteira para taxas (~0.015)

    { "*" }*Configurações de valores:**
    • **Mín./máx. de SOL** — faixa de valores de compra por carteira
    • **Limite total** — teto opcional do total de SOL a gastar

    { "*" }*Configurações de execução:**
    • **Atraso** — atraso aleatório entre transações
    • **Concorrência** — execução em paralelo (1 = sequencial)
    • **Slippage** — slippage máximo aceitável
    • **Roteador** — roteamento do swap (Auto, { -jupiter }, Raydium)

    { "*" }*Importante:**
    • Exige SOL suficiente na carteira principal
    • Compras com falha são registradas, mas não interrompem a sessão
    • As subcarteiras podem ser reutilizadas entre sessões
hints-tools-multi-sell-title = Ferramenta de venda múltipla
hints-tools-multi-sell-content =
    { "*" }*Coordene vendas em várias carteiras**

    Venda tokens de todas as subcarteiras que possuem um token específico, com consolidação automática de SOL.

    { "*" }*Como funciona:**
    1. Escaneia as subcarteiras em busca de saldos do token
    2. Opcionalmente recarrega carteiras com pouco SOL para taxas
    3. Executa ordens de venda com porcentagem configurável
    4. Consolida os ganhos de volta na carteira principal

    { "*" }*Configurações de venda:**
    • **% de venda** — porcentagem de tokens a vender (padrão 100%)
    • **Mín. de SOL para taxa** — SOL mínimo necessário para a transação
    • **Recarga automática** — transfere SOL da carteira principal, se necessário

    { "*" }*Ações após a venda:**
    • **Consolidar SOL** — transfere todo o SOL de volta para a carteira principal
    • **Fechar ATAs** — fecha contas de token para recuperar o rent (~0.002 SOL cada)

    { "*" }*Configurações de execução:**
    • **Atraso** — atraso aleatório entre transações
    • **Concorrência** — execução em paralelo
    • **Slippage** — slippage máximo aceitável
    • **Roteador** — preferência de roteamento do swap

    { "*" }*Dicas:**
    • A prévia mostra todas as carteiras que possuem o token
    • Desmarque as carteiras das quais você não quer vender
    • A consolidação acontece depois que todas as vendas terminam
hints-tools-trade-watcher-title = Ferramenta Monitor de Trades
hints-tools-trade-watcher-content =
    { "*" }*Monitore trades e dispare ações automáticas**

    Acompanhe a atividade de negociação de um token e reaja automaticamente quando ocorrerem trades.

    { "*" }*Tipos de monitoramento:**
    • **Comprar na venda** — compra automaticamente quando alguém vende (aproveite quedas)
    • **Vender na compra** — vende automaticamente quando alguém compra (siga o mercado)
    • **Somente notificar** — receba alertas sem tomar nenhuma ação

    { "*" }*Como funciona:**
    1. Informe o endereço do mint de um token
    2. Clique em "Buscar pools" para encontrar os pools de liquidez disponíveis
    3. Selecione um pool para monitorar (obrigatório para ações de compra/venda)
    4. Defina o valor de gatilho (tamanho mínimo do trade para reagir)
    5. Defina o valor da ação (quanto SOL comprar/vender)
    6. Inicie o monitoramento

    { "*" }*Requisitos:**
    • Endereço de mint do token válido
    • Seleção de pool (para ações de compra/venda)
    • Saldo em SOL suficiente para os valores das ações

    { "*" }*Integração com o { -telegram }:**
    Configure o { -telegram } em Configurações → { -telegram } para receber notificações instantâneas quando os monitoramentos dispararem.
hints-tools-wallet-consolidation-title = Ferramenta de consolidação de carteiras
hints-tools-wallet-consolidation-content =
    { "*" }*Gerencie e consolide os fundos das subcarteiras**

    Veja todas as subcarteiras e consolide SOL, tokens e o rent das ATAs de volta na sua carteira principal.

    { "*" }*O resumo mostra:**
    • **Subcarteiras** — quantidade total de subcarteiras criadas
    • **Total de SOL** — saldo combinado em SOL de todas as subcarteiras
    • **Tipos de token** — número de tokens diferentes em posse
    • **Rent recuperável** — SOL preso em ATAs vazias

    { "*" }*Ações:**
    • **Transferir SOL** — move todo o SOL das carteiras selecionadas para a principal
    • **Transferir tokens** — move todos os tokens para a carteira principal
    • **Limpar ATAs** — fecha contas de token vazias para reaver o rent

    { "*" }*Informações da tabela:**
    • Caixa de seleção para escolher carteiras em operações em lote
    • Nome, endereço, saldo em SOL, quantidade de tokens, ATAs vazias
    • Carteiras vazias aparecem esmaecidas para fácil identificação

    { "*" }*Dicas:**
    • Use depois da Venda múltipla para coletar o SOL restante
    • Limpe as ATAs regularmente para recuperar o rent
    • Carteiras vazias podem ser reutilizadas em operações futuras

## config

hints-config-overview-title = Configurações
hints-config-overview-content =
    Configurações de todo o sistema do { -brand }.

    Categorias:
    • **Trader** — regras de entrada/saída, dimensionamento de posição
    • **Filtragem** — limites dos filtros de tokens
    • **Swaps** — configurações de roteamento e slippage
    • **RPC** — configuração de nós
    • **Serviços** — configurações dos serviços em segundo plano

    As alterações têm efeito imediato (hot reload).
hints-config-telegram-title = Notificações do { -telegram }
hints-config-telegram-content =
    { "*" }*Receba alertas de trading instantâneos pelo { -telegram }**

    Seja notificado sobre trades, posições e eventos importantes diretamente no { -telegram }.

    { "*" }*Etapas de configuração:**

    1. **Crie um bot:**
       • Abra o { -telegram } e envie uma mensagem para @BotFather
       • Envie /newbot e siga as instruções
       • Copie o token do bot (algo como: 123456:ABC-DEF...)

    2. **Obtenha seu Chat ID:**
       • Envie uma mensagem para @userinfobot ou @getidsbot
       • Copie o ID numérico retornado

    3. **Configure no { -brand }:**
       • Ative a chave de notificações
       • Cole o token do bot e o chat ID
       • Clique em "Testar conexão" para verificar

    { "*" }*O que você vai receber:**
    • Confirmações de execução de trades
    • Atualizações de posições (entrada/saída)
    • Alertas do Monitor de Trades
    • Notificações de erro

    { "*" }*Privacidade:**
    As mensagens são enviadas diretamente do { -brand } para o seu bot do { -telegram } — sem servidores de terceiros.
hints-config-telegram-password-title = Senha de autenticação do bot
hints-config-telegram-password-content =
    { "*" }*Proteja seu bot do { -telegram } com autenticação por senha**

    Ao interagir com o seu bot do { -brand } no { -telegram }, você precisará se autenticar com esta senha antes de executar comandos sensíveis.

    { "*" }*Por que definir uma senha?**
    • Impede que usuários não autorizados controlem seu bot
    • Necessária para executar comandos de trading pelo { -telegram }
    • Deve ter pelo menos 8 caracteres

    { "*" }*Como funciona:**
    1. Defina uma senha aqui no painel
    2. Quando você enviar um comando de trading ao bot, ele pedirá autenticação
    3. Informe sua senha para confirmar sua identidade
    4. Opcionalmente, ative o 2FA para mais segurança

    { "*" }*Observação:** A senha é armazenada como um hash SHA256 seguro — nunca guardamos o texto simples.
hints-config-telegram-totp-title = Autenticação em dois fatores (2FA)
hints-config-telegram-totp-content =
    { "*" }*Adicione uma camada extra de segurança com 2FA TOTP**

    A autenticação em dois fatores usa senhas de uso único baseadas em tempo (TOTP) de apps como Google Authenticator, Authy ou 1Password.

    { "*" }*Por que ativar o 2FA?**
    • Mesmo que alguém saiba sua senha, não consegue acessar seu bot sem o código
    • Os códigos de 6 dígitos mudam a cada 30 segundos
    • Funciona offline depois de configurado

    { "*" }*Processo de configuração:**
    1. Clique em "Ativar 2FA" e informe sua senha
    2. Escaneie o QR code com seu app autenticador
    3. Informe o código de 6 dígitos para confirmar a configuração

    { "*" }*Apps compatíveis:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • Qualquer app compatível com TOTP

    { "*" }*Importante:** Guarde sua chave secreta em um lugar seguro. Se você perder o acesso ao app autenticador, precisará desativar o 2FA por este painel.

## token_details

hints-token-details-chart-title = Gráfico de preço (OHLCV)
hints-token-details-chart-content =
    { "*" }*Importante:** Este gráfico exibe **dados OHLCV em cache** para a avaliação de estratégias, *não* o preço de execução ao vivo.

    { "*" }*Por que dados em cache?**
    • **Finalidade:** Usados por estratégias e indicadores automatizados (ex.: RSI, MA).
    • **Atualização:** Depende da prioridade do token (posições abertas = atualizações mais rápidas).
    • **Fonte:** Agregado do { -dexscreener }/{ -geckoterminal }, não direto do RPC on-chain.

    { "*" }*A realidade do preço em DEXs:**
    No DeFi, os tokens são negociados em **vários pools** (Raydium, Orca, Meteora). Cada pool tem um preço próprio, conforme a profundidade de liquidez e os trades recentes.
    • **Preço do gráfico:** Uma média/agregado entre mercados.
    • **Preço do swap:** A taxa específica obtida na melhor rota no exato momento do trade.

    { "*" }Espere pequenas diferenças entre este gráfico e o seu preço final de execução.*

    { "*" }*Status:** "Aguardando dados" significa que os workers em segundo plano estão buscando candles novos.
hints-token-details-token-info-title = Informações do token
hints-token-details-token-info-content =
    Metadados básicos do token de fontes on-chain e de mercado.

        • **Mint** — endereço único do token na Solana (clique para copiar)
        • **Decimais** — precisão do token (normalmente 6-9)
        • **Idade** — tempo desde a criação do pool/token principal
        • **DEX** — principal local de negociação deste token
        • **Holders** — carteiras únicas que possuem o token
        • **Top 10** — % em poder das 10 maiores carteiras

        Mais holders e menor concentração geralmente indicam uma distribuição mais saudável.
hints-token-details-liquidity-title = Liquidez e dados de mercado
hints-token-details-liquidity-content =
    Métricas de mercado do pool SOL com maior liquidez.

        • **FDV** — preço × supply total (preço do agregador)
        • **Liquidez** — valor em USD das reservas do pool
        • **Pool SOL** / **Pool Token** — reservas ao vivo que definem o preço do pool

        { "*" }*Por que importa:**
        • Mais liquidez = menos slippage
        • Pools rasos podem se mover com trades pequenos
        • As reservas do pool definem diretamente o preço de execução do swap

        Os dados são atualizados periodicamente a partir do { -dexscreener }/{ -geckoterminal }, além de leituras on-chain dos pools.
hints-token-details-market-pulse-title = Pulso do mercado
hints-token-details-market-pulse-content =
    A variação de preço e o volume de negociação em USD compartilham a mesma linha do tempo de **5M / 1H / 6H / 24H**, para que o momentum e a participação possam ser comparados diretamente.

    { "*" }*Interpretação:**
    • **Preço** — variação percentual derivada do agregador, não o preço de execução ao vivo do pool.
    • **Volume alto** — mais interesse, descoberta de preço mais eficiente e saídas mais fáceis.
    • **Volume baixo** — mais slippage, spreads maiores e saídas grandes mais difíceis.
    • **Volume alto + liquidez baixa** — volatilidade elevada e risco de execução.

    Os dados de mercado são agregados das principais DEXs via { -dexscreener }/{ -geckoterminal }, então a variação de preço pode diferir do preço atual do pool on-chain.
hints-token-details-activity-title = Atividade de transações (contagens)
hints-token-details-activity-content =
    Analisa o **número de trades** (compras vs. vendas) em vários timeframes. Isso revela a intenção dos traders, independentemente do tamanho do trade.

    { "*" }*Detalhamento das métricas:**
    • **Timeframes:** janelas de 5M, 1H, 6H e 24H.
    • **Barras:** Proporção visual entre compras (verde) e vendas (vermelho).
    • **Taxa:** Trades por minuto (ex.: "12.5/m"). Taxas mais altas = atividade viral.
    • **Contagens:** Número exato de compras/vendas e sua participação percentual.

    { "*" }*Métricas resumidas:**
    • **% de compras 24H:** >50% é altista (mais compradores), { "<" }50% é baixista (mais vendedores).
    • **Fluxo líquido:** Total de compras menos vendas. Positivo = acumulação.
    • **Pico 5M:** Quão mais rápida está a negociação *agora* em relação à média de 1H.
      • **>1.0x:** Interesse acelerando.
      • **>3.0x:** Rompimento viral ou evento de pânico.
      • **{ "<" }1.0x:** Esfriando.

    { "*" }*Dica de estratégia:** Uma "% de compras" alta com um "fator de pico" alto costuma sinalizar uma boa entrada em rompimento.
hints-token-details-security-title = Análise de segurança
hints-token-details-security-content =
    Avaliação de risco do { -rugcheck }.xyz e análise on-chain.

    { "*" }*Pontuação de segurança (0-100):**
    Pontuações mais altas indicam tokens mais seguros. Os fatores incluem:
    • Permissões de autoridade (mint/congelamento)
    • Concentração de holders
    • Status de bloqueio do LP
    • Padrões de risco conhecidos

    { "*" }*Principais indicadores de risco:**
    • **Autoridade de mint** — pode criar novos tokens (risco de inflação)
    • **Autoridade de congelamento** — pode congelar contas de token
    • **% dos maiores holders** — risco de concentração
    • **Provedores de LP** — número de provedores de liquidez

    Sempre verifique a segurança antes de negociar valores significativos.
hints-token-details-pools-title = Pools de liquidez
hints-token-details-pools-content =
    Todos os pools de liquidez descobertos para este token.

    { "*" }*Por que vários pools importam:**
    • Cada pool tem liquidez e preços diferentes
    • Roteadores de swap encontram a melhor rota entre os pools
    • O preço pode variar de 1% a 5% entre pools

    { "*" }*Informações do pool:**
    • **DEX** — qual exchange hospeda o pool
    • **Liquidez** — valor em USD das reservas do pool
    • **Volume** — atividade de negociação recente
    • **Preço** — preço atual do pool

    O serviço de pools calcula os preços a partir do par SOL de maior liquidez.

## ui

hints-ui-featured-title = Em destaque
hints-ui-featured-content =
    Primeiro os tokens impulsionados, depois os projetos em alta da { -jupiter } e do { -dexscreener }.

    { "*" }*O que você verá:**
    • Tokens impulsionados — suas equipes pagaram para promovê-los — fixados no início, marcados em dourado
    • Depois, tokens em alta dos quadros de descoberta
    • Clique em qualquer token para abrir seus detalhes completos

    { "*" }*Impulsionar um token:**
    Um boost compra visibilidade, nunca uma recomendação. As linhas impulsionadas são marcadas em dourado
    em todo lugar em que aparecem, inclusive na sua tabela de tokens, para que você sempre saiba quais são. Impulsione um token em
    { "*" }*screenerbot.io/boost**.

    { "*" }*Desativar a linha:**
    Oculte-a em **Configurações → Interface → Mostrar linha em destaque**. A ação do cabeçalho continua abrindo a
    visão completa de Em destaque.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = Ajuda: { $title }
hints-popover-close =
    .aria-label = Fechar
hints-popover-learn-more = Saiba mais
hints-popover-dismiss = Não mostrar novamente
