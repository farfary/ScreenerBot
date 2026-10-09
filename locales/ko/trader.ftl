# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = 손절
trader-exit-type-take-profit = 익절
trader-exit-type-roi = ROI 목표
trader-exit-type-roi-exit = ROI 목표
trader-exit-type-trailing-stop = 트레일링 스톱
trader-exit-type-time-override = 시간 우선 적용
trader-exit-type-time-rule = 시간 규칙
trader-exit-type-manual = 수동
trader-exit-type-manual-close = 수동
trader-exit-type-dca = DCA
trader-exit-type-unknown = 알 수 없음

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 통계
trader-tab-strategy-control = 전략 제어
trader-tab-strategies = 전략
trader-tab-stop-loss = 손절
trader-tab-trailing-stop = 트레일링 스톱
trader-tab-roi = 익절
trader-tab-time-rules = 시간 규칙
trader-tab-dca = DCA
trader-tab-settings = 설정

## Feature status badges and their messages

trader-feature-coming-soon = 출시 예정
    .message = 이 기능은 곧 제공될 예정이며 아직 사용할 수 없습니다.
trader-feature-beta = 베타
trader-feature-disabled = 사용 안 함
    .message = 이 기능은 현재 사용하지 않도록 설정되어 있습니다.

## Status bar and trading controls

trader-status-title = 자동 트레이더
trader-status-loading = 불러오는 중...
trader-status-running = 실행 중
trader-status-stopped = 중지됨
trader-status-setup-required = 설정 필요
trader-status-unavailable = 자동 트레이더를 사용하려면 지갑과 RPC 설정을 완료하세요
trader-toggle-on = 사용
trader-toggle-off = 사용 안 함
trader-toggle-unavailable = 사용 불가
trader-toggle-start-failed = 트레이더를 시작하지 못했습니다
trader-toggle-stop-failed = 트레이더를 중지하지 못했습니다
trader-controls-title = 트레이딩 제어
trader-halt-title = 거래 중단됨
trader-halt-reason-default = 수동 강제 중지
trader-halt-resume = 재개
trader-monitor-entry = 진입 모니터
trader-monitor-exit = 청산 모니터
trader-monitor-master-off = 자동 트레이더 꺼짐
trader-loss-limit-title = 기간 손실 한도
trader-loss-limit-resume = 거래 재개
trader-loss-limit-reset = 기간 초기화
trader-loss-limit-off = 꺼짐
trader-loss-limit-none = 설정된 기간 손실 한도가 없습니다
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = { $hours } { $minutes } 후 초기화
trader-loss-limit-reached = 한도 도달
trader-force-stop = 모두 강제 중지

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = 거래 강제 중지
    .message = 모든 거래 작업이 즉시 중단됩니다. 계속하시겠습니까?
    .confirm = 거래 중지
trader-loss-limit-resume-confirm = 손실 한도 이후 재개
    .message = 기간 손실 한도로 신규 진입이 중지되었습니다. 재개하면 기간이 초기화되기 전에 트레이더가 다시 포지션을 열 수 있습니다. 계속하시겠습니까?
trader-loss-limit-reset-confirm = 손실 한도 기간 초기화
    .message = 현재 기간의 누적 손실이 지워지고 새 기간이 시작됩니다. 계속하시겠습니까?

## Toasts

trader-toast-control-failed = 자동 트레이더 제어에 실패했습니다
trader-toast-force-stop-on = 강제 중지가 활성화되었습니다
trader-toast-force-stop-failed = 강제 중지를 활성화하지 못했습니다
trader-toast-force-stop-cleared = 강제 중지가 해제되었습니다
trader-toast-resume-failed = 거래를 재개하지 못했습니다
trader-toast-loss-limit-reset-failed = 손실 한도를 초기화하지 못했습니다
trader-toast-entry-monitor-failed = 진입 모니터를 전환하지 못했습니다
trader-toast-exit-monitor-failed = 청산 모니터를 전환하지 못했습니다
trader-toast-load-failed = 불러오기 실패
    .message = 트레이더 설정을 불러오지 못했습니다
trader-toast-saved = 설정 저장됨
    .message = 트레이더 설정이 적용되었습니다
trader-toast-save-failed = 저장 실패
    .message = 트레이더 설정을 저장하지 못했습니다
trader-toast-feature-enabled = 기능 사용 설정됨
trader-toast-feature-disabled = 기능 사용 안 함
trader-toast-feature-applied = 자동 트레이더 설정이 적용되었습니다
trader-toast-strategy-enabled = 전략 사용 설정됨
    .message = 전략이 활성 상태입니다
trader-toast-strategy-disabled = 전략 사용 안 함
    .message = 전략이 비활성 상태입니다
trader-toast-strategy-failed = 업데이트 실패
    .message = 전략 상태를 업데이트하지 못했습니다

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = 통계 기간
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = 실현 성과
trader-metric-net-pnl = 순손익
trader-metric-win-rate = 승률
trader-metric-profit-factor = 프로핏 팩터
trader-metric-max-drawdown = 최대 낙폭
trader-metric-capital = 운용 자본
trader-metric-avg-win-loss = 평균 수익 / 손실
trader-metric-closed-trades = 종료된 거래
trader-metric-median-hold = 중앙 보유 시간
trader-stats-empty = 이 기간에 종료된 거래가 없습니다
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = 수익 { $won } · 손실 { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
       *[other] { $amount }승
    }
trader-stats-losses =
    { $count ->
       *[other] { $amount }패
    }
# $amount is a formatted SOL amount.
trader-stats-expected = 거래당 기대값 { $amount }
trader-stats-profit-factor-basis = 총수익 ÷ 총손실
trader-stats-drawdown-basis = 실현 기준 최대 고점 대비 저점
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
       *[other] 포지션 슬롯 { $max }개 중 { $used }개 사용 중
    }
trader-stats-avg-basis = 수익 거래와 손실 거래의 평균 결과
trader-stats-closed =
    { $count ->
       *[other] 종료된 포지션 { $amount }개
    }
# $span is a formatted duration.
trader-stats-hold-average = 평균 { $span }
trader-stats-excluded =
    { $count ->
       *[other] 종료된 라운드 { $amount }개 제외됨 - 완전한 매입 원가가 없어 정확한 손익을 산출할 수 없습니다.
    }

## Stats: daily P&L and extremes

trader-daily-title = 일별 손익
trader-daily-subtitle = 일별 실현 { -sol }과 누적 합계
trader-daily-loading = 일별 손익 불러오는 중...
trader-daily-chart = { -sol } 기준 일별 실현 손익
trader-extreme-best = 최고 거래
trader-extreme-worst = 최악 거래

## Stats: exit breakdown

trader-exit-title = 청산 전략별 분석
trader-exit-subtitle = 포지션이 종료된 방식과 각 청산의 회수 결과
trader-exit-loading = 청산 데이터 불러오는 중...
trader-exit-empty-day = 최근 24시간 동안 종료된 거래가 없습니다
trader-exit-empty-days =
    { $count ->
       *[other] 최근 { $amount }일 동안 종료된 거래가 없습니다
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
       *[other] 거래 { $amount }건 · 전체 청산의 { $share }
    }
# $value is a formatted average percentage.
trader-exit-average = 평균 { $value }

## Shared example vocabulary

trader-impact-label = 영향:
trader-current-label = 현재:
trader-readable-label = 읽기 쉬운 표기:
trader-example-how-it-works = 작동 방식
trader-step-entry = 진입
trader-step-initial-position = 초기 포지션
trader-step-auto-exit = 자동 청산
trader-step-exit = 청산
trader-step-full-exit = 포지션 전량 청산
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = 수익 +{ $value }%

## Stop loss

trader-stop-loss-title = 손절
trader-stop-loss-subtitle = 손실이 설정한 임계값을 넘으면 포지션을 자동으로 청산합니다
# $threshold is the threshold as typed.
trader-stop-loss-impact = 진입가 대비 { $threshold }% 하락 시 청산
trader-stop-loss-hold-immediate = 즉시
# $span is a formatted duration.
trader-stop-loss-hold-delay = { $span } 지연
trader-stop-loss-price-falls = 가격 하락
trader-stop-loss-threshold-reached = 임계값 도달
trader-stop-loss-partial = 부분 청산 허용
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = 손실이 <strong>{ $loss }</strong>로 제한됨
trader-stop-loss-note = <strong>참고:</strong> 손절은 조기에 청산하여 더 큰 손실을 방지합니다

## Trailing stop

trader-trailing-title = 트레일링 스톱
trader-trailing-subtitle = 가격이 오르는 동안 따라가며 수익을 자동으로 보호합니다
# $value is the activation percentage as typed.
trader-trailing-activation-impact = 수익 +{ $value }%에서 추적 시작
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = 고점 대비 -{ $value }%에서 청산
trader-trailing-activation = 활성화
trader-trailing-peak = 고점
# $value is a formatted percentage.
trader-trailing-final = 최종 +{ $value }%
# $value is a formatted percentage.
trader-trailing-summary-protected = 수익 <strong>{ $value }</strong> 보호됨
# $value is a formatted percentage.
trader-trailing-summary-avoided = 고점 대비 손실 <strong>{ $value }</strong> 회피
## Take profit

trader-roi-title = 익절
trader-roi-subtitle = 수익이 목표에 도달하면 포지션 전체를 자동으로 청산합니다
# $target is the target percentage as typed.
trader-roi-impact = 수익 +{ $target }%에서 청산
trader-roi-example-title = 예시 시나리오
trader-roi-initial-buy = 최초 매수
trader-roi-target-hit = 목표 도달
trader-roi-full-position = 전체 포지션
trader-roi-sold = 100% 매도
# $target is the target percentage as typed.
trader-roi-summary = 수익 <strong>+{ $target }%</strong> 확정

## Time-based exit

trader-time-title = 시간 기반 청산
trader-time-subtitle = 최대 보유 시간이 지난 뒤 손실이 임계값을 넘으면 포지션을 자동으로 청산합니다
trader-time-unit-seconds = 초
trader-time-unit-minutes = 분
trader-time-unit-hours = 시간
trader-time-unit-days = 일
# Shown before the configured duration loads.
trader-time-conversion-default = 168시간 = 7일
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
       *[other] { $amount }초
    }
trader-duration-minutes =
    { $count ->
       *[other] { $amount }분
    }
trader-duration-hours =
    { $count ->
       *[other] { $amount }시간
    }
trader-duration-days =
    { $count ->
       *[other] { $amount }일
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = 보유 기간 이후 { $value }% 이상 하락 시 청산
# $day is the day number of the example.
trader-time-day = { $day }일째
trader-time-position-opened = 포지션 진입
trader-time-limit = 시간 제한
trader-time-hold-reached = 보유 기간 도달
trader-time-loss-met = 손실 임계값 충족
trader-time-note = <strong>참고:</strong> 수익 상태이거나 손실이 더 작은 포지션은 청산되지 않습니다
trader-time-positions-title = 현재 포지션 상태
trader-time-positions-loading = 포지션 불러오는 중...
trader-time-positions-empty = 보유 중인 포지션 없음
trader-time-positions-token = 토큰
trader-time-positions-hold = 보유 시간
trader-time-positions-roi = ROI

## Strategy control

trader-strategy-entry-title = 진입 전략
trader-strategy-entry-subtitle = 새 포지션을 열 수 있는 시그널입니다.
trader-strategy-exit-title = 청산 전략
trader-strategy-exit-subtitle = 보유 중인 포지션을 종료하거나 보호할 수 있는 시그널입니다.
trader-strategy-active-unknown = -- 활성
trader-strategy-active = { $enabled }/{ $total } 활성
trader-strategy-loading = 전략 불러오는 중...
trader-strategy-load-failed = 전략을 불러오지 못했습니다
trader-strategy-empty = 정의된 전략이 없습니다
trader-strategy-no-description = 설명이 없습니다.
trader-strategy-unnamed = 이름 없는 전략
trader-strategy-priority-auto = 자동
trader-strategy-priority = 우선순위 { $priority }

## Dollar-cost averaging

trader-dca-title = 분할 매수 (DCA)
trader-dca-subtitle = 손실 중인 포지션에 자동으로 추가 매수하여 평균 진입가를 낮춥니다
trader-dca-example-title = DCA 예시
trader-dca-example = 초기 0.01 { -sol } → DCA #1: 0.005 { -sol } @ -10% → DCA #2: 0.005 { -sol } @ 추가 -10%
trader-dca-info-title = DCA 전략 정보
trader-dca-info-subtitle = DCA 거래 시 유의할 점
trader-dca-how-title = DCA 작동 방식
trader-dca-how-trigger = <strong>조건:</strong> 포지션이 DCA 임계값(예: -10%) 아래로 하락
trader-dca-how-action = <strong>동작:</strong> { -sol }을 추가 투입하여 평균 매입 원가를 낮춥니다
trader-dca-how-repeat = <strong>반복:</strong> 최대 횟수까지 여러 번 DCA할 수 있습니다
trader-dca-risk-title = 위험 경고
trader-dca-risk-exposure = <strong>노출 증가:</strong> DCA는 포지션당 위험에 노출되는 총자본을 늘립니다
trader-dca-risk-knife = <strong>떨어지는 칼날:</strong> 토큰이 계속 하락하면 DCA는 도움이 되지 않습니다
trader-dca-risk-cooldown = <strong>쿨다운:</strong> 쿨다운을 사용하여 연속적인 DCA 진입을 방지하세요

## General settings

trader-sizing-title = 포지션 규모
trader-sizing-subtitle = 포지션당 투자 금액을 조절합니다
trader-timing-title = 타이밍 및 쿨다운
trader-timing-subtitle = 작업 간 타이밍을 조절합니다
trader-timing-close-cooldown = 포지션 종료 쿨다운
trader-timing-close-cooldown-hint = 같은 토큰을 다시 진입하기 전에 기다리는 시간(분)
trader-timing-concurrency = 진입 확인 동시 처리 수
trader-timing-concurrency-hint = 동시에 확인할 토큰 수 (높을수록 빠르지만 CPU 사용량 증가)
trader-timing-unit-minutes = 분
trader-timing-unit-tokens = 개
