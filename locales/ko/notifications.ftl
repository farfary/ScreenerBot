notifications-action-swap-buy = 매수
notifications-action-swap-sell = 매도
notifications-action-position-open = 진입
notifications-action-position-close = 종료
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = 부분 청산
notifications-action-manual-order = 수동
notifications-action-unknown = 작업

actions-step-evaluate = 평가 중
actions-step-validate = 검증 중
actions-step-quote = 견적 조회 중
actions-step-swap = 스왑 실행 중
actions-step-verify = 확인 중
actions-step-unknown = 처리 중
actions-step-evaluate-short = 평가 중
actions-step-validate-short = 점검 중
actions-step-quote-short = 견적
actions-step-swap-short = 스왑 중
actions-step-verify-short = 확정 중
actions-step-unknown-short = 작업 중

actions-failure-recorded = { $message }
actions-failure-unknown = 알 수 없는 오류
actions-failure-interrupted = 애플리케이션 재시작으로 중단되었습니다
actions-failure-validation = 검증에 실패했습니다
actions-failure-quote = 견적 조회에 실패했습니다
actions-failure-swap = 스왑에 실패했습니다
actions-failure-trade = 거래에 실패했습니다
actions-failure-entry = 진입에 실패했습니다
actions-failure-exit = 청산에 실패했습니다
actions-failure-dca = DCA에 실패했습니다
actions-failure-verification-expired = 확인 시간이 만료되었습니다. 트랜잭션이 반영되지 않았습니다
actions-failure-verification-gave-up = 확인을 중단했습니다
actions-failure-transaction-failed = 트랜잭션이 온체인에서 실패했습니다
actions-failure-sell-transaction-failed = 매도 트랜잭션이 온체인에서 실패했습니다
actions-failure-dca-verification-failed = DCA 확인에 실패했습니다

notifications-empty-all = 작업 없음
notifications-empty-active = 진행 중인 작업 없음
notifications-empty-completed = 완료된 작업 없음
notifications-empty-failed = 실패한 작업 없음
notifications-source-auto = 자동
notifications-source-manual = 수동
notifications-cancelled = 취소됨
notifications-load-failed = 불러오지 못했습니다
notifications-clear-title = 알림 지우기
notifications-clear-message = 이 목록의 모든 알림을 닫으시겠습니까? 완료/실패 기록에는 그대로 남습니다.
notifications-clear-failed = 알림을 지우지 못했습니다
notifications-stream-lag-title = 작업 스트림이 지연되었습니다
notifications-stream-lag-missed =
    { $count ->
       *[other] 누락된 업데이트 { $count }건 — 새로고침 중
    }
notifications-stream-lag-refreshing = 새로고침 중
notifications-sync-failed = 작업을 새로고침하지 못했습니다

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = 오늘
notifications-day-yesterday = 어제
notifications-source-copy = 카피 트레이딩
notifications-steps = 단계
notifications-detail-transaction = 트랜잭션
notifications-detail-fill-price = 체결 가격
    .title = 이 스왑의 { -sol } 금액을 토큰 수량으로 나눈 값
notifications-detail-fee = 네트워크 수수료
notifications-detail-pnl = 포지션 손익
notifications-detail-route = 경로
notifications-detail-duration = 소요 시간
notifications-detail-reason = 사유
notifications-view-token = 토큰 보기
notifications-view-position = 포지션 보기
notifications-view-transaction = 트랜잭션 보기
notifications-unrecorded = 포지션에 기록되지 않음
notifications-unrecorded-detail = 이 스왑은 포지션에 기록되지 않았으며 체결되지 않았을 수 있습니다. 트랜잭션을 확인하세요.
