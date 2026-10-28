# Wallet and RPC setup: the full-screen wizard, its shared validators and the Explore Mode setup dialog.

# Source: scripts/core/setup_runtime.js

## Wallet key validation

setup-wallet-required = 지갑 개인 키를 입력하세요.
setup-wallet-json-recognized = 64바이트 JSON 키 형식이 확인되었습니다.
setup-wallet-json-invalid = 정확히 64개의 바이트 값(0–255)으로 이루어진 JSON 배열을 사용하세요.
setup-wallet-format-invalid = base58 개인 키 또는 64바이트 JSON 배열을 사용하세요.
setup-wallet-base58-recognized = Base58 키 형식이 확인되었습니다.

## RPC endpoint validation

setup-rpc-required = RPC 엔드포인트를 하나 이상 입력하세요.
setup-dialog-rpc-optional-note = 선택 사항: 이미 설정된 RPC 엔드포인트를 유지하려면 필드를 비워 두세요.
setup-rpc-too-many = RPC 엔드포인트는 10개 이하로 사용하세요.
setup-rpc-url-invalid = 모든 엔드포인트는 올바른 HTTPS URL이어야 합니다.
setup-rpc-url-credentials = RPC URL에는 사용자 이름이나 비밀번호를 포함할 수 없습니다.
setup-rpc-url-fragment = RPC URL에는 프래그먼트를 포함할 수 없습니다.
setup-rpc-public-endpoint = 공용 Solana RPC는 지속적인 폴링을 지원하지 않습니다.
setup-rpc-private-host = RPC 엔드포인트는 로컬 또는 사설 네트워크 호스트를 사용할 수 없습니다.
setup-rpc-duplicate = 중복된 RPC 엔드포인트를 제거하세요.
setup-rpc-ready =
    { $count ->
       *[other] HTTPS 엔드포인트 { $count }개를 테스트할 준비가 되었습니다.
    }

## Verification results

setup-wallet-verified = 지갑 확인됨
setup-wallet-unverified = 지갑을 확인하지 못했습니다
setup-wallet-address-detail = 주소 { $address }
setup-wallet-format-hint = 개인 키 형식을 확인하세요.
setup-rpc-none-working = 작동하는 메인넷 RPC 없음
setup-rpc-health-failed = 메인넷 상태 점검을 통과한 엔드포인트가 없습니다.
setup-rpc-partial = 작동 { $working }개, 사용 불가 { $failed }개
setup-rpc-verified =
    { $count ->
       *[other] 메인넷 엔드포인트 { $count }개 확인됨
    }
setup-rpc-fastest = 가장 빠름: { $url } ({ $latency }ms).
setup-error-request-failed = 요청 실패 ({ $status })
setup-error-restart-timeout = { -brand }가 아직 다시 연결되지 않았습니다.

# Source: scripts/core/setup.js

## Verification steps

setup-verify-wallet-parsing = 개인 키 해석 중
setup-verify-wallet-parsing-detail = 키를 확인하고 공개 주소를 도출하는 중입니다.
setup-verify-wallet-waiting = 검증 대기 중
setup-verify-rpc-testing = Solana 메인넷 테스트 중
setup-verify-rpc-testing-detail =
    { $count ->
       *[other] 엔드포인트 { $count }개를 확인하는 중입니다.
    }
setup-verify-rpc-waiting = 엔드포인트 테스트 대기 중
setup-verify-save-waiting = 저장 대기 중
setup-verify-save-running = 암호화 및 저장 중
setup-verify-save-running-detail = 검증된 설정을 이 기기에 기록하는 중입니다.
setup-verify-save-done = 설정 저장됨
setup-verify-save-done-detail = 개인 키가 암호화되었고 작동하는 RPC 엔드포인트가 저장되었습니다.
setup-verify-save-failed = 설정을 저장하지 못했습니다
setup-verify-save-skipped = 저장되지 않음. 위 문제를 해결한 뒤 다시 검증하세요.
setup-verify-request-failed = 검증 요청에 실패했습니다
setup-verify-summary-checking = 지갑과 Solana 메인넷 연결을 확인하는 중입니다.
setup-verify-summary-running = 입력한 인증 정보를 그대로 검증하는 중입니다.
setup-verify-summary-saving = 인증 정보가 검증되었습니다. 안전하게 저장하는 중입니다.
setup-verify-summary-failed = 문제를 확인한 후 다시 검증하세요.

## Errors

setup-error-credentials-failed = 인증 정보 검증에 실패했습니다.
setup-error-save-failed = 설정을 저장하지 못했습니다.
setup-error-verify-failed = 검증에 실패했습니다.
setup-error-explore-failed = 탐색 모드를 시작하지 못했습니다.
setup-error-gateway-failed = 트랜잭션 전송 설정을 저장하지 못했습니다.
setup-action-review-credentials = 인증 정보 검토

## Completion

setup-explore-opening = 탐색 모드를 여는 중…
setup-complete-restarting = 검증된 설정으로 { -brand }을 재시작하는 중입니다.
setup-complete-finishing = 재시작 마무리 중…
setup-complete-ready = { -brand }이 준비되었습니다. 대시보드를 여는 중…
setup-complete-stored = 검증된 설정이 이 기기에 안전하게 저장되었습니다.

## Wallet controls (shared with the setup dialog)

setup-wallet-show-key = 개인 키 표시
setup-wallet-hide-key = 개인 키 숨기기
setup-wallet-copy =
    .aria-label = 지갑 주소 복사
    .title = 지갑 주소 복사
setup-wallet-copy-done =
    .aria-label = 지갑 주소가 복사되었습니다
    .title = 복사됨
setup-wallet-copy-failed =
    .aria-label = 지갑 주소를 복사하지 못했습니다
    .title = 복사 실패

# Source: scripts/ui/setup_dialog.js

## Setup dialog

setup-dialog-title = 지갑 및 RPC 설정
setup-dialog-subtitle = Solana 지갑과 프리미엄 RPC 엔드포인트를 연결하면 거래와 실시간 온체인 데이터를 사용할 수 있습니다. 개인 키는 이 기기에서 암호화되며 외부로 전송되지 않습니다.
setup-dialog-close =
    .title = 닫기
    .aria-label = 닫기
setup-dialog-wallet-label = 지갑 개인 키
setup-dialog-wallet-input =
    .placeholder = Base58 문자열 또는 JSON 배열 [1,2,3,...]
setup-dialog-submit = 검증 및 연결
setup-dialog-working = 처리 중…
setup-dialog-validating = 검증 중…
setup-dialog-saving = 저장 중…
setup-dialog-restarting = 재시작 중…
setup-dialog-saved = 설정이 저장되었습니다. { -brand }을 전체 모드로 재시작하는 중…
setup-dialog-error-validation = 검증에 실패했습니다.
setup-dialog-error-incomplete = 설정을 완료하지 못했습니다.
setup-dialog-error-restart-helper = 자동 재시작 도우미를 사용할 수 없습니다. 잠시 후 대시보드를 새로 고치세요.
setup-dialog-error-unexpected = 예기치 않은 오류가 발생했습니다.
setup-dialog-replace-title = 트레이딩 지갑 교체
setup-dialog-replace-subtitle = 다른 지갑으로 거래합니다. 해당 개인 키는 이 기기에서 암호화되며 기기 밖으로 나가지 않습니다.
setup-dialog-replace-submit = 검증 및 교체
setup-dialog-replace-saved = 교체가 준비되었습니다. 새 지갑으로 { -brand }을 재시작하는 중…
setup-dialog-replace-confirm-intro = 트레이딩 지갑을 교체하면:
setup-dialog-replace-confirm-stop = 트레이더와 카피 트레이딩이 중지됩니다. 재시작 후 다시 켜세요.
setup-dialog-replace-confirm-backup = 현재 지갑의 포지션, 트랜잭션, 기록이 백업 폴더로 이동됩니다.
setup-dialog-replace-confirm-keep = 현재 지갑의 키는 기타 지갑에 보관되며, 계속 내보낼 수 있습니다.
setup-dialog-replace-confirm-funds = 자금은 이동되지 않습니다. 토큰과 { -sol }은 현재 지갑에 남습니다.
setup-dialog-replace-confirm-rpc = 설정된 RPC 엔드포인트를 입력한 것으로 바꿉니다. 필드를 비워 두면 현재 엔드포인트가 유지됩니다.
setup-dialog-replace-confirm-restart = 새 지갑으로 { -brand }이 재시작됩니다.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = 설정 진행 상황
setup-wizard-wallet-security-note = 저장하기 전에 암호화됩니다.
setup-wizard-rpc-title = RPC 엔드포인트
setup-wizard-rpc-input =
    .placeholder = 한 줄에 HTTPS URL 하나
setup-wizard-rpc-guidance = 안정적인 메인넷 제공업체를 사용하세요. 공개 엔드포인트는 지원되지 않습니다.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 권장
setup-wizard-account-title = { -brand } 계정
setup-wizard-account-optional = 선택 사항
setup-wizard-account-loading = 계정 상태 확인 중…
setup-wizard-verify-title = 검증 및 저장
setup-wizard-verify-list =
    .aria-label = 설정 검증 상태
setup-wizard-verify-wallet = 지갑
setup-wizard-verify-save = 보안 설정
setup-wizard-complete-title = 설정 저장됨
setup-wizard-reconnect = 연결 재시도
setup-wizard-reload = 대시보드 새로 고침
setup-wizard-error-title = 설정 확인이 필요합니다
setup-wizard-explore = 대시보드 둘러보기
setup-wizard-continue = 계속

## Setup wizard: wallet, connection and funding

setup-wizard-step-wallet = 지갑
setup-wizard-step-connection = 연결
setup-wizard-step-verify = 확인
setup-wizard-step-ready = 준비 완료
setup-wallet-title = 거래 지갑 설정
setup-wallet-description = { -brand }가 거래에 사용하는 지갑입니다. 키는 이 컴퓨터에서 암호화되며 절대 외부로 나가지 않습니다.
setup-wallet-source =
    .aria-label = 지갑 출처
setup-wallet-create-title = 새 지갑 만들기
setup-wallet-create-detail = { -brand }가 새 지갑을 만듭니다. 키를 저장한 뒤 지갑으로 { -sol }을 보내세요.
setup-wallet-import-title = 지갑 가져오기
setup-wallet-import-detail = 이미 가지고 있는 개인 키를 사용합니다(base58 또는 숫자 64개로 된 JSON 배열).
setup-wallet-creating = 지갑을 만드는 중…
setup-wallet-create-failed = 지갑을 만들지 못했습니다. 다시 시도하세요.
setup-backup-title = 개인 키 저장
setup-backup-description = 이 키를 가진 사람은 누구나 지갑을 제어할 수 있습니다. { -brand }는 키를 복구할 수 없습니다.
setup-backup-address = 주소
setup-backup-key = 개인 키
setup-backup-copy-address = 주소 복사
setup-backup-download = 다운로드
setup-backup-copied = 복사했습니다. 저장한 뒤 클립보드를 비우세요.
setup-backup-machine-warning = { -brand }가 보관하는 암호화된 사본은 이 컴퓨터에서만 열립니다. 컴퓨터를 잃어버리거나 교체하면 지금 저장하는 키로만 지갑을 복원할 수 있습니다. 종이나 비밀번호 관리자처럼 오프라인에 보관하세요.
setup-backup-ack-saved = 개인 키를 이 컴퓨터 밖의 안전한 곳에 저장했습니다.
setup-backup-ack-control = 이 키를 가진 사람은 누구나 이 지갑의 자금을 가져갈 수 있다는 것을 이해합니다.
setup-backup-ack-recovery = { -brand }가 이 키를 복구할 수 없다는 것을 이해합니다. 키가 없으면 컴퓨터를 잃는 것은 자금을 잃는 것입니다.
setup-backup-create-another = 다른 지갑 만들기
setup-backup-replace-confirm = 이 키를 버리고 새 지갑을 만들까요? 아직 이 지갑으로 아무것도 보내지 않았을 때만 하세요.
setup-connection-title = { -brand }를 { -solana }에 어떻게 연결할까요?
setup-connection-description = 나중에 설정 → 연결에서 바꿀 수 있습니다.
setup-connection-choice =
    .aria-label = 연결
setup-connection-service-title = { -brand } 서비스
setup-connection-service-badge = RPC 불필요
setup-connection-service-detail = 무료 { -brand } 계정을 사용합니다.
setup-connection-service-prices = 가격이 몇 초마다 갱신됩니다
setup-connection-service-features = 카피 트레이딩, 지갑 감시, 다른 지갑의 잔액, 이전 기록, 풀 직접 스왑, 실시간 풀 가격에는 자체 RPC가 필요합니다
setup-connection-service-pause = 로그아웃하거나 screenerbot.io에 연결할 수 없으면 거래가 일시 중지됩니다
setup-connection-service-privacy = 개인 키는 이 컴퓨터를 절대 떠나지 않습니다. 스왑은 여기서 서명되며, screenerbot.io는 지갑의 공개 주소만으로 잔액과 거래를 읽습니다
setup-connection-own-title = 직접 RPC
setup-connection-own-badge = 가장 빠름
setup-connection-own-detail = 가격이 초당 약 두 번 갱신되고 모든 기능을 쓸 수 있습니다. 공급자의 RPC 엔드포인트가 필요합니다.
setup-connection-signin-required = { -brand } 서비스를 사용하려면 로그인하거나 무료 계정을 만드세요.
setup-connection-reauthorize = { -brand } 서비스의 지갑 조회를 허용하려면 다시 로그인하세요.
setup-connection-sending = { -brand }를 통해 트랜잭션 전송
setup-connection-sending-hint = 로그인하면 사용할 수 있습니다. 사용할 수 없을 때는 내 RPC를 사용합니다.
setup-wizard-account-required = 필수
setup-wizard-verify-connection = 연결
setup-verify-service-checking = { -brand } 서비스 확인 중
setup-verify-service-ready = { -brand } 서비스 준비됨
setup-verify-service-busy = 지금은 { -brand } 서비스가 혼잡합니다. 잠시 후 다시 시도하거나 직접 RPC를 사용하세요.
setup-verify-service-daily-limit = 계정이 오늘의 { -brand } 서비스 한도에 도달했습니다. 00:00 UTC 이후 다시 시도하거나 직접 RPC를 사용하세요.
setup-verify-service-unreachable = screenerbot.io에 연결할 수 없습니다. 인터넷 연결을 확인하세요.
setup-verify-service-version = 이 버전은 더 이상 { -brand } 서비스에서 지원되지 않습니다. 계속하려면 { -brand }를 업데이트하세요.
setup-verify-service-signin = { -brand } 서비스를 사용하려면 로그인하세요.
setup-verify-save-done-service-detail = 개인 키를 암호화했습니다. { -brand } 서비스를 선택했습니다.
setup-action-try-again = 다시 시도
setup-action-use-own-rpc = 내 RPC 사용
setup-action-sign-in-again = 다시 로그인
setup-ready-title = { -brand } 준비 완료
setup-fund-title = 거래 지갑에 입금
setup-fund-waiting = { -sol } 입금 대기 중…
setup-fund-received = { $amount } { -sol } 받음
setup-fund-balance-unavailable = 잔액을 읽는 중… 자동으로 갱신됩니다.
setup-fund-open-dashboard = 대시보드 열기
