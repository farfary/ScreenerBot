# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = 확인

## Splash and loading status.

desktop-splash-starting = { -brand } 시작 중
desktop-splash-restarting = { -brand } 재시작 중
desktop-splash-recovering = 복구 중
desktop-splash-opening-dashboard = 대시보드 여는 중
desktop-splash-checking-dependencies = 종속성 확인 중
desktop-splash-installing-dependencies = 시스템 종속성 설치 중
desktop-splash-installing-dependencies-detail = { -brand }을 실행하려면 Microsoft Visual C++ Redistributable이 필요합니다.
desktop-splash-resetting-wallet = 지갑 데이터 초기화 중
desktop-splash-resetting-wallet-detail = 기존 지갑 데이터는 삭제하기 전에 백업됩니다.
desktop-splash-updating = v{ $version } 버전으로 업데이트 중
desktop-splash-updating-detail = 설정과 데이터는 그대로 유지됩니다.
desktop-splash-restoring = v{ $version } 복원 중
desktop-splash-restoring-detail = 업데이트 v{ $failed }이 시작되지 않아 이전 버전으로 복원합니다.

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand }을 시작하지 못했습니다
desktop-boot-detail-fallback = 백엔드가 예기치 않게 중지되었습니다.
desktop-boot-remedy-label = 해결 방법
desktop-boot-log-file-label = 로그 파일:
desktop-boot-action-reset-wallet = 지갑 데이터 초기화 후 재시작
desktop-boot-action-working = 처리 중...
desktop-boot-action-open-logs = 로그 폴더 열기
desktop-boot-action-copy = 상세 정보 복사
desktop-boot-action-copied = 복사됨
desktop-boot-action-quit = 종료
desktop-boot-subtitle-wallet-mismatch = 다른 지갑이 감지되었습니다
desktop-boot-subtitle-port-in-use = 필요한 네트워크 포트가 사용 중입니다
desktop-boot-subtitle-lock-held = { -brand }이 이미 실행 중입니다
desktop-boot-subtitle-config-invalid = 설정 문제
desktop-boot-subtitle-directory-setup = 저장소 문제
desktop-boot-subtitle-generic = 시작 오류

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand }을 시작하지 못했습니다
desktop-boot-error-remedy = 로그 폴더를 열어 원인을 확인한 후 앱을 다시 시작하세요. 문제가 계속되면 t.me/screenerbotio_support 로 문의하세요.
desktop-boot-error-default = 대시보드가 준비되기 전에 백엔드가 예기치 않게 중지되었습니다.
desktop-boot-error-restore-failed = 업데이트된 백엔드가 실패했으며 이전 버전을 복원하지 못했습니다 ({ $error }).
desktop-boot-error-spawn-failed = 백엔드 프로그램을 시작하지 못했습니다 ({ $error }).
desktop-boot-error-spawn-missing = 백엔드 프로그램을 시작하지 못했습니다. 파일이 없거나 보안 소프트웨어가 차단했을 수 있습니다.
desktop-boot-error-exited-running = 대시보드 실행 중 백엔드가 중지되었습니다 (종료 코드 { $code }).
desktop-boot-error-exited-early = 대시보드가 준비되기 전에 백엔드가 중지되었습니다 (종료 코드 { $code }).
desktop-boot-error-dashboard-load = 대시보드를 불러오지 못했습니다 ({ $description }, { $code }).
desktop-boot-error-renderer-gone = 대시보드 렌더러가 중지되었습니다 ({ $reason }).
desktop-boot-error-unresponsive = 대시보드가 응답하지 않습니다.
desktop-boot-error-url-failed = 대시보드 URL을 불러오지 못했습니다 ({ $error }).
desktop-boot-error-relaunch-setup = 설정 후 백엔드를 다시 시작하지 못했습니다.
desktop-boot-error-relaunch-recovery = 복구를 위해 백엔드를 다시 시작하지 못했습니다.
desktop-boot-error-restart-offline = 재시작 후 백엔드가 다시 온라인 상태가 되지 않았습니다.
desktop-boot-error-recovery-offline = 복구가 끝났지만 백엔드가 준비되지 않았습니다.
desktop-boot-error-start-timeout = 백엔드가 제시간에 시작을 마치지 못했습니다. 처음 실행이 느리거나 다른 프로그램이 연결을 차단하는 경우 발생할 수 있습니다.

## System tray.

desktop-tray-tooltip = { -brand } - Solana 트레이딩 봇
desktop-tray-show = { -brand } 표시
desktop-tray-open-dashboard = 대시보드 열기
desktop-tray-quit = { -brand } 종료

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = 데이터 폴더 열기
desktop-menu-open-logs-folder = 로그 폴더 열기
desktop-menu-documentation = 문서
desktop-menu-telegram-support = { -telegram } 지원
desktop-menu-check-updates = 업데이트 확인...

## Application menu.

desktop-menu-file = 파일
desktop-menu-edit = 편집
desktop-menu-view = 보기
desktop-menu-window = 창
desktop-menu-help = 도움말
desktop-menu-reset-zoom = 확대/축소 초기화
desktop-menu-zoom-in = 확대
desktop-menu-zoom-out = 축소
desktop-menu-keyboard-shortcuts = 키보드 단축키
desktop-menu-telegram-channel = { -telegram } 채널
desktop-menu-telegram-community = { -telegram } 커뮤니티
desktop-menu-follow-x = { -x }({ -twitter })에서 팔로우
desktop-menu-visit-website = 웹사이트 방문
desktop-menu-about = { -brand } 정보

## About dialog.

desktop-about-title = { -brand } 정보
desktop-about-message = { -brand }
desktop-about-detail =
    버전 { $version }

    고급 Solana 지갑 관리 및 자동 트레이딩 봇.

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = 키보드 단축키
desktop-shortcuts-message = { -brand } 키보드 단축키
desktop-shortcuts-body-mac =
    키보드 단축키:

    창 제어:
      Cmd+M          최소화
      Cmd+W          창 닫기
      Cmd+Q          종료
      Cmd+Ctrl+F     전체 화면 전환

    확대/축소:
      Cmd++          확대
      Cmd+-          축소
      Cmd+0          확대/축소 초기화

    탐색:
      Cmd+R          대시보드 새로 고침
      Cmd+Shift+D    데이터 폴더 열기

    기타:
      F1             문서 열기
      Cmd+Alt+I      개발자 도구 전환
desktop-shortcuts-body-other =
    키보드 단축키:

    창 제어:
      Alt+F4         종료
      F11            전체 화면 전환

    확대/축소:
      Ctrl++         확대
      Ctrl+-         축소
      Ctrl+0         확대/축소 초기화

    탐색:
      Ctrl+R         대시보드 새로 고침
      Ctrl+Shift+D   데이터 폴더 열기

    기타:
      F1             문서 열기
      Ctrl+Shift+I   개발자 도구 전환

## Close confirmation (Windows and Linux).

desktop-close-title = { -brand } 종료
desktop-close-message = 어떻게 하시겠습니까?
desktop-close-detail = { -brand }은 백그라운드에서 계속 실행될 수 있습니다. 시스템 트레이로 최소화해도 트레이딩 봇은 계속 모니터링하고 거래합니다.
desktop-close-minimize = 트레이로 최소화
desktop-close-quit = 완전히 종료
desktop-close-cancel = 취소

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = 종속성 누락
desktop-vcredist-missing-message = Visual C++ Redistributable이 없습니다
desktop-vcredist-missing-detail = { -brand }을 실행하려면 Microsoft Visual C++ Redistributable이 필요합니다. 지금 설치하시겠습니까?
desktop-vcredist-install = 설치 및 수정
desktop-vcredist-exit = 종료
desktop-vcredist-not-found-title = 설치 프로그램을 찾을 수 없음
desktop-vcredist-not-found-message = { $name }을 올바르게 찾을 수 없습니다.
desktop-vcredist-done-title = 설치 완료
desktop-vcredist-done-message = 종속성이 설치되었습니다.
desktop-vcredist-done-detail = 이제 { -brand }이 시작됩니다.
desktop-vcredist-failed-title = 설치 실패
desktop-vcredist-failed-message = Visual C++ Redistributable을 직접 설치하세요.
