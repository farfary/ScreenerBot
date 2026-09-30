# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = OK

## Splash and loading status.

desktop-splash-starting = Iniciando o { -brand }
desktop-splash-restarting = Reiniciando o { -brand }
desktop-splash-recovering = Recuperando
desktop-splash-opening-dashboard = Abrindo o painel
desktop-splash-checking-dependencies = Verificando dependências
desktop-splash-installing-dependencies = Instalando dependências do sistema
desktop-splash-installing-dependencies-detail = O { -brand } precisa do Microsoft Visual C++ Redistributable para funcionar.
desktop-splash-resetting-wallet = Redefinindo os dados da carteira
desktop-splash-resetting-wallet-detail = Os dados atuais da carteira passam por backup antes de serem apagados.
desktop-splash-updating = Atualizando para a v{ $version }
desktop-splash-updating-detail = Suas configurações e dados permanecem exatamente como estão.
desktop-splash-restoring = Restaurando a v{ $version }
desktop-splash-restoring-detail = A atualização v{ $failed } não iniciou, então a versão anterior está assumindo.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = Não foi possível iniciar o { -brand }
desktop-boot-detail-fallback = O backend parou inesperadamente.
desktop-boot-remedy-label = Como resolver
desktop-boot-log-file-label = Arquivo de log:
desktop-boot-action-reset-wallet = Redefinir dados da carteira e reiniciar
desktop-boot-action-working = Processando...
desktop-boot-action-open-logs = Abrir pasta de logs
desktop-boot-action-copy = Copiar detalhes
desktop-boot-action-copied = Copiado
desktop-boot-action-quit = Sair
desktop-boot-subtitle-wallet-mismatch = Uma carteira diferente foi detectada
desktop-boot-subtitle-port-in-use = Uma porta de rede necessária está ocupada
desktop-boot-subtitle-lock-held = O { -brand } já está em execução
desktop-boot-subtitle-config-invalid = Problema de configuração
desktop-boot-subtitle-directory-setup = Problema de armazenamento
desktop-boot-subtitle-generic = Erro de inicialização

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = Não foi possível iniciar o { -brand }
desktop-boot-error-remedy = Abra a pasta de logs para ver o que aconteceu e reinicie o app. Se o problema persistir, fale com o suporte em t.me/screenerbotio_support.
desktop-boot-error-default = O backend parou inesperadamente antes de o painel ficar pronto.
desktop-boot-error-restore-failed = O backend atualizado falhou e não foi possível restaurar a versão anterior ({ $error }).
desktop-boot-error-spawn-failed = Não foi possível iniciar o programa do backend ({ $error }).
desktop-boot-error-spawn-missing = Não foi possível iniciar o programa do backend. Ele pode estar ausente ou bloqueado por um software de segurança.
desktop-boot-error-exited-running = O backend parou enquanto o painel estava em execução (código de saída { $code }).
desktop-boot-error-exited-early = O backend parou antes de o painel ficar pronto (código de saída { $code }).
desktop-boot-error-dashboard-load = Falha ao carregar o painel ({ $description }, { $code }).
desktop-boot-error-renderer-gone = O renderizador do painel parou ({ $reason }).
desktop-boot-error-unresponsive = O painel deixou de responder.
desktop-boot-error-url-failed = Não foi possível carregar a URL do painel ({ $error }).
desktop-boot-error-relaunch-setup = Não foi possível reiniciar o backend após a configuração.
desktop-boot-error-relaunch-recovery = Não foi possível reiniciar o backend para a recuperação.
desktop-boot-error-restart-offline = O backend não voltou a ficar online após a reinicialização.
desktop-boot-error-recovery-offline = A recuperação terminou, mas o backend não ficou pronto.
desktop-boot-error-start-timeout = O backend não terminou de iniciar a tempo. Isso pode acontecer na primeira execução em um computador lento ou se outro programa estiver bloqueando a conexão.

## System tray.

desktop-tray-tooltip = { -brand } - Bot de trading Solana
desktop-tray-show = Mostrar o { -brand }
desktop-tray-open-dashboard = Abrir painel
desktop-tray-quit = Sair do { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = Abrir pasta de dados
desktop-menu-open-logs-folder = Abrir pasta de logs
desktop-menu-documentation = Documentação
desktop-menu-telegram-support = Suporte no { -telegram }
desktop-menu-check-updates = Verificar atualizações...

## Application menu.

desktop-menu-file = Arquivo
desktop-menu-edit = Editar
desktop-menu-view = Exibir
desktop-menu-window = Janela
desktop-menu-help = Ajuda
desktop-menu-reset-zoom = Restaurar zoom
desktop-menu-zoom-in = Aumentar zoom
desktop-menu-zoom-out = Diminuir zoom
desktop-menu-keyboard-shortcuts = Atalhos de teclado
desktop-menu-telegram-channel = Canal no { -telegram }
desktop-menu-telegram-community = Comunidade no { -telegram }
desktop-menu-follow-x = Seguir no { -x } ({ -twitter })
desktop-menu-visit-website = Visitar o site
desktop-menu-about = Sobre o { -brand }

## About dialog.

desktop-about-title = Sobre o { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    Versão { $version }

    Bot avançado de gestão de carteiras e trading automático na Solana.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = Atalhos de teclado
desktop-shortcuts-message = Atalhos de teclado do { -brand }
desktop-shortcuts-body-mac =
    Atalhos de teclado:

    Controles da janela:
      Cmd+M          Minimizar
      Cmd+W          Fechar janela
      Cmd+Q          Sair
      Cmd+Ctrl+F     Alternar tela cheia

    Zoom:
      Cmd++          Aumentar zoom
      Cmd+-          Diminuir zoom
      Cmd+0          Restaurar zoom

    Navegação:
      Cmd+R          Recarregar painel
      Cmd+Shift+D    Abrir pasta de dados

    Outros:
      F1             Abrir documentação
      Cmd+Alt+I      Alternar DevTools
desktop-shortcuts-body-other =
    Atalhos de teclado:

    Controles da janela:
      Alt+F4         Sair
      F11            Alternar tela cheia

    Zoom:
      Ctrl++         Aumentar zoom
      Ctrl+-         Diminuir zoom
      Ctrl+0         Restaurar zoom

    Navegação:
      Ctrl+R         Recarregar painel
      Ctrl+Shift+D   Abrir pasta de dados

    Outros:
      F1             Abrir documentação
      Ctrl+Shift+I   Alternar DevTools

## Close confirmation (Windows and Linux).

desktop-close-title = Fechar o { -brand }
desktop-close-message = O que você deseja fazer?
desktop-close-detail = O { -brand } pode continuar em execução em segundo plano. O bot de trading continua monitorando e operando enquanto estiver minimizado na bandeja do sistema.
desktop-close-minimize = Minimizar para a bandeja
desktop-close-quit = Sair por completo
desktop-close-cancel = Cancelar

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = Dependência ausente
desktop-vcredist-missing-message = O Visual C++ Redistributable está ausente
desktop-vcredist-missing-detail = O { -brand } precisa do Microsoft Visual C++ Redistributable para funcionar. Deseja instalá-lo agora?
desktop-vcredist-install = Instalar e corrigir
desktop-vcredist-exit = Sair
desktop-vcredist-not-found-title = Instalador não encontrado
desktop-vcredist-not-found-message = Não foi possível localizar { $name } corretamente.
desktop-vcredist-done-title = Instalação concluída
desktop-vcredist-done-message = Dependências instaladas com sucesso.
desktop-vcredist-done-detail = O { -brand } será iniciado agora.
desktop-vcredist-failed-title = Falha na instalação
desktop-vcredist-failed-message = Instale o Visual C++ Redistributable manualmente.
