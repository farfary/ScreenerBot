# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = 購入
transactions-type-sell = 売却
transactions-type-swap = スワップ
transactions-type-sol-transfer = SOL 送金
transactions-type-token-transfer = トークン送金
transactions-type-transfer = 送金
transactions-type-dust = ダスト
transactions-type-spam = スパム
transactions-type-ata-create = アカウント開設
transactions-type-ata-close = レント回収
transactions-type-ata = トークンアカウント
transactions-type-liquidity-add = 流動性の追加
transactions-type-liquidity-remove = 流動性の削除
transactions-type-nft = NFT
transactions-type-program = プログラム呼び出し
transactions-type-compute = コンピュート
transactions-type-failed = 失敗
transactions-type-unknown = 未分類

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label }（{ $detail }）
transactions-type-token-transfer-detail = { $label } { $mint }（{ $amount }）
transactions-type-spam-detail = スパムエアドロップ（{ $mint }）
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = すべての種類
transactions-filter-transfer = 送金
transactions-filter-ata = レントとアカウント
transactions-filter-liquidity = 流動性
transactions-filter-program = プログラム呼び出し

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = 入金
transactions-direction-outgoing = 出金
transactions-direction-internal = 内部
transactions-direction-unknown = 未分類

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = 保留中
transactions-status-confirmed = 承認済み
transactions-status-finalized = ファイナライズ済み
transactions-status-failed = 失敗
transactions-status-success = 成功
transactions-status-unknown = 不明

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = 作成
transactions-ata-operation-closure = クローズ

## Transactions page (pages/transactions.js)

transactions-toolbar-title = トランザクション履歴
transactions-search =
    .placeholder = シグネチャを検索…
    .aria-label = トランザクションのシグネチャを検索
transactions-load-failed = トランザクションを更新できませんでした
transactions-summary-total = 合計
transactions-summary-estimate = 推定
transactions-summary-success = 成功
transactions-summary-failed = 失敗
transactions-filter-wallet = ウォレット
transactions-filter-type = 種類
transactions-filter-direction = 方向
transactions-filter-status = ステータス
transactions-filter-all-directions = すべての方向
transactions-filter-all-statuses = すべてのステータス
transactions-wallet-main = メインウォレット
transactions-col-time = 時刻
transactions-col-signature = シグネチャ
transactions-col-type = 種類
transactions-col-direction = 方向
transactions-col-status = ステータス
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = 手数料（{ -sol }）
transactions-col-token = トークン
transactions-col-router = ルーター
transactions-col-instructions = 命令数

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = シグネチャをコピー
transactions-dialog-close =
    .title = 閉じる（ESC）
transactions-dialog-tabs-label = トランザクション詳細のセクション
transactions-dialog-meta-slot = スロット:
transactions-dialog-meta-fee = 手数料:
transactions-dialog-loading = 読み込み中...
transactions-dialog-loading-details = トランザクションの詳細を読み込み中...
transactions-dialog-load-failed = トランザクションの詳細を読み込めませんでした
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = トランザクションの詳細を読み込めませんでした: { $reason }
transactions-dialog-not-found = トランザクションが見つかりません
transactions-dialog-tab-overview = 概要
transactions-dialog-tab-balances = 残高
transactions-dialog-tab-instructions = インストラクション
transactions-dialog-tab-logs = ログ
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = Raw
transactions-dialog-unknown = 不明
transactions-dialog-unknown-asset = 不明な資産
transactions-dialog-unavailable = 利用不可

## Transaction details dialog: overview

transactions-dialog-failed-title = トランザクションが失敗しました
transactions-dialog-no-program-error = プログラムエラーは提供されていません。
transactions-dialog-story-title = 何が起きたか
# $router is the routing program name.
transactions-dialog-router-via = { $router } 経由
transactions-dialog-flow-paid = 支払い
transactions-dialog-flow-received = 受取
transactions-dialog-flow-from = 送信元
transactions-dialog-flow-to = 送信先
transactions-dialog-flow-amount = 数量
transactions-dialog-net-wallet-change = ウォレットの正味増減:
transactions-dialog-processed = Solana で処理済み
transactions-dialog-execution-title = 約定
transactions-dialog-metric-execution-price = 約定価格
transactions-dialog-metric-effective-received = 実質受取額
transactions-dialog-metric-effective-spent = 実質支払額
transactions-dialog-metric-network-fee = ネットワーク手数料
transactions-dialog-metric-estimated-pnl = 推定損益
transactions-dialog-metric-net-native-change = { -sol } の正味増減
transactions-dialog-route-title = ルートと資産
transactions-dialog-route-router = ルーター
transactions-dialog-route-input-asset = 入力資産
transactions-dialog-route-output-asset = 出力資産
transactions-dialog-route-pool = プール
transactions-dialog-route-program = プログラム
transactions-dialog-tech-title = 技術詳細
transactions-dialog-tech-summary = シグネチャ、スロット、リソース
transactions-dialog-tech-signature = シグネチャ
transactions-dialog-tech-timestamp = タイムスタンプ
transactions-dialog-tech-slot = スロット
transactions-dialog-tech-exact-fee = 正確な手数料
transactions-dialog-tech-accounts = アカウント
transactions-dialog-tech-instructions = インストラクション
transactions-dialog-tech-compute-units = コンピュートユニット
transactions-dialog-tech-token-decimals = トークンの小数桁数

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = { -sol } 残高の増減
transactions-dialog-balances-native-empty = { -sol } 残高の増減はありません
transactions-dialog-balances-token-title = トークン残高の増減
transactions-dialog-balances-token-empty = トークン残高の増減はありません
transactions-dialog-balances-net-native = { -sol } の正味増減
transactions-dialog-balances-fee = トランザクション手数料
transactions-dialog-col-account = アカウント
transactions-dialog-col-token = トークン
transactions-dialog-col-pre-balance = 変更前残高
transactions-dialog-col-post-balance = 変更後残高
transactions-dialog-col-change = 増減
transactions-dialog-col-type = 種類
transactions-dialog-col-rent = レント（{ -sol }）
transactions-dialog-instructions-empty = インストラクションが見つかりません
transactions-dialog-instructions-count =
    { $count ->
       *[other] インストラクション { $count }件
    }
transactions-dialog-instruction-program-id = プログラム ID
transactions-dialog-instruction-accounts = アカウント（{ $count }）
transactions-dialog-instruction-data = データ
transactions-dialog-logs-empty = 利用できるログはありません
transactions-dialog-logs-filter = ログを絞り込み...
transactions-dialog-logs-no-match = 一致するログはありません
transactions-dialog-logs-count =
    { $count ->
       *[other] ログ { $count }件
    }
transactions-dialog-ata-empty = このトランザクションに ATA 操作はありません
transactions-dialog-ata-summary-title = ATA 分析サマリー
transactions-dialog-ata-creations = 作成
transactions-dialog-ata-closures = クローズ
transactions-dialog-ata-rent-spent = 支払ったレント
transactions-dialog-ata-rent-recovered = 回収したレント
transactions-dialog-ata-net-rent = レントの正味影響
transactions-dialog-ata-operations-title = ATA 操作（{ $count }）
transactions-dialog-raw-copy = JSON をコピー
transactions-dialog-raw-empty = 利用できる Raw データはありません
