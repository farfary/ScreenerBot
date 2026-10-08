copy-skip-not-buy-swap = 지갑 활동이 매수가 아니었습니다
copy-skip-task-disabled = 작업이 일시 중지되었습니다
copy-skip-mode-transition-required = 실행 모드는 별도로 변경해야 합니다
copy-skip-live-confirmation-required = 실거래 실행에는 확인이 필요합니다
copy-skip-unsupported-sizing-mode = 아직 지원하지 않는 규모 모드입니다
copy-skip-self-copy = 본인 소유의 지갑입니다
copy-skip-target-below-minimum = 지갑의 거래가 최소 기준 미만입니다
copy-skip-target-above-maximum = 지갑의 거래가 최대 기준을 초과했습니다
copy-skip-already-bought = 이미 매수한 토큰입니다 (1회 매수)
copy-skip-blacklisted = 위험 관리 정책으로 차단된 토큰입니다
copy-skip-filter-required = 필터링을 통과하지 못한 토큰입니다
copy-skip-budget-exhausted = 작업 예산을 모두 사용했습니다
copy-skip-token-cap-reached = 토큰별 한도에 도달했습니다
copy-skip-below-minimum-size = 카피 규모가 너무 작습니다
copy-skip-invalid-sizing = 작업 규모 설정이 올바르지 않습니다
copy-skip-invalid-slippage = 작업 슬리피지 설정이 올바르지 않습니다
copy-skip-invalid-exit-policy = 작업 청산 규칙이 올바르지 않습니다
copy-skip-invalid-price = 사용할 수 있는 시장 가격이 없습니다
copy-skip-not-sell-swap = 지갑 활동이 매도가 아니었습니다
copy-skip-exit-mode-disabled = 지갑 매도 무시됨: 작업이 자체 규칙으로 매도합니다
copy-skip-force-stopped = 거래가 강제 중지되었습니다
copy-skip-copy-position-not-found = 이 작업이 보유한 포지션이 없습니다
copy-skip-position-user-only = 사용자가 직접 관리하는 포지션입니다
copy-skip-position-management-mismatch = 이 포지션은 더 이상 카피 매도를 따르지 않습니다
copy-skip-latency-kill-switch = 자동 일시 중지: 거래가 너무 늦게 감지되었습니다
copy-skip-claim-reconciled-abandoned = 중단된 실거래 제출을 재시도 없이 종료했습니다
copy-skip-stale-observation = 중단 이후 재처리됨, 카피하기에는 너무 오래되었습니다
copy-skip-unknown-observation-time = 재처리된 거래에 블록 시간이 없습니다
copy-skip-entry-blocked = 진입 차단됨

copy-entry-block-force-stopped = 거래가 강제 중지되었습니다
copy-entry-block-loss-limit = 손실 한도로 신규 진입이 차단되었습니다
copy-entry-block-connectivity = 필요한 서비스를 사용할 수 없습니다
copy-entry-block-position-limit = 보유 포지션 한도에 도달했습니다
copy-entry-block-already-open = 이미 포지션이 열려 있습니다
copy-entry-block-reentry-cooldown = 토큰 재진입 쿨다운
copy-entry-block-open-cooldown = 전체 진입 쿨다운
copy-entry-block-entry-reserved = 다른 진입을 처리 중입니다
copy-entry-block-blacklisted = 위험 관리 정책으로 차단된 토큰입니다
copy-entry-block-check-failed = 안전 점검을 완료하지 못했습니다

copy-pause-user = 사용자가 일시 중지함
copy-pause-latency-kill-switch = 자동 일시 중지: 거래가 평균 { $average }초 늦게 도착했습니다 (한도 { $threshold }초)
copy-pause-watch-detached = 자동 일시 중지: 지갑을 더 이상 감시하지 않습니다
copy-pause-watch-budget-exceeded = 일시 중지: 이 지갑이 따라잡기 전에 서명 { $limit }건의 감시 확인 한도에 도달했습니다
copy-pause-helius-unavailable = 일시 중지: { -helius } 지갑 확인에 실패했습니다
copy-pause-watch-processing-failed = 일시 중지: 지갑 활동을 처리하지 못했습니다
copy-pause-unspecified = 일시 중지됨

copy-pause-short-user = 사용자
copy-pause-short-latency-kill-switch = 너무 느림
copy-pause-short-watch-detached = 감시 끊김
copy-pause-short-watch-budget-exceeded = 감시 한도
copy-pause-short-helius-unavailable = 감시 제공자
copy-pause-short-watch-processing-failed = 감시 처리
copy-state-paused = 일시 중지됨
copy-state-paused-reason = 일시 중지됨 · { $reason }

copy-readiness-history = 모의 거래 기록
copy-readiness-history-met =
    { $count ->
       *[other] 종료된 모의 라운드 { $count }개, 필요 { $needed }개
    }
copy-readiness-history-short = 종료된 모의 라운드 { $needed }개 중 { $count }개
copy-readiness-profit = 모의 거래 수익
copy-readiness-profit-detail =
    { $count ->
       *[other] { $count }개 라운드에서 { $realized } { -sol } 실현, { $wins }개 수익
    }
copy-readiness-latency = 거래 제때 감지
copy-readiness-latency-detail = p95 도착 { $p95 }초, 한도 { $limit }초
copy-readiness-latency-none = 아직 도착 샘플 없음
copy-readiness-priced = 모든 보유 수량 가격 확인
copy-readiness-priced-ok = 보유 중인 모든 모의 수량에 풀 가격이 있습니다
copy-readiness-priced-missing =
    { $count ->
       *[other] 풀 가격이 없는 보유 수량 { $count }건
    }
copy-readiness-runtime = 실거래 실행 가능
copy-readiness-runtime-ok = 설정과 안전 게이트가 실거래 카피를 허용합니다

copy-live-block-setup-incomplete = 먼저 지갑과 RPC 설정을 완료하세요
copy-live-block-force-stop = 긴급 중지가 작동 중입니다
copy-live-block-copy-trading-disabled = 카피 처리가 전역으로 일시 중지되었습니다
copy-live-block-unavailable = 실거래 실행을 사용할 수 없습니다

## Task state, mode and exit labels. Ids come from effective_state
## (src/trader/copy/control.rs), CopyMode, ExitMode and PaperExitRule
## (src/trader/copy/types.rs) plus the target_sell exit bucket.

copy-state-system-paused = 전역 일시 중지
copy-state-force-stopped = 강제 중지됨
copy-state-entries-blocked = 진입 차단됨
copy-state-running-live = 실행 중
copy-state-running-paper = 실행 중
copy-mode-paper = 모의
copy-mode-live = 실거래
copy-exit-mode-buy-only = 내 청산 규칙
copy-exit-mode-mirror = 지갑 매도 따라가기
copy-exit-mode-hybrid = 지갑 매도 및 내 규칙
copy-exit-target-sell = 지갑 매도
copy-exit-stop-loss = 손절
copy-exit-trailing-stop = 트레일링 스톱
copy-exit-take-profit = 익절
copy-exit-time-override = 시간 규칙
copy-exit-manual = 수동 종료

## Shared wording

copy-request-failed = 요청 실패
copy-keep-paused = 일시 중지 유지
copy-paused-suffix = · 일시 중지
copy-mode-paused = { $mode } · 일시 중지
copy-task-ref = "{ $name }" ({ $mode })
copy-metric-realized-pnl = 실현 손익
copy-metric-unrealized-pnl = 미실현 손익
copy-metric-win-rate = 승률
copy-metric-budget-spent = 사용한 예산
copy-metric-median-arrival = 도착 중앙값
copy-metric-open-holdings = 보유 수량
copy-record-won-lost = { $won }승 · { $lost }패
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = 체결
copy-kind-exits = 청산
copy-kind-skips = 건너뜀
copy-kind-errors = 오류
copy-field-per-trade-cap = 거래당 한도
copy-field-per-token-cap = 토큰당 한도
copy-field-total-budget = 총 예산
copy-field-slippage = 슬리피지
copy-rules-wallet-sells-only = 지갑 매도만
copy-filter-copy-setting-required = 카피 설정 (필수)
copy-filter-copy-setting-not-required = 카피 설정 (필수 아님)
copy-count-closed-rounds =
    { $count ->
       *[other] 종료된 라운드 { $count }개
    }
copy-count-open-holdings =
    { $count ->
       *[other] 보유 수량 { $count }건
    }
copy-unrealized-partial =
    { $priced ->
       *[other] 가격 확인 { $priced }건 · 가격 없음 { $unpriced }건
    }
copy-unrealized-unpriced =
    { $count ->
       *[other] 가격 없는 보유 수량 { $count }건
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = 전체
copy-range-label =
    .aria-label = 기간

## Page strip (pages/copy.html, pages/copy/summary.js)

copy-page-title = 카피 트레이딩
copy-page-beta = 베타
copy-strip-loading = 불러오는 중
copy-strip-unavailable = 사용할 수 없음
copy-strip-setup-required = 설정 필요 · 카피 트레이딩에는 지갑과 RPC가 필요합니다
copy-strip-pause-all = 모두 일시 중지
copy-strip-resume = 처리 재개
copy-strip-settings = 설정
copy-strip-add-wallet = 지갑 추가
copy-strip-paused-globally = 전역 일시 중지 · 신규 카피 없음, 청산은 계속 실행
copy-strip-force-stopped = 강제 중지됨 · 카피하지 않음
copy-strip-loss-limit = 손실 한도 · 신규 진입 차단, 청산은 계속 실행
copy-strip-idle-paused =
    { $count ->
       *[other] 대기 중 · 일시 중지된 작업 { $count }개
    }
copy-strip-idle-empty = 대기 중 · 아직 작업 없음
copy-strip-processing = 처리 중 · 모의 { $paper }
copy-strip-processing-live = 처리 중 · 실거래 { $live } · 모의 { $paper }
copy-figures-label =
    .aria-label = 카피 트레이딩 합계
copy-figure-marked-at-pool = 풀 가격 기준 평가
copy-figure-across-tasks = 모든 작업 합계
copy-figure-budget-lifetime = 사용 중인 작업의 누적 지출
copy-figure-budget-none = 사용 중인 작업 없음
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
       *[other] 거래 { $count }건
    }
copy-figure-arrival-none = 사용 중인 작업의 샘플 없음

## Page frame (pages/copy.js)

copy-load-failed = 카피 트레이딩을 불러오지 못했습니다: { $error }
copy-resume-all-title = 카피 처리 재개
copy-resume-all-message =
    { $count ->
       *[other] 실거래 작업 { $count }개는 지갑이 다시 거래하면 실제 스왑을 제출합니다.
    }
copy-toast-resumed-all = 카피 처리를 재개했습니다
copy-toast-paused-all = 모든 카피 처리를 일시 중지했습니다
copy-toast-global-failed = 카피 처리 상태를 변경하지 못했습니다

## Onboarding (pages/copy.html)

copy-onboarding-title = 신뢰하는 지갑을 카피하되, 먼저 모의 거래로 검증하세요
copy-onboarding-body = 모든 작업은 모의 거래로 시작합니다. 대상 지갑의 거래는 풀 가격에 설정한 슬리피지와 수수료를 반영해 시뮬레이션되며, 청산 규칙은 모의 장부에서 실행됩니다. 모의 결과가 충분하면 지갑별로 실거래를 활성화하세요.
copy-onboarding-add = 첫 지갑 추가
copy-setup-gate-title = 카피 트레이딩에는 지갑이 필요합니다
copy-onboarding-observe = 관찰
copy-onboarding-observe-detail = { -sol }을 사용하지 않고 지갑의 스왑을 감지합니다.
copy-onboarding-evaluate = 평가
copy-onboarding-evaluate-detail = 모의 손익, 승률, 건너뜀, 감지 속도, 슬리피지를 확인합니다.
copy-onboarding-arm = 활성화
copy-onboarding-arm-detail = 준비 상태 점검을 통과한 뒤 실제 스왑을 사용합니다.

## Wallet list (pages/copy.html, pages/copy/list.js)

copy-list-label =
    .aria-label = 카피 중인 지갑
copy-list-title = 지갑
copy-list-compare = 비교
copy-list-sort-label = 지갑 정렬
copy-list-count = 활성 { $active }개 · 전체 { $total }개
copy-sort-pnl = 손익
copy-sort-state = 상태
copy-sort-name = 이름
copy-compare-label =
    .aria-label = 지갑 비교

## Dialog chrome (pages/copy.html)

copy-dialog-close =
    .aria-label = 닫기
copy-editor-title-add = 지갑 추가
copy-editor-sub-add = 새 작업은 모의 거래로 시작합니다
copy-arm-title = 실거래 카피 활성화
copy-arm-sub = 내 지갑으로 실제 스왑 실행
copy-arm-keep-paper = 모의 유지
copy-arm-confirm = 실거래 활성화
copy-profile-title = 지갑 프로필
copy-profile-sub = 이 봇이 관찰한 지갑 정보

## Settings dialog (pages/copy.html, pages/copy/settings.js). Field labels and hints
## come from the config catalog.

copy-settings-title = 카피 트레이딩 설정
copy-settings-subtitle = 모든 작업에 적용되는 전역 정책
copy-settings-filter-warning = 기본 필터링 설정에서는 거의 모든 토큰이 제외되어 아무것도 카피되지 않습니다. 지갑이 거래하는 토큰을 필터가 통과시키는 경우가 아니라면 꺼 두세요.
copy-settings-unit-seconds = 초
copy-settings-unit-trades = 건
copy-settings-unit-tasks = 개
copy-settings-unit-closed-rounds = 개
copy-settings-save = 설정 저장
copy-settings-load-failed = 카피 설정을 불러오지 못했습니다
copy-settings-saved = 카피 트레이딩 설정이 저장되었습니다

## Workspace (pages/copy/workspace.js)

copy-tab-overview = 개요
copy-tab-holdings = 보유 수량
copy-tab-activity = 활동
copy-tab-rules = 규칙
copy-tab-execution = 실행
copy-tabs-label = 작업 보기
copy-workspace-select = 지갑을 선택하면 워크스페이스가 열립니다.
copy-workspace-loading = 작업 불러오는 중…
copy-workspace-load-failed = 이 작업을 불러오지 못했습니다: { $error }

copy-state-detail-paper = 모의 실행 중 · 거래는 시뮬레이션되며 자금은 사용되지 않습니다
copy-state-detail-live = 실거래 실행 중 · 지갑 거래를 실제 스왑으로 카피합니다
copy-state-detail-system-paused = 대기 중 · 카피 처리가 전역으로 일시 중지되었으며 청산은 계속 실행됩니다
copy-state-detail-entries-blocked = 손실 한도로 진입 차단됨 · 청산은 계속 실행됩니다
copy-state-detail-force-stopped = 강제 중지됨 · 카피하지 않음

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = 재개해도 같은 한도가 유지되므로 거래가 계속 늦게 도착하면 다시 일시 중지됩니다. RPC 스트림을 확인하거나 설정에서 도착 한도를 높이세요.
copy-paused-resume-detached = 재개하면 지갑을 다시 감시합니다.
copy-paused-holdings-rules =
    { $count ->
       *[other] 청산 규칙은 보유 수량 { $count }건에 계속 적용됩니다.
    }
copy-paused-holdings-mirror =
    { $count ->
       *[other] 지갑의 매도는 보유 수량 { $count }건에 계속 적용됩니다.
    }
copy-paused-holdings-hybrid =
    { $count ->
       *[other] 지갑의 매도와 청산 규칙은 보유 수량 { $count }건에 계속 적용됩니다.
    }

copy-watch-state-catching-up = 지갑 감시: 따라잡는 중. { -helius }로 이 지갑을 확인하고 있습니다.
copy-watch-state-watching = 지갑 감시: 감시 중. { -helius }로 이 지갑을 확인하고 있습니다.
copy-watch-last-check = 마지막 확인: { $ago }.
copy-watch-recovery-active = 지갑 감시 활성
copy-watch-recovery-catching-up = 지갑 감시가 따라잡는 중입니다
copy-watch-recovery-still-paused = 카피 작업은 아직 일시 중지 상태입니다. 준비되면 카피를 재개하세요.
copy-watch-recovery-title = 지갑 감시 복원
copy-watch-recovery-processing-failed = 지갑 활동을 처리하지 못했습니다. 저장된 진행 상태는 유지됩니다. 문제를 해결한 뒤 다시 시도하세요.
copy-watch-recovery-provider-failed = { -helius } 확인에 실패했습니다. 저장된 진행 상태는 유지됩니다. 제공자를 사용할 수 있게 되면 다시 시도하세요.
copy-watch-recovery-budget-intro = 이 지갑은 현재 감시 범위로 확인할 수 있는 것보다 활동이 많습니다. 계속할 방법을 선택하세요.
copy-watch-approve = { -helius }로 따라잡기 시도
copy-watch-approve-help = 저장된 진행 상태에서 이어서 진행합니다. { -helius } 크레딧을 더 사용할 수 있으며 여전히 뒤처질 수 있습니다.
copy-watch-approve-unavailable = { -helius } 따라잡기를 사용할 수 없습니다. 확인하지 못한 활동을 건너뛰지 않고 계속하려면 사용 설정된 { -helius } RPC 엔드포인트를 구성하세요.
copy-watch-no-provider = 이 감시에는 지원되는 따라잡기 제공자가 없습니다.
copy-watch-budget-label = 확인당 확인할 서명 수
copy-watch-budget-hint = 또는 확인하지 못한 활동을 건너뛰고 지금부터 재개합니다. 확인당 서명 { $min }–{ $max }건 중에서 선택하세요. 한도가 높으면 RPC 호출이 더 많아질 수 있습니다.
copy-watch-ack = 놓친 활동은 카피되지 않음을 이해했습니다.
copy-watch-toast-range = 폴링당 서명 수를 { $min }건에서 { $max }건 사이에서 { $step }건 단위로 선택하세요
copy-watch-toast-ack = 마지막으로 완료된 확인 이후의 서명은 건너뛴다는 점에 동의하세요
copy-watch-resumed = 지갑 감시를 지금부터 재개했습니다. 카피 작업은 일시 중지 상태를 유지합니다
copy-watch-resume-failed = 지갑 감시를 재개하지 못했습니다
copy-watch-retry-started = 저장된 진행 상태부터 지갑 감시를 다시 시도합니다. 카피 작업은 일시 중지 상태를 유지합니다
copy-watch-retry-failed = 지갑 감시를 다시 시도하지 못했습니다
copy-watch-approve-title = 이 지갑에 { -helius } 따라잡기 허용
copy-watch-approve-message = { -helius }는 확인하지 못한 구간을 건너뛰지 않고 저장된 진행 상태부터 성공한 Solana 트랜잭션을 확인할 수 있습니다. 현재 전체 트랜잭션 100건이 반환될 때마다 10 크레딧(올림)이 차감되며, 요청당 최소 10 크레딧입니다. 한 번의 확인에 여러 요청이 발생할 수 있으며 사용량과 제공자 요금은 달라질 수 있습니다. 별도로 재개하기 전까지 카피는 일시 중지 상태를 유지합니다.
copy-watch-approve-confirm = 이 지갑에 허용
copy-watch-approved = 저장된 진행 상태부터 지갑 감시를 시작했습니다. 카피 작업은 일시 중지 상태를 유지합니다
copy-watch-restore-failed = 지갑 감시를 복원하지 못했습니다

copy-action-pause = 일시 중지
copy-action-resume = 재개
copy-action-resume-copy = 카피 재개
copy-action-resume-from-now = 지금부터 재개
copy-action-retry-watch = 지갑 감시 다시 시도
copy-action-return-paper = 모의로 전환
copy-action-edit-rules = 규칙 편집
copy-action-clone = 복제
copy-action-profile = 지갑 프로필
copy-resume-live-title = 실거래 카피 재개
copy-resume-live-message = 이 지갑이 다시 거래하면 "{ $name }" 작업이 내 지갑으로 실제 스왑을 제출합니다.
copy-resume-live-confirm = 실거래 재개
copy-task-resumed = 작업을 재개했습니다
copy-task-paused = 작업을 일시 중지했습니다
copy-task-state-failed = 작업 상태를 변경하지 못했습니다
copy-return-paper-message = "{ $name }" 작업의 새 카피는 { -sol }을 사용하지 않고 다시 시뮬레이션됩니다.
copy-return-paper-cancel = 실거래 유지
copy-task-returned-paper = 작업을 모의로 전환했습니다
copy-mode-change-failed = 실행 모드를 변경하지 못했습니다
copy-delete-title = 카피 작업 삭제
copy-delete-message = "{ $name }" 작업을 삭제하시겠습니까? 작업의 결정과 모의 결과가 삭제되며 이 작업을 위한 지갑 감시도 중단됩니다.
copy-delete-confirm = 작업 삭제
copy-delete-cancel = 작업 유지
copy-task-deleted = 카피 작업을 삭제했습니다
copy-task-delete-failed = 카피 작업을 삭제하지 못했습니다

## Overview tab (pages/copy/overview.js)

copy-overview-results = 결과
copy-analytics-load-failed = 분석을 불러오지 못했습니다: { $error }
copy-analytics-loading = 분석 불러오는 중…
copy-exit-bucket =
    { $count ->
       *[other] 매도 { $count }건 · { $pnl }
    }
copy-overview-average-win = 평균 수익
copy-overview-average-loss = 평균 손실 { $amount }
copy-overview-profit-factor = 프로핏 팩터
copy-overview-profit-factor-note = 총수익 ÷ 총손실
copy-overview-average-hold = 평균 보유 시간
copy-overview-average-hold-note = 진입부터 청산까지
copy-overview-best-round = 최고 라운드
copy-overview-worst-round = 최저 { $amount }
copy-overview-curve-title = 누적 손익
copy-overview-exits-title = 청산 유형별 매도
copy-overview-skips-title = 거래를 건너뛴 이유
copy-book-title-live = 실거래 장부
copy-book-title-paper = 모의 장부
copy-book-all-time = 전체 기간
copy-book-buys =
    { $count ->
       *[other] 매수 <strong>{ $count }</strong>건
    }
copy-book-policy-exits =
    { $count ->
       *[other] 내 규칙에 따른 청산 <strong>{ $count }</strong>건
    }
copy-book-wallet-sells =
    { $count ->
       *[other] 지갑 매도 <strong>{ $count }</strong>건
    }
copy-book-manual-closes = 수동 종료 <strong>{ $count }</strong>건
copy-book-skipped = 건너뜀 <strong>{ $count }</strong>건
copy-book-failed = 실패 <strong>{ $count }</strong>건
copy-book-closed = 종료 { $count }건
copy-book-budget-note = { $mode } 지출 (총 { $total }) · 잔여 { $remaining }
copy-check-passed = 통과
copy-check-not-passed = 미통과
copy-readiness-title = 실거래 전 점검
copy-readiness-live-note = 이 작업은 실거래 중입니다. 위 헤더에서 모의로 전환할 수 있습니다.
copy-readiness-all-pass = 모든 점검을 통과했습니다.
copy-readiness-needs-review = 활성화하려면 준비되지 않은 항목을 직접 검토해야 합니다.
copy-readiness-arm = 검토 후 실거래 활성화

## Rules tab and review (pages/copy/rules.js)

copy-rules-title = 적용 중인 규칙
copy-rules-size-ratio = 지갑 거래의 { $pct }
copy-rules-size-fixed = 카피당 { $amount }
copy-rules-target-any = 모든 규모
copy-rules-target-min = 최소 { $amount }
copy-rules-target-max = 최대 { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = 작업 재정의 · 트레이더 { $value }
copy-rules-source-default = 트레이더 기본값
copy-rules-not-used = 사용 안 함: 지갑의 매도가 결정합니다
copy-rules-col-rule = 규칙
copy-rules-col-applies = 적용 값
copy-rules-col-source = 출처
copy-rules-budget-note = { $mode } 지출 { $spent } · 잔여 { $remaining }
copy-rules-token-copies =
    { $count ->
       *[other] 토큰 하나당 전체 규모 약 { $count }회 카피
    }
copy-rules-sizing = 규모 설정
copy-rules-copy-size = 카피 규모
copy-rules-entry-filters = 진입 필터
copy-rules-target-size = 지갑 거래 규모
copy-rules-repeat-buys = 반복 매수
copy-rules-repeat-first-only = 토큰별 첫 매수만
copy-rules-repeat-every = 모든 매수 (토큰당 한도까지)
copy-rules-filter-pass = 필터링 통과
copy-rules-filter-required = 필수
copy-rules-filter-not-required = 필수 아님
copy-rules-filter-task-override = 작업 재정의
copy-rules-exits = 청산
copy-rules-exits-inactive = 보유 수량은 지갑이 매도할 때만 매도됩니다. 이 모드에서는 아래 규칙이 실행되지 않습니다.

## Exit rules (pages/copy/policy.js)

copy-rule-status = 상태
copy-rule-on = 사용
copy-rule-off = 사용 안 함
copy-rule-unit-seconds = 초
copy-rule-unit-minutes = 분
copy-rule-stop-loss-threshold = 매도 손실률
copy-rule-stop-loss-min-hold = 최소 보유 후 적용
copy-rule-no-minimum = 최소 없음
copy-rule-partial-exits = 부분 청산
copy-rule-partial-allowed = 허용
copy-rule-partial-full-only = 전량 청산만
copy-rule-partial-size = 부분 청산 규모
copy-rule-trailing-activation = 활성화 수익률
copy-rule-trailing-distance = 고점 대비 매도 하락폭
copy-rule-take-profit-target = 매도 수익률
copy-rule-time-duration = 보유 후 확인 시점
copy-rule-time-threshold = 손익이 다음 이하일 때 매도
copy-preset-inherit = 트레이더 기본값
copy-preset-conservative = 보수적
copy-preset-balanced = 균형
copy-preset-aggressive = 공격적
copy-preset-custom = 사용자 지정
copy-validate-stop-loss = 손절은 0%보다 크고 100% 이하여야 합니다.
copy-validate-partial-size = 부분 청산 규모는 0%에서 100% 사이여야 합니다.
copy-validate-min-hold = 최소 보유 시간은 초 단위 정수여야 합니다.
copy-validate-trailing-activation = 트레일링 활성화는 0%보다 크고 100% 이하여야 합니다.
copy-validate-trailing-distance = 트레일링 거리는 0%보다 크고 100% 이하여야 합니다.
copy-validate-take-profit = 익절은 0%보다 커야 합니다.
copy-validate-time-duration = 시간 규칙에는 0보다 큰 기간이 필요합니다.
copy-validate-time-threshold = 시간 규칙의 기준은 손실입니다. 0% 또는 음수를 사용하세요.
copy-warning-mirror = 지갑의 매도만 보유 수량을 종료합니다. 손절로 보호되지 않으며, 지갑이 매도하지 않는 토큰은 계속 보유됩니다.
copy-warning-no-rules = 청산 규칙이 없고 지갑 매도도 무시됩니다. 보유 수량이 매도되지 않습니다.
copy-warning-no-stop-loss = 적용되는 손절이 없습니다. 하락하는 토큰은 다른 규칙이나 지갑의 매도가 있을 때까지 보유됩니다.
copy-warning-stop-delay = 손절은 매수 후 { $hold } 동안 대기합니다. 그보다 빨리 하락하는 토큰은 { $threshold }보다 훨씬 낮은 가격에서 종료됩니다.
copy-warning-take-profit-cost = { $target } 익절은 매도 비용(슬리피지 { $slippage }, 스왑 수수료 { $fee })을 충당하지 못해 손실로 라운드가 종료됩니다.
copy-warning-trailing-distance = 트레일링 거리가 활성화 수익률 이상이므로 활성화된 트레일링이 진입가보다 낮은 가격에 매도할 수 있습니다.

## Execution tab (pages/copy/execution.js)

copy-execution-title = 실행 품질
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = 전체
copy-execution-limit-on =
    { $count ->
       *[other] 최근 거래 { $count }건 평균이 { $limit }을 넘으면 일시 중지
    }
copy-execution-limit-off = 킬 스위치 꺼짐
copy-execution-arrival-samples =
    { $count ->
       *[other] 실시간으로 확인한 거래 { $count }건
    }
copy-execution-p95 = p95 도착
copy-execution-median-slippage = 슬리피지 중앙값
copy-execution-slippage-samples =
    { $count ->
       *[other] 측정된 체결 { $count }건
    }
copy-execution-worst-slippage = 최대 슬리피지
copy-execution-average-slippage = 평균 { $amount }
copy-execution-delay-title = 감지 지연
copy-execution-delay-note = 지갑의 블록 시점부터 이 봇이 거래를 확인하기까지의 시간입니다. 중단 후 재처리분은 제외됩니다.
copy-execution-delay-limit = 도착 한도 { $limit }를 넘는 막대는 주황색입니다.
copy-execution-fastest = 가장 빠름
copy-execution-average = 평균
copy-execution-slowest = 가장 느림
copy-execution-fill-title = 지갑 대비 체결
copy-execution-fill-note = 양수는 지갑보다 불리하다는 뜻입니다. 매수는 더 많이 지불했고, 따라가는 매도는 더 적게 받았습니다. 풀 가격이 없는 토큰의 모의 체결은 지갑 자신의 거래 가격으로 계산되므로 측정할 것이 없어 제외됩니다.
copy-execution-samples = 샘플
copy-execution-median = 중앙값
copy-execution-worst = 최대
copy-execution-decisions = 기간 내 결정

## Compare view (pages/copy/compare.js)

copy-compare-title = 지갑 비교
copy-compare-back = 지갑으로 돌아가기
copy-compare-load-failed = 비교를 불러오지 못했습니다: { $error }
copy-compare-loading = 비교 불러오는 중…
copy-compare-empty = 비교할 작업이 없습니다.
copy-compare-curve-title = 누적 실현 손익
copy-table-wallet = 지갑
copy-table-mode = 모드
copy-table-rounds = 라운드
copy-table-realized = 실현
copy-table-profit-factor = 프로핏 팩터
copy-table-average-hold = 평균 보유
copy-table-median-slippage = 슬리피지 중앙값

## Charts (pages/copy/charts.js)

copy-chart-curve-label = 누적 손익 { $amount } { -sol }
copy-chart-compare-label = 작업별 누적 손익
copy-chart-empty-curve = 이 기간에 종료된 라운드가 아직 없습니다.
copy-chart-empty-bars = 이 기간에 기록된 내용이 없습니다.
copy-chart-empty-histogram = 이 기간에 도착 샘플이 없습니다.
copy-chart-empty-compare = 이 기간에 비교할 종료 라운드가 없습니다.
copy-chart-histogram-title = { $total }개 중 { $count }개

## Wallet profile (pages/copy/profile.js)

copy-profile-copy = 이 지갑 카피
copy-profile-copy-other = 다른 규칙으로 카피
copy-profile-loading = 지갑 프로필 불러오는 중…
copy-profile-watch-title = 감시
copy-profile-watched = 감시 중
copy-profile-watch-resume-hint = 작업을 재개하면 다시 감시합니다
copy-profile-watch-add-hint = 작업을 추가하면 감시를 시작합니다
copy-profile-stream = 스트림
copy-profile-subscribed = 구독 중
copy-profile-not-subscribed = 구독 안 함
copy-profile-sources =
    { $count ->
       *[other] 소스 { $count }개
    }
copy-profile-last-activity = 마지막 활동
copy-profile-last-error = 마지막 오류
copy-profile-own-wallet = 본인 소유의 지갑이므로 카피할 수 없습니다.
copy-profile-observed-title = 관찰된 거래
copy-profile-observed-none = 이 봇에서 관찰된 이 지갑의 거래가 아직 없습니다. 모의 작업은 { -sol }을 사용하지 않고 관찰합니다.
copy-profile-swaps-seen = 확인된 스왑
copy-profile-swaps-seen-note = 내 작업 전체에서 고유한 지갑 스왑 수
copy-profile-buys-sells = 매수 / 매도
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = 거래한 토큰
copy-profile-first-seen = 처음 확인
copy-profile-last-seen = 마지막 확인
copy-profile-tasks-title = 이 지갑에 대한 내 작업
copy-table-task = 작업

## Arm live dialog (pages/copy/arm_gate.js)

copy-arm-acks-left =
    { $count ->
       *[other] 확인할 동의 항목 { $count }개 남음
    }
copy-arm-readiness-title = 모의 장부 기준 준비 상태
copy-arm-exposure-title = 노출
copy-arm-per-copy = 카피당
copy-arm-budget-left-value = { $left } / { $total } { -sol }
copy-arm-budget-left = 남은 실거래 예산
copy-arm-budget-left-note = 모의 지출은 별도로 계산되며 이 예산을 사용하지 않습니다
copy-arm-exits = 청산
copy-arm-stop-note = { $hold } 보유 전에는 적용되지 않음: 더 빠른 하락은 더 낮은 가격에서 종료됩니다
copy-arm-shared = 이 지갑은 다음 작업에서도 카피합니다: { $tasks }. 각 작업은 자체 예산으로 거래를 카피합니다.
copy-arm-unavailable = 현재 실거래 실행을 사용할 수 없습니다. 마지막 점검 결과를 확인하세요.
copy-arm-ack-real-native = 실제 { -sol }: 이 작업은 내 지갑에서 최대 { $budget } { -sol }을 사용할 수 있으며, 카피당 최대 { $trade } { -sol }입니다.
copy-arm-ack-fees = 실거래 카피에는 실제 네트워크 수수료와 슬리피지가 발생하며, 모의 결과가 실거래 결과를 보장하지 않습니다.
copy-arm-ack-unready = 일부 준비 상태 점검을 통과하지 못했습니다. 그래도 이 작업을 활성화합니다.
copy-arm-lead = "{ $name }" 작업은 이 지갑의 거래를 내 지갑의 실제 스왑으로 카피합니다.
copy-arm-confirmation-missing = 실거래 확인 정보를 불러오지 못했습니다
copy-arm-armed = 실거래 카피를 활성화했습니다
copy-arm-failed = 실거래 카피를 활성화하지 못했습니다

## Holdings tab (pages/copy/holdings.js)

copy-holdings-title = 보유 수량
copy-holdings-view-label = 보유 수량 보기
copy-holdings-view-open = 보유 중 ({ $count })
copy-holdings-view-closed = 종료된 라운드 ({ $count })
copy-holdings-reset = 모의 장부 초기화
copy-holdings-live-note = 실거래 카피는 실제 포지션입니다.
copy-holdings-open-positions = 보유 포지션
copy-holdings-token-details = 토큰 상세 열기
copy-holdings-opened = 진입 { $time }
copy-holdings-no-pool-price = 풀 가격 없음
copy-holdings-close = 종료
copy-holdings-write-off = 상각
copy-holdings-activity = 활동
copy-holdings-no-exit-rule = 청산 규칙 없음
copy-holdings-watch-stop = 손절 { $level }
copy-holdings-watch-stop-until = 손절 { $level } ({ $span } 후)
copy-holdings-watch-take = 익절 { $level }
copy-holdings-watch-trail = 트레일링 { $level }
copy-holdings-watch-trail-arms = 트레일링 활성화 { $level }
copy-holdings-watch-time = 시간 ≤ { $level }
copy-holdings-watch-time-until = 시간 ≤ { $level } ({ $span } 후)
copy-holdings-watch-wallet-sells = 지갑 매도
copy-holdings-empty = 보유 중인 모의 수량이 없습니다. 지갑에서 카피한 매수가 여기에 표시됩니다.
copy-holdings-col-token = 토큰
copy-holdings-col-cost = 원가
copy-holdings-col-entry = 진입
copy-holdings-col-mark = 평가가
copy-holdings-col-peak = 고점
copy-holdings-col-pnl = 손익
copy-holdings-col-exit-rules = 청산 규칙
copy-holdings-col-held = 보유
copy-holdings-col-actions = 작업
copy-holdings-col-invested = 투자금
copy-holdings-col-proceeds = 회수액
copy-holdings-col-exit = 청산
copy-holdings-col-closed = 종료
copy-holdings-price-note = 가격은 토큰당 { -sol }입니다. 진입가에는 매수 시 슬리피지와 수수료가 포함되며, 고점과 청산 기준은 진입가를 기준으로 하므로 보유 수량은 고점이 진입가보다 낮은 상태로 시작합니다. 마우스를 올리면 풀 가격을 볼 수 있습니다.
copy-holdings-paused-rules = 일시 중지됨: 신규 카피 없음. 내 청산 규칙은 이 보유 수량을 계속 종료합니다.
copy-holdings-paused-mirror = 일시 중지됨: 신규 카피 없음. 지갑의 매도는 이 보유 수량을 계속 종료합니다.
copy-holdings-paused-hybrid = 일시 중지됨: 신규 카피 없음. 지갑의 매도와 내 청산 규칙은 이 보유 수량을 계속 종료합니다.
copy-holdings-closed-load-failed = 종료된 라운드를 불러오지 못했습니다: { $error }
copy-holdings-closed-loading = 종료된 라운드 불러오는 중…
copy-holdings-closed-empty = 종료된 라운드가 아직 없습니다.
copy-holdings-closed-latest = 최근 { $shown }개 (전체 { $total }개 라운드)
copy-holdings-close-title = 모의 보유 수량 종료
copy-holdings-close-message = 모의 장부에서 풀 가격 { $price } 기준으로 작업의 슬리피지와 수수료를 적용해 매도합니다. 토큰: { $token }
copy-holdings-close-confirm = 보유 수량 종료
copy-holdings-write-off-title = 모의 보유 수량 상각
copy-holdings-write-off-message = 매도할 풀 가격이 없는 토큰입니다. 상각하면 0으로 종료되고 원가 { $cost }이 손실로 기록됩니다. 토큰: { $token }
copy-holdings-keep = 유지
copy-holdings-written-off = 상각 완료: { $token }
copy-holdings-closed = 종료 완료: { $token }
copy-holdings-written-off-detail = 회수액 0으로 종료됨
copy-holdings-sold-at = 매도가 { $price }
copy-holdings-close-failed = 보유 수량을 종료하지 못했습니다
copy-holdings-reset-message = "{ $name }" 작업을 처음부터 다시 시작합니다. 모의 보유 수량, 지출, 체결, 청산, 건너뜀 기록이 삭제되며 규칙과 지갑은 유지됩니다.
copy-holdings-reset-cancel = 기록 유지
copy-holdings-reset-done = 모의 장부를 초기화했습니다
copy-holdings-reset-detail =
    { $count ->
       *[other] 결정 { $count }건 삭제됨
    }
copy-holdings-reset-failed = 모의 장부를 초기화하지 못했습니다

## Activity tab (pages/copy/activity.js). Ids come from CopyOutcome
## (src/trader/copy/types.rs).

copy-activity-title = 활동
copy-activity-filter-label = 활동 필터
copy-filter-all = 전체
copy-outcome-paper-filled = 모의 매수
copy-outcome-live-submitted = 실거래 매수 제출됨
copy-outcome-live-confirmed = 실거래 매수 확정됨
copy-outcome-live-failed = 실거래 매수 실패
copy-outcome-paper-sell-observed = 모의 매도 · 지갑 매도
copy-outcome-live-sell-submitted = 실거래 매도 제출됨
copy-outcome-live-sell-failed = 실거래 매도 실패
copy-outcome-skipped = 건너뜀
copy-activity-decision = 결정
copy-activity-paper-exit = 모의 청산 · { $rule }
copy-activity-filled = { $input }, 가격 { $price } · 지갑 매수 { $target }
copy-activity-filled-slippage = { $input }, 가격 { $price } · 지갑 매수 { $target } · 슬리피지 { $slippage }
copy-activity-filled-unpriced = { $input }, 가격 { $price } · 지갑 매수 { $target } · 풀 가격이 없어 지갑의 거래 가격 적용
copy-activity-live-sized = { $sized } · 지갑 매수 { $target }
copy-activity-sell-nothing = 지갑 매도 { $amount } · 매도할 보유 수량 없음
copy-activity-written-off = 0으로 상각: 풀 가격 없음
copy-activity-sold = 토큰 { $tokens }개 매도, 가격 { $price }, 회수 { $proceeds }
copy-activity-full-close = 전량 종료
copy-activity-partial-exit = { $pct } 청산
copy-activity-skip-detail = { $label } ({ $detail })
copy-activity-skip-minimum-size = 최소 { $amount }
copy-activity-skip-maximum = 최대 { $value }
copy-activity-skip-stale = { $arrival } 지연, 한도 { $limit }
copy-activity-skip-latency = 평균 { $average }, 한도 { $limit }
copy-activity-arrival-replayed = 블록 { $span } 후 재처리됨
copy-activity-arrival-seen = 블록 { $span } 후 확인됨
copy-activity-link-wallet-tx = 지갑 트랜잭션
copy-activity-link-own-tx = 내 트랜잭션
copy-activity-only-token = 이 토큰만
copy-activity-skipped-group = 건너뜀 ×{ $count }
copy-activity-group-detail =
    { $tokens ->
       *[other] 토큰 { $tokens }개 · { $since }부터
    }
copy-activity-mint-filter =
    .placeholder = 토큰 민트
    .aria-label = 토큰 민트로 필터
copy-activity-clear = 지우기
copy-activity-load-failed = 활동을 불러오지 못했습니다: { $error }
copy-activity-loading = 활동 불러오는 중…
copy-activity-no-match = 이 필터와 일치하는 항목이 없습니다.
copy-activity-empty = 아직 결정이 없습니다. 지갑이 거래하면 체결, 청산, 건너뜀이 여기에 표시됩니다.
copy-activity-load-older = 이전 항목 불러오기
copy-activity-start = 기록의 시작
copy-activity-older-failed = 이전 활동을 불러오지 못했습니다

## Task editor (pages/copy/editor.js, pages/copy/editor_steps.js)

copy-step-wallet = 지갑
copy-step-sizing = 규모
copy-step-entry = 진입 필터
copy-step-exits = 청산
copy-step-review = 검토
copy-editor-title-edit = 편집: { $name }
copy-editor-title-clone = 복제: { $name }
copy-editor-sub-edit = { $mode } 작업 · 변경 사항은 다음 결정부터 적용됩니다
copy-editor-sub-clone = 같은 규칙, 빈 모의 장부, 모의로 시작
copy-editor-save-edit = 변경 사항 저장
copy-editor-save-clone = 복제본 만들기
copy-editor-save-create = 모의 작업 만들기
copy-editor-clone-suffix = (복사본)
copy-editor-discard-edit = 변경 사항 취소
copy-editor-discard-create = 이 작업 취소
copy-editor-discard-edit-message = "{ $name }" 작업의 변경 사항이 저장되지 않았습니다.
copy-editor-discard-create-message = 지금까지 입력한 지갑과 규칙이 저장되지 않았습니다.
copy-editor-discard-confirm = 취소
copy-editor-keep-editing = 계속 편집
copy-editor-toast-updated = 작업을 업데이트했습니다
copy-editor-toast-clone = 복제본을 만들었습니다
copy-editor-toast-created = 모의 작업을 만들었습니다
copy-unit-native = { -sol }
copy-editor-any = 제한 없음
copy-editor-duplicate = 이미 카피 중인 작업: { $tasks }. 이 작업은 자체 규칙과 예산으로 같은 거래를 다시 카피합니다.
copy-editor-wallet = 지갑
copy-editor-wallet-identity = 작업의 지갑이 곧 작업의 식별 정보입니다. 이 규칙으로 다른 지갑을 카피하려면 작업을 복제하세요.
copy-editor-address-label = 지갑 주소
copy-editor-address-placeholder = Solana 지갑 주소
copy-editor-address-help-clone = 같은 규칙에 빈 모의 장부입니다. 이 지갑으로 다른 규칙을 테스트하거나 다른 지갑을 입력하세요.
copy-editor-address-help-create = 이 작업이 매수(선택하면 매도도)를 카피할 지갑입니다.
copy-editor-name-label = 이름 <em>선택</em>
copy-editor-name-placeholder = 예: 빠른 로테이터
copy-editor-enabled-title = 지갑의 거래 처리
copy-editor-enabled-help = 끄면 재개하기 전까지 작업이 일시 중지된 상태로 유지됩니다.
copy-editor-note-live = 이 작업은 실거래 중입니다. 변경 사항은 다음 실제 카피부터 적용됩니다.
copy-editor-note-paper = 작업은 활성화하기 전까지 모의로 실행됩니다. 거래는 풀 가격으로 시뮬레이션되며 자금은 사용되지 않습니다.
copy-editor-copy-size = 카피 규모
copy-editor-sizing-fixed = 고정 금액
copy-editor-sizing-ratio = 지갑 거래의 비율
copy-editor-amount-fixed = 카피당 금액
copy-editor-amount-ratio = 거래당 비율
copy-editor-amount-help-fixed = 카피하는 매수마다 사용하는 금액이며 최소 { $minimum }입니다.
copy-editor-amount-help-ratio = 지갑 자신의 매수 금액 기준이며 거래당 한도까지입니다.
copy-editor-help-trade-cap = 카피 한 건에 이 금액을 넘게 사용하지 않습니다.
copy-editor-help-token-cap = 한 토큰에 사용하는 총 금액입니다.
copy-editor-help-budget = 이 작업이 존속하는 동안 사용할 수 있는 전체 금액입니다. 모의와 실거래는 각자의 지출을 따로 계산합니다.
copy-editor-preview-title = 카피 비용
copy-editor-preview-empty = 규모를 입력하면 카피 비용을 확인할 수 있습니다.
copy-editor-preview-example = 지갑이 { $target } 매수 → <strong>{ $copy }</strong> 카피
copy-editor-preview-once = 토큰은 한 번만 매수하므로 토큰 하나당 { $size } 카피 1회
copy-editor-preview-token-cap =
    { $count ->
       *[other] 토큰 하나당 최대 { $size } 카피 { $count }회
    }
copy-editor-preview-summary-exact = { $perToken }. 예산으로 약 { $count }회 카피할 수 있습니다. 네트워크 및 우선 수수료는 별도입니다.
copy-editor-preview-summary-minimum = { $perToken }. 예산으로 최소 { $count }회 카피할 수 있습니다. 네트워크 및 우선 수수료는 별도입니다.
copy-editor-target-min = 카피할 지갑 최소 거래
copy-editor-target-min-help = 지갑의 더 작은 매수는 무시합니다. 비워 두면 최소 기준이 없습니다.
copy-editor-target-max = 카피할 지갑 최대 거래
copy-editor-target-max-help = 지갑의 더 큰 매수는 무시합니다. 비워 두면 최대 기준이 없습니다.
copy-editor-buy-once-title = 토큰별 1회만 매수
copy-editor-buy-once-help = 지갑의 첫 매수만 카피하고 이후 매수는 건너뜁니다.
copy-editor-filter-require = 필수
copy-editor-filter-skip = 필수 아님
copy-editor-filter-help = 토큰을 카피하기 전에 내 필터링 파이프라인을 통과하도록 요구합니다.
copy-editor-filter-warning = 기본 필터링 설정에서는 거의 모든 토큰이 통과하지 못하므로 통과를 요구하면 아무것도 카피되지 않습니다. 이 지갑이 거래하는 토큰을 필터가 통과시킬 때만 요구하세요.
copy-editor-exit-both = 둘 다
copy-editor-exit-help-buy-only = 아래 내 규칙이 모든 보유 수량을 매도하며 지갑의 매도는 무시됩니다.
copy-editor-exit-help-hybrid = 먼저 발생하는 쪽이 적용됩니다: 지갑이 매도하거나 내 규칙 중 하나가 작동합니다.
copy-editor-exit-help-mirror = 지갑이 매도할 때만 보유 수량을 매도합니다. 내 청산 규칙은 실행되지 않습니다.
copy-editor-who-sells = 매도 주체
copy-editor-preset = 프리셋
copy-editor-preset-help = 프리셋은 아래 모든 규칙을 채우며, 이후 각 규칙을 조정할 수 있습니다.
copy-editor-mirror-note = 지갑의 매도가 결정하는 동안에는 이 규칙이 실행되지 않습니다. 다음으로 전환하면 적용됩니다: { $mine } 또는 { $both }.
copy-editor-rule-inherit = 트레이더 기본값
copy-editor-inherit-value = 트레이더 기본값 ({ $value })
copy-editor-rule-aria = { $rule } 설정
copy-editor-rule-empty-uses = 비워 두면 트레이더 기본값 사용: { $value }
copy-editor-rule-follows = 트레이더 설정을 따름: { $summary }
copy-editor-rule-follows-plain = 트레이더 설정을 따릅니다.
copy-editor-rule-off-note = 트레이더 설정과 관계없이 이 작업에서는 꺼져 있습니다.
copy-task-unnamed = 이름 없는 작업
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = 저장하면 거래 처리
copy-editor-review-paused = 일시 중지 상태로 저장
copy-editor-error-address = 올바른 Solana 지갑 주소를 입력하세요.
copy-editor-error-sizing = 모든 규모 값은 0보다 커야 합니다.
copy-editor-error-min-copy = 카피는 최소 { $minimum } 이상이어야 합니다. 카피당 금액을 늘리세요.
copy-editor-error-min-cap = 카피는 최소 { $minimum } 이상이어야 합니다. 거래당 한도를 늘리세요.
copy-editor-error-trade-cap = 거래당 한도는 토큰당 한도를 넘을 수 없습니다.
copy-editor-error-token-cap = 토큰당 한도는 총 예산을 넘을 수 없습니다.
copy-editor-error-slippage = 슬리피지는 { $min }에서 { $max } 사이여야 합니다.
copy-editor-error-target-limits = 지갑 거래 한도는 0 이상이어야 합니다.
copy-editor-error-target-order = 지갑 최소 거래는 최대 거래를 넘을 수 없습니다.

## Copy notices: toasts, the event log and Telegram (trader/copy/notify.rs).

copy-notice-task-unnamed = 작업 #{ $id }
copy-notice-heading = { $task }: { $title }
copy-notice-event = { $task }: { $title } — { $detail }
copy-notice-title-paper-buy = 모의 카피 매수
copy-notice-title-paper-sell = 모의 카피 매도
copy-notice-title-paper-closed = 모의 보유 수량 종료
copy-notice-title-paper-exit = 모의 청산: { $rule }
copy-notice-title-live-buy-submitted = 실거래 카피 매수 제출됨
copy-notice-title-live-buy-confirmed = 실거래 카피 매수 확정됨
copy-notice-title-live-buy-failed = 실거래 카피 매수 실패
copy-notice-title-live-sell-submitted = 실거래 카피 매도 제출됨
copy-notice-title-live-sell-failed = 실거래 카피 매도 실패
copy-notice-title-auto-paused = 카피 작업 자동 일시 중지
copy-notice-detail-bought = 매수 금액 { $amount } { -sol }
copy-notice-detail-sold = 매도 금액 { $amount } { -sol }
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = 보유 수량의 { $percent }%
copy-notice-detail-full-close = 전량 종료
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = 스왑에 실패했습니다
