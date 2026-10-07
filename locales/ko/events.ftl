# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = OHLCV 이벤트: { $subtype }
events-filtering-default = 필터링 이벤트: { $subtype }
events-trader-default = 트레이더 이벤트: { $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = 작업 '{ $name }' 완료
events-task-failed = 작업 '{ $name }' 실패
events-task-timed-out = 작업 '{ $name }' 시간 초과

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = 작업 완료
events-subtype-task-failed = 작업 실패
events-subtype-task-timed-out = 작업 시간 초과

# Shown when an event carries no display text.
events-message-none = 메시지 없음

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = OHLCV 캐시를 정리하지 못했습니다
events-ohlcv-gap-cleanup-failed = 채워진 공백 레코드를 정리하지 못했습니다
events-ohlcv-gap-fill-failed = { $mint } 공백 채우기 오류
events-ohlcv-backfill-scheduled = { $mint }의 멀티 타임프레임 백필을 { $pool } 경유로 예약했습니다
events-ohlcv-fetch-failed = { $mint }의 OHLCV를 { $pool } 경유로 가져오지 못했습니다: { $error }
events-ohlcv-gap-detection-failed = { $mint }의 공백 감지에 실패했습니다 ({ $pool } 경유)
events-ohlcv-fetch-success = { $mint }의 OHLCV 데이터 { $count }개를 저장했습니다
events-ohlcv-retention-backfill-failed = { $mint }의 보존 기간 백필에 실패했습니다 ({ $pool } 경유)
events-ohlcv-empty-fetch = { $mint }의 OHLCV 조회 결과가 비어 있습니다 ({ $pool } 경유)
events-ohlcv-pool-discovery-failed = { $mint }의 풀 탐색에 실패했습니다
events-ohlcv-pool-discovery-success = { $mint }의 풀을 찾았습니다
events-ohlcv-process-token-error = { $mint } 처리 중 오류: { $error }
events-ohlcv-rate-limit-hit = { $mint } 처리 중 요청 제한이 발생했습니다
events-ohlcv-pool-unavailable = { $mint }에 사용할 수 있는 정상 풀이 없어 보류합니다
events-ohlcv-token-missing = 처리 중 토큰 { $mint }을 찾을 수 없었습니다
events-monitors-stopped = 자동 거래 모니터가 중지되었습니다
events-monitors-starting = 자동 거래 모니터를 시작하는 중입니다
events-entry-monitor-started = 진입 기회 모니터가 시작되었습니다
events-exit-monitor-started = 청산/포지션 모니터가 시작되었습니다
events-trader-service-stopped = 트레이더 서비스가 정상적으로 중지되었습니다
events-trader-service-stopping = 트레이더 서비스 종료가 시작되었습니다
events-trader-service-started = 트레이더 서비스가 완전히 초기화되어 실행 중입니다
events-trader-auto-trading-error = 자동 거래 중 오류가 발생했습니다
events-trader-trading-enabled = 거래가 활성화되어 있습니다
events-trader-trading-disabled = 설정에서 거래가 비활성화되어 있습니다
events-trader-service-initializing = 트레이더 서비스 초기화를 시작합니다
events-connectivity-monitoring-stopped = 연결 모니터링이 중지되었습니다
events-connectivity-monitoring-started = 연결 모니터링이 시작되었습니다 (간격={ $seconds }초)
events-connectivity-service-initialized = 연결 서비스가 모니터 { $count }개로 초기화되었습니다
events-connectivity-critical-unhealthy = 핵심 엔드포인트 { $count }개가 비정상입니다 - 시스템이 작업을 일시 중지해야 합니다
events-connectivity-endpoint-recovered = 엔드포인트가 { $from }에서 정상으로 복구되었습니다
events-position-entry-not-landed = { $symbol } 매수가 온체인에 반영되지 않아 포지션을 삭제했습니다
events-position-fill-after-force-close = { $symbol } 거래가 포지션 강제 종료 후 온체인에 반영되어 기록하고 포지션을 다시 계산했습니다

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = 스왑
events-category-transaction = 트랜잭션
events-category-pool = 풀
events-category-position = 포지션
events-category-token = 토큰
events-category-wallet = 지갑
events-category-trader = 트레이더
events-category-entry = 진입
events-category-system = 시스템
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = 보안
events-category-connectivity = 연결
events-category-filtering = 필터링
events-category-scheduled-task = 예약 작업
events-category-learner = 학습기
events-category-other = 기타

events-loading = 이벤트 불러오는 중...
events-load-failed = 이벤트를 불러오지 못했습니다
events-load-failed-description = 백엔드의 응답을 기다리는 중입니다. 자동으로 다시 시도합니다.
events-load-error = 이벤트를 불러올 수 없습니다
events-search-placeholder = 이벤트 검색...
events-summary-total = 전체
events-filter-category = 카테고리
events-filter-all-categories = 모든 카테고리
events-filter-all-severities = 모든 심각도
events-col-time = 시각
events-col-category = 카테고리
events-col-type = 유형
events-col-severity = 심각도
events-col-message = 메시지
events-col-token = 토큰
events-col-details = 상세
# $count is the number of payload entries not shown in the preview.
events-payload-more = +{ $count }개 더

## Event details dialog (ui/events_dialog.js)

events-dialog-title = 이벤트 상세
events-dialog-close =
    .aria-label = 대화상자 닫기
events-dialog-payload = 페이로드
events-dialog-copy = 상세 정보 복사
events-dialog-copy-title =
    .title = 모든 이벤트 상세 정보 복사
events-dialog-copy-done = 복사됨
events-dialog-copy-failed = 실패
events-dialog-not-available = 해당 없음
# $category is the category label; shown when an event has no message.
events-dialog-category-event = { $category } 이벤트
events-dialog-field-id = 이벤트 ID
events-dialog-field-severity = 심각도
events-dialog-field-category = 카테고리
events-dialog-field-subtype = 하위 유형
events-dialog-field-mint = 토큰 민트
events-dialog-field-reference = 참조
events-dialog-field-time = 이벤트 시각
events-dialog-field-age = 경과 시간
events-dialog-field-created = 생성 시각
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = 이벤트 상세
events-dialog-export-message = 메시지
events-dialog-export-payload = 페이로드
events-dialog-export-line = { $label }: { $value }
