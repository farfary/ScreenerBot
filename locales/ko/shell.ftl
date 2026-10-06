# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = 대시보드 홈 열기
    .title = 대시보드 홈
shell-bot-card =
    .aria-label = 자동 트레이더 상태 불러오는 중
shell-bot-label = 자동
shell-bot-status-loading = 불러오는 중
shell-bot-today = 오늘
shell-explore-control =
    .aria-label = 탐색 모드. 지갑과 RPC 엔드포인트를 연결하면 모든 기능을 사용할 수 있습니다
    .title = 지갑과 RPC 엔드포인트를 연결하면 거래, 잔액, 실시간 온체인 데이터를 사용할 수 있습니다
shell-explore-title = 탐색 모드
shell-explore-detail = 지갑 및 RPC 연결 안 됨
shell-explore-action = 설정 완료
shell-wallet-card =
    .aria-label = 지갑 가치, 포지션 열기
    .title = 지갑 가치 ({ -sol } + 토큰) · 포지션 열기
shell-wallet-worth-label = 가치
shell-wallet-native-label = { -sol }
shell-wallet-tokens-label = 토큰
shell-sol-price-card =
    .aria-label = { -sol } USD 가격 - 차트 열기
    .title = { -sol } 가격 · 클릭하면 차트 표시
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = 카피 트레이딩, 카피 트레이딩 열기
    .title = 카피 트레이딩 · 카피 트레이딩 열기
shell-copy-label = 카피
shell-actions-more =
    .aria-label = 헤더 작업 더 보기
    .title = 작업 더 보기
shell-actions-group =
    .aria-label = 헤더 작업
shell-action-search =
    .aria-label = 토큰 검색
    .title = 토큰 검색 (Ctrl/Cmd+K)
shell-action-featured =
    .aria-label = 추천 토큰
    .title = 추천 토큰
shell-action-notifications =
    .aria-label = 작업 및 알림
    .title = 작업 및 알림
shell-action-restart =
    .aria-label = 앱 재시작
    .title = 앱 재시작
shell-action-theme =
    .aria-label = 테마 전환
    .title = 테마 전환
shell-action-settings =
    .aria-label = 설정
    .title = 설정

## Ticker

shell-ticker-monitoring-segment =
    .title = 풀 서비스가 모니터링 중인 토큰
shell-ticker-monitoring = 모니터링:
shell-ticker-filtering-segment =
    .title = 필터링 기준을 통과/제외한 토큰
shell-ticker-passed = 통과:
shell-ticker-rejected = 제외:
shell-ticker-pnl-segment =
    .title = 오늘의 실현 손익
shell-ticker-pnl = 오늘 손익:
shell-ticker-rpc-segment =
    .title = 분당 RPC 호출 수와 성공률
shell-ticker-rpc = RPC:
shell-ticker-rpc-rate = { $amount }/분
shell-ticker-services-segment =
    .title = 백그라운드 서비스 상태
shell-ticker-services-loading = 서비스: <strong>불러오는 중</strong>

## Notification drawer

shell-notification-title = 작업
shell-notification-mark-all-read =
    .title = 모두 읽음으로 표시
shell-notification-clear-all =
    .title = 모두 지우기
shell-notification-close =
    .aria-label = 닫기
shell-notification-tab-all = 전체
shell-notification-tab-active = 진행 중
shell-notification-tab-done = 완료
shell-notification-tab-failed = 실패
shell-notification-filter-type-all = 모든 유형
shell-notification-filter-type-buy = 매수
shell-notification-filter-type-sell = 매도
shell-notification-filter-type-open = 진입
shell-notification-filter-type-close = 종료
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = 부분
shell-notification-filter-state-all = 모든 상태
shell-notification-filter-state-in-progress = 진행 중
shell-notification-filter-state-completed = 완료됨
shell-notification-filter-state-failed = 실패
shell-notification-filter-state-cancelled = 취소됨
shell-notification-list =
    .aria-label = 알림
shell-notification-empty = 아직 작업이 없습니다
shell-notification-loading-more = 더 불러오는 중...
shell-notification-back-to-top =
    .title = 맨 위로

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = 가동
shell-status-bar-memory = 메모리
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/분
shell-status-bar-trading = 거래
shell-status-bar-positions = 포지션
shell-status-bar-tokens = 토큰

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = { -brand } 시작 중
shell-splash-waiting = 로컬 코어의 응답을 기다리는 중입니다.
shell-splash-failed = { -brand }을 시작하지 못했습니다
shell-splash-failed-detail = 로그 파일을 확인한 후 앱을 다시 시작하세요.

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = 코어 연결됨
shell-connection-waiting = 코어 대기 중…
shell-connection-retry-now = 지금 다시 시도
shell-connection-overlay-detail = 코어에 연결할 수 없습니다. 거래가 일시 중지되었으며 자동으로 복구됩니다.
shell-connection-restored = 코어 연결이 복구되었습니다

# Source: scripts/core/header.js
shell-trader-control-failed = 트레이더 제어에 실패했습니다
shell-notification-button-unread = 작업 및 알림, 읽지 않음 { $count }건
shell-restart-confirm-title = 봇 재시작
shell-restart-confirm-message =
    봇을 재시작하시겠습니까?

    다음이 수행됩니다:
    • 모든 서비스 중지
    • 프로세스 재시작
    • 약 10-15초 소요

    진행 중인 모든 작업이 중단됩니다.
shell-restart-confirm-action = 재시작
shell-restart-progress = 봇 재시작 중
shell-restart-failed = 재시작 실패
shell-restart-failed-status = 재시작 실패: { $status }
shell-restart-helper-unavailable = 자동 재시작 도우미를 사용할 수 없습니다. 잠시 후 대시보드를 새로 고치세요.

# Source: scripts/core/router.js
shell-page-title-fallback = 대시보드
shell-page-load-failed = 페이지를 불러오지 못했습니다
shell-page-offline-detail = 지금은 코어에 연결할 수 없습니다. 연결이 복구되면 이 페이지가 자동으로 로드됩니다.

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = 탐색
shell-bot-state-halted = 중단됨
shell-bot-state-off = 꺼짐
shell-bot-state-waiting = 대기 중
shell-bot-state-idle = 유휴
shell-bot-state-entry-paused = 진입 일시 중지
shell-bot-state-running = 실행 중
shell-bot-control-explore = 탐색 모드에서는 자동 트레이더를 사용할 수 없습니다. 지갑 및 RPC 설정을 여세요.
shell-bot-control-halted = 긴급 중지가 활성화되어 있습니다. 자동 트레이더 제어를 여세요.
shell-bot-control-off = 자동 트레이더가 꺼져 있습니다. 클릭하여 켜세요.
shell-bot-control-waiting = 자동 트레이더가 켜져 있으며 코어 서비스를 기다리는 중입니다. 클릭하여 끄세요.
shell-bot-control-idle = 자동 트레이더가 켜져 있지만 두 모니터가 모두 꺼져 있습니다. 자동 트레이더 제어를 여세요.
shell-bot-control-entry-paused = 손실 보호로 진입이 일시 중지되었으며 청산은 계속될 수 있습니다. 자동 트레이더 제어를 여세요.
shell-bot-control-running = 자동 트레이더가 실행 중입니다. 클릭하여 끄세요.

## Wallet and copy cards

shell-wallet-card-summary = 지갑 가치: { $equity } { -sol } (잔액 { $balance } { -sol }, 토큰 { $tokens }), 포지션 열기
shell-copy-running-live = 실거래 { $count }
shell-copy-running-paper = 모의 { $count }
shell-copy-value-paused = 일시 중지됨
shell-copy-value-idle = 유휴
shell-copy-sub-active = { $total } 중 { $active } 활성

## Ticker services state

shell-ticker-services-healthy = 서비스: <strong>정상</strong>
shell-ticker-services-issues =
    { $count ->
       *[other] 서비스: <strong>문제 { $count }건</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = 에이전트 요청
shell-agent-request-client-fallback = 연결된 에이전트
shell-agent-request-message = 요청 클라이언트: { $client }. { -brand }에서 "{ $tool }" 실행을 요청했습니다. 이 요청은 { $expiry }.
shell-agent-request-message-arguments = 요청 클라이언트: { $client }. { -brand }에서 "{ $tool }" 실행을 요청했습니다. 인수: { $summary }. 이 요청은 { $expiry }.
shell-agent-request-expires-minutes = { $minutes }분 후 만료됩니다
shell-agent-request-expires-seconds = { $seconds }초 후 만료됩니다
shell-agent-request-approve = 승인
shell-agent-request-deny = 거부

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } 복사됨
shell-toast-copy-failed = 복사 실패
shell-toast-still-running = 아직 실행 중입니다. 알림 센터를 확인하세요
shell-toast-dismiss =
    .aria-label = 닫기
shell-confirm-title = 작업 확인
shell-confirm-message = 계속하시겠습니까?
shell-address-open-solscan = - { -solscan }에서 열기
shell-address-copy = 주소 복사

# Source: scripts/core/global_chat.js
shell-assistant-label = 어시스턴트
shell-assistant-dialog =
    .aria-label = 어시스턴트

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = 활성
shell-status-bar-trading-inactive = 비활성

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title } 취소됨
shell-action-swap-buy-live = 매수 중
shell-action-swap-buy-done = 매수 완료
shell-action-swap-buy-failed = 매수 실패
shell-action-swap-sell-live = 매도 중
shell-action-swap-sell-done = 매도 완료
shell-action-swap-sell-failed = 매도 실패
shell-action-position-open-live = 포지션 진입 중
shell-action-position-open-done = 진입 완료
shell-action-position-open-failed = 진입 실패
shell-action-position-close-live = 포지션 종료 중
shell-action-position-close-done = 종료됨
shell-action-position-close-failed = 종료 실패
shell-action-position-dca-live = 포지션 추가 매수 중
shell-action-position-dca-done = 추가 매수 완료
shell-action-position-dca-failed = 추가 매수 실패
shell-action-partial-exit-live = 부분 청산 중
shell-action-partial-exit-done = 부분 청산 완료
shell-action-partial-exit-failed = 부분 청산 실패
shell-action-manual-order-live = 주문 접수 중
shell-action-manual-order-done = 주문 완료
shell-action-manual-order-failed = 주문 실패
shell-action-trade-live = 거래 중
shell-action-trade-done = 거래 완료
shell-action-trade-failed = 거래 실패
shell-action-via-router = { $action } ({ $router } 경유)
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = { $venue } 회피 중
shell-action-cost-guard-avoiding-cost = { $venue } 회피 중 · { $cost }
shell-action-cost-guard-avoiding-unnamed = 거래소 회피 중
shell-action-cost-guard-avoiding-unnamed-cost = 거래소 회피 중 · { $cost }
shell-action-cost-guard-avoided = { $outcome } · { $venue } 렌트 { $cost } 회피
shell-action-cost-guard-avoided-unnamed = { $outcome } · 거래소 렌트 { $cost } 회피
shell-action-exit-full = 전체 청산
shell-action-exit-percent = { $percent } 청산

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = { -brand }을 종료하시겠습니까?
shell-exit-description = 애플리케이션을 종료할 방식을 선택하세요
shell-exit-minimize = 트레이로 최소화
shell-exit-minimize-detail = 백그라운드에서 계속 실행
shell-exit-quit = 앱 종료
shell-exit-quit-detail = 완전히 종료하고 모든 서비스를 중지

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = 이미지 저장
shell-lightbox-close =
    .title = 닫기 (ESC)

## Theme control (scripts/theme.js)

shell-theme-light = 라이트
shell-theme-dark = 다크
shell-theme-switch-to-light = 라이트 테마로 전환
shell-theme-switch-to-dark = 다크 테마로 전환
