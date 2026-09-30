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
setup-error-restart-timeout = 설정은 저장되었지만 { -brand }이 아직 다시 연결되지 않았습니다.

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
setup-verify-save-skipped = 저장되지 않음
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
setup-error-gateway-failed = 게이트웨이 설정을 저장하지 못했습니다.
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
setup-dialog-rpc-label = RPC 엔드포인트
setup-dialog-rpc-input =
    .placeholder = https://your-endpoint... (한 줄에 하나씩)
setup-dialog-rpc-hint = 프리미엄 제공자({ -helius }, { -quicknode }, { -alchemy })를 강력히 권장합니다. 공용 Solana RPC는 요청 제한이 있어 작동하지 않을 수 있습니다.
setup-dialog-submit = 검증 및 연결
setup-dialog-working = 처리 중…
setup-dialog-validating = 검증 중…
setup-dialog-saving = 저장 중…
setup-dialog-restarting = 재시작 중…
setup-dialog-saved = 설정이 저장되었습니다. { -brand }을 전체 모드로 재시작하는 중…
setup-dialog-error-missing-fields = 지갑 개인 키와 RPC URL을 하나 이상 입력하세요.
setup-dialog-error-validation = 검증에 실패했습니다.
setup-dialog-error-incomplete = 설정을 완료하지 못했습니다.
setup-dialog-error-restart-helper = 자동 재시작 도우미를 사용할 수 없습니다. 잠시 후 대시보드를 새로 고치세요.
setup-dialog-error-unexpected = 예기치 않은 오류가 발생했습니다.

# Source: templates/pages/setup.html

## Setup wizard

setup-wizard-progress =
    .aria-label = 설정 진행 상황
setup-wizard-step-credentials = 인증 정보
setup-wizard-step-verification = 검증
setup-wizard-step-complete = 완료
setup-wizard-credentials-title = 인증 정보 설정
setup-wizard-credentials-description = 로컬 지갑과 안정적인 Solana 메인넷 RPC 엔드포인트를 연결하세요.
setup-wizard-wallet-toggle =
    .title = 개인 키 표시
    .aria-label = 개인 키 표시
setup-wizard-wallet-security-note = 저장하기 전에 암호화됩니다.
setup-wizard-rpc-title = RPC 엔드포인트
setup-wizard-rpc-input =
    .placeholder = 한 줄에 HTTPS URL 하나
setup-wizard-rpc-guidance = 지속적인 폴링에는 안정적인 메인넷 RPC를 권장합니다.
setup-provider-helius = { -helius }
setup-provider-quicknode = { -quicknode }
setup-provider-alchemy = { -alchemy }
setup-wizard-provider-recommended = 권장
setup-wizard-gateway-title = 무료 트랜잭션 전송
setup-wizard-gateway-hint = 로그인 시 사용할 수 있습니다. RPC는 대체 수단으로 계속 사용할 수 있습니다.
setup-wizard-account-title = { -brand } 계정
setup-wizard-account-optional = 선택 사항
setup-wizard-account-loading = 계정 상태 확인 중…
setup-wizard-verify-title = 검증 및 저장
setup-wizard-verify-list =
    .aria-label = 설정 검증 상태
setup-wizard-verify-wallet = 지갑
setup-wizard-verify-rpc = Solana RPC
setup-wizard-verify-save = 보안 설정
setup-wizard-complete-title = 설정 저장됨
setup-wizard-reconnect = 연결 재시도
setup-wizard-reload = 대시보드 새로 고침
setup-wizard-error-title = 설정 확인이 필요합니다
setup-wizard-explore = 대시보드 둘러보기
setup-wizard-continue = 계속
