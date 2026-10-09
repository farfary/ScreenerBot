# Filter rejection reasons. Message ids derive from the stored rejection codes
# (src/filtering/sources/rejection.rs); rows hold codes, never this text.

filtering-reject-no-decimals = データベースに小数桁数がありません
filtering-reject-token-too-new = トークンが新しすぎます
filtering-reject-cooldown-filtered = クールダウンにより除外
filtering-reject-dex-data-missing = { -dexscreener } のデータがありません
filtering-reject-gecko-data-missing = { -geckoterminal } のデータがありません
filtering-reject-rug-data-missing = { -rugcheck } のデータがありません
filtering-reject-onchain-numeric-symbol = 数字のみのシンボル（詐欺）
filtering-reject-onchain-empty-symbol = シンボルが空（詐欺）
filtering-reject-onchain-suspicious-symbol = 不審なシンボル（詐欺）
filtering-reject-onchain-known-scam-authority = 既知の詐欺の権限
filtering-reject-onchain-immutable-with-freeze = イミュータブル + フリーズ権限（詐欺）
filtering-reject-onchain-high-risk-score = オンチェーンのリスクスコアが高い
filtering-reject-dex-empty-name = 名前が空
filtering-reject-dex-empty-symbol = シンボルが空
filtering-reject-dex-empty-logo = ロゴ URL が空
filtering-reject-dex-empty-website = ウェブサイト URL が空
filtering-reject-dex-txn-5m = 5m のトランザクションが少ない
filtering-reject-dex-txn-1h = 1h のトランザクションが少ない
filtering-reject-dex-zero-liq = 流動性がゼロ
filtering-reject-dex-liq-low = 流動性が低すぎます
filtering-reject-dex-liq-high = 流動性が高すぎます
filtering-reject-dex-mcap-low = 時価総額が低すぎます
filtering-reject-dex-mcap-high = 時価総額が高すぎます
filtering-reject-dex-vol-low = 出来高が少なすぎます
filtering-reject-dex-vol-missing = 出来高がありません
filtering-reject-dex-fdv-low = FDV が低すぎます
filtering-reject-dex-fdv-high = FDV が高すぎます
filtering-reject-dex-vol5m-low = 5m の出来高が少なすぎます
filtering-reject-dex-vol5m-missing = 5m の出来高がありません
filtering-reject-dex-vol1h-low = 1h の出来高が少なすぎます
filtering-reject-dex-vol1h-missing = 1h の出来高がありません
filtering-reject-dex-vol6h-low = 6h の出来高が少なすぎます
filtering-reject-dex-vol6h-missing = 6h の出来高がありません
filtering-reject-dex-price-change-5m-low = 5m の価格変動が小さすぎます
filtering-reject-dex-price-change-5m-high = 5m の価格変動が大きすぎます
filtering-reject-dex-price-change-low = 価格変動が小さすぎます
filtering-reject-dex-price-change-high = 価格変動が大きすぎます
filtering-reject-dex-price-change-6h-low = 6h の価格変動が小さすぎます
filtering-reject-dex-price-change-6h-high = 6h の価格変動が大きすぎます
filtering-reject-dex-price-change-24h-low = 24h の価格変動が小さすぎます
filtering-reject-dex-price-change-24h-high = 24h の価格変動が大きすぎます
filtering-reject-gecko-liq-low = 流動性が低すぎます
filtering-reject-gecko-liq-high = 流動性が高すぎます
filtering-reject-gecko-mcap-low = 時価総額が低すぎます
filtering-reject-gecko-mcap-high = 時価総額が高すぎます
filtering-reject-gecko-vol5m-low = 5m の出来高が少なすぎます
filtering-reject-gecko-vol5m-missing = 5m の出来高がありません
filtering-reject-gecko-vol1h-low = 1h の出来高が少なすぎます
filtering-reject-gecko-vol1h-missing = 1h の出来高がありません
filtering-reject-gecko-vol24h-low = 24h の出来高が少なすぎます
filtering-reject-gecko-vol24h-missing = 24h の出来高がありません
filtering-reject-gecko-price-change-5m-low = 5m の価格変動が小さすぎます
filtering-reject-gecko-price-change-5m-high = 5m の価格変動が大きすぎます
filtering-reject-gecko-price-change-1h-low = 1h の価格変動が小さすぎます
filtering-reject-gecko-price-change-1h-high = 1h の価格変動が大きすぎます
filtering-reject-gecko-price-change-24h-low = 24h の価格変動が小さすぎます
filtering-reject-gecko-price-change-24h-high = 24h の価格変動が大きすぎます
filtering-reject-gecko-pool-count-low = プール数が少なすぎます
filtering-reject-gecko-pool-count-high = プール数が多すぎます
filtering-reject-gecko-pool-count-missing = プール数がありません
filtering-reject-gecko-reserve-low = リザーブが少なすぎます
filtering-reject-gecko-reserve-missing = リザーブがありません
filtering-reject-rug-rugged = ラグプルされたトークン
filtering-reject-rug-score = リスクスコアが高すぎます
filtering-reject-rug-level-danger = 危険リスクレベル
filtering-reject-rug-mint-authority = ミント権限が存在します
filtering-reject-rug-freeze-authority = フリーズ権限が存在します
filtering-reject-rug-top-holder = 最大ホルダーの割合が高すぎます
filtering-reject-rug-top3-holders = 上位3ホルダーの割合が高すぎます
filtering-reject-rug-min-holders = ホルダーが足りません
filtering-reject-rug-insider-count = インサイダーホルダーが多すぎます
filtering-reject-rug-insider-pct = インサイダーの割合が高すぎます
filtering-reject-rug-creator-pct = 作成者の残高が多すぎます
filtering-reject-rug-transfer-fee-present = 送金手数料があります
filtering-reject-rug-transfer-fee-high = 送金手数料が高すぎます
filtering-reject-rug-graph-insiders = グラフ上のインサイダーが多すぎます
filtering-reject-rug-lp-providers-low = LP プロバイダーが少なすぎます
filtering-reject-rug-lp-providers-missing = LP プロバイダーがありません
filtering-reject-rug-lp-lock-low = LP ロックが少なすぎます
filtering-reject-rug-lp-lock-missing = LP ロックがありません
filtering-reject-llm-analysis-rejected = LLM 分析で除外: { $reason }（信頼度 { $confidence }%、{ $provider }）
filtering-reject-llm-analysis-rejected-generic = LLM 分析で除外
filtering-reject-unknown = { $code }

# Codes no longer emitted; they appear only in stored rows and keep their wording.
filtering-reject-dex-fdv-missing = FDV がありません
filtering-reject-dex-price-change-5m-missing = 5m の価格変動がありません
filtering-reject-dex-price-change-missing = 価格変動がありません
filtering-reject-dex-price-change-6h-missing = 6h の価格変動がありません
filtering-reject-dex-price-change-24h-missing = 24h の価格変動がありません
filtering-reject-gecko-liq-missing = 流動性がありません
filtering-reject-gecko-mcap-missing = 時価総額がありません
filtering-reject-gecko-price-change-5m-missing = 5m の価格変動がありません
filtering-reject-gecko-price-change-1h-missing = 1h の価格変動がありません
filtering-reject-gecko-price-change-24h-missing = 24h の価格変動がありません
filtering-reject-rug-transfer-fee-missing = 送金手数料のデータがありません

# Rejection categories used to group reasons.
filtering-reject-category-security = セキュリティ上の問題
filtering-reject-category-distribution = ホルダー分布
filtering-reject-category-liquidity-lock = LP ロックの問題
filtering-reject-category-fees = 送金手数料
filtering-reject-category-liquidity = 流動性
filtering-reject-category-volume = 取引出来高
filtering-reject-category-market-cap = 時価総額/FDV
filtering-reject-category-price-action = 価格変動
filtering-reject-category-activity = 取引アクティビティ
filtering-reject-category-data-quality = データ欠損
filtering-reject-category-timing = タイミングフィルター
filtering-reject-category-market = マーケットデータ
filtering-reject-category-other = その他

# Filtering page: sub-tabs, sources, status, analytics, explorer and configuration.

## Sub-tabs and sources. Source ids are FilterSource::as_str plus the `meta` settings tab.

filtering-tab-status = ステータス
filtering-tab-analytics = 分析
filtering-tab-explorer = エクスプローラー
filtering-source-core = コア
filtering-source-onchain = オンチェーン
filtering-source-dexscreener = { -dexscreener }
filtering-source-geckoterminal = { -geckoterminal }
filtering-source-rugcheck = { -rugcheck }
filtering-source-llm-analysis = LLM 分析

## Time range

filtering-range-1h = 1H
filtering-range-6h = 6H
filtering-range-24h = 24H
filtering-range-7d = 7D
filtering-range-all = すべて
filtering-range-all-time = 全期間
filtering-range-custom = カスタム
filtering-range-now = 現在
# $start and $end are formatted moments, or the open-ended markers.
filtering-range-span = { $start } → { $end }
# $min and $max are the two ends of a value range.
filtering-range-bounds = { $min } – { $max }

## Footer status line

filtering-footer-saving = 変更を保存中...
filtering-footer-refreshing = スナップショットを更新中...
filtering-footer-unsaved = 未保存の変更があります
# $time is a relative time such as "5m ago".
filtering-footer-last-saved = 最終保存 { $time }
filtering-footer-in-sync = 設定は同期済みです

## Info bar and status metrics

filtering-info-total = 合計
filtering-info-priced = 価格あり
filtering-info-passed = 通過
filtering-info-positions = ポジション
filtering-info-blacklisted = ブラックリスト
filtering-info-cache = キャッシュ
# A count followed by its share of the total, e.g. "120 (4.0%)".
filtering-count-share = { $count }（{ $share }）
filtering-refresh-building = 構築中…
filtering-refresh-never = なし

filtering-status-loading = 統計を読み込み中...
filtering-status-total = トークン合計
filtering-status-total-detail = フィルタリングキャッシュ内
filtering-status-total-detail-building = スナップショット構築中。件数は次回の更新で反映されます
filtering-status-priced = 価格あり
filtering-status-priced-detail = { $share } に価格あり
filtering-status-passed = フィルター通過
filtering-status-passed-detail = { $share } が通過
filtering-status-positions = オープンポジション
filtering-status-positions-detail = アクティブな取引
filtering-status-blacklisted = ブラックリスト
filtering-status-blacklisted-detail = フラグ付きトークン
filtering-status-ohlcv = OHLCV あり
filtering-status-ohlcv-detail = 履歴データ
filtering-status-refresh = 最終更新
filtering-status-refresh-building = 最初のスナップショットを作成中
filtering-status-refresh-none = まだ更新されていません
filtering-status-no-rejections = 除外データはありません

## Analytics

filtering-analytics-loading = { $range } の分析を読み込み中…
filtering-analytics-scanned = スキャン合計
# $time is a relative time such as "5m ago".
filtering-analytics-updated = { $time }に更新
filtering-analytics-passed = 通過したトークン
filtering-analytics-pass-rate = 通過率 <strong>{ $share }</strong>
filtering-analytics-rejected = 除外されたトークン
filtering-analytics-rejection-rate = 除外率 <strong>{ $share }</strong>
filtering-analytics-by-category = カテゴリー別の除外
filtering-analytics-by-source = ソース別の除外
filtering-analytics-no-category = カテゴリーデータはありません
filtering-analytics-no-source = ソースデータはありません
filtering-analytics-top-reasons = 除外理由の上位
filtering-analytics-no-data = データがありません
filtering-analytics-column-reason = 理由
filtering-analytics-column-category = カテゴリー
filtering-analytics-column-count = 件数
filtering-analytics-column-share = %
filtering-analytics-column-impact = 影響
# $amount is the formatted count, $count selects the plural.
filtering-tokens-count =
    { $count ->
       *[other] { $amount }トークン
    }

## Explorer

filtering-explorer-top-reasons = 上位の理由
filtering-explorer-recent = 最近の除外
filtering-explorer-none = データなし
filtering-explorer-none-recent = 最近の除外なし
filtering-explorer-search =
    .placeholder = 理由を検索...
filtering-explorer-overview = 概要
filtering-explorer-no-match = 一致する理由はありません
filtering-explorer-column-token = トークン
filtering-explorer-column-source = ソース
filtering-explorer-column-time = 時刻
filtering-explorer-page = { $page } ページ
filtering-explorer-no-results = 結果なし
filtering-explorer-empty = トークンが見つかりません
filtering-explorer-empty-filtered = フィルターに一致するトークンが見つかりません
filtering-explorer-load-failed = トークンを読み込めませんでした

## Configuration panels

filtering-config-loading = 設定を読み込み中…
# $query is the text typed in the filter box.
filtering-config-no-match = 「{ $query }」に一致するパラメーターはありません
filtering-config-no-parameters = このソースにはパラメーターがありません
# $source is the source name.
filtering-source-off = { $source } のフィルタリングはオフです。これらのパラメーターは評価されません。
filtering-toolbar-filter =
    .placeholder = パラメーターを絞り込み
    .aria-label = パラメーターを絞り込み
filtering-toolbar-clear =
    .aria-label = フィルターをクリア
# $count selects the plural, $amount is the number shown.
filtering-parameter-count =
    { $count ->
       *[other] パラメーター { $amount }個
    }
# $count is the total and selects the plural.
filtering-parameter-count-filtered =
    { $count ->
       *[other] パラメーター { $total }個中 { $visible }個
    }
filtering-group-enable =
    .aria-label = { $group } のチェックを有効化
filtering-field-min = 最小
filtering-field-max = 最大
# $label is the parameter name.
filtering-field-min-aria =
    .aria-label = { $label } の最小値
filtering-field-max-aria =
    .aria-label = { $label } の最大値
# $default is the shipped value, $label the parameter name.
filtering-field-reset =
    .title = デフォルトに戻す（{ $default }）
    .aria-label = { $label } をデフォルトに戻す

## Toasts. A message value is the title; `.message` is the body.

filtering-toast-saved = 設定を保存しました
    .message = フィルタリング設定を保存し、スナップショットを更新しました
filtering-toast-save-failed = 保存失敗
    .message = フィルタリング設定を保存できませんでした
filtering-toast-reset = 変更をリセットしました
    .message = 設定を最後に保存した状態に戻しました
filtering-toast-refresh-failed = 更新失敗
    .message = フィルタリングのスナップショットを更新できませんでした
filtering-toast-exported = 設定をエクスポートしました
    .message = フィルタリング設定をファイルに保存しました
filtering-toast-imported = 設定をインポートしました
    .message = フィルタリング設定をファイルから読み込みました
filtering-toast-import-failed = インポート失敗
    .message = 設定をインポートできませんでした。ファイル形式が無効です
filtering-toast-load-failed = 読み込み失敗
    .message = フィルタリング設定を読み込めませんでした
filtering-toast-range-missing = 開始日と終了日の両方を選択してください
filtering-toast-range-order = 開始時刻は終了時刻より前にしてください
filtering-toast-range-future = 終了時刻は未来にできません
