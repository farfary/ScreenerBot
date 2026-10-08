# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.
#
# Remedies are written for the compiled binary. Paths, ports, wallet addresses
# and error details arrive as arguments. Keep the command-line flag, the file
# names and the support handle unchanged.

## Wallet mismatch.

startup-wallet-mismatch-title = A carteira mudou
startup-wallet-mismatch-detail =
    A carteira da sua configuração não corresponde à carteira registrada no histórico local deste computador.

    Carteira atual: { $current }
    Carteira anterior: { $stored }

    Dados locais afetados: { $systems }

    Isso costuma acontecer depois de importar uma chave privada diferente ou restaurar uma configuração diferente. O trading, as posições e o histórico pertencem à carteira anterior e precisam ser limpos antes que a nova carteira possa iniciar com segurança.
startup-wallet-mismatch-systems-default = Transações, posições, histórico da carteira
startup-wallet-mismatch-remedy =
    Limpe o histórico local da carteira anterior para continuar (seus bancos de dados recebem backup automático antes):

      - No app: escolha "{ $action }" abaixo.
      - Por um terminal: execute  screenerbot --clean-wallet-data

    Nenhum fundo on-chain é afetado; apenas o histórico local de trades/posições deste computador é redefinido. Os backups são gravados em:
      { $path }
startup-recovery-reset-wallet = Redefinir dados da carteira e reiniciar

## Port in use.

startup-port-in-use-title = A porta de rede está ocupada
startup-port-in-use-detail = A porta do painel { $address } já está em uso.
startup-port-in-use-remedy = Outro programa está usando a porta de que o { -brand } precisa. Feche esse programa ou altere a porta do servidor web em Configurações e inicie o { -brand } novamente.

## Another instance is running.

startup-lock-held-title = O { -brand } já está em execução
startup-lock-held-detail = Outra cópia do { -brand } já está em execução neste computador, então uma segunda não pode iniciar.
startup-lock-held-remedy = Mude para a janela que já está aberta. Se não encontrar nenhuma, encerre qualquer processo do { -brand } em segundo plano e tente novamente. Se o problema persistir após reiniciar o computador, o arquivo de bloqueio pode estar obsoleto e pode ser removido da pasta de dados (.screenerbot.lock).

## Configuration.

startup-config-invalid-title = Não foi possível ler a configuração
startup-config-parse-detail = Não foi possível analisar o config.toml: { $detail }
startup-config-load-parse-detail = Falha ao carregar a configuração: não foi possível analisar o config.toml: { $detail }
startup-config-parse-remedy = Não foi possível ler o seu arquivo de configuração. Restaure um backup da pasta de dados ou redefina a configuração para os padrões e configure sua carteira e o RPC novamente.
startup-config-load-parse-remedy = Restaure uma configuração válida ou conclua a configuração inicial novamente.
startup-option-invalid-title = Opção de inicialização inválida
startup-option-invalid-remedy = Uma opção de linha de comando é inválida. Inicie o { -brand } sem essa opção ou corrija-a e tente novamente.

## Storage upgrade.

startup-storage-upgrade-title = Não foi possível atualizar seus dados
startup-storage-upgrade-detail =
    O { -brand } não conseguiu atualizar { $database } para esta versão e parou antes de alterá-lo. Seus dados não foram alterados.

    Causa:
    { $error }
startup-storage-upgrade-remedy = Copie os detalhes e envie-os com o arquivo de log ao suporte em t.me/screenerbotio_support. Não edite, mova nem exclua o banco de dados: o { -brand } o abrirá novamente assim que uma correção for instalada.

## Generic failures.

startup-generic-title = Não foi possível iniciar o { -brand }
startup-generic-remedy = Consulte o arquivo de log para ver os detalhes e reinicie o app. Se o problema persistir, fale com o suporte em t.me/screenerbotio_support.
startup-generic-detail = { $error }
startup-failure-directories = Falha ao criar os diretórios necessários: { $error }
startup-failure-config-load = Falha ao carregar a configuração: { $error }
startup-failure-actions-init = Falha ao inicializar o banco de dados de ações: { $error }
startup-failure-actions-sync = Falha ao sincronizar as ações do banco de dados: { $error }
startup-failure-strategy-init = Falha ao inicializar o sistema de estratégias: { $error }
startup-failure-analysis-init = Falha ao inicializar o motor de análise: { $error }
startup-failure-assistant-init = Falha ao inicializar o motor de chat do Assistente: { $error }
startup-failure-wallets-init = Falha ao inicializar as carteiras: { $error }
startup-failure-wallet-validation = Falha ao validar a consistência da carteira: { $error }
