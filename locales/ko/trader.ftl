# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = 손절
trader-exit-type-take-profit = 익절
trader-exit-type-roi = 익절
trader-exit-type-roi-exit = 익절
trader-exit-type-trailing-stop = 트레일링 스톱
trader-exit-type-time-override = 시간 청산
trader-exit-type-time-rule = 시간 규칙
trader-exit-type-manual = 수동
trader-exit-type-manual-close = 수동
trader-exit-type-dca = DCA
trader-exit-type-unknown = 알 수 없음

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 통계
trader-tab-strategies = 전략
trader-tab-stop-loss = 손절
trader-tab-trailing-stop = 트레일링 스톱
trader-tab-take-profit = 익절
trader-tab-time-rules = 시간 청산
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
trader-setup-action = 설정 완료
trader-toggle-on = 사용
trader-toggle-off = 사용 안 함
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
trader-force-stop-hint = 모든 모니터, 복사 작업, 거래를 한 번에 중지합니다.
trader-force-stop-idle = 강제 중지로 멈출 실행 중인 항목이 없습니다.

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

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = 통계 기간
trader-stats-book =
    .aria-label = 거래 장부
trader-paper-run-since = { $date }부터의 모의 운용
trader-paper-start = 시작
trader-paper-cash = 현금
trader-paper-open = 보유 중
trader-paper-unrealized = 미실현
trader-paper-reset = 모의 운용 초기화
trader-paper-reset-title = 모의 운용을 초기화할까요?
trader-paper-reset-run-message = 카피 작업의 모의 보유 수량을 포함해 열린 모의 포지션은 모두 현재 가격으로 종료되고, 설정된 시작 잔액으로 새 운용이 시작됩니다. 이전 운용은 기록에 남습니다.
trader-paper-reset-done = 새 모의 운용이 시작되었습니다
trader-paper-reset-failed = 모의 운용을 초기화하지 못했습니다
trader-paper-account-failed = 모의 운용을 불러오지 못했습니다
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
trader-weekly-title = 주별 손익
trader-weekly-subtitle = ISO 주별 실현 { -sol }과 누적 합계
trader-weekly-chart = { -sol } 기준 주별 실현 손익
trader-extreme-best = 최고 거래
trader-extreme-worst = 최악 거래

## Stats: exit breakdown

trader-exit-title = 청산 전략별 분석
trader-exit-subtitle = 포지션이 종료된 방식과 각 청산의 회수 결과
trader-exit-loading = 청산 데이터 불러오는 중...
trader-exit-empty-day = 최근 24시간 동안 종료된 거래가 없습니다
trader-exit-empty-all = 아직 종료된 거래가 없습니다
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
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = { $part } 매도
trader-stop-loss-part-of-position = 포지션의 일부
trader-stop-loss-sell-rest = 나머지 매도
trader-stop-loss-still-below = 여전히 손절가 아래
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
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = 활성화 시 확보되는 최소 수익: { $value }
trader-trailing-locked-minimum-warning = 활성화 시 스톱이 진입가 이하에 놓이며, 수익 중일 때만 매도합니다. 하한선이 필요하면 본전 스톱을 켜세요.
trader-trailing-activation = 활성화
trader-trailing-peak = 고점
# $value is a formatted percentage.
trader-trailing-final = 최종 +{ $value }%
# $value is a formatted percentage.
trader-trailing-summary-protected = 수익 <strong>{ $value }</strong> 보호됨
# $value is a formatted percentage.
trader-trailing-summary-avoided = 고점 대비 손실 <strong>{ $value }</strong> 회피
# $step is the band's position in the list, from 1.
trader-trailing-band = 구간 { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = 구간 { $step } 최고 수익
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = 구간 { $step } 트레일링 간격
trader-trailing-band-profit-caption = 최고 수익부터
trader-trailing-band-distance-caption = 간격
trader-trailing-add-band = 구간 추가
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = 구간 { $step } 삭제
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = 각 구간의 최고 수익은 { $min }%에서 { $max }% 사이여야 합니다
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = 각 구간의 간격은 { $min }%에서 { $max }% 사이여야 합니다
trader-trailing-band-error-order = 각 구간의 최고 수익은 활성화 값과 이전 구간보다 높아야 합니다
trader-trailing-band-error-distance = 각 구간의 간격은 그 구간의 최고 수익보다 작아야 합니다

## Break-even stop

trader-break-even-title = 본전 스톱
trader-break-even-subtitle = 포지션이 수익에 들어선 뒤 손실로 바뀌기 전에 전량 매도합니다
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = 마지막 매수 이후 최고가가 { $trigger }에 도달하면, { $offset }까지 되돌아올 때 전량 매도합니다.
## Take profit

trader-take-profit-title = 익절
trader-take-profit-subtitle = 수익이 각 수준에 도달할 때마다 포지션을 단계적으로 매도합니다
trader-take-profit-initial-buy = 최초 매수
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = { $step }단계
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = { $step }단계 수익 수준
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = { $step }단계 매도 비중
trader-take-profit-profit-caption = 수익 수준
trader-take-profit-sell-caption = 매도 비중
trader-take-profit-add-step = 단계 추가
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = { $step }단계 삭제
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = 각 수익 수준은 { $min }%에서 { $max }% 사이여야 합니다
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = 각 비중은 { $min }%에서 { $max }% 사이여야 합니다
trader-take-profit-error-order = 각 단계의 수익 수준은 이전 단계보다 높아야 합니다
# $max is the largest total share, the whole position.
trader-take-profit-error-total = 모든 단계를 합쳐 최대 { $max }%까지 매도할 수 있습니다
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = 포지션의 { $runner }%는 러너로 계속 보유합니다
trader-take-profit-runner-none = 마지막 단계가 포지션의 나머지를 매도합니다
trader-take-profit-runner-warning = 트레일링 스톱, 본전 스톱, 두 시간 청산, 모든 청산 전략이 꺼져 있으면 러너는 손절이나 수동으로만 청산됩니다
trader-take-profit-info = 비중은 최초 포지션 기준입니다. 가격이 한 번에 여러 수준을 넘으면 확인할 때마다 한 단계씩 매도합니다.
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = +{ $profit }%에서 { $share }% 매도
trader-take-profit-example-runner = 러너
trader-take-profit-example-runner-detail = 다른 청산 규칙에 맡깁니다
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = +{ $profit }% 수익까지 <strong>{ $sold }%</strong> 매도

## Time-based exit

trader-time-loss-title = 손실 포지션 청산
trader-time-subtitle = 최대 보유 시간이 지난 뒤 손실이 임계값을 넘으면 포지션을 자동으로 청산합니다
trader-time-flat-title = 횡보 포지션 청산
trader-time-flat-subtitle = 오르지도 내리지도 않는 포지션의 자금을 회수합니다
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = { $span } 이상 보유하고 손익이 ±{ $band } 이내인 포지션을 청산합니다.
trader-time-unit-seconds = 초
trader-time-unit-minutes = 분
trader-time-unit-hours = 시간
trader-time-unit-days = 일
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
trader-time-loss-hint = 보유 시간이 지난 뒤 시간 청산이 매도하는 손실 (40 = -40%에서 청산)
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
trader-time-positions-empty-message = 보유 중인 포지션이 보유 시간과 시간 제한 대비 ROI와 함께 여기에 표시됩니다.
trader-time-positions-token = 토큰
trader-time-positions-hold = 보유 시간
trader-time-positions-roi = ROI
trader-time-positions-rule = 시간 청산
trader-time-status-off = 시간 청산이 꺼져 있습니다
trader-time-status-none = 지금 적용되는 시간 청산이 없습니다
trader-time-status-not-managed = 자동 트레이더가 관리하지 않음
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = 손실 { $pnl }; { $remaining } 후 청산
trader-time-status-losing-due = 손실 { $pnl }; 다음 점검 때 청산
# $held and $remaining are formatted durations.
trader-time-status-flat = { $held } 동안 횡보; { $remaining } 후 청산
trader-time-status-flat-due = { $held } 동안 횡보; 다음 점검 때 청산

## Dollar-cost averaging

trader-dca-title = 분할 매수 (DCA)
trader-dca-subtitle = 손실 중인 포지션에 자동으로 추가 매수하여 평균 진입가를 낮춥니다
trader-dca-measure-average = 평균 진입가
trader-dca-measure-first-entry = 첫 진입가
trader-dca-step = 추가 { $step }
trader-dca-drop-label = 추가 { $step } 가격 하락
trader-dca-size-label = 추가 { $step } 크기, 첫 매수의 배수
trader-dca-drop-caption = 가격 하락
trader-dca-size-caption = 매수 × 첫 매수
trader-dca-add-step = 단계 추가
trader-dca-remove-step = 추가 { $step } 삭제
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = 최소 거래액 { $minimum } 미만: 이 추가 매수는 전송되지 않습니다
trader-dca-error-drop-range = 각 하락률은 { $min }%에서 { $max }% 사이여야 합니다
trader-dca-error-size-range = 각 크기는 첫 매수의 { $min }×에서 { $max }× 사이여야 합니다
trader-dca-error-order = 첫 진입가 기준일 때 각 추가는 이전보다 더 깊은 하락이 필요합니다
trader-dca-cooldown-impact = 추가 진입 사이 최소 { $span }
trader-dca-min-hold-none = 첫 추가 매수는 진입 직후 가능합니다
trader-dca-min-hold-impact = 첫 추가 매수는 진입 후 최소 { $span } 기다립니다
trader-dca-capacity = 이 포지션의 최대 보유액: { $amount } (첫 매수의 { $multiple }×)
trader-dca-info-title = DCA 전략 정보
trader-dca-info-subtitle = DCA 거래 시 유의할 점
trader-dca-how-title = DCA 작동 방식
trader-dca-how-trigger = <strong>트리거:</strong> 가격이 다음 단계 아래로 내려갈 때 (평균 또는 첫 진입가 기준)
trader-dca-how-action = <strong>동작:</strong> { -sol }을 추가 투입하여 평균 매입 원가를 낮춥니다
trader-dca-how-repeat = <strong>사다리:</strong> 추가할 때마다 다음 단계로 넘어가며, 모든 단계를 쓰면 추가가 멈춥니다
trader-dca-how-pause = <strong>일시 중지:</strong> 이전 추가 매수가 정산 중이거나 손실 한도가 신규 진입을 막는 동안에는 추가하지 않습니다
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
