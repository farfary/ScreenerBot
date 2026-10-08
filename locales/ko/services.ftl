# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

services-health-component-unavailable = { $component } 컴포넌트를 사용할 수 없습니다
services-health-unavailable = 상태를 확인할 수 없습니다
services-health-pools-not-running = 풀 서비스가 실행 중이 아닙니다
services-health-events-db-uninitialized = 이벤트 데이터베이스가 초기화되지 않았습니다
services-health-sol-price-not-running = SOL 가격 서비스가 실행 중이 아닙니다
services-health-sol-price-stale = SOL 가격 데이터가 오래되었습니다 ({ $seconds }초 경과)
services-health-sol-price-no-data = 아직 SOL 가격 데이터가 없습니다
services-health-telegram-discovery = 디스커버리 모드
services-health-telegram-disconnected = 연결 끊김
services-health-wallet-watch-polling-only = 폴링만으로 감지 중
services-health-assistant-tasks-disabled = 설정에서 비활성화됨
services-health-connectivity-critical-unhealthy = 핵심 엔드포인트 비정상: { $endpoints }
services-health-filtering-snapshot-stale = 필터링 스냅샷이 { $seconds }초 지났습니다

## Services page (pages/services.js)

services-status-healthy = 정상
services-status-starting = 시작 중
services-status-degraded = 성능 저하
services-status-unhealthy = 비정상
services-status-stopping = 중지 중
services-status-disabled = 비활성화됨
services-status-unknown = 알 수 없음

services-name-account = 계정
services-name-assistant-scheduled-tasks = 어시스턴트 예약 작업
services-name-ata-cleanup = 토큰 계정 정리
services-name-connectivity = 연결 상태
services-name-copy-trading = 카피 트레이딩
services-name-events = 이벤트
services-name-filtering = 필터링
services-name-llm-analysis = LLM 분석
services-name-ohlcv = OHLCV
services-name-pool-pricing = 풀 가격 산정
services-name-pools = 풀
services-name-positions = 포지션
services-name-referral = 추천
services-name-rpc-stats = RPC 통계
services-name-sol-price = { -sol } 가격
services-name-telegram = { -telegram }
services-name-tokens = 토큰
services-name-trader = 트레이더
services-name-transactions = 트랜잭션
services-name-update-check = 업데이트 확인
services-name-wallet = 지갑
services-name-wallet-watch = 지갑 감시
services-name-webserver = 웹 서버

services-loading = 서비스를 불러오는 중...
services-load-failed = 서비스를 불러오지 못했습니다
services-load-failed-description = 백엔드 응답을 기다리는 중입니다. 자동으로 다시 시도합니다.
services-refresh-failed = 서비스를 새로 고칠 수 없습니다
services-search-placeholder = 서비스 검색...
services-summary-total = 전체
services-summary-alerts = 경고
services-summary-alerts-tooltip = 성능 저하 { $degraded } / 비정상 { $unhealthy }
services-filter-status = 상태
services-filter-all-statuses = 모든 상태
services-filter-all-services = 모든 서비스
services-filter-enabled-only = 활성화된 항목만
services-filter-disabled-only = 비활성화된 항목만
services-col-service = 서비스
services-col-health = 상태
services-col-priority = 우선순위
services-col-uptime = 가동 시간
services-col-activity = 활동
services-col-last-cycle = 마지막 사이클
services-col-avg-cycle = 평균 사이클
services-col-avg-poll = 평균 폴링
services-col-cycle-rate = 사이클 속도
services-col-tasks = 작업
services-col-ops = 작업/초
services-col-errors = 오류
services-col-dependencies = 의존성
services-dependencies-none = 없음
services-activity-busy = 사용률 { $percent }
services-activity-polls =
    { $count ->
       *[other] 폴링 { $count }회
    }
services-tasks-tooltip =
    { $count ->
       *[other] 작업 { $count }개
    }
    마지막: { $last }
    평균: { $avg }
    폴링: { $poll }
    유휴: { $idle }
    전체 폴링: { $polls }
services-tasks-none = 계측된 작업 없음

# Empty table (scripts/pages/services.js)
services-empty = 실행 중인 서비스가 없습니다
    .message = 봇이 서비스를 시작하면 여기에 표시됩니다.
