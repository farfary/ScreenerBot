# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = 매수
transactions-type-sell = 매도
transactions-type-swap = 스왑
transactions-type-sol-transfer = SOL 전송
    .short = 전송
transactions-type-token-transfer = 토큰 전송
    .short = 전송
transactions-type-transfer = 전송
transactions-type-dust = 더스트
transactions-type-spam = 스팸
transactions-type-ata-create = 계정 개설
    .short = ATA 개설
transactions-type-ata-close = 렌트 회수
    .short = 렌트 반환
transactions-type-ata = 토큰 계정
transactions-type-liquidity-add = 유동성 추가
    .short = LP 추가
transactions-type-liquidity-remove = 유동성 제거
    .short = LP 제거
transactions-type-nft = NFT
transactions-type-program = 프로그램 호출
transactions-type-compute = 컴퓨트
transactions-type-failed = 실패
transactions-type-unknown = 미분류

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label } ({ $detail })
transactions-type-token-transfer-detail = { $label } { $mint } ({ $amount })
transactions-type-spam-detail = 스팸 에어드롭 ({ $mint })
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = 모든 유형
transactions-filter-transfer = 전송
transactions-filter-ata = 렌트 및 계정
transactions-filter-liquidity = 유동성
transactions-filter-program = 프로그램 호출

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-tokens-in = 토큰 입금
transactions-direction-tokens-out = 토큰 출금
transactions-direction-sol-in = { -sol } 입금
transactions-direction-sol-out = { -sol } 출금
transactions-direction-internal = 내부
transactions-direction-unknown = 미분류

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = 대기 중
transactions-status-confirmed = 확인됨
transactions-status-finalized = 최종 확정
transactions-status-failed = 실패
transactions-status-success = 성공
transactions-status-unknown = 알 수 없음

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = 생성
transactions-ata-operation-closure = 종료

## Transactions page (pages/transactions.js)

transactions-toolbar-title = 트랜잭션 내역
transactions-search =
    .placeholder = 서명 검색…
    .aria-label = 트랜잭션 서명 검색
transactions-load-failed = 트랜잭션을 새로 고치지 못했습니다
transactions-setup-gate-title = 거래 내역을 보려면 지갑이 필요합니다
transactions-summary-total = 전체
transactions-summary-success = 성공
transactions-summary-failed = 실패
transactions-filter-wallet = 지갑
transactions-filter-type = 유형
transactions-filter-direction = 방향
transactions-filter-status = 상태
transactions-filter-all-directions = 모든 방향
transactions-filter-all-statuses = 모든 상태
transactions-wallet-main = 메인 지갑
transactions-col-time = 시각
transactions-col-signature = 서명
transactions-col-type = 유형
transactions-col-direction = 방향
transactions-col-status = 상태
transactions-col-native-delta = Δ ({ -sol })
transactions-col-fees = 수수료 ({ -sol })
transactions-col-token = 토큰
transactions-col-router = 라우터
transactions-col-instructions = 인스트럭션

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = 서명 복사
transactions-dialog-close =
    .title = 닫기 (ESC)
transactions-dialog-tabs-label = 트랜잭션 상세 섹션
transactions-dialog-meta-slot = 슬롯:
transactions-dialog-meta-fee = 수수료:
transactions-dialog-loading = 불러오는 중...
transactions-dialog-loading-details = 트랜잭션 상세 정보 불러오는 중...
transactions-dialog-load-failed = 트랜잭션 상세 정보를 불러오지 못했습니다
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = 트랜잭션 상세 정보를 불러오지 못했습니다: { $reason }
transactions-dialog-not-found = 트랜잭션을 찾을 수 없습니다
transactions-dialog-tab-overview = 개요
transactions-dialog-tab-balances = 잔액
transactions-dialog-tab-instructions = 인스트럭션
transactions-dialog-tab-logs = 로그
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = 원본
transactions-dialog-unknown = 알 수 없음
transactions-dialog-unknown-asset = 알 수 없는 자산
transactions-dialog-unavailable = 사용 불가

## Transaction details dialog: overview

transactions-dialog-failed-title = 트랜잭션 실패
transactions-dialog-no-program-error = 제공된 프로그램 오류가 없습니다.
transactions-dialog-story-title = 발생한 일
# $router is the routing program name.
transactions-dialog-router-via = { $router } 경유
transactions-dialog-flow-paid = 지불
transactions-dialog-flow-received = 수령
transactions-dialog-flow-from = 보낸 주소
transactions-dialog-flow-to = 받는 주소
transactions-dialog-flow-amount = 수량
transactions-dialog-net-wallet-change = 지갑 순변동:
transactions-dialog-processed = Solana에서 처리됨
transactions-dialog-execution-title = 체결
transactions-dialog-metric-execution-price = 체결가
transactions-dialog-metric-effective-received = 실수령액
transactions-dialog-metric-effective-spent = 실지출액
transactions-dialog-metric-network-fee = 네트워크 수수료
transactions-dialog-metric-estimated-pnl = 추정 손익
transactions-dialog-metric-net-native-change = { -sol } 순변동
transactions-dialog-route-title = 경로 및 자산
transactions-dialog-route-router = 라우터
transactions-dialog-route-input-asset = 입력 자산
transactions-dialog-route-output-asset = 출력 자산
transactions-dialog-route-pool = 풀
transactions-dialog-route-program = 프로그램
transactions-dialog-tech-title = 기술 상세
transactions-dialog-tech-summary = 서명, 슬롯 및 리소스
transactions-dialog-tech-signature = 서명
transactions-dialog-tech-timestamp = 타임스탬프
transactions-dialog-tech-slot = 슬롯
transactions-dialog-tech-exact-fee = 정확한 수수료
transactions-dialog-tech-accounts = 계정
transactions-dialog-tech-instructions = 인스트럭션
transactions-dialog-tech-compute-units = 컴퓨트 유닛
transactions-dialog-tech-token-decimals = 토큰 소수 자릿수

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = { -sol } 잔액 변동
transactions-dialog-balances-native-empty = { -sol } 잔액 변동 없음
transactions-dialog-balances-token-title = 토큰 잔액 변동
transactions-dialog-balances-token-empty = 토큰 잔액 변동 없음
transactions-dialog-balances-net-native = { -sol } 순변동
transactions-dialog-balances-fee = 트랜잭션 수수료
transactions-dialog-col-account = 계정
transactions-dialog-col-token = 토큰
transactions-dialog-col-pre-balance = 이전 잔액
transactions-dialog-col-post-balance = 이후 잔액
transactions-dialog-col-change = 변동
transactions-dialog-col-type = 유형
transactions-dialog-col-rent = 렌트 ({ -sol })
transactions-dialog-instructions-empty = 인스트럭션이 없습니다
transactions-dialog-instructions-count =
    { $count ->
       *[other] 인스트럭션 { $count }개
    }
transactions-dialog-instruction-program-id = 프로그램 ID
transactions-dialog-instruction-accounts = 계정 ({ $count })
transactions-dialog-instruction-data = 데이터
transactions-dialog-logs-empty = 사용 가능한 로그가 없습니다
transactions-dialog-logs-filter = 로그 필터...
transactions-dialog-logs-no-match = 일치하는 로그 없음
transactions-dialog-logs-count =
    { $count ->
       *[other] 로그 { $count }건
    }
transactions-dialog-ata-empty = 이 트랜잭션에는 ATA 작업이 없습니다
transactions-dialog-ata-summary-title = ATA 분석 요약
transactions-dialog-ata-creations = 생성
transactions-dialog-ata-closures = 종료
transactions-dialog-ata-rent-spent = 지출된 렌트
transactions-dialog-ata-rent-recovered = 회수된 렌트
transactions-dialog-ata-net-rent = 렌트 순영향
transactions-dialog-ata-operations-title = ATA 작업 ({ $count })
transactions-dialog-raw-copy = JSON 복사
transactions-dialog-raw-empty = 사용 가능한 원본 데이터가 없습니다

# Empty table (scripts/pages/transactions.js)
transactions-empty = 아직 거래 내역이 없습니다
    .message = 트레이딩 지갑의 스왑과 전송은 온체인에서 확인되면 여기에 표시됩니다.
