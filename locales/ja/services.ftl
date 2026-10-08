# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

services-health-component-unavailable = { $component } コンポーネントを利用できません
services-health-unavailable = ヘルスステータスを利用できません
services-health-pools-not-running = プールサービスが実行されていません
services-health-events-db-uninitialized = イベントデータベースが初期化されていません
services-health-sol-price-not-running = { -sol } 価格サービスが実行されていません
services-health-sol-price-stale = { -sol } 価格データが古くなっています（{ $seconds }秒前）
services-health-sol-price-no-data = { -sol } 価格データはまだありません
services-health-telegram-discovery = ディスカバリーモード
services-health-telegram-disconnected = 切断
services-health-wallet-watch-polling-only = ポーリングのみで検出中
services-health-assistant-tasks-disabled = 設定で無効
services-health-connectivity-critical-unhealthy = 重要なエンドポイントが異常です: { $endpoints }
services-health-filtering-snapshot-stale = フィルタリングのスナップショットは{ $seconds }秒前のものです

## Services page (pages/services.js)

services-status-healthy = 正常
services-status-starting = 起動中
services-status-degraded = 低下
services-status-unhealthy = 異常
services-status-stopping = 停止中
services-status-disabled = 無効
services-status-unknown = 不明

services-name-account = アカウント
services-name-assistant-scheduled-tasks = アシスタントのスケジュールタスク
services-name-ata-cleanup = トークンアカウントのクリーンアップ
services-name-connectivity = 接続状況
services-name-copy-trading = コピートレード
services-name-events = イベント
services-name-filtering = フィルタリング
services-name-llm-analysis = LLM 分析
services-name-ohlcv = OHLCV
services-name-pool-pricing = プール価格算出
services-name-pools = プール
services-name-positions = ポジション
services-name-referral = 紹介
services-name-rpc-stats = RPC 統計
services-name-sol-price = { -sol } 価格
services-name-telegram = { -telegram }
services-name-tokens = トークン
services-name-trader = トレーダー
services-name-transactions = トランザクション
services-name-update-check = アップデート確認
services-name-wallet = ウォレット
services-name-wallet-watch = ウォレットウォッチ
services-name-webserver = Web サーバー

services-loading = サービスを読み込み中...
services-load-failed = サービスを読み込めませんでした
services-load-failed-description = バックエンドの応答を待っています。自動的に再試行します。
services-refresh-failed = サービスを更新できませんでした
services-search-placeholder = サービスを検索...
services-summary-total = 合計
services-summary-alerts = アラート
services-summary-alerts-tooltip = 低下 { $degraded } / 異常 { $unhealthy }
services-filter-status = ステータス
services-filter-all-statuses = すべてのステータス
services-filter-all-services = すべてのサービス
services-filter-enabled-only = 有効のみ
services-filter-disabled-only = 無効のみ
services-col-service = サービス
services-col-health = ヘルス
services-col-priority = 優先度
services-col-uptime = 稼働時間
services-col-activity = アクティビティ
services-col-last-cycle = 前回サイクル
services-col-avg-cycle = 平均サイクル
services-col-avg-poll = 平均ポーリング
services-col-cycle-rate = サイクルレート
services-col-tasks = タスク
services-col-ops = 操作/秒
services-col-errors = エラー
services-col-dependencies = 依存関係
services-activity-busy = 使用率 { $percent }
services-activity-polls =
    { $count ->
       *[other] ポーリング{ $count }回
    }
services-tasks-tooltip =
    { $count ->
       *[other] { $count }個のタスク
    }
    前回: { $last }
    平均: { $avg }
    ポーリング: { $poll }
    アイドル: { $idle }
    ポーリング合計: { $polls }
services-tasks-none = 計測対象のタスクはありません

# Empty table (scripts/pages/services.js)
services-empty = 実行中のサービスはありません
    .message = ボットがサービスを起動すると、ここに表示されます。
