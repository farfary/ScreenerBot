# Assistant page shell and chat widget. Model output (chat replies, tool
# results, decision reasoning) is data and is never part of this catalog.

assistant-provider-openai = { -openai }
assistant-provider-anthropic = { -anthropic }
assistant-provider-groq = { -groq }
assistant-provider-deepseek = { -deepseek }
assistant-provider-gemini = { -google-gemini }
assistant-provider-ollama = { -ollama }
assistant-provider-together = { -together-ai }
assistant-provider-openrouter = { -openrouter }
assistant-provider-mistral = { -mistral-ai }
assistant-provider-select = 제공자 선택...

assistant-decision-allow = 허용
assistant-decision-pass = 통과
assistant-decision-reject = 제외
assistant-decision-buy = 매수
assistant-decision-sell = 매도
assistant-decision-hold = 보유

assistant-risk-low = 낮음
assistant-risk-medium = 보통
assistant-risk-high = 높음
assistant-risk-critical = 치명적

# assistant.html: page shell.
assistant-overview-title = 어시스턴트 개요
assistant-overview-description = 실시간 LLM 및 분석 상태, 성능 지표, 최신 모델 평가를 확인합니다.
assistant-features-title = 모델 기능
assistant-metric-total-evaluations = 총 평가 수
assistant-metric-cache-hit-rate = 캐시 적중률
assistant-metric-avg-latency = 평균 지연 시간
assistant-metric-active-providers = 활성 제공자
assistant-decisions-title = 최근 결정
assistant-decisions-subtitle = 최신 모델 분석 결정

assistant-providers-title = LLM 제공자
assistant-providers-description = 분석과 어시스턴트가 공유하는 LLM 제공자를 설정합니다

assistant-master-title = 마스터 설정
assistant-master-description = 공유 LLM 마스터 스위치와 제공자 선택
assistant-setting-enabled-description = LLM 제공자를 사용한 지능형 토큰 분석을 활성화합니다
assistant-setting-default-provider-label = 기본 제공자
assistant-setting-default-provider-description = 분석, 어시스턴트, 예약 작업에서 사용하는 단일 제공자

assistant-filtering-title = 필터링
assistant-filtering-description = 진입 결정 전에 어시스턴트가 토큰을 필터링합니다
assistant-setting-filtering-enabled-label = 어시스턴트 토큰 필터링
assistant-setting-filtering-enabled-description = 진입 거래를 허용하기 전에 어시스턴트로 토큰을 평가합니다
assistant-setting-min-confidence-label = 최소 신뢰도
assistant-setting-min-confidence-description = 통과하려면 토큰이 최소 이 신뢰도 점수를 받아야 합니다 (0-100)
assistant-setting-fallback-pass-label = 실패 시 통과 처리
assistant-setting-fallback-pass-description = 어시스턴트 평가가 실패하거나 시간 초과되면 토큰 진입을 허용합니다

assistant-trading-title = 거래
assistant-trading-description = 거래 과정에서의 어시스턴트 분석
assistant-setting-entry-analysis-label = 진입 분석
assistant-setting-entry-analysis-description = 포지션을 열기 전에 어시스턴트 분석을 실행합니다
assistant-setting-exit-analysis-label = 청산 분석
assistant-setting-exit-analysis-description = 포지션을 종료하기 전에 어시스턴트 분석을 실행합니다
assistant-setting-trailing-stop-label = 트레일링 스톱 분석
assistant-setting-trailing-stop-description = 트레일링 스톱이 발동되면 어시스턴트 분석을 실행합니다

assistant-blacklist-title = 자동 블랙리스트
assistant-blacklist-description = 어시스턴트 점수를 기준으로 토큰을 자동으로 차단합니다
assistant-setting-auto-blacklist-label = 자동 블랙리스트
assistant-setting-auto-blacklist-description = 어시스턴트 점수가 매우 낮은 토큰을 자동으로 블랙리스트에 추가합니다
assistant-setting-blacklist-threshold-label = 블랙리스트 기준값
assistant-setting-blacklist-threshold-description = 이 점수 미만인 토큰은 자동으로 블랙리스트에 추가됩니다 (0-100)

assistant-performance-title = 성능
assistant-performance-description = 캐시 및 동시성 설정
assistant-setting-cache-ttl-label = 캐시 TTL
assistant-setting-cache-ttl-description = 어시스턴트 결과를 캐시하는 기간 (60-3600초)
assistant-setting-max-evaluations-label = 최대 평가 수
assistant-setting-max-evaluations-description = 동시에 허용되는 최대 어시스턴트 평가 수 (1-20)

assistant-cache-title = 캐시 관리
assistant-cache-description = 캐시된 어시스턴트 평가를 확인하고 삭제합니다
assistant-cache-size-label = 캐시 크기:
assistant-cache-memory-label = 메모리 사용량:
assistant-cache-clear = 캐시 삭제

assistant-testing-title = 어시스턴트 테스트 플레이그라운드
assistant-testing-subtitle = 모든 토큰에 대해 어시스턴트 분석을 테스트합니다
assistant-testing-mint-label = 민트 주소
assistant-testing-mint-input =
    .placeholder = Solana 토큰 민트 주소 입력...
assistant-testing-priority-label = 우선순위
assistant-testing-priority-low = 낮음
assistant-testing-priority-medium = 보통
assistant-testing-priority-high = 높음
assistant-testing-evaluate = 평가
assistant-testing-results = 결과

assistant-instructions-title = 사용자 지정 지침
assistant-instructions-description = 어시스턴트 평가에 주입되는 사용자 지정 프롬프트를 추가합니다
assistant-instructions-new = 새 지침
assistant-instructions-category-all = 모든 카테고리
assistant-instructions-category-filtering = 필터링
assistant-instructions-category-trading = 거래
assistant-instructions-category-analysis = 분석
assistant-instructions-category-general = 일반
assistant-instructions-status-all = 모든 상태
assistant-instructions-status-active = 활성만
assistant-instructions-status-inactive = 비활성만
assistant-instructions-search =
    .placeholder = 지침 검색...
assistant-instructions-empty = 사용자 지정 지침이 아직 없습니다
assistant-instructions-empty-add = 첫 지침 추가
assistant-templates-title = 지침 템플릿
assistant-templates-description = 추가할 수 있는 사전 구성된 지침 템플릿

assistant-automation-title = 자동화
assistant-automation-description = 일정 간격 또는 지정한 시간에 어시스턴트 작업이 자동으로 실행되도록 예약합니다
assistant-automation-new =
    .aria-label = 새 자동화 작업 만들기
assistant-automation-new-label = 새 작업
assistant-automation-stat-total = 전체 작업
assistant-automation-stat-active = 활성
assistant-automation-stat-runs = 총 실행 횟수
assistant-automation-stat-success-rate = 성공률
assistant-automation-empty = 예약된 작업이 아직 없습니다
assistant-automation-empty-subtitle = 첫 자동화 어시스턴트 작업을 만들어 시작하세요
assistant-automation-empty-add = 첫 작업 만들기
    .aria-label = 첫 자동화 작업 만들기
assistant-automation-runs-title = 최근 실행

assistant-history-title = 결정 기록
assistant-history-subtitle = 최근 어시스턴트 평가

# assistant.js: page tabs, overview, settings and history.
assistant-tab-chat = 채팅
assistant-tab-overview = 개요
assistant-tab-providers = 제공자
assistant-tab-instructions = 지침
assistant-tab-automation = 자동화
assistant-tab-history = 기록
assistant-tab-testing = 테스트
assistant-tab-settings = 설정
assistant-toggle-on = 사용
assistant-toggle-off = 사용 안 함
assistant-status-active = 어시스턴트 활성
assistant-status-disabled = 어시스턴트 비활성
assistant-status-load-failed = 모델 기능 상태를 불러올 수 없습니다
assistant-toggle-enabled-title = 어시스턴트 활성화됨
assistant-toggle-enabled-message = 모델 기반 기능이 활성화되었습니다
assistant-toggle-disabled-title = 어시스턴트 비활성화됨
assistant-toggle-disabled-message = 모델 기반 기능이 비활성화되었습니다
assistant-toggle-failed = 모델 기능 상태를 업데이트하지 못했습니다
assistant-decisions-empty = 최근 결정이 없습니다
assistant-decision-latency =
    .title = 지연 시간
assistant-decision-confidence =
    .title = 신뢰도
assistant-config-load-failed = 분석 설정을 불러올 수 없습니다
assistant-cache-clear-message = 분석 캐시를 삭제하시겠습니까? 캐시된 모든 모델 결정이 제거됩니다.
assistant-cache-cleared-title = 캐시 삭제됨
assistant-cache-cleared-message = 분석 캐시가 비어 있습니다
assistant-cache-clear-failed = 캐시를 삭제하지 못했습니다
assistant-config-saved-title = 저장됨
assistant-config-saved-message = 설정이 저장되었습니다
assistant-config-save-failed = 설정을 저장하지 못했습니다
assistant-history-load-failed = 기록을 불러오지 못했습니다
assistant-history-empty = LLM 분석 요청이 아직 없습니다
assistant-history-column-token = 토큰
assistant-history-column-decision = 결정
assistant-history-column-confidence = 신뢰도
assistant-history-column-risk = 위험
assistant-history-column-reasoning = 근거
assistant-history-column-model = 모델
assistant-history-column-latency = 지연 시간
assistant-history-column-when = 시각
assistant-history-previous = 이전
assistant-history-page = { $page } / { $total } 페이지
assistant-history-cached = 캐시됨

# chat_widget.js: sessions sidebar and header.
assistant-chat-sessions-title = 세션
assistant-chat-search =
    .placeholder = 채팅 검색...
    .aria-label = 채팅 세션 검색
assistant-chat-history-close =
    .aria-label = 채팅 기록 닫기
assistant-chat-history-open =
    .title = 채팅 기록
    .aria-label = 채팅 기록 열기
assistant-chat-header-new =
    .title = 새 채팅
    .aria-label = 새 채팅 시작
assistant-chat-delete =
    .title = 삭제
    .aria-label = 세션 삭제
assistant-chat-close =
    .title = 닫기
    .aria-label = 어시스턴트 닫기
assistant-chat-title-new = 새 채팅
assistant-chat-sessions-empty = 채팅 세션이 아직 없습니다
assistant-chat-sessions-empty-search = 일치하는 채팅이 없습니다
assistant-chat-group-today = 오늘
assistant-chat-group-yesterday = 어제
assistant-chat-group-week = 지난 7일
assistant-chat-group-older = 이전

# chat_widget.js: empty state and quick prompts.
assistant-chat-empty-kicker = 어시스턴트
assistant-chat-empty-title = 무엇을 도와드릴까요?
assistant-chat-empty-subtitle = 포트폴리오를 점검하고, 토큰을 조사하고, 최근 거래 활동을 파악하세요.
assistant-chat-prompt-positions-label = 보유 포지션 점검
assistant-chat-prompt-positions-text = 현재 지갑 잔액과 보유 포지션을 알려주세요.
assistant-chat-prompt-token-label = 토큰 분석
assistant-chat-prompt-token-text = 다음 토큰의 보안과 위험을 분석해 주세요:
assistant-chat-prompt-activity-label = 최근 활동 설명
assistant-chat-prompt-activity-text = 최근 거래 활동과 주요 결과를 설명해 주세요

# chat_widget.js: input area.
assistant-chat-input =
    .placeholder = 어시스턴트에게 메시지 보내기...
    .aria-label = 메시지 입력
assistant-chat-hint-key-enter = Enter
assistant-chat-hint-key-shift = Shift
assistant-chat-hint-send = 전송
assistant-chat-hint-newline = 줄바꿈
assistant-chat-send =
    .title = 메시지 전송
    .aria-label = 메시지 전송
assistant-chat-send-empty =
    .aria-label = 전송할 메시지를 입력하세요
assistant-chat-stop =
    .title = 응답 중지 (Esc)
    .aria-label = 응답 중지
assistant-chat-cancelled-title = 취소됨
assistant-chat-cancelled-message = 요청이 취소되었습니다
assistant-chat-message-too-long-title = 메시지가 너무 깁니다
assistant-chat-message-too-long-message = 메시지를 { $limit }자 이내로 줄여 주세요
assistant-chat-start-failed = 채팅 세션을 시작하지 못했습니다
assistant-chat-send-failed = 어시스턴트가 응답을 완료하지 못했습니다. 메시지를 다시 보낼 수 있도록 준비되어 있습니다.

# chat_widget.js: messages and tool calls.
assistant-chat-role-user = 나
assistant-chat-role-assistant = 어시스턴트
assistant-chat-message-actions =
    .aria-label = 메시지 작업
assistant-chat-message-copy =
    .title = 복사
    .aria-label = 메시지 복사
assistant-chat-message-regenerate =
    .title = 다시 생성
    .aria-label = 응답 다시 생성
assistant-chat-copied-message = 메시지
assistant-chat-copy-failed = 메시지를 복사하지 못했습니다
assistant-chat-regenerate-none = 다시 생성할 메시지가 없습니다
assistant-chat-regenerate-reload = 이 응답을 다시 생성하려면 채팅을 새로 불러오세요
assistant-chat-regenerate-done-title = 다시 생성됨
assistant-chat-regenerate-done-message = 응답이 다시 생성되었습니다
assistant-chat-regenerate-failed = 다시 생성하지 못했습니다
assistant-chat-regenerate-failed-toast = 응답을 다시 생성하지 못했습니다

# Status of a tool call.
assistant-chat-tool-status-executed = 실행됨
assistant-chat-tool-status-failed = 실패
assistant-chat-tool-status-denied = 거부됨
assistant-chat-tool-status-pending-confirmation = 확인 대기 중
assistant-chat-tool-status-pending = 대기 중
assistant-chat-tool-section-input = 입력:
assistant-chat-tool-section-output = 출력:
assistant-chat-tool-section-error = 오류:

# chat_widget.js: progress while a response is generated.
assistant-chat-typing =
    .aria-label = 어시스턴트가 생각하는 중
assistant-chat-progress-preparing = 응답 준비 중
assistant-chat-progress-planning = 응답 계획 중
assistant-chat-progress-reviewing = 도구 결과 검토 중
assistant-chat-progress-using-tools = 도구 사용 중
assistant-chat-progress-running = 실행 중
assistant-chat-progress-failed = 실패
assistant-chat-progress-complete = 완료
assistant-chat-error-unknown = 알 수 없는 오류
assistant-chat-stream-http = API 오류: { $status }
assistant-chat-stream-unavailable = 어시스턴트 진행 스트림을 사용할 수 없습니다
assistant-chat-stream-failed = 어시스턴트 요청에 실패했습니다
assistant-chat-stream-incomplete = 어시스턴트 응답이 완료되기 전에 종료되었습니다

# chat_widget.js: tool approval and sessions.
assistant-chat-tool-review = 입력 검토
assistant-chat-tool-deny = 거부
assistant-chat-tool-allow = 허용
assistant-chat-tool-unknown = 알 수 없는 도구
assistant-chat-tool-default-description = 이 도구를 실행하려면 승인이 필요합니다.
assistant-chat-tool-executed = 도구가 실행되었습니다
assistant-chat-tool-cancelled = 도구 실행이 취소되었습니다
assistant-chat-tool-confirm-failed = 도구 실행을 확인하지 못했습니다
assistant-chat-sessions-load-failed = 채팅 세션을 불러올 수 없습니다
assistant-chat-delete-title = 채팅 세션 삭제
assistant-chat-delete-message = 이 채팅 세션을 삭제하시겠습니까? 이 작업은 되돌릴 수 없습니다.
assistant-chat-delete-done = 채팅 세션이 삭제되었습니다
assistant-chat-delete-failed = 채팅 세션을 삭제하지 못했습니다

# Agent tool labels.
assistant-tool-analyze-token = 토큰 분석
assistant-tool-get-market-data = 시장 데이터 조회
assistant-tool-check-security = 보안 확인
assistant-tool-get-positions = 포지션 조회
assistant-tool-get-position = 포지션 상세 조회
assistant-tool-get-balance = 잔액 조회
assistant-tool-get-pnl = 손익 조회
assistant-tool-buy-token = 토큰 매수
assistant-tool-add-to-position = 포지션 추가 매수
assistant-tool-sell-token = 토큰 매도
assistant-tool-close-position = 포지션 종료
assistant-tool-get-config = 설정 조회
assistant-tool-describe-config = 설정 설명
assistant-tool-update-config = 설정 업데이트
assistant-tool-get-status = 상태 조회
assistant-tool-get-events = 이벤트 조회
assistant-tool-force-stop = 강제 중지
assistant-tool-clear-force-stop = 강제 중지 해제
assistant-tool-get-trader-status = 트레이더 상태 조회
assistant-tool-get-trader-stats = 트레이더 통계 조회
assistant-tool-set-trader-enabled = 트레이더 사용 설정
assistant-tool-set-trader-monitor = 트레이더 모니터 설정
assistant-tool-manage-loss-limit = 손실 한도 관리
assistant-tool-list-trader-templates = 트레이더 템플릿 목록
assistant-tool-apply-trader-template = 트레이더 템플릿 적용
assistant-tool-get-copy-trading-overview = 카피 트레이딩 개요 조회
assistant-tool-get-copy-task = 카피 작업 조회
assistant-tool-get-copy-activity = 카피 활동 조회
assistant-tool-create-copy-task = 카피 작업 생성
assistant-tool-update-copy-task = 카피 작업 업데이트
assistant-tool-delete-copy-task = 카피 작업 삭제
assistant-tool-set-copy-task-mode = 카피 작업 모드 설정
assistant-tool-get-copy-insights = 카피 인사이트 조회
assistant-tool-get-copy-wallet-profile = 카피 지갑 프로필 조회
assistant-tool-clone-copy-task = 카피 작업 복제
assistant-tool-reset-copy-paper-book = 카피 모의 장부 초기화
assistant-tool-close-copy-paper-holding = 카피 모의 보유 수량 종료

# Built-in instruction templates.
assistant-template-liquidity-guard-name = 유동성 가드
assistant-template-liquidity-guard-description = 유동성이 얇아 슬리피지 위험이 높고 청산이 어려운 토큰을 제외합니다.
assistant-template-holder-distribution-name = 홀더 분포 확인
assistant-template-holder-distribution-description = 상위 홀더가 공급량의 큰 비중을 차지하는 토큰을 표시합니다.
assistant-template-honeypot-detection-name = 허니팟 탐지
assistant-template-honeypot-detection-description = 동결 또는 민트 권한이 활성화되었거나 비정상적인 전송 제한이 있는 토큰을 제외합니다.
assistant-template-momentum-filter-name = 모멘텀 필터
assistant-template-momentum-filter-description = 거래량 증가로 확인되는 긍정적인 가격 모멘텀을 가진 토큰을 우선합니다.
assistant-template-new-token-caution-name = 신규 토큰 주의
assistant-template-new-token-caution-description = 생성된 지 하루가 안 된 토큰에는 더 높은 신뢰도를 요구합니다.
assistant-template-whale-activity-name = 고래 활동 모니터
assistant-template-whale-activity-description = 대형 홀더의 이동과 비정상적인 배포자 또는 초기 지갑 활동을 감시합니다.

# Template tags.
assistant-template-tag-activity = 활동
assistant-template-tag-age = 경과 기간
assistant-template-tag-authority = 권한
assistant-template-tag-caution = 주의
assistant-template-tag-distribution = 분포
assistant-template-tag-holders = 홀더
assistant-template-tag-honeypot = 허니팟
assistant-template-tag-large-holders = 대형 홀더
assistant-template-tag-liquidity = 유동성
assistant-template-tag-momentum = 모멘텀
assistant-template-tag-new-tokens = 신규 토큰
assistant-template-tag-price-action = 가격 움직임
assistant-template-tag-risk-management = 위험 관리
assistant-template-tag-rug-risk = 러그풀 위험
assistant-template-tag-safety = 안전
assistant-template-tag-security = 보안
assistant-template-tag-volume = 거래량
assistant-template-tag-whales = 고래

# Shared by the Assistant tab dialogs.
assistant-modal-close =
    .aria-label = 닫기

# providers_tab.js: provider list.
assistant-providers-load-failed = 제공자를 불러올 수 없습니다
assistant-providers-select-default =
    .title = 기본값으로 설정
assistant-providers-use-default =
    .aria-label = 기본 제공자로 사용: { $name }
assistant-providers-model-none = 설정되지 않음
assistant-providers-status-ready = 준비됨
assistant-providers-status-not-set-up = 설정 안 됨
assistant-providers-status-default = 기본
assistant-providers-test = 테스트
assistant-providers-configure = 설정
assistant-providers-default-set-title = 기본 제공자 설정됨
assistant-providers-default-set-message = 기본 제공자: { $name }
assistant-providers-default-set-failed = 기본 제공자를 설정하지 못했습니다
assistant-providers-testing-title = 제공자 테스트
assistant-providers-testing-message = 테스트 중: { $name }...
assistant-providers-test-http = HTTP { $status }
assistant-providers-test-success-title = 연결 성공
assistant-providers-test-success-message = 정상 작동 중: { $name }
assistant-providers-test-failed = 테스트 실패

# providers_tab.js: configuration dialog.
assistant-providers-config-title = { $name } 설정
assistant-providers-api-key = API 키
assistant-providers-key-saved = 키 저장됨
assistant-providers-key-missing = 키가 설정되지 않음
assistant-providers-api-key-update =
    .placeholder = 업데이트할 새 키 입력...
assistant-providers-api-key-enter =
    .placeholder = API 키 입력...
assistant-providers-key-toggle =
    .title = 표시/숨기기
assistant-providers-key-help-saved = 현재 키를 유지하려면 비워 두고, 업데이트하려면 새 키를 입력하세요
assistant-providers-key-help-new = API 키는 안전하게 저장되며 공유되지 않습니다
assistant-providers-model = 모델
assistant-providers-model-input =
    .placeholder = 예: gpt-4, claude-3-opus...
assistant-providers-model-help = 어시스턴트 분석 요청에 사용할 모델
assistant-providers-enable = 이 제공자 사용
assistant-providers-enable-help = 활성화하면 이 제공자를 어시스턴트 분석에 사용할 수 있습니다
assistant-providers-connection-test = 연결 테스트
assistant-providers-test-connection = 연결 테스트
assistant-providers-testing = 테스트 중...
assistant-providers-save = 설정 저장
assistant-providers-saving = 저장 중...
assistant-providers-missing-key-title = API 키 없음
assistant-providers-missing-key-test = 먼저 API 키를 입력하세요
assistant-providers-missing-key-enable = 이 제공자를 사용하려면 API 키를 입력하세요
assistant-providers-missing-model-title = 모델 없음
assistant-providers-missing-model-message = 모델 이름을 입력하세요
assistant-providers-test-save-failed = 테스트를 위해 설정을 저장하지 못했습니다
assistant-providers-test-connected = 연결 성공!
assistant-providers-detail-model = 모델:
assistant-providers-detail-none = 해당 없음
assistant-providers-detail-latency = 지연 시간:
assistant-providers-detail-tokens = 토큰:
assistant-providers-saved-title = 제공자 저장됨
assistant-providers-saved-message = 설정이 저장되었습니다: { $name }
assistant-providers-save-failed = 제공자 설정을 저장하지 못했습니다

# instructions_tab.js: list, templates and dialogs.
assistant-instructions-load-failed = 지침을 불러오지 못했습니다
assistant-instructions-priority = 우선순위: { $position }
assistant-instructions-actions =
    .aria-label = 지침 작업
assistant-instructions-hint-filtering = 토큰 필터링 결정을 위한 지침 - LLM 분석이 건너뛸 토큰을 판단하는 데 도움이 됩니다
assistant-instructions-hint-trading = 진입/청산 분석을 위한 지침 - 모델 점수 기반 거래 결정을 안내합니다
assistant-instructions-hint-analysis = 모델 점수 기반 결정을 위한 일반 시장 분석 가이드라인
assistant-instructions-hint-general = 모델 기반 동작을 위한 기타 지침
assistant-instructions-char-count =
    { $count ->
       *[other] { $amount }자
    }
assistant-instructions-reordered-title = 순서 변경됨
assistant-instructions-reordered-message = 지침 순서가 변경되었습니다
assistant-instructions-reorder-failed = 지침 순서를 변경하지 못했습니다
assistant-templates-empty = 사용 가능한 템플릿이 없습니다
assistant-templates-preview-title = 템플릿 미리보기: { $name }
assistant-templates-preview-content = 내용:
assistant-templates-customize-add = 사용자 지정 후 추가
assistant-templates-customize-title = 템플릿 사용자 지정
assistant-instructions-field-name = 이름
assistant-instructions-field-category = 카테고리
assistant-instructions-field-content = 내용
assistant-instructions-name-input =
    .placeholder = 예: 유동성 가드
assistant-instructions-content-input =
    .placeholder = 지침을 입력하세요...
assistant-instructions-create = 생성
assistant-instructions-create-title = 지침 생성
assistant-instructions-missing-title = 필수 항목 누락
assistant-instructions-missing-message = 이름과 내용은 필수입니다
assistant-instructions-created-title = 생성됨
assistant-instructions-created-message = 지침이 생성되었습니다
assistant-instructions-created-from-template = 템플릿에서 지침이 생성되었습니다: { $name }
assistant-instructions-create-failed = 지침을 생성하지 못했습니다
assistant-instructions-create-from-template-failed = 템플릿에서 지침을 생성하지 못했습니다
assistant-instructions-toggle-failed = 지침을 전환하지 못했습니다
assistant-instructions-edit-title = 지침 편집
assistant-instructions-preview = 미리보기
assistant-instructions-save-changes = 변경 사항 저장
assistant-instructions-untitled = 제목 없음
assistant-instructions-load-item-failed = 지침 데이터를 불러오지 못했습니다
assistant-instructions-updated-title = 업데이트됨
assistant-instructions-updated-message = 지침이 업데이트되었습니다
assistant-instructions-update-failed = 지침을 업데이트하지 못했습니다
assistant-instructions-delete-title = 지침 삭제
assistant-instructions-delete-message = 이 지침을 삭제하시겠습니까?
assistant-instructions-deleted-title = 삭제됨
assistant-instructions-deleted-message = 지침이 삭제되었습니다
assistant-instructions-delete-failed = 지침을 삭제하지 못했습니다
assistant-instructions-copy-name = { $name } (사본)
assistant-instructions-duplicated-title = 복제됨
assistant-instructions-duplicated-message = 지침이 복제되었습니다
assistant-instructions-duplicate-failed = 지침을 복제하지 못했습니다

# automation_tab.js: task list, runs and dialogs.
assistant-automation-schedule-type-interval = 간격
assistant-automation-schedule-type-daily = 매일
assistant-automation-schedule-type-weekly = 매주
assistant-automation-permission-read-only = 읽기 전용
assistant-automation-permission-full = 전체 권한
assistant-automation-permission-option-read-only = 읽기 전용 (안전)
assistant-automation-permission-option-full = 전체 권한 (거래 가능)
assistant-automation-run-status-running = 실행 중
assistant-automation-run-status-success = 성공
assistant-automation-run-status-failed = 실패
assistant-automation-run-status-timeout = 시간 초과
assistant-automation-run-status-skipped = 건너뜀
assistant-automation-hint-interval = 초 단위 간격 (예: 300 = 5분마다)
assistant-automation-hint-daily = HH:MM UTC 형식의 시간 (예: 14:00)
assistant-automation-hint-weekly = 요일과 시간: mon,wed,fri:09:00
assistant-automation-schedule-every = { $span }마다
assistant-automation-schedule-daily = 매일 { $time } UTC
assistant-automation-schedule-weekly = { $days } { $time } UTC
assistant-automation-schedule-day-separator = { ", " }
assistant-automation-task-active = 활성
assistant-automation-task-paused = 일시 중지됨
assistant-automation-never = 없음
assistant-automation-last-run = 마지막: { $when }
assistant-automation-next-run = 다음: { $when }
assistant-automation-run-now =
    .title = 지금 실행
    .aria-label = 지금 실행
assistant-automation-actions =
    .aria-label = 자동화 작업
assistant-automation-runs-count =
    { $count ->
       *[other] { $amount }회 실행
    }
assistant-automation-runs-empty = 실행 기록이 아직 없습니다
assistant-automation-task-runs-empty = 이 작업의 실행 기록이 아직 없습니다
assistant-automation-task-fallback = 작업 #{ $id }
assistant-automation-task-generic = 작업
assistant-automation-create-title = 자동화 작업 생성
assistant-automation-edit-title = 작업 편집
assistant-automation-field-name = 작업 이름
assistant-automation-name-input =
    .placeholder = 예: 포트폴리오 모니터
assistant-automation-field-instruction = 지침
assistant-automation-instruction-input =
    .placeholder = 어시스턴트가 무엇을 해야 하나요? 예: 보유 포지션의 반전 징후를 확인하고 결과를 보고하세요.
assistant-automation-field-schedule-type = 일정 유형
assistant-automation-field-schedule-value = 일정 값
assistant-automation-field-permissions = 도구 권한
assistant-automation-field-timeout = 제한 시간 (초)
assistant-automation-notify-telegram = { -telegram } 알림
assistant-automation-notify-success = 성공 시 알림
assistant-automation-notify-failure = 실패 시 알림
assistant-automation-create-task = 작업 생성
assistant-automation-save-changes = 변경 사항 저장
assistant-automation-validation-title = 유효성 검사
assistant-automation-validation-required = 모든 필수 항목을 입력하세요
assistant-automation-validation-interval = 간격은 최소 60초여야 합니다
assistant-automation-validation-daily = 매일 일정은 HH:MM 형식이어야 합니다
assistant-automation-validation-weekly = 매주 일정은 mon,wed,fri:09:00 형식이어야 합니다
assistant-automation-created = 작업이 생성되었습니다
assistant-automation-create-failed = 작업을 생성하지 못했습니다
assistant-automation-updated = 작업이 업데이트되었습니다
assistant-automation-update-failed = 작업을 업데이트하지 못했습니다
assistant-automation-toggle-failed = 작업을 전환하지 못했습니다
assistant-automation-triggered = 작업이 실행되었습니다
assistant-automation-trigger-failed = 작업을 실행하지 못했습니다
assistant-automation-delete-title = 작업 삭제
assistant-automation-delete-message = 이 자동화 작업을 삭제하시겠습니까? 이 작업은 되돌릴 수 없습니다.
assistant-automation-deleted = 작업이 삭제되었습니다
assistant-automation-delete-failed = 작업을 삭제하지 못했습니다
assistant-automation-view-runs = 실행 기록 보기
assistant-automation-runs-load-failed = 실행 기록을 불러오지 못했습니다
assistant-automation-runs-history-title = 실행 기록 — { $task }
assistant-automation-run-load-failed = 실행 세부 정보를 불러오지 못했습니다
assistant-automation-run-details-title = 실행 세부 정보
assistant-automation-run-task = 작업
assistant-automation-run-status = 상태
assistant-automation-run-started = 시작 시각
assistant-automation-run-duration = 소요 시간
assistant-automation-run-provider = 제공자
assistant-automation-run-tokens = 토큰
assistant-automation-run-tools-title = 도구 호출 ({ $amount })
assistant-automation-run-response = 어시스턴트 응답
