# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = 도구
tools-category-wallet = 지갑
tools-category-token = 토큰
tools-category-single-token = 단일 토큰
tools-category-utilities = 유틸리티
tools-sidebar-hint = 도구를 선택하여 시작하세요
tools-help-button =
    .aria-label = 이 도구의 도움말 보기
tools-help-unavailable = 도움말을 사용할 수 없습니다
tools-placeholder-title = 도구 선택
tools-placeholder-subtitle = 사이드바에서 도구를 선택하여 시작하세요
tools-placeholder-hint-wallets = 지갑 도구는 Solana 지갑 관리를 도와줍니다
tools-placeholder-hint-secure = 모든 작업은 안전하게 보호되며 가능한 경우 되돌릴 수 있습니다

tools-status-ready = 사용 가능
tools-status-coming = 출시 예정
tools-status-beta = 베타 - 버그가 있을 수 있습니다
tools-status-disabled = 현재 사용 중지됨
tools-status-badge-coming = 출시 예정
tools-status-badge-beta = 베타
tools-toast-coming-soon = 이 도구는 곧 출시됩니다
tools-toast-disabled = 이 도구는 현재 사용할 수 없습니다
tools-setup-gate-title = 이 도구를 사용하려면 지갑이 필요합니다

## Tool names.

tools-tool-wallet-cleanup-title = 지갑 정리
tools-tool-wallet-cleanup-summary = 빈 ATA 닫기
tools-tool-wallet-cleanup-description = 빈 Associated Token Account를 닫고 { -sol }을 회수합니다
tools-tool-burn-tokens-title = 토큰 소각
tools-tool-burn-tokens-summary = 토큰 영구 파기
tools-tool-burn-tokens-description = 지갑의 토큰을 영구적으로 파기합니다
tools-tool-token-analyzer-title = 토큰 분석기
tools-tool-token-analyzer-summary = 토큰 심층 분석
tools-tool-token-analyzer-description = 모든 Solana 토큰을 다각도로 심층 분석합니다
tools-tool-create-token-title = 토큰 생성
tools-tool-create-token-summary = 새 SPL 토큰 배포
tools-tool-create-token-description = Solana에 새 SPL 토큰을 배포합니다
tools-tool-trade-watcher-title = 트레이드 워처
tools-tool-trade-watcher-summary = 거래 모니터링 및 자동 실행
tools-tool-trade-watcher-description = 토큰 거래를 모니터링하고 자동 매수/매도를 실행합니다
tools-tool-token-watch-title = 홀더 워치
tools-tool-token-watch-summary = 신규 토큰 홀더 추적
tools-tool-token-watch-description = 신규 토큰 홀더를 실시간으로 추적하고 모니터링합니다
tools-tool-buy-multi-wallets-title = 멀티 매수
tools-tool-buy-multi-wallets-summary = 여러 지갑에서 조율된 매수
tools-tool-buy-multi-wallets-description = 무작위 수량으로 여러 지갑에서 조율된 매수 주문을 실행합니다
tools-tool-sell-multi-wallets-title = 멀티 매도
tools-tool-sell-multi-wallets-summary = 여러 지갑에서 조율된 매도
tools-tool-sell-multi-wallets-description = { -sol } 통합과 함께 여러 지갑에서 조율된 매도 주문을 실행합니다
tools-tool-wallet-consolidation-title = 지갑 통합
tools-tool-wallet-consolidation-nav-title = 통합
tools-tool-wallet-consolidation-summary = 지갑 자금 통합
tools-tool-wallet-consolidation-description = 하위 지갑의 { -sol }과 토큰을 메인 지갑으로 모읍니다
tools-tool-airdrop-checker-title = 에어드롭 확인
tools-tool-airdrop-checker-summary = 대기 중인 에어드롭 확인
tools-tool-airdrop-checker-description = 대기 중인 에어드롭과 수령 가능한 보상을 확인합니다
tools-tool-wallet-generator-title = 지갑 생성기
tools-tool-wallet-generator-summary = 새 키페어 생성
tools-tool-wallet-generator-description = 새 Solana 키페어를 안전하게 생성합니다

## Shared by the tools

tools-validation-mint-required = 토큰 민트 주소를 입력하세요
tools-validation-mint-format = 토큰 민트 주소 형식이 올바르지 않습니다
tools-validation-mint-invalid = 올바른 민트 주소를 입력하세요

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = 토큰 세부 정보
tools-create-token-name-label = 토큰 이름
tools-create-token-name-input =
    .placeholder = My Token
tools-create-token-symbol-label = 심볼
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = 소수 자릿수
tools-create-token-supply-label = 초기 발행량
tools-create-token-description-label = 설명
tools-create-token-description-input =
    .placeholder = 토큰 설명...
tools-create-token-image-title = 토큰 이미지
tools-create-token-image-drop = 이미지를 여기에 놓거나 클릭하여 업로드
tools-create-token-image-hint = 권장: 512x512 PNG
tools-create-token-action-preview = 미리보기
tools-create-token-action-create = 토큰 생성

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = 설정을 불러오는 중...
tools-holder-watch-saved = 홀더 워치 설정이 저장되었습니다
tools-holder-watch-save-failed = 설정을 저장하지 못했습니다
tools-holder-watch-save-error = 설정 저장 중 오류가 발생했습니다
tools-holder-watch-settings-title = 홀더 워치 설정
tools-holder-watch-enabled-label = 홀더 감시 사용
tools-holder-watch-interval-label = 확인 간격 (초)
tools-holder-watch-interval-hint = 홀더 수를 확인하는 주기 (10-3600초)
tools-holder-watch-max-tokens-label = 최대 감시 토큰 수
tools-holder-watch-max-tokens-hint = 동시에 감시할 수 있는 최대 토큰 수
tools-holder-watch-notify-new-label = 신규 홀더 알림
tools-holder-watch-notify-drop-label = 홀더 감소 알림
tools-holder-watch-min-change-label = 최소 홀더 변동
tools-holder-watch-min-change-hint = 알림을 보내는 최소 홀더 변동 수
tools-holder-watch-drop-percent-label = 홀더 감소 기준 (%)
tools-holder-watch-drop-percent-hint = 경고를 보내는 감소 비율
tools-holder-watch-action-save = 설정 저장
tools-holder-watch-tokens-title = 감시 중인 토큰
tools-holder-watch-token-input =
    .placeholder = 토큰 민트 주소 입력...
tools-holder-watch-empty = 감시 중인 토큰이 없습니다
tools-holder-watch-empty-hint = 위에 토큰 민트 주소를 추가하여 감시를 시작하세요
tools-holder-watch-coming-soon = 토큰 감시 기능은 곧 출시됩니다

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = 토큰 분석
tools-analyzer-mint-input =
    .placeholder = 토큰 민트 주소 붙여넣기...
tools-analyzer-action-analyze = 분석
tools-analyzer-action-analyzing = 분석 중...
tools-analyzer-action-copy-report = 리포트 복사
tools-analyzer-loading = 토큰을 분석하는 중...
tools-analyzer-failed = 토큰을 분석하지 못했습니다
tools-analyzer-empty = 분석할 토큰 민트 주소를 입력하세요
tools-analyzer-empty-hint = 모든 Solana 토큰에 대한 종합 인사이트를 확인하세요
tools-analyzer-tab-overview = 개요
tools-analyzer-tab-security = 보안
tools-analyzer-tab-market = 시장
tools-analyzer-tab-liquidity = 유동성
tools-analyzer-unknown-token = 알 수 없는 토큰

# Token header actions.
tools-analyzer-favorite-add =
    .title = 즐겨찾기에 추가
    .aria-label = 즐겨찾기에 추가
tools-analyzer-favorite-already = 이미 즐겨찾기에 있습니다
tools-analyzer-favorite-added = 즐겨찾기에 추가됨: { $symbol }
tools-analyzer-favorite-failed = 즐겨찾기에 추가하지 못했습니다
tools-analyzer-blacklist-add =
    .title = 블랙리스트에 추가
    .aria-label = 블랙리스트에 추가
tools-analyzer-blacklist-title = 토큰 블랙리스트 추가
tools-analyzer-blacklist-message = 블랙리스트에 추가할 토큰: { $symbol }. 이 토큰은 거래에서 제외됩니다.
tools-analyzer-blacklist-confirm = 블랙리스트 추가
tools-analyzer-blacklisted = 블랙리스트 등록됨
tools-analyzer-blacklist-done = 블랙리스트 등록됨: { $symbol }
tools-analyzer-blacklist-failed = 토큰을 블랙리스트에 추가하지 못했습니다

# Overview tab.
tools-analyzer-card-quick-stats = 주요 통계
tools-analyzer-card-market-summary = 시장 요약
tools-analyzer-card-token-info = 토큰 정보
tools-analyzer-stat-holders = 홀더
tools-analyzer-stat-decimals = 소수 자릿수
tools-analyzer-stat-safety-score = 안전 점수
tools-analyzer-stat-pools = 풀
tools-analyzer-stat-volume-24h = 24h 거래량
tools-analyzer-stat-change-24h = 24h 변동
tools-analyzer-stat-market-cap = 시가총액
tools-analyzer-stat-liquidity = 유동성
tools-analyzer-info-mint = 민트 주소
tools-analyzer-info-description = 설명
tools-analyzer-info-supply = 공급량

# Security tab.
tools-analyzer-security-empty = 보안 데이터가 없습니다
tools-analyzer-security-empty-hint = 이 토큰은 보안 분석을 사용할 수 없습니다
tools-analyzer-card-safety-score = 안전 점수
tools-analyzer-score-good = 양호
tools-analyzer-score-moderate = 보통
tools-analyzer-score-risky = 위험
tools-analyzer-raw-score = 원시 위험 점수: { $score }
tools-analyzer-card-authorities = 토큰 권한
tools-analyzer-authority-mint = 민트 권한
tools-analyzer-authority-freeze = 동결 권한
tools-analyzer-authority-transfer-fee = 전송 수수료
tools-analyzer-authority-mutable = 변경 가능
tools-analyzer-authority-active = 활성
tools-analyzer-authority-revoked = 폐기됨
tools-analyzer-card-holder-concentration = 홀더 집중도
tools-analyzer-top-holders = 상위 10개 홀더 보유
tools-analyzer-risks-title = 보안 위험 ({ $count })
tools-analyzer-risks-title-none = 보안 위험
tools-analyzer-risks-none = 감지된 보안 위험이 없습니다

# Market tab.
tools-analyzer-market-empty = 시장 데이터가 없습니다
tools-analyzer-market-empty-hint = 이 토큰은 시장 데이터를 사용할 수 없습니다
tools-analyzer-card-price = 현재 가격
tools-analyzer-card-price-changes = 가격 변동
tools-analyzer-card-volume = 거래량
tools-analyzer-card-transactions = 24h 트랜잭션
tools-analyzer-card-valuation = 가치 평가
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = 1h 거래량
tools-analyzer-stat-volume-6h = 6h 거래량
tools-analyzer-stat-fdv = 완전 희석 가치
tools-analyzer-txn-buys = 매수
tools-analyzer-txn-sells = 매도

# Liquidity tab.
tools-analyzer-liquidity-empty = 유동성 데이터가 없습니다
tools-analyzer-liquidity-empty-hint = 이 토큰의 풀을 찾을 수 없습니다
tools-analyzer-card-total-liquidity = 총 유동성
tools-analyzer-card-pools = 풀
tools-analyzer-active-pools =
    { $count ->
       *[other] 활성 풀
    }
tools-analyzer-card-pool-details = 풀 상세 정보
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = 유동성 ({ -sol })
tools-analyzer-pools-column-status = 상태
tools-analyzer-pool-primary = 주요

# Copied report. Each line is one message; values arrive already formatted.
tools-analyzer-report-empty = 복사할 분석 결과가 없습니다
tools-analyzer-report-label = 분석 리포트
tools-analyzer-report-title = 토큰 분석 리포트
tools-analyzer-report-token = 토큰: { $symbol } ({ $name })
tools-analyzer-report-mint = 민트: { $mint }
tools-analyzer-report-price = 가격: { $sol }
tools-analyzer-report-price-with-usd = 가격: { $sol } ({ $usd })
tools-analyzer-report-security = 보안:
tools-analyzer-report-safety-score = - 안전 점수: { $score }/100
tools-analyzer-report-mint-authority = - 민트 권한: { $state }
tools-analyzer-report-freeze-authority = - 동결 권한: { $state }
tools-analyzer-report-risks = - 위험: { $count }
tools-analyzer-report-market = 시장:
tools-analyzer-report-volume = - 24h 거래량: { $amount }
tools-analyzer-report-change = - 24h 변동: { $amount }
tools-analyzer-report-market-cap = - 시가총액: { $amount }
tools-analyzer-report-liquidity = 유동성:
tools-analyzer-report-liquidity-total = - 합계: { $amount }
tools-analyzer-report-pools = - 풀: { $count }
tools-analyzer-report-generated = 생성 시각: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = 매도 시 매수
tools-watch-type-sell-on-buy = 매수 시 매도
tools-watch-type-notify = 알림
tools-watch-type-notify-only = 알림만

tools-trade-watcher-setup-title = 감시 설정
tools-trade-watcher-mint-label = 토큰 민트 주소
tools-trade-watcher-mint-input =
    .placeholder = 토큰 민트 주소 입력...
tools-trade-watcher-action-search-pools = 풀 검색
tools-trade-watcher-pool-label = 선택한 풀
tools-trade-watcher-pool-none = 선택한 풀이 없습니다
tools-trade-watcher-pool-clear =
    .title = 풀 선택 해제
tools-trade-watcher-pool-selected = 선택한 풀: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = 감시 유형
tools-trade-watcher-type-hint = 매도 시 매수: 누군가 매도하면 자동으로 매수합니다. 매수 시 매도: 누군가 매수하면 자동으로 매도합니다.
tools-trade-watcher-trigger-label = 트리거 수량 ({ -sol })
tools-trade-watcher-trigger-hint = 동작을 실행하는 최소 거래 규모 ({ -sol })
tools-trade-watcher-action-amount-label = 실행 수량 ({ -sol })
tools-trade-watcher-action-amount-hint = 트리거 시 매수/매도할 수량
tools-trade-watcher-slippage-label = 슬리피지 (%)
tools-trade-watcher-slippage-hint = 거래에서 허용하는 최대 슬리피지
tools-trade-watcher-active-title = 활성 감시
tools-trade-watcher-empty = 활성 감시가 없습니다
tools-trade-watcher-empty-hint = 위에서 감시를 설정한 후 "감시 시작"을 클릭하여 모니터링을 시작하세요
tools-trade-watcher-action-start = 감시 시작
tools-trade-watcher-action-starting = 시작하는 중...
tools-trade-watcher-action-stop-all = 모두 중지
tools-trade-watcher-action-stopping = 중지하는 중...
tools-trade-watcher-started = 감시 시작됨: { $token }...
tools-trade-watcher-start-failed = 감시를 시작하지 못했습니다
tools-trade-watcher-stopped = 감시가 중지되었습니다
tools-trade-watcher-stop-failed = 감시를 중지하지 못했습니다
tools-trade-watcher-stopped-all = 모든 감시가 중지되었습니다
tools-trade-watcher-stop-all-failed = 감시를 중지하지 못했습니다
tools-trade-watcher-load-failed = 감시 목록을 불러오지 못했습니다
tools-trade-watcher-column-token = 토큰
tools-trade-watcher-column-type = 유형
tools-trade-watcher-column-trigger = 트리거
tools-trade-watcher-column-action = 동작
tools-trade-watcher-column-triggered = 실행됨
tools-trade-watcher-stop-watch =
    .title = 감시 중지

## Results returned by the tools backend.

tools-burn-failure-native-asset = { -sol }은 소각할 수 없습니다
tools-burn-failure-open-position = 보유 포지션의 토큰은 소각할 수 없습니다
tools-burn-failure-account-not-found = 토큰 계정을 찾을 수 없습니다
tools-burn-failure-zero-balance = 토큰 잔액이 이미 0입니다
tools-burn-failure-transaction = 트랜잭션에 실패했습니다
tools-burn-warning-open-position = 보유 포지션의 토큰은 소각할 수 없습니다
tools-burn-warning-closed-position = 종료된 포지션의 잔여분
tools-burn-warning-worth = 가치 약 { $amount } { -sol }
tools-multi-buy-warning-insufficient = 잔액이 부족합니다. 필요: { $needed } { -sol }, 보유: { $have } { -sol }
tools-multi-buy-warning-over-limit = 필요한 총 { -sol } ({ $needed })이 한도 ({ $limit })를 초과합니다
tools-multi-sell-warning-no-wallets = 보조 지갑을 찾을 수 없습니다
tools-multi-sell-warning-no-balance = 토큰 잔액이 있는 지갑이 없습니다
tools-multi-op-buy-failed = 매수 실패
tools-multi-op-sell-failed = 매도 실패
tools-multi-op-transfer-failed = 전송 실패
tools-multi-op-balance-failed = 잔액을 가져오지 못했습니다
tools-multi-op-mint-invalid = 올바르지 않은 민트 주소
tools-multi-buy-session-failed = 멀티 매수에 실패했습니다
tools-multi-sell-session-failed = 멀티 매도에 실패했습니다
tools-multi-session-aborted = 사용자가 작업을 중단했습니다

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = 지갑 스캔
tools-wallet-action-scanning = 스캔 중...
tools-wallet-scan-failed = 스캔 실패: { $reason }
tools-wallet-amount-approx = ~{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
       *[other] 선택한 지갑: { $count }개
    }
tools-wallet-transfer-failed = 전송 실패: { $reason }
tools-wallet-cleanup-failed = 정리 실패: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = 스캔 결과
tools-wallet-cleanup-stat-empty = 빈 ATA
tools-wallet-cleanup-stat-reclaimable = 회수 가능한 { -sol }
tools-wallet-cleanup-stat-failed = 실패 (캐시됨)
tools-wallet-cleanup-prompt = "지갑 스캔"을 클릭하여 빈 ATA를 찾으세요
tools-wallet-cleanup-prompt-hint = 지갑의 모든 토큰 계정을 확인합니다
tools-wallet-cleanup-action-cleanup = 모두 정리
tools-wallet-cleanup-action-cleaning = 정리 중...
tools-wallet-cleanup-scanning = 지갑을 스캔하는 중...
tools-wallet-cleanup-found =
    { $count ->
       *[other] 빈 ATA { $count }개 발견 (약 { $amount } 상당)
    }
tools-wallet-cleanup-clean = 빈 ATA가 없습니다 - 지갑이 깨끗합니다!
tools-wallet-cleanup-scan-failed = ATA를 스캔하지 못했습니다
tools-wallet-cleanup-done =
    { $count ->
       *[other] ATA { $count }개 정리 완료
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = 토큰 소각
tools-burn-info-title = 소각이란?
tools-burn-info-body = 소각은 토큰을 영구적으로 파기하며 복구할 수 없습니다. 소각 후 지갑 정리를 실행하면 빈 ATA를 닫고 토큰당 약 0.002 { -sol }의 렌트를 회수할 수 있습니다.
tools-burn-stat-total = 전체 토큰
tools-burn-stat-selected = 선택됨
tools-burn-stat-rent = 회수 가능한 렌트
tools-burn-prompt = "지갑 스캔"을 클릭하여 토큰을 찾으세요
tools-burn-scanning = 지갑에서 토큰을 스캔하는 중...
tools-burn-scan-failed = 토큰을 스캔하지 못했습니다
tools-burn-empty = 지갑에 토큰이 없습니다
tools-burn-action-burn = 선택 항목 소각 ({ $count })
tools-burn-action-burning = 소각 중...
tools-burn-cannot-burn = 소각 불가
tools-burn-no-value = 가치 없음

# Category titles and descriptions. Ids are the token categories of the scan.
tools-burn-category-open-position = 보유 포지션
tools-burn-category-has-value = 가치 있음
tools-burn-category-closed-position = 종료된 포지션
tools-burn-category-zero-liquidity = 유동성 없음
tools-burn-category-hint-open-position = 보유 포지션의 토큰은 소각할 수 없습니다
tools-burn-category-hint-has-value = 소각 대신 매도를 고려하세요
tools-burn-category-hint-closed-position = 종료된 거래의 잔여분
tools-burn-category-hint-zero-liquidity = 시장 가치가 없어 소각해도 안전합니다

tools-burn-confirm-title = 소각 확인
tools-burn-confirm-message =
    { $count ->
       *[other] 토큰 <strong>{ $count }</strong>개를 소각하시겠습니까?
    }
tools-burn-confirm-value = 총 예상 가치: <strong>{ $amount }</strong>
tools-burn-confirm-continue = 계속
tools-burn-final-title = 최종 경고
tools-burn-final-headline = 이 작업은 되돌릴 수 없습니다!
tools-burn-final-message =
    { $count ->
       *[other] 다음 토큰 { $count }개가 영구적으로 파기되며 어떠한 경우에도 복구할 수 없습니다.
    }
tools-burn-final-confirm = 예, 토큰 소각
tools-burn-toast-burned =
    { $total ->
       *[other] 소각 완료: { $successful }/{ $total }개. 지갑 정리를 실행하여 약 { $amount }을 회수하세요
    }
tools-burn-toast-failed =
    { $count ->
       *[other] 토큰 { $count }개를 소각하지 못했습니다
    }
tools-burn-failed = 소각 실패: { $reason }
tools-burn-failures-title =
    { $count ->
       *[other] 소각할 수 없는 토큰: { $count }개
    }
tools-burn-failure-unknown = 보고된 사유가 없습니다

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = 소개
tools-airdrop-about-body = 주요 Solana 프로토콜 전반에서 대기 중인 에어드롭, 수령 가능한 보상, 미수령 할당량을 확인합니다.
tools-airdrop-list-title = 사용 가능한 에어드롭
tools-airdrop-prompt = "에어드롭 확인"을 클릭하여 수령 가능한 항목을 스캔하세요
tools-airdrop-action-check = 에어드롭 확인
tools-airdrop-action-claim-all = 모두 수령

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = 생성 옵션
tools-generator-warning-title = 프라이빗 키를 안전하게 보관하세요!
tools-generator-warning-body = 생성된 키페어는 로컬에서 만들어지며 외부로 전송되지 않습니다. 키는 반드시 안전한 위치에 백업하세요.
tools-generator-count-label = 지갑 수
tools-generator-vanity-label = 베니티 주소 (특정 문자로 시작)
tools-generator-prefix-label = 접두사
tools-generator-prefix-input =
    .placeholder = 예: SOL
tools-generator-prefix-hint = 접두사가 길수록 생성 시간이 기하급수적으로 늘어납니다
tools-generator-list-title = 생성된 지갑
tools-generator-empty = 아직 생성된 지갑이 없습니다
tools-generator-action-generate = 생성
tools-generator-action-generating = 생성 중...
tools-generator-count-invalid = 1에서 10 사이의 숫자를 입력하세요
tools-generator-no-keypairs = 반환된 키페어가 없습니다
tools-generator-generated =
    { $count ->
       *[other] 지갑 { $count }개 생성됨
    }
tools-generator-failed = 지갑을 생성하지 못했습니다: { $reason }
tools-generator-copy-public-key =
    .title = 퍼블릭 키 복사
tools-generator-copy-private-key =
    .title = 프라이빗 키 복사
tools-generator-remove =
    .title = 목록에서 제거
tools-generator-reveal =
    .title = 프라이빗 키 표시
tools-generator-public-key-label = 퍼블릭 키:
tools-generator-private-key-label = 프라이빗 키:
tools-generator-public-key-name = 퍼블릭 키
tools-generator-private-key-copied = 프라이빗 키가 복사되었습니다
tools-generator-private-key-warning = 이 키를 가진 사람은 누구나 지갑을 제어할 수 있습니다
tools-generator-export-empty = 내보낼 지갑이 없습니다
tools-generator-exported = 지갑을 내보냈습니다 - 안전하게 보관하세요

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = 요약
tools-consolidation-stat-wallets = 하위 지갑
tools-consolidation-stat-native = 총 { -sol }
tools-consolidation-stat-tokens = 토큰 종류
tools-consolidation-stat-rent = 회수 가능한 렌트
tools-consolidation-wallets-title = 지갑
tools-consolidation-loading-wallets = 지갑을 불러오는 중...
tools-consolidation-loading-data = 지갑 데이터를 불러오는 중...
tools-consolidation-action-transfer-native = { -sol } 전송
tools-consolidation-action-transfer-tokens = 모든 토큰 전송
tools-consolidation-action-cleanup = ATA 정리
tools-consolidation-action-transferring = 전송 중...
tools-consolidation-column-name = 이름
tools-consolidation-column-native = { -sol } 잔액
tools-consolidation-column-tokens = 토큰
tools-consolidation-column-atas = 빈 ATA
tools-consolidation-empty = 하위 지갑이 없습니다
tools-consolidation-empty-hint = 멀티 매수로 하위 지갑을 만들어 시작하세요
tools-consolidation-load-failed = 불러오기 실패: { $reason }
tools-consolidation-select-prompt = 통합할 지갑을 선택하세요
tools-consolidation-selection-totals =
    | { $amount } | 토큰 { $tokens }개 | 빈 ATA { $atas }개
tools-consolidation-transferred-native = 메인 지갑으로 { $amount } 전송됨
tools-consolidation-transferred-tokens =
    { $count ->
       *[other] 토큰 { $count }개를 메인 지갑으로 전송했습니다
    }
tools-consolidation-cleaned =
    { $count ->
       *[other] ATA { $count }개를 닫고 { $amount }을 회수했습니다
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = 토큰
tools-multi-mint-label = 토큰 민트 주소
tools-multi-mint-input =
    .placeholder = 토큰 민트 주소 붙여넣기...
tools-multi-execution-title = 실행 설정
tools-multi-delay-min-label = 최소 지연 (ms)
tools-multi-delay-max-label = 최대 지연 (ms)
tools-multi-concurrency-label = 동시 실행
tools-multi-concurrency-sequential = { $count } (순차)
tools-multi-concurrency-parallel = 병렬 { $count }
tools-multi-slippage-label = 슬리피지 (%)
tools-multi-router-label = 라우터
tools-multi-router-auto = 자동 (최적 경로)
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = 직접 풀
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = 진행 상황
tools-multi-progress-preparing = 준비 중...
tools-multi-status-line = { $label } ({ $completed }/{ $total })
tools-multi-column-wallet = 지갑
tools-multi-column-route = 경로
tools-multi-column-status = 상태
tools-multi-op-completed = 완료
tools-multi-op-failed = 실패
tools-multi-action-stop = 중지
tools-multi-action-loading = 불러오는 중...
tools-multi-start-failed = 시작 실패: { $reason }

# Session states. Ids are the states of a multi-wallet session.
tools-multi-state-pending = 대기 중
tools-multi-state-funding = 자금 충전 중
tools-multi-state-executing = 실행 중
tools-multi-state-consolidating = 통합 중
tools-multi-state-completed = 완료
tools-multi-state-failed = 실패
tools-multi-state-aborted = 중단됨

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = 여러 지갑에서 매수할 토큰
tools-multi-buy-wallets-title = 지갑 설정
tools-multi-buy-wallet-count-label = 지갑 수
tools-multi-buy-wallet-count-option =
    { $count ->
       *[other] { $count }개
    }
tools-multi-buy-wallet-count-hint = 사용할 하위 지갑 수
tools-multi-buy-buffer-label = 지갑당 { -sol } 버퍼
tools-multi-buy-buffer-hint = 수수료용으로 예약 (최소 0.015 { -sol })
tools-multi-buy-amounts-title = 수량 설정
tools-multi-buy-min-label = 지갑당 최소 { -sol }
tools-multi-buy-min-hint = 최소 매수 수량
tools-multi-buy-max-label = 지갑당 최대 { -sol }
tools-multi-buy-max-hint = 최대 매수 수량
tools-multi-buy-limit-label = 총 { -sol } 한도 (선택)
tools-multi-buy-limit-hint = 최대 총 지출
tools-multi-buy-preview-title = 미리보기
tools-multi-buy-preview-create = 생성할 지갑
tools-multi-buy-preview-amount = 지갑당 수량
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = 필요한 총 { -sol }
tools-multi-buy-preview-balance = 메인 지갑 잔액
tools-multi-buy-action-preview = 미리보기
tools-multi-buy-action-start = 멀티 매수 시작
tools-multi-buy-executing = 매수를 실행하는 중...
tools-multi-buy-column-spent = 사용한 { -sol }
tools-multi-buy-column-tokens = 토큰
tools-multi-buy-preview-failed = 미리보기 실패: { $reason }
tools-multi-buy-started = 멀티 매수가 시작되었습니다
tools-multi-buy-stopped = 멀티 매수가 중지되었습니다
tools-multi-buy-completed = 멀티 매수 완료! 성공 { $successful }/{ $total }

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = 토큰 주소를 입력하면 해당 토큰을 보유한 지갑을 스캔합니다
tools-multi-sell-action-scan = 스캔
tools-multi-sell-settings-title = 매도 설정
tools-multi-sell-percent-label = 매도 비율
tools-multi-sell-percent-hint = 지갑당 매도할 토큰 비율 (%)
tools-multi-sell-min-fee-label = 수수료용 최소 { -sol }
tools-multi-sell-min-fee-hint = 트랜잭션 수수료에 필요한 최소 { -sol }
tools-multi-sell-topup-label = 필요 시 자동 충전
tools-multi-sell-topup-hint = 하위 지갑 잔액이 부족하면 메인 지갑에서 { -sol }을 전송합니다
tools-multi-sell-post-title = 매도 후 작업
tools-multi-sell-consolidate-label = { -sol }을 메인 지갑으로 통합
tools-multi-sell-consolidate-hint = 하위 지갑의 모든 { -sol }을 메인 지갑으로 되돌립니다
tools-multi-sell-close-atas-label = 매도 후 토큰 ATA 닫기
tools-multi-sell-close-atas-hint = ATA당 약 0.002 { -sol } 회수
tools-multi-sell-wallets-title = 토큰 보유 지갑
tools-multi-sell-empty = 이 토큰을 보유한 하위 지갑이 없습니다
tools-multi-sell-column-tokens = 토큰
tools-multi-sell-column-native = { -sol } 잔액
tools-multi-sell-column-topup = 충전 필요
tools-multi-sell-none-selected = 선택한 지갑이 없습니다
tools-multi-sell-select-required = 지갑을 하나 이상 선택하세요
tools-multi-sell-action-start = 멀티 매도 시작
tools-multi-sell-executing = 매도를 실행하는 중...
tools-multi-sell-column-sold = 매도한 토큰
tools-multi-sell-column-received = 수령한 { -sol }
tools-multi-sell-started = 멀티 매도가 시작되었습니다
tools-multi-sell-stopped = 멀티 매도가 중지되었습니다
tools-multi-sell-completed = 멀티 매도 완료! 수령: { $amount }

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = 즐겨찾기
tools-favorites-saved = 저장된 즐겨찾기
tools-favorites-save-current = 현재 설정 저장
tools-favorites-empty = 저장된 즐겨찾기가 없습니다
tools-favorites-no-label = 라벨 없음
tools-favorites-uses = { $count }회
tools-favorites-remove = 제거
tools-favorites-loaded = 즐겨찾기 불러옴: { $name }
tools-favorites-default-name = 설정
tools-favorites-mint-required = 먼저 토큰 민트 주소를 입력하세요
tools-favorites-add-title = 즐겨찾기 추가
tools-favorites-add-message = 이 즐겨찾기의 라벨을 입력하세요
tools-favorites-add-placeholder = 라벨 (선택)...
tools-favorites-saved-toast = 즐겨찾기에 저장되었습니다
tools-favorites-save-failed = 즐겨찾기를 저장하지 못했습니다
tools-favorites-remove-title = 즐겨찾기 제거
tools-favorites-remove-message = 이 즐겨찾기를 제거하시겠습니까?
tools-favorites-removed-toast = 즐겨찾기가 제거되었습니다
tools-favorites-remove-failed = 즐겨찾기를 제거하지 못했습니다
