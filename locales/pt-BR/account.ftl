# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = Os dados do { -brand } estão ativos
    .detail = Candles compartilhados, registro de pools, relatórios de segurança e identidade dos tokens estão sendo servidos pelo screenerbot.io.

account-data-access-disabled = Os dados do { -brand } estão desativados
    .detail = A fonte do { -brand } está desativada nas suas configurações, então os dados vêm apenas dos provedores públicos.

account-data-access-offline = Os dados do { -brand } estão offline
    .detail = Não há conexão de rede. Os dados voltarão automaticamente quando a conexão for restabelecida.

account-data-access-signed-out = Os dados do { -brand } exigem uma conta
    .detail = Gráficos, pools, relatórios de segurança e identidade dos tokens vêm dos provedores públicos. Eles são mais lentos, têm limite de requisições e menos histórico. Entrar é grátis e não muda mais nada no funcionamento do { -brand }.

account-data-access-reauthorization-required = Os dados do { -brand } exigem que você entre novamente
    .detail = Este dispositivo foi autorizado antes de os dados do { -brand } existirem. Entre novamente para restaurá-los — até lá, os provedores públicos estão sendo usados.

account-data-access-version-unsupported = Os dados do { -brand } exigem uma versão mais recente
    .detail = Esta versão não é mais atendida. Atualize para { $minimum } ou superior para voltar a usar os dados do { -brand }; até lá, os provedores públicos estão sendo usados.

account-data-access-unreachable = Os dados do { -brand } não estão respondendo
    .detail = O serviço não respondeu. Os provedores públicos estão sendo usados e o { -brand } continuará tentando.

account-data-access-unknown = Os dados do { -brand } ainda não foram verificados
    .detail = O { -brand } ainda não precisou de dados compartilhados nesta sessão.

## Account panel (ui/account/panel.js), shared by Setup and Settings

account-scope-data-read = Dados de mercado do { -brand }
account-scope-rpc-submit = Envio gratuito de transações assinadas
account-scope-vote = Votação em tokens
account-scope-referral-read = Ganhos de indicações
account-scope-account-read = Detalhes da conta

account-panel-request-failed = Não deu certo. Tente novamente.
account-panel-checking = Verificando o status da conta…
account-panel-status-unavailable = O status da conta está indisponível.
account-panel-browser-notice = Conclua o login no navegador e volte aqui. Este painel será atualizado.
account-panel-browser-timeout = O login pelo navegador não foi concluído. Você pode iniciá-lo novamente.
account-panel-unavailable = Os recursos da conta estão indisponíveis no momento. Continue a configuração sem entrar.
account-panel-retry-status = Tentar novamente o status da conta
account-panel-signed-in-fallback = Conectado
account-panel-features =
    .aria-label = Recursos da conta
account-panel-sign-out = Sair
account-panel-signing-out = Saindo…
account-panel-sign-in = Entrar
account-panel-signing-in = Entrando…
account-panel-sign-in-wallet = Entrar com a carteira
account-panel-opening-browser = Abrindo o navegador…
account-panel-continue-browser = Continuar no navegador
account-panel-sign-in-email = Entrar com e-mail
account-panel-new-to = Novo no { -brand }?
account-panel-create-account = Criar uma conta
account-panel-unlocks-title = Incluído na conta
account-panel-back-to-options = Voltar às opções de login
account-panel-email-label = E-mail
account-panel-email-input =
    .placeholder = voce@exemplo.com
account-panel-password-label = Senha
account-panel-password-input =
    .placeholder = Sua senha
account-panel-need-account = Precisa de uma conta ou esqueceu sua senha?
account-panel-open-website = Abrir screenerbot.io
