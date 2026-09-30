# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = 전체
strategies-filter-entry = 진입
strategies-filter-exit = 청산
strategies-type-entry = 진입
strategies-type-exit = 청산
strategies-list-empty-title = 아직 전략이 없습니다
strategies-list-empty-hint = 첫 전략을 만들어 보세요
strategies-new = 새 전략
strategies-import =
    .title = 전략 가져오기
    .aria-label = 전략 가져오기
strategies-item-enable =
    .title = 사용
strategies-item-disable =
    .title = 사용 안 함

# Name given to a strategy before it is saved.
strategies-new-name = 새 전략

## Editor

strategies-editor-name =
    .placeholder = 전략 이름
strategies-editor-dirty =
    .title = 저장되지 않은 변경 사항
strategies-action-validate = 검증
strategies-editor-empty = 편집할 전략을 선택하거나 새로 만드세요
strategies-conditions-empty-title = 아직 조건이 없습니다
strategies-conditions-empty-hint = "{ strategies-add-condition }"을 눌러 구성을 시작하세요
strategies-add-condition = 조건 추가
strategies-modal-close =
    .aria-label = 닫기
strategies-card-move-up =
    .title = 위로 이동
strategies-card-move-down =
    .title = 아래로 이동
strategies-card-duplicate =
    .title = 복제
strategies-card-delete =
    .title = 삭제

# Card summary: up to three "label: value" entries.
strategies-summary-param = { $label }: { $value }
strategies-summary-parts =
    { $count ->
        [1] { $first }
        [2] { $first }, { $second }
       *[3] { $first }, { $second }, { $third }
    }
strategies-summary-none = 매개변수 없음
strategies-summary-period-seconds = 기간: { $amount }초
strategies-summary-period-minutes = 기간: { $amount }분
strategies-summary-period-hours = 기간: { $amount }시간

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
       *[other] { $amount }시간
    }
strategies-value-candles =
    { $count ->
       *[other] 캔들 { $amount }개
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-sol = { -sol }
strategies-unit-hours = 시간
strategies-unit-multiplier = ×

## Condition catalog

strategies-catalog-search =
    .placeholder = 조건 검색...
strategies-catalog-search-clear =
    .aria-label = 검색 지우기
strategies-catalog-fold-all = 모두 접기
strategies-catalog-unfold-all = 모두 펼치기
strategies-catalog-no-description = 설명이 없습니다

## New strategy dialog

strategies-create-title = 새 전략 만들기
strategies-create-prompt = 만들려는 전략 유형을 선택하세요:
strategies-create-entry-name = 진입 전략
strategies-create-entry-description = 토큰을 매수할 시점의 조건을 정의합니다
strategies-create-exit-name = 청산 전략
strategies-create-exit-description = 토큰을 매도할 시점의 조건을 정의합니다

## Delete dialog

strategies-delete-title = 전략 삭제
# $name is the strategy name.
strategies-delete-message = 전략 "{ $name }"을 삭제하시겠습니까? 이 작업은 되돌릴 수 없습니다.

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = 저장하기 전에 검증 오류를 수정하세요
strategies-toast-enabled = 전략 사용 설정됨
    .message = "{ $name }" 사용 설정됨
strategies-toast-disabled = 전략 사용 안 함
    .message = "{ $name }" 사용 안 함
strategies-toast-toggle-failed = 전환 실패
    .message = 전략 상태를 업데이트하지 못했습니다
strategies-toast-load-failed = 불러오기 실패
    .message = 서버에서 전략을 불러오지 못했습니다
strategies-toast-created = 새 전략
    .message =
        { $type ->
            [EXIT] 새 청산 전략을 만들었습니다
           *[ENTRY] 새 진입 전략을 만들었습니다
        }
strategies-toast-load-strategy-failed = 전략을 불러오지 못했습니다
strategies-toast-no-strategy = 생성된 전략 없음
    .message = 조건을 하나 이상 추가하거나 '새 전략'을 눌러 먼저 전략을 만드세요
strategies-toast-no-conditions-save = 조건 없음
    .message = 저장하기 전에 전략에 조건을 하나 이상 추가하세요
strategies-toast-name-required = 이름 필요
    .message = 저장하기 전에 전략 이름을 입력하세요
strategies-toast-saved = 전략 저장됨
    .message = "{ $name }" 저장 완료
strategies-toast-save-failed = 저장 실패
    .message = 전략을 데이터베이스에 저장하지 못했습니다
strategies-toast-no-strategy-validate = 검증할 전략이 없습니다
strategies-toast-no-conditions-validate = 조건 없음
    .message = 검증하기 전에 조건을 하나 이상 추가하세요
strategies-toast-valid = 전략이 유효합니다
strategies-toast-invalid = 전략에 오류가 있습니다
strategies-toast-validation-failed = 검증에 실패했습니다
strategies-toast-item-enabled = 전략을 사용 설정했습니다
strategies-toast-item-disabled = 전략을 사용 안 함으로 설정했습니다
strategies-toast-item-toggle-failed = 전략을 전환하지 못했습니다
strategies-toast-deleted = 전략 삭제됨
    .message = "{ $name }" 삭제 완료
strategies-toast-delete-failed = 삭제 실패
    .message = 데이터베이스에서 전략을 삭제하지 못했습니다
strategies-toast-imported = 전략을 가져왔습니다
strategies-toast-import-failed = 전략을 가져오지 못했습니다
strategies-toast-unknown-condition = 알 수 없는 조건
    .message = 조건 유형을 찾을 수 없습니다
strategies-toast-create-first = 먼저 전략 만들기
    .message = 조건을 추가하기 전에 '새 전략'을 눌러 전략을 만드세요
strategies-toast-condition-added = 조건 추가됨
    .message = 전략에 { $name } 추가됨


## Conditions

strategies-condition-candle-size = 캔들 크기 패턴
    .description = 특정 캔들 패턴을 감지합니다: 큰 몸통, 작은 몸통(도지), 긴 꼬리
strategies-condition-candle-size-param-pattern = 패턴 유형
    .description = 감지할 캔들 패턴
strategies-condition-candle-size-param-pattern-option-large-body = 큰 몸통 (강한 움직임)
strategies-condition-candle-size-param-pattern-option-small-body = 작은 몸통 (도지/불확실)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 긴 윗꼬리 (저항)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 긴 아랫꼬리 (지지)
strategies-condition-candle-size-param-threshold = 크기 임계값 %
    .description = 패턴 감지 기준 비율

strategies-condition-consecutive-candles = 연속 캔들
    .description = 최소 크기 필터와 함께 연속된 녹색(상승) 또는 적색(하락) 캔들을 감지합니다
strategies-condition-consecutive-candles-param-count = 캔들 수
    .description = 필요한 연속 캔들 수
strategies-condition-consecutive-candles-param-direction = 캔들 방향
    .description = 연속 캔들의 색상/방향
strategies-condition-consecutive-candles-param-direction-option-green = 녹색 (상승)
strategies-condition-consecutive-candles-param-direction-option-red = 적색 (하락)
strategies-condition-consecutive-candles-param-minimum-change = 최소 변동 %
    .description = 각 캔들의 최소 변동률 (노이즈 제거)

strategies-condition-liquidity-level = 풀 유동성 수준
    .description = { -sol } 기준 풀 유동성을 확인합니다 (진입: 충분한 유동성 확보, 청산: 유동성 유출 감지)
strategies-condition-liquidity-level-param-threshold = 유동성 임계값 ({ -sol })
    .description = { -sol } 기준 풀 유동성 수준
strategies-condition-liquidity-level-param-comparison = 비교
    .description = 풀 유동성을 임계값과 비교하는 방식
strategies-condition-liquidity-level-param-comparison-option-greater-than = 초과 (>)
strategies-condition-liquidity-level-param-comparison-option-greater-equal = 이상 (≥)
strategies-condition-liquidity-level-param-comparison-option-less-than = 미만 ({ "<" })
strategies-condition-liquidity-level-param-comparison-option-less-equal = 이하 (≤)

strategies-condition-position-holding-time = 포지션 보유 시간
    .description = 포지션을 보유한 기간을 확인합니다 (청산 전략용 - 시간 기반 청산)
strategies-condition-position-holding-time-param-hours = 시간 임계값 (시간)
    .description = 포지션을 연 이후 경과 시간(시간 단위)
strategies-condition-position-holding-time-param-comparison = 비교
    .description = 포지션 경과 시간을 임계값과 비교하는 방식
strategies-condition-position-holding-time-param-comparison-option-greater-than = 초과 (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 이상 (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = 미만 ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = 이하 (≤)

strategies-condition-price-breakout = 가격 돌파
    .description = 가격이 저항선(기간 고점)을 위로, 또는 지지선(기간 저점)을 아래로 돌파하는 것을 감지합니다
strategies-condition-price-breakout-param-lookback = 조회 기간
    .description = 지지/저항 수준을 찾는 데 사용할 캔들 수
strategies-condition-price-breakout-param-direction = 돌파 방향
    .description = 돌파 방향
strategies-condition-price-breakout-param-direction-option-upward = 상향 (저항 돌파)
strategies-condition-price-breakout-param-direction-option-downward = 하향 (지지 붕괴)
strategies-condition-price-breakout-param-confirmation = 확인 %
    .description = 돌파를 확인하기 위해 해당 수준을 얼마나 넘어야 하는지 (거짓 시그널 방지)

strategies-condition-price-change-percent = 가격 변동 %
    .description = 일정 기간 내 가격이 임계 비율만큼 변동했는지 확인합니다
strategies-condition-price-change-percent-param-percentage = 변동 임계값 %
    .description = 발동할 가격 변동률 (0.1-1000%)
strategies-condition-price-change-percent-param-direction = 방향
    .description = 가격 움직임 방향
strategies-condition-price-change-percent-param-direction-option-above = 상승 (+%)
strategies-condition-price-change-percent-param-direction-option-below = 하락 (-%)
strategies-condition-price-change-percent-param-direction-option-within = 범위 내 (±%)
strategies-condition-price-change-percent-param-time-value = 기간
    .description = 조회 기간 값 (초 1-3600, 분 1-1440, 시간 1-720)
strategies-condition-price-change-percent-param-time-unit = 시간 단위
    .description = 조회 기간의 시간 단위
strategies-condition-price-change-percent-param-time-unit-option-seconds = 초
strategies-condition-price-change-percent-param-time-unit-option-minutes = 분
strategies-condition-price-change-percent-param-time-unit-option-hours = 시간

strategies-condition-price-to-ma = 가격 대 이동평균
    .description = 가격이 단순 이동평균 위, 아래 또는 범위 내에 있는지 확인합니다
strategies-condition-price-to-ma-param-period = MA 기간
    .description = 이동평균 계산에 사용할 캔들 수
strategies-condition-price-to-ma-param-position = 위치
    .description = MA 대비 가격 위치
strategies-condition-price-to-ma-param-position-option-above = MA 위
strategies-condition-price-to-ma-param-position-option-below = MA 아래
strategies-condition-price-to-ma-param-position-option-within = 범위 내
strategies-condition-price-to-ma-param-distance = 거리 %
    .description = MA와의 최소 거리 (위/아래) 또는 최대 범위 (범위 내)

strategies-condition-volume-spike = 거래량 급증
    .description = 평균 거래량 대비 거래량 급증을 감지합니다 (관심 증가를 의미)
strategies-condition-volume-spike-param-lookback = 조회 기간
    .description = 평균 거래량을 계산할 캔들 수
strategies-condition-volume-spike-param-multiplier = 거래량 배수
    .description = 평균의 몇 배인지 (예: 2.0 = 평균의 200%)

## Shared by every condition

strategies-condition-param-timeframe = 타임프레임
    .description = 분석할 캔들 타임프레임 (설정하지 않으면 전략 타임프레임 사용)
strategies-condition-timeframe-option-1m = 1분
strategies-condition-timeframe-option-5m = 5분
strategies-condition-timeframe-option-15m = 15분
strategies-condition-timeframe-option-1h = 1시간
strategies-condition-timeframe-option-4h = 4시간
strategies-condition-timeframe-option-12h = 12시간
strategies-condition-timeframe-option-1d = 1일

## Condition categories

strategies-condition-category-price-analysis = 가격 분석
strategies-condition-category-candle-patterns = 캔들 패턴
strategies-condition-category-technical-indicators = 기술 지표
strategies-condition-category-market-context = 시장 상황
strategies-condition-category-position-performance = 포지션 및 성과
strategies-condition-category-volume-analysis = 거래량 분석

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = 누락된 매개변수: { $field }
strategies-error-parameter-type = 매개변수 { $field }의 형식이 올바르지 않습니다. 필요한 형식: { $expected }
strategies-error-invalid-value = { $field }에 유효하지 않은 값: "{ $value }"
strategies-error-missing-data = 사용할 수 없는 데이터: { $data }
strategies-error-no-candle-data = 타임프레임 { $timeframe }에 캔들 데이터가 없습니다
strategies-error-insufficient-history = { $indicator } 기록 부족: 사용 가능 { $available }초, 필요 { $required }초
strategies-error-insufficient-candles = { $indicator } 캔들 부족: 보유 { $available }, 필요 { $required }
strategies-error-stale-candle-data = 타임프레임 { $timeframe }의 캔들 데이터가 오래되었습니다: 경과 시간 { $age }초가 { $max }초를 초과합니다
strategies-error-invalid-rule-tree = 규칙 트리가 올바르지 않습니다: { $reason }
strategies-error-evaluation-timeout = { $timeout }ms 후 전략 평가가 시간 초과되었습니다
strategies-error-invalid-rules = 규칙을 읽을 수 없습니다: { $reason }

# Parameter names

strategies-error-field-average-volume = 평균 거래량
strategies-error-field-candle-open = 캔들 시가
strategies-error-field-comparison = 비교
strategies-error-field-condition-type = 조건 유형
strategies-error-field-confirmation = 확인
strategies-error-field-count = 수
strategies-error-field-current-price = 현재가
strategies-error-field-direction = 방향
strategies-error-field-distance = 거리
strategies-error-field-hours = 시간
strategies-error-field-lookback = 조회 기간
strategies-error-field-minimum-change = 최소 변동
strategies-error-field-multiplier = 배수
strategies-error-field-pattern = 패턴
strategies-error-field-percentage = 비율
strategies-error-field-period = 기간
strategies-error-field-position = 위치
strategies-error-field-threshold = 임계값
strategies-error-field-time-unit = 시간 단위
strategies-error-field-time-value = 시간 값
strategies-error-field-timeframe = 타임프레임

# Expected parameter types

strategies-error-expected-boolean = 불리언
strategies-error-expected-number = 숫자
strategies-error-expected-string = 문자열

# Missing context data

strategies-error-data-current-price = 현재가
strategies-error-data-liquidity-data = 유동성 데이터
strategies-error-data-market-data = 시장 데이터
strategies-error-data-ohlcv-data = OHLCV 데이터
strategies-error-data-position-data = 포지션 데이터

# Indicators

strategies-error-indicator-consecutive-candles = 연속 캔들
strategies-error-indicator-moving-average = 이동평균
strategies-error-indicator-price-breakout = 가격 돌파
strategies-error-indicator-price-change-lookback = 가격 변동 조회
strategies-error-indicator-volume-spike = 거래량 급증

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = 분기 노드에 조건이 없습니다
strategies-error-rule-branch-node-missing-operator = 분기 노드에 연산자가 없습니다
strategies-error-rule-branch-node-must-have-at-least-one-child = 분기 노드에는 하위 노드가 하나 이상 있어야 합니다
strategies-error-rule-invalid-rule-tree-structure = 규칙 트리 구조가 올바르지 않습니다
strategies-error-rule-leaf-node-missing-condition = 리프 노드에 조건이 없습니다
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 연산자에는 하위 노드가 정확히 하나 있어야 합니다
