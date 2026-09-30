# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = A instalação automática está desativada. A atualização está pronta e será aplicada quando você escolher.
updates-defer-trading-active = Há uma posição, trade ou ferramenta em operação, então a reinicialização foi adiada. A atualização é aplicada automaticamente quando o app estiver ocioso.
updates-defer-needs-installer = Esta versão também atualiza a interface desktop, então o instalador precisa ser executado uma vez.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = Baixando a atualização v{ $version }...
updates-apply-started = Instalando a atualização. O { -brand } reinicia e reconecta automaticamente.
updates-install-opened = Instalador da atualização verificada aberto. Conclua o instalador do sistema operacional.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = Instalador aberto
updates-installer-toast-message = O { -brand } será encerrado com segurança agora.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = Status
updates-tab-release-notes = Notas de versão
updates-tab-preferences = Preferências
updates-tab-sections = Seções de atualização
updates-checking-installation = Verificando esta instalação...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = Pronto para verificar atualizações
updates-phase-idle-detail = O { -brand } v{ $version } está instalado.
updates-phase-up-to-date-headline = Você está com a versão mais recente
updates-phase-up-to-date-detail = O { -brand } v{ $version } é a versão mais recente.
updates-phase-checking-headline = Verificando atualizações
updates-phase-checking-detail = Procurando a versão publicada mais recente.
updates-phase-available-headline = A versão { $version } está disponível
updates-phase-downloading-headline = Baixando a v{ $version }
updates-phase-verifying-headline = Verificando a v{ $version }
updates-phase-verifying-detail = Conferindo o download com o checksum publicado.
updates-phase-ready-to-apply-headline = A versão { $version } está pronta
updates-phase-ready-to-apply-detail = A atualização pode ser instalada agora com uma reinicialização rápida, ou automaticamente na próxima inicialização.
updates-phase-ready-to-install-headline = A versão { $version } está pronta
updates-phase-ready-to-install-detail = O instalador desktop está pronto para concluir esta atualização.
updates-phase-applying-headline = Instalando atualização
updates-phase-applying-detail = O { -brand } está reiniciando na nova versão.
updates-phase-applied-headline = Atualizado para a v{ $version }
updates-phase-applied-detail = A atualização foi instalada. Nada mais é necessário.
updates-phase-failed-headline = A atualização não foi concluída
updates-phase-failed-detail = Tente atualizar novamente.
updates-phase-check-failed-headline = Não foi possível verificar atualizações
updates-phase-check-failed-detail = Não foi possível acessar o serviço de versões.
updates-status-unavailable-headline = O status da atualização está indisponível
updates-phase-unrecognized-detail = O estado de atualização informado não é reconhecido.
updates-status-load-failed-detail = Não foi possível carregar o status da instalação.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = Atualização do core · { $size } · reinicialização rápida
updates-kind-full = Atualização desktop · { $size } · instalador necessário
updates-size-unknown = tamanho desconhecido

updates-action-check-now = Verificar agora
updates-action-check-again = Verificar novamente
updates-action-try-again = Tentar novamente
updates-action-download = Baixar atualização
updates-action-restart = Reiniciar para atualizar
updates-action-open-installer = Abrir instalador

updates-busy-checking = Verificando...
updates-busy-resuming = Retomando download...
updates-busy-starting-download = Iniciando download...
updates-busy-restarting = Reiniciando...
updates-busy-opening-installer = Abrindo instalador...

updates-progress-downloading = Baixando atualização
updates-progress-verifying = Verificando atualização
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } de { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = Detalhes da instalação
updates-detail-installed-version = Versão instalada
updates-detail-system = Sistema
updates-detail-last-checked = Última verificação
updates-detail-never = Nunca
updates-detail-available-version = Versão disponível
updates-detail-download-size = Tamanho do download

updates-version-installed = Instalada
updates-version-available = Disponível

updates-notes-highlights = Destaques
updates-notes-empty-title = Ainda não há notas de versão
updates-notes-empty-error = Não foi possível carregar o histórico de versões. Verifique a conexão e tente novamente.
updates-notes-empty-none = As notas de versão aparecerão aqui assim que uma versão for publicada.
updates-notes-history-notice = Exibindo o que esta instalação já conhece: não foi possível carregar o histórico de versões.
updates-release-empty = Nenhuma alteração foi listada para esta versão.
updates-release-changes =
    { $count ->
        [one] { $count } alteração
        [many] { $count } alterações
       *[other] { $count } alterações
    }

updates-preferences-unavailable-title = As preferências de atualização estão indisponíveis
updates-preferences-unavailable-detail = Não foi possível carregar a configuração de atualização.
updates-preference-fallback-name = preferência de atualização
updates-preference-save-failed = Não foi possível salvar { $preference }

updates-request-failed = Falha na requisição
updates-check-request-failed = Não foi possível verificar atualizações
updates-resume-failed = Não foi possível retomar o download da atualização
updates-download-failed = Não foi possível iniciar o download da atualização
updates-apply-failed = Não foi possível instalar a atualização
updates-install-failed = Não foi possível abrir o instalador da atualização
updates-apply-confirm-title = Instalar a v{ $version }
updates-apply-confirm-message = O { -brand } reinicia na nova versão. O trading para por alguns segundos e retoma automaticamente; as posições abertas não são afetadas.
updates-install-confirm-title = Executar o instalador
updates-install-confirm-message = O instalador verificado é aberto e o { -brand } é encerrado com segurança. Conclua o instalador e reabra o { -brand }.

# A release version as displayed.
updates-version-number = v{ $version }
