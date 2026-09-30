# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = A configuração em memória difere da versão em disco
system-result-config-matches = A configuração em memória é igual à versão em disco

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    Importação concluída: { $count ->
        [one] { $count } seção
        [many] { $count } seções
       *[other] { $count } seções
    }
system-result-config-imported-with-warnings =
    Importadas { $count ->
        [one] { $count } seção
        [many] { $count } seções
       *[other] { $count } seções
    } com { $warnings ->
        [one] { $warnings } aviso
        [many] { $warnings } avisos
       *[other] { $warnings } avisos
    }: { $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = Buscar configurações...
system-config-export-title =
    .title = Exportar configuração para arquivo
system-config-import-title =
    .title = Importar configuração de arquivo
system-config-reload = Recarregar do disco
system-config-reset-defaults = Restaurar padrões
system-config-select-section = Selecione uma seção de configuração
system-config-select-section-details = Selecione uma seção de configuração para ver os detalhes.
system-config-no-metadata = Sem metadados para <code>{ $section }</code>
system-config-technical-settings = Configurações técnicas
system-config-expand-title = Expandir todas as seções e todas as subconfigurações aninhadas
system-config-collapse-title = Recolher todas as seções e todas as subconfigurações aninhadas
system-config-toolbar-no-changes = Nenhuma alteração na seção
system-config-toolbar-section-changes =
    { $count ->
        [one] <strong>{ $count }</strong> alteração na seção
        [many] <strong>{ $count }</strong> alterações na seção
       *[other] <strong>{ $count }</strong> alterações na seção
    }
system-config-toolbar-total-changes =
    { $count ->
        [one] <strong>{ $count }</strong> alteração no total
        [many] <strong>{ $count }</strong> alterações no total
       *[other] <strong>{ $count }</strong> alterações no total
    }

## Config page: state banner

system-config-loading = Carregando configuração…
system-config-refreshing = Atualizando configuração…
system-config-saving-title = Salvando alterações…
system-config-saving-detail = Atualizando configuração
system-config-validation-issues = <strong>Problemas de validação detectados.</strong> Revise os campos destacados.

## Config page: section header and category chips

system-config-save-changes = Salvar alterações
system-config-saving = Salvando…
system-config-compare = Comparar com o disco
system-config-revert-section = Reverter seção
system-config-summary-critical = { $count } críticos
system-config-summary-performance = { $count } de desempenho
system-config-summary-pending =
    { $count ->
        [one] { $count } alteração pendente
        [many] { $count } alterações pendentes
       *[other] { $count } alterações pendentes
    }
system-config-summary-none = Sem resumo de metadados
system-config-fields-count =
    { $count ->
        [one] { $count } campo
        [many] { $count } campos
       *[other] { $count } campos
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · { $pending } pendentes
system-config-chip-visible = { $visible } de { $fields }

## Config page: field rows

system-config-field-unit = Unidade: { $unit }
system-config-field-default = Padrão: { $value }
system-config-field-reset = Restaurar padrão
system-config-array-invalid-title = Entrada de array inválida
system-config-json-invalid-title = JSON inválido
system-config-list-separator = { ", " }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
        [one] A linha { $lines } deve ser um número inteiro válido.
        [many] As linhas { $lines } devem ser números inteiros válidos.
       *[other] As linhas { $lines } devem ser números inteiros válidos.
    }
system-config-array-invalid-number =
    { $count ->
        [one] A linha { $lines } deve ser um número válido.
        [many] As linhas { $lines } devem ser números válidos.
       *[other] As linhas { $lines } devem ser números válidos.
    }
system-config-array-invalid-boolean =
    { $count ->
        [one] A linha { $lines } deve ser um booleano válido.
        [many] As linhas { $lines } devem ser booleanos válidos.
       *[other] As linhas { $lines } devem ser booleanos válidos.
    }
system-config-array-invalid-value =
    { $count ->
        [one] A linha { $lines } deve ser um valor válido.
        [many] As linhas { $lines } devem ser valores válidos.
       *[other] As linhas { $lines } devem ser valores válidos.
    }

## Config page: Telegram actions

system-config-telegram-actions = Ações
system-config-telegram-test-title = Testar conexão
system-config-telegram-test-description = Envie uma mensagem de teste para verificar se a sua configuração do { -telegram } está funcionando
system-config-telegram-send-test = Enviar mensagem de teste
system-config-telegram-sending = Enviando...
system-config-telegram-configure-token-title = Configure o token do bot primeiro
system-config-telegram-configure-token-status = Configure o token do bot acima para habilitar o teste
system-config-telegram-test-sent-status = Mensagem de teste enviada com sucesso! Confira o seu { -telegram }.
system-config-telegram-test-sent = Mensagem de teste do { -telegram } enviada
system-config-telegram-test-failed = Falha ao enviar a mensagem de teste
system-config-telegram-auth-title = Autenticação do bot
system-config-telegram-totp-title = Autenticação de dois fatores (TOTP)
system-config-telegram-totp-configured = Configurado
system-config-telegram-totp-not-configured = Não configurado
system-config-telegram-totp-active = A autenticação de dois fatores está ativa. Sessões expiradas do { -telegram } exigem o código TOTP do seu app autenticador.
system-config-telegram-totp-inactive = Ative a autenticação de dois fatores nas configurações de Segurança para proteger os comandos do { -telegram }.
system-config-telegram-totp-note = O TOTP é compartilhado com a tela de bloqueio do painel. Configure-o nas configurações de Segurança.
system-config-telegram-require-2fa = Exigir 2FA nos comandos
# $status is the HTTP status code.
system-config-telegram-save-rejected = Salvamento rejeitado ({ $status })
system-config-telegram-save-failed = Não foi possível salvar a configuração do { -telegram }

## Config page: operations

system-config-saved = Configuração salva
system-config-save-failed = Não foi possível salvar a configuração
system-config-reloaded = Configuração recarregada do disco
system-config-reload-failed = Não foi possível recarregar a configuração
system-config-diff-title = Diferenças da configuração
system-config-diff-console = Gravado no console do navegador
system-config-diff-failed = Não foi possível calcular as diferenças
system-config-reset-title = Restaurar configuração
system-config-reset-message =
    Isso restaura toda a configuração para os valores padrão embutidos. Todas as configurações atuais serão perdidas.

    Esta ação não pode ser desfeita.
system-config-reset-done-title = Configuração restaurada
system-config-reset-done-message = Todas as configurações voltaram aos valores padrão
system-config-reset-failed = Não foi possível restaurar a configuração
system-config-load-failed = Não foi possível carregar a configuração
system-config-metadata-failed = Não foi possível carregar os metadados da configuração

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = Fechar
system-config-select-none = Desmarcar tudo
system-config-section-gui = GUI
system-config-changes-count =
    { $count ->
        [one] { $count } alteração
        [many] { $count } alterações
       *[other] { $count } alterações
    }
system-config-sections-count =
    { $count ->
        [one] { $count } seção
        [many] { $count } seções
       *[other] { $count } seções
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-rpc = Endpoints RPC e configurações de conexão
system-config-section-hint-trader = Regras de trading e automação
system-config-section-hint-positions = Configurações de gestão de posições
system-config-section-hint-filtering = Regras e limites de filtragem de tokens
system-config-section-hint-swaps = Configurações de execução de swaps
system-config-section-hint-tokens = Descoberta de tokens e fontes de dados
system-config-section-hint-sol-price = Configuração do serviço de preço do { -sol }
system-config-section-hint-events = Configurações de registro de eventos
system-config-section-hint-services = Configurações dos serviços em segundo plano
system-config-section-hint-monitoring = Configuração de monitoramento do sistema
system-config-section-hint-ohlcv = Configurações de dados de candles
system-config-section-hint-gui = Configurações do painel e da interface
system-config-section-hint-telegram = Configuração do bot do { -telegram }

## Export dialog

system-config-export-dialog-title = Exportar configuração
system-config-export-intro = Selecione quais seções da configuração exportar. O arquivo exportado pode ser importado depois para restaurar ou compartilhar configurações.
system-config-export-sections = Seções
system-config-export-timestamp = Incluir data e hora da exportação
system-config-sections-selected =
    { $count ->
        [one] { $count } seção selecionada
        [many] { $count } seções selecionadas
       *[other] { $count } seções selecionadas
    }
system-config-exporting = Exportando...
system-config-export-invalid-response = Resposta inválida do servidor
system-config-exported-title = Configuração exportada
system-config-exported-message =
    { $count ->
        [one] { $count } seção exportada
        [many] { $count } seções exportadas
       *[other] { $count } seções exportadas
    }
system-config-export-failed-title = Falha na exportação
system-config-export-failed = Falha ao exportar a configuração

## Import dialog

system-config-import-dialog-title = Importar configuração
system-config-import-upload-intro = Envie um arquivo de configuração exportado anteriormente. Você poderá visualizar e selecionar quais seções importar.
system-config-import-dropzone-title = Solte o arquivo de configuração aqui
system-config-import-dropzone-hint = ou clique para procurar
system-config-import-analyzing = Analisando configuração...
system-config-import-preview = Prévia
system-config-import-preview-intro = Revise as seções da configuração abaixo. Selecione quais seções importar.
system-config-import-sections = Seções no arquivo
system-config-import-select-valid = Selecionar todas as válidas
system-config-import-merge-label = Mesclar com o existente
system-config-import-merge-hint = Atualiza apenas os campos presentes no arquivo. Desmarcado = substitui seções inteiras.
system-config-import-save-label = Salvar em disco
system-config-import-save-hint = Grava as alterações no config.toml após a importação
system-config-import-selected = Importar selecionadas
system-config-import-warnings =
    { $count ->
        [one] { $count } aviso
        [many] { $count } avisos
       *[other] { $count } avisos
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = A seção desconhecida "{ $section }" será ignorada
system-config-import-warning-sensitive-field = Importar { $field } pode sobrescrever configurações de autenticação
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = Não está no arquivo
system-config-import-status-invalid = Configuração inválida
system-config-import-status-unchanged = Sem alterações
system-config-import-not-included = Não incluída no arquivo
system-config-import-show-changes = Mostrar alterações
system-config-import-hide-changes = Ocultar alterações
system-config-import-value-current = Valor atual
system-config-import-value-new = Novo valor
system-config-import-more-changes =
    { $count ->
        [one] +{ $count } alteração
        [many] +{ $count } alterações
       *[other] +{ $count } alterações
    }
system-config-import-value-items =
    { "[" }{ $count ->
        [one] { $count } item
        [many] { $count } itens
       *[other] { $count } itens
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
        [one] { $count } chave
        [many] { $count } chaves
       *[other] { $count } chaves
    }{ "}" }
system-config-importing = Importando...
system-config-import-failed = Falha na importação
system-config-import-invalid-file-title = Arquivo inválido
system-config-import-invalid-file = Falha ao interpretar o arquivo de configuração
system-config-imported-title = Configuração importada
system-config-imported-message =
    { $count ->
        [one] { $count } seção importada
        [many] { $count } seções importadas
       *[other] { $count } seções importadas
    }
system-config-import-failed-title = Falha na importação
system-config-import-failed-message = Falha ao importar a configuração
