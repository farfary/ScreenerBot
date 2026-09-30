# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = 見積もりを取得できませんでした。接続を確認して、もう一度お試しください
# Fallback title when the quote request failed without a message.
trade-quote-error-title = 見積もりを取得できませんでした
trade-quote-title = スワップのプレビュー
trade-quote-refresh =
    .aria-label = 見積もりを更新
    .title = 見積もりを更新
trade-quote-idle = 数量を選択するとスワップをプレビューできます
trade-quote-loading = 最適なルートを検索中…
trade-quote-retry = 再試行
trade-quote-pay = 支払い
trade-quote-receive = 受取（推定）
trade-quote-minimum = 保証される最小受取額
    .title = 最大スリッページ適用後に受け取れる最小額です。これを下回る約定になる場合、スワップは実行されずに取り消されます。
trade-quote-impact = 価格インパクト
trade-quote-slippage = 最大スリッページ
trade-quote-platform-fee = プラットフォーム手数料
    .title = 0.5%。開発を支えるための手数料で、上記の見積もりにすでに含まれています。
trade-quote-network-fee = ネットワーク手数料
trade-quote-route = ルート
trade-quote-disclaimer = 価格はチェーンからリアルタイムで更新されます。保証される最小受取額を上回って約定できない場合、スワップは取り消されるため、表示より少ない額を受け取ることはありません。
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = 価格インパクト { $impact } が最大スリッページ { $tolerance }% を上回っています。この数量ではプールの価格が動きます。数量を減らすと、市場価格に近い価格で約定します。

## Units

trade-unit-sol = { -sol }
trade-unit-tokens = トークン

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = トークンを購入
trade-buy-subtitle = { -sol } で数量を入力
trade-buy-confirm = 購入を実行
trade-buy-hint = 空欄の場合は設定のデフォルト値を使用
trade-sell-title = ポジションを売却
trade-sell-subtitle = 売却する割合を選択
trade-sell-confirm = 売却を実行
trade-sell-hint = 1～100 の値を入力してください
trade-sell-input = カスタム割合
    .placeholder = 1-100
trade-add-title = ポジションに追加
trade-add-subtitle = 既存ポジションに DCA
trade-add-confirm = ポジションに追加
trade-add-hint = 空欄の場合は設定済みの DCA サイズを使用
trade-amount-input = カスタム数量
    .placeholder = { -sol } の数量を入力

## Presets

trade-presets-quick-amount = クイック数量
trade-presets-quick-sell = クイック売却
trade-presets-match-entry = エントリーに合わせる
trade-presets-fixed-amount = 固定数量
trade-preset-partial = 一部
trade-preset-half = 半分
trade-preset-most = ほとんど
trade-preset-full = 全量エグジット
# $label is the preset's amount.
trade-preset-select =
    .aria-label = { $label } を選択

## Dialog chrome

trade-dialog-close =
    .aria-label = ダイアログを閉じる
trade-input-max = MAX
    .aria-label = 最大値を使用
trade-slider =
    .aria-label = 数量スライダー
trade-context-available = 利用可能
trade-context-position-size = ポジションサイズ
trade-context-holdings = 保有
trade-held-badge = 保有中
    .title = このトークンのオープンポジションを保有しています
trade-manage-title = 手動管理
trade-manage-description = 自動トレーダーはこのポジションの売却や DCA を行いません。チェックを外すと、エグジットを自動トレーダーに任せます。

## Slippage

trade-slippage-label = スリッページ
trade-slippage-presets =
    .aria-label = スリッページのプリセット
trade-slippage-auto = 自動
trade-slippage-custom =
    .placeholder = カスタム
    .aria-label = カスタムスリッページ（%）
trade-slippage-note-auto = 自動（設定から）
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = 自動（設定の { $pct }%）
# $pct is the override as typed.
trade-slippage-note-override = 上書き: { $pct }%
# $pct is the override as typed.
trade-slippage-warning = スリッページが高めです。見積もりより最大 { $pct }% 少ない額を受け取る可能性があります。
trade-impact-warning-title = 価格インパクトが高い警告
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = この取引の価格インパクトは <strong>{ $impact }</strong> で、スリッページ許容値 <strong>{ $tolerance }%</strong> を超えています。予想を大きく下回る額を受け取る可能性があります。
trade-impact-warning-proceed = このまま続行

## Validation and verification

trade-error-invalid-number = 数値が無効です
trade-error-percentage-range = 割合は 1 から 100 の間で指定してください
trade-error-amount-positive = 数量は 0 より大きい値にしてください
trade-error-amount-minimum = 最小: 0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = 残高が不足しています（必要額 { $needed } + 手数料の余裕分 { $reserve }、現在の残高 { $balance }）
trade-error-position-closed = このポジションはすでにオープンではありません。
trade-error-verify-failed = トークン残高を検証できませんでした
trade-error-position-missing = ポジションが見つかりません。クローズされた可能性があります
# $expected and $current are formatted token amounts.
trade-error-balance-changed = トークン残高が変わりました。想定 { $expected }、現在 { $current }。更新してください。
trade-error-verify-network = 残高の検証中にネットワークエラーが発生しました

## Quick trade

trade-quick-buy-title = クイック購入
trade-quick-sell-title = クイック売却
trade-quick-subtitle = トークンのミントアドレスを入力
trade-quick-mint-label = トークンのミントアドレスを入力
trade-quick-mint-input =
    .placeholder = ミントアドレスを入力、またはシンボルで検索...
trade-quick-paste =
    .aria-label = クリップボードから貼り付け
trade-quick-recent = 最近:
trade-quick-fetching = トークン情報を取得中...
trade-quick-continue = 続ける
trade-quick-token-not-found = トークンが見つかりません
trade-quick-token-failed = トークンを取得できませんでした
trade-quick-token-not-in-database = データベースにトークンが見つかりません
trade-quick-token-info-failed = トークン情報を取得できませんでした
trade-quick-no-position = このトークンのポジションが見つかりません
trade-quick-no-holdings = ポジションに残っているトークンがありません
trade-quick-position-failed = ポジションデータを取得できませんでした

## Manual trade toasts

trade-toast-no-mint = 利用できるミントアドレスがありません
trade-toast-open-failed = 取引ダイアログを開けませんでした
trade-toast-pending-buy = 購入を実行中です
trade-toast-pending-add = 追加購入を実行中です
trade-toast-pending-sell = 売却を実行中です
trade-toast-pending-message = ブラウザーは待機を終了しました。結果はポジション行で確認してください
trade-toast-failed-buy = 購入に失敗しました
trade-toast-failed-add = ポジションへの追加に失敗しました
trade-toast-failed-sell = 売却に失敗しました

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = ストラテジーシグナル
trade-reason-manual-entry = 手動エントリー
trade-reason-force-buy = 強制購入
trade-reason-copy-buy = コピー購入
trade-reason-dca-scheduled = DCA 予約
trade-reason-take-profit = 利確
trade-reason-stop-loss = 損切り
trade-reason-trailing-stop = トレーリングストップ
trade-reason-time-override = 時間による上書き
trade-reason-strategy-exit = ストラテジーエグジット
trade-reason-llm-analysis-exit = LLM 分析エグジット
trade-reason-manual-exit = 手動エグジット
trade-reason-risk-management = リスク管理
trade-reason-blacklisted = ブラックリスト入り
trade-reason-force-sell = 強制売却
trade-reason-copy-sell = コピー売却
trade-reason-closed-externally = 外部でクローズ
trade-reason-wallet-history = ウォレット履歴
trade-reason-exit-retry-pending = エグジット再試行待ち
trade-reason-synthetic-exit-permanent-failure = 合成エグジットの恒久的な失敗
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason }（検証待ち）
# $note is the operator text of a force close.
trade-reason-force-closed = 強制クローズ: { $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = トークンが選択されていません
