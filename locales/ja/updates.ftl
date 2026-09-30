# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = 自動インストールは無効です。アップデートの準備はできており、操作したときに適用されます。
updates-defer-trading-active = ポジション、取引、またはツールの操作が実行中のため、再起動を延期しています。アプリがアイドル状態になると、アップデートは自動的に適用されます。
updates-defer-needs-installer = このリリースではデスクトップシェルも更新されるため、インストーラーを一度実行する必要があります。

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }

## Progress and outcome of update actions

updates-download-started = アップデート v{ $version } をダウンロード中...
updates-apply-started = アップデートをインストールしています。{ -brand } は自動的に再起動して再接続します。
updates-install-opened = 検証済みのアップデートインストーラーを開きました。OS のインストーラーを完了してください。

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = インストーラーを開きました
updates-installer-toast-message = { -brand } はまもなく正常に終了します。

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = ステータス
updates-tab-release-notes = リリースノート
updates-tab-preferences = 環境設定
updates-tab-sections = アップデートのセクション
updates-checking-installation = このインストールを確認中...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = アップデートを確認できます
updates-phase-idle-detail = { -brand } v{ $version } がインストールされています。
updates-phase-up-to-date-headline = 最新の状態です
updates-phase-up-to-date-detail = { -brand } v{ $version } は最新バージョンです。
updates-phase-checking-headline = アップデートを確認中
updates-phase-checking-detail = 公開されている最新のリリースを確認しています。
updates-phase-available-headline = バージョン { $version } が利用可能です
updates-phase-downloading-headline = v{ $version } をダウンロード中
updates-phase-verifying-headline = v{ $version } を検証中
updates-phase-verifying-detail = ダウンロードしたファイルを、公開されているチェックサムと照合しています。
updates-phase-ready-to-apply-headline = バージョン { $version } の準備ができました
updates-phase-ready-to-apply-detail = 短い再起動で今すぐインストールするか、次回の起動時に自動でインストールできます。
updates-phase-ready-to-install-headline = バージョン { $version } の準備ができました
updates-phase-ready-to-install-detail = デスクトップインストーラーでこのアップデートを完了できます。
updates-phase-applying-headline = アップデートをインストール中
updates-phase-applying-detail = { -brand } は新しいバージョンで再起動しています。
updates-phase-applied-headline = v{ $version } に更新しました
updates-phase-applied-detail = アップデートをインストールしました。これ以上の操作は必要ありません。
updates-phase-failed-headline = アップデートが完了しませんでした
updates-phase-failed-detail = もう一度アップデートをお試しください。
updates-phase-check-failed-headline = アップデートを確認できませんでした
updates-phase-check-failed-detail = リリースサービスに接続できませんでした。
updates-status-unavailable-headline = アップデートの状態を取得できません
updates-phase-unrecognized-detail = 報告されたアップデートの状態を認識できません。
updates-status-load-failed-detail = インストールの状態を読み込めませんでした。

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = コアのアップデート · { $size } · 短い再起動
updates-kind-full = デスクトップのアップデート · { $size } · インストーラーが必要
updates-size-unknown = サイズ不明

updates-action-check-now = 今すぐ確認
updates-action-check-again = もう一度確認
updates-action-try-again = 再試行
updates-action-download = アップデートをダウンロード
updates-action-restart = 再起動して更新
updates-action-open-installer = インストーラーを開く

updates-busy-checking = 確認中...
updates-busy-resuming = ダウンロードを再開中...
updates-busy-starting-download = ダウンロードを開始中...
updates-busy-restarting = 再起動中...
updates-busy-opening-installer = インストーラーを開いています...

updates-progress-downloading = アップデートをダウンロード中
updates-progress-verifying = アップデートを検証中
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } / { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }、{ $percent }、{ $transferred }

updates-detail-list-label = インストールの詳細
updates-detail-installed-version = インストール済みのバージョン
updates-detail-system = システム
updates-detail-last-checked = 最終確認
updates-detail-never = なし
updates-detail-available-version = 利用可能なバージョン
updates-detail-download-size = ダウンロードサイズ

updates-version-installed = インストール済み
updates-version-available = 利用可能

updates-notes-highlights = ハイライト
updates-notes-empty-title = リリースノートはまだありません
updates-notes-empty-error = リリース履歴を読み込めませんでした。接続を確認して、もう一度お試しください。
updates-notes-empty-none = リリースが公開されると、リリースノートがここに表示されます。
updates-notes-history-notice = このインストールがすでに把握している内容を表示しています。リリース履歴は読み込めませんでした。
updates-release-empty = このリリースに記載された変更はありません。
updates-release-changes =
    { $count ->
       *[other] 変更 { $count }件
    }

updates-preferences-unavailable-title = アップデートの環境設定を利用できません
updates-preferences-unavailable-detail = アップデートの設定を読み込めませんでした。
updates-preference-fallback-name = アップデートの環境設定
updates-preference-save-failed = { $preference } を保存できませんでした

updates-request-failed = リクエストに失敗しました
updates-check-request-failed = アップデートを確認できませんでした
updates-resume-failed = アップデートのダウンロードを再開できませんでした
updates-download-failed = アップデートのダウンロードを開始できませんでした
updates-apply-failed = アップデートをインストールできませんでした
updates-install-failed = アップデートインストーラーを開けませんでした
updates-apply-confirm-title = v{ $version } をインストール
updates-apply-confirm-message = { -brand } は新しいバージョンで再起動します。取引は数秒間停止し、自動的に再開します。オープンポジションには影響しません。
updates-install-confirm-title = インストーラーを実行
updates-install-confirm-message = 検証済みのインストーラーが開き、{ -brand } は正常に終了します。インストーラーを完了してから、{ -brand } をもう一度開いてください。

# A release version as displayed.
updates-version-number = v{ $version }
