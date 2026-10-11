notifications-action-swap-buy = 購入
notifications-action-swap-sell = 売却
notifications-action-position-open = オープン
notifications-action-position-close = クローズ
notifications-action-position-dca = DCA
notifications-action-position-partial-exit = 部分エグジット
notifications-action-manual-order = 手動
notifications-action-unknown = アクション

actions-step-evaluate = 評価中
actions-step-validate = 検証中
actions-step-quote = 見積もり取得中
actions-step-swap = スワップ実行中
actions-step-verify = 確認中
actions-step-unknown = 処理中
actions-step-evaluate-short = 評価中
actions-step-validate-short = 確認中
actions-step-quote-short = 見積もり
actions-step-swap-short = スワップ中
actions-step-verify-short = 承認待ち
actions-step-unknown-short = 処理中

actions-failure-recorded = { $message }
actions-failure-unknown = 不明なエラー
actions-failure-interrupted = アプリケーションの再起動により中断されました
actions-failure-validation = 検証に失敗しました
actions-failure-quote = 見積もりに失敗しました
actions-failure-swap = スワップに失敗しました
actions-failure-trade = 取引に失敗しました
actions-failure-entry = エントリーに失敗しました
actions-failure-exit = エグジットに失敗しました
actions-failure-dca = DCA に失敗しました
actions-failure-verification-expired = 確認の期限が切れました: トランザクションは反映されませんでした
actions-failure-verification-gave-up = 確認を断念しました
actions-failure-transaction-failed = トランザクションがオンチェーンで失敗しました
actions-failure-sell-transaction-failed = 売却トランザクションがオンチェーンで失敗しました
actions-failure-dca-verification-failed = DCA の確認に失敗しました

notifications-empty-all = アクションはありません
notifications-empty-active = 進行中のアクションはありません
notifications-empty-completed = 完了したアクションはありません
notifications-empty-failed = 失敗したアクションはありません
notifications-source-auto = 自動
notifications-source-manual = 手動
notifications-cancelled = キャンセル済み
notifications-load-failed = 読み込みに失敗しました
notifications-clear-title = 通知をクリア
notifications-clear-message = このリストのすべての通知を閉じますか？完了・失敗の履歴には残ります。
notifications-clear-failed = 通知をクリアできませんでした
notifications-stream-lag-title = アクションストリームが遅延しました
notifications-stream-lag-missed =
    { $count ->
       *[other] { $count }件の更新を取り逃しました — 更新中
    }
notifications-stream-lag-refreshing = 更新中
notifications-sync-failed = アクションを更新できませんでした

# Action rows (ui/action_row.js): day headings, the copy-trading source, the detail.
notifications-day-today = 今日
notifications-day-yesterday = 昨日
notifications-source-copy = コピートレード
notifications-steps = ステップ
notifications-detail-transaction = トランザクション
notifications-detail-fill-price = 約定価格
    .title = このスワップの { -sol } 額をトークン数量で割った値
notifications-detail-fee = ネットワーク手数料
notifications-detail-pnl = ポジション損益
notifications-detail-route = ルート
notifications-detail-duration = 所要時間
notifications-detail-reason = 理由
notifications-view-token = トークンを表示
notifications-view-position = ポジションを表示
notifications-view-transaction = トランザクションを表示
notifications-unrecorded = ポジションに未記録
notifications-unrecorded-detail = このスワップはポジションに記録されておらず、成立していない可能性があります。トランザクションを確認してください。
