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
strategies-editor-enabled =
    .aria-label = ストラテジーを有効化
    .title = ストラテジーを有効化
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
strategies-summary-period-seconds = 参照期間: { $amount }秒
strategies-summary-period-minutes = 参照期間: { $amount }分
strategies-summary-period-hours = 参照期間: { $amount }時間

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

## Condition catalog

strategies-catalog-search =
    .placeholder = 条件を検索...
strategies-catalog-search-clear =
    .aria-label = 検索をクリア
strategies-catalog-fold-all = すべて折りたたむ
strategies-catalog-unfold-all = すべて展開
strategies-catalog-no-description = 説明はありません

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
strategies-toast-enabled = ストラテジーを有効にしました
    .message = 「{ $name }」を有効にしました
strategies-toast-disabled = ストラテジーを無効にしました
    .message = 「{ $name }」を無効にしました
strategies-toast-toggle-failed = 切り替え失敗
    .message = ストラテジーのステータスを更新できませんでした
strategies-toast-load-failed = 読み込み失敗
    .message = サーバーからストラテジーを読み込めませんでした
strategies-toast-load-strategy-failed = ストラテジーを読み込めませんでした
strategies-toast-no-strategy = ストラテジーが作成されていません
    .message = 条件を1つ以上追加するか、「新規ストラテジー」をクリックして先にストラテジーを作成してください
strategies-toast-no-conditions-save = 条件がありません
    .message = 保存する前に、ストラテジーに条件を1つ以上追加してください
strategies-toast-name-required = 名前が必要です
    .message = 保存する前にストラテジー名を入力してください
strategies-toast-saved = ストラテジーを保存しました
    .message = 「{ $name }」を保存しました
strategies-toast-save-failed = 保存失敗
    .message = ストラテジーをデータベースに保存できませんでした
strategies-toast-no-strategy-validate = 検証するストラテジーがありません
strategies-toast-no-conditions-validate = 条件がありません
    .message = 検証する前に条件を1つ以上追加してください
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
strategies-toast-condition-added = 条件を追加しました
    .message = { $name } をストラテジーに追加しました


## Conditions

strategies-condition-candle-size = ローソク足サイズパターン
    .description = 特定のローソク足パターン（大きな実体、小さな実体（十字線）、長いヒゲ）を検出します
strategies-condition-candle-size-param-pattern = パターンの種類
    .description = 検出するローソク足パターン
strategies-condition-candle-size-param-pattern-option-large-body = 大きな実体（強い値動き）
strategies-condition-candle-size-param-pattern-option-small-body = 小さな実体（十字線/迷い）
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 長い上ヒゲ（反落）
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 長い下ヒゲ（サポート）
strategies-condition-candle-size-param-threshold = サイズしきい値 %
    .description = パターン検出のしきい値（パーセント）

strategies-condition-consecutive-candles = 連続ローソク足
    .description = 最小サイズフィルター付きで、連続する陽線（強気）または陰線（弱気）を検出します
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
    .description = ポジションの保有時間をチェックします（エグジットストラテジー用: 時間ベースのエグジット）
strategies-condition-position-holding-time-param-hours = 時間しきい値（時間）
    .description = ポジションをオープンしてからの経過時間（時間単位）
strategies-condition-position-holding-time-param-comparison = 比較
    .description = ポジションの経過時間としきい値の比較方法
strategies-condition-position-holding-time-param-comparison-option-greater-than = より長い（>）
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 以上（≥）
strategies-condition-position-holding-time-param-comparison-option-less-than = より短い（{ "<" }）
strategies-condition-position-holding-time-param-comparison-option-less-equal = 以下（≤）

strategies-condition-price-breakout = 価格ブレイクアウト
    .description = 価格がレジスタンス（期間高値）を上抜け、またはサポート（期間安値）を下抜けたことを検出します
strategies-condition-price-breakout-param-lookback = 参照期間
    .description = サポート/レジスタンスを求めるためのローソク足の本数
strategies-condition-price-breakout-param-direction = ブレイクアウトの方向
    .description = ブレイクアウトの方向
strategies-condition-price-breakout-param-direction-option-upward = 上方向（レジスタンス突破）
strategies-condition-price-breakout-param-direction-option-downward = 下方向（サポート割れ）
strategies-condition-price-breakout-param-confirmation = 確認 %
    .description = ブレイクアウトを確認するために水準をどれだけ超える必要があるか（だましを回避）

strategies-condition-price-change-percent = 価格変動率 %
    .description = 一定期間内に価格がしきい値の割合だけ変動したかをチェックします
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
    .description = 価格が単純移動平均の上、下、または範囲内にあるかをチェックします
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
    .description = 平均出来高と比べた出来高の急増を検出します（関心の高まりを示します）
strategies-condition-volume-spike-param-lookback = 参照期間
    .description = 平均出来高を計算するローソク足の本数
strategies-condition-volume-spike-param-multiplier = 出来高倍率
    .description = 平均の何倍か（例: 2.0 = 平均の 200%）

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
strategies-error-invalid-rule-tree = ルールツリーが無効です: { $reason }
strategies-error-evaluation-timeout = ストラテジーの評価が { $timeout }ミリ秒でタイムアウトしました
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

# Indicators

strategies-error-indicator-consecutive-candles = 連続ローソク足
strategies-error-indicator-moving-average = 移動平均
strategies-error-indicator-price-breakout = 価格ブレイクアウト
strategies-error-indicator-price-change-lookback = 価格変動の参照期間
strategies-error-indicator-volume-spike = 出来高急増

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = ブランチノードに条件がありません
strategies-error-rule-branch-node-missing-operator = ブランチノードに演算子がありません
strategies-error-rule-branch-node-must-have-at-least-one-child = ブランチノードには子を1つ以上持たせる必要があります
strategies-error-rule-invalid-rule-tree-structure = ルールツリーの構造が無効です
strategies-error-rule-leaf-node-missing-condition = リーフノードに条件がありません
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 演算子の子はちょうど1つである必要があります
