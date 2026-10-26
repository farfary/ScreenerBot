# Fatal startup errors. Server-only: rendered by src/errors/startup.rs into the
# finished text that the Electron shell displays, never sent to the dashboard.

## Wallet mismatch.

startup-wallet-mismatch-title = 지갑이 변경되었습니다
startup-wallet-mismatch-detail =
    설정에 있는 지갑이 이 컴퓨터의 로컬 기록에 저장된 지갑과 일치하지 않습니다.

    현재 지갑: { $current }
    이전 지갑: { $stored }

    영향을 받는 로컬 데이터: { $systems }

    다른 프라이빗 키를 가져오거나 다른 설정을 복원한 후에 주로 발생합니다. 거래, 포지션, 기록은 이전 지갑에 속하므로 새 지갑을 안전하게 시작하려면 먼저 삭제해야 합니다.
startup-wallet-mismatch-systems-default = 트랜잭션, 포지션, 지갑 기록
startup-wallet-mismatch-remedy =
    계속하려면 이전 지갑의 로컬 기록을 삭제하세요 (데이터베이스는 먼저 자동으로 백업됩니다):

      - 앱에서: 아래의 "{ $action }"을 선택하세요.
      - 터미널에서: screenerbot --clean-wallet-data 를 실행하세요

    온체인 자산에는 영향이 없으며 이 컴퓨터의 로컬 거래/포지션 기록만 초기화됩니다. 백업은 다음 위치에 저장됩니다:
      { $path }
startup-recovery-reset-wallet = 지갑 데이터 초기화 후 재시작

## Port in use.

startup-port-in-use-title = 네트워크 포트가 사용 중입니다
startup-port-in-use-detail = 대시보드 포트({ $address })가 이미 사용 중입니다.
startup-port-in-use-remedy = 다른 프로그램이 { -brand }에 필요한 포트를 사용하고 있습니다. 해당 프로그램을 종료하거나 설정에서 웹서버 포트를 변경한 후 { -brand }을 다시 시작하세요.

## Another instance is running.

startup-lock-held-title = { -brand }이 이미 실행 중입니다
startup-lock-held-detail = 이 컴퓨터에서 { -brand }의 다른 사본이 이미 실행 중이므로 두 번째 사본을 시작할 수 없습니다.
startup-lock-held-remedy = 이미 열려 있는 창으로 전환하세요. 창이 보이지 않으면 백그라운드에서 실행 중인 { -brand } 프로세스를 종료하고 다시 시도하세요. 재부팅 후에도 문제가 계속되면 잠금 파일이 오래된 것일 수 있으며, 데이터 폴더의 .screenerbot.lock 파일을 삭제할 수 있습니다.

## Configuration.

startup-config-invalid-title = 설정을 읽을 수 없습니다
startup-config-parse-detail = config.toml을 파싱할 수 없습니다: { $detail }
startup-config-load-parse-detail = 설정을 불러오지 못했습니다: config.toml을 파싱할 수 없습니다: { $detail }
startup-config-parse-remedy = 설정 파일을 읽을 수 없습니다. 데이터 폴더의 백업을 복원하거나, 설정을 기본값으로 초기화한 후 지갑과 RPC를 다시 설정하세요.
startup-config-load-parse-remedy = 올바른 설정을 복원하거나 설정을 다시 완료하세요.
startup-option-invalid-title = 시작 옵션이 올바르지 않습니다
startup-option-invalid-remedy = 명령줄 옵션이 올바르지 않습니다. 해당 옵션 없이 { -brand }을 시작하거나, 옵션을 수정한 후 다시 시도하세요.

## Storage upgrade.

startup-storage-upgrade-title = 데이터를 업그레이드할 수 없습니다
startup-storage-upgrade-detail =
    { -brand }이(가) { $database }을(를) 이 버전으로 업그레이드하지 못해 변경하기 전에 중지했습니다. 데이터는 변경되지 않았습니다.

    원인:
    { $error }
startup-storage-upgrade-remedy = 세부 정보를 복사해 로그 파일과 함께 t.me/screenerbotio_support 지원팀에 보내 주세요. 데이터베이스를 편집, 이동, 삭제하지 마세요. 수정 사항이 설치되면 { -brand }이(가) 다시 엽니다.

## Wallet replacement and wallet data reset.

startup-wallet-replacement-incomplete-title = 지갑 교체를 완료할 수 없습니다
startup-wallet-replacement-incomplete-detail =
    { -brand }이 새 트레이딩 지갑으로 전환하는 중에 중지되었습니다. 삭제된 것은 없습니다. 이전 지갑의 키는 지갑 보관소에 남아 있고, 로컬 기록은 원래 위치나 로그 파일에 적힌 백업 폴더에 있습니다.

    원인:
    { $error }
startup-wallet-replacement-incomplete-remedy = { -brand }을 다시 시작하면 교체가 이어서 진행됩니다. 이 메시지가 다시 나타나면 세부 정보를 복사해 로그 파일과 함께 t.me/screenerbotio_support의 지원팀에 보내세요. wallet_replacement.json과 백업 폴더를 편집, 이동, 삭제하지 마세요.
startup-wallet-reset-refused-title = 지갑 데이터가 초기화되지 않았습니다
startup-wallet-reset-refused-open-positions = 트레이딩 지갑에 열린 포지션이 있어 ({ $count }) 지갑 데이터를 초기화하지 않았습니다.
startup-wallet-reset-refused-positions-unreadable = 포지션 저장소를 읽을 수 없어 열린 포지션이 없다고 확인할 수 없으므로 지갑의 로컬 기록이 보관되었습니다.
startup-wallet-reset-refused-remedy = 이동된 것은 없습니다. 초기화 없이 { -brand }을 시작해 트레이딩 지갑의 열린 포지션을 종료하거나 보관한 후 지갑 데이터를 다시 초기화하세요.
startup-wallet-reset-refused-unreadable-remedy = 이동된 것은 없습니다. 초기화 없이 { -brand }을 시작해 포지션 저장소를 확인하게 한 후 지갑 데이터를 다시 초기화하세요. 이 메시지가 다시 나타나면 로그 파일을 t.me/screenerbotio_support의 지원팀에 보내세요.
startup-wallet-reset-incomplete-title = 지갑 데이터 초기화를 완료할 수 없습니다
startup-wallet-reset-incomplete-detail =
    { -brand }이 지갑 데이터를 { $directory }(으)로 옮기는 중에 중지되었고 모든 파일을 되돌리지 못했습니다. 지갑의 로컬 기록 일부는 그 폴더에, 일부는 원래 위치에 있습니다.

    원인:
    { $error }
startup-wallet-reset-incomplete-remedy = { -brand }을 다시 시작하지 말고 { $directory }의 파일을 이동, 편집, 삭제하지 마세요. 세부 정보를 복사해 로그 파일과 함께 t.me/screenerbotio_support의 지원팀에 보내세요.

## Generic failures.

startup-generic-title = { -brand }을 시작할 수 없습니다
startup-generic-remedy = 로그 파일에서 자세한 내용을 확인한 후 앱을 다시 시작하세요. 문제가 계속되면 t.me/screenerbotio_support 로 지원팀에 문의하세요.
startup-generic-detail = { $error }
startup-failure-directories = 필요한 디렉터리를 만들지 못했습니다: { $error }
startup-failure-config-load = 설정을 불러오지 못했습니다: { $error }
startup-failure-actions-sync = 데이터베이스에서 작업을 동기화하지 못했습니다: { $error }
startup-failure-analysis-init = 분석 엔진을 초기화하지 못했습니다: { $error }
startup-failure-assistant-init = 어시스턴트 채팅 엔진을 초기화하지 못했습니다: { $error }
startup-failure-wallets-init = 지갑을 초기화하지 못했습니다: { $error }
startup-failure-wallet-validation = 지갑 일관성을 검증하지 못했습니다: { $error }
startup-failure-storage-phase = 스토리지 단계가 예기치 않게 중지되었습니다: { $error }
