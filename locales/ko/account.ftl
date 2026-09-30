# Account and ScreenerBot data access status. Each message is the headline;
# `.detail` explains what happens instead. Ids come from DataAccess in
# src/data_server/access.rs.

account-data-access-ready = { -brand } 데이터가 활성화되어 있습니다
    .detail = 공유 캔들, 풀 레지스트리, 보안 리포트, 토큰 식별 정보를 screenerbot.io에서 제공받고 있습니다.

account-data-access-disabled = { -brand } 데이터가 꺼져 있습니다
    .detail = 설정에서 { -brand } 소스가 꺼져 있어 공개 제공자의 데이터만 사용합니다.

account-data-access-offline = { -brand } 데이터가 오프라인입니다
    .detail = 네트워크에 연결되어 있지 않습니다. 연결이 복구되면 데이터가 자동으로 재개됩니다.

account-data-access-signed-out = { -brand } 데이터를 사용하려면 계정이 필요합니다
    .detail = 차트, 풀, 보안 리포트, 토큰 식별 정보는 대신 공개 제공자에서 가져옵니다. 속도가 느리고, 요청 제한이 있으며, 과거 데이터도 적습니다. 로그인은 무료이며 { -brand } 실행 방식에는 다른 영향이 없습니다.

account-data-access-reauthorization-required = { -brand } 데이터를 사용하려면 다시 로그인해야 합니다
    .detail = 이 기기는 { -brand } 데이터가 도입되기 전에 승인되었습니다. 다시 로그인하면 복구됩니다. 그때까지는 공개 제공자를 사용합니다.

account-data-access-version-unsupported = { -brand } 데이터를 사용하려면 최신 버전이 필요합니다
    .detail = 이 버전은 더 이상 지원되지 않습니다. { -brand } 데이터를 다시 사용하려면 { $minimum } 이상으로 업데이트하세요. 그때까지는 공개 제공자를 사용합니다.

account-data-access-unreachable = { -brand } 데이터가 응답하지 않습니다
    .detail = 서비스가 응답하지 않았습니다. 공개 제공자를 사용하며 계속 재시도합니다.

account-data-access-unknown = { -brand } 데이터를 아직 확인하지 않았습니다
    .detail = 이번 세션에서는 아직 공유 데이터가 필요하지 않았습니다.

## Account panel (ui/account/panel.js), shared by Setup and Settings

# Features a signed-in account carries.
account-scope-data-read = { -brand } 시장 데이터
account-scope-rpc-submit = 서명된 트랜잭션 무료 제출
account-scope-vote = 토큰 투표
account-scope-referral-read = 추천 수익
account-scope-account-read = 계정 세부 정보

account-panel-request-failed = 작업이 실패했습니다. 다시 시도하세요.
account-panel-checking = 계정 상태를 확인하는 중…
account-panel-status-unavailable = 계정 상태를 사용할 수 없습니다.
account-panel-browser-notice = 브라우저에서 로그인을 완료한 후 이곳으로 돌아오세요. 이 패널이 업데이트됩니다.
account-panel-browser-timeout = 브라우저 로그인이 완료되지 않았습니다. 다시 시작할 수 있습니다.
account-panel-unavailable = 지금은 계정 기능을 사용할 수 없습니다. 로그인하지 않고 설정을 계속하세요.
account-panel-retry-status = 계정 상태 다시 시도
account-panel-signed-in-fallback = 로그인됨
account-panel-features =
    .aria-label = 계정 기능
account-panel-sign-out = 로그아웃
account-panel-signing-out = 로그아웃하는 중…
account-panel-sign-in = 로그인
account-panel-signing-in = 로그인하는 중…
account-panel-sign-in-wallet = 지갑으로 로그인
account-panel-opening-browser = 브라우저를 여는 중…
account-panel-continue-browser = 브라우저에서 계속
account-panel-sign-in-email = 이메일로 로그인
account-panel-new-to = { -brand } 사용이 처음이신가요?
account-panel-create-account = 계정 만들기
account-panel-unlocks-title = 계정에 포함된 기능
account-panel-back-to-options = 로그인 옵션으로 돌아가기
account-panel-email-label = 이메일
account-panel-email-input =
    .placeholder = you@example.com
account-panel-password-label = 비밀번호
account-panel-password-input =
    .placeholder = 비밀번호 입력
account-panel-need-account = 계정이 필요하거나 비밀번호를 잊으셨나요?
account-panel-open-website = screenerbot.io 열기
