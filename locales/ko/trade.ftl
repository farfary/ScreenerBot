# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = 견적을 가져오지 못했습니다. 연결을 확인한 후 다시 시도하세요
# Fallback title when the quote request failed without a message.
trade-quote-error-title = 견적을 가져오지 못했습니다
trade-quote-title = 스왑 미리보기
trade-quote-refresh =
    .aria-label = 견적 새로 고침
    .title = 견적 새로 고침
trade-quote-idle = 수량을 선택하면 스왑을 미리 볼 수 있습니다
trade-quote-loading = 최적 경로를 찾는 중…
trade-quote-retry = 다시 시도
trade-quote-pay = 지불
trade-quote-receive = 수령 (예상)
trade-quote-minimum = 보장 최소 수령량
    .title = 최대 슬리피지 적용 후 받을 수 있는 최소 수량입니다. 이보다 적게 체결되는 경우 스왑은 실행되지 않고 되돌려집니다.
trade-quote-impact = 가격 영향
trade-quote-slippage = 최대 슬리피지
trade-quote-platform-fee = 플랫폼 수수료
    .title = 0.5% - 개발을 지원합니다. 위 견적에 이미 반영되어 있습니다.
trade-quote-network-fee = 네트워크 수수료
trade-quote-route = 경로
trade-quote-disclaimer = 가격은 체인에서 실시간으로 업데이트됩니다. 보장 최소 수령량 이상으로 체결할 수 없으면 스왑이 되돌려지므로 표시된 수량보다 적게 받는 일은 없습니다.
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = 가격 영향({ $impact })이 최대 슬리피지({ $tolerance }%)를 초과합니다. 이 규모는 풀 가격을 움직입니다. 수량을 줄이면 시장가에 더 가깝게 체결됩니다.

## Units

trade-unit-sol = { -sol }
trade-unit-tokens = 토큰

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = 토큰 매수
trade-buy-subtitle = { -sol } 수량을 입력하세요
trade-buy-confirm = 매수 실행
trade-buy-hint = 비워 두면 설정 기본값을 사용합니다
trade-sell-title = 포지션 매도
trade-sell-subtitle = 매도 비율을 선택하세요
trade-sell-confirm = 매도 실행
trade-sell-hint = 1-100 사이의 값을 입력하세요
trade-sell-input = 직접 입력 비율
    .placeholder = 1-100
trade-add-title = 포지션에 추가 매수
trade-add-subtitle = 기존 포지션에 DCA
trade-add-confirm = 추가 매수
trade-add-hint = 비워 두면 설정된 DCA 규모를 사용합니다
trade-amount-input = 직접 입력 수량
    .placeholder = { -sol } 수량 입력

## Presets

trade-presets-quick-amount = 빠른 수량
trade-presets-quick-sell = 빠른 매도
trade-presets-match-entry = 진입 수량과 동일
trade-presets-fixed-amount = 고정 수량
trade-preset-partial = 일부
trade-preset-half = 절반
trade-preset-most = 대부분
trade-preset-full = 전량 청산
# $label is the preset's amount.
trade-preset-select =
    .aria-label = 선택: { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = 대화상자 닫기
trade-input-max = 최대
    .aria-label = 최대값 사용
trade-slider =
    .aria-label = 수량 슬라이더
trade-context-available = 사용 가능
trade-context-position-size = 포지션 규모
trade-context-holdings = 보유 수량
trade-held-badge = 보유 중
    .title = 이 토큰의 보유 포지션이 있습니다
trade-manage-title = 수동 관리
trade-manage-description = 자동 트레이더는 이 포지션을 매도하거나 DCA하지 않습니다. 체크를 해제하면 청산을 자동으로 관리합니다.

## Slippage

trade-slippage-label = 슬리피지
trade-slippage-presets =
    .aria-label = 슬리피지 프리셋
trade-slippage-auto = 자동
trade-slippage-custom =
    .placeholder = 직접 입력
    .aria-label = 슬리피지 비율 직접 입력
trade-slippage-note-auto = 자동 (설정 기준)
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = 자동 (설정 기준 { $pct }%)
# $pct is the override as typed.
trade-slippage-note-override = 직접 지정: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = 높은 슬리피지: 견적보다 최대 { $pct }% 적게 받을 수 있습니다.
trade-impact-warning-title = 높은 가격 영향 경고
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = 이 거래의 가격 영향은 <strong>{ $impact }</strong>로, 슬리피지 허용치 <strong>{ $tolerance }%</strong>를 초과합니다. 예상보다 훨씬 적게 받을 수 있습니다.
trade-impact-warning-proceed = 그래도 진행

## Validation and verification

trade-error-invalid-number = 올바르지 않은 숫자입니다
trade-error-percentage-range = 비율은 1에서 100 사이여야 합니다
trade-error-amount-positive = 수량은 0보다 커야 합니다
trade-error-amount-minimum = 최소: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = 잔액이 부족합니다 (필요 { $needed } + 수수료 여유분 { $reserve }, 보유 { $balance })
trade-error-position-closed = 이 포지션은 더 이상 보유 중이 아닙니다.
trade-error-verify-failed = 토큰 잔액을 확인하지 못했습니다
trade-error-position-missing = 포지션을 찾을 수 없습니다. 이미 종료되었을 수 있습니다
# $expected and $current are formatted token amounts.
trade-error-balance-changed = 토큰 잔액이 변경되었습니다. 예상 { $expected }, 현재 { $current }. 새로 고치세요.
trade-error-verify-network = 잔액 확인 중 네트워크 오류가 발생했습니다

## Quick trade

trade-quick-buy-title = 빠른 매수
trade-quick-sell-title = 빠른 매도
trade-quick-subtitle = 토큰 민트 주소를 입력하세요
trade-quick-mint-label = 토큰 민트 주소 입력
trade-quick-mint-input =
    .placeholder = 민트 주소를 입력하거나 심볼로 검색...
trade-quick-paste =
    .aria-label = 클립보드에서 붙여넣기
trade-quick-recent = 최근:
trade-quick-fetching = 토큰 정보를 가져오는 중...
trade-quick-continue = 계속
trade-quick-token-not-found = 토큰을 찾을 수 없습니다
trade-quick-token-failed = 토큰을 가져오지 못했습니다
trade-quick-token-not-in-database = 데이터베이스에서 토큰을 찾을 수 없습니다
trade-quick-token-info-failed = 토큰 정보를 가져오지 못했습니다
trade-quick-no-position = 이 토큰의 포지션이 없습니다
trade-quick-no-holdings = 포지션에 남은 토큰이 없습니다
trade-quick-position-failed = 포지션 데이터를 가져오지 못했습니다

## Manual trade toasts

trade-toast-no-mint = 사용할 수 있는 민트 주소가 없습니다
trade-toast-open-failed = 거래 대화상자를 열지 못했습니다
trade-toast-pending-buy = 매수 진행 중
trade-toast-pending-add = 추가 매수 진행 중
trade-toast-pending-sell = 매도 진행 중
trade-toast-pending-message = 브라우저가 응답 대기를 중단했습니다. 결과는 포지션 행에서 확인하세요
trade-toast-failed-buy = 매수 실패
trade-toast-failed-add = 포지션 추가 매수 실패
trade-toast-failed-sell = 매도 실패

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = 전략 시그널
trade-reason-manual-entry = 수동 진입
trade-reason-force-buy = 강제 매수
trade-reason-copy-buy = 카피 매수
trade-reason-dca-scheduled = 예약된 DCA
trade-reason-take-profit = 익절
trade-reason-stop-loss = 손절
trade-reason-trailing-stop = 트레일링 스톱
trade-reason-time-override = 시간 우선 적용
trade-reason-strategy-exit = 전략 청산
trade-reason-llm-analysis-exit = LLM 분석 청산
trade-reason-manual-exit = 수동 청산
trade-reason-risk-management = 위험 관리
trade-reason-blacklisted = 블랙리스트
trade-reason-force-sell = 강제 매도
trade-reason-copy-sell = 카피 매도
trade-reason-closed-externally = 외부에서 종료됨
trade-reason-wallet-history = 지갑 내역
trade-reason-exit-retry-pending = 청산 재시도 대기
trade-reason-synthetic-exit-permanent-failure = 합성 청산 영구 실패
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason } (검증 대기)
# $note is the operator text of a force close.
trade-reason-force-closed = 강제 종료: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = 선택된 토큰이 없습니다
