# Contextual hints: one title and one body per hint, keyed by the hint id.
# Bodies use a small markdown subset (**bold** and bullet lines) that the hint popover renders.

# Source: scripts/core/hints.js

## Categories

hints-category-tokens = 토큰
hints-category-positions = 포지션
hints-category-filtering = 필터링
hints-category-trader = 자동 트레이더
hints-category-services = 서비스
hints-category-wallet = 지갑
hints-category-wallets = 지갑
hints-category-tools = 도구
hints-category-config = 설정
hints-category-config-telegram = { -telegram }
hints-category-token-details = 토큰 상세
hints-category-ui = 인터페이스

## tokens

hints-tokens-pool-service-title = 풀 서비스 토큰
hints-tokens-pool-service-content =
    여기에 표시되는 토큰은 다음 조건을 충족합니다:

    • **모든 필터링 기준 통과** — 유동성, 거래량, 경과 기간, 보안 검사
    • **유효한 SOL 유동성 풀** — DEX 디코더(Raydium, Orca, Meteora 등)가 지원하는 풀
    • **가격 계산 성공** — 온체인 풀 리저브에서 직접 계산한 가격

    가격이 외부 API가 아닌 실제 풀 데이터에서 산출되므로 거래에 가장 신뢰할 수 있는 토큰 목록입니다.

    토큰을 클릭하면 상세 정보를 확인하고 블랙리스트 상태를 관리할 수 있습니다.
hints-tokens-no-market-title = 시장 데이터 없음
hints-tokens-no-market-content =
    온체인에서 발견되었지만 { -dexscreener } 또는 { -geckoterminal }의 시장 데이터가 없는 토큰입니다.

    주요 원인:
    • **매우 새로운 토큰** — 아직 애그리게이터에 색인되지 않음
    • **낮은 거래량** — 애그리게이터 기준 미달
    • **미등록 페어** — 애그리게이터가 추적하지 않는 DEX에서 거래

    이 토큰들도 유효한 풀이 있을 수 있고 거래할 수 있지만, 외부 시장 지표는 없습니다.
hints-tokens-all-title = 전체 토큰
hints-tokens-all-content =
    필터링 상태와 관계없이 발견된 모든 토큰의 전체 데이터베이스입니다.

    포함 항목:
    • 필터링을 통과한 토큰
    • 제외된 토큰
    • 시장 데이터가 없는 토큰
    • 블랙리스트 토큰

    조사 목적이나 필터링에서 제외된 토큰을 찾을 때 이 보기를 사용하세요.
hints-tokens-passed-title = 필터링 통과
hints-tokens-passed-content =
    활성화된 모든 필터링 기준을 통과한 토큰입니다.

    필터링 검사 항목:
    • **유동성** — 최소 SOL 유동성 기준
    • **거래량** — 24h 거래량 요건
    • **토큰 경과 기간** — 생성 이후 최소 경과 시간
    • **보안** — { -rugcheck } 위험 점수 한도
    • **시가총액** — 선택적 FDV/시가총액 필터

    필터는 **필터링** 페이지에서 설정하세요.
hints-tokens-rejected-title = 제외된 토큰
hints-tokens-rejected-content =
    하나 이상의 필터링 기준을 통과하지 못한 토큰입니다.

    각 토큰에는 구체적인 제외 사유가 표시됩니다:
    • 통과하지 못한 필터
    • 실제 값과 요구 기준값 비교
    • 검사가 이루어진 시각

    제외된 토큰을 검토하여 필터 설정을 세밀하게 조정하세요.
hints-tokens-blacklisted-title = 블랙리스트 토큰
hints-tokens-blacklisted-content =
    거래에서 영구적으로 제외된 토큰입니다.

    블랙리스트 사유:
    • **수동 블랙리스트** — 직접 차단한 토큰
    • **보안 위험** — 러그풀 징후 감지
    • **손실 기준** — 설정한 손실 한도 초과
    • **트랜잭션 실패** — 반복된 스왑 실패

    블랙리스트 토큰은 통과 목록에 표시되지 않으며 자동 거래 대상에서도 제외됩니다.
hints-tokens-positions-title = 포지션 토큰
hints-tokens-positions-content =
    현재 보유 포지션으로 가지고 있는 토큰입니다.

    보유 중인 자산의 실시간 데이터를 표시합니다:
    • 풀 리저브 기준 현재 가격
    • 미실현 손익
    • 포지션 규모와 진입 가격
    • 보유 시간

    토큰을 클릭하면 상세 포지션 관리로 이동합니다.
hints-tokens-recent-title = 최근 발견
hints-tokens-recent-content =
    발견 시각 순으로 정렬된 새로 발견된 토큰입니다.

    활용 예:
    • 신규 토큰 출시 포착
    • 새로운 유동성 모니터링
    • 초기 진입 기회

    참고: 신규 토큰은 처음에 시장 데이터가 완전하지 않을 수 있습니다.
hints-tokens-ohlcv-title = OHLCV 데이터 관리
hints-tokens-ohlcv-content =
    토큰별로 저장된 OHLCV(캔들) 데이터를 확인하고 관리합니다.

    표시 항목:
    • **캔들 수** — 저장된 전체 데이터 포인트
    • **백필 진행률** — 타임프레임별 완료 상태
    • **데이터 범위** — 시간 단위 커버리지
    • **풀 수** — 추적 중인 유동성 풀
    • **상태** — 모니터링 중 또는 비활성

    작업:
    • **삭제** — 토큰의 모든 OHLCV 데이터 제거
    • **정리** — 비활성 토큰 데이터 일괄 제거

    OHLCV 데이터는 영구 보존되며 자동으로 삭제되지 않습니다.

## positions

hints-positions-overview-title = 포지션 개요
hints-positions-overview-content =
    현재 보유 중인 토큰과 거래 포지션입니다.

    주요 지표:
    • **진입 가격** — 지불한 평균 가격 (DCA 포함)
    • **현재 가격** — 풀 리저브 기준 실시간 가격
    • **손익** — SOL 및 % 기준 미실현 손익
    • **규모** — 보유한 전체 토큰 수량

    포지션을 클릭하면 상세 관리 옵션을 볼 수 있습니다.
hints-positions-dca-title = DCA (분할 매수)
hints-positions-dca-content =
    DCA로 기존 포지션에 서로 다른 가격으로 추가 매수할 수 있습니다.

    DCA가 실행되면:
    • 추가 토큰이 매수됩니다
    • 진입 가격이 가중 평균으로 다시 계산됩니다
    • 포지션 규모가 늘어납니다
    • 진입 횟수가 증가합니다

    DCA 규칙은 **자동 트레이더** 설정에서 구성하세요.
hints-positions-partial-exit-title = 부분 청산
hints-positions-partial-exit-content =
    포지션의 일부만 매도하고 나머지는 유지합니다.

    장점:
    • 노출을 유지하면서 일부 수익 확정
    • 포지션을 완전히 종료하지 않고 규모 축소
    • 익절 사다리 구현

    각 부분 청산은 정확한 손익 추적을 위해 별도로 기록됩니다.
hints-positions-management-title = 포지션 관리
hints-positions-management-content =
    관리 방식은 포지션에 어떤 자동화가 동작할 수 있는지 정의합니다:

    • 자동 트레이더: 안전 청산, 정책 청산, 자동 DCA
    • 사용자 전용: 자동 동작 없음
    • 카피 작업: 안전 청산 및 카피 매도
    • 하이브리드: 안전 청산, 정책 청산, 카피 매도

    매도나 추가 매수는 직접 수행합니다. 수동 매수는 기본적으로 수동 관리로 설정되어 봇이 의도적으로 매수한 토큰을 매도하지 않습니다. 해제하면 포지션을 자동 트레이더에게 돌려줍니다.

## filtering

hints-filtering-overview-title = 토큰 필터링
hints-filtering-overview-content =
    필터링은 어떤 토큰이 거래 대상이 되는지 결정합니다.

    통과 목록에 표시되려면 토큰이 **활성화된 모든 기준**을 통과해야 합니다:
    • { -dexscreener } 지표 (유동성, 거래량 등)
    • { -geckoterminal } 지표 (시가총액, FDV)
    • { -rugcheck } 보안 분석
    • 메타 필터 (토큰 경과 기간 등)

    비활성화된 기준은 완전히 건너뜁니다.
hints-filtering-dexscreener-title = { -dexscreener } 필터
hints-filtering-dexscreener-content =
    { -dexscreener } 시장 데이터 기반 필터:

    • **유동성** — 풀의 최소 USD 유동성
    • **거래량 24h** — 최소 거래량
    • **트랜잭션** — 활동 기준 (매수/매도)
    • **가격 변동** — 변동성 필터

    { -dexscreener } 데이터는 몇 분마다 업데이트됩니다.
hints-filtering-geckoterminal-title = { -geckoterminal } 필터
hints-filtering-geckoterminal-content =
    { -geckoterminal } 시장 데이터 기반 필터:

    • **시가총액** — 최소 시가총액
    • **FDV** — 완전 희석 가치 한도
    • **리저브 비율** — 풀 건전성 지표

    { -geckoterminal }에는 비교적 새로운 토큰의 데이터가 있는 경우가 많습니다.
hints-filtering-rugcheck-title = 보안 필터
hints-filtering-rugcheck-content =
    { -rugcheck }.xyz의 보안 분석:

    • **위험 점수** — 전체 위험 등급 (0-100)
    • **민트 권한** — 새 토큰을 발행할 수 있는가?
    • **동결 권한** — 전송을 동결할 수 있는가?
    • **상위 홀더** — 집중 위험

    위험 점수가 높을수록 잠재적인 위험 신호가 많다는 뜻입니다.
hints-filtering-meta-title = 메타 필터
hints-filtering-meta-content =
    추가 필터링 기준:

    • **토큰 경과 기간** — 토큰 생성 이후 최소 경과 시간
    • **풀 경과 기간** — 풀 생성 이후 최소 경과 시간
    • **웹사이트 보유** — 소셜/웹사이트 링크 필수
    • **소셜 보유** — Twitter/{ -telegram } 필수

    매우 새롭거나 의심스러운 토큰을 걸러내는 데 도움이 됩니다.

## trader

hints-trader-overview-title = 자동 트레이더
hints-trader-overview-content =
    토큰을 모니터링하고 거래를 실행하는 자동 거래 엔진입니다.

    구성 요소:
    • **진입 모니터** — 매수 기회 감시
    • **청산 모니터** — 매도와 익절 관리
    • **DCA 모니터** — 포지션 평균화 처리
    • **위험 제어** — 손실 한도와 안전 게이트

    제어판에서 거래를 시작/중지하세요.
hints-trader-entry-title = 진입 모니터
hints-trader-entry-content =
    필터링된 토큰에서 진입 시그널을 감시합니다.

    진입 평가 검사:
    • 토큰이 현재 필터링을 통과
    • 이미 포지션이 없음
    • 블랙리스트에 없음
    • 포지션 한도 미초과
    • 전략 조건 충족 (설정된 경우)

    진입 규모와 한도는 설정에서 구성하세요.
hints-trader-exit-title = 청산 모니터
hints-trader-exit-content =
    보유 포지션에서 청산 시그널을 모니터링합니다.

    청산 트리거:
    • **익절** — 목표 가격 도달
    • **손절** — 최대 손실 초과
    • **트레일링 스톱** — 고점에서 가격이 되돌림
    • **전략 청산** — 사용자 지정 조건 충족
    • **시간 기반** — 최대 보유 시간

    기준값은 설정에서 구성하세요.

## services

hints-services-overview-title = 시스템 서비스
hints-services-overview-content =
    { -brand }을 구동하는 백그라운드 서비스입니다.

    서비스 상태:
    • **실행 중** (녹색) — 정상 작동
    • **시작 중** (노란색) — 초기화 중
    • **중지됨** (빨간색) — 실행되지 않음
    • **오류** (경고) — 실패, 자동 재시작될 수 있음

    서비스에는 의존 관계가 있으며 순서대로 시작됩니다.
hints-services-health-title = 서비스 상태
hints-services-health-content =
    상태 표시기는 서비스 상태를 보여줍니다:

    • **가동 시간** — 마지막 시작 이후 경과 시간
    • **작업** — 활성 백그라운드 작업
    • **오류** — 최근 오류 수
    • **지표** — 성능 데이터 (있는 경우)

    핵심 서비스는 거래 기능에 영향을 줍니다.

## wallet

hints-wallet-overview-title = 지갑 개요
hints-wallet-overview-content =
    연결된 Solana 지갑의 상태입니다.

    표시 항목:
    • **SOL 잔액** — 가스와 거래에 사용하는 네이티브 SOL
    • **토큰 보유 현황** — 가치가 포함된 SPL 토큰
    • **24h 변동** — 포트폴리오 가치 변동
    • **기록** — 시간별 잔액 스냅샷

    잔액은 1분마다 새로 고침됩니다.
hints-wallet-tokens-title = 토큰 잔액
hints-wallet-tokens-content =
    지갑에 보유한 SPL 토큰입니다.

    표시 항목:
    • 토큰 심볼과 이름
    • 보유 수량
    • SOL/USD 기준 현재 가치
    • 풀 또는 시장 데이터 기준 가격

    빈 토큰 계정은 설정에서 정리할 수 있습니다.

## wallets

hints-wallets-main-title = 메인 지갑
hints-wallets-main-content =
    모든 거래 작업에 사용하는 기본 지갑입니다.

    • **자동 거래** — 진입/청산 거래가 이 지갑에서 실행됩니다
    • **잔액 표시** — 헤더와 대시보드에 표시됩니다
    • **토큰 보유 현황** — 이 지갑이 보유한 SPL 토큰

    보조 지갑에서 "메인으로 설정"을 선택하면 메인 지갑을 변경할 수 있습니다.
hints-wallets-secondary-title = 보조 지갑
hints-wallets-secondary-content =
    멀티 지갑 작업을 위한 추가 지갑입니다.

    • **멀티 지갑 거래** — 여러 지갑에서 매수/매도를 조율
    • **포트폴리오 분리** — 전략이나 목적별로 구성
    • **독립된 잔액** — 각 지갑이 자체 SOL/토큰을 보유

    보조 지갑은 명시적으로 설정하지 않으면 자동 거래에 사용되지 않습니다.

## tools

hints-tools-wallet-cleanup-title = 지갑 정리 도구
hints-tools-wallet-cleanup-content =
    { "*" }*빈 토큰 계정에서 SOL 회수**

    { "*" }*ATA란?**
    Associated Token Account(ATA)는 토큰을 보관하는 Solana 계정입니다. 상호작용한 토큰마다 ATA가 생성되며 약 0.002 SOL의 렌트가 필요합니다.

    { "*" }*빈 ATA를 정리해야 하는 이유?**
    • 렌트 회수 (ATA당 약 0.002 SOL)
    • 활발한 트레이더는 빈 ATA가 수백 개 쌓일 수 있습니다
    • 빈 ATA 100개 = 약 0.2 SOL 회수 가능

    { "*" }*작동 방식:**
    • 잔액이 0인 ATA를 지갑에서 스캔
    • 회수 가능한 총 SOL 금액 표시
    • 빈 계정을 닫아 렌트 회수

    { "*" }*자동 정리:**
    활성화하면 5분마다 백그라운드에서 빈 ATA를 자동으로 스캔하고 닫습니다.

    { "*" }*중요:**
    • 잔액이 정확히 0인 계정만 닫습니다
    • 닫기에 실패한 계정은 반복 재시도를 막기 위해 캐시됩니다
    • 큰 지갑은 여러 번 정리해야 할 수 있습니다
hints-tools-burn-tokens-title = 토큰 소각 도구
hints-tools-burn-tokens-content =
    { "*" }*토큰 영구 파기**

    토큰을 소각하면 지갑과 유통에서 영구적으로 제거됩니다.

    { "*" }*소각 시 일어나는 일:**
    • 토큰이 소각 주소로 전송됩니다 (복구 불가)
    • 토큰 잔액이 0이 됩니다
    • 이후 지갑 정리로 ATA를 닫아 약 0.002 SOL 렌트를 회수할 수 있습니다

    { "*" }*토큰 카테고리:**
    • **보유 포지션** - 소각 불가 (진행 중인 거래)
    • **종료된 포지션** - 과거 거래의 잔여분
    • **가치 있음** - 유동성이 있는 토큰 (소각 대신 매도 고려)
    • **유동성 없음** - 먼지/무가치 토큰 (소각해도 안전)

    { "*" }*경고:** 이 작업은 **되돌릴 수 없습니다**. 소각된 토큰은 어떠한 경우에도 복구할 수 없습니다.

    { "*" }*소각 후:** 지갑 정리를 실행하여 빈 ATA를 닫고 SOL 렌트를 회수하세요.
hints-tools-wallet-generator-title = 지갑 생성기 도구
hints-tools-wallet-generator-content =
    { "*" }*새 Solana 키페어 생성**

    기기에서 안전하게 새 지갑을 만듭니다.

    { "*" }*기능:**
    • 암호학적으로 안전한 키페어 생성
    • 선택적 베니티 주소 접두사 (예: "SOL...")
    • base58 또는 JSON 배열로 내보내기

    { "*" }*보안:**
    • 키는 로컬에서 생성됩니다
    • 네트워크로 전송되지 않습니다
    • 키는 항상 안전하게 백업하세요
hints-tools-multi-buy-title = 멀티 매수 도구
hints-tools-multi-buy-content =
    { "*" }*여러 지갑에서 조율된 매수**

    무작위 수량으로 여러 하위 지갑에서 매수 주문을 실행하여 자연스러운 매수 활동을 재현합니다.

    { "*" }*작동 방식:**
    1. 하위 지갑을 새로 만들거나 기존 지갑을 사용
    2. 메인 지갑의 SOL을 하위 지갑에 분배
    3. 무작위 수량과 지연으로 매수 주문 실행
    4. 각 지갑이 고유한 서명으로 독립적으로 매수

    { "*" }*지갑 설정:**
    • **지갑 수** — 사용할 하위 지갑 수 (2-10)
    • **SOL 버퍼** — 수수료용으로 지갑마다 예약하는 SOL (약 0.015)

    { "*" }*수량 설정:**
    • **최소/최대 SOL** — 지갑당 매수 수량 범위
    • **총 한도** — 지출할 총 SOL의 선택적 상한

    { "*" }*실행 설정:**
    • **지연** — 트랜잭션 사이의 무작위 지연
    • **동시 실행** — 병렬 실행 (1 = 순차)
    • **슬리피지** — 허용하는 최대 슬리피지
    • **라우터** — 스왑 라우팅 (자동, Jupiter, Raydium)

    { "*" }*중요:**
    • 메인 지갑에 충분한 SOL이 필요합니다
    • 실패한 매수는 기록되지만 세션은 중단되지 않습니다
    • 하위 지갑은 여러 세션에서 재사용할 수 있습니다
hints-tools-multi-sell-title = 멀티 매도 도구
hints-tools-multi-sell-content =
    { "*" }*여러 지갑에서 조율된 매도**

    특정 토큰을 보유한 모든 하위 지갑에서 토큰을 매도하고 SOL을 자동으로 통합합니다.

    { "*" }*작동 방식:**
    1. 하위 지갑의 토큰 잔액 스캔
    2. 수수료용 SOL이 부족한 지갑은 선택적으로 충전
    3. 설정한 비율로 매도 주문 실행
    4. 수익금을 메인 지갑으로 통합

    { "*" }*매도 설정:**
    • **매도 %** — 매도할 토큰 비율 (기본값 100%)
    • **수수료용 최소 SOL** — 트랜잭션에 필요한 최소 SOL
    • **자동 충전** — 필요 시 메인 지갑에서 SOL 전송

    { "*" }*매도 후 작업:**
    • **SOL 통합** — 모든 SOL을 메인 지갑으로 전송
    • **ATA 닫기** — 토큰 계정을 닫아 렌트 회수 (각 약 0.002 SOL)

    { "*" }*실행 설정:**
    • **지연** — 트랜잭션 사이의 무작위 지연
    • **동시 실행** — 병렬 실행
    • **슬리피지** — 허용하는 최대 슬리피지
    • **라우터** — 스왑 라우팅 선호도

    { "*" }*팁:**
    • 미리보기에서 해당 토큰을 보유한 모든 지갑을 볼 수 있습니다
    • 매도하지 않을 지갑은 선택 해제하세요
    • 통합은 모든 매도가 끝난 후 진행됩니다
hints-tools-trade-watcher-title = 트레이드 워처 도구
hints-tools-trade-watcher-content =
    { "*" }*거래 모니터링 및 자동 동작 실행**

    토큰의 거래 활동을 감시하고 거래가 발생하면 자동으로 반응합니다.

    { "*" }*감시 유형:**
    • **매도 시 매수** — 누군가 매도하면 자동 매수 (하락 포착)
    • **매수 시 매도** — 누군가 매수하면 자동 매도 (시장 추종)
    • **알림만** — 동작 없이 알림만 수신

    { "*" }*작동 방식:**
    1. 토큰 민트 주소 입력
    2. "풀 검색"을 클릭하여 사용 가능한 유동성 풀 찾기
    3. 모니터링할 풀 선택 (매수/매도 동작에 필요)
    4. 트리거 수량 설정 (반응할 최소 거래 규모)
    5. 실행 수량 설정 (매수/매도할 SOL 양)
    6. 감시 시작

    { "*" }*요구 사항:**
    • 유효한 토큰 민트 주소
    • 풀 선택 (매수/매도 동작용)
    • 실행 수량에 충분한 SOL 잔액

    { "*" }*{ -telegram } 연동:**
    설정 → { -telegram }에서 { -telegram } 연동을 구성하면 감시가 트리거될 때 즉시 알림을 받을 수 있습니다.
hints-tools-wallet-consolidation-title = 지갑 통합 도구
hints-tools-wallet-consolidation-content =
    { "*" }*하위 지갑 자금 관리 및 통합**

    모든 하위 지갑을 확인하고 SOL, 토큰, ATA 렌트를 메인 지갑으로 모읍니다.

    { "*" }*요약 표시 항목:**
    • **하위 지갑** — 생성된 하위 지갑의 총 수
    • **총 SOL** — 모든 하위 지갑의 SOL 잔액 합계
    • **토큰 종류** — 보유한 서로 다른 토큰의 수
    • **회수 가능한 렌트** — 빈 ATA에 묶인 SOL

    { "*" }*작업:**
    • **SOL 전송** — 선택한 지갑의 모든 SOL을 메인으로 이동
    • **토큰 전송** — 모든 토큰을 메인 지갑으로 이동
    • **ATA 정리** — 빈 토큰 계정을 닫아 렌트 환급

    { "*" }*테이블 정보:**
    • 일괄 작업할 지갑을 선택하는 체크박스
    • 이름, 주소, SOL 잔액, 토큰 수, 빈 ATA
    • 빈 지갑은 쉽게 구분되도록 흐리게 표시됩니다

    { "*" }*팁:**
    • 멀티 매도 후에 사용하여 남은 SOL을 모으세요
    • ATA를 정기적으로 정리하여 렌트를 회수하세요
    • 빈 지갑은 이후 작업에 재사용할 수 있습니다

## config

hints-config-overview-title = 설정
hints-config-overview-content =
    { -brand }의 시스템 전체 설정입니다.

    카테고리:
    • **트레이더** — 진입/청산 규칙, 포지션 규모
    • **필터링** — 토큰 필터 기준값
    • **스왑** — 라우팅과 슬리피지 설정
    • **RPC** — 노드 구성
    • **서비스** — 백그라운드 서비스 설정

    변경 사항은 즉시 적용됩니다 (핫 리로드).
hints-config-telegram-title = { -telegram } 알림
hints-config-telegram-content =
    { "*" }*{ -telegram }으로 거래 알림을 즉시 수신**

    거래, 포지션, 주요 이벤트 알림을 { -telegram }에서 바로 받아보세요.

    { "*" }*설정 단계:**

    1. **봇 만들기:**
       • { -telegram }을 열고 @BotFather에게 메시지 보내기
       • /newbot을 보내고 안내에 따르기
       • 봇 토큰 복사 (예: 123456:ABC-DEF...)

    2. **채팅 ID 확인:**
       • @userinfobot 또는 @getidsbot에게 메시지 보내기
       • 반환된 숫자 ID 복사

    3. **{ -brand }에서 설정:**
       • 알림 토글 켜기
       • 봇 토큰과 채팅 ID 붙여넣기
       • "연결 테스트"를 클릭하여 확인

    { "*" }*수신하는 알림:**
    • 거래 실행 확인
    • 포지션 업데이트 (진입/청산)
    • 트레이드 워처 알림
    • 오류 알림

    { "*" }*개인정보 보호:**
    메시지는 서드파티 서버를 거치지 않고 { -brand }에서 { -telegram } 봇으로 직접 전송됩니다.
hints-config-telegram-password-title = 봇 인증 비밀번호
hints-config-telegram-password-content =
    { "*" }*비밀번호 인증으로 { -telegram } 봇 보호**

    { -brand } { -telegram } 봇을 사용할 때 민감한 명령을 실행하기 전에 이 비밀번호로 인증해야 합니다.

    { "*" }*비밀번호를 설정하는 이유:**
    • 권한 없는 사용자가 봇을 제어하는 것을 방지
    • { -telegram }을 통한 거래 명령 실행에 필요
    • 최소 8자 이상이어야 함

    { "*" }*작동 방식:**
    1. 대시보드에서 비밀번호 설정
    2. 봇에 거래 명령을 보내면 인증을 요청
    3. 비밀번호를 입력하여 본인 확인
    4. 추가 보안을 위해 2FA를 선택적으로 활성화

    { "*" }*참고:** 비밀번호는 안전한 SHA256 해시로 저장되며 평문은 저장하지 않습니다.
hints-config-telegram-totp-title = 2단계 인증 (2FA)
hints-config-telegram-totp-content =
    { "*" }*TOTP 2FA로 보안 강화**

    2단계 인증은 Google Authenticator, Authy, 1Password 같은 앱의 시간 기반 일회용 비밀번호(TOTP)를 사용합니다.

    { "*" }*2FA를 활성화하는 이유:**
    • 비밀번호가 유출되어도 코드 없이는 봇에 접근할 수 없음
    • 6자리 코드가 30초마다 바뀜
    • 설정 후에는 오프라인에서도 작동

    { "*" }*설정 과정:**
    1. "2FA 활성화"를 클릭하고 비밀번호 입력
    2. 인증 앱으로 QR 코드 스캔
    3. 6자리 코드를 입력하여 설정 확인

    { "*" }*호환 앱:**
    • Google Authenticator
    • Authy
    • 1Password
    • Microsoft Authenticator
    • TOTP를 지원하는 모든 앱

    { "*" }*중요:** 비밀 키를 안전한 곳에 보관하세요. 인증 앱에 접근할 수 없게 되면 이 대시보드에서 2FA를 비활성화해야 합니다.

## token_details

hints-token-details-chart-title = 가격 차트 (OHLCV)
hints-token-details-chart-content =
    { "*" }*중요:** 이 차트는 실시간 체결 가격이 *아니라* 전략 평가용 **캐시된 OHLCV 데이터**를 표시합니다.

    { "*" }*캐시 데이터를 쓰는 이유?**
    • **용도:** 자동 전략과 지표(예: RSI, MA)에 사용됩니다.
    • **최신성:** 업데이트는 토큰 우선순위에 따라 달라집니다 (보유 포지션 = 더 빠른 업데이트).
    • **출처:** 온체인 RPC 직접 조회가 아닌 { -dexscreener }/{ -geckoterminal }에서 집계됩니다.

    { "*" }*DEX 가격의 현실:**
    DeFi에서 토큰은 **여러 풀**(Raydium, Orca, Meteora)에서 거래됩니다. 각 풀은 유동성 깊이와 최근 거래에 따라 고유한 가격을 가집니다.
    • **차트 가격:** 여러 시장의 평균/집계 값.
    • **스왑 가격:** 거래 시점에 최적 경로에서 실제로 적용되는 환율.

    { "*" }이 차트와 최종 체결 가격 사이에는 작은 차이가 있을 수 있습니다.*

    { "*" }*상태:** "데이터 대기 중"은 백그라운드 워커가 새 캔들을 가져오는 중이라는 뜻입니다.
hints-token-details-token-info-title = 토큰 정보
hints-token-details-token-info-content =
    온체인 및 시장 소스의 기본 토큰 메타데이터입니다.

        • **민트** — Solana에서 토큰의 고유 주소 (클릭하여 복사)
        • **소수 자릿수** — 토큰 정밀도 (보통 6-9)
        • **경과 기간** — 주요 풀/토큰이 생성된 이후 시간
        • **DEX** — 이 토큰의 주요 거래소
        • **홀더** — 토큰을 보유한 고유 지갑 수
        • **상위 10 보유** — 상위 10개 지갑이 보유한 비율(%)

        홀더 수가 많고 집중도가 낮을수록 일반적으로 분포가 건전합니다.
hints-token-details-liquidity-title = 유동성 및 시장 데이터
hints-token-details-liquidity-content =
    유동성이 가장 높은 SOL 풀의 시장 지표입니다.

        • **FDV** — 가격 × 총 공급량 (애그리게이터 가격)
        • **유동성** — 풀 리저브의 USD 가치
        • **풀 SOL** / **풀 토큰** — 풀 가격을 결정하는 실시간 리저브

        { "*" }*중요한 이유:**
        • 유동성이 깊을수록 슬리피지가 낮음
        • 얕은 풀은 작은 거래에도 가격이 움직일 수 있음
        • 풀 리저브가 스왑 체결 가격을 직접 결정

        데이터는 { -dexscreener }/{ -geckoterminal } 및 온체인 풀 조회를 통해 주기적으로 새로 고쳐집니다.
hints-token-details-market-pulse-title = 시장 동향
hints-token-details-market-pulse-content =
    가격 변동과 USD 거래량은 동일한 **5M / 1H / 6H / 24H** 타임라인을 사용하므로 모멘텀과 참여도를 직접 비교할 수 있습니다.

    { "*" }*해석:**
    • **가격** — 애그리게이터 기반 변동률이며 실시간 풀 체결 가격이 아닙니다.
    • **높은 거래량** — 관심 증가, 효율적인 가격 발견, 쉬운 청산.
    • **낮은 거래량** — 슬리피지 증가, 스프레드 확대, 대량 청산 어려움.
    • **높은 거래량 + 낮은 유동성** — 변동성과 체결 위험 증가.

    시장 데이터는 { -dexscreener }/{ -geckoterminal }을 통해 주요 DEX 전반에서 집계되므로 가격 변동이 현재 온체인 풀 가격과 다를 수 있습니다.
hints-token-details-activity-title = 트랜잭션 활동 (건수)
hints-token-details-activity-content =
    여러 타임프레임에 걸친 **거래 건수**(매수 대 매도)를 분석합니다. 거래 규모와 관계없이 트레이더의 의도를 보여줍니다.

    { "*" }*지표 상세:**
    • **타임프레임:** 5M, 1H, 6H, 24H 구간.
    • **막대:** 매수 건수(녹색)와 매도 건수(빨간색)의 시각적 비율.
    • **속도:** 분당 거래 수 (예: "12.5/m"). 속도가 높을수록 바이럴 활동.
    • **건수:** 매수/매도의 정확한 건수와 비율.

    { "*" }*요약 지표:**
    • **24H 매수 %:** >50%는 강세(매수자 우세), { "<" }50%는 약세(매도자 우세).
    • **순유입:** 총 매수에서 매도를 뺀 값. 양수 = 매집.
    • **5M 급증:** 1H 평균과 비교해 *지금* 거래가 얼마나 빠른지.
      • **>1.0x:** 관심 가속.
      • **>3.0x:** 바이럴 돌파 또는 패닉.
      • **{ "<" }1.0x:** 진정 국면.

    { "*" }*전략 팁:** "매수 %"와 "급증 배수"가 모두 높으면 강한 돌파 진입 신호인 경우가 많습니다.
hints-token-details-security-title = 보안 분석
hints-token-details-security-content =
    { -rugcheck }.xyz와 온체인 분석의 위험 평가입니다.

    { "*" }*안전 점수 (0-100):**
    점수가 높을수록 더 안전한 토큰입니다. 반영 요소:
    • 권한 설정 (민트/동결)
    • 홀더 집중도
    • LP 잠금 상태
    • 알려진 위험 패턴

    { "*" }*주요 위험 지표:**
    • **민트 권한** — 새 토큰 발행 가능 (인플레이션 위험)
    • **동결 권한** — 토큰 계정 동결 가능
    • **상위 홀더 %** — 집중 위험
    • **LP 제공자** — 유동성 제공자 수

    상당한 금액을 거래하기 전에 항상 보안을 확인하세요.
hints-token-details-pools-title = 유동성 풀
hints-token-details-pools-content =
    이 토큰에 대해 발견된 모든 유동성 풀입니다.

    { "*" }*여러 풀이 중요한 이유:**
    • 풀마다 유동성과 가격이 다름
    • 스왑 라우터가 풀 전반에서 최적 경로를 탐색
    • 풀 간 가격이 1-5% 차이 날 수 있음

    { "*" }*풀 정보:**
    • **DEX** — 풀을 호스팅하는 거래소
    • **유동성** — 풀 리저브의 USD 가치
    • **거래량** — 최근 거래 활동
    • **가격** — 현재 풀 가격

    풀 서비스는 유동성이 가장 높은 SOL 페어에서 가격을 계산합니다.

## ui

hints-ui-featured-title = 추천
hints-ui-featured-content =
    부스트된 토큰이 먼저 표시되고, 이어서 Jupiter와 { -dexscreener }의 트렌딩 프로젝트가 표시됩니다.

    { "*" }*표시 내용:**
    • 부스트된 토큰 — 팀이 홍보 비용을 지불한 토큰 — 맨 앞에 고정되고 금색으로 표시
    • 그 뒤에 디스커버리 보드의 트렌딩 토큰
    • 토큰을 클릭하면 전체 상세 정보 열기

    { "*" }*토큰 부스트:**
    부스트는 노출을 구매하는 것이며 추천이 아닙니다. 부스트된 행은 토큰 테이블을 포함해
    표시되는 모든 곳에서 금색으로 표시되므로 항상 구분할 수 있습니다. 토큰은
    { "*" }*screenerbot.io/boost**에서 부스트하세요.

    { "*" }*행 비활성화:**
    숨기려면 **설정 → 인터페이스 → 추천 행 표시**를 사용하세요. 헤더 동작으로는
    전체 추천 보기를 계속 열 수 있습니다.

## Hint popover chrome (ui/hint_popover.js)

hints-trigger =
    .aria-label = 도움말: { $title }
hints-popover-close =
    .aria-label = 닫기
hints-popover-learn-more = 자세히 알아보기
hints-popover-dismiss = 다시 표시하지 않음
