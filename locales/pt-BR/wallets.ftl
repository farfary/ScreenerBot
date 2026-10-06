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
wallets-field-address = Endereço
wallets-field-name = Nome da carteira
wallets-field-notes = Notas
wallets-field-private-key = Chave privada
wallets-address-copy = Copiar endereço
wallets-modal-close =
    .aria-label = Fechar janela
wallets-this-wallet = esta carteira
wallets-summary-native = { -sol }
wallets-copied-address = Endereço
wallets-copied-mint = Endereço do mint
wallets-copied-private-key = Chave privada

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = Carteira principal
wallets-tab-secondaries = Secundárias
wallets-tab-archive = Arquivo
wallets-tab-watched = Monitoradas
wallets-refresh-failed = Não foi possível atualizar as carteiras
wallets-action-failed = Falhou
wallets-toast-failed = Falhou: { $reason }
wallets-create-busy = Criando...
wallets-create-fallback = Falha na criação
wallets-create-done = Carteira "{ $name }" criada!
wallets-import-busy = Importando...
wallets-import-failed = Falha na importação
wallets-import-done = Carteira "{ $name }" importada!
wallets-archive-busy = Arquivando...
wallets-archive-confirm-text = Tem certeza de que deseja arquivar <strong>{ $name }</strong>?
wallets-archive-done = Carteira arquivada
wallets-restore-done = Carteira restaurada
wallets-export-busy = Descriptografando...
wallets-export-revealed = Chave revelada: manuseie com cuidado
wallets-delete-busy = Excluindo...
wallets-delete-confirm-text = Tem certeza de que deseja excluir <strong>{ $name }</strong>?
wallets-delete-done = Carteira excluída permanentemente

# wallets.html: Add Wallet dialog.
wallets-add-title = Adicionar carteira
wallets-add-tab-create = Criar nova
wallets-add-tab-import = Importar existente
wallets-create-name-input =
    .placeholder = Ex.: Carteira de trading
wallets-create-name-hint = Um nome simples para identificar esta carteira
wallets-create-notes-input =
    .placeholder = Descrição ou finalidade (opcional)...
wallets-create-submit = Criar carteira
wallets-import-warning-title = Aviso de segurança
wallets-import-warning-body = Importe chaves privadas apenas de fontes confiáveis. Sua chave será criptografada e armazenada com segurança neste dispositivo.
wallets-import-name-input =
    .placeholder = Ex.: Minha carteira
wallets-import-key-input =
    .placeholder = Texto em base58 ou array JSON [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = Alternar visibilidade da chave privada
wallets-import-key-hint = Aceita chave codificada em base58 ou formato de array de bytes
wallets-import-notes-input =
    .placeholder = Descrição (opcional)...
wallets-import-submit = Importar carteira

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

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = Arquivar carteira
wallets-archive-note = Carteiras arquivadas não são usadas em nenhuma operação, mas podem ser restauradas a qualquer momento.
wallets-archive-confirm = Sim, arquivar
wallets-delete-title = Excluir carteira
wallets-delete-warning-title = Esta ação não pode ser desfeita!
wallets-delete-warning-body = Excluir esta carteira remove permanentemente ela e a chave privada criptografada deste dispositivo.
wallets-delete-confirm = Sim, excluir

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = Importar carteiras
wallets-bulk-import-submit = Importar carteiras
wallets-bulk-step-upload = Enviar arquivo
wallets-bulk-step-map = Mapear colunas
wallets-bulk-step-results = Resultados
wallets-bulk-import-file-warning-body = Importe arquivos apenas de fontes confiáveis. As chaves privadas serão criptografadas e armazenadas com segurança neste dispositivo.
wallets-bulk-drop-title = Solte seu arquivo aqui
wallets-bulk-drop-subtitle = ou clique para procurar
wallets-bulk-drop-formats = Aceita CSV e Excel (.xlsx, .xls)
wallets-bulk-file-remove =
    .aria-label = Remover arquivo
wallets-bulk-map-subtitle = Associe as colunas do arquivo aos campos da carteira
wallets-bulk-preview-title = Prévia (primeiras 5 linhas)
wallets-bulk-summary-valid = <strong>{ $count }</strong> válidas
wallets-bulk-summary-invalid = <strong>{ $count }</strong> inválidas
wallets-bulk-summary-duplicate =
    { $count ->
        [one] <strong>{ $count }</strong> duplicada
        [many] <strong>{ $count }</strong> duplicadas
       *[other] <strong>{ $count }</strong> duplicadas
    }
wallets-bulk-done = Concluído
wallets-bulk-file-invalid = Tipo de arquivo inválido. Use arquivos CSV ou Excel.
wallets-bulk-preview-busy = Processando...
wallets-bulk-preview-fallback = Falha ao processar o arquivo
wallets-bulk-preview-failed = Falha ao processar o arquivo: { $reason }
wallets-bulk-column-select = -- Selecionar coluna --
wallets-bulk-preview-empty = Nenhuma linha de dados encontrada no arquivo
wallets-bulk-preview-status = Status
wallets-bulk-status-valid = Válida
wallets-bulk-status-duplicate = Duplicada
wallets-bulk-status-invalid = Inválida
wallets-bulk-import-busy = Importando...
wallets-bulk-import-toast =
    { $count ->
        [one] { $count } carteira importada
        [many] { $count } carteiras importadas
       *[other] { $count } carteiras importadas
    }
wallets-bulk-import-error = Falha na importação: { $reason }
wallets-bulk-result-success-title = Importação concluída
wallets-bulk-result-success-detail =
    { $count ->
        [one] { $count } carteira importada com sucesso
        [many] Todas as { $count } carteiras foram importadas com sucesso
       *[other] Todas as { $count } carteiras foram importadas com sucesso
    }
wallets-bulk-result-partial-title = Sucesso parcial
wallets-bulk-result-partial-detail = { $imported } importadas, { $failed } com falha
wallets-bulk-result-failed-title = Falha na importação
wallets-bulk-result-failed-detail =
    { $count ->
        [one] Falha ao importar { $count } carteira
        [many] Falha ao importar todas as { $count } carteiras
       *[other] Falha ao importar todas as { $count } carteiras
    }
wallets-bulk-result-imported = Importadas
wallets-bulk-result-failed = Falharam

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = Exportar carteiras
wallets-bulk-export-format = Formato
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = Incluir carteiras arquivadas
wallets-bulk-export-safe-title = Exportação segura
wallets-bulk-export-safe-body = Exporta apenas endereços e metadados das carteiras. Não inclui chaves privadas.
wallets-bulk-export-safe-submit = Exportar endereços
wallets-bulk-export-or = ou
wallets-bulk-export-danger-title = Exportação perigosa
wallets-bulk-export-danger-body = Inclui as chaves privadas na exportação. Qualquer pessoa com este arquivo pode roubar seus fundos.
wallets-bulk-export-danger-submit = Exportar com chaves privadas
wallets-bulk-export-busy = Exportando...
wallets-bulk-export-done = Carteiras exportadas para { $filename }
wallets-bulk-export-fallback = Falha na exportação
wallets-bulk-export-error = Falha na exportação: { $reason }
wallets-bulk-confirm-title = Confirmar exportação perigosa
wallets-bulk-confirm-warning =
    { $count ->
        [one] Você está prestes a exportar <strong>{ $count }</strong> chave privada. Isso é extremamente perigoso!
        [many] Você está prestes a exportar <strong>{ $count }</strong> chaves privadas. Isso é extremamente perigoso!
       *[other] Você está prestes a exportar <strong>{ $count }</strong> chaves privadas. Isso é extremamente perigoso!
    }
wallets-bulk-confirm-risk-steal = Qualquer pessoa com este arquivo pode roubar todos os fundos
wallets-bulk-confirm-risk-share = Nunca compartilhe este arquivo com ninguém
wallets-bulk-confirm-risk-delete = Exclua o arquivo imediatamente após o uso
wallets-bulk-confirm-prompt = Digite a frase abaixo para confirmar
wallets-bulk-confirm-submit = Exportar chaves

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = Token
wallets-holdings-col-balance = Saldo
wallets-holdings-col-value = Valor ({ -sol })
wallets-holdings-col-type = Tipo
wallets-holdings-col-decimals = Decimais
wallets-holdings-col-mint = Mint
wallets-holdings-empty-title = Nenhum token em carteira
wallets-holdings-empty-message = Os tokens mantidos por esta carteira aparecerão aqui.
wallets-holdings-no-main = Nenhuma carteira principal
wallets-holdings-main-tag = Principal
wallets-holdings-main-title = Carteira principal
wallets-holdings-tokens = Tokens
wallets-holdings-last-used = Último uso
wallets-holdings-never = Nunca
wallets-holdings-search =
    .placeholder = Buscar por símbolo ou mint...
wallets-holdings-export = Exportar chave
wallets-holdings-export-tooltip = Exportar a chave privada desta carteira
wallets-list-col-name = Nome
wallets-list-col-balance = Saldo ({ -sol })
wallets-list-col-type = Tipo
wallets-list-col-created = Criada
wallets-list-col-actions = Ações
wallets-list-action-export = Exportar chave privada
wallets-list-action-archive = Arquivar carteira
wallets-list-action-restore = Restaurar carteira
wallets-list-action-delete = Excluir permanentemente
wallets-list-count = Carteiras
wallets-list-search =
    .placeholder = Buscar por nome ou endereço...
wallets-list-loading-title = Carregando carteiras…
wallets-list-loading-description = Preparando a visualização da carteira selecionada.
wallets-secondaries-empty-title = Nenhuma carteira secundária
wallets-secondaries-empty-message = Crie carteiras adicionais para organizar suas atividades de trading em várias contas.
wallets-secondaries-add = Adicionar carteira
wallets-archive-empty-title = Nenhuma carteira arquivada
wallets-archive-empty-message = As carteiras que você arquivar ficarão guardadas aqui com segurança para consulta futura.

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
