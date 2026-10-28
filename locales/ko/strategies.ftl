# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-type-entry = 진입
strategies-type-exit = 청산
strategies-list-empty-title = 아직 전략이 없습니다
strategies-list-empty-hint = 첫 전략을 만들어 보세요
strategies-new = 새 전략
strategies-import =
    .title = 전략 가져오기
    .aria-label = 전략 가져오기

# Name given to a strategy before it is saved.
strategies-new-name = 새 전략

## Editor

strategies-editor-name =
    .placeholder = 전략 이름
strategies-editor-dirty =
    .title = 저장되지 않은 변경 사항
strategies-action-validate = 검증
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
# $name is the condition name.
strategies-card-delete-confirm = 조건 제거
    .message = 이 전략에서 "{ $name }"을(를) 제거할까요?

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = 매개변수 없음
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = 전략 설정 ({ $value })
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount }초
strategies-value-duration-minutes = { $amount }분
strategies-value-duration-hours = { $amount }시간
strategies-value-duration-days = { $amount }일
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = 현지 시간: { $time }
strategies-hint-price-impact = 이 크기의 거래는 가격을 약 { $amount }% 움직입니다

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
strategies-unit-native = { -sol }
strategies-unit-hours = 시간
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = 조건 검색...
    .aria-label = 조건 검색
strategies-catalog-search-clear =
    .aria-label = 검색 지우기
strategies-catalog-no-description = 설명이 없습니다

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = 켜짐
strategies-editor-timeframe = 기본 타임프레임
    .title = 자체 타임프레임이 없는 조건이 사용하는 캔들 타임프레임
strategies-editor-order = 순서
    .title = 1부터 100까지의 평가 순서이며, 작은 숫자가 먼저 평가됩니다
strategies-editor-description = 설명
strategies-editor-description-field =
    .placeholder = 이 전략이 하는 일
    .aria-label = 전략 설명
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = 모든 진입 전략
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = 관리 대상
strategies-editor-entry-binding-hint = 선택한 진입 전략이 연 포지션을 관리합니다. 아무것도 선택하지 않으면 수동 및 복사 포지션을 포함한 모든 포지션을 관리합니다.
strategies-editor-entry-binding-empty = 아직 진입 전략이 없습니다
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id } (삭제됨)
strategies-action-export = 내보내기
strategies-action-test = 토큰으로 테스트
# $token is the tested token's symbol or address.
strategies-test-title = { $token } 테스트
# $token is the tested token's symbol or address.
strategies-test-running = { $token }(으)로 테스트 중…
strategies-test-verdict-pass = 충족
strategies-test-verdict-fail = 미충족
strategies-test-verdict-unknown = 알 수 없음

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = 다음 조건 중 { $mode } 충족
strategies-group-match = 다음 중 { $mode } 충족
strategies-match-all = 모두
strategies-match-any = 하나 이상
strategies-match-mode =
    .aria-label = 모든 조건 또는 하나 이상의 조건
strategies-invert = 반전
    .title = 반전: 이 조건이 충족되지 않을 때 참이 됩니다
strategies-chip-off = 꺼짐
strategies-chip-inverted = 반전됨
strategies-row-switch =
    .title = 저장은 유지하되 평가하지 않으려면 끄세요
    .aria-label = 이 전략에서 평가됨
strategies-row-expand =
    .aria-label = 매개변수 표시 또는 숨기기
strategies-add-group = 그룹 추가
# $count is the number of conditions in the group.
strategies-group-delete-confirm = 그룹 제거
    .message = 이 그룹과 그 안의 조건 { $count }개를 제거할까요?

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems = 해결할 문제 { $count }개: { $first }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }, { $field }: { $problem }
strategies-problem-rules-empty = 켜진 조건을 하나 이상 추가하세요
strategies-problem-group-empty = 그룹에는 켜진 조건이 하나 이상 필요합니다
strategies-problem-not-applicable = 이 조건은 이 유형의 전략에서 사용할 수 없습니다
strategies-problem-unknown-condition = 알 수 없는 조건 유형: { $type }
strategies-problem-too-deep = 규칙은 최대 { $max }단계까지 중첩할 수 있으며, 반전된 그룹은 한 단계를 더 차지합니다
strategies-problem-required = 숫자를 입력하세요
strategies-problem-range = { $min }에서 { $max } 사이여야 합니다
strategies-problem-invalid-option = 목록의 옵션 중 하나를 선택하세요

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = 전체
strategies-catalog-categories =
    .aria-label = 카테고리
strategies-catalog-empty = 검색과 일치하는 조건이 없습니다
strategies-catalog-exit-only = 청산 전략 전용
strategies-catalog-entry-only = 진입 전략 전용
strategies-catalog-candles-note = { $description } 캔들만 읽습니다.
strategies-data-candles = 캔들
strategies-data-pool = 풀
strategies-data-position = 포지션
strategies-data-market = 시장 활동
strategies-data-clock = 시간대

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = 건
strategies-value-trades = { $amount }건

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = 단계가 너무 많음
    .message = 규칙은 최대 { $max }단계까지 중첩할 수 있습니다
strategies-toast-priority-invalid = 잘못된 우선순위
    .message = 1에서 100 사이의 정수를 입력하세요
strategies-toast-exported = 전략을 내보냈습니다
# $name is the strategy name.
strategies-unsaved-confirm = 저장하지 않은 변경 사항을 버릴까요?
    .message = "{ $name }"에 저장되지 않은 변경 사항이 있습니다.
strategies-unsaved-discard = 버리기
strategies-import-error-json = 파일이 올바른 JSON이 아닙니다
# $min and $max are the bounds of the evaluation order.
strategies-import-error-priority = 파일의 우선순위는 { $min }부터 { $max }까지의 정수여야 합니다
strategies-import-error-shape = 파일이 전략을 설명하지 않습니다
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = 파일이 이 버전에 없는 조건을 사용합니다: { $type }
strategies-import-error-not-applicable = "{ $name }"은(는) 이 유형의 전략에서 사용할 수 없습니다

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
strategies-toast-load-failed = 불러오기 실패
    .message = 서버에서 전략을 불러오지 못했습니다
strategies-toast-load-strategy-failed = 전략을 불러오지 못했습니다
strategies-toast-no-strategy = 생성된 전략 없음
    .message = 조건을 하나 이상 추가하거나 '새 전략'을 눌러 먼저 전략을 만드세요
strategies-toast-name-required = 이름 필요
    .message = 저장하기 전에 전략 이름을 입력하세요
strategies-toast-saved = 전략 저장됨
    .message = "{ $name }" 저장 완료
strategies-toast-save-failed = 저장 실패
    .message = 전략을 데이터베이스에 저장하지 못했습니다
strategies-toast-no-strategy-validate = 검증할 전략이 없습니다
strategies-toast-no-strategy-test = 테스트할 전략이 없습니다
strategies-toast-test-save-first = 테스트하기 전에 전략을 저장하세요. 테스트는 저장된 규칙을 읽습니다
strategies-toast-test-failed = 테스트 실패
    .message = 전략을 테스트하지 못했습니다
strategies-toast-valid = 전략이 유효합니다
strategies-toast-invalid = 전략에 오류가 있습니다
strategies-toast-validation-failed = 검증에 실패했습니다
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

## Conditions

strategies-condition-candle-size = 캔들 크기 패턴
    .description = 특정 캔들 패턴을 감지합니다: 큰 몸통, 작은 몸통(도지), 긴 꼬리.
strategies-condition-candle-size-param-pattern = 패턴 유형
    .description = 감지할 캔들 패턴
strategies-condition-candle-size-param-pattern-option-large-body = 큰 몸통 (강한 움직임)
strategies-condition-candle-size-param-pattern-option-small-body = 작은 몸통 (도지/불확실)
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 긴 윗꼬리 (저항)
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 긴 아랫꼬리 (지지)
strategies-condition-candle-size-param-threshold = 크기 임계값 %
    .description = 패턴 감지 기준 비율

strategies-condition-consecutive-candles = 연속 캔들
    .description = 최소 크기 필터와 함께 연속된 녹색(상승) 또는 적색(하락) 캔들을 감지합니다.
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
    .description = 포지션을 보유한 기간을 확인합니다
strategies-condition-position-holding-time-param-hours = 시간 임계값 (시간)
    .description = 포지션을 연 이후 경과 시간(시간 단위)
strategies-condition-position-holding-time-param-comparison = 비교
    .description = 포지션 경과 시간을 임계값과 비교하는 방식
strategies-condition-position-holding-time-param-comparison-option-greater-than = 초과 (>)
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 이상 (≥)
strategies-condition-position-holding-time-param-comparison-option-less-than = 미만 ({ "<" })
strategies-condition-position-holding-time-param-comparison-option-less-equal = 이하 (≤)

strategies-condition-price-breakout = 가격 돌파
    .description = 가격이 저항선(기간 고점)을 위로, 또는 지지선(기간 저점)을 아래로 돌파하는 것을 감지합니다.
strategies-condition-price-breakout-param-lookback = 조회 기간
    .description = 지지/저항 수준을 찾는 데 사용할 캔들 수
strategies-condition-price-breakout-param-direction = 돌파 방향
    .description = 돌파 방향
strategies-condition-price-breakout-param-direction-option-upward = 상향 (저항 돌파)
strategies-condition-price-breakout-param-direction-option-downward = 하향 (지지 붕괴)
strategies-condition-price-breakout-param-confirmation = 확인 %
    .description = 돌파를 확인하기 위해 해당 수준을 얼마나 넘어야 하는지 (거짓 시그널 방지)

strategies-condition-price-change-percent = 가격 변동 %
    .description = 일정 기간 내 가격이 임계 비율만큼 변동했는지 확인합니다.
strategies-condition-price-change-percent-param-percentage = 변동 임계값 %
    .description = 발동할 가격 변동률 (0.1-1000%)
strategies-condition-price-change-percent-param-direction = 방향
    .description = 가격 움직임 방향
strategies-condition-price-change-percent-param-direction-option-above = 상승 (+%)
strategies-condition-price-change-percent-param-direction-option-below = 하락 (-%)
strategies-condition-price-change-percent-param-direction-option-within = 범위 내 (±%)
strategies-condition-price-change-percent-param-time-value = 조회 기간
    .description = 변화를 측정할 과거 기간, 필드 안에서 고른 단위 기준 (1-3600초, 1-1440분, 1-720시간)
strategies-condition-price-change-percent-param-time-unit = 시간 단위
    .description = 조회 기간의 시간 단위
strategies-condition-price-change-percent-param-time-unit-option-seconds = 초
strategies-condition-price-change-percent-param-time-unit-option-minutes = 분
strategies-condition-price-change-percent-param-time-unit-option-hours = 시간

strategies-condition-price-to-ma = 가격 대 이동평균
    .description = 가격이 단순 이동평균 위, 아래 또는 범위 내에 있는지 확인합니다.
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
    .description = 평균 거래량 대비 거래량 급증을 감지합니다 (관심 증가를 의미).
strategies-condition-volume-spike-param-lookback = 조회 기간
    .description = 평균 거래량을 계산할 캔들 수
strategies-condition-volume-spike-param-multiplier = 거래량 배수
    .description = 평균의 몇 배인지 (예: 2.0 = 평균의 200%)

strategies-condition-rsi = RSI (상대강도지수)
    .description = 마감된 캔들에 와일더 평활화를 적용한 RSI가 임계값보다 높거나 낮을 때 발동합니다.
strategies-condition-rsi-param-period = RSI 기간
    .description = RSI를 계산하는 캔들 수. 이 값보다 캔들이 하나 더 필요합니다
strategies-condition-rsi-param-comparison = 비교
    .description = RSI가 임계값보다 낮아야 하는지 높아야 하는지
strategies-condition-rsi-param-comparison-option-below = 미만 (과매도)
strategies-condition-rsi-param-comparison-option-above = 초과 (과매수)
strategies-condition-rsi-param-threshold = RSI 임계값
    .description = 1~99 사이의 RSI 수준 (과매도 30, 과매수 70이 일반적입니다)

strategies-condition-ma-cross = 이동평균 교차
    .description = 빠른 이동평균이 최근 몇 개의 캔들 안에 느린 이동평균을 교차하면 발동합니다.
strategies-condition-ma-cross-param-ma-type = 평균 종류
    .description = 지수 평균은 최근 캔들에 더 큰 가중치를 주고, 단순 평균은 모든 캔들에 같은 가중치를 줍니다
strategies-condition-ma-cross-param-ma-type-option-ema = 지수 (EMA)
strategies-condition-ma-cross-param-ma-type-option-sma = 단순 (SMA)
strategies-condition-ma-cross-param-fast-period = 빠른 기간
    .description = 빠른 평균의 캔들 수
strategies-condition-ma-cross-param-slow-period = 느린 기간
    .description = 느린 평균의 캔들 수. 빠른 기간보다 커야 합니다
strategies-condition-ma-cross-param-direction = 교차 방향
    .description = 빠른 평균이 느린 평균을 어느 방향으로 교차하는지
strategies-condition-ma-cross-param-direction-option-bullish = 상승 (빠른 선이 위로 교차)
strategies-condition-ma-cross-param-direction-option-bearish = 하락 (빠른 선이 아래로 교차)
strategies-condition-ma-cross-param-within-candles = 교차 허용 범위
    .description = 교차가 포함될 수 있는 최근 캔들 수. 1은 가장 최근 캔들만 의미합니다

strategies-condition-position-profit = 포지션 수익
    .description = 열린 포지션의 미실현 수익이 진입 가격 대비 지정한 비율보다 높거나 낮을 때 발동합니다(풀 가격 기준).
strategies-condition-position-profit-param-comparison = 비교
    .description = 수익이 임계값 이상이어야 하는지 이하여야 하는지
strategies-condition-position-profit-param-comparison-option-above = 이상 (≥)
strategies-condition-position-profit-param-comparison-option-below = 이하 (≤)
strategies-condition-position-profit-param-threshold-pct = 수익 임계값 %
    .description = 진입 가격 대비 미실현 수익 비율. 음수는 손실입니다

strategies-condition-drawdown-from-peak = 고점 대비 하락
    .description = 마지막 매수 이후 최고 가격에서 지정한 비율만큼 풀 가격이 하락하면 발동합니다. 단, 그 고점이 진입 가격보다 충분히 높았어야 합니다.
strategies-condition-drawdown-from-peak-param-drop-pct = 고점 대비 하락 %
    .description = 현재 가격이 고점보다 얼마나 낮아야 하는지
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = 수익 후 활성화 %
    .description = 조건이 발동하려면 고점이 진입 가격보다 최소한 이 비율만큼 높았어야 합니다. 0은 항상 활성

strategies-condition-buy-pressure = 매수 압력
    .description = 시간 구간의 거래 중 매수 비율이 지정 값 이상이고 그 구간에 거래가 충분할 때 발동합니다. 토큰의 매수·매도 건수를 읽습니다.
strategies-condition-buy-pressure-param-window = 시간 구간
    .description = 매수·매도 건수가 다루는 기간
strategies-condition-buy-pressure-param-window-option-5m = 5분
strategies-condition-buy-pressure-param-window-option-1h = 1시간
strategies-condition-buy-pressure-param-window-option-6h = 6시간
strategies-condition-buy-pressure-param-window-option-24h = 24시간
strategies-condition-buy-pressure-param-min-buy-share-pct = 최소 매수 비율 %
    .description = 구간 내 전체 거래 중 매수의 비율
strategies-condition-buy-pressure-param-min-transactions = 최소 거래 수
    .description = 거래가 이보다 적은 구간은 조건을 발동하지 않습니다

strategies-condition-transaction-velocity = 거래 속도
    .description = 최근 구간의 거래 속도가 그 구간을 포함하는 더 긴 구간 속도의 배수 이상 또는 이하일 때 충족됩니다. 토큰의 매수 및 매도 건수를 읽습니다.
strategies-condition-transaction-velocity-param-window = 구간
    .description = 최근 구간과 비교 대상인 더 긴 구간
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5분 대 1시간
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1시간 대 6시간
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6시간 대 24시간
strategies-condition-transaction-velocity-param-comparison = 비교
    .description = 최근 속도가 비율 이상이어야 하는지 이하여야 하는지
strategies-condition-transaction-velocity-param-comparison-option-above = 이상 (≥)
strategies-condition-transaction-velocity-param-comparison-option-below = 이하 (≤)
strategies-condition-transaction-velocity-param-ratio = 속도 비율
    .description = 최근 구간의 초당 거래 수를 긴 구간의 초당 거래 수로 나눈 값
strategies-condition-transaction-velocity-param-min-transactions = 최소 거래 수
    .description = 거래 수가 이보다 적은 최근 구간은 조건을 충족하지 않습니다

strategies-condition-pullback-from-high = 고점 대비 되돌림
    .description = 실시간 가격이 최근 캔들 최고가 아래로 최소 하락과 최대 하락 사이에 있을 때 충족됩니다.
strategies-condition-pullback-from-high-param-lookback = 조회 기간
    .description = 최고가를 찾는 캔들 수
strategies-condition-pullback-from-high-param-min-drop-pct = 최소 하락 %
    .description = 가격은 최고가보다 최소 이만큼 아래에 있어야 합니다
strategies-condition-pullback-from-high-param-max-drop-pct = 최대 하락 %
    .description = 가격은 최고가보다 최대 이만큼 아래에 있어야 합니다. 최소 하락보다 커야 합니다

strategies-condition-profit-giveback = 수익 반납
    .description = 포지션이 마지막 매수 이후 최고 미실현 수익이 일정 수준에 도달한 뒤 그 일부를 반납했을 때 충족됩니다.
strategies-condition-profit-giveback-param-arm-after-profit-pct = 수익 후 활성화 %
    .description = 조건이 충족되려면 최고 수익이 먼저 진입가의 이 비율에 도달해야 합니다
strategies-condition-profit-giveback-param-giveback-pct = 반납 %
    .description = 최고 수익 중 반납한 비율. 100%는 가격이 진입가로 돌아왔다는 뜻입니다

strategies-condition-token-age = 토큰 경과 시간
    .description = 토큰이 온체인에서 생성된 후 지난 시간이 지정 기간보다 길거나 짧을 때 충족됩니다. 생성 시각을 알 수 없는 토큰은 조건을 미정으로 둡니다.
strategies-condition-token-age-param-comparison = 비교
    .description = 토큰이 기간보다 오래되었는지 새로운지
strategies-condition-token-age-param-comparison-option-older-than = 초과 (≥)
strategies-condition-token-age-param-comparison-option-younger-than = 미만 ({ "<" })
strategies-condition-token-age-param-time-value = 경과 시간
    .description = 토큰 생성 후 시간. 단위는 필드 안에서 선택 (1-10000)
strategies-condition-token-age-param-time-unit = 시간 단위
    .description = 경과 시간의 단위
strategies-condition-token-age-param-time-unit-option-minutes = 분
strategies-condition-token-age-param-time-unit-option-hours = 시간
strategies-condition-token-age-param-time-unit-option-days = 일

strategies-condition-vwap = VWAP
    .description = 실시간 가격이 최근 캔들의 거래량 가중 평균 가격보다 최소 일정 거리 위 또는 아래에 있을 때 충족됩니다.
strategies-condition-vwap-param-lookback = 조회 기간
    .description = 거래량 가중 평균이 포함하는 캔들 수
strategies-condition-vwap-param-position = 위치
    .description = VWAP 대비 가격 위치
strategies-condition-vwap-param-position-option-above = VWAP 위
strategies-condition-vwap-param-position-option-below = VWAP 아래
strategies-condition-vwap-param-distance-pct = 거리 %
    .description = VWAP으로부터의 최소 거리. 0은 선택한 쪽이면 거리와 무관함을 뜻합니다

strategies-condition-volatility-range = 변동성 범위
    .description = 최근 캔들의 평균 실제 범위(ATR)가 실시간 가격 대비 비율로 기준값보다 높거나 낮을 때 충족됩니다. 그룹 안에 두 개를 두면 범위가 됩니다.
strategies-condition-volatility-range-param-period = ATR 기간
    .description = 평균 실제 범위를 평활하는 캔들 수. 조건에는 이보다 캔들 하나가 더 필요합니다
strategies-condition-volatility-range-param-comparison = 비교
    .description = 범위가 기준값보다 높아야 하는지 낮아야 하는지
strategies-condition-volatility-range-param-comparison-option-above = 높음
strategies-condition-volatility-range-param-comparison-option-below = 낮음
strategies-condition-volatility-range-param-threshold-pct = 범위 기준값 %
    .description = 실시간 가격 대비 평균 실제 범위의 비율

strategies-condition-bollinger-bands = 볼린저 밴드
    .description = 실시간 가격이 하단 밴드 아래 또는 상단 밴드 위에 있거나 밴드가 수축할 때 충족됩니다. 밴드는 평균 종가에서 표준편차의 몇 배만큼 떨어져 있습니다.
strategies-condition-bollinger-bands-param-period = 기간
    .description = 평균과 표준편차가 포함하는 캔들 수
strategies-condition-bollinger-bands-param-std-dev = 표준편차
    .description = 각 밴드가 평균에서 떨어진 표준편차 수
strategies-condition-bollinger-bands-param-signal = 신호
    .description = 밴드가 보여야 하는 상태
strategies-condition-bollinger-bands-param-signal-option-below-lower = 하단 밴드 아래
strategies-condition-bollinger-bands-param-signal-option-above-upper = 상단 밴드 위
strategies-condition-bollinger-bands-param-signal-option-squeeze = 스퀴즈
strategies-condition-bollinger-bands-param-squeeze-width-pct = 스퀴즈 폭 %
    .description = 스퀴즈의 경우: 밴드 사이 최대 거리(평균 대비 비율)

strategies-condition-liquidity-to-size = 규모 대비 유동성
    .description = 풀 준비금이 걸린 규모의 배수 이상이거나 미만일 때 충족됩니다. 진입 전략에서는 거래 규모, 청산 전략에서는 포지션 잔여 가치입니다.
strategies-condition-liquidity-to-size-param-comparison = 비교
    .description = 준비금이 배수 이상이어야 하는지 미만이어야 하는지
strategies-condition-liquidity-to-size-param-comparison-option-at-least = 이상 (≥)
strategies-condition-liquidity-to-size-param-comparison-option-below = 미만 ({ "<" })
strategies-condition-liquidity-to-size-param-multiple = 유동성 배수
    .description = 풀 준비금을 걸린 규모로 나눈 값. 배수 N이면 가격이 약 1/(N+1) 움직입니다

strategies-condition-position-adds = 포지션 추가
    .description = 첫 진입 후 추가 매수 횟수 또는 부분 매도 횟수가 지정 수 이상 또는 이하일 때 충족됩니다.
strategies-condition-position-adds-param-subject = 대상
    .description = 세는 동작
strategies-condition-position-adds-param-subject-option-adds = 추가 매수
strategies-condition-position-adds-param-subject-option-partial-sells = 부분 매도
strategies-condition-position-adds-param-comparison = 비교
    .description = 횟수가 지정 수 이상이어야 하는지 이하여야 하는지
strategies-condition-position-adds-param-comparison-option-at-least = 이상 (≥)
strategies-condition-position-adds-param-comparison-option-at-most = 이하 (≤)
strategies-condition-position-adds-param-count = 횟수
    .description = 동작 횟수

strategies-condition-time-of-day = 시간대
    .description = 현재 시각이 UTC 시간 구간 안에 있을 때 충족됩니다. 시작이 종료보다 늦으면 자정을 넘깁니다.
strategies-condition-time-of-day-param-start-hour = 시작 시각 (UTC)
    .description = 구간의 첫 시각, 0부터 23
strategies-condition-time-of-day-param-end-hour = 종료 시각 (UTC)
    .description = 구간이 끝나는 시각, 1부터 24. 그 시각 자체는 구간 밖입니다

strategies-condition-liquidity-change-since-entry = 진입 이후 유동성 변화
    .description = 포지션을 연 이후 풀 준비금이 일정 비율 이상 감소하거나 증가했을 때 충족됩니다. 진입 시 유동성이 기록되지 않은 포지션은 조건을 미정으로 둡니다.
strategies-condition-liquidity-change-since-entry-param-comparison = 비교
    .description = 준비금이 감소했어야 하는지 증가했어야 하는지
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = 감소
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = 증가
strategies-condition-liquidity-change-since-entry-param-change-pct = 변화 %
    .description = 진입 이후 풀 준비금의 변화

## Shared by every condition

strategies-condition-param-timeframe = 타임프레임
    .description = 캔들 크기: 조건이 읽는 각 캔들의 길이 (설정하지 않으면 전략 타임프레임)
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
strategies-error-stale-market-data = { $window } 거래 건수가 오래되었습니다: 경과 시간 { $age }초가 { $max }초를 초과합니다
strategies-error-unusable-data = { $field } 값 "{ $value }"을(를) 사용할 수 없습니다
strategies-error-invalid-rule-tree = 규칙 트리가 올바르지 않습니다: { $reason }
strategies-error-evaluation-timeout = { $timeout }ms 후 전략 평가가 시간 초과되었습니다
strategies-error-condition-not-applicable = { $condition }은(는) { $strategy } 전략에서 사용할 수 없습니다
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
strategies-error-field-arm-after-profit = 수익 후 활성화
strategies-error-field-drop-percentage = 하락률
strategies-error-field-entry-price = 진입 가격
strategies-error-field-fast-period = 빠른 기간
strategies-error-field-minimum-buy-share = 최소 매수 비율
strategies-error-field-minimum-transactions = 최소 거래 수
strategies-error-field-moving-average-type = 평균 종류
strategies-error-field-moving-average = 이동평균
strategies-error-field-reference-candle-close = 기준 캔들 종가
strategies-error-field-peak-price = 고점 가격
strategies-error-field-slow-period = 느린 기간
strategies-error-field-window = 시간 구간
strategies-error-field-within-candles = 교차 캔들 수
strategies-error-field-change = 변화
strategies-error-field-end-hour = 종료 시각
strategies-error-field-entry-liquidity = 진입 시 유동성
strategies-error-field-giveback = 반납
strategies-error-field-highest-high = 최고가
strategies-error-field-long-window-transactions = 긴 구간 거래 수
strategies-error-field-maximum-drop = 최대 하락
strategies-error-field-minimum-drop = 최소 하락
strategies-error-field-multiple = 배수
strategies-error-field-pool-liquidity = 풀 유동성
strategies-error-field-ratio = 비율
strategies-error-field-remaining-position-value = 포지션 잔여 가치
strategies-error-field-signal = 신호
strategies-error-field-squeeze-width = 스퀴즈 폭
strategies-error-field-standard-deviations = 표준편차
strategies-error-field-start-hour = 시작 시각
strategies-error-field-subject = 대상
strategies-error-field-token-creation-time = 토큰 생성 시각
strategies-error-field-total-volume = 총 거래량
strategies-error-field-trade-size = 거래 규모
strategies-error-field-vwap = VWAP

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
strategies-error-data-peak-price = 고점 가격
strategies-error-data-transaction-counts = 거래 건수
strategies-error-data-entry-liquidity = 진입 시 유동성
strategies-error-data-entry-pool-liquidity = 진입 풀의 유동성
strategies-error-data-real-pool-reserves = 풀의 실제 유동성
strategies-error-data-remaining-position-value = 포지션 잔여 가치
strategies-error-data-token-creation-time = 토큰 생성 시각
strategies-error-data-trade-size = 거래 규모

# Indicators

strategies-error-indicator-consecutive-candles = 연속 캔들
strategies-error-indicator-moving-average = 이동평균
strategies-error-indicator-price-breakout = 가격 돌파
strategies-error-indicator-price-change-lookback = 가격 변동 조회
strategies-error-indicator-volume-spike = 거래량 급증
strategies-error-indicator-moving-average-cross = 이동평균 교차
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = 볼린저 밴드
strategies-error-indicator-pullback-from-high = 고점 대비 되돌림
strategies-error-indicator-volatility-range = 변동성 범위
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = 분기 노드에 조건이 없습니다
strategies-error-rule-branch-node-missing-operator = 분기 노드에 연산자가 없습니다
strategies-error-rule-branch-node-must-have-at-least-one-child = 분기 노드에는 하위 노드가 하나 이상 있어야 합니다
strategies-error-rule-invalid-rule-tree-structure = 규칙 트리 구조가 올바르지 않습니다
strategies-error-rule-leaf-node-missing-condition = 리프 노드에 조건이 없습니다
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 연산자에는 하위 노드가 정확히 하나 있어야 합니다
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = 그룹에는 켜진 조건이 하나 이상 필요합니다
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT 연산자의 자식 노드는 꺼져 있을 수 없습니다
strategies-error-rule-rules-nest-deeper-than-four-levels = 규칙은 4단계보다 깊게 중첩할 수 없습니다
strategies-error-rule-the-root-group-cannot-be-switched-off = 루트 그룹은 끌 수 없습니다

strategies-readiness-history-since-met =
    { $count ->
       *[other] { $changed } 규칙 변경 이후 종료된 모의 라운드 { $count }개, 필요 { $needed }개
    }
strategies-readiness-history-since-short = { $changed } 규칙 변경 이후 종료된 모의 라운드 { $needed }개 중 { $count }개
strategies-readiness-priced = 모든 포지션 가격 확인
strategies-readiness-priced-ok = 열린 모든 모의 포지션에 풀 가격이 있습니다
strategies-readiness-priced-missing =
    { $count ->
       *[other] 풀 가격이 없는 열린 모의 포지션 { $count }건
    }
strategies-readiness-runtime = 실거래 가능
strategies-readiness-runtime-ok = 설정과 안전 게이트가 실거래를 허용합니다

## Roster: the strategy list, one view per kind. Status and notice ids come from
## src/strategies/overview.rs.

strategies-back = 모든 전략
strategies-view-kind = 전략 종류
strategies-unnamed = 이름 없는 전략
strategies-col-on = 켜짐
strategies-col-strategy = 전략
strategies-col-order = 순서
    .title = 같은 모드의 활성 진입 전략 사이의 평가 순서이며, 먼저 신호를 낸 전략이 토큰을 가져갑니다
strategies-col-mode = 모드
strategies-col-status = 상태
strategies-col-open = 보유
strategies-col-watching = 감시 중
strategies-col-rounds = 라운드
strategies-col-closed = 종료
strategies-col-win-rate = 승률
strategies-col-realized = 실현 ({ -sol })
strategies-col-manages = 관리 대상
strategies-col-last-signal = 마지막 신호
strategies-status-running = 실행 중
strategies-status-not-running = 실행 안 됨
strategies-status-off = 꺼짐
strategies-status-setup-required = 설정 필요
strategies-status-live-paused = 실거래 진입 일시 중지
strategies-status-paper-cash = 모의 자금 부족
strategies-status-slots-full = 슬롯 가득 참
strategies-status-entry-monitor-off = 진입 모니터 꺼짐
strategies-status-exit-monitor-off = 청산 모니터 꺼짐
strategies-notice-setup-required = 모의 거래는 설정을 마친 후 시작됩니다
strategies-notice-force-stopped = 긴급 정지 중
strategies-notice-trader-off = Auto Trader가 꺼져 있습니다
strategies-notice-waiting = 핵심 서비스 대기 중
strategies-notice-monitors-off = 진입 및 청산 모니터가 꺼져 있습니다
strategies-notice-start = Auto Trader 시작
# $closed and $needed count closed paper rounds; $changed is a formatted date.
strategies-readiness-progress = 모의 라운드 { $closed }/{ $needed }
strategies-readiness-ready = 실거래 전환 가능
strategies-readiness-since = 규칙 변경 { $changed }
# $book is the trading mode of the other book; $count its open positions.
strategies-open-other-book =
    { $count ->
       *[other] { $book }: 포지션 { $count }개
    }
strategies-manages-all = 모든 진입 전략
# $first is the first managed entry strategy; $more the count of the others.
strategies-manages-some =
    { $more ->
       *[other] { $first } 외 { $more }개
    }
strategies-exit-before = 먼저 확인
strategies-exit-hint = 이 청산 규칙은 모든 포지션에서 어떤 청산 전략보다 먼저 확인됩니다
strategies-action-edit = { $name } 편집
strategies-action-delete = { $name } 삭제
strategies-roster-stale = 마지막 업데이트

## Saving new rules: the server answers whether a save changed the rules a strategy trades on.

strategies-live-rules-confirm = 실거래 중인 "{ $name }"의 규칙 변경
    .message = 새 규칙은 다음 확인부터 실제 { -sol }(으)로 거래하며, 이 전략의 모의 근거는 이번 저장부터 다시 시작됩니다.
strategies-live-rules-save = 저장하고 실거래
strategies-toast-evidence-restarted = 전략 저장됨
    .message = "{ $name }"의 규칙이 바뀌었습니다. 모의 근거는 지금부터 다시 시작됩니다

## Trading mode: arming a paper strategy live and returning it to paper.

strategies-mode-arm = 실거래 활성화
strategies-mode-return-paper = 모의로 되돌리기
strategies-mode-arm-title = "{ $name }" 실거래 활성화
strategies-mode-arm-lead = "{ $name }"의 새 진입은 내 거래 지갑의 실제 스왑으로 거래됩니다.
strategies-mode-arm-lead-exit = "{ $name }"은(는) 신호가 나면 트레이딩 지갑에서 실제 스왑으로 실거래 포지션을 매도합니다.
strategies-mode-arm-readiness = 모의 장부 기준 준비 상태
strategies-mode-arm-open-paper = 열린 모의 포지션은 모의로 유지되며 모의로 종료됩니다.
strategies-mode-arm-ack-real-native = 새 진입은 내 거래 지갑의 실제 { -sol }를 사용합니다.
strategies-mode-arm-ack-real-native-exit = 관리하는 실거래 포지션은 실제 스왑으로 매도됩니다.
strategies-mode-arm-ack-fees = 실거래 스왑에는 실제 네트워크 수수료와 슬리피지가 발생하며, 모의 결과가 실거래 결과를 보장하지 않습니다.
strategies-mode-arm-ack-unready = 일부 준비 상태 점검을 통과하지 못했습니다. 그래도 이 전략을 활성화합니다.
strategies-mode-arm-unavailable = 현재 실거래를 사용할 수 없습니다. 마지막 점검 결과를 확인하세요.
strategies-mode-return-title = "{ $name }"을(를) 모의로 되돌리기
strategies-mode-return-message = 새 진입은 { -sol }를 쓰지 않고 모의 장부에서 열립니다. 열린 실거래 포지션은 실거래로 유지되며 실거래로 종료됩니다.
strategies-mode-return-message-exit = 실거래 포지션 판단을 멈추며, 해당 포지션은 규칙 청산을 유지합니다.
strategies-mode-readiness-failed = 전략의 준비 상태를 읽지 못했습니다
strategies-mode-toast-armed = { $name } 전략이 이제 실거래로 거래합니다
strategies-mode-toast-paper = { $name } 전략이 이제 모의로 거래합니다
strategies-mode-toast-failed = 거래 모드를 변경하지 못했습니다
