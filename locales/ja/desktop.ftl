# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = OK

## Splash and loading status.

desktop-splash-starting = { -brand } を起動中
desktop-splash-restarting = { -brand } を再起動中
desktop-splash-recovering = 復旧中
desktop-splash-opening-dashboard = ダッシュボードを開いています
desktop-splash-checking-dependencies = 依存関係を確認中
desktop-splash-installing-dependencies = システムの依存関係をインストール中
desktop-splash-installing-dependencies-detail = { -brand } の実行には Microsoft Visual C++ 再頒布可能パッケージが必要です。
desktop-splash-resetting-wallet = ウォレットデータをリセット中
desktop-splash-resetting-wallet-detail = 既存のウォレットデータは、消去する前にバックアップされます。
desktop-splash-updating = v{ $version } に更新中
desktop-splash-updating-detail = 設定とデータはそのまま保持されます。
desktop-splash-restoring = v{ $version } に復元中
desktop-splash-restoring-detail = アップデート v{ $failed } が起動しなかったため、以前のバージョンに切り替えます。

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand } を起動できませんでした
desktop-boot-detail-fallback = バックエンドが予期せず停止しました。
desktop-boot-remedy-label = 対処方法
desktop-boot-log-file-label = ログファイル:
desktop-boot-action-reset-wallet = ウォレットデータをリセットして再起動
desktop-boot-action-working = 処理中...
desktop-boot-action-open-logs = ログフォルダーを開く
desktop-boot-action-copy = 詳細をコピー
desktop-boot-action-copied = コピーしました
desktop-boot-action-quit = 終了
desktop-boot-subtitle-wallet-mismatch = 別のウォレットが検出されました
desktop-boot-subtitle-port-in-use = 必要なネットワークポートが使用中です
desktop-boot-subtitle-lock-held = { -brand } はすでに実行中です
desktop-boot-subtitle-config-invalid = 設定の問題
desktop-boot-subtitle-directory-setup = ストレージの問題
desktop-boot-subtitle-storage-upgrade = データベースのアップグレードの問題
desktop-boot-subtitle-generic = 起動エラー

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand } を起動できませんでした
desktop-boot-error-remedy = ログフォルダーを開いて原因を確認し、アプリを再起動してください。問題が解決しない場合は、t.me/screenerbotio_support のサポートまでご連絡ください。
desktop-boot-error-default = ダッシュボードの準備が整う前に、バックエンドが予期せず停止しました。
desktop-boot-error-restore-failed = 更新後のバックエンドが失敗し、以前のバージョンを復元できませんでした（{ $error }）。
desktop-boot-error-spawn-failed = バックエンドプログラムを起動できませんでした（{ $error }）。
desktop-boot-error-spawn-missing = バックエンドプログラムを起動できませんでした。ファイルが見つからないか、セキュリティソフトにブロックされている可能性があります。
desktop-boot-error-exited-running = ダッシュボードの実行中にバックエンドが停止しました（終了コード { $code }）。
desktop-boot-error-exited-early = ダッシュボードの準備が整う前にバックエンドが停止しました（終了コード { $code }）。
desktop-boot-error-dashboard-load = ダッシュボードを読み込めませんでした（{ $description }、{ $code }）。
desktop-boot-error-renderer-gone = ダッシュボードのレンダラーが停止しました（{ $reason }）。
desktop-boot-error-unresponsive = ダッシュボードが応答しなくなりました。
desktop-boot-error-url-failed = ダッシュボードの URL を読み込めませんでした（{ $error }）。
desktop-boot-error-relaunch-setup = セットアップ後にバックエンドを再起動できませんでした。
desktop-boot-error-relaunch-recovery = 復旧のためにバックエンドを再起動できませんでした。
desktop-boot-error-restart-offline = 再起動後にバックエンドがオンラインに戻りませんでした。
desktop-boot-error-recovery-offline = 復旧は完了しましたが、バックエンドの準備が整いませんでした。
desktop-boot-error-start-timeout = バックエンドの起動が時間内に完了しませんでした。初回起動が遅い場合や、他のプログラムが接続をブロックしている場合に発生することがあります。

## System tray.

desktop-tray-tooltip = { -brand } - Solana トレーディングボット
desktop-tray-show = { -brand } を表示
desktop-tray-open-dashboard = ダッシュボードを開く
desktop-tray-quit = { -brand } を終了

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = データフォルダーを開く
desktop-menu-open-logs-folder = ログフォルダーを開く
desktop-menu-documentation = ドキュメント
desktop-menu-telegram-support = { -telegram } サポート
desktop-menu-check-updates = アップデートを確認...

## Application menu.

desktop-menu-file = ファイル
desktop-menu-edit = 編集
desktop-menu-view = 表示
desktop-menu-window = ウィンドウ
desktop-menu-help = ヘルプ
desktop-menu-reset-zoom = ズームをリセット
desktop-menu-zoom-in = ズームイン
desktop-menu-zoom-out = ズームアウト
desktop-menu-keyboard-shortcuts = キーボードショートカット
desktop-menu-telegram-channel = { -telegram } チャンネル
desktop-menu-telegram-community = { -telegram } コミュニティ
desktop-menu-follow-x = { -x }（{ -twitter }）でフォロー
desktop-menu-visit-website = ウェブサイトを開く
desktop-menu-about = { -brand } について

## About dialog.

desktop-about-title = { -brand } について
desktop-about-message = { -brand }
desktop-about-detail =
    バージョン { $version }

    高度な Solana ウォレット管理・自動売買ボット。

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = キーボードショートカット
desktop-shortcuts-message = { -brand } のキーボードショートカット
desktop-shortcuts-body-mac =
    キーボードショートカット:

    ウィンドウ操作:
      Cmd+M          最小化
      Cmd+W          ウィンドウを閉じる
      Cmd+Q          終了
      Cmd+Ctrl+F     フルスクリーンを切り替え

    ズーム:
      Cmd++          ズームイン
      Cmd+-          ズームアウト
      Cmd+0          ズームをリセット

    ナビゲーション:
      Cmd+R          ダッシュボードを再読み込み
      Cmd+Shift+D    データフォルダーを開く

    その他:
      F1             ドキュメントを開く
      Cmd+Alt+I      DevTools を切り替え
desktop-shortcuts-body-other =
    キーボードショートカット:

    ウィンドウ操作:
      Alt+F4         終了
      F11            フルスクリーンを切り替え

    ズーム:
      Ctrl++         ズームイン
      Ctrl+-         ズームアウト
      Ctrl+0         ズームをリセット

    ナビゲーション:
      Ctrl+R         ダッシュボードを再読み込み
      Ctrl+Shift+D   データフォルダーを開く

    その他:
      F1             ドキュメントを開く
      Ctrl+Shift+I   DevTools を切り替え

## Close confirmation (Windows and Linux).

desktop-close-title = { -brand } を閉じる
desktop-close-message = どうしますか？
desktop-close-detail = { -brand } はバックグラウンドで実行を続けられます。システムトレイに最小化している間も、トレーディングボットは監視と取引を続けます。
desktop-close-minimize = トレイに最小化
desktop-close-quit = 完全に終了
desktop-close-cancel = キャンセル

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = 依存関係が不足しています
desktop-vcredist-missing-message = Visual C++ 再頒布可能パッケージがありません
desktop-vcredist-missing-detail = { -brand } の実行には Microsoft Visual C++ 再頒布可能パッケージが必要です。今すぐインストールしますか？
desktop-vcredist-install = インストールして修復
desktop-vcredist-exit = 終了
desktop-vcredist-not-found-title = インストーラーが見つかりません
desktop-vcredist-not-found-message = { $name } を正しく見つけられませんでした。
desktop-vcredist-done-title = インストール完了
desktop-vcredist-done-message = 依存関係をインストールしました。
desktop-vcredist-done-detail = { -brand } を起動します。
desktop-vcredist-failed-title = インストール失敗
desktop-vcredist-failed-message = Visual C++ 再頒布可能パッケージを手動でインストールしてください。
