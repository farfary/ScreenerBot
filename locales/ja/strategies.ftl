# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = すべて
strategies-filter-entry = エントリー
strategies-filter-exit = エグジット
strategies-type-entry = エントリー
strategies-type-exit = エグジット
strategies-list-empty-title = ストラテジーはまだありません
strategies-list-empty-hint = 最初のストラテジーを作成しましょう
strategies-new = 新規ストラテジー
strategies-import =
    .title = ストラテジーをインポート
    .aria-label = ストラテジーをインポート
strategies-item-enable =
    .title = 有効化
strategies-item-disable =
    .title = 無効化

# Name given to a strategy before it is saved.
strategies-new-name = 新規ストラテジー

## Editor

strategies-editor-name =
    .placeholder = ストラテジー名
strategies-editor-dirty =
    .title = 未保存の変更
strategies-action-validate = 検証
strategies-editor-empty = 編集するストラテジーを選択するか、新規作成してください
strategies-conditions-empty-title = 条件はまだありません
strategies-conditions-empty-hint = 「{ strategies-add-condition }」から作成を始めましょう
strategies-add-condition = 条件を追加
strategies-modal-close =
    .aria-label = 閉じる
strategies-card-move-up =
    .title = 上へ移動
strategies-card-move-down =
    .title = 下へ移動
strategies-card-duplicate =
    .title = 複製
strategies-card-delete =
    .title = 削除
# $name is the condition name.
strategies-card-delete-confirm = 条件を削除
    .message = このストラテジーから「{ $name }」を削除しますか？

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }: { $value }
strategies-summary-none = パラメーターなし
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = ストラテジーの設定（{ $value }）
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount }秒
strategies-value-duration-minutes = { $amount }分
strategies-value-duration-hours = { $amount }時間
strategies-value-duration-days = { $amount }日
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = 現地時刻: { $time }
strategies-hint-price-impact = このサイズの取引で価格は約 { $amount }% 動きます

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
       *[other] { $amount }時間
    }
strategies-value-candles =
    { $count ->
       *[other] { $amount }本
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = 時間
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = 条件を検索...
    .aria-label = 条件を検索
strategies-catalog-search-clear =
    .aria-label = 検索をクリア
strategies-catalog-no-description = 説明はありません

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = 有効
strategies-editor-timeframe = 既定の時間足
    .title = 独自の時間足を指定しない条件が使うローソク足の時間足
strategies-editor-priority = 優先度
    .title = 1〜100 の評価順。小さい数値ほど先に評価されます
strategies-editor-description = 説明
strategies-editor-description-field =
    .placeholder = このストラテジーの内容
    .aria-label = ストラテジーの説明
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = すべてのエントリー戦略
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = 管理対象
strategies-editor-entry-binding-hint = チェックしたエントリー戦略が開いたポジションを管理します。何もチェックしないと、手動やコピーを含むすべてのポジションを管理します。
strategies-editor-entry-binding-empty = エントリー戦略はまだありません
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id }（削除済み）
strategies-action-export = エクスポート
strategies-action-test = トークンでテスト
# $token is the tested token's symbol or address.
strategies-test-title = { $token } でのテスト
# $token is the tested token's symbol or address.
strategies-test-running = { $token } でテスト中…
strategies-test-verdict-pass = 成立
strategies-test-verdict-fail = 不成立
strategies-test-verdict-unknown = 不明

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = 次の条件のうち{ $mode }を満たす
strategies-group-match = 次のうち{ $mode }を満たす
strategies-match-all = すべて
strategies-match-any = いずれか
strategies-match-mode =
    .aria-label = すべてまたはいずれかの条件
strategies-invert = 反転
    .title = 反転：これに一致しないときに成立します
strategies-chip-off = オフ
strategies-chip-inverted = 反転中
strategies-row-switch =
    .title = 保存したまま評価しない場合はオフにします
    .aria-label = このストラテジーで評価
strategies-row-expand =
    .aria-label = パラメーターの表示・非表示
strategies-add-group = グループを追加
# $count is the number of conditions in the group.
strategies-group-delete-confirm = グループを削除
    .message = このグループと中の{ $count }件の条件を削除しますか？

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems = 修正が必要な問題が{ $count }件あります: { $first }
strategies-problem-located = { $name }: { $problem }
strategies-problem-located-field = { $name }（{ $field }）: { $problem }
strategies-problem-rules-empty = オンの条件を少なくとも1つ追加してください
strategies-problem-group-empty = グループにはオンの条件が少なくとも1つ必要です
strategies-problem-not-applicable = この条件はこの種類のストラテジーでは使用できません
strategies-problem-unknown-condition = 不明な条件タイプ: { $type }
strategies-problem-too-deep = ルールの入れ子は最大{ $max }階層までです。反転したグループは1階層多く数えます
strategies-problem-required = 数値を入力してください
strategies-problem-range = { $min }〜{ $max }の範囲で指定してください
strategies-problem-invalid-option = 一覧のいずれかを選択してください

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = すべて
strategies-catalog-categories =
    .aria-label = カテゴリ
strategies-catalog-empty = 検索に一致する条件はありません
strategies-catalog-exit-only = エグジットストラテジー専用
strategies-catalog-entry-only = エントリーストラテジー専用
strategies-catalog-candles-note = { $description }ローソク足のみを読み取ります。
strategies-data-candles = ローソク足
strategies-data-pool = プール
strategies-data-position = ポジション
strategies-data-market = 市場アクティビティ
strategies-data-clock = 時刻

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = 件
strategies-value-trades = { $amount }件

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = 階層が多すぎます
    .message = ルールの入れ子は最大{ $max }階層までです
strategies-toast-priority-invalid = 優先度が正しくありません
    .message = 1〜100の整数を入力してください
strategies-toast-exported = ストラテジーをエクスポートしました
# $name is the strategy name.
strategies-unsaved-confirm = 未保存の変更を破棄しますか？
    .message = 「{ $name }」に未保存の変更があります。
strategies-unsaved-discard = 破棄
strategies-import-error-json = ファイルは有効なJSONではありません
strategies-import-error-shape = ファイルにストラテジーが記述されていません
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = このバージョンにない条件がファイルで使われています: { $type }
strategies-import-error-not-applicable = 「{ $name }」はこの種類のストラテジーでは使用できません

## New strategy dialog

strategies-create-title = 新規ストラテジーを作成
strategies-create-prompt = 作成するストラテジーの種類を選択してください:
strategies-create-entry-name = エントリーストラテジー
strategies-create-entry-description = トークンを購入するタイミングの条件を定義します
strategies-create-exit-name = エグジットストラテジー
strategies-create-exit-description = トークンを売却するタイミングの条件を定義します

## Delete dialog

strategies-delete-title = ストラテジーを削除
# $name is the strategy name.
strategies-delete-message = ストラテジー「{ $name }」を削除しますか？ この操作は元に戻せません。

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = 保存する前に検証エラーを修正してください
strategies-toast-load-failed = 読み込み失敗
    .message = サーバーからストラテジーを読み込めませんでした
strategies-toast-load-strategy-failed = ストラテジーを読み込めませんでした
strategies-toast-no-strategy = ストラテジーが作成されていません
    .message = 条件を1つ以上追加するか、「新規ストラテジー」をクリックして先にストラテジーを作成してください
strategies-toast-name-required = 名前が必要です
    .message = 保存する前にストラテジー名を入力してください
strategies-toast-saved = ストラテジーを保存しました
    .message = 「{ $name }」を保存しました
strategies-toast-save-failed = 保存失敗
    .message = ストラテジーをデータベースに保存できませんでした
strategies-toast-no-strategy-validate = 検証するストラテジーがありません
strategies-toast-no-strategy-test = テストするストラテジーがありません
strategies-toast-test-save-first = テストの前にストラテジーを保存してください。テストは保存済みのルールを読み込みます
strategies-toast-test-failed = テストに失敗しました
    .message = ストラテジーをテストできませんでした
strategies-toast-valid = ストラテジーは有効です
strategies-toast-invalid = ストラテジーにエラーがあります
strategies-toast-validation-failed = 検証に失敗しました
strategies-toast-item-enabled = ストラテジーを有効にしました
strategies-toast-item-disabled = ストラテジーを無効にしました
strategies-toast-item-toggle-failed = ストラテジーを切り替えられませんでした
strategies-toast-deleted = ストラテジーを削除しました
    .message = 「{ $name }」を削除しました
strategies-toast-delete-failed = 削除失敗
    .message = ストラテジーをデータベースから削除できませんでした
strategies-toast-imported = ストラテジーをインポートしました
strategies-toast-import-failed = ストラテジーをインポートできませんでした
strategies-toast-unknown-condition = 不明な条件
    .message = 条件の種類が見つかりません
strategies-toast-create-first = 先にストラテジーを作成してください
    .message = 条件を追加する前に、「新規ストラテジー」をクリックしてストラテジーを作成してください

## Conditions

strategies-condition-candle-size = ローソク足サイズパターン
    .description = 特定のローソク足パターン（大きな実体、小さな実体（十字線）、長いヒゲ）を検出します。
strategies-condition-candle-size-param-pattern = パターンの種類
    .description = 検出するローソク足パターン
strategies-condition-candle-size-param-pattern-option-large-body = 大きな実体（強い値動き）
strategies-condition-candle-size-param-pattern-option-small-body = 小さな実体（十字線/迷い）
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 長い上ヒゲ（反落）
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 長い下ヒゲ（サポート）
strategies-condition-candle-size-param-threshold = サイズしきい値 %
    .description = パターン検出のしきい値（パーセント）

strategies-condition-consecutive-candles = 連続ローソク足
    .description = 最小サイズフィルター付きで、連続する陽線（強気）または陰線（弱気）を検出します。
strategies-condition-consecutive-candles-param-count = ローソク足の本数
    .description = 必要な連続ローソク足の本数
strategies-condition-consecutive-candles-param-direction = ローソク足の方向
    .description = 連続するローソク足の色/方向
strategies-condition-consecutive-candles-param-direction-option-green = 陽線（強気）
strategies-condition-consecutive-candles-param-direction-option-red = 陰線（弱気）
strategies-condition-consecutive-candles-param-minimum-change = 最小変動 %
    .description = 各ローソク足の最小変動率（ノイズを除外）

strategies-condition-liquidity-level = プール流動性レベル
    .description = プールの流動性を { -sol } でチェックします（エントリー: 十分な流動性を確保、エグジット: 流動性の流出を検出）
strategies-condition-liquidity-level-param-threshold = 流動性しきい値（{ -sol }）
    .description = { -sol } 建てのプール流動性レベル
strategies-condition-liquidity-level-param-comparison = 比較
    .description = プール流動性としきい値の比較方法
strategies-condition-liquidity-level-param-comparison-option-greater-than = より大きい（>）
strategies-condition-liquidity-level-param-comparison-option-greater-equal = 以上（≥）
strategies-condition-liquidity-level-param-comparison-option-less-than = より小さい（{ "<" }）
strategies-condition-liquidity-level-param-comparison-option-less-equal = 以下（≤）

strategies-condition-position-holding-time = ポジション保有時間
    .description = ポジションの保有時間をチェックします
strategies-condition-position-holding-time-param-hours = 時間しきい値（時間）
    .description = ポジションをオープンしてからの経過時間（時間単位）
strategies-condition-position-holding-time-param-comparison = 比較
    .description = ポジションの経過時間としきい値の比較方法
strategies-condition-position-holding-time-param-comparison-option-greater-than = より長い（>）
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 以上（≥）
strategies-condition-position-holding-time-param-comparison-option-less-than = より短い（{ "<" }）
strategies-condition-position-holding-time-param-comparison-option-less-equal = 以下（≤）

strategies-condition-price-breakout = 価格ブレイクアウト
    .description = 価格がレジスタンス（期間高値）を上抜け、またはサポート（期間安値）を下抜けたことを検出します。
strategies-condition-price-breakout-param-lookback = 参照期間
    .description = サポート/レジスタンスを求めるためのローソク足の本数
strategies-condition-price-breakout-param-direction = ブレイクアウトの方向
    .description = ブレイクアウトの方向
strategies-condition-price-breakout-param-direction-option-upward = 上方向（レジスタンス突破）
strategies-condition-price-breakout-param-direction-option-downward = 下方向（サポート割れ）
strategies-condition-price-breakout-param-confirmation = 確認 %
    .description = ブレイクアウトを確認するために水準をどれだけ超える必要があるか（だましを回避）

strategies-condition-price-change-percent = 価格変動率 %
    .description = 一定期間内に価格がしきい値の割合だけ変動したかをチェックします。
strategies-condition-price-change-percent-param-percentage = 変動しきい値 %
    .description = トリガーとなる価格変動率（0.1～1000%）
strategies-condition-price-change-percent-param-direction = 方向
    .description = 価格の動く方向
strategies-condition-price-change-percent-param-direction-option-above = 上昇（+%）
strategies-condition-price-change-percent-param-direction-option-below = 下落（-%）
strategies-condition-price-change-percent-param-direction-option-within = 範囲内（±%）
strategies-condition-price-change-percent-param-time-value = 参照期間
    .description = 変化を測る期間。単位はフィールド内で選択（1～3600 秒、1～1440 分、1～720 時間）
strategies-condition-price-change-percent-param-time-unit = 時間の単位
    .description = 参照期間の時間単位
strategies-condition-price-change-percent-param-time-unit-option-seconds = 秒
strategies-condition-price-change-percent-param-time-unit-option-minutes = 分
strategies-condition-price-change-percent-param-time-unit-option-hours = 時間

strategies-condition-price-to-ma = 価格と移動平均の比較
    .description = 価格が単純移動平均の上、下、または範囲内にあるかをチェックします。
strategies-condition-price-to-ma-param-period = MA 期間
    .description = 移動平均の計算に使うローソク足の本数
strategies-condition-price-to-ma-param-position = 位置
    .description = MA に対する価格の位置
strategies-condition-price-to-ma-param-position-option-above = MA より上
strategies-condition-price-to-ma-param-position-option-below = MA より下
strategies-condition-price-to-ma-param-position-option-within = 範囲内
strategies-condition-price-to-ma-param-distance = 乖離 %
    .description = MA からの最小乖離（上/下の場合）または最大範囲（範囲内の場合）

strategies-condition-volume-spike = 出来高急増
    .description = 平均出来高と比べた出来高の急増を検出します（関心の高まりを示します）。
strategies-condition-volume-spike-param-lookback = 参照期間
    .description = 平均出来高を計算するローソク足の本数
strategies-condition-volume-spike-param-multiplier = 出来高倍率
    .description = 平均の何倍か（例: 2.0 = 平均の 200%）

strategies-condition-rsi = RSI（相対力指数）
    .description = 確定したローソク足でワイルダー方式の平滑化を行った RSI がしきい値を上回る、または下回ると発動します。
strategies-condition-rsi-param-period = RSI 期間
    .description = RSI の計算に使うローソク足の本数。この値より 1 本多く必要です
strategies-condition-rsi-param-comparison = 比較
    .description = RSI がしきい値を下回る必要があるか、上回る必要があるか
strategies-condition-rsi-param-comparison-option-below = 下回る（売られすぎ）
strategies-condition-rsi-param-comparison-option-above = 上回る（買われすぎ）
strategies-condition-rsi-param-threshold = RSI しきい値
    .description = 1〜99 の RSI 水準（売られすぎは 30、買われすぎは 70 が一般的）

strategies-condition-ma-cross = 移動平均のクロス
    .description = 短期の移動平均が直近の数本以内に長期の移動平均を交差すると発動します。
strategies-condition-ma-cross-param-ma-type = 平均の種類
    .description = 指数移動平均は新しいローソク足を重く、単純移動平均はすべてのローソク足を同じ重みで扱います
strategies-condition-ma-cross-param-ma-type-option-ema = 指数（EMA）
strategies-condition-ma-cross-param-ma-type-option-sma = 単純（SMA）
strategies-condition-ma-cross-param-fast-period = 短期の期間
    .description = 短期移動平均のローソク足の本数
strategies-condition-ma-cross-param-slow-period = 長期の期間
    .description = 長期移動平均のローソク足の本数。短期の期間より大きくする必要があります
strategies-condition-ma-cross-param-direction = クロスの方向
    .description = 短期平均が長期平均をどちら向きに交差するか
strategies-condition-ma-cross-param-direction-option-bullish = 強気（短期が上抜け）
strategies-condition-ma-cross-param-direction-option-bearish = 弱気（短期が下抜け）
strategies-condition-ma-cross-param-within-candles = クロスの対象範囲
    .description = クロスを含んでよい直近のローソク足の本数。1 は最新の 1 本のみ

strategies-condition-position-profit = ポジション損益
    .description = オープン中のポジションの含み益がエントリー価格に対する指定割合を上回る、または下回ると発動します（プール価格で算出）。
strategies-condition-position-profit-param-comparison = 比較
    .description = 利益がしきい値以上か、以下か
strategies-condition-position-profit-param-comparison-option-above = 以上（≥）
strategies-condition-position-profit-param-comparison-option-below = 以下（≤）
strategies-condition-position-profit-param-threshold-pct = 利益しきい値 %
    .description = エントリー価格に対する含み益の割合。負の値は損失です

strategies-condition-drawdown-from-peak = 高値からの下落
    .description = 最後の購入以降の最高価格から指定した割合だけプール価格が下落すると発動します。その最高価格がエントリー価格より十分高かった場合に限ります。
strategies-condition-drawdown-from-peak-param-drop-pct = 高値からの下落 %
    .description = 現在価格が最高価格よりどれだけ低い必要があるか
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = 利益後に有効化 %
    .description = 条件が発動できるようになるには、最高価格がエントリー価格よりこの割合以上高かったことが必要です。0 は常に有効

strategies-condition-buy-pressure = 買い圧力
    .description = 集計期間内の取引に占める買いの割合が指定以上で、その期間に十分な取引数がある場合に発動します。トークンの買い・売りの取引数を読み取ります。
strategies-condition-buy-pressure-param-window = 集計期間
    .description = 買い・売りの取引数が対象とする期間
strategies-condition-buy-pressure-param-window-option-5m = 5 分
strategies-condition-buy-pressure-param-window-option-1h = 1 時間
strategies-condition-buy-pressure-param-window-option-6h = 6 時間
strategies-condition-buy-pressure-param-window-option-24h = 24 時間
strategies-condition-buy-pressure-param-min-buy-share-pct = 最低買い比率 %
    .description = 集計期間内の全取引に占める買いの割合
strategies-condition-buy-pressure-param-min-transactions = 最低取引数
    .description = 取引数がこれより少ない集計期間では条件が発動しません

strategies-condition-transaction-velocity = 取引速度
    .description = 直近期間の取引ペースが、それを含む長い期間のペースの指定倍率以上または以下のときに成立します。トークンの買いと売りの件数を読み取ります。
strategies-condition-transaction-velocity-param-window = 期間
    .description = 直近期間と、比較する長い期間
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5分 対 1時間
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1時間 対 6時間
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6時間 対 24時間
strategies-condition-transaction-velocity-param-comparison = 比較
    .description = 直近のペースが比率以上か以下か
strategies-condition-transaction-velocity-param-comparison-option-above = 以上（≥）
strategies-condition-transaction-velocity-param-comparison-option-below = 以下（≤）
strategies-condition-transaction-velocity-param-ratio = ペース比率
    .description = 直近期間の毎秒取引数を長い期間の毎秒取引数で割った値
strategies-condition-transaction-velocity-param-min-transactions = 最低取引数
    .description = 取引数がこれより少ない直近期間では成立しません

strategies-condition-pullback-from-high = 高値からの押し目
    .description = 現在価格が直近ローソク足の最高値から最小下落と最大下落の間にあるときに成立します。
strategies-condition-pullback-from-high-param-lookback = 参照期間
    .description = 最高値を取るローソク足の本数
strategies-condition-pullback-from-high-param-min-drop-pct = 最小下落 %
    .description = 価格は最高値から少なくともこれだけ下にある必要があります
strategies-condition-pullback-from-high-param-max-drop-pct = 最大下落 %
    .description = 価格は最高値から最大でもこれだけ下にある必要があります。最小下落より大きくしてください

strategies-condition-profit-giveback = 利益の戻し
    .description = 直近の買い以降の最高含み益が一定水準に達した後、その一部を戻したときに成立します。
strategies-condition-profit-giveback-param-arm-after-profit-pct = 有効化する利益 %
    .description = 条件が成立するには、最高含み益がエントリー価格のこの割合に達している必要があります
strategies-condition-profit-giveback-param-giveback-pct = 戻し %
    .description = 最高含み益のうち戻した割合。100% は価格がエントリー価格まで戻ったことを意味します

strategies-condition-token-age = トークンの経過時間
    .description = トークンのオンチェーン作成からの時間が指定期間より長いまたは短いときに成立します。作成時刻が不明なトークンでは条件は未確定のままです。
strategies-condition-token-age-param-comparison = 比較
    .description = トークンが期間より古いか新しいか
strategies-condition-token-age-param-comparison-option-older-than = より古い（≥）
strategies-condition-token-age-param-comparison-option-younger-than = より新しい（{ "<" }）
strategies-condition-token-age-param-time-value = 経過時間
    .description = トークン作成からの時間。単位はフィールド内で選択（1-10000）
strategies-condition-token-age-param-time-unit = 時間単位
    .description = 経過時間の単位
strategies-condition-token-age-param-time-unit-option-minutes = 分
strategies-condition-token-age-param-time-unit-option-hours = 時間
strategies-condition-token-age-param-time-unit-option-days = 日

strategies-condition-vwap = VWAP
    .description = 現在価格が直近ローソク足の出来高加重平均価格より、指定距離以上上または下にあるときに成立します。
strategies-condition-vwap-param-lookback = 参照期間
    .description = 出来高加重平均の対象となるローソク足の本数
strategies-condition-vwap-param-position = 位置
    .description = VWAP に対する価格の位置
strategies-condition-vwap-param-position-option-above = VWAP より上
strategies-condition-vwap-param-position-option-below = VWAP より下
strategies-condition-vwap-param-distance-pct = 距離 %
    .description = VWAP からの最小距離。0 は選んだ側であれば距離を問いません

strategies-condition-volatility-range = ボラティリティレンジ
    .description = 直近ローソク足の ATR（平均真の値幅）が現在価格に対する割合でしきい値より上または下のときに成立します。グループ内に 2 つ置くと範囲になります。
strategies-condition-volatility-range-param-period = ATR 期間
    .description = 平均真の値幅を平滑化するローソク足の本数。条件にはこれより 1 本多く必要です
strategies-condition-volatility-range-param-comparison = 比較
    .description = 値幅がしきい値より上か下か
strategies-condition-volatility-range-param-comparison-option-above = 上
strategies-condition-volatility-range-param-comparison-option-below = 下
strategies-condition-volatility-range-param-threshold-pct = 値幅しきい値 %
    .description = 現在価格に対する平均真の値幅の割合

strategies-condition-bollinger-bands = ボリンジャーバンド
    .description = 現在価格が下側バンドより下または上側バンドより上にあるとき、またはバンドが収縮したときに成立します。バンドは平均終値から標準偏差の数倍離れた位置にあります。
strategies-condition-bollinger-bands-param-period = 期間
    .description = 平均と標準偏差の対象となるローソク足の本数
strategies-condition-bollinger-bands-param-std-dev = 標準偏差
    .description = 各バンドが平均から何標準偏差離れるか
strategies-condition-bollinger-bands-param-signal = シグナル
    .description = バンドが示すべき状態
strategies-condition-bollinger-bands-param-signal-option-below-lower = 下側バンドより下
strategies-condition-bollinger-bands-param-signal-option-above-upper = 上側バンドより上
strategies-condition-bollinger-bands-param-signal-option-squeeze = スクイーズ
strategies-condition-bollinger-bands-param-squeeze-width-pct = スクイーズ幅 %
    .description = スクイーズの場合: バンド間の最大距離（平均に対する割合）

strategies-condition-liquidity-to-size = サイズに対する流動性
    .description = プールの準備金が、対象サイズの倍率以上または未満のときに成立します。エントリー戦略では取引サイズ、エグジット戦略ではポジションの残存価値です。
strategies-condition-liquidity-to-size-param-comparison = 比較
    .description = 準備金が倍率以上か未満か
strategies-condition-liquidity-to-size-param-comparison-option-at-least = 以上（≥）
strategies-condition-liquidity-to-size-param-comparison-option-below = 未満（{ "<" }）
strategies-condition-liquidity-to-size-param-multiple = 流動性倍率
    .description = プール準備金を対象サイズで割った値。倍率 N で価格は約 1/(N+1) 動きます

strategies-condition-position-adds = ポジションの追加
    .description = 最初のエントリー後の追加買いの回数、または部分売却の回数が指定数以上または以下のときに成立します。
strategies-condition-position-adds-param-subject = 対象
    .description = 数える操作
strategies-condition-position-adds-param-subject-option-adds = 追加買い
strategies-condition-position-adds-param-subject-option-partial-sells = 部分売却
strategies-condition-position-adds-param-comparison = 比較
    .description = 回数が指定数以上か以下か
strategies-condition-position-adds-param-comparison-option-at-least = 以上（≥）
strategies-condition-position-adds-param-comparison-option-at-most = 以下（≤）
strategies-condition-position-adds-param-count = 回数
    .description = 操作の回数

strategies-condition-time-of-day = 時刻
    .description = 現在の時が UTC の時間帯に入っているときに成立します。開始が終了より後の場合は深夜 0 時をまたぎます。
strategies-condition-time-of-day-param-start-hour = 開始時（UTC）
    .description = 時間帯の最初の時。0 から 23
strategies-condition-time-of-day-param-end-hour = 終了時（UTC）
    .description = 時間帯が終わる時。1 から 24。その時自体は時間帯に含まれません

strategies-condition-liquidity-change-since-entry = エントリー以降の流動性変化
    .description = ポジションを開いてからプールの準備金が指定割合以上減少または増加したときに成立します。エントリー時の流動性が記録されていないポジションでは条件は未確定のままです。
strategies-condition-liquidity-change-since-entry-param-comparison = 比較
    .description = 準備金が減少したか増加したか
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = 減少
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = 増加
strategies-condition-liquidity-change-since-entry-param-change-pct = 変化 %
    .description = エントリー以降のプール準備金の変化

## Shared by every condition

strategies-condition-param-timeframe = 時間足
    .description = ローソク足のサイズ：条件が読む各ローソク足の長さ（未設定の場合はストラテジーの時間足）
strategies-condition-timeframe-option-1m = 1分
strategies-condition-timeframe-option-5m = 5分
strategies-condition-timeframe-option-15m = 15分
strategies-condition-timeframe-option-1h = 1時間
strategies-condition-timeframe-option-4h = 4時間
strategies-condition-timeframe-option-12h = 12時間
strategies-condition-timeframe-option-1d = 1日

## Condition categories

strategies-condition-category-price-analysis = 価格分析
strategies-condition-category-candle-patterns = ローソク足パターン
strategies-condition-category-technical-indicators = テクニカル指標
strategies-condition-category-market-context = マーケット状況
strategies-condition-category-position-performance = ポジションとパフォーマンス
strategies-condition-category-volume-analysis = 出来高分析

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = { $field } パラメーターがありません
strategies-error-parameter-type = { $field } パラメーターは { $expected } である必要があります
strategies-error-invalid-value = 「{ $value }」は { $field } として無効です
strategies-error-missing-data = { $data } を利用できません
strategies-error-no-candle-data = { $timeframe } の時間足にローソク足データがありません
strategies-error-insufficient-history = { $indicator } の履歴が不足しています: 利用可能 { $available }秒、必要 { $required }秒
strategies-error-insufficient-candles = { $indicator } のローソク足が不足しています: 現在 { $available }、必要 { $required }
strategies-error-stale-candle-data = { $timeframe } のローソク足データが古くなっています: 経過 { $age }秒が上限 { $max }秒を超えています
strategies-error-stale-market-data = { $window } の取引数が古くなっています: 経過 { $age }秒が上限 { $max }秒を超えています
strategies-error-unusable-data = { $field } の値「{ $value }」は使用できません
strategies-error-invalid-rule-tree = ルールツリーが無効です: { $reason }
strategies-error-evaluation-timeout = ストラテジーの評価が { $timeout }ミリ秒でタイムアウトしました
strategies-error-condition-not-applicable = { $condition } は { $strategy } 戦略では使用できません
strategies-error-invalid-rules = ルールを読み取れませんでした: { $reason }

# Parameter names

strategies-error-field-average-volume = 平均出来高
strategies-error-field-candle-open = ローソク足の始値
strategies-error-field-comparison = 比較
strategies-error-field-condition-type = 条件の種類
strategies-error-field-confirmation = 確認
strategies-error-field-count = 本数
strategies-error-field-current-price = 現在価格
strategies-error-field-direction = 方向
strategies-error-field-distance = 乖離
strategies-error-field-hours = 時間
strategies-error-field-lookback = 参照期間
strategies-error-field-minimum-change = 最小変動
strategies-error-field-multiplier = 倍率
strategies-error-field-pattern = パターン
strategies-error-field-percentage = パーセンテージ
strategies-error-field-period = 期間
strategies-error-field-position = 位置
strategies-error-field-threshold = しきい値
strategies-error-field-time-unit = 時間の単位
strategies-error-field-time-value = 時間の値
strategies-error-field-timeframe = 時間足
strategies-error-field-arm-after-profit = 利益後の有効化
strategies-error-field-drop-percentage = 下落率
strategies-error-field-entry-price = エントリー価格
strategies-error-field-fast-period = 短期の期間
strategies-error-field-minimum-buy-share = 最低買い比率
strategies-error-field-minimum-transactions = 最低取引数
strategies-error-field-moving-average-type = 平均の種類
strategies-error-field-moving-average = 移動平均
strategies-error-field-reference-candle-close = 基準ローソク足の終値
strategies-error-field-peak-price = 最高価格
strategies-error-field-slow-period = 長期の期間
strategies-error-field-window = 集計期間
strategies-error-field-within-candles = クロス対象の本数
strategies-error-field-change = 変化
strategies-error-field-end-hour = 終了時
strategies-error-field-entry-liquidity = エントリー時の流動性
strategies-error-field-giveback = 利益の戻し
strategies-error-field-highest-high = 最高値
strategies-error-field-long-window-transactions = 長い期間の取引数
strategies-error-field-maximum-drop = 最大下落
strategies-error-field-minimum-drop = 最小下落
strategies-error-field-multiple = 倍率
strategies-error-field-pool-liquidity = プール流動性
strategies-error-field-ratio = 比率
strategies-error-field-remaining-position-value = ポジションの残存価値
strategies-error-field-signal = シグナル
strategies-error-field-squeeze-width = スクイーズ幅
strategies-error-field-standard-deviations = 標準偏差
strategies-error-field-start-hour = 開始時
strategies-error-field-subject = 対象
strategies-error-field-token-creation-time = トークン作成時刻
strategies-error-field-total-volume = 総出来高
strategies-error-field-trade-size = 取引サイズ
strategies-error-field-vwap = VWAP

# Expected parameter types

strategies-error-expected-boolean = 真偽値
strategies-error-expected-number = 数値
strategies-error-expected-string = 文字列

# Missing context data

strategies-error-data-current-price = 現在価格
strategies-error-data-liquidity-data = 流動性データ
strategies-error-data-market-data = マーケットデータ
strategies-error-data-ohlcv-data = OHLCV データ
strategies-error-data-position-data = ポジションデータ
strategies-error-data-peak-price = 最高価格
strategies-error-data-transaction-counts = 取引数
strategies-error-data-entry-liquidity = エントリー時の流動性
strategies-error-data-entry-pool-liquidity = エントリー時のプールの流動性
strategies-error-data-real-pool-reserves = プールの実際の流動性
strategies-error-data-remaining-position-value = ポジションの残存価値
strategies-error-data-token-creation-time = トークン作成時刻
strategies-error-data-trade-size = 取引サイズ

# Indicators

strategies-error-indicator-consecutive-candles = 連続ローソク足
strategies-error-indicator-moving-average = 移動平均
strategies-error-indicator-price-breakout = 価格ブレイクアウト
strategies-error-indicator-price-change-lookback = 価格変動の参照期間
strategies-error-indicator-volume-spike = 出来高急増
strategies-error-indicator-moving-average-cross = 移動平均のクロス
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = ボリンジャーバンド
strategies-error-indicator-pullback-from-high = 高値からの押し目
strategies-error-indicator-volatility-range = ボラティリティレンジ
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = ブランチノードに条件がありません
strategies-error-rule-branch-node-missing-operator = ブランチノードに演算子がありません
strategies-error-rule-branch-node-must-have-at-least-one-child = ブランチノードには子を1つ以上持たせる必要があります
strategies-error-rule-invalid-rule-tree-structure = ルールツリーの構造が無効です
strategies-error-rule-leaf-node-missing-condition = リーフノードに条件がありません
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 演算子の子はちょうど1つである必要があります
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = グループには有効な条件が 1 つ以上必要です
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT 演算子の子ノードを無効にすることはできません
strategies-error-rule-rules-nest-deeper-than-four-levels = ルールのネストは 4 階層までです
strategies-error-rule-the-root-group-cannot-be-switched-off = ルートグループは無効にできません

strategies-readiness-priced = すべてのポジションに価格あり
strategies-readiness-priced-ok = オープン中のペーパーポジションすべてにプール価格があります
strategies-readiness-priced-missing =
    { $count ->
       *[other] プール価格のないオープン中のペーパーポジションが { $count }件あります
    }
strategies-readiness-runtime = ライブ取引が利用可能
strategies-readiness-runtime-ok = セットアップと安全ゲートによりライブ取引が許可されています
