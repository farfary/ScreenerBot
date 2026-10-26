# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = Gerada
wallets-type-imported = Importada
wallets-type-migrated = Migrada

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = Pausado por você
wallets-watch-disabled-signature-budget = Pausado: atingiu o limite de { $limit } assinaturas por verificação antes de se atualizar
wallets-watch-disabled-unknown = Pausado: não foi possível ler o motivo de segurança salvo do monitoramento
wallets-watch-disabled-helius-unavailable = Pausado: o provedor de alta atividade está indisponível; cursor preservado
wallets-watch-disabled-processing-failed = Pausado: não foi possível processar a atividade da carteira; cursor preservado

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = Provedor de alta atividade indisponível; monitoramento pausado
wallets-watch-error-provider-repeated-failure = As verificações do { -helius } falharam repetidamente; monitoramento pausado
wallets-watch-error-processing-repeated-failure = O processamento da atividade da carteira falhou repetidamente; monitoramento pausado
wallets-watch-error-position-unreadable = O monitoramento da carteira não conseguiu ler a posição salva; tentando novamente
wallets-watch-error-provider-check-failed = Falha na verificação do provedor de alta atividade; tentando novamente
wallets-watch-error-decode-failed = Não foi possível decodificar a transação de alta atividade; cursor mantido
wallets-watch-error-processing-failed = Não foi possível processar a atividade da carteira; tentando novamente
wallets-watch-error-position-save-failed = O monitoramento da carteira não conseguiu salvar a posição; tentando novamente

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = Pausado por você.
wallets-watch-reason-signature-budget = Esta carteira tem mais atividade do que o monitoramento atual consegue verificar.
wallets-watch-reason-helius-unavailable = As verificações do { -helius } falharam. O progresso salvo foi preservado.
wallets-watch-reason-processing-failed = Não foi possível processar a atividade da carteira. O progresso salvo foi preservado.

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = Fechar janela
wallets-summary-native = { -sol }
wallets-copied-private-key = Chave privada

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Carteira principal
wallets-tab-other = Outras carteiras
wallets-tab-watched = Monitoradas
wallets-refresh-failed = Não foi possível atualizar as carteiras
wallets-action-failed = Falhou
wallets-toast-failed = Falhou: { $reason }
wallets-export-busy = Descriptografando...
wallets-export-revealed = Chave revelada: manuseie com cuidado

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = Monitorar carteira
wallets-watch-add-address = Endereço da carteira
wallets-watch-add-address-input =
    .placeholder = Endereço Solana
wallets-watch-add-address-hint = Registra a atividade on-chain da carteira e envia alertas de trade pelas suas configurações do { -telegram }.
wallets-watch-add-label = Rótulo
wallets-watch-add-label-input =
    .placeholder = Nome (opcional)
wallets-watch-add-submit = Adicionar monitoramento

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = Opções de monitoramento da carteira
wallets-watch-budget-title-restore = Restaurar monitoramento da carteira
wallets-watch-budget-close =
    .aria-label = Fechar
wallets-watch-budget-label-signatures = Assinaturas verificadas por checagem
wallets-watch-budget-label-transactions = Transações completas bem-sucedidas verificadas por checagem
wallets-watch-budget-hint-signatures = Limite atual: { $limit }. Escolha de 500 a 5.000 assinaturas por checagem, em intervalos de 100.
wallets-watch-budget-hint-transactions = Limite atual: { $limit }. Escolha de 500 a 5.000 transações bem-sucedidas por checagem, em intervalos de 100.
wallets-watch-budget-error-range = Escolha entre 500 e 5.000 registros por checagem, em intervalos de 100.
wallets-watch-budget-error-ack = Confirme que as assinaturas desde a última checagem concluída serão ignoradas.
wallets-watch-budget-save-failed = Não foi possível salvar o limite do monitoramento.
wallets-watch-budget-save = Salvar limite
wallets-watch-budget-resume = Retomar a partir de agora
wallets-watch-budget-resume-notice = Esta carteira atingiu o limite de checagem antes de se atualizar. Retomar a partir de agora começa na atividade mais recente da carteira; a atividade desde a última checagem concluída não será copiada.
wallets-watch-budget-resume-tasks = As tarefas de cópia continuam pausadas até você retomar cada uma em Copy Trading.
wallets-watch-budget-resume-ack = Entendo que a atividade perdida não será copiada.
wallets-watch-budget-resumed = Monitoramento retomado a partir do ponto atual da carteira
wallets-watch-budget-updated = Limite de monitoramento da carteira atualizado
wallets-watch-helius-allow = Permitir recuperação via { -helius } se necessário
wallets-watch-helius-try = Tentar recuperar usando o { -helius }
wallets-watch-helius-stop = Parar a recuperação via { -helius } desta carteira
wallets-watch-helius-description-approved = A recuperação via { -helius } está permitida para esta carteira. Ao desativá-la, volta às checagens padrão, que podem ficar para trás em uma carteira movimentada.
wallets-watch-helius-description-available = O { -helius } pode verificar transações Solana bem-sucedidas a partir da posição salva, sem pular o intervalo não verificado. Pode consumir mais créditos do provedor e ainda assim ficar para trás.
wallets-watch-helius-description-unavailable = A recuperação via { -helius } está indisponível. Configure um endpoint RPC do { -helius } ativado para usá-la.
wallets-watch-helius-description-unsupported = Nenhum provedor de recuperação é compatível com este monitoramento. Retomar a partir de agora fica disponível se o monitoramento atingir o limite.
wallets-watch-helius-allow-title = Permitir recuperação via { -helius } para esta carteira
wallets-watch-helius-allow-message = O { -helius } pode verificar transações Solana bem-sucedidas a partir da posição salva, sem pular o intervalo não verificado. Atualmente cobra 10 créditos a cada 100 transações completas retornadas, arredondando para cima, com mínimo de 10 créditos por requisição. Uma checagem pode fazer várias requisições; o consumo e o preço do provedor podem variar. As tarefas de cópia continuam pausadas até serem retomadas separadamente.
wallets-watch-helius-allow-confirm = Permitir para esta carteira
wallets-watch-helius-stop-message = Esta carteira voltará às checagens padrão. Uma carteira movimentada pode atingir o limite de monitoramento e pausar novamente. As outras carteiras e a sua configuração RPC do { -helius } não mudam.
wallets-watch-helius-stop-confirm = Parar para esta carteira
wallets-watch-helius-stop-keep = Manter permitido
wallets-watch-helius-restored = Monitoramento restaurado a partir do progresso salvo; as tarefas de cópia continuam pausadas
wallets-watch-helius-allowed = Recuperação via { -helius } permitida para esta carteira quando necessário
wallets-watch-helius-stopped = Recuperação via { -helius } interrompida para esta carteira
wallets-watch-helius-update-failed = Não foi possível atualizar a configuração de recuperação da carteira

# wallets.html: Export Private Key dialog.
wallets-export-title = Exportar chave privada
wallets-export-warning-title = Aviso crítico de segurança
wallets-export-warning-body = Nunca compartilhe sua chave privada com ninguém. Qualquer pessoa com acesso a esta chave pode roubar todos os fundos desta carteira.
wallets-export-key-label = Chave privada (Base58)
wallets-export-copy =
    .title = Copiar para a área de transferência
    .aria-label = Copiar para a área de transferência
wallets-export-reveal = Revelar chave

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Saldo
wallets-holdings-col-value = Valor ({ -sol })
wallets-holdings-col-type = Tipo
wallets-holdings-col-decimals = Decimais
wallets-holdings-empty-title = Nenhum token em carteira
wallets-holdings-unreadable-title = Ativos desconhecidos
wallets-holdings-empty-message = Os tokens mantidos por esta carteira aparecerão aqui.
wallets-funding-title = Deposite na carteira de trading para começar
wallets-funding-message = Esta carteira não tem { -sol }. Envie { -sol } para este endereço de uma corretora ou outra carteira; cada compra gasta o seu valor mais as taxas de rede.
wallets-holdings-no-main = Nenhuma carteira principal
wallets-holdings-main-title = Carteira principal
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Último uso
wallets-holdings-never = Nunca
wallets-holdings-search =
    .placeholder = Buscar por símbolo ou mint...
wallets-holdings-export = Exportar chave
wallets-holdings-export-tooltip = Exportar a chave privada desta carteira
wallets-holdings-replace = Substituir carteira
wallets-holdings-replace-tooltip = Opere com outra carteira; a chave e o histórico desta carteira são mantidos
wallets-list-col-name = Nome
wallets-list-col-balance = Saldo ({ -sol })
wallets-list-col-type = Tipo
wallets-list-col-created = Criada
wallets-list-col-replaced = Substituída
wallets-list-replaced-on = Em { $date }
wallets-replace-abandoned-title = Substituição da carteira cancelada
wallets-replace-abandoned-open-positions = A carteira de trading tinha posições abertas ({ $count }) na reinicialização, então a substituição foi cancelada. A carteira de trading não mudou; a nova chave fica em Outras carteiras.
wallets-replace-abandoned-positions-unreadable = Não foi possível ler o armazenamento de posições na reinicialização, então a substituição foi cancelada. A carteira de trading não mudou; a nova chave fica em Outras carteiras.
wallets-replace-abandoned-vault-changed = O cofre de carteiras não correspondia mais à substituição preparada na reinicialização, então a substituição foi cancelada. A carteira de trading não mudou; a nova chave fica em Outras carteiras.
wallets-list-col-actions = Ações
wallets-list-action-export = Exportar chave privada
wallets-list-count = Carteiras
wallets-list-search =
    .placeholder = Buscar por nome ou endereço...
wallets-list-loading-title = Carregando carteiras…
wallets-list-loading-description = Preparando a visualização da carteira selecionada.
wallets-list-kept-empty-message = As carteiras de trading substituídas e as carteiras de versões anteriores aparecem aqui para que você ainda possa exportar as chaves delas.
wallets-list-previous-trading-wallet = Carteira de trading anterior
wallets-other-empty-title = Nenhuma outra carteira

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = Carteira
wallets-watched-col-status = Status
wallets-watched-col-progress = Progresso salvo
wallets-watched-col-last-check = Última checagem
wallets-watched-unlabelled = Carteira sem rótulo
wallets-watched-generic-name = carteira
wallets-watched-not-synced = Ainda não sincronizada
wallets-watched-not-checked = Ainda não verificada
wallets-watched-action-copy = Copiar trades
    .title = Abrir esta carteira em Copy Trading
wallets-watched-action-restore = Restaurar monitoramento
wallets-watched-action-options = Opções de monitoramento
wallets-watched-action-retry = Tentar monitoramento novamente
wallets-watched-action-pause = Pausar
wallets-watched-action-enable = Ativar
wallets-watched-action-remove =
    .title = Remover
    .aria-label = Remover { $name }
wallets-watch-state-paused = Pausado
wallets-watch-state-catching-up = Recuperando
wallets-watch-state-watching = Monitorando
wallets-watch-state-streaming = Streaming
wallets-watch-state-polling = Consulta periódica
wallets-watched-detail-helius = Verificando pelo { -helius } para esta carteira.
wallets-watched-empty-title = Nenhum endereço monitorado
wallets-watched-empty-message = Use Monitorar carteira para registrar a atividade on-chain de uma carteira pública.
wallets-watched-count = Monitoradas
wallets-watched-search =
    .placeholder = Buscar carteiras monitoradas...
wallets-watched-add = Monitorar carteira
wallets-watched-refresh = Atualizar carteiras monitoradas
wallets-watched-loading-title = Carregando carteiras monitoradas...
wallets-watched-loading-description = Buscando alvos de observação.
wallets-watched-load-error-title = Não foi possível carregar os endereços monitorados
wallets-watched-load-error-description = Use atualizar para tentar de novo.
wallets-watched-address-invalid = Informe um endereço de carteira Solana válido.
wallets-watched-added = Monitoramento de carteira adicionado
wallets-watched-duplicate = Essa carteira já está sendo monitorada.
wallets-watched-add-failed = Não foi possível adicionar o monitoramento da carteira.
wallets-watched-retried = Monitoramento da carteira restaurado com o cursor salvo
wallets-watched-paused = Monitoramento da carteira pausado
wallets-watched-enabled = Monitoramento da carteira ativado
wallets-watched-removed = Monitoramento da carteira removido
wallets-watched-update-failed = Não foi possível atualizar o monitoramento da carteira
wallets-setup-gate-title = As carteiras exigem configuração
