# Dashboard shell: header, ticker, notification drawer and status bar.

# Source: templates/base.html
# Document title: the page title, then the product name.
shell-document-title = { $page } - { -brand }
shell-brand-name = { -brand }
shell-brand-logo =
    .alt = { -brand }
# Version label; the number itself is passed as an argument.
shell-version = v{ $version }

## Header

shell-header-brand =
    .aria-label = ダッシュボードのホームを開く
    .title = ダッシュボードホーム
shell-bot-card =
    .aria-label = 自動トレーダーのステータスを読み込み中
shell-bot-label = 自動
shell-bot-status-loading = 読み込み中
shell-bot-today = 本日
shell-explore-control =
    .aria-label = Explore モード。ウォレットと RPC エンドポイントを接続すると、すべての機能を利用できます
    .title = ウォレットと RPC エンドポイントを接続すると、取引、残高、ライブのオンチェーンデータを利用できます
shell-explore-title = Explore モード
shell-explore-detail = ウォレットと RPC が未接続です
shell-explore-action = セットアップを完了
shell-wallet-card =
    .aria-label = ウォレット評価額。ポジションを開く
    .title = ウォレット評価額（{ -sol } + トークン）· ポジションを開く
shell-wallet-worth-label = 評価額
shell-wallet-sol-label = { -sol }
shell-wallet-tokens-label = トークン
shell-sol-price-card =
    .aria-label = { -sol } の USD 価格。チャートを開く
    .title = { -sol } 価格 · クリックでチャートを表示
shell-sol-price-label = { -sol }/USD
shell-sol-price-period = 24h
shell-copy-card =
    .aria-label = コピートレード。コピートレードを開く
    .title = コピートレード · コピートレードを開く
shell-copy-label = コピー
shell-actions-more =
    .aria-label = その他のヘッダー操作
    .title = その他の操作
shell-actions-group =
    .aria-label = ヘッダー操作
shell-action-search =
    .aria-label = トークンを検索
    .title = トークンを検索（Ctrl/Cmd+K）
shell-action-featured =
    .aria-label = 注目トークン
    .title = 注目トークン
shell-action-notifications =
    .aria-label = アクションと通知
    .title = アクションと通知
shell-action-restart =
    .aria-label = アプリを再起動
    .title = アプリを再起動
shell-action-theme =
    .aria-label = テーマを切り替え
    .title = テーマを切り替え
shell-action-settings =
    .aria-label = 設定
    .title = 設定

## Ticker

shell-ticker-monitoring-segment =
    .title = プールサービスが監視中のトークン
shell-ticker-monitoring = 監視中:
shell-ticker-filtering-segment =
    .title = フィルタリング条件を通過/除外されたトークン
shell-ticker-passed = 通過:
shell-ticker-rejected = 除外:
shell-ticker-pnl-segment =
    .title = 本日の確定損益
shell-ticker-pnl = 本日の損益:
shell-ticker-rpc-segment =
    .title = 1分あたりの RPC 呼び出し数と成功率
shell-ticker-rpc = RPC:
shell-ticker-rpc-per-minute = /分
shell-ticker-services-segment =
    .title = バックグラウンドサービスの稼働状況
shell-ticker-services-loading = サービス: <strong>読み込み中</strong>

## Notification drawer

shell-notification-title = アクション
shell-notification-mark-all-read =
    .title = すべて既読にする
shell-notification-clear-all =
    .title = すべてクリア
shell-notification-close =
    .aria-label = 閉じる
shell-notification-tab-all = すべて
shell-notification-tab-active = 進行中
shell-notification-tab-done = 完了
shell-notification-tab-failed = 失敗
shell-notification-filter-type-all = すべての種類
shell-notification-filter-type-buy = 購入
shell-notification-filter-type-sell = 売却
shell-notification-filter-type-open = オープン
shell-notification-filter-type-close = クローズ
shell-notification-filter-type-dca = DCA
shell-notification-filter-type-partial = 部分
shell-notification-filter-state-all = すべての状態
shell-notification-filter-state-in-progress = 進行中
shell-notification-filter-state-completed = 完了
shell-notification-filter-state-failed = 失敗
shell-notification-filter-state-cancelled = キャンセル
shell-notification-list =
    .aria-label = 通知
shell-notification-empty = アクションはまだありません
shell-notification-loading-more = さらに読み込み中...
shell-notification-back-to-top =
    .title = 先頭に戻る

## Status bar

shell-status-bar-version = v
shell-status-bar-uptime = 稼働
shell-status-bar-memory = メモリ
shell-status-bar-rpc = RPC
shell-status-rpc-per-minute = { $rate }/分
shell-status-bar-trading = 取引
shell-status-bar-positions = ポジ
shell-status-bar-tokens = トークン

# Source: templates/pages/splash.html, scripts/core/splash.js

## Splash

shell-splash-starting = { -brand } を起動中
shell-splash-waiting = ローカルコアの応答を待っています。
shell-splash-failed = { -brand } を起動できませんでした
shell-splash-failed-detail = ログファイルを確認してから、アプリを再起動してください。

# Source: scripts/core/header.js, scripts/core/connectivity_watcher.js, scripts/core/router.js

## Connection state

shell-connection-connected = コア接続済み
shell-connection-waiting = コアを待機中…
shell-connection-retry-now = 今すぐ再試行
shell-connection-overlay-detail = コアに接続できません。取引は一時停止中で、自動的に復旧します。
shell-connection-restored = コアとの接続を復旧しました

# Source: scripts/core/header.js
shell-trader-control-failed = トレーダーの操作に失敗しました
shell-notification-button-unread = アクションと通知、未読 { $count }件
shell-restart-confirm-title = ボットを再起動
shell-restart-confirm-message =
    ボットを再起動しますか？

    再起動すると次のようになります:
    • すべてのサービスを停止
    • プロセスを再起動
    • 約10～15秒かかります

    実行中の操作はすべて中断されます。
shell-restart-confirm-action = 再起動
shell-restart-progress = ボットを再起動中
shell-restart-failed = 再起動に失敗しました
shell-restart-failed-status = 再起動に失敗しました: { $status }
shell-restart-helper-unavailable = 自動再起動ヘルパーを利用できません。しばらくしてからダッシュボードを再読み込みしてください。

# Source: scripts/core/router.js
shell-page-title-fallback = ダッシュボード
shell-page-load-failed = ページを読み込めませんでした
shell-page-offline-detail = 現在コアに接続できません。接続が復旧すると、このページは自動的に読み込まれます。

# Source: scripts/core/header_metrics.js

## Auto Trader card

shell-bot-state-explore = EXPLORE
shell-bot-state-halted = 停止
shell-bot-state-off = 無効
shell-bot-state-waiting = 待機中
shell-bot-state-idle = アイドル
shell-bot-state-entry-paused = エントリー停止
shell-bot-state-running = 実行中
shell-bot-control-explore = 自動トレーダーは Explore モードでは利用できません。ウォレットと RPC のセットアップを開いてください。
shell-bot-control-halted = 緊急停止が有効です。自動トレーダーの制御を開いてください。
shell-bot-control-off = 自動トレーダーは無効です。クリックで有効にします。
shell-bot-control-waiting = 自動トレーダーは有効で、コアサービスを待機しています。クリックで無効にします。
shell-bot-control-idle = 自動トレーダーは有効ですが、両方のモニターがオフです。自動トレーダーの制御を開いてください。
shell-bot-control-entry-paused = 損失保護によりエントリーが停止中です。エグジットは継続できます。自動トレーダーの制御を開いてください。
shell-bot-control-running = 自動トレーダーは実行中です。クリックで無効にします。

## Wallet and copy cards

shell-wallet-card-summary = ウォレット評価額: { $equity } { -sol }（現金 { $balance } { -sol }、トークン { $tokens }個）。ポジションを開く
shell-copy-running-live = ライブ { $count }件
shell-copy-running-paper = ペーパー { $count }件
shell-copy-value-paused = 一時停止中
shell-copy-value-idle = アイドル
shell-copy-sub-active = { $total }件中 { $active }件が稼働中

## Ticker services state

shell-ticker-services-healthy = サービス: <strong>正常</strong>
shell-ticker-services-issues =
    { $count ->
       *[other] サービス: <strong>問題 { $count }件</strong>
    }

# Source: scripts/core/agent_approvals.js

## Agent approval prompt

shell-agent-request-title = エージェントのリクエスト
shell-agent-request-client-fallback = ペアリング済みのエージェント
shell-agent-request-message = { $client } が { -brand } で「{ $tool }」を実行しようとしています。このリクエストは{ $expiry }。
shell-agent-request-message-arguments = { $client } が { -brand } で「{ $tool }」を実行しようとしています。引数: { $summary }。このリクエストは{ $expiry }。
shell-agent-request-expires-minutes = { $minutes }分後に期限切れになります
shell-agent-request-expires-seconds = { $seconds }秒後に期限切れになります
shell-agent-request-approve = 承認
shell-agent-request-deny = 拒否

# Source: scripts/core/utils.js, scripts/core/toast.js, scripts/ui/toast.js, scripts/ui/confirmation_dialog.js

## Toasts, dialogs and shared widgets

shell-toast-copied = { $label } をコピーしました
shell-toast-copy-failed = コピーに失敗しました
shell-toast-still-running = 実行中です。通知センターを確認してください
shell-toast-dismiss =
    .aria-label = 閉じる
shell-confirm-title = 操作の確認
shell-confirm-message = よろしいですか？
shell-address-open-solscan = — { -solscan } で開く
shell-address-copy = アドレスをコピー

# Source: scripts/core/global_chat.js
shell-assistant-label = アシスタント
shell-assistant-dialog =
    .aria-label = アシスタント

# Source: scripts/core/status_bar.js
shell-status-bar-trading-active = 稼働中
shell-status-bar-trading-inactive = 停止中

# Source: scripts/core/action_toasts.js

## Action toasts

shell-action-title-symbol = { $label } { $symbol }
shell-action-cancelled = { $title }をキャンセルしました
shell-action-swap-buy-live = 購入中
shell-action-swap-buy-done = 購入済み
shell-action-swap-buy-failed = 購入失敗
shell-action-swap-sell-live = 売却中
shell-action-swap-sell-done = 売却済み
shell-action-swap-sell-failed = 売却失敗
shell-action-position-open-live = ポジションをオープン中
shell-action-position-open-done = オープン済み
shell-action-position-open-failed = オープン失敗
shell-action-position-close-live = ポジションをクローズ中
shell-action-position-close-done = クローズ済み
shell-action-position-close-failed = クローズ失敗
shell-action-position-dca-live = ポジションに追加中
shell-action-position-dca-done = 追加済み
shell-action-position-dca-failed = 追加失敗
shell-action-partial-exit-live = 部分エグジット
shell-action-partial-exit-done = 部分エグジット
shell-action-partial-exit-failed = 部分エグジット失敗
shell-action-manual-order-live = 注文中
shell-action-manual-order-done = 注文完了
shell-action-manual-order-failed = 注文失敗
shell-action-trade-live = 取引
shell-action-trade-done = 取引完了
shell-action-trade-failed = 取引失敗
shell-action-via-router = { $action }（{ $router } 経由）
shell-action-with-note = { $label } · { $note }
shell-action-step-progress = { $label } · { $current }/{ $total }
shell-action-cost-guard-avoiding = { $venue } を回避中
shell-action-cost-guard-avoiding-cost = { $venue } を回避中 · { $cost }
shell-action-cost-guard-avoiding-unnamed = 取引所を回避中
shell-action-cost-guard-avoiding-unnamed-cost = 取引所を回避中 · { $cost }
shell-action-cost-guard-avoided = { $outcome } · { $venue } のレント { $cost } を回避
shell-action-cost-guard-avoided-unnamed = { $outcome } · 取引所のレント { $cost } を回避
shell-action-exit-full = 全量エグジット
shell-action-exit-percent = { $percent } エグジット

## Exit dialog (ui/exit_dialog.js)

shell-exit-title = { -brand } を閉じますか？
shell-exit-description = アプリケーションの閉じ方を選択してください
shell-exit-minimize = トレイに最小化
shell-exit-minimize-detail = バックグラウンドで実行を続けます
shell-exit-quit = アプリを終了
shell-exit-quit-detail = 完全に閉じ、すべてのサービスを停止します

## Image lightbox (ui/image_lightbox.js)

shell-lightbox-save =
    .title = 画像を保存
shell-lightbox-close =
    .title = 閉じる（ESC）

## Theme control (scripts/theme.js)

shell-theme-light = ライト
shell-theme-dark = ダーク
shell-theme-switch-to-light = ライトテーマに切り替え
shell-theme-switch-to-dark = ダークテーマに切り替え
