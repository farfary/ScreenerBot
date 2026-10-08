# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = 포지션 생성됨


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = 보유 중
positions-status-closed = 종료됨
positions-status-archived = 보관됨
positions-origin-copy = 카피
positions-origin-manual = 수동
positions-origin-wallet = 지갑
positions-origin-copy-link =
    .title = 이 포지션을 연 카피 작업 열기
positions-holding-frozen = 동결됨
    .title = 민트 권한이 이 토큰 계정을 동결했습니다. 잔액을 전송하거나 매도할 수 없습니다
positions-toolbar-total = 전체
positions-toolbar-delete-all = 모두 삭제
positions-search-placeholder = 심볼 또는 민트로 검색...
positions-filter-origin = 출처
positions-filter-origin-all = 모든 출처
positions-filter-origin-auto = 자동 트레이더
positions-filter-origin-copy = 카피 트레이딩
positions-delete-all-tooltip = 보관된 포지션을 모두 영구 삭제합니다

## Columns

positions-column-token = 토큰
positions-column-archived-at = 보관 시각
positions-column-entry-time = 진입 시각
positions-column-exit-time = 청산 시각
positions-column-avg-entry = 평균 진입가 ({ -sol })
positions-column-avg-exit = 평균 청산가 ({ -sol })
positions-column-current-price = 현재가 ({ -sol })
positions-column-total-invested = 총 투자금
positions-column-proceeds = 회수금
positions-column-pnl = 손익
positions-column-pnl-percent = 손익 %
positions-column-size = 규모
positions-column-dca = DCA
positions-column-exits = 청산
positions-column-unrealized-pnl = 미실현 손익
positions-column-unrealized-percent = 미실현 %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = 이 지갑의 내역에 매입 원가가 없습니다 (에어드롭, USD 기준 체결 또는 SOL 구간이 없는 스왑)
positions-unknown-history = 이 라운드는 온체인 잔액과 일치하지 않습니다
positions-dca-count =
    { $count ->
       *[other] DCA { $count }회
    }
positions-exit-count =
    { $count ->
       *[other] 청산 { $count }회
    }

## Row actions

positions-action-add =
    .title = 포지션에 추가 매수 (DCA)
    .aria-label = 포지션에 추가 매수
positions-action-sell =
    .title = 매도 (전량 또는 % 부분 매도)
    .aria-label = 포지션 매도
positions-action-sell-frozen = 민트 권한이 동결하여 이 보유 수량은 매도할 수 없습니다
positions-action-remove =
    .title = 제거 (보관 또는 삭제)
    .aria-label = 포지션 제거
positions-action-restore =
    .title = 보유 중/종료됨으로 복원
    .aria-label = 포지션 복원
positions-action-delete =
    .title = 영구 삭제
    .aria-label = 영구 삭제
positions-action-in-progress = 진행 중…

## Live state of a row

positions-caption-buying = 매수 중
# $step is the label of the current action step.
positions-caption-buying-step = 매수 중 · { $step }
positions-caption-selling = 매도 중
positions-caption-selling-step = 매도 중 · { $step }
positions-caption-closing = 종료 중
positions-caption-failed = 실패
# $error is the failure text of the action.
positions-caption-failed-detail = 실패 · { $error }
positions-step-adding = 추가 매수 중
positions-pending-buying = 매수 중…
positions-pending-buy-failed = 매수 실패

## Messages and confirmations

positions-load-failed = 포지션을 새로 고치지 못했습니다
positions-toast-not-found = 포지션 데이터를 찾을 수 없습니다
positions-toast-deleted = 포지션이 삭제되었습니다
positions-toast-archived = 포지션이 보관되었습니다
positions-toast-restored = 포지션이 복원되었습니다
positions-buy-adds-to-archived = 이 토큰에는 이미 보관함에 보유 포지션이 있습니다. 매수는 해당 포지션에 추가되며, 포지션은 보유 포지션 목록으로 돌아갑니다. 포지션의 현재 관리 모드는 그대로 유지됩니다.
positions-action-failed = 작업에 실패했습니다
positions-delete-title = 포지션 영구 삭제
# $symbol is the token symbol.
positions-delete-message = 토큰 { $symbol }의 포지션을 영구 삭제하시겠습니까? 데이터베이스에서 포지션과 내역이 제거되며 되돌릴 수 없습니다. 트랜잭션과 토큰 데이터에는 영향이 없습니다.
positions-delete-confirm = 영구 삭제
positions-delete-all-title = 보관된 포지션 모두 삭제
positions-delete-all-message =
    { $count ->
       *[other] 보관된 포지션 { $count }개를 모두 영구 삭제하시겠습니까? 되돌릴 수 없습니다. 트랜잭션과 토큰 데이터에는 영향이 없습니다.
    }
positions-delete-all-message-empty = 보관된 포지션을 모두 영구 삭제하시겠습니까? 되돌릴 수 없습니다.
positions-delete-all-confirm = 모두 삭제
positions-delete-all-done =
    { $count ->
       *[other] 보관된 포지션 { $count }개를 삭제했습니다
    }
positions-delete-all-failed = 보관된 포지션을 삭제하지 못했습니다

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = 포지션 제거
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>이 포지션은 아직 보유 중입니다.</strong> 봇이 이 토큰을 보유하고 있습니다. 제거하면 거래 슬롯이 비워지고 추적이 중단되지만 매도되지는 <strong>않습니다</strong>. { -sol }을 돌려받으려면 먼저 매도하세요.
positions-remove-modes =
    .aria-label = 제거 방식
positions-remove-archive = 보관
positions-remove-recommended = 권장
positions-remove-archive-description = 보관됨 탭으로 숨깁니다. 언제든 되돌릴 수 있으며 매도되지 않고 모든 거래 기록이 유지됩니다.
positions-remove-delete = 영구 삭제
positions-remove-delete-description = 데이터베이스에서 이 포지션과 전체 내역을 삭제합니다.
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = 포지션과 내역이 영구적으로 제거됩니다. <strong>되돌릴 수 없습니다.</strong> 트랜잭션과 토큰 데이터에는 영향이 없습니다.
positions-remove-confirm-archive = 포지션 보관

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = 포지션 관리 방식: { $mode }
positions-details-load-failed = 포지션 상세 정보를 불러오지 못했습니다
positions-details-mint-label = 민트 주소
positions-details-management-failed = 포지션 관리 방식을 변경하지 못했습니다
positions-details-favorite-add =
    .title = 즐겨찾기에 추가
    .aria-label = 즐겨찾기에 추가
positions-details-favorite-remove =
    .title = 즐겨찾기에서 제거
    .aria-label = 즐겨찾기에서 제거
positions-details-view-solscan =
    .title = { -solscan }에서 보기
    .aria-label = { -solscan }에서 토큰 보기
positions-details-close =
    .title = 닫기 (Esc)
    .aria-label = 닫기
positions-details-chart-section =
    .aria-label = 가격 차트
positions-details-loading-chart = 차트 불러오는 중...
positions-details-activity-section =
    .aria-label = 활동
positions-details-activity-title = 활동
positions-details-split-handle =
    .aria-label = 차트와 활동 크기 조절
positions-details-activity-pane =
    .aria-label = 활동 패널
positions-details-activity-expand =
    .title = 활동 확대
    .aria-label = 활동 확대
positions-details-summary-section =
    .aria-label = 포지션 요약
positions-details-loading = 포지션 불러오는 중...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = 자동 트레이더
positions-management-user-only = 사용자 전용
positions-management-copy-task = 카피 작업
positions-management-hybrid = 하이브리드
positions-pane-show-chart = 차트 표시
positions-pane-show-activity = 활동 표시
positions-pane-restore-activity = 활동 복원
positions-pane-expand-chart =
    .title = 차트 확대
    .aria-label = 차트 확대

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = 낮은 위험
positions-risk-medium = 중간 위험
positions-risk-high = 높은 위험
positions-risk-unknown = 위험 알 수 없음
positions-busy-buying = 매수 진행 중…
positions-busy-selling = 매도 진행 중…
positions-busy-closing = 종료 진행 중…
positions-header-avg-entry = 평균 진입가
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
       *[other] 매수 { $count }회
    }
positions-header-exit-price = 청산가
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = 종료: { $ago }
positions-header-realized-pnl = 실현 손익
positions-header-usd-note = 오늘의 { -sol } 가격 기준 USD
positions-header-returned = 회수
# $amount is the formatted SOL amount invested.
positions-header-of-invested = 투자금 { $amount } 대비
positions-header-price = 가격
positions-header-last-price = 최종 가격
positions-header-pool-ago = 풀 · { $ago }
positions-header-api-ago = API · { $ago }
positions-header-unrealized-pnl = 미실현 손익
positions-header-pnl-last-price = 최종 가격 기준 손익
positions-header-value = 가치
positions-header-last-value = 최종 가치
positions-header-invested = 투자금 { $amount }
positions-header-origin-hint = 이 포지션을 연 방식
positions-header-risk-hint = { -rugcheck } 점수 - 낮을수록 안전
positions-header-frozen = 동결됨
    .title = 민트 권한이 이 보유 수량을 동결했습니다
positions-header-managed-by = 관리 주체
positions-header-management-select =
    .aria-label = 포지션 관리

## Entry origin shown in the header badge

positions-origin-unknown = 알 수 없음
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = 카피 · 작업 { $task }
positions-origin-manual-entry = 수동 진입
positions-origin-wallet-entry = 지갑 진입
# $strategy is the strategy id.
positions-origin-auto-strategy = 자동 · { $strategy }
positions-origin-auto-entry = 자동 진입

## Swaps that are submitted and not yet booked

positions-pending-adding = 추가 매수 중
positions-pending-adding-amount = { $amount } 추가 매수 중
positions-pending-selling = 매도 중
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = { $percent } 매도 중
# $label is the pending swap wording.
positions-pending-confirming = { $label } · 확인 중
    .title = 제출되었으며 온체인 확인을 기다리는 중입니다. 검증되면 수치가 갱신됩니다.

## Trade controls

positions-trade-add = 추가 매수
    .title = 포지션에 추가 매수
positions-trade-sell = 매도
    .title = 포지션 일부 매도
positions-trade-close = 포지션 종료
    .title = 전량 매도 후 종료
positions-trade-token = 토큰 상세
    .title = 토큰 상세 열기

## Favorites

positions-favorite-token-fallback = 토큰
# $symbol is the token symbol.
positions-favorite-added = 즐겨찾기에 추가됨: { $symbol }
positions-favorite-removed = 즐겨찾기에서 제거됨: { $symbol }
positions-favorite-add-failed = 즐겨찾기를 추가하지 못했습니다
positions-favorite-remove-failed = 즐겨찾기를 제거하지 못했습니다
positions-favorite-update-failed = 즐겨찾기를 업데이트하지 못했습니다

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = 포지션
positions-summary-price-path = 가격 경로
positions-summary-network-fees = 네트워크 수수료
positions-summary-risk = 위험
positions-summary-market = 시장
positions-summary-market-now = 현재 시장
positions-summary-links = 링크
positions-fact-tokens-fallback = 토큰
positions-fact-bought = 매수
positions-fact-holding = 보유 수량
positions-fact-sold = 매도
positions-fact-realized = 실현
positions-fact-opened = 진입
positions-fact-closed = 종료
positions-fact-reason = 사유
positions-fact-archived = 보관
positions-fact-entry = 진입
positions-fact-exit = 청산
positions-fact-total = 합계
positions-fact-verified = 온체인 검증됨
positions-fact-confirming = 확인 중
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = 매수량의 { $percent }
positions-fact-share-of-invested = 투자금의 { $percent }
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 진입 1회
       *[other] 진입 1회 + 추가 매수 { $count }회
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
       *[other] 부분 청산 { $count }회 · { $returned } 회수
    }
# $age is the elapsed time of the hold.
positions-fact-held = 보유 { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = 진입가 대비 { $percent }
positions-fact-exit-vs-peak = 청산가 대 고점
positions-fact-now-vs-peak = 현재가 대 고점
positions-fact-entry-range = 진입 범위
positions-range-low = 저점
positions-range-peak = 고점
positions-range-now = 현재
positions-range-label-exit = 저점과 고점 사이의 진입가 및 청산가
positions-range-label-now = 저점과 고점 사이의 진입가 및 현재가
positions-fact-mint-authority = 민트 권한
positions-fact-freeze-authority = 동결 권한
positions-fact-active = 활성
positions-fact-pool = 풀
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = 유동성 { $amount } { -sol }
positions-fact-market-cap = 시가총액
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = 유동성
positions-fact-volume-24h = 24h 거래량
positions-fact-price-change = 가격 변동
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = 홀더
positions-link-website = 웹사이트
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = 활동을 불러오지 못했습니다
positions-activity-loading = 활동 불러오는 중...
positions-activity-empty = 이 지갑에서 이 토큰에 대한 활동이 아직 없습니다
positions-activity-filter-empty = 이 필터에 해당하는 활동이 없습니다
positions-activity-round-count =
    { $count ->
       *[other] 라운드 { $count }개
    }
positions-activity-event-count =
    { $count ->
       *[other] 이벤트 { $count }건
    }
positions-activity-pending-count = 대기 { $count }건
positions-activity-failed-count = 실패 { $count }건
positions-filter-all = 전체
positions-filter-trades = 거래
positions-filter-buys = 매수
positions-filter-sells = 매도
positions-filter-wallet = 지갑
positions-filter-issues = 문제
positions-activity-filters =
    .aria-label = 활동 필터
positions-activity-totals =
    .aria-label = 이 토큰의 모든 라운드
positions-activity-realized-all = 실현 손익, 전체 라운드
positions-activity-invested = 투자금
positions-activity-returned = 회수금
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = 진입 { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = 포지션 { $index }
positions-activity-this-position = 이 포지션
positions-activity-dates-unavailable = 날짜 없음
positions-activity-wallet-title = 지갑 트랜잭션
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
       *[other] 포지션 외 · { $range } · 이벤트 { $count }건
    }
positions-details-signature-label = 서명

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = 포지션 보유 중
positions-state-closing = 포지션 종료 중
positions-state-closed = 포지션 종료됨
positions-state-exit-pending = 포지션 청산 대기
positions-state-exit-failed = 포지션 청산 실패
positions-state-phantom = 포지션 팬텀
positions-state-reconciling = 포지션 대사 중

## Activity events

positions-event-kind-entry = 진입
positions-event-kind-dca = 추가 매수
positions-event-kind-partial-exit = 부분 청산
positions-event-kind-exit = 청산
positions-event-kind-buy = 지갑 매수
positions-event-kind-sell = 지갑 매도
positions-event-kind-transfer = 전송
positions-event-kind-ata = 토큰 계정
positions-event-kind-other = 트랜잭션
positions-event-state-pending = 대기 중
positions-event-state-failed = 실패
positions-event-state-synthetic = 합성
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = 실패: { $error }
positions-event-tokens-fallback = 토큰
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = { $amount } 매수 제출됨
positions-event-entry-for = { $amount } 매수 ({ $sol })
positions-event-entry = { $amount } 매수
positions-event-dca-submitted = { $amount } 추가 매수 제출됨
positions-event-dca-for = { $amount } 추가 매수 ({ $sol })
positions-event-dca = { $amount } 추가 매수
positions-event-partial-exit-submitted-percent = { $amount } 부분 청산 ({ $percent }) 제출됨
positions-event-partial-exit-submitted = { $amount } 부분 청산 제출됨
positions-event-sold-percent-for = { $amount } ({ $percent }) 매도 ({ $sol })
positions-event-sold-percent = { $amount } ({ $percent }) 매도
positions-event-sold-for = { $amount } 매도 ({ $sol })
positions-event-sold = { $amount } 매도
positions-event-exit-submitted = 포지션 전량 청산 제출됨
positions-event-exit-for = { $amount } 매도로 종료 ({ $sol })
positions-event-exit-closed = 포지션 종료됨
positions-event-wallet-bought = 지갑이 다른 경로로 { $amount } 매수
positions-event-wallet-sold = 지갑이 다른 경로로 { $amount } 매도
positions-event-received = { $amount } 수신
positions-event-sent = { $amount } 전송
positions-event-transferred = { $amount } 이동
positions-event-ata = 토큰 계정 활동
positions-event-wallet-transaction = { $amount } 관련 지갑 트랜잭션
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / 토큰
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = 지갑 변동 { $amount }
positions-event-after-title = 이 이벤트 이후 포지션
positions-event-capital-invested = 투입 자본
positions-event-average-entry = 평균 진입가
positions-event-transfers-title = 토큰 전송
positions-event-transfer-amount = 수량
positions-event-transfer-mint = 민트
positions-event-transfer-from = 보낸 주소
positions-event-transfer-to = 받는 주소
positions-event-no-signature = 온체인 서명 없음
positions-event-click-to-copy = 클릭하여 복사
positions-event-solscan = { -solscan }
positions-event-token-amount = 토큰 수량
positions-event-trade-price = 거래 가격
positions-event-native-amount = { -sol } 수량
positions-event-cost-basis = 매입 원가
positions-event-usd-value = USD 가치
positions-event-network-fee = 네트워크 수수료
positions-event-router = 라우터
positions-event-slot = 슬롯
positions-event-chain-status = 체인 상태
positions-event-transaction-type = 트랜잭션 유형
positions-event-direction = 방향
positions-event-wallet-native-change = 지갑 { -sol } 변동
positions-event-instructions = 인스트럭션
positions-event-compute-units = 컴퓨트 유닛
positions-event-accounts = 계정
positions-event-record-id = 레코드 ID
positions-event-time-unavailable = 시각 없음
positions-event-details = 상세
positions-event-hide-details = 상세 숨기기

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = 캔들
positions-chart-type-line = 라인
positions-chart-type-area = 영역
positions-chart-type-group =
    .aria-label = 차트 유형
positions-chart-overlays-group =
    .aria-label = 차트 오버레이
positions-chart-ema = EMA
    .title = 지수이동평균 9, 21
positions-chart-fit = 맞춤
    .title = 이 포지션의 전체 기간에 맞춥니다
positions-chart-timeframes-group =
    .aria-label = 타임프레임
positions-chart-pane-group =
    .aria-label = 차트 패널
positions-chart-unavailable = 차트 엔진을 사용할 수 없습니다
positions-chart-collecting = 차트 데이터 수집 중…
positions-chart-no-data = 이 토큰의 차트 데이터가 아직 없습니다
positions-chart-avg-entry = 평균 진입가
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = 평균 진입가
positions-chart-legend-avg-entry-off-scale = 평균 진입가 (범위 밖)
positions-chart-dropped-events =
    { $count ->
       *[other] 이 타임프레임에 캔들이 없는 이벤트 { $count }건
    }
positions-chart-level = 레벨
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price }이 현재 보기 위쪽에 있습니다
positions-chart-level-below = { $label } { $price }이 현재 보기 아래쪽에 있습니다
positions-chart-scale-hint = 가격 축을 드래그하여 해당 가격까지 축소합니다
positions-chart-pnl-at-bar = 손익 @ 봉
positions-chart-click-to-locate = 클릭하여 위치 찾기
