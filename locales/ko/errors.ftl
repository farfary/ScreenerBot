# API error messages. Each key names the failed operation; technical causes are
# carried separately in the response `details` and are never part of a message.

errors-with-details = { $message }: { $details }

# Configuration
errors-config-save-failed = 설정을 저장하지 못했습니다

# Authentication
errors-auth-current-password-incorrect = 현재 비밀번호가 올바르지 않습니다
errors-auth-current-password-required = 비밀번호를 변경하려면 현재 비밀번호가 필요합니다
errors-auth-password-too-short = 비밀번호는 4자 이상이어야 합니다
errors-auth-password-too-long = 비밀번호는 128자 이하여야 합니다
errors-auth-hash-failed = 비밀번호를 해시하지 못했습니다
errors-auth-not-enabled = 인증이 활성화되어 있지 않습니다
errors-auth-no-password = 설정된 비밀번호가 없습니다
errors-auth-password-incorrect = 비밀번호가 올바르지 않습니다
errors-auth-required = 인증이 필요합니다. 이 엔드포인트에 접근하려면 로그인하세요.
errors-auth-totp-invalid = 2FA 코드가 올바르지 않거나 만료되었습니다
errors-auth-totp-verify-failed = 2FA 코드를 확인하지 못했습니다
errors-auth-totp-password-required = 2FA를 활성화하기 전에 비밀번호를 설정해야 합니다
errors-auth-totp-uri-failed = TOTP URI를 생성하지 못했습니다
errors-auth-totp-qr-failed = QR 코드를 생성하지 못했습니다
errors-auth-totp-secret-required = 비밀 키가 필요합니다
errors-auth-totp-save-failed = TOTP 설정을 저장하지 못했습니다
errors-auth-totp-code-invalid = 인증 코드가 올바르지 않습니다. 코드를 확인한 후 다시 시도하세요.
errors-auth-totp-code-verify-failed = 코드를 확인하지 못했습니다

# Request security
errors-security-invalid-local-request = 요청은 로컬 대시보드에서 발생해야 합니다
errors-security-invalid-token = 보안 토큰이 올바르지 않습니다
errors-security-token-required = 보안 토큰이 필요합니다. 이 엔드포인트는 { -brand } 내부에서만 접근할 수 있습니다.

# Lockscreen
errors-lockscreen-no-password-set = 설정된 비밀번호가 없습니다
errors-lockscreen-invalid-type = 비밀번호 유형이 올바르지 않습니다. 'pin4', 'pin6', 'text' 중 하나여야 합니다
errors-lockscreen-invalid-format = 비밀번호가 선택한 유형과 일치하지 않습니다
errors-lockscreen-no-current-password = 현재 설정된 비밀번호가 없습니다
errors-lockscreen-password-incorrect = 비밀번호가 올바르지 않습니다
errors-lockscreen-enable-needs-password = 비밀번호를 먼저 설정하지 않으면 잠금 화면을 활성화할 수 없습니다

# Account
errors-account-signin-failed = 로그인에 실패했습니다
errors-account-signin-refused = { $reason }
errors-account-signin-unavailable = 로그인을 사용할 수 없습니다
errors-account-browser-open-failed = 브라우저를 열 수 없습니다. 기본 브라우저를 열고 다시 시도하세요.
errors-account-signup-open-failed = 가입 페이지를 열 수 없습니다. 브라우저에서 screenerbot.io/signup을 여세요.
errors-account-credentials-required = 이메일 주소와 비밀번호를 입력하세요.
errors-account-signout-failed = 로그아웃에 실패했습니다
errors-account-gateway-update-failed = 게이트웨이 설정을 업데이트할 수 없습니다

# Localization
errors-i18n-locale-not-registered = 등록되지 않은 로캘입니다
errors-i18n-catalog-encode-failed = 카탈로그를 인코딩할 수 없습니다

# System
errors-system-paths-init-failed = 애플리케이션 디렉터리를 만들 수 없습니다
errors-system-open-data-failed = 데이터 폴더를 열 수 없습니다
errors-system-url-empty = URL은 비워 둘 수 없습니다
errors-system-open-url-failed = URL을 열 수 없습니다

# Initialization
errors-initialization-required = 이 엔드포인트에 접근하려면 봇 초기화가 필요합니다. 웹 인터페이스에서 초기화 과정을 완료하세요.
errors-initialization-onboarding-update-failed = 온보딩 상태를 업데이트하지 못했습니다
errors-initialization-validation-required = 설정을 저장하기 전에 자격 증명 확인이 필요합니다
errors-initialization-encrypt-failed = 프라이빗 키를 암호화하지 못했습니다

# Dashboard state
errors-ui-state-save-failed = 상태를 저장하지 못했습니다
errors-ui-state-clear-failed = 상태를 초기화하지 못했습니다

# Agent control
errors-agent-config-failed = 에이전트 제어 설정에 실패했습니다
errors-agent-invalid-parameters = 매개변수가 올바르지 않습니다
errors-agent-wallet-key-material = 지갑 키 자료는 에이전트가 접근할 수 없습니다
errors-agent-store-failed = 에이전트 제어 저장소에 실패했습니다
errors-agent-invalid-pairing-request = 페어링 요청이 올바르지 않습니다
errors-agent-pairing-rejected = 페어링 자격 증명이 거부되었습니다
errors-agent-disabled = 에이전트 제어가 비활성화되어 있습니다
errors-agent-approval-not-pending = 승인이 더 이상 대기 중이 아닙니다
errors-agent-approval-not-found = 승인을 찾을 수 없습니다
errors-agent-bridge-task-failed = 에이전트 제어 브리지 작업에 실패했습니다
errors-agent-task-failed = 에이전트 제어 작업에 실패했습니다
errors-agent-pairing-not-found = 해당 ID의 활성 페어링이 없습니다
errors-agent-permissions-update-failed = 권한을 업데이트하지 못했습니다

# Connectivity
errors-connectivity-endpoint-not-found = 엔드포인트를 찾을 수 없거나 모니터링되지 않습니다: '{ $endpoint }'

# Copy trading
errors-copy-task-not-found = 카피 작업을 찾을 수 없습니다
errors-copy-holding-not-found = 이 토큰에 열린 모의 보유 수량이 없습니다
errors-copy-live-confirmation-required = 실거래 카피 트레이딩을 활성화하려면 명시적인 확인이 필요합니다
errors-copy-task-invalid = 카피 작업이 올바르지 않습니다
errors-copy-request-rejected = 카피 트레이딩 요청이 거부되었습니다
errors-copy-task-limit = 활성 카피 작업의 최대 개수에 도달했습니다
errors-copy-watch-rejected = 카피 대상을 감시할 수 없습니다
errors-copy-live-unavailable = 실거래 카피 트레이딩을 사용할 수 없습니다
errors-copy-task-live = 삭제하기 전에 실거래 작업을 일시 중지하세요
errors-copy-task-owns-positions = 카피 작업이 아직 보유 포지션을 관리하고 있습니다
errors-copy-request-failed = 카피 트레이딩 요청에 실패했습니다

# Strategies
errors-strategies-not-found = 전략을 찾을 수 없습니다
errors-strategies-invalid-type = 전략 유형이 올바르지 않습니다. ENTRY 또는 EXIT여야 합니다
errors-strategies-list-failed = 전략을 가져오지 못했습니다
errors-strategies-get-failed = 전략을 가져오지 못했습니다
errors-strategies-serialize-rules-failed = 규칙을 직렬화하지 못했습니다
errors-strategies-invalid-rules-json = 규칙 JSON이 올바르지 않습니다
errors-strategies-already-exists = ID '{ $id }' 전략이 이미 있습니다
errors-strategies-validation-failed = 전략 유효성 검사에 실패했습니다
errors-strategies-create-failed = 전략을 생성하지 못했습니다
errors-strategies-update-failed = 전략을 업데이트하지 못했습니다
errors-strategies-update-enabled-failed = 전략 사용 상태를 업데이트하지 못했습니다
errors-strategies-delete-failed = 전략을 삭제하지 못했습니다
errors-strategies-deploy-failed = 전략을 배포하지 못했습니다
errors-strategies-no-performance = 이 전략의 성과 데이터가 없습니다
errors-strategies-performance-failed = 성과 통계를 가져오지 못했습니다
errors-strategies-schemas-failed = 조건 스키마를 가져오지 못했습니다
errors-strategies-evaluation-failed = 전략 평가에 실패했습니다

# Transactions
errors-transactions-own-wallet-unavailable = 메인 지갑이 설정되어 있지 않습니다
errors-transactions-invalid-subject = 트랜잭션 대상이 올바른 Solana 주소가 아닙니다
errors-transactions-subject-not-watched = 트랜잭션 대상이 감시 중인 지갑이 아닙니다
errors-transactions-watch-store-unavailable = 감시 중인 지갑을 사용할 수 없습니다

# Wallet
errors-wallet-unavailable = 메인 지갑을 사용할 수 없습니다
errors-wallet-changed = 메인 지갑이 변경되었습니다. 새로 고친 후 다시 시도하세요
errors-wallet-qr-failed = 지갑 QR 코드를 생성할 수 없습니다

# Updates
errors-updates-none-available = 다운로드할 수 있는 업데이트가 없습니다
errors-updates-version-changed = 사용 가능한 업데이트가 변경되었습니다. 업데이트를 다시 확인하세요
errors-updates-check-failed = 업데이트 확인에 실패했습니다
errors-updates-download-failed = 업데이트 다운로드를 시작할 수 없습니다
errors-updates-history-unavailable = 릴리스 기록을 사용할 수 없습니다
errors-updates-apply-failed = 업데이트를 적용할 수 없습니다
errors-updates-install-failed = 업데이트 설치 프로그램을 열 수 없습니다

# Telegram
errors-telegram-settings-update-failed = 설정을 업데이트하지 못했습니다
errors-telegram-disabled = { -telegram } 기능이 활성화되어 있지 않습니다
errors-telegram-not-configured = 봇 토큰 또는 채팅 ID가 설정되어 있지 않습니다
errors-telegram-send-failed = 메시지를 전송하지 못했습니다
errors-telegram-notifier-failed = 알림 발송기를 생성하지 못했습니다
errors-telegram-token-required = 봇 토큰을 먼저 설정해야 합니다
errors-telegram-discovery-failed = 탐색을 시작하지 못했습니다
errors-telegram-chat-select-failed = 채팅을 선택하지 못했습니다

# Assistant chat
errors-chat-message-empty = 메시지는 비워 둘 수 없습니다
errors-chat-message-too-long = 메시지가 최대 길이인 10,000자를 초과했습니다
errors-chat-database-unavailable = 채팅 데이터베이스가 초기화되지 않았습니다
errors-chat-session-not-found = 채팅 세션을 찾을 수 없습니다: { $id }
errors-chat-session-validate-failed = 세션을 확인하지 못했습니다
errors-chat-engine-unavailable = 채팅 엔진이 초기화되지 않았습니다
errors-chat-process-failed = 채팅 메시지를 처리하지 못했습니다
errors-chat-stream-serialize-failed = 채팅 이벤트를 직렬화하지 못했습니다
errors-chat-sessions-list-failed = 채팅 세션 목록을 가져오지 못했습니다
errors-chat-session-create-failed = 채팅 세션을 생성하지 못했습니다
errors-chat-session-get-failed = 채팅 세션을 가져오지 못했습니다
errors-chat-messages-get-failed = 채팅 메시지를 가져오지 못했습니다
errors-chat-session-delete-failed = 채팅 세션을 삭제하지 못했습니다
errors-chat-messages-load-failed = 메시지를 가져오지 못했습니다
errors-chat-summarize-empty = 비어 있는 채팅 세션은 요약할 수 없습니다
errors-chat-provider-invalid = 올바르지 않은 제공자: { $provider }
errors-chat-summary-save-failed = 요약을 저장하지 못했습니다
errors-chat-title-empty-session = 비어 있는 채팅 세션은 제목을 생성할 수 없습니다
errors-chat-no-user-message = 세션에 사용자 메시지가 없습니다
errors-chat-title-save-failed = 세션 제목을 업데이트하지 못했습니다
errors-chat-confirmation-save-failed = 확인 응답을 저장하지 못했습니다
errors-chat-confirmation-failed = 확인을 처리하지 못했습니다
errors-chat-summary-failed = 요약을 생성하지 못했습니다

# Assistant automation
errors-automation-database-unavailable = 데이터베이스가 초기화되지 않았습니다
errors-automation-tasks-list-failed = 작업 목록을 가져오지 못했습니다
errors-automation-name-empty = 작업 이름은 비워 둘 수 없습니다
errors-automation-instruction-empty = 작업 지침은 비워 둘 수 없습니다
errors-automation-schedule-type-invalid = schedule_type이 올바르지 않습니다. interval, daily, weekly 중 하나여야 합니다
errors-automation-schedule-value-invalid = schedule_value가 올바르지 않습니다
errors-automation-task-create-failed = 작업을 생성하지 못했습니다
errors-automation-task-not-found = 작업을 찾을 수 없습니다
errors-automation-task-get-failed = 작업을 가져오지 못했습니다
errors-automation-schedule-invalid = 일정이 올바르지 않습니다
errors-automation-tool-permissions-invalid = tool_permissions는 'full' 또는 'readonly'여야 합니다
errors-automation-priority-invalid = priority는 'low', 'medium', 'high' 중 하나여야 합니다
errors-automation-task-update-failed = 작업을 업데이트하지 못했습니다
errors-automation-task-running-delete = 실행 중인 작업은 삭제할 수 없습니다
errors-automation-task-delete-failed = 작업을 삭제하지 못했습니다
errors-automation-task-toggle-failed = 작업을 전환하지 못했습니다
errors-automation-task-disabled = 비활성화된 작업은 실행할 수 없습니다
errors-automation-task-already-running = 작업이 이미 실행 중입니다
errors-automation-runs-list-failed = 실행 목록을 가져오지 못했습니다
errors-automation-recent-runs-failed = 최근 실행 목록을 가져오지 못했습니다
errors-automation-run-not-found = 실행 기록을 찾을 수 없습니다
errors-automation-run-get-failed = 실행 기록을 가져오지 못했습니다
errors-automation-stats-failed = 통계를 가져오지 못했습니다

# LLM providers
errors-llm-config-update-failed = LLM 설정을 업데이트하지 못했습니다
errors-llm-provider-unknown = 알 수 없는 제공자: { $provider }
errors-llm-manager-unavailable = LLM 관리자가 초기화되지 않았습니다
errors-llm-provider-disabled = 제공자 '{ $provider }'가 설정되지 않았거나 비활성화되어 있습니다
errors-llm-provider-config-update-failed = 제공자 설정을 업데이트하지 못했습니다
errors-llm-provider-test-failed = 제공자 테스트에 실패했습니다
errors-llm-provider-refused = { $reason }

# LLM analysis
errors-llm-analysis-config-update-failed = 분석 설정을 업데이트하지 못했습니다
errors-llm-analysis-unavailable = 분석 엔진이 초기화되지 않았습니다
errors-llm-analysis-disabled = LLM 기능이 비활성화되어 있습니다. 먼저 [llm]을 활성화하세요.
errors-llm-analysis-priority-invalid = 우선순위가 올바르지 않습니다: '{ $priority }'. 'high', 'medium', 'low' 중 하나를 사용하세요.
errors-llm-analysis-evaluation-failed = 모델 분석에 실패했습니다
errors-llm-analysis-instructions-list-failed = 지침 목록을 가져오지 못했습니다
errors-llm-analysis-instruction-not-found = 지침을 찾을 수 없습니다: { $id }
errors-llm-analysis-instruction-get-failed = 지침을 가져오지 못했습니다
errors-llm-analysis-instruction-created-retrieve-failed = 생성된 지침을 가져오지 못했습니다
errors-llm-analysis-instruction-create-failed = 지침을 생성하지 못했습니다
errors-llm-analysis-instruction-updated-retrieve-failed = 업데이트된 지침을 가져오지 못했습니다
errors-llm-analysis-instruction-update-failed = 지침을 업데이트하지 못했습니다
errors-llm-analysis-instruction-delete-failed = 지침을 삭제하지 못했습니다
errors-llm-analysis-instructions-reorder-failed = 지침 순서를 변경하지 못했습니다
errors-llm-analysis-decisions-list-failed = 결정 기록 목록을 가져오지 못했습니다
errors-llm-analysis-decision-not-found = 결정을 찾을 수 없습니다: { $id }
errors-llm-analysis-decision-get-failed = 결정을 가져오지 못했습니다

# Wallets
errors-wallets-list-failed = 지갑 목록을 가져오지 못했습니다
errors-wallets-name-empty = 지갑 이름은 비워 둘 수 없습니다
errors-wallets-create-failed = 지갑을 생성하지 못했습니다
errors-wallets-key-empty = 프라이빗 키는 비워 둘 수 없습니다
errors-wallets-already-exists = 지갑이 이미 있습니다
errors-wallets-key-invalid = 프라이빗 키 형식이 올바르지 않습니다
errors-wallets-import-failed = 지갑을 가져오지 못했습니다
errors-wallets-summary-failed = 지갑 요약을 가져오지 못했습니다
errors-wallets-no-main-wallet = 설정된 메인 지갑이 없습니다
errors-wallets-main-get-failed = 메인 지갑을 가져오지 못했습니다
errors-wallets-not-found = 지갑을 찾을 수 없습니다
errors-wallets-get-failed = 지갑을 가져오지 못했습니다
errors-wallets-update-failed = 지갑을 업데이트하지 못했습니다
errors-wallets-delete-failed = 지갑을 삭제하지 못했습니다
errors-wallets-export-failed = 지갑을 내보내지 못했습니다
errors-wallets-set-main-failed = 메인 지갑을 설정하지 못했습니다
errors-wallets-archive-failed = 지갑을 보관하지 못했습니다
errors-wallets-restore-failed = 지갑을 복원하지 못했습니다
errors-wallets-export-format-unsupported = 현재 CSV 형식만 지원됩니다
errors-wallets-export-confirmation-required = 다음 문구를 입력하여 확인해야 합니다: "{ $confirmation }"
errors-wallets-export-no-ids = 지갑 ID가 제공되지 않았습니다
errors-wallets-export-bulk-failed = 지갑을 내보내지 못했습니다
errors-wallets-export-no-match = 제공된 ID와 일치하는 지갑이 없습니다
errors-wallets-import-file-too-large = 파일이 최대 크기 { $megabytes }MB를 초과했습니다
errors-wallets-import-read-failed = 업로드한 파일을 읽지 못했습니다
errors-wallets-import-no-file = 업로드된 파일이 없습니다. 멀티파트 폼의 'file' 필드를 사용하세요
errors-wallets-import-encoding-invalid = CSV 파일은 UTF-8로 인코딩되어야 합니다
errors-wallets-import-csv-parse-failed = CSV 파일을 파싱하지 못했습니다
errors-wallets-import-excel-parse-failed = Excel 파일을 파싱하지 못했습니다
errors-wallets-import-format-unsupported = 지원하지 않는 파일 형식입니다. .csv, .xlsx 또는 .xls를 사용하세요
errors-wallets-import-file-empty = 파일에 데이터 행이 없습니다
errors-wallets-import-existing-check-failed = 기존 지갑을 확인하지 못했습니다
errors-wallets-import-mapping-invalid = 필수 열이 없습니다: { $columns }
errors-wallets-import-session-not-found = 가져오기 세션을 찾을 수 없거나 만료되었습니다. 파일을 다시 업로드하세요
errors-wallets-import-no-valid-rows = 가져올 유효한 행이 없습니다

# Wallet watching
errors-wallet-watch-list-failed = 감시 대상 목록을 가져오지 못했습니다
errors-wallet-watch-address-empty = 주소는 비워 둘 수 없습니다
errors-wallet-watch-add-failed = 감시 대상을 추가하지 못했습니다
errors-wallet-watch-remove-failed = 감시 대상을 제거하지 못했습니다
errors-wallet-watch-update-failed = 감시 대상을 업데이트하지 못했습니다
errors-wallet-watch-budget-failed = 감시 예산을 업데이트할 수 없습니다
errors-wallet-watch-resume-failed = 감시를 재개할 수 없습니다
errors-wallet-watch-approval-failed = { -helius } 승인을 업데이트할 수 없습니다
errors-wallet-watch-status-failed = 감시 상태를 가져오지 못했습니다

# Tools
errors-tools-wallet-failed = 지갑을 가져오지 못했습니다
errors-tools-wallet-address-failed = 지갑 주소를 가져오지 못했습니다
errors-tools-accounts-scan-failed = 계정을 스캔하지 못했습니다
errors-tools-token-accounts-scan-failed = 토큰 계정을 스캔하지 못했습니다
errors-tools-token-accounts-get-failed = 토큰 계정을 가져오지 못했습니다
errors-tools-cleanup-failed = 정리에 실패했습니다
errors-tools-cache-clear-failed = 캐시를 삭제하지 못했습니다
errors-tools-no-tokens = 소각할 토큰이 선택되지 않았습니다
errors-tools-burn-failed = 토큰을 소각하지 못했습니다
errors-tools-favorites-list-failed = 즐겨찾기를 가져오지 못했습니다
errors-tools-favorite-type-invalid = 도구 유형이 올바르지 않습니다. 다음 중 하나여야 합니다: { $types }
errors-tools-favorite-add-failed = 즐겨찾기를 추가하지 못했습니다
errors-tools-favorite-not-found = 즐겨찾기를 찾을 수 없습니다
errors-tools-favorite-update-failed = 즐겨찾기를 업데이트하지 못했습니다
errors-tools-favorite-delete-failed = 즐겨찾기를 삭제하지 못했습니다
errors-tools-favorite-use-failed = 사용 횟수를 업데이트하지 못했습니다
errors-tools-pool-search-failed = 토큰 풀 검색에 실패했습니다: { $mint }
errors-tools-watched-list-failed = 감시 중인 토큰 목록을 가져오지 못했습니다
errors-tools-watched-add-failed = 감시 토큰을 추가하지 못했습니다
errors-tools-watched-delete-failed = 감시 토큰을 삭제하지 못했습니다
errors-tools-mint-invalid = 토큰 민트 주소가 올바르지 않습니다
errors-tools-wallets-get-failed = 지갑을 가져오지 못했습니다
errors-tools-balance-failed = 지갑 잔액을 가져오지 못했습니다
errors-tools-session-active = 다른 멀티 지갑 작업이 이미 진행 중입니다
errors-tools-config-invalid = 도구 설정이 올바르지 않습니다: { $reason }
errors-tools-config-rejected = 도구 설정이 올바르지 않습니다
errors-tools-consolidate-failed = 지갑을 통합하지 못했습니다
errors-tools-ata-cleanup-failed = ATA를 정리하지 못했습니다
errors-tools-routers-unavailable = 스왑 라우터가 아직 준비되지 않았습니다
errors-tools-router-disabled-chain-settings = 설정 > 체인에서 사용 중지됨: { $router }
errors-tools-router-unknown = 알 수 없는 스왑 라우터 '{ $router }'
errors-tools-session-type-mismatch = 세션 유형 불일치: 예상 { $expected }, 실제 { $actual }
errors-tools-session-not-found = 세션을 찾을 수 없습니다
errors-tools-session-complete = 세션이 이미 완료되었습니다

# Configuration import and reload
errors-config-reload-failed = 설정을 다시 불러오지 못했습니다
errors-config-reset-failed = 설정을 초기화하지 못했습니다
errors-config-disk-parse-failed = 디스크 설정을 파싱하지 못했습니다
errors-config-disk-read-failed = 디스크 설정을 읽지 못했습니다
errors-config-update-failed = 설정을 업데이트하지 못했습니다
errors-config-import-not-object = 설정은 JSON 객체여야 합니다
errors-config-import-no-sections = 가져올 유효한 섹션이 없습니다
errors-config-import-validation-failed = 설정 유효성 검사에 실패했습니다. 변경 사항이 적용되지 않았습니다.
errors-config-import-commit-failed = 설정 변경 사항을 커밋하지 못했습니다
errors-config-import-failed = 설정을 가져오지 못했습니다

# Filtering
errors-filtering-analytics-failed = 분석 데이터를 가져오지 못했습니다
errors-filtering-refresh-failed = 필터링 스냅샷을 다시 구성하지 못했습니다
errors-filtering-rejection-stats-failed = 제외 통계를 가져오지 못했습니다
errors-filtering-rejected-tokens-failed = 제외된 토큰을 가져오지 못했습니다
errors-filtering-csv-header-failed = CSV 헤더를 쓰지 못했습니다
errors-filtering-csv-record-failed = CSV 레코드를 쓰지 못했습니다
errors-filtering-csv-finalize-failed = CSV를 마무리하지 못했습니다
errors-filtering-export-response-failed = 응답을 구성하지 못했습니다

# OHLCV
errors-ohlcv-fetch-failed = OHLCV 데이터를 가져오지 못했습니다
errors-ohlcv-pools-failed = 풀을 가져오지 못했습니다
errors-ohlcv-gaps-failed = 공백을 가져오지 못했습니다
errors-ohlcv-refresh-failed = 새로 고치지 못했습니다
errors-ohlcv-monitor-start-failed = 모니터링을 시작하지 못했습니다
errors-ohlcv-monitor-stop-failed = 모니터링을 중지하지 못했습니다
errors-ohlcv-activity-failed = 활동을 기록하지 못했습니다
errors-ohlcv-list-failed = OHLCV 토큰 목록을 가져오지 못했습니다
errors-ohlcv-delete-failed = 토큰 데이터를 삭제하지 못했습니다
errors-ohlcv-clear-failed = OHLCV 캐시를 삭제하지 못했습니다
errors-ohlcv-cleanup-failed = 비활성 토큰을 정리하지 못했습니다

# Trader and manual trading
errors-trade-already-running = 트레이더가 이미 실행 중입니다
errors-trade-already-stopped = 트레이더가 이미 중지되어 있습니다
errors-trade-config-update-failed = 트레이더 설정 업데이트에 실패했습니다
errors-trade-trader-unavailable = 자동 트레이더를 사용하기 전에 지갑과 RPC 설정을 완료하세요
errors-trade-force-stop-active = 긴급 중지가 활성화되어 있습니다. 먼저 해제하세요
errors-trade-template-not-found = 트레이더 템플릿을 찾을 수 없습니다: { $template }
errors-trade-manual-force-stopped = 긴급 중지가 활성화된 동안에는 수동 거래를 사용할 수 없습니다
errors-trade-core-services-not-ready = 거래에 필요한 핵심 서비스가 준비되지 않았습니다: { $pending }
errors-trade-mint-invalid = 토큰 민트 주소가 올바르지 않습니다: { $mint }
errors-trade-blacklisted = 블랙리스트에 등록된 토큰입니다: { $mint }
errors-trade-slippage-invalid = 슬리피지 { $slippage }%는 (0, { $maximum }] 범위여야 합니다
errors-trade-percentage-invalid = 매도 비율({ $percentage })은 (0, 100] 범위여야 합니다
errors-trade-record-failed = 수동 거래를 기록할 수 없습니다
errors-trade-task-cancelled = 앱이 종료되는 중이어서 수동 거래가 응답 전에 중단되었습니다. 결과는 포지션에 기록되어 있습니다
errors-trade-no-open-position = 보유 포지션이 없는 토큰입니다: { $mint }
errors-trade-size-invalid = 거래 규모가 올바르지 않습니다: { $amount } { -sol }
errors-trade-management-invalid = 포지션 관리 값이 올바르지 않습니다: { $management }
errors-trade-strategy-evaluation-failed = 토큰 전략 평가에 실패했습니다: { $mint }
errors-trade-token-data-missing = 토큰 데이터를 사용할 수 없습니다: { $mint }
errors-trade-endpoints-unhealthy = 정상 상태의 엔드포인트가 없습니다
errors-trade-dependency-failed = 의존 항목 실패: { $dependency }
errors-trade-storage-failed = 거래 요청을 완료할 수 없습니다
errors-trade-manual-failed = 수동 거래에 실패했습니다
errors-trade-manual-refused = { $reason }
errors-trade-swap-too-large = 전송할 수 있을 만큼 작은 트랜잭션을 만들 수 있는 스왑 경로가 없습니다
    .hint = 최적 경로에 트랜잭션 하나가 담을 수 있는 것보다 많은 계정이 필요해 아무것도 전송되지 않았고 비용도 발생하지 않았습니다. 잠시 후 다른 경로로 다시 시도하거나 다른 스왑 라우터를 활성화하세요.
errors-trade-wallet-not-configured = 지갑이 설정되지 않았습니다
errors-trade-amount-sol-invalid = 매수에는 amount_sol이 필요하며 양수여야 합니다
errors-trade-no-tokens-in-wallet = 이 포지션의 토큰이 지갑에 없습니다. 토큰 잔액이 0이므로 스왑으로 포지션을 종료할 수 없습니다.
errors-trade-percentage-range = percentage는 (0, 100] 범위여야 합니다
errors-trade-amount-tokens-invalid = amount_tokens는 양수여야 합니다
errors-trade-sell-amount-zero = 계산된 매도 수량이 0입니다

# Swap quotes. The message is the dialog headline; `.hint` is what the user can do.
errors-trade-quote-registry-unavailable = 스왑 라우팅이 아직 준비되지 않았습니다
    .hint = 스왑 서비스가 아직 시작 중입니다. 서비스가 준비될 때까지 기다린 후 다시 시도하세요.
errors-trade-quote-no-routers-enabled = 활성화된 스왑 제공자가 없습니다
    .hint = 트레이더 설정에서 스왑 라우터를 하나 이상 활성화한 후 다시 시도하세요.
errors-trade-quote-not-tradable = 이 토큰은 현재 거래할 수 없습니다
    .hint = 사용 가능한 유동성이나 스왑 경로가 없습니다. 출시 전이거나, 방치되었거나, 풀이 없는 토큰일 수 있습니다. 나중에 다시 시도하거나 다른 토큰을 선택하세요.
errors-trade-quote-no-route = 사용 가능한 스왑 경로가 없습니다
    .hint = 요청한 수량으로 이 거래를 라우팅할 수 있는 제공자가 없습니다. 수량을 줄이거나 잠시 후 다시 시도하세요.
errors-trade-quote-rate-limited = 스왑 제공자가 요청을 제한하고 있습니다
    .hint = 스왑 제공자가 요청을 제한하고 있습니다. 몇 초 기다린 후 다시 시도하세요.
errors-trade-quote-timeout = 견적 요청 시간이 초과되었습니다
    .hint = 스왑 제공자가 제시간에 응답하지 않았습니다. 연결을 확인하고 다시 시도하세요.
errors-trade-quote-router-rejected = 견적이 거부되었습니다
    .hint = 제공자가 반환한 견적이 안전 검사를 통과하지 못해 폐기되었습니다. 다시 시도하여 새 견적을 가져오세요.
errors-trade-quote-not-offered-exact-out = 정확한 출력 수량으로 견적을 내는 활성 스왑 라우터가 없습니다
    .hint = 활성화된 스왑 라우터는 지불 금액을 기준으로만 거래 견적을 냅니다. 지불할 금액을 입력하거나 다른 스왑 라우터를 활성화하세요.
errors-trade-quote-not-offered-unsupported-venue = 이 토큰의 풀에서 거래하는 활성 스왑 라우터가 없습니다
    .hint = 이 토큰은 활성화된 스왑 라우터가 아직 지원하지 않는 거래소에서 거래됩니다. 다른 스왑 라우터를 활성화한 뒤 다시 시도하세요.
errors-trade-quote-unavailable = 견적을 가져올 수 없습니다
    .hint = 스왑 제공자가 이 거래의 견적을 제시하지 못했습니다. 잠시 후 다시 시도하세요.

# Positions
errors-positions-not-found = 포지션을 찾을 수 없습니다
errors-positions-already-closed = 이미 종료된 포지션입니다
errors-positions-force-close-failed = 포지션을 강제 종료하지 못했습니다
errors-positions-already-archived = 이미 보관된 포지션입니다
errors-positions-unverified-entry-archive = 이 포지션의 매수가 아직 확인되지 않았습니다. 확인된 후 보관하세요.
errors-positions-not-archived = 보관되지 않은 포지션입니다
errors-positions-archive-failed = 포지션을 보관하지 못했습니다
errors-positions-unarchive-failed = 포지션 보관을 해제하지 못했습니다
errors-positions-management-invalid = 카피 관리는 카피에서 시작된 포지션에만 사용할 수 있습니다
errors-positions-management-failed = 포지션 관리를 업데이트하지 못했습니다
errors-positions-delete-failed = 포지션을 삭제하지 못했습니다
errors-positions-bulk-delete-failed = 보관된 포지션을 삭제하지 못했습니다
errors-positions-detail-failed = 포지션 상세 정보를 불러오지 못했습니다
errors-positions-resolve-failed = 포지션을 확인하지 못했습니다
errors-positions-wrapped-sol-activity = 래핑된 SOL에는 토큰 활동이 없습니다

# Tokens
errors-tokens-database-unavailable = 토큰 데이터베이스를 사용할 수 없습니다
errors-tokens-blacklist-failed = 토큰을 블랙리스트에 추가하지 못했습니다
errors-tokens-blacklist-internal = 블랙리스트 작업 중 내부 오류가 발생했습니다
errors-tokens-unblacklist-failed = 블랙리스트에서 제거하지 못했습니다
errors-tokens-unblacklist-internal = 블랙리스트 해제 작업 중 내부 오류가 발생했습니다
errors-tokens-blacklist-status-failed = 블랙리스트 상태를 확인하지 못했습니다
errors-tokens-blacklist-status-internal = 블랙리스트 상태 확인 중 내부 오류가 발생했습니다
errors-tokens-favorites-fetch-failed = 즐겨찾기를 가져오지 못했습니다
errors-tokens-favorite-add-failed = 즐겨찾기를 추가하지 못했습니다
errors-tokens-favorite-remove-failed = 즐겨찾기를 제거하지 못했습니다
errors-tokens-favorite-update-failed = 즐겨찾기를 업데이트하지 못했습니다
errors-tokens-detail-not-found = 데이터베이스 또는 외부 소스에서 토큰을 찾을 수 없습니다
errors-tokens-fetch-failed = 토큰을 가져오지 못했습니다
errors-tokens-refresh-all-failed = 모든 데이터 소스가 실패했습니다
errors-tokens-refresh-failed = 토큰을 새로 고치지 못했습니다
errors-tokens-search-query-required = 검색어 'q'가 필요합니다
errors-tokens-search-failed = 토큰 검색에 실패했습니다

# Actions and services
errors-actions-not-found = 작업을 찾을 수 없습니다: { $id }
errors-services-not-found = 서비스를 찾을 수 없습니다: '{ $name }'
