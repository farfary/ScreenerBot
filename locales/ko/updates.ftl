# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = 자동 설치가 꺼져 있습니다. 업데이트가 준비되어 있으며 원할 때 적용할 수 있습니다.
updates-defer-trading-active = 포지션, 거래 또는 도구 작업이 진행 중이어서 재시작이 보류되었습니다. 앱이 유휴 상태가 되면 업데이트가 자동으로 적용됩니다.
updates-defer-needs-installer = 이 릴리스는 데스크톱 셸도 업데이트하므로 설치 프로그램을 한 번 실행해야 합니다.

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

## Progress and outcome of update actions

updates-download-started = 업데이트 v{ $version } 다운로드 중...
updates-apply-started = 업데이트를 설치하는 중입니다. { -brand }이 자동으로 재시작되고 다시 연결됩니다.
updates-install-opened = 검증된 업데이트 설치 프로그램을 열었습니다. 운영체제 설치 과정을 완료하세요.

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = 설치 프로그램 열림
updates-installer-toast-message = 이제 { -brand }이 정상적으로 종료됩니다.

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = 상태
updates-tab-release-notes = 릴리스 노트
updates-tab-preferences = 환경설정
updates-tab-sections = 업데이트 섹션
updates-checking-installation = 설치 정보 확인 중...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = 업데이트 확인 준비됨
updates-phase-idle-detail = { -brand } v{ $version }이 설치되어 있습니다.
updates-phase-up-to-date-headline = 최신 버전입니다
updates-phase-up-to-date-detail = { -brand } v{ $version }이 최신 버전입니다.
updates-phase-checking-headline = 업데이트 확인 중
updates-phase-checking-detail = 게시된 최신 릴리스를 확인하는 중입니다.
updates-phase-available-headline = 버전 { $version } 사용 가능
updates-phase-downloading-headline = v{ $version } 다운로드 중
updates-phase-verifying-headline = v{ $version } 검증 중
updates-phase-verifying-detail = 게시된 체크섬과 다운로드 파일을 대조하는 중입니다.
updates-phase-ready-to-apply-headline = 버전 { $version } 준비됨
updates-phase-ready-to-apply-detail = 짧은 재시작으로 지금 설치하거나 다음 시작 시 자동으로 설치할 수 있습니다.
updates-phase-ready-to-install-headline = 버전 { $version } 준비됨
updates-phase-ready-to-install-detail = 데스크톱 설치 프로그램으로 이 업데이트를 완료할 수 있습니다.
updates-phase-applying-headline = 업데이트 설치 중
updates-phase-applying-detail = { -brand }이 새 버전으로 재시작하는 중입니다.
updates-phase-applied-headline = v{ $version } 버전으로 업데이트됨
updates-phase-applied-detail = 업데이트가 설치되었습니다. 추가 작업은 필요하지 않습니다.
updates-phase-failed-headline = 업데이트가 완료되지 않았습니다
updates-phase-failed-detail = 업데이트를 다시 시도하세요.
updates-phase-check-failed-headline = 업데이트를 확인하지 못했습니다
updates-phase-check-failed-detail = 릴리스 서비스에 연결할 수 없습니다.
updates-status-unavailable-headline = 업데이트 상태를 사용할 수 없습니다
updates-phase-unrecognized-detail = 보고된 업데이트 상태를 인식할 수 없습니다.
updates-status-load-failed-detail = 설치 상태를 불러오지 못했습니다.

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = 코어 업데이트 · { $size } · 짧은 재시작
updates-kind-full = 데스크톱 업데이트 · { $size } · 설치 프로그램 필요
updates-size-unknown = 크기 알 수 없음

updates-action-check-now = 지금 확인
updates-action-check-again = 다시 확인
updates-action-try-again = 다시 시도
updates-action-download = 업데이트 다운로드
updates-action-restart = 재시작하여 업데이트
updates-action-open-installer = 설치 프로그램 열기

updates-busy-checking = 확인 중...
updates-busy-resuming = 다운로드 재개 중...
updates-busy-starting-download = 다운로드 시작 중...
updates-busy-restarting = 재시작 중...
updates-busy-opening-installer = 설치 프로그램 여는 중...

updates-progress-downloading = 업데이트 다운로드 중
updates-progress-verifying = 업데이트 검증 중
# $done and $total are formatted sizes.
updates-progress-transferred = { $total } 중 { $done }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }, { $percent }, { $transferred }

updates-detail-list-label = 설치 상세 정보
updates-detail-installed-version = 설치된 버전
updates-detail-system = 시스템
updates-detail-last-checked = 마지막 확인
updates-detail-never = 없음
updates-detail-available-version = 사용 가능한 버전
updates-detail-download-size = 다운로드 크기

updates-version-installed = 설치됨
updates-version-available = 사용 가능

updates-notes-highlights = 주요 내용
updates-notes-empty-title = 아직 릴리스 노트가 없습니다
updates-notes-empty-error = 릴리스 내역을 불러오지 못했습니다. 연결을 확인한 후 다시 시도하세요.
updates-notes-empty-none = 릴리스가 게시되면 릴리스 노트가 여기에 표시됩니다.
updates-notes-history-notice = 이 설치본이 이미 알고 있는 내용을 표시합니다. 릴리스 내역을 불러오지 못했습니다.
updates-release-empty = 이 릴리스에 기재된 변경 사항이 없습니다.
updates-release-changes =
    { $count ->
       *[other] 변경 { $count }건
    }

updates-preferences-unavailable-title = 업데이트 환경설정을 사용할 수 없습니다
updates-preferences-unavailable-detail = 업데이트 설정을 불러오지 못했습니다.
updates-preference-fallback-name = 업데이트 환경설정
updates-preference-save-failed = 저장하지 못했습니다: { $preference }

updates-request-failed = 요청 실패
updates-check-request-failed = 업데이트를 확인하지 못했습니다
updates-resume-failed = 업데이트 다운로드를 재개하지 못했습니다
updates-download-failed = 업데이트 다운로드를 시작하지 못했습니다
updates-apply-failed = 업데이트를 설치하지 못했습니다
updates-install-failed = 업데이트 설치 프로그램을 열지 못했습니다
updates-apply-confirm-title = v{ $version } 설치
updates-apply-confirm-message = { -brand }이 새 버전으로 재시작됩니다. 거래가 몇 초간 중지되었다가 자동으로 재개되며 보유 중인 포지션은 영향을 받지 않습니다.
updates-install-confirm-title = 설치 프로그램 실행
updates-install-confirm-message = 검증된 설치 프로그램이 열리고 { -brand }이 정상적으로 종료됩니다. 설치를 완료한 후 { -brand }을 다시 여세요.

# A release version as displayed.
updates-version-number = v{ $version }
