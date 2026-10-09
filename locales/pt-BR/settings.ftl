settings-duration-minutes =
    { $count ->
        [one] { $count } minuto
        [many] { $count } minutos
       *[other] { $count } minutos
    }
settings-duration-hours =
    { $count ->
        [one] { $count } hora
        [many] { $count } horas
       *[other] { $count } horas
    }

settings-dialog-title = Configurações
settings-dialog-close =
    .title = Fechar (ESC)
    .aria-label = Fechar configurações
settings-dialog-save = Salvar alterações
settings-dialog-saving = Salvando...
settings-dialog-saved = Salvo
settings-dialog-save-success = Configurações salvas com sucesso
settings-dialog-save-failed = Falha ao salvar as configurações
settings-dialog-update-attention = A atualização precisa de atenção
settings-dialog-tab-interface = Interface
settings-dialog-tab-navigation = Navegação
settings-dialog-tab-startup = Inicialização
settings-dialog-tab-hints = Dicas
settings-dialog-tab-data = Dados
settings-dialog-tab-security = Segurança
settings-dialog-tab-account = Conta
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = Conexões de agentes
settings-dialog-tab-updates = Atualizações
settings-dialog-tab-licenses = Licenças
settings-dialog-tab-about = Sobre
settings-dialog-link-privacy = Política de Privacidade
settings-dialog-link-terms = Termos de Serviço

settings-startup-section-title = Comportamento na inicialização
settings-startup-auto-start-label = Iniciar o Trader automaticamente
settings-startup-auto-start-hint = Inicia o Trader automaticamente ao abrir o app
settings-startup-coming-soon = Em breve
settings-startup-default-page-label = Página inicial
settings-startup-default-page-hint = Página exibida ao abrir o app
settings-startup-page-dashboard = Painel
settings-startup-page-tokens = Tokens
settings-startup-page-positions = Posições
settings-startup-page-wallet = Carteira
settings-startup-page-config = Config
settings-startup-notifications-label = Mostrar notificações em segundo plano
settings-startup-notifications-hint = Exibe notificações de eventos em segundo plano

settings-about-tagline = Motor de trading nativo para Solana
settings-about-link-github = { -github }
settings-about-link-docs = Documentação
settings-about-link-telegram = { -telegram }
settings-about-link-website = Site
settings-about-credits = Feito para traders de Solana
settings-about-copyright = © { $year } { -brand }. Todos os direitos reservados.

settings-interface-section-appearance = Aparência
settings-interface-theme-label = Tema
settings-interface-theme-hint = Escolha seu esquema de cores preferido
settings-interface-theme-dark = Escuro
settings-interface-theme-light = Claro
settings-interface-language-label = Idioma
settings-interface-language-hint = Idioma de exibição do painel
settings-interface-logo-shape-label = Formato do logo do token
settings-interface-logo-shape-hint = Círculo recorta todos os logos; Natural preserva a silhueta original de cada arte
settings-interface-logo-shape-circle = Círculo
settings-interface-logo-shape-natural = Natural
settings-interface-animations-label = Ativar animações
settings-interface-animations-hint = Transições e efeitos suaves
settings-interface-compact-label = Modo compacto
settings-interface-compact-hint = Reduz o espaçamento para exibir mais conteúdo
settings-interface-section-data = Dados e exibição
settings-interface-refresh-label = Intervalo de atualização
settings-interface-refresh-hint = Com que frequência os dados são atualizados
settings-interface-refresh-seconds =
    { $count ->
        [one] { $count } segundo
        [many] { $count } segundos
       *[other] { $count } segundos
    }
settings-interface-refresh-minutes =
    { $count ->
        [one] { $count } minuto
        [many] { $count } minutos
       *[other] { $count } minutos
    }
settings-interface-ticker-label = Mostrar barra de ticker
settings-interface-ticker-hint = Ticker de métricas ao vivo no cabeçalho
settings-interface-page-size-label = Tamanho da página da tabela
settings-interface-page-size-hint = Linhas por página de tabela por padrão
settings-interface-page-size-rows =
    { $count ->
        [one] { $count } linha
        [many] { $count } linhas
       *[other] { $count } linhas
    }
settings-interface-auto-expand-label = Expandir categorias automaticamente
settings-interface-auto-expand-hint = Expande as categorias de configuração por padrão
settings-interface-hints-label = Mostrar dicas contextuais
settings-interface-hints-hint = Exibe ícones de ajuda que explicam os recursos do painel
settings-interface-featured-label = Mostrar linha em destaque
settings-interface-featured-hint = Exibe a linha de tokens em destaque nas páginas Início e Tokens
settings-interface-section-sound = Efeitos sonoros
settings-interface-sounds-label = Ativar sons
settings-interface-sounds-hint = Sinais táteis para navegação, mudanças de estado e resultados

settings-security-loading = Carregando configurações de segurança...
settings-security-load-failed = Falha ao carregar as configurações de segurança

settings-security-type-pin4 = PIN de 4 dígitos
settings-security-type-pin6 = PIN de 6 dígitos
settings-security-type-text = Senha de texto
settings-security-type-unset = Não definida

settings-security-lockscreen-title = Tela de bloqueio do painel
settings-security-lockscreen-description = Proteja seu painel com um PIN ou senha. A tela de bloqueio aparece quando acionada e exige autenticação para continuar.
settings-security-enable-label = Ativar tela de bloqueio
settings-security-enable-hint = Proteja seu painel com autenticação por senha
settings-security-password-status-label = Status da senha
settings-security-password-current = Atual: { $type }
settings-security-password-none = Nenhuma senha definida
settings-security-change = Alterar
settings-security-remove = Remover
settings-security-set-password = Definir senha
settings-security-auto-lock-label = Bloquear após inatividade
settings-security-auto-lock-hint = Bloqueia automaticamente após um período sem atividade
settings-security-auto-lock-never = Nunca
settings-security-lock-blur-label = Bloquear ao perder o foco da janela
settings-security-lock-blur-hint = Bloqueia automaticamente ao trocar para outro aplicativo
settings-security-quick-actions-title = Ações rápidas
settings-security-lock-now-label = Bloquear painel agora
settings-security-lock-now-hint = Bloqueia o painel imediatamente
settings-security-needs-password = Defina uma senha primeiro para usar isto
settings-security-needs-lockscreen = Ative a tela de bloqueio primeiro para usar isto
settings-security-lock-now = Bloquear agora
settings-security-lock-not-ready = Não foi possível bloquear: a tela de bloqueio não está pronta
settings-security-setting-save-failed = Não foi possível salvar a configuração de segurança

settings-security-2fa-title = Autenticação em duas etapas
settings-security-2fa-description = Adicione uma camada extra de segurança usando um app autenticador (Google Authenticator, Authy etc.)
settings-security-2fa-status-label = Status do 2FA
settings-security-2fa-status-enabled = A autenticação em duas etapas está ativada
settings-security-2fa-status-none = Não configurado
settings-security-2fa-disable = Desativar 2FA
settings-security-2fa-enable = Ativar 2FA

settings-security-modal-close =
    .aria-label = Fechar
settings-security-password-set-title = Definir senha
settings-security-password-change-title = Alterar senha
settings-security-password-current-label = Senha atual
settings-security-password-current-input =
    .placeholder = Digite a senha atual
settings-security-password-type-label = Tipo de senha
settings-security-password-new-label = Nova senha
settings-security-password-new-input =
    .placeholder = Digite a nova senha
settings-security-password-confirm-label = Confirmar senha
settings-security-password-confirm-input =
    .placeholder = Confirme a senha
settings-security-password-update = Atualizar senha
settings-security-placeholder-pin4 = Digite o PIN de 4 dígitos
settings-security-placeholder-pin6 = Digite o PIN de 6 dígitos
settings-security-placeholder-text = Digite a senha
settings-security-password-required = Digite uma senha
settings-security-password-mismatch = As senhas não coincidem
settings-security-pin4-invalid = O PIN deve ter exatamente 4 dígitos
settings-security-pin6-invalid = O PIN deve ter exatamente 6 dígitos
settings-security-text-too-short = A senha deve ter pelo menos 4 caracteres
settings-security-password-saved = Senha salva
settings-security-password-save-failed = Falha ao salvar a senha
settings-security-password-save-failed-detail = Falha ao salvar a senha: { $message }

settings-security-remove-title = Remover senha
settings-security-remove-description = Digite sua senha atual para remover a proteção da tela de bloqueio.
settings-security-remove-confirm = Remover senha
settings-security-current-required = Digite sua senha atual
settings-security-password-removed = Senha removida
settings-security-password-remove-failed = Falha ao remover a senha
settings-security-password-remove-failed-detail = Falha ao remover a senha: { $message }

settings-security-2fa-enable-title = Ativar autenticação em duas etapas
settings-security-2fa-password-prompt = Digite sua senha para continuar:
settings-security-2fa-password-input =
    .placeholder = Digite a senha
settings-security-2fa-continue = Continuar
settings-security-2fa-manual-code = Código para inserção manual:
settings-security-2fa-qr =
    .alt = QR code do TOTP
settings-security-2fa-code-prompt = Digite o código de 6 dígitos do seu app autenticador:
settings-security-2fa-verify-enable = Verificar e ativar
settings-security-2fa-password-required = Digite sua senha
settings-security-2fa-setup-failed = Falha ao configurar o 2FA
settings-security-2fa-code-invalid-length = Digite um código de 6 dígitos
settings-security-2fa-code-invalid = Código inválido
settings-security-2fa-enabled = Autenticação em duas etapas ativada
settings-security-2fa-verify-failed = Falha ao verificar o código
settings-security-2fa-disable-title = Desativar autenticação em duas etapas
settings-security-2fa-disable-prompt = Digite sua senha para desativar o 2FA:
settings-security-2fa-disable-failed = Falha ao desativar o 2FA
settings-security-2fa-disabled = Autenticação em duas etapas desativada

settings-agent-category-analysis = Análise
settings-agent-category-portfolio = Portfólio
settings-agent-category-trading = Trading
settings-agent-category-config = Configuração
settings-agent-category-system = Sistema
settings-agent-category-analysis-description = Análise de tokens, dados de mercado e verificações de segurança.
settings-agent-category-portfolio-description = Posições abertas, saldos e P&L.
settings-agent-category-trading-description = Compra, venda e fechamento de posições com fundos reais.
settings-agent-category-config-description = Todas as configurações do bot, incluindo endpoints de RPC. Nunca as chaves da carteira.
settings-agent-category-system-description = Status, eventos e a parada de emergência.
settings-agent-category-analysis-inline = análise
settings-agent-category-portfolio-inline = portfólio
settings-agent-category-trading-inline = trading
settings-agent-category-config-inline = configuração
settings-agent-category-system-inline = sistema

settings-agent-level-allow = Permitir
settings-agent-level-ask-user = Perguntar
settings-agent-level-deny = Desativado
settings-agent-level-allow-hint = Executa imediatamente.
settings-agent-level-ask-user-hint = Aguarda sua aprovação no app.
settings-agent-level-deny-hint = Recusado e oculto para o agente.

settings-agent-preset-full = Acesso total
settings-agent-preset-ask = Perguntar antes
settings-agent-preset-read = Somente leitura
settings-agent-preset-full-description = Tudo é executado sem perguntar. As chaves da carteira continuam inacessíveis.
settings-agent-preset-ask-description = Toda ação aguarda sua aprovação no app.
settings-agent-preset-read-description = Leitura de análises e do portfólio. Nada pode ser alterado.
settings-agent-preset-custom = Personalizado
settings-agent-preset-group =
    .aria-label = Predefinição de permissões
settings-agent-permission-group = Permissão de { $category }

settings-agent-summary-asks-only = Limitada — pergunta antes de { $asking }
settings-agent-summary-off-only = Limitada — sem { $off }
settings-agent-summary-asks-and-off = Limitada — pergunta antes de { $asking }; sem { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = MCP stdio genérico

settings-agent-note-placeholder = Substitua /absolute/path/to/screenerbot pelo caminho absoluto do binário do { -brand }. O app em execução não conseguiu representar o caminho do próprio executável neste sistema.
settings-agent-note-data-dir = Se você executa o { -brand } com um diretório de dados diferente do padrão, defina também SCREENERBOT_DATA_DIR no cliente (outra flag -e / --env ou uma entrada env) com o mesmo caminho.
settings-agent-note-codex-run = Execute o comando ou adicione o bloco TOML a ~/.codex/config.toml ($CODEX_HOME/config.toml). Depois, reinicie o { -codex }.
settings-agent-note-codex-get = `codex mcp get screenerbot` oculta o segredo na saída.
settings-agent-note-claude-code = { -claude } Code: execute o comando e reinicie o { -claude } Code. `claude mcp get screenerbot` exibe o ambiente configurado, incluindo o segredo.
settings-agent-note-claude-desktop = { -claude } Desktop: mescle o JSON em claude_desktop_config.json em `mcpServers` e reinicie o app.
settings-agent-note-openclaw = Execute o comando e use `openclaw mcp doctor screenerbot --probe` para verificar se o servidor stdio salvo inicia e expõe as ferramentas.
settings-agent-note-hermes = Adicione isto em `mcp_servers` no arquivo de configuração do { -hermes } e reinicie o { -hermes }.
settings-agent-note-generic = Qualquer cliente MCP que use stdio: execute este comando com estes argumentos e este ambiente, onde quer que o cliente guarde sua lista de servidores.
settings-agent-block-codex-command = { -codex } CLI — comando de terminal
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (alternativa)
settings-agent-block-claude-command = { -claude } Code — comando de terminal
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — comando de terminal
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = Cliente MCP stdio genérico

settings-agent-name-required = Digite um nome para esta conexão.
settings-agent-name-too-long = O nome deve ter no máximo { $max } caracteres.
settings-agent-name-control-characters = O nome não pode conter caracteres de controle.

settings-agent-title = Conexões de agentes
settings-agent-description = Conecte o { -claude }, o { -codex }, o { -hermes }, o { -openclaw } ou qualquer cliente MCP stdio. O { -brand } precisa continuar em execução. Cada conexão tem suas próprias permissões: acesso total por padrão, limitado por conexão quando você quiser. Nenhuma conexão pode ler ou alterar a chave da sua carteira.
settings-agent-name-label = Nome da conexão
settings-agent-name-hint = Exibido na lista abaixo para você distinguir as conexões.
settings-agent-name-input =
    .placeholder = Agente de código do notebook
settings-agent-client-label = Cliente
settings-agent-client-hint = Define a configuração exibida após a criação da conexão.
settings-agent-permissions-label = Permissões
settings-agent-permissions-hint = Uma nova conexão pode fazer tudo. Limite qualquer categoria agora ou depois, pela lista abaixo. As chaves da carteira nunca ficam acessíveis de qualquer forma.
settings-agent-create = Criar conexão
settings-agent-issued-group =
    .aria-label = Credencial da nova conexão
settings-agent-issued-warning = Copie o segredo agora. Ele é exibido uma única vez e não pode ser recuperado; revogue e recrie a conexão se o perder. O { -brand } guarda apenas um verificador unidirecional; seu cliente MCP armazena o texto original na própria configuração.
settings-agent-issued-client-id = ID do cliente
settings-agent-issued-secret = Segredo de uso único
settings-agent-setup-for = Configuração para
settings-agent-done = Concluído
settings-agent-list-title = Conexões
settings-agent-loading = Carregando conexões...
settings-agent-active-count = { $count } ativas
settings-agent-empty = Nenhuma conexão ainda. Crie uma acima para parear um cliente.
settings-agent-empty-active = Nenhuma conexão ativa.
settings-agent-revoked-title = Conexões revogadas
settings-agent-created = Criada { $time }
settings-agent-last-used = Último uso { $time }
settings-agent-never-used = Nunca usada
settings-agent-permissions-edit = Permissões
settings-agent-revoke = Revogar
settings-agent-permissions-save = Salvar permissões

settings-agent-load-failed = Falha ao carregar as Conexões de agentes
settings-agent-list-failed = Não foi possível carregar as conexões
settings-agent-create-failed = Não foi possível criar a conexão.
settings-agent-unreachable-create = Não foi possível acessar o { -brand } para criar a conexão.
settings-agent-permissions-update-failed = Não foi possível atualizar as permissões
settings-agent-permissions-updated = Permissões atualizadas
settings-agent-permissions-updated-detail = Vale a partir da próxima requisição da conexão.
settings-agent-unreachable-save = Não foi possível acessar o { -brand } para salvar
settings-agent-revoke-title = Revogar conexão
settings-agent-revoke-message = Revogar "{ $label }"? O cliente deixa de funcionar na próxima requisição e não pode ser restaurado.
settings-agent-revoke-fallback-name = esta conexão
settings-agent-revoke-failed = Não foi possível revogar a conexão
settings-agent-unreachable-revoke = Não foi possível acessar o { -brand } para revogar

settings-telegram-loading = Carregando configurações do { -telegram }...
settings-telegram-load-failed = Falha ao carregar as configurações do { -telegram }
settings-telegram-unknown = Desconhecido
settings-telegram-session-active = Ativa: { $duration }
settings-telegram-sessions-empty = Nenhuma sessão ativa
settings-telegram-session-revoke = Revogar

settings-telegram-connection-title = Conexão
settings-telegram-connection-description = Conecte seu bot do { -telegram } para receber notificações e controlar o { -brand } remotamente.
settings-telegram-enable-label = Ativar { -telegram }
settings-telegram-enable-hint = Ativa a integração com o bot do { -telegram }
settings-telegram-token-label = Token do bot
settings-telegram-token-saved = Token salvo
settings-telegram-token-help = Obtenha com o @BotFather no { -telegram }
settings-telegram-token-input-saved =
    .placeholder = Token salvo (digite um novo para alterar)
settings-telegram-token-input =
    .placeholder = Digite o token do bot
settings-telegram-token-toggle =
    .title = Mostrar/ocultar
settings-telegram-chat-label = ID do chat
settings-telegram-chat-connected = Conectado ao chat:
settings-telegram-chat-discover-hint = Descubra seu ID do chat automaticamente
settings-telegram-chat-change =
    .title = Alterar
settings-telegram-chat-discover = Descobrir ID do chat
settings-telegram-discovery-step-add = Adicione seu bot a um grupo do { -telegram } ou inicie uma conversa direta com ele
settings-telegram-discovery-step-privacy = Para grupos: verifique @BotFather → /mybots → [seu bot] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>Modo de privacidade DESATIVADO:</strong> o bot recebe todas as mensagens do grupo<br/><strong>Modo de privacidade ATIVADO:</strong> o bot só recebe mensagens quando é mencionado com @
settings-telegram-discovery-step-send = Envie qualquer mensagem (ou mencione seu bot com @ se o modo de privacidade estiver ATIVADO)
settings-telegram-discovery-listening = Aguardando mensagens...
settings-telegram-discovery-select = Selecionar
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = Idioma das mensagens
settings-telegram-language-hint = Idioma das mensagens e dos botões do bot do { -telegram }
settings-telegram-language-follow-app = Seguir o idioma do app
settings-telegram-test-label = Testar conexão
settings-telegram-test-hint = Envia uma mensagem de teste para verificar a configuração
settings-telegram-test-send = Enviar teste
settings-telegram-test-sending = Enviando...

settings-telegram-chat-type-private = privado
settings-telegram-chat-type-group = grupo
settings-telegram-chat-type-supergroup = supergrupo
settings-telegram-chat-type-channel = canal

settings-telegram-auth-title = Autenticação de comandos
settings-telegram-auth-description = Os comandos do { -telegram } usam o mesmo 2FA da tela de bloqueio do painel.
settings-telegram-auth-protected = Protegido
settings-telegram-auth-disabled = Desativado
settings-telegram-auth-not-configured = Não configurado
settings-telegram-auth-error = Erro
settings-telegram-auth-protected-note = Os comandos são protegidos pelo 2FA da tela de bloqueio. Quando as sessões expiram, os usuários precisam informar o código do autenticador pelo comando <code>/login</code>.
settings-telegram-auth-disabled-note = O 2FA da tela de bloqueio está configurado, mas desativado para o { -telegram }. Ative "Exigir 2FA para comandos" acima para proteger os comandos do { -telegram }.
settings-telegram-auth-missing-note = O 2FA da tela de bloqueio não está configurado. Sem 2FA, as sessões expiradas serão reativadas automaticamente, sem verificação.
settings-telegram-auth-managed-in = O 2FA é gerenciado em
settings-telegram-auth-configure-in = Configure o 2FA em
settings-telegram-auth-configure-suffix = para exigir verificação nos comandos do { -telegram }.
settings-telegram-security-link = Configurações de segurança
settings-telegram-timeout-title = Tempo limite da sessão
settings-telegram-timeout-description = Por quanto tempo uma sessão autenticada permanece ativa
settings-telegram-sessions-title = Sessões ativas

settings-telegram-notifications-title = Configurações de notificação
settings-telegram-notifications-description = Escolha quais eventos disparam notificações do { -telegram }.
settings-telegram-notify-opened-label = Posição aberta
settings-telegram-notify-opened-hint = Notifica quando uma nova posição é aberta
settings-telegram-notify-closed-label = Posição fechada
settings-telegram-notify-closed-hint = Notifica quando uma posição é fechada
settings-telegram-notify-partial-label = Saída parcial
settings-telegram-notify-partial-hint = Notifica nas saídas parciais de posições
settings-telegram-notify-dca-label = DCA executado
settings-telegram-notify-dca-hint = Notifica quando ordens de DCA são executadas
settings-telegram-notify-errors-label = Erros
settings-telegram-notify-errors-hint = Notifica sobre erros e falhas
settings-telegram-notify-startup-label = Inicialização/encerramento
settings-telegram-notify-startup-hint = Notifica quando o bot inicia ou para
settings-telegram-notify-filtering-label = Alertas de filtragem
settings-telegram-notify-filtering-hint = Notifica quando novos tokens passam nos critérios de filtragem
settings-telegram-notify-trades-label = Alertas de trades
settings-telegram-notify-trades-hint = Notifica sobre trades relevantes de tokens monitorados
settings-telegram-notify-daily-label = Resumo diário
settings-telegram-notify-daily-hint = Receba um resumo diário da atividade de trading e do P&L

settings-telegram-features-title = Recursos
settings-telegram-features-description = Configure os recursos do bot do { -telegram }.
settings-telegram-commands-label = Ativar comandos
settings-telegram-commands-hint = Permite controlar o bot por comandos do { -telegram }
settings-telegram-require-2fa-label = Exigir 2FA para comandos
settings-telegram-require-2fa-hint = Quando as sessões expiram, exige o código 2FA para reativar. Usa o 2FA da tela de bloqueio.
settings-telegram-inline-label = Botões de ação inline
settings-telegram-inline-hint = Exibe botões de ação nas mensagens de notificação

settings-telegram-setting-save-failed = Não foi possível salvar a configuração do { -telegram }
settings-telegram-discovery-start-failed = Não foi possível iniciar a descoberta
settings-telegram-chat-selected = Chat selecionado
settings-telegram-chat-select-failed = Não foi possível selecionar o chat
settings-telegram-test-sent = Mensagem de teste enviada
settings-telegram-test-failed = Falha na mensagem de teste
settings-telegram-session-revoked = Sessão revogada
settings-telegram-session-revoke-failed = Não foi possível revogar a sessão

settings-licenses-title = Licenças de código aberto
settings-licenses-subtitle = O { -brand } é construído com os seguintes softwares de código aberto
settings-licenses-footer = Os textos completos das licenças estão disponíveis no repositório do projeto e no código-fonte de cada dependência.
settings-licenses-category-framework = Framework da aplicação
settings-licenses-category-solana = Blockchain Solana
settings-licenses-category-data = Dados e armazenamento
settings-licenses-category-networking = Rede
settings-licenses-category-cryptography = Criptografia e codificação
settings-licenses-category-assets = Recursos de interface
settings-licenses-desc-electron = Framework de aplicativos desktop
settings-licenses-desc-tokio = Runtime assíncrono para Rust
settings-licenses-desc-axum = Framework de servidor web
settings-licenses-desc-tower = Abstrações de serviço
settings-licenses-desc-hyper = Implementação de HTTP
settings-licenses-desc-solana-sdk = Núcleo do SDK da Solana
settings-licenses-desc-solana-client = Cliente RPC
settings-licenses-desc-solana-program = Biblioteca de programas
settings-licenses-desc-spl-token = Programa SPL Token
settings-licenses-desc-spl-token-2022 = Extensões do Token-2022
settings-licenses-desc-spl-associated-token-account = Contas de token associadas
settings-licenses-desc-sqlite = Motor de banco de dados embutido
settings-licenses-desc-rusqlite = Bindings do SQLite para Rust
settings-licenses-desc-r2d2 = Pool de conexões de banco de dados
settings-licenses-desc-serde = Framework de serialização
settings-licenses-desc-toml = Leitura de configuração
settings-licenses-desc-reqwest = Cliente HTTP
settings-licenses-desc-tokio-tungstenite = Cliente WebSocket
settings-licenses-desc-rustls = Implementação de TLS
settings-licenses-desc-blake3 = Função de hash
settings-licenses-desc-sha-2 = Hash SHA-256/512
settings-licenses-desc-bs58 = Codificação Base58
settings-licenses-desc-base64 = Codificação Base64
settings-licenses-desc-lucide-icons = Biblioteca de fonte de ícones
settings-licenses-desc-inter = Fonte da interface
settings-licenses-desc-jetbrains-mono = Fonte monoespaçada
settings-licenses-desc-orbitron = Fonte de exibição
settings-licenses-desc-vazirmatn = Fonte para árabe e persa
settings-licenses-desc-noto-sans-devanagari = Fonte para devanágari
settings-licenses-desc-noto-sans-sc = Fonte para chinês simplificado
settings-licenses-desc-pretendard = Fonte para coreano
settings-licenses-desc-pretendard-jp = Fonte para japonês

settings-hints-title = Dicas contextuais
settings-hints-description = As dicas contextuais são os ícones de ajuda que explicam os recursos do painel. Revise todas as dicas abaixo e restaure as que você ocultou com "Não mostrar novamente", uma a uma ou todas de uma vez.
settings-hints-hidden-label = Dicas ocultas
settings-hints-hidden-summary = { $hidden } de { $total } dicas estão ocultas no momento.
settings-hints-restore-all = Restaurar todas as dicas
settings-hints-toggle-shown =
    .title = Mostrar esta dica
settings-hints-toggle-shown-title = Visível
settings-hints-toggle-hidden-title = Oculta — ative para mostrar
settings-hints-restore-title = Restaurar todas as dicas
settings-hints-restore-message = Mostrar novamente todas as dicas contextuais, incluindo as que você ocultou?
settings-hints-restore-confirm = Restaurar todas
settings-hints-restored = Todas as dicas restauradas

settings-account-title = Conta { -brand }
settings-account-description = Gratuita e opcional. O { -brand } faz trading, descobre tokens e gera gráficos sem conta, usando os provedores públicos. O painel abaixo lista o que o login acrescenta.
settings-account-data-title = Dados do { -brand }
settings-account-data-description = Mantemos um serviço compartilhado de dados de mercado em screenerbot.io: candles agregados em sete timeframes, um registro de pools resolvido, relatórios de segurança em cache e identidade de tokens normalizada. Ele existe para que cada instalação não sofra limites de taxa separados nos provedores públicos, e o uso exige uma conta para que esse custo compartilhado tenha um responsável.
settings-account-data-fallback = Quando ele está indisponível, o { -brand } recorre automaticamente aos provedores públicos. Nada para; os gráficos são preenchidos mais devagar e têm menos histórico.
settings-account-gateway-title = Envio de transações
settings-account-gateway-description = Com o login ativo, o { -brand } pode transmitir seus swaps por screenerbot.io em vez do seu próprio RPC. O bot continua montando e assinando cada transação nesta máquina; o servidor apenas a retransmite e não consegue alterar uma transação assinada sem invalidar a assinatura.
settings-account-gateway-label = Usar o RPC do { -brand } para enviar transações
settings-account-gateway-hint = Somente envio. Os dados de preço sempre vêm do seu próprio RPC: a consulta periódica dos pools é pesada demais para um endpoint compartilhado, por isso nunca é enviada para lá.
settings-account-manage-title = Gerenciar sua conta
settings-account-manage-description = Sua senha, e-mail, dispositivos conectados e pagamentos de indicação são gerenciados no site. Revogar um dispositivo lá encerra a sessão dele em todos os lugares, inclusive neste.
settings-account-open-dashboard = Abrir seu painel

settings-navigation-title = Abas de navegação
settings-navigation-hint = Arraste os itens para reordenar. Alterne a visibilidade com o interruptor.
settings-navigation-section-layout = Layout
settings-navigation-overflow-label = Abas que não cabem
settings-navigation-overflow-hint = Rolar a fileira de abas para o lado ou reunir as que não cabem em um menu Mais no final.
settings-navigation-overflow-scroll = Rolar
settings-navigation-overflow-menu = Menu Mais
settings-navigation-drag-handle =
    .title = Arraste para reordenar
settings-navigation-defaults-failed = Não foi possível carregar a navegação padrão
settings-navigation-reset = Navegação restaurada para o padrão

settings-data-storage-title = Armazenamento do banco de dados
settings-data-storage-description = Visão geral de todos os bancos de dados que armazenam seus dados de trading, posições e informações históricas.
settings-data-stats-loading = Carregando estatísticas do banco de dados...
settings-data-stats-load-failed = Falha ao carregar as estatísticas do banco de dados
settings-data-total-storage = Armazenamento total do banco de dados
settings-data-db-tokens = Tokens
settings-data-db-transactions = Transações
settings-data-db-positions = Posições
settings-data-db-events = Eventos
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = Carteira
settings-data-db-pools = Pools
settings-data-db-strategies = Estratégias
settings-data-db-actions = Ações
settings-data-directory-label = Diretório de dados
settings-data-directory-copied = Diretório de dados
settings-data-config-path-copied = Caminho da configuração
settings-data-path-unavailable = Indisponível
settings-data-path-copy-title = Clique para copiar o caminho
settings-data-path-copy-failed = Falha ao copiar o caminho

settings-data-config-title = Gerenciamento de configuração
settings-data-config-description = Exporte, importe e gerencie a configuração do seu bot. Faça backups antes de grandes mudanças.
settings-data-config-export = Exportar configuração
settings-data-config-import = Importar configuração
settings-data-config-reset = Restaurar padrões
settings-data-config-location-label = Local da configuração
settings-data-config-fetch-failed = Falha ao obter a configuração
settings-data-config-exported = Configuração exportada
settings-data-config-export-failed = Falha ao exportar a configuração: { $message }
settings-data-config-import-title = Importar configuração
settings-data-config-import-message = Importar esta configuração? As configurações atuais serão sobrescritas. As credenciais da carteira serão preservadas.
settings-data-config-imported = Configuração importada com sucesso. Algumas mudanças podem exigir reinicialização.
settings-data-config-import-failed = Falha ao importar a configuração: { $message }
settings-data-config-reset-title = Restaurar configuração
settings-data-config-reset-message = Restaurar todas as configurações para o padrão? As credenciais da carteira serão preservadas, mas todas as outras configurações serão redefinidas.
settings-data-config-reset-done = Configuração restaurada para o padrão
settings-data-config-reset-failed = Falha ao restaurar a configuração: { $message }
settings-data-unknown-error = Erro desconhecido

settings-data-cleanup-title = Limpeza de dados
settings-data-cleanup-description = Libere espaço em disco removendo dados antigos ou não utilizados. Estas ações não podem ser desfeitas.
settings-data-ohlcv-cleanup-label = Limpeza de dados OHLCV
settings-data-ohlcv-cleanup-hint = Remove os dados de candles de tokens que não ficaram ativos pelo tempo especificado.
settings-data-cleanup-hours-unit = h
settings-data-cleanup-ohlcv = Limpar OHLCV
settings-data-cleanup-running = Limpando...
settings-data-cleanup-hours-invalid = Valor de horas inválido
settings-data-cleanup-confirm-title = Excluir dados OHLCV
settings-data-cleanup-confirm-message =
    Excluir os dados OHLCV de tokens inativos há mais de { $hours ->
        [one] { $hours } hora
        [many] { $hours } horas
       *[other] { $hours } horas
    }?
settings-data-cleanup-done =
    { $count ->
        [one] { $count } token inativo removido
        [many] { $count } tokens inativos removidos
       *[other] { $count } tokens inativos removidos
    }
settings-data-cleanup-failed = Falha na limpeza
settings-data-cleanup-failed-detail = Falha na limpeza: { $message }

settings-data-cache-clear-label = Limpar todo o cache OHLCV
settings-data-cache-clear-hint = Apaga todos os dados de candles em cache e busca novamente cada token monitorado do zero. Use se os gráficos parecerem errados ou após uma atualização da lógica de dados.
settings-data-cache-clear = Limpar cache OHLCV
settings-data-cache-clearing = Limpando...
settings-data-cache-confirm-title = Limpar todo o cache OHLCV
settings-data-cache-confirm-message = Apagar todos os dados de candles em cache de todos os tokens? Os tokens monitorados buscarão o histórico novamente do zero. Isso não pode ser desfeito.
settings-data-candles-count =
    { $count ->
        [one] { $count } candle
        [many] { $count } candles
       *[other] { $count } candles
    }
settings-data-tokens-count =
    { $count ->
        [one] { $count } token
        [many] { $count } tokens
       *[other] { $count } tokens
    }
settings-data-cache-cleared = { $candles } removidos em { $tokens }; buscando novamente
settings-data-cache-clear-failed = Falha ao limpar o cache OHLCV
settings-data-cache-clear-failed-detail = Falha ao limpar o cache OHLCV: { $message }

settings-data-ui-cache-label = Cache de estado da interface
settings-data-ui-cache-hint = Limpa as preferências de tabela, os estados de filtro e as configurações de exibição salvos.
settings-data-ui-cache-clear = Limpar cache da interface
settings-data-ui-cache-confirm-title = Limpar estado da interface
settings-data-ui-cache-confirm-message = Limpar todas as preferências de interface salvas? Isso redefine as colunas das tabelas, os filtros e as configurações de exibição.
settings-data-ui-cache-cleared =
    { $count ->
        [one] { $count } configuração de interface em cache removida
        [many] { $count } configurações de interface em cache removidas
       *[other] { $count } configurações de interface em cache removidas
    }

settings-data-folder-label = Abrir pasta de dados
settings-data-folder-hint = Abre no gerenciador de arquivos a pasta com todos os dados do { -brand }.
settings-data-folder-open = Abrir pasta
settings-data-folder-open-failed = Não foi possível abrir a pasta de dados
