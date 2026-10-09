## Shared

settings-duration-minutes =
    { $count ->
       *[other] { $count }분
    }
settings-duration-hours =
    { $count ->
       *[other] { $count }시간
    }

## settings_dialog.js

settings-dialog-title = 설정
settings-dialog-close =
    .title = 닫기 (ESC)
    .aria-label = 설정 닫기
settings-dialog-save = 변경 사항 저장
settings-dialog-saving = 저장 중...
settings-dialog-saved = 저장됨
settings-dialog-save-success = 설정이 저장되었습니다
settings-dialog-save-failed = 설정을 저장하지 못했습니다
settings-dialog-update-attention = 업데이트 확인 필요
settings-dialog-tab-interface = 인터페이스
settings-dialog-tab-navigation = 내비게이션
settings-dialog-tab-startup = 시작
settings-dialog-tab-hints = 힌트
settings-dialog-tab-data = 데이터
settings-dialog-tab-security = 보안
settings-dialog-tab-account = 계정
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = 에이전트 연결
settings-dialog-tab-updates = 업데이트
settings-dialog-tab-licenses = 라이선스
settings-dialog-tab-about = 정보
settings-dialog-link-privacy = 개인정보 처리방침
settings-dialog-link-terms = 이용약관

## settings_dialog.js: Startup tab

settings-startup-section-title = 시작 동작
settings-startup-auto-start-label = 트레이더 자동 시작
settings-startup-auto-start-hint = 실행 시 트레이더를 자동으로 시작합니다
settings-startup-coming-soon = 출시 예정
settings-startup-default-page-label = 기본 페이지
settings-startup-default-page-hint = 앱을 열 때 표시할 페이지
settings-startup-page-dashboard = 대시보드
settings-startup-page-tokens = 토큰
settings-startup-page-positions = 포지션
settings-startup-page-wallet = 지갑
settings-startup-page-config = 설정
settings-startup-notifications-label = 백그라운드 알림 표시
settings-startup-notifications-hint = 백그라운드 이벤트 알림을 표시합니다

## settings_dialog.js: About tab

settings-about-tagline = 네이티브 솔라나 트레이딩 엔진
settings-about-link-github = { -github }
settings-about-link-docs = 문서
settings-about-link-telegram = { -telegram }
settings-about-link-website = 웹사이트
settings-about-credits = 솔라나 트레이더를 위해 제작되었습니다
settings-about-copyright = © { $year } { -brand }. All rights reserved.

## interface_tab.js

settings-interface-section-appearance = 외관
settings-interface-theme-label = 테마
settings-interface-theme-hint = 선호하는 색상 테마를 선택하세요
settings-interface-theme-dark = 다크
settings-interface-theme-light = 라이트
settings-interface-language-label = 언어
settings-interface-language-hint = 대시보드 표시 언어
settings-interface-logo-shape-label = 토큰 로고 모양
settings-interface-logo-shape-hint = 원형은 모든 로고를 원으로 자르고, 원본은 각 로고의 고유한 윤곽을 유지합니다
settings-interface-logo-shape-circle = 원형
settings-interface-logo-shape-natural = 원본
settings-interface-animations-label = 애니메이션 사용
settings-interface-animations-hint = 부드러운 전환과 효과
settings-interface-compact-label = 간결 모드
settings-interface-compact-hint = 여백을 줄여 더 많은 내용을 표시합니다
settings-interface-section-data = 데이터 및 표시
settings-interface-refresh-label = 새로고침 주기
settings-interface-refresh-hint = 데이터를 새로고침하는 주기
settings-interface-refresh-seconds =
    { $count ->
       *[other] { $count }초
    }
settings-interface-refresh-minutes =
    { $count ->
       *[other] { $count }분
    }
settings-interface-ticker-label = 티커 바 표시
settings-interface-ticker-hint = 헤더에 실시간 지표 티커를 표시합니다
settings-interface-page-size-label = 표 페이지 크기
settings-interface-page-size-hint = 표 페이지당 기본 행 수
settings-interface-page-size-rows =
    { $count ->
       *[other] { $count }행
    }
settings-interface-auto-expand-label = 카테고리 자동 펼치기
settings-interface-auto-expand-hint = 설정 카테고리를 기본으로 펼칩니다
settings-interface-hints-label = 상황별 힌트 표시
settings-interface-hints-hint = 대시보드 기능을 설명하는 도움말 아이콘을 표시합니다
settings-interface-featured-label = 추천 행 표시
settings-interface-featured-hint = 홈과 토큰 페이지에 추천 토큰 행을 표시합니다
settings-interface-section-sound = 효과음
settings-interface-sounds-label = 소리 사용
settings-interface-sounds-hint = 탐색, 상태 변경, 결과에 대한 소리 피드백

## security_tab.js

settings-security-loading = 보안 설정 불러오는 중...
settings-security-load-failed = 보안 설정을 불러오지 못했습니다

settings-security-type-pin4 = 4자리 PIN
settings-security-type-pin6 = 6자리 PIN
settings-security-type-text = 텍스트 비밀번호
settings-security-type-unset = 설정 안 됨

settings-security-lockscreen-title = 대시보드 잠금 화면
settings-security-lockscreen-description = PIN 또는 비밀번호로 대시보드를 보호합니다. 잠금 화면이 나타나면 계속하려면 인증이 필요합니다.
settings-security-enable-label = 잠금 화면 사용
settings-security-enable-hint = 비밀번호 인증으로 대시보드를 보호합니다
settings-security-password-status-label = 비밀번호 상태
settings-security-password-current = 현재: { $type }
settings-security-password-none = 설정된 비밀번호 없음
settings-security-change = 변경
settings-security-remove = 제거
settings-security-set-password = 비밀번호 설정
settings-security-auto-lock-label = 비활동 시 자동 잠금
settings-security-auto-lock-hint = 일정 시간 동안 활동이 없으면 자동으로 잠급니다
settings-security-auto-lock-never = 안 함
settings-security-lock-blur-label = 창 포커스를 잃으면 잠금
settings-security-lock-blur-hint = 다른 애플리케이션으로 전환하면 자동으로 잠급니다
settings-security-quick-actions-title = 빠른 작업
settings-security-lock-now-label = 대시보드 지금 잠금
settings-security-lock-now-hint = 대시보드를 즉시 잠급니다
settings-security-lock-now = 지금 잠금
settings-security-lock-not-ready = 잠글 수 없습니다. 잠금 화면이 준비되지 않았습니다
settings-security-setting-save-failed = 보안 설정을 저장하지 못했습니다

## security_tab.js: two-factor authentication

settings-security-2fa-title = 2단계 인증
settings-security-2fa-description = 인증 앱(Google Authenticator, Authy 등)으로 보안 계층을 추가합니다
settings-security-2fa-status-label = 2FA 상태
settings-security-2fa-status-enabled = 2단계 인증이 사용 중입니다
settings-security-2fa-status-none = 설정되지 않음
settings-security-2fa-disable = 2FA 사용 안 함
settings-security-2fa-enable = 2FA 사용

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = 닫기
settings-security-password-set-title = 비밀번호 설정
settings-security-password-change-title = 비밀번호 변경
settings-security-password-current-label = 현재 비밀번호
settings-security-password-current-input =
    .placeholder = 현재 비밀번호 입력
settings-security-password-type-label = 비밀번호 유형
settings-security-password-new-label = 새 비밀번호
settings-security-password-new-input =
    .placeholder = 새 비밀번호 입력
settings-security-password-confirm-label = 비밀번호 확인
settings-security-password-confirm-input =
    .placeholder = 비밀번호 확인
settings-security-password-update = 비밀번호 업데이트
settings-security-placeholder-pin4 = 4자리 PIN 입력
settings-security-placeholder-pin6 = 6자리 PIN 입력
settings-security-placeholder-text = 비밀번호 입력
settings-security-password-required = 비밀번호를 입력하세요
settings-security-password-mismatch = 비밀번호가 일치하지 않습니다
settings-security-pin4-invalid = PIN은 정확히 4자리여야 합니다
settings-security-pin6-invalid = PIN은 정확히 6자리여야 합니다
settings-security-text-too-short = 비밀번호는 4자 이상이어야 합니다
settings-security-password-saved = 비밀번호가 저장되었습니다
settings-security-password-save-failed = 비밀번호를 저장하지 못했습니다
settings-security-password-save-failed-detail = 비밀번호를 저장하지 못했습니다: { $message }

## security_tab.js: remove password dialog

settings-security-remove-title = 비밀번호 제거
settings-security-remove-description = 잠금 화면 보호를 해제하려면 현재 비밀번호를 입력하세요.
settings-security-remove-confirm = 비밀번호 제거
settings-security-current-required = 현재 비밀번호를 입력하세요
settings-security-password-removed = 비밀번호가 제거되었습니다
settings-security-password-remove-failed = 비밀번호를 제거하지 못했습니다
settings-security-password-remove-failed-detail = 비밀번호를 제거하지 못했습니다: { $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = 2단계 인증 사용
settings-security-2fa-password-prompt = 계속하려면 비밀번호를 입력하세요:
settings-security-2fa-password-input =
    .placeholder = 비밀번호 입력
settings-security-2fa-continue = 계속
settings-security-2fa-manual-code = 수동 입력 코드:
settings-security-2fa-qr =
    .alt = TOTP QR 코드
settings-security-2fa-code-prompt = 인증 앱의 6자리 코드를 입력하세요:
settings-security-2fa-verify-enable = 확인 및 사용
settings-security-2fa-password-required = 비밀번호를 입력하세요
settings-security-2fa-setup-failed = 2FA를 설정하지 못했습니다
settings-security-2fa-code-invalid-length = 6자리 코드를 입력하세요
settings-security-2fa-code-invalid = 코드가 올바르지 않습니다
settings-security-2fa-enabled = 2단계 인증이 사용 설정되었습니다
settings-security-2fa-verify-failed = 코드를 확인하지 못했습니다
settings-security-2fa-disable-title = 2단계 인증 사용 안 함
settings-security-2fa-disable-prompt = 2FA를 해제하려면 비밀번호를 입력하세요:
settings-security-2fa-disable-failed = 2FA를 해제하지 못했습니다
settings-security-2fa-disabled = 2단계 인증이 해제되었습니다

## agent_connections_tab.js

settings-agent-category-analysis = 분석
settings-agent-category-portfolio = 포트폴리오
settings-agent-category-trading = 거래
settings-agent-category-config = 설정
settings-agent-category-system = 시스템
settings-agent-category-analysis-description = 토큰 분석, 시장 데이터, 보안 점검.
settings-agent-category-portfolio-description = 보유 포지션, 잔액, 손익.
settings-agent-category-trading-description = 실제 자금으로 포지션 매수, 매도, 종료.
settings-agent-category-config-description = RPC 엔드포인트를 포함한 모든 봇 설정. 지갑 키는 제외됩니다.
settings-agent-category-system-description = 상태, 이벤트, 긴급 중지.
settings-agent-category-analysis-inline = 분석
settings-agent-category-portfolio-inline = 포트폴리오
settings-agent-category-trading-inline = 거래
settings-agent-category-config-inline = 설정
settings-agent-category-system-inline = 시스템

settings-agent-level-allow = 허용
settings-agent-level-ask-user = 확인
settings-agent-level-deny = 끔
settings-agent-level-allow-hint = 즉시 실행됩니다.
settings-agent-level-ask-user-hint = 앱에서 승인할 때까지 대기합니다.
settings-agent-level-deny-hint = 거부되며 에이전트에게 표시되지 않습니다.

settings-agent-preset-full = 전체 권한
settings-agent-preset-ask = 먼저 확인
settings-agent-preset-read = 읽기 전용
settings-agent-preset-full-description = 모든 작업이 확인 없이 실행됩니다. 지갑 키에는 접근할 수 없습니다.
settings-agent-preset-ask-description = 모든 작업이 앱에서 승인할 때까지 대기합니다.
settings-agent-preset-read-description = 분석 및 포트폴리오 조회만 가능합니다. 변경할 수 없습니다.
settings-agent-preset-custom = 사용자 지정
settings-agent-preset-group =
    .aria-label = 권한 프리셋
settings-agent-permission-group = 권한: { $category }

settings-agent-summary-asks-only = 제한됨 — 승인 필요: { $asking }
settings-agent-summary-off-only = 제한됨 — 사용 불가: { $off }
settings-agent-summary-asks-and-off = 제한됨 — 승인 필요: { $asking } / 사용 불가: { $off }

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = 범용 stdio MCP

settings-agent-note-placeholder = /absolute/path/to/screenerbot를 { -brand } 바이너리의 절대 경로로 바꾸세요. 실행 중인 앱이 이 시스템에서 실행 파일 경로를 표현할 수 없었습니다.
settings-agent-note-data-dir = { -brand } 앱을 기본이 아닌 데이터 디렉터리로 실행하는 경우, 클라이언트에도 SCREENERBOT_DATA_DIR를 같은 경로로 설정하세요 (-e / --env 플래그 추가 또는 env 항목).
settings-agent-note-codex-run = 명령을 실행하거나 TOML 블록을 ~/.codex/config.toml ($CODEX_HOME/config.toml)에 추가하세요. 이후 { -codex } 재시작이 필요합니다.
settings-agent-note-codex-get = `codex mcp get screenerbot`는 출력에서 시크릿을 가립니다.
settings-agent-note-claude-code = { -claude } Code: 명령을 실행한 뒤 { -claude } Code 재시작이 필요합니다. `claude mcp get screenerbot`는 시크릿을 포함한 설정된 환경을 출력합니다.
settings-agent-note-claude-desktop = { -claude } Desktop: JSON을 claude_desktop_config.json의 `mcpServers`에 병합한 뒤 앱을 재시작하세요.
settings-agent-note-openclaw = 명령을 실행한 뒤 `openclaw mcp doctor screenerbot --probe`로 저장된 stdio 서버가 시작되고 도구를 제공하는지 확인하세요.
settings-agent-note-hermes = { -hermes } 설정 파일의 `mcp_servers` 아래에 다음을 추가한 뒤 다시 시작하세요.
settings-agent-note-generic = stdio를 지원하는 모든 MCP 클라이언트: 클라이언트가 서버 목록을 보관하는 위치에 이 명령을 인수와 환경 변수와 함께 실행하도록 등록하세요.
settings-agent-block-codex-command = { -codex } CLI — 터미널 명령
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml (대체 방법)
settings-agent-block-claude-command = { -claude } Code — 터미널 명령
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — 터미널 명령
settings-agent-block-hermes = { -hermes } — mcp_servers (YAML)
settings-agent-block-generic = 범용 stdio MCP 클라이언트

settings-agent-name-required = 이 연결의 이름을 입력하세요.
settings-agent-name-too-long = 이름은 { $max }자 이하여야 합니다.
settings-agent-name-control-characters = 이름에 제어 문자를 포함할 수 없습니다.

settings-agent-title = 에이전트 연결
settings-agent-description = { -claude }, { -codex }, { -hermes }, { -openclaw } 또는 모든 stdio MCP 클라이언트를 연결합니다. { -brand } 앱이 계속 실행 중이어야 합니다. 각 연결은 자체 권한을 가지며 기본은 전체 권한이고, 언제든 연결별로 제한할 수 있습니다. 어떤 연결도 지갑 키를 읽거나 변경할 수 없습니다.
settings-agent-name-label = 연결 이름
settings-agent-name-hint = 아래 목록에서 연결을 구분하는 데 사용됩니다.
settings-agent-name-input =
    .placeholder = 노트북 코딩 에이전트
settings-agent-client-label = 클라이언트
settings-agent-client-hint = 연결을 만든 뒤 표시될 설정 안내를 선택합니다.
settings-agent-permissions-label = 권한
settings-agent-permissions-hint = 새 연결은 모든 작업을 수행할 수 있습니다. 지금 또는 아래 목록에서 나중에 카테고리별로 제한할 수 있으며, 어느 경우에도 지갑 키에는 접근할 수 없습니다.
settings-agent-create = 연결 만들기
settings-agent-issued-group =
    .aria-label = 새 연결 자격 증명
settings-agent-issued-warning = 지금 시크릿을 복사하세요. 한 번만 표시되며 다시 확인할 수 없습니다. 분실하면 연결을 해지하고 다시 만드세요. { -brand } 앱은 단방향 검증값만 보관하며, MCP 클라이언트가 자체 설정에 원문을 저장합니다.
settings-agent-issued-client-id = 클라이언트 ID
settings-agent-issued-secret = 일회용 시크릿
settings-agent-setup-for = 설정 대상
settings-agent-done = 완료
settings-agent-list-title = 연결
settings-agent-loading = 연결 불러오는 중...
settings-agent-active-count = 활성 { $count }개
settings-agent-empty = 아직 연결이 없습니다. 위에서 만들어 클라이언트를 연결하세요.
settings-agent-empty-active = 활성 연결이 없습니다.
settings-agent-revoked-title = 해지된 연결
settings-agent-created = 생성: { $time }
settings-agent-last-used = 마지막 사용: { $time }
settings-agent-never-used = 사용 기록 없음
settings-agent-permissions-edit = 권한
settings-agent-revoke = 해지
settings-agent-permissions-save = 권한 저장

settings-agent-load-failed = 에이전트 연결을 불러오지 못했습니다
settings-agent-list-failed = 연결을 불러오지 못했습니다
settings-agent-create-failed = 연결을 만들지 못했습니다.
settings-agent-unreachable-create = 연결을 만들기 위해 { -brand } 앱에 접속하지 못했습니다.
settings-agent-permissions-update-failed = 권한을 업데이트하지 못했습니다
settings-agent-permissions-updated = 권한이 업데이트되었습니다
settings-agent-permissions-updated-detail = 연결의 다음 요청부터 적용됩니다.
settings-agent-unreachable-save = 저장하기 위해 { -brand } 앱에 접속하지 못했습니다
settings-agent-revoke-title = 연결 해지
settings-agent-revoke-message = "{ $label }" 연결을 해지하시겠습니까? 클라이언트는 다음 요청부터 작동하지 않으며 복원할 수 없습니다.
settings-agent-revoke-fallback-name = 이 연결
settings-agent-revoke-failed = 연결을 해지하지 못했습니다
settings-agent-unreachable-revoke = 해지하기 위해 { -brand } 앱에 접속하지 못했습니다

## telegram_tab.js

settings-telegram-loading = { -telegram } 설정 불러오는 중...
settings-telegram-load-failed = { -telegram } 설정을 불러오지 못했습니다
settings-telegram-unknown = 알 수 없음
settings-telegram-session-active = 활성: { $duration }
settings-telegram-sessions-empty = 활성 세션 없음
settings-telegram-session-revoke = 해지

settings-telegram-connection-title = 연결
settings-telegram-connection-description = { -telegram } 봇을 연결하면 알림을 받고 { -brand } 앱을 원격으로 제어할 수 있습니다.
settings-telegram-enable-label = { -telegram } 사용
settings-telegram-enable-hint = { -telegram } 봇 연동을 사용합니다
settings-telegram-token-label = 봇 토큰
settings-telegram-token-saved = 토큰 저장됨
settings-telegram-token-help = { -telegram }의 @BotFather에서 발급받으세요
settings-telegram-token-input-saved =
    .placeholder = 토큰 저장됨 (변경하려면 새로 입력)
settings-telegram-token-input =
    .placeholder = 봇 토큰 입력
settings-telegram-token-toggle =
    .title = 표시/숨기기
settings-telegram-chat-label = 채팅 ID
settings-telegram-chat-connected = 연결된 채팅:
settings-telegram-chat-discover-hint = 채팅 ID를 자동으로 찾습니다
settings-telegram-chat-change =
    .title = 변경
settings-telegram-chat-discover = 채팅 ID 찾기
settings-telegram-discovery-step-add = 봇을 { -telegram } 그룹에 추가하거나 봇과 1:1 채팅을 시작하세요
settings-telegram-discovery-step-privacy = 그룹의 경우: @BotFather → /mybots → [내 봇] → Bot Settings → Group Privacy를 확인하세요
settings-telegram-discovery-privacy = <strong>개인정보 보호 모드 OFF:</strong> 봇이 모든 그룹 메시지를 수신합니다<br/><strong>개인정보 보호 모드 ON:</strong> 봇이 @멘션된 메시지만 수신합니다
settings-telegram-discovery-step-send = 아무 메시지나 보내세요 (개인정보 보호 모드가 ON이면 봇을 @멘션하세요)
settings-telegram-discovery-listening = 메시지 수신 대기 중...
settings-telegram-discovery-select = 선택
settings-telegram-chat-id-label = ID:
settings-telegram-language-label = 메시지 언어
settings-telegram-language-hint = { -telegram } 봇 메시지와 버튼의 언어
settings-telegram-language-follow-app = 앱 언어 따르기
settings-telegram-test-label = 연결 테스트
settings-telegram-test-hint = 테스트 메시지를 보내 설정을 확인합니다
settings-telegram-test-send = 테스트 전송
settings-telegram-test-sending = 전송 중...

settings-telegram-chat-type-private = 개인
settings-telegram-chat-type-group = 그룹
settings-telegram-chat-type-supergroup = 슈퍼그룹
settings-telegram-chat-type-channel = 채널

settings-telegram-auth-title = 명령 인증
settings-telegram-auth-description = { -telegram } 명령은 대시보드 잠금 화면과 같은 2FA를 사용합니다.
settings-telegram-auth-protected = 보호됨
settings-telegram-auth-disabled = 사용 안 함
settings-telegram-auth-not-configured = 설정되지 않음
settings-telegram-auth-error = 오류
settings-telegram-auth-protected-note = 명령은 잠금 화면 2FA로 보호됩니다. 세션이 만료되면 사용자는 <code>/login</code> 명령으로 인증 앱 코드를 입력해야 합니다.
settings-telegram-auth-disabled-note = 잠금 화면 2FA는 설정되어 있지만 { -telegram }에는 사용 안 함 상태입니다. { -telegram } 명령을 보호하려면 위의 "명령에 2FA 요구"를 사용하세요.
settings-telegram-auth-missing-note = 잠금 화면 2FA가 설정되지 않았습니다. 2FA가 없으면 만료된 세션이 확인 없이 자동으로 다시 활성화됩니다.
settings-telegram-auth-managed-in = 2FA 관리 위치:
settings-telegram-auth-configure-in = 2FA 설정 위치:
settings-telegram-auth-configure-suffix = 여기에서 설정하면 { -telegram } 명령에 인증을 요구할 수 있습니다.
settings-telegram-security-link = 보안 설정
settings-telegram-timeout-title = 세션 제한 시간
settings-telegram-timeout-description = 인증된 세션이 활성 상태로 유지되는 시간
settings-telegram-sessions-title = 활성 세션

settings-telegram-notifications-title = 알림 설정
settings-telegram-notifications-description = { -telegram } 알림을 보낼 이벤트를 선택하세요.
settings-telegram-notify-opened-label = 포지션 진입
settings-telegram-notify-opened-hint = 새 포지션이 열리면 알립니다
settings-telegram-notify-closed-label = 포지션 종료
settings-telegram-notify-closed-hint = 포지션이 종료되면 알립니다
settings-telegram-notify-partial-label = 부분 청산
settings-telegram-notify-partial-hint = 포지션 부분 청산 시 알립니다
settings-telegram-notify-dca-label = DCA 실행
settings-telegram-notify-dca-hint = DCA 주문이 실행되면 알립니다
settings-telegram-notify-errors-label = 오류
settings-telegram-notify-errors-hint = 오류와 실패 시 알립니다
settings-telegram-notify-startup-label = 시작/종료
settings-telegram-notify-startup-hint = 봇이 시작되거나 중지되면 알립니다
settings-telegram-notify-filtering-label = 필터링 알림
settings-telegram-notify-filtering-hint = 새 토큰이 필터링 조건을 통과하면 알립니다
settings-telegram-notify-trades-label = 거래 알림
settings-telegram-notify-trades-hint = 관심 토큰의 주요 거래 시 알립니다
settings-telegram-notify-daily-label = 일일 요약
settings-telegram-notify-daily-hint = 일일 거래 활동과 손익 요약을 받습니다

settings-telegram-features-title = 기능
settings-telegram-features-description = { -telegram } 봇 기능을 설정합니다.
settings-telegram-commands-label = 명령 사용
settings-telegram-commands-hint = { -telegram } 명령으로 봇을 제어할 수 있습니다
settings-telegram-require-2fa-label = 명령에 2FA 요구
settings-telegram-require-2fa-hint = 세션이 만료되면 다시 활성화하기 위해 2FA 코드를 요구합니다. 잠금 화면 2FA를 사용합니다.
settings-telegram-inline-label = 인라인 동작 버튼
settings-telegram-inline-hint = 알림 메시지에 동작 버튼을 표시합니다

settings-telegram-setting-save-failed = { -telegram } 설정을 저장하지 못했습니다
settings-telegram-discovery-start-failed = 검색을 시작하지 못했습니다
settings-telegram-chat-selected = 채팅이 선택되었습니다
settings-telegram-chat-select-failed = 채팅을 선택하지 못했습니다
settings-telegram-test-sent = 테스트 메시지를 보냈습니다
settings-telegram-test-failed = 테스트 메시지 전송에 실패했습니다
settings-telegram-session-revoked = 세션이 해지되었습니다
settings-telegram-session-revoke-failed = 세션을 해지하지 못했습니다

## licenses_tab.js

settings-licenses-title = 오픈 소스 라이선스
settings-licenses-subtitle = { -brand } 앱은 다음 오픈 소스 소프트웨어로 제작되었습니다
settings-licenses-footer = 전체 라이선스 원문은 프로젝트 저장소와 각 의존성의 소스 코드에서 확인할 수 있습니다.
settings-licenses-category-framework = 애플리케이션 프레임워크
settings-licenses-category-solana = 솔라나 블록체인
settings-licenses-category-data = 데이터 및 스토리지
settings-licenses-category-networking = 네트워킹
settings-licenses-category-cryptography = 암호화 및 인코딩
settings-licenses-category-assets = UI 에셋
settings-licenses-desc-electron = 데스크톱 애플리케이션 프레임워크
settings-licenses-desc-tokio = Rust 비동기 런타임
settings-licenses-desc-axum = 웹 서버 프레임워크
settings-licenses-desc-tower = 서비스 추상화
settings-licenses-desc-hyper = HTTP 구현
settings-licenses-desc-solana-sdk = Solana SDK 코어
settings-licenses-desc-solana-client = RPC 클라이언트
settings-licenses-desc-solana-program = 프로그램 라이브러리
settings-licenses-desc-spl-token = SPL 토큰 프로그램
settings-licenses-desc-spl-token-2022 = Token-2022 확장
settings-licenses-desc-spl-associated-token-account = 연관 토큰 계정
settings-licenses-desc-sqlite = 내장 데이터베이스 엔진
settings-licenses-desc-rusqlite = SQLite Rust 바인딩
settings-licenses-desc-r2d2 = 데이터베이스 연결 풀
settings-licenses-desc-serde = 직렬화 프레임워크
settings-licenses-desc-toml = 설정 파싱
settings-licenses-desc-reqwest = HTTP 클라이언트
settings-licenses-desc-tokio-tungstenite = WebSocket 클라이언트
settings-licenses-desc-rustls = TLS 구현
settings-licenses-desc-blake3 = 해시 함수
settings-licenses-desc-sha-2 = SHA-256/512 해싱
settings-licenses-desc-bs58 = Base58 인코딩
settings-licenses-desc-base64 = Base64 인코딩
settings-licenses-desc-lucide-icons = 아이콘 폰트 라이브러리
settings-licenses-desc-inter = 인터페이스 폰트
settings-licenses-desc-jetbrains-mono = 고정폭 폰트
settings-licenses-desc-orbitron = 디스플레이 폰트
settings-licenses-desc-vazirmatn = 아랍어 및 페르시아어 폰트
settings-licenses-desc-noto-sans-devanagari = 데바나가리 문자 폰트
settings-licenses-desc-noto-sans-sc = 중국어 간체 폰트
settings-licenses-desc-pretendard = 한국어 폰트
settings-licenses-desc-pretendard-jp = 일본어 폰트

## hints_tab.js

settings-hints-title = 상황별 힌트
settings-hints-description = 상황별 힌트는 대시보드 기능을 설명하는 도움말 아이콘입니다. 아래에서 모든 힌트를 확인하고, "다시 표시하지 않음"으로 숨긴 힌트를 하나씩 또는 한 번에 복원할 수 있습니다.
settings-hints-hidden-label = 숨긴 힌트
settings-hints-hidden-summary = 전체 힌트 { $total }개 중 { $hidden }개가 숨겨져 있습니다.
settings-hints-restore-all = 모든 힌트 복원
settings-hints-toggle-shown =
    .title = 이 힌트 표시
settings-hints-toggle-shown-title = 표시됨
settings-hints-toggle-hidden-title = 숨김 — 켜면 표시됩니다
settings-hints-restore-title = 모든 힌트 복원
settings-hints-restore-message = 숨긴 힌트를 포함하여 모든 상황별 힌트를 다시 표시하시겠습니까?
settings-hints-restore-confirm = 모두 복원
settings-hints-restored = 모든 힌트가 복원되었습니다

## account_tab.js

settings-account-title = { -brand } 계정
settings-account-description = 무료이며 선택 사항입니다. { -brand } 앱은 계정 없이도 거래, 탐색, 차트 표시를 하지만 공개 제공자를 통해서만 수행합니다. 아래 패널에서 로그인 시 추가되는 기능을 확인할 수 있습니다.
settings-account-data-title = { -brand } 데이터
settings-account-data-description = screenerbot.io에서 공유 시장 데이터 서비스를 운영합니다. 7개 타임프레임의 통합 캔들, 확인된 풀 레지스트리, 캐시된 보안 리포트, 정규화된 토큰 정보를 제공합니다. 모든 설치가 공개 제공자의 요청 제한을 각각 받지 않도록 하기 위한 것이며, 공유 비용을 계정에 귀속시키기 위해 계정이 필요합니다.
settings-account-data-fallback = 사용할 수 없을 때는 { -brand } 앱이 자동으로 공개 제공자로 전환합니다. 중단되는 기능은 없으며, 차트가 더 느리게 채워지고 히스토리가 줄어듭니다.
settings-account-gateway-title = 트랜잭션 전송
settings-account-gateway-description = 로그인하면 { -brand } 앱이 스왑을 사용자의 RPC 대신 screenerbot.io를 통해 전송할 수 있습니다. 모든 트랜잭션은 이 컴퓨터에서 생성 및 서명되며, 서버는 전달만 하고 서명된 트랜잭션을 서명 무효화 없이 변경할 수 없습니다.
settings-account-gateway-label = 트랜잭션 전송에 { -brand } RPC 사용
settings-account-gateway-hint = 전송 전용입니다. 가격 데이터는 항상 사용자의 RPC에서 가져오며, 풀 폴링은 공유 엔드포인트에 너무 부담이 커서 그곳으로 보내지 않습니다.
settings-account-manage-title = 계정 관리
settings-account-manage-description = 비밀번호, 이메일 주소, 연결된 기기, 추천 보상 지급은 웹사이트에서 관리합니다. 그곳에서 기기를 해지하면 이 기기를 포함해 모든 곳에서 로그아웃됩니다.
settings-account-open-dashboard = 내 대시보드 열기

## navigation_tab.js

settings-navigation-title = 내비게이션 탭
settings-navigation-hint = 항목을 드래그하여 순서를 변경하세요. 스위치로 표시 여부를 전환합니다.
settings-navigation-section-layout = 레이아웃
settings-navigation-overflow-label = 들어가지 않는 탭
settings-navigation-overflow-hint = 탭 줄을 옆으로 스크롤하거나, 들어가지 않는 탭을 끝의 더보기 메뉴에 모읍니다.
settings-navigation-overflow-scroll = 스크롤
settings-navigation-overflow-menu = 더보기 메뉴
settings-navigation-drag-handle =
    .title = 드래그하여 순서 변경
settings-navigation-defaults-failed = 기본 내비게이션을 불러오지 못했습니다
settings-navigation-reset = 내비게이션이 기본값으로 초기화되었습니다

## data_tab.js

settings-data-storage-title = 데이터베이스 저장소
settings-data-storage-description = 거래 데이터, 포지션, 과거 정보를 저장하는 모든 데이터베이스의 개요입니다.
settings-data-stats-loading = 데이터베이스 통계 불러오는 중...
settings-data-stats-load-failed = 데이터베이스 통계를 불러오지 못했습니다
settings-data-total-storage = 전체 데이터베이스 저장 용량
settings-data-db-tokens = 토큰
settings-data-db-transactions = 트랜잭션
settings-data-db-positions = 포지션
settings-data-db-events = 이벤트
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = 지갑
settings-data-db-pools = 풀
settings-data-db-strategies = 전략
settings-data-db-actions = 작업
settings-data-directory-label = 데이터 디렉터리
settings-data-directory-copied = 데이터 디렉터리
settings-data-config-path-copied = 설정 경로
settings-data-path-unavailable = 사용할 수 없음
settings-data-path-copy-title = 클릭하여 경로 복사
settings-data-path-copy-failed = 경로를 복사하지 못했습니다

settings-data-config-title = 설정 관리
settings-data-config-description = 봇 설정을 내보내고, 가져오고, 관리합니다. 큰 변경 전에는 백업을 보관하세요.
settings-data-config-export = 설정 내보내기
settings-data-config-import = 설정 가져오기
settings-data-config-reset = 기본값으로 초기화
settings-data-config-location-label = 설정 위치
settings-data-config-fetch-failed = 설정을 가져오지 못했습니다
settings-data-config-exported = 설정을 내보냈습니다
settings-data-config-export-failed = 설정을 내보내지 못했습니다: { $message }
settings-data-config-import-title = 설정 가져오기
settings-data-config-import-message = 이 설정을 가져오시겠습니까? 현재 설정을 덮어씁니다. 지갑 자격 증명은 유지됩니다.
settings-data-config-imported = 설정을 가져왔습니다. 일부 변경 사항은 재시작이 필요할 수 있습니다.
settings-data-config-import-failed = 설정을 가져오지 못했습니다: { $message }
settings-data-config-reset-title = 설정 초기화
settings-data-config-reset-message = 모든 설정을 기본값으로 초기화하시겠습니까? 지갑 자격 증명은 유지되지만 나머지 모든 설정은 초기화됩니다.
settings-data-config-reset-done = 설정이 기본값으로 초기화되었습니다
settings-data-config-reset-failed = 설정을 초기화하지 못했습니다: { $message }
settings-data-unknown-error = 알 수 없는 오류

settings-data-cleanup-title = 데이터 정리
settings-data-cleanup-description = 오래되었거나 사용하지 않는 데이터를 삭제하여 디스크 공간을 확보합니다. 이 작업은 되돌릴 수 없습니다.
settings-data-ohlcv-cleanup-label = OHLCV 데이터 정리
settings-data-ohlcv-cleanup-hint = 지정한 시간 동안 활동이 없던 토큰의 캔들스틱 데이터를 삭제합니다.
settings-data-cleanup-hours-unit = 시간
settings-data-cleanup-ohlcv = OHLCV 정리
settings-data-cleanup-running = 정리 중...
settings-data-cleanup-hours-invalid = 시간 값이 올바르지 않습니다
settings-data-cleanup-confirm-title = OHLCV 데이터 삭제
settings-data-cleanup-confirm-message =
    { $hours ->
       *[other] { $hours }시간 넘게 비활성 상태인 토큰의 OHLCV 데이터를 삭제하시겠습니까?
    }
settings-data-cleanup-done =
    { $count ->
       *[other] 비활성 토큰 { $count }개를 정리했습니다
    }
settings-data-cleanup-failed = 정리에 실패했습니다
settings-data-cleanup-failed-detail = 정리에 실패했습니다: { $message }

settings-data-cache-clear-label = 모든 OHLCV 캐시 삭제
settings-data-cache-clear-hint = 캐시된 모든 캔들스틱 데이터를 삭제하고 모니터링 중인 모든 토큰을 처음부터 다시 가져옵니다. 차트가 이상하거나 데이터 로직 업데이트 후에 사용하세요.
settings-data-cache-clear = OHLCV 캐시 삭제
settings-data-cache-clearing = 삭제 중...
settings-data-cache-confirm-title = 모든 OHLCV 캐시 삭제
settings-data-cache-confirm-message = 모든 토큰의 캐시된 캔들스틱 데이터를 삭제하시겠습니까? 모니터링 중인 토큰은 히스토리를 처음부터 다시 가져옵니다. 이 작업은 되돌릴 수 없습니다.
settings-data-candles-count =
    { $count ->
       *[other] 캔들 { $count }개
    }
settings-data-tokens-count =
    { $count ->
       *[other] 토큰 { $count }개
    }
settings-data-cache-cleared = 삭제됨: { $candles } ({ $tokens }) — 다시 가져오는 중
settings-data-cache-clear-failed = OHLCV 캐시를 삭제하지 못했습니다
settings-data-cache-clear-failed-detail = OHLCV 캐시를 삭제하지 못했습니다: { $message }

settings-data-ui-cache-label = UI 상태 캐시
settings-data-ui-cache-hint = 저장된 표 환경설정, 필터 상태, 보기 설정을 삭제합니다.
settings-data-ui-cache-clear = UI 캐시 삭제
settings-data-ui-cache-confirm-title = UI 상태 삭제
settings-data-ui-cache-confirm-message = 저장된 모든 UI 환경설정을 삭제하시겠습니까? 표 열, 필터, 보기 설정이 초기화됩니다.
settings-data-ui-cache-cleared =
    { $count ->
       *[other] 캐시된 UI 설정 { $count }개를 삭제했습니다
    }

settings-data-folder-label = 데이터 폴더 열기
settings-data-folder-hint = { -brand } 데이터가 모두 들어 있는 폴더를 파일 관리자에서 엽니다.
settings-data-folder-open = 폴더 열기
settings-data-folder-open-failed = 데이터 폴더를 열지 못했습니다
