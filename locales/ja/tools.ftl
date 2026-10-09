# Tools page: the shell, the token tools, the trading tools, the wallet tools and the
# multi-wallet tools.

## Shell (pages/tools.html, scripts/pages/tools.js)

tools-sidebar-title = ツール
tools-category-wallet = ウォレット
tools-category-token = トークン
tools-category-single-token = 単一トークン
tools-category-utilities = ユーティリティ
tools-sidebar-hint = ツールを選択して開始します
tools-help-button =
    .aria-label = このツールのヘルプを表示
tools-help-unavailable = ヘルプはありません
tools-placeholder-title = ツールを選択
tools-placeholder-subtitle = サイドバーからツールを選んで開始します
tools-placeholder-hint-wallets = ウォレットツールで Solana ウォレットを管理できます
tools-placeholder-hint-secure = すべての操作は安全に保護され、可能な限り元に戻せます

tools-status-ready = 利用可能
tools-status-coming = 近日公開
tools-status-beta = ベータ版 - 不具合の可能性があります
tools-status-disabled = 現在無効です
tools-status-badge-coming = 近日公開
tools-status-badge-beta = ベータ
tools-toast-coming-soon = このツールは近日公開予定です
tools-toast-disabled = このツールは現在無効です
tools-setup-gate-title = このツールにはウォレットが必要です

## Tool names. `-title` names the tool in the navigation and the header, `-summary` is the
## navigation line, `-description` is the header line. Ids are the tool ids of the registry.

tools-tool-wallet-cleanup-title = ウォレットクリーンアップ
tools-tool-wallet-cleanup-summary = 空の ATA をクローズ
tools-tool-wallet-cleanup-description = 空の Associated Token Account をクローズして { -sol } を回収します
tools-tool-burn-tokens-title = トークンのバーン
tools-tool-burn-tokens-summary = トークンを完全に破棄
tools-tool-burn-tokens-description = ウォレット内のトークンを完全に破棄します
tools-tool-token-analyzer-title = トークンアナライザー
tools-tool-token-analyzer-summary = トークンの詳細分析
tools-tool-token-analyzer-description = あらゆる Solana トークンを多角的に詳しく分析します
tools-tool-create-token-title = トークン作成
tools-tool-create-token-summary = 新しい SPL トークンをデプロイ
tools-tool-create-token-description = Solana に新しい SPL トークンをデプロイします
tools-tool-trade-watcher-title = トレードウォッチャー
tools-tool-trade-watcher-summary = 取引を監視して自動実行
tools-tool-trade-watcher-description = トークンの取引を監視し、自動で購入 / 売却を実行します
tools-tool-token-watch-title = ホルダーウォッチ
tools-tool-token-watch-summary = 新規ホルダーを追跡
tools-tool-token-watch-description = 新規トークンホルダーをリアルタイムで追跡・監視します
tools-tool-buy-multi-wallets-title = マルチ購入
tools-tool-buy-multi-wallets-summary = 複数ウォレットで連携購入
tools-tool-buy-multi-wallets-description = 数量をランダム化して、複数のウォレットで連携した購入注文を実行します
tools-tool-sell-multi-wallets-title = マルチ売却
tools-tool-sell-multi-wallets-summary = 複数ウォレットで連携売却
tools-tool-sell-multi-wallets-description = { -sol } の統合を伴う連携売却注文を、複数のウォレットで実行します
tools-tool-wallet-consolidation-title = ウォレット統合
tools-tool-wallet-consolidation-nav-title = 統合
tools-tool-wallet-consolidation-summary = ウォレットの資金を統合
tools-tool-wallet-consolidation-description = サブウォレットの { -sol } とトークンをメインウォレットに集約します
tools-tool-airdrop-checker-title = エアドロップチェッカー
tools-tool-airdrop-checker-summary = 保留中のエアドロップを確認
tools-tool-airdrop-checker-description = 保留中のエアドロップと請求可能な報酬を確認します
tools-tool-wallet-generator-title = ウォレットジェネレーター
tools-tool-wallet-generator-summary = 新しいキーペアを生成
tools-tool-wallet-generator-description = 新しい Solana キーペアを安全に生成します

## Shared by the tools

tools-validation-mint-required = トークンのミントアドレスを入力してください
tools-validation-mint-format = トークンのミントアドレスの形式が正しくありません
tools-validation-mint-invalid = 有効なミントアドレスを入力してください

## Create token (scripts/pages/tools/token_tools.js)

tools-create-token-details-title = トークンの詳細
tools-create-token-name-label = トークン名
tools-create-token-name-input =
    .placeholder = My Token
tools-create-token-symbol-label = シンボル
tools-create-token-symbol-input =
    .placeholder = MTK
tools-create-token-decimals-label = 小数桁数
tools-create-token-supply-label = 初期供給量
tools-create-token-description-label = 説明
tools-create-token-description-input =
    .placeholder = トークンの説明...
tools-create-token-image-title = トークン画像
tools-create-token-image-drop = ここに画像をドロップ、またはクリックしてアップロード
tools-create-token-image-hint = 推奨: 512x512 の PNG
tools-create-token-action-preview = プレビュー
tools-create-token-action-create = トークンを作成

## Holder watch (scripts/pages/tools/token_tools.js)

tools-holder-watch-loading = 設定を読み込み中...
tools-holder-watch-saved = ホルダーウォッチの設定を保存しました
tools-holder-watch-save-failed = 設定を保存できませんでした
tools-holder-watch-save-error = 設定の保存中にエラーが発生しました
tools-holder-watch-settings-title = ホルダーウォッチの設定
tools-holder-watch-enabled-label = ホルダーウォッチを有効化
tools-holder-watch-interval-label = チェック間隔
tools-holder-watch-interval-hint = ホルダー数を確認する頻度（10～3600秒）
tools-holder-watch-max-tokens-label = 最大ウォッチ数
tools-holder-watch-max-tokens-hint = 同時に監視できるトークンの最大数
tools-holder-watch-notify-new-label = 新規ホルダーを通知
tools-holder-watch-notify-drop-label = ホルダー減少を通知
tools-holder-watch-min-change-label = ホルダー最小変動数
tools-holder-watch-min-change-hint = 通知を発生させるホルダー数の最小変動
tools-holder-watch-drop-percent-label = ホルダー減少のしきい値
tools-holder-watch-drop-percent-hint = アラートを発生させる減少率
tools-holder-watch-action-save = 設定を保存
tools-holder-watch-tokens-title = ウォッチ中のトークン
tools-holder-watch-token-input =
    .placeholder = トークンのミントアドレスを入力...
tools-holder-watch-empty = ウォッチ中のトークンはありません
tools-holder-watch-empty-hint = 上でトークンのミントアドレスを追加すると監視を開始します
tools-holder-watch-coming-soon = トークンウォッチ機能は近日公開予定です

## Token analyzer (scripts/pages/tools/token_tools.js)

tools-analyzer-input-title = トークンを分析
tools-analyzer-mint-input =
    .placeholder = トークンのミントアドレスを貼り付け...
tools-analyzer-action-analyze = 分析
tools-analyzer-action-analyzing = 分析中...
tools-analyzer-action-copy-report = レポートをコピー
tools-analyzer-loading = トークンを分析中...
tools-analyzer-failed = トークンを分析できませんでした
tools-analyzer-empty = 分析するトークンのミントアドレスを入力してください
tools-analyzer-empty-hint = あらゆる Solana トークンの総合的な分析結果を確認できます
tools-analyzer-tab-overview = 概要
tools-analyzer-tab-security = セキュリティ
tools-analyzer-tab-market = マーケット
tools-analyzer-tab-liquidity = 流動性
tools-analyzer-unknown-token = 不明なトークン

tools-analyzer-favorite-add =
    .title = お気に入りに追加
    .aria-label = お気に入りに追加
tools-analyzer-favorite-already = すでにお気に入りに登録されています
tools-analyzer-favorite-added = { $symbol } をお気に入りに追加しました
tools-analyzer-favorite-failed = お気に入りに追加できませんでした
tools-analyzer-blacklist-add =
    .title = ブラックリストに追加
    .aria-label = ブラックリストに追加
tools-analyzer-blacklist-title = トークンをブラックリストに追加
tools-analyzer-blacklist-message = { $symbol } をブラックリストに追加しますか？ このトークンは取引の対象から除外されます。
tools-analyzer-blacklist-confirm = ブラックリストに追加
tools-analyzer-blacklisted = ブラックリスト登録済み
tools-analyzer-blacklist-done = { $symbol } をブラックリストに追加しました
tools-analyzer-blacklist-failed = トークンをブラックリストに追加できませんでした

tools-analyzer-card-quick-stats = クイック統計
tools-analyzer-card-market-summary = マーケット概要
tools-analyzer-card-token-info = トークン情報
tools-analyzer-stat-holders = ホルダー
tools-analyzer-stat-decimals = 小数桁数
tools-analyzer-stat-safety-score = 安全スコア
tools-analyzer-stat-pools = プール
tools-analyzer-stat-volume-24h = 24h 出来高
tools-analyzer-stat-change-24h = 24h 変動率
tools-analyzer-stat-market-cap = 時価総額
tools-analyzer-stat-liquidity = 流動性
tools-analyzer-info-mint = ミントアドレス
tools-analyzer-info-description = 説明
tools-analyzer-info-supply = 供給量

tools-analyzer-security-empty = セキュリティデータがありません
tools-analyzer-security-empty-hint = このトークンのセキュリティ分析は利用できません
tools-analyzer-card-safety-score = 安全スコア
tools-analyzer-score-good = 良好
tools-analyzer-score-moderate = 普通
tools-analyzer-score-risky = 危険
tools-analyzer-raw-score = 生のリスクスコア: { $score }
tools-analyzer-card-authorities = トークン権限
tools-analyzer-authority-mint = ミント権限
tools-analyzer-authority-freeze = フリーズ権限
tools-analyzer-authority-transfer-fee = 送金手数料
tools-analyzer-authority-mutable = 変更可能
tools-analyzer-authority-active = 有効
tools-analyzer-authority-revoked = 放棄済み
tools-analyzer-card-holder-concentration = ホルダー集中度
tools-analyzer-top-holders = 上位10ホルダーの保有割合
tools-analyzer-risks-title = セキュリティリスク（{ $count }）
tools-analyzer-risks-title-none = セキュリティリスク
tools-analyzer-risks-none = セキュリティリスクは検出されませんでした

tools-analyzer-market-empty = マーケットデータがありません
tools-analyzer-market-empty-hint = このトークンのマーケットデータは利用できません
tools-analyzer-card-price = 現在価格
tools-analyzer-card-price-changes = 価格変動
tools-analyzer-card-volume = 取引出来高
tools-analyzer-card-transactions = 24h トランザクション
tools-analyzer-card-valuation = バリュエーション
tools-analyzer-stat-window-1h = 1h
tools-analyzer-stat-window-6h = 6h
tools-analyzer-stat-window-24h = 24h
tools-analyzer-stat-volume-1h = 1h 出来高
tools-analyzer-stat-volume-6h = 6h 出来高
tools-analyzer-stat-fdv = 完全希薄化後時価総額
tools-analyzer-txn-buys = 購入
tools-analyzer-txn-sells = 売却

tools-analyzer-liquidity-empty = 流動性データがありません
tools-analyzer-liquidity-empty-hint = このトークンのプールが見つかりません
tools-analyzer-card-total-liquidity = 総流動性
tools-analyzer-card-pools = プール
tools-analyzer-active-pools =
    { $count ->
       *[other] アクティブなプール
    }
tools-analyzer-card-pool-details = プールの詳細
tools-analyzer-pools-column-dex = DEX
tools-analyzer-pools-column-liquidity = 流動性（{ -sol }）
tools-analyzer-pools-column-status = ステータス
tools-analyzer-pool-primary = メイン

tools-analyzer-report-empty = コピーする分析結果がありません
tools-analyzer-report-label = 分析レポート
tools-analyzer-report-title = トークン分析レポート
tools-analyzer-report-token = トークン: { $symbol }（{ $name }）
tools-analyzer-report-mint = ミント: { $mint }
tools-analyzer-report-price = 価格: { $sol }
tools-analyzer-report-price-with-usd = 価格: { $sol }（{ $usd }）
tools-analyzer-report-security = セキュリティ:
tools-analyzer-report-safety-score = - 安全スコア: { $score }/100
tools-analyzer-report-mint-authority = - ミント権限: { $state }
tools-analyzer-report-freeze-authority = - フリーズ権限: { $state }
tools-analyzer-report-risks = - リスク: { $count }
tools-analyzer-report-market = マーケット:
tools-analyzer-report-volume = - 24h 出来高: { $amount }
tools-analyzer-report-change = - 24h 変動率: { $amount }
tools-analyzer-report-market-cap = - 時価総額: { $amount }
tools-analyzer-report-liquidity = 流動性:
tools-analyzer-report-liquidity-total = - 合計: { $amount }
tools-analyzer-report-pools = - プール: { $count }
tools-analyzer-report-generated = 生成日時: { $time }

## Trade watcher (scripts/pages/tools/trading_tools.js)

tools-watch-type-buy-on-sell = 売りで購入
tools-watch-type-sell-on-buy = 買いで売却
tools-watch-type-notify = 通知
tools-watch-type-notify-only = 通知のみ

tools-trade-watcher-setup-title = ウォッチの設定
tools-trade-watcher-mint-label = トークンのミントアドレス
tools-trade-watcher-mint-input =
    .placeholder = トークンのミントアドレスを入力...
tools-trade-watcher-action-search-pools = プールを検索
tools-trade-watcher-pool-label = 選択中のプール
tools-trade-watcher-pool-none = プールが選択されていません
tools-trade-watcher-pool-clear =
    .title = プールをクリア
tools-trade-watcher-pool-selected = 選択中のプール: { $dex } { $base }/{ $quote }
tools-trade-watcher-type-label = ウォッチの種類
tools-trade-watcher-type-hint = 売りで購入: 誰かが売却したときに自動で購入します。買いで売却: 誰かが購入したときに自動で売却します。
tools-trade-watcher-trigger-label = トリガー数量
tools-trade-watcher-trigger-hint = アクションを実行する最小取引サイズ（{ -sol }）
tools-trade-watcher-action-amount-label = アクション数量
tools-trade-watcher-action-amount-hint = トリガー時に購入 / 売却する数量
tools-trade-watcher-slippage-label = スリッページ
tools-trade-watcher-slippage-hint = 取引で許容する最大スリッページ
tools-trade-watcher-active-title = アクティブなウォッチ
tools-trade-watcher-empty = アクティブなウォッチはありません
tools-trade-watcher-empty-hint = 上でウォッチを設定し、「ウォッチを開始」をクリックして監視を始めます
tools-trade-watcher-action-start = ウォッチを開始
tools-trade-watcher-action-starting = 開始中...
tools-trade-watcher-action-stop-all = すべて停止
tools-trade-watcher-action-stopping = 停止中...
tools-trade-watcher-started = { $token } のウォッチを開始しました...
tools-trade-watcher-start-failed = ウォッチを開始できませんでした
tools-trade-watcher-stopped = ウォッチを停止しました
tools-trade-watcher-stop-failed = ウォッチを停止できませんでした
tools-trade-watcher-stopped-all = すべてのウォッチを停止しました
tools-trade-watcher-stop-all-failed = ウォッチを停止できませんでした
tools-trade-watcher-load-failed = ウォッチを読み込めませんでした
tools-trade-watcher-column-token = トークン
tools-trade-watcher-column-type = 種類
tools-trade-watcher-column-trigger = トリガー
tools-trade-watcher-column-action = アクション
tools-trade-watcher-column-triggered = 発動
tools-trade-watcher-stop-watch =
    .title = ウォッチを停止

## Results returned by the tools backend. Failures are catalog text; the technical cause
## travels separately as details and is appended by the dashboard.

tools-burn-failure-native-asset = { -sol } はバーンできません
tools-burn-failure-open-position = オープンポジションのトークンはバーンできません
tools-burn-failure-account-not-found = トークンアカウントが見つかりません
tools-burn-failure-zero-balance = トークン残高はすでにゼロです
tools-burn-failure-transaction = トランザクションに失敗しました
tools-burn-warning-open-position = オープンポジションのトークンはバーンできません
tools-burn-warning-closed-position = クローズ済みポジションの残り
tools-burn-warning-worth = 価値 約{ $amount } { -sol }
tools-multi-buy-warning-insufficient = 残高が不足しています。必要額 { $needed } { -sol }、現在 { $have } { -sol }
tools-multi-buy-warning-over-limit = 必要な { -sol } の合計（{ $needed }）が上限（{ $limit }）を超えています
tools-multi-sell-warning-no-wallets = セカンダリウォレットが見つかりません
tools-multi-sell-warning-no-balance = トークン残高のあるウォレットがありません
tools-multi-op-buy-failed = 購入に失敗しました
tools-multi-op-sell-failed = 売却に失敗しました
tools-multi-op-transfer-failed = 送金に失敗しました
tools-multi-op-balance-failed = 残高を取得できませんでした
tools-multi-op-mint-invalid = ミントアドレスが無効です
tools-multi-buy-session-failed = マルチ購入に失敗しました
tools-multi-sell-session-failed = マルチ売却に失敗しました
tools-multi-session-aborted = ユーザーが操作を中止しました

## Shared by the wallet tools (scripts/pages/tools/wallet_tools.js)

tools-wallet-action-scan = ウォレットをスキャン
tools-wallet-action-scanning = スキャン中...
tools-wallet-scan-failed = スキャンに失敗しました: { $reason }
tools-wallet-amount-approx = 約{ $amount }
tools-wallet-amount-gain = +{ $amount }
tools-wallet-selected =
    { $count ->
       *[other] 選択中: { $count }個のウォレット
    }
tools-wallet-transfer-failed = 送金に失敗しました: { $reason }
tools-wallet-cleanup-failed = クリーンアップに失敗しました: { $reason }

## Wallet cleanup (scripts/pages/tools/wallet_tools.js)

tools-wallet-cleanup-results-title = スキャン結果
tools-wallet-cleanup-stat-empty = 空の ATA
tools-wallet-cleanup-stat-reclaimable = 回収可能な { -sol }
tools-wallet-cleanup-stat-failed = 失敗（キャッシュ）
tools-wallet-cleanup-prompt = 「ウォレットをスキャン」をクリックして空の ATA を検出します
tools-wallet-cleanup-prompt-hint = ウォレット内のすべてのトークンアカウントを確認します
tools-wallet-cleanup-action-cleanup = すべてクリーンアップ
tools-wallet-cleanup-action-cleaning = クリーンアップ中...
tools-wallet-cleanup-scanning = ウォレットをスキャン中...
tools-wallet-cleanup-found =
    { $count ->
       *[other] 空の ATA を{ $count }個検出しました（約{ $amount }相当）
    }
tools-wallet-cleanup-clean = 空の ATA はありません。ウォレットはきれいです。
tools-wallet-cleanup-scan-failed = ATA をスキャンできませんでした
tools-wallet-cleanup-done =
    { $count ->
       *[other] ATA を{ $count }個クリーンアップしました
    }

## Burn tokens (scripts/pages/tools/wallet_tools.js)

tools-burn-section-title = トークンのバーン
tools-burn-info-title = バーンとは？
tools-burn-info-body = バーンするとトークンは完全に破棄され、復元できません。バーン後にウォレットクリーンアップを実行すると、空の ATA をクローズして、トークン 1 個あたり約 0.002 { -sol } のレントを回収できます。
tools-burn-stat-total = トークン合計
tools-burn-stat-selected = 選択中
tools-burn-stat-rent = 回収可能なレント
tools-burn-prompt = 「ウォレットをスキャン」をクリックしてトークンを検出します
tools-burn-scanning = ウォレット内のトークンをスキャン中...
tools-burn-scan-failed = トークンをスキャンできませんでした
tools-burn-empty = ウォレットにトークンが見つかりません
tools-burn-action-burn = 選択をバーン（{ $count }）
tools-burn-action-burning = バーン中...
tools-burn-cannot-burn = バーン不可
tools-burn-no-value = 価値なし

tools-burn-category-open-position = オープンポジション
tools-burn-category-has-value = 価値あり
tools-burn-category-closed-position = クローズ済みポジション
tools-burn-category-zero-liquidity = 流動性ゼロ
tools-burn-category-hint-open-position = オープンポジションのトークンはバーンできません
tools-burn-category-hint-has-value = バーンではなく売却を検討してください
tools-burn-category-hint-closed-position = クローズ済み取引の残り
tools-burn-category-hint-zero-liquidity = 市場価値がないため、安全にバーンできます

tools-burn-confirm-title = バーンの確認
tools-burn-confirm-message =
    { $count ->
       *[other] <strong>{ $count }</strong>個のトークンをバーンしますか？
    }
tools-burn-confirm-value = 推定価値の合計: <strong>{ $amount }</strong>
tools-burn-confirm-continue = 続行
tools-burn-final-title = 最終警告
tools-burn-final-headline = この操作は取り消せません！
tools-burn-final-message =
    { $count ->
       *[other] 次の{ $count }個のトークンは完全に破棄され、どのような場合でも復元できません。
    }
tools-burn-final-confirm = はい、バーンします
tools-burn-toast-burned =
    { $total ->
       *[other] { $successful }/{ $total }個のトークンをバーンしました。ウォレットクリーンアップを実行すると約{ $amount }を回収できます
    }
tools-burn-toast-failed =
    { $count ->
       *[other] { $count }個のトークンのバーンに失敗しました
    }
tools-burn-failed = バーンに失敗しました: { $reason }
tools-burn-failures-title =
    { $count ->
       *[other] { $count }個のトークンをバーンできませんでした
    }
tools-burn-failure-unknown = 理由は報告されませんでした

## Airdrop checker (scripts/pages/tools/wallet_tools.js)

tools-airdrop-about-title = 概要
tools-airdrop-about-body = 主要な Solana プロトコルで、保留中のエアドロップ、請求可能な報酬、未請求の割り当てを確認します。
tools-airdrop-list-title = 利用可能なエアドロップ
tools-airdrop-prompt = 「エアドロップを確認」をクリックして請求可能なものをスキャンします
tools-airdrop-action-check = エアドロップを確認
tools-airdrop-action-claim-all = すべて請求

## Wallet generator (scripts/pages/tools/wallet_tools.js)

tools-generator-options-title = ジェネレーターのオプション
tools-generator-warning-title = 秘密鍵は安全に保管してください
tools-generator-warning-body = 生成されたキーペアはローカルで作成され、送信されることはありません。鍵は必ず安全な場所にバックアップしてください。
tools-generator-count-label = ウォレット数
tools-generator-vanity-label = バニティアドレス（特定の文字で始まる）
tools-generator-prefix-label = プレフィックス
tools-generator-prefix-input =
    .placeholder = 例: SOL
tools-generator-prefix-hint = プレフィックスが長いほど、生成に指数関数的に時間がかかります
tools-generator-list-title = 生成されたウォレット
tools-generator-empty = 生成されたウォレットはまだありません
tools-generator-action-generate = 生成
tools-generator-action-generating = 生成中...
tools-generator-count-invalid = 1～10の数値を入力してください
tools-generator-no-keypairs = キーペアが返されませんでした
tools-generator-generated =
    { $count ->
       *[other] { $count }個のウォレットを生成しました
    }
tools-generator-failed = ウォレットを生成できませんでした: { $reason }
tools-generator-copy-public-key =
    .title = 公開鍵をコピー
tools-generator-copy-private-key =
    .title = 秘密鍵をコピー
tools-generator-remove =
    .title = リストから削除
tools-generator-reveal =
    .title = 秘密鍵を表示
tools-generator-public-key-label = 公開鍵:
tools-generator-private-key-label = 秘密鍵:
tools-generator-public-key-name = 公開鍵
tools-generator-private-key-copied = 秘密鍵をコピーしました
tools-generator-private-key-warning = この鍵を持つ人は誰でもウォレットを操作できます
tools-generator-export-empty = エクスポートするウォレットがありません
tools-generator-exported = ウォレットをエクスポートしました。安全に保管してください

## Wallet consolidation (scripts/pages/tools/wallet_tools.js)

tools-consolidation-summary-title = サマリー
tools-consolidation-stat-wallets = サブウォレット
tools-consolidation-stat-native = { -sol } 合計
tools-consolidation-stat-tokens = トークンの種類
tools-consolidation-stat-rent = 回収可能なレント
tools-consolidation-wallets-title = ウォレット
tools-consolidation-loading-wallets = ウォレットを読み込み中...
tools-consolidation-loading-data = ウォレットデータを読み込み中...
tools-consolidation-action-transfer-native = { -sol } を送金
tools-consolidation-action-transfer-tokens = すべてのトークンを送金
tools-consolidation-action-cleanup = ATA をクリーンアップ
tools-consolidation-action-transferring = 送金中...
tools-consolidation-column-name = 名前
tools-consolidation-column-native = { -sol } 残高
tools-consolidation-column-tokens = トークン
tools-consolidation-column-atas = 空の ATA
tools-consolidation-empty = サブウォレットが見つかりません
tools-consolidation-empty-hint = マルチ購入でサブウォレットを作成して開始します
tools-consolidation-load-failed = 読み込みに失敗しました: { $reason }
tools-consolidation-select-prompt = 統合するウォレットを選択してください
tools-consolidation-selection-totals =
    | { $amount } | { $tokens ->
       *[other] { $tokens }個のトークン
    } | { $atas ->
       *[other] 空の ATA { $atas }個
    }
tools-consolidation-transferred-native = { $amount } をメインウォレットに送金しました
tools-consolidation-transferred-tokens =
    { $count ->
       *[other] { $count }個のトークンをメインウォレットに送金しました
    }
tools-consolidation-cleaned =
    { $count ->
       *[other] ATA を{ $count }個クローズし、{ $amount }を回収しました
    }

## Shared by the multi-wallet tools (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-token-title = トークン
tools-multi-mint-label = トークンのミントアドレス
tools-multi-mint-input =
    .placeholder = トークンのミントアドレスを貼り付け...
tools-multi-execution-title = 実行設定
tools-multi-delay-min-label = 最小遅延
tools-unit-native = { -sol }
tools-unit-seconds = 秒
tools-unit-ms = ms
tools-multi-delay-max-label = 最大遅延
tools-multi-concurrency-label = 同時実行数
tools-multi-concurrency-sequential = { $count }（逐次）
tools-multi-concurrency-parallel = { $count }並列
tools-multi-slippage-label = スリッページ
tools-multi-router-label = ルーター
tools-multi-router-auto = 自動（最適ルート）
tools-multi-router-jupiter = { -jupiter }
tools-multi-router-direct = ダイレクトプール
tools-multi-router-raptor = { -raptor }
tools-multi-progress-title = 進行状況
tools-multi-progress-preparing = 準備中...
tools-multi-status-line = { $label }（{ $completed }/{ $total }）
tools-multi-column-wallet = ウォレット
tools-multi-column-route = ルート
tools-multi-column-status = ステータス
tools-multi-op-completed = 完了
tools-multi-op-failed = 失敗
tools-multi-action-stop = 停止
tools-multi-action-loading = 読み込み中...
tools-multi-start-failed = 開始に失敗しました: { $reason }

tools-multi-state-pending = 保留中
tools-multi-state-funding = 資金供給中
tools-multi-state-executing = 実行中
tools-multi-state-consolidating = 統合中
tools-multi-state-completed = 完了
tools-multi-state-failed = 失敗
tools-multi-state-aborted = 中止

## Multi-buy (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-buy-mint-hint = 複数のウォレットで購入するトークン
tools-multi-buy-wallets-title = ウォレット設定
tools-multi-buy-wallet-count-label = ウォレット数
tools-multi-buy-wallet-count-option =
    { $count ->
       *[other] { $count }個のウォレット
    }
tools-multi-buy-wallet-count-hint = 使用するサブウォレットの数
tools-multi-buy-buffer-label = ウォレットごとの { -sol } バッファ
tools-multi-buy-buffer-hint = 手数料用に確保（最小 0.015 { -sol }）
tools-multi-buy-amounts-title = 数量設定
tools-multi-buy-min-label = ウォレットごとの最小 { -sol }
tools-multi-buy-min-hint = 最小購入数量
tools-multi-buy-max-label = ウォレットごとの最大 { -sol }
tools-multi-buy-max-hint = 最大購入数量
tools-multi-buy-limit-label = { -sol } 合計上限（任意）
tools-multi-buy-limit-hint = 支出の最大合計額
tools-multi-buy-preview-title = プレビュー
tools-multi-buy-preview-create = 作成するウォレット
tools-multi-buy-preview-amount = ウォレットごとの数量
tools-multi-buy-preview-range = { $min } - { $max }
tools-multi-buy-preview-total = 必要な { -sol } 合計
tools-multi-buy-preview-balance = メイン残高
tools-multi-buy-action-preview = プレビュー
tools-multi-buy-action-start = マルチ購入を開始
tools-multi-buy-executing = 購入を実行中...
tools-multi-buy-column-spent = { -sol } 使用額
tools-multi-buy-column-tokens = トークン
tools-multi-buy-preview-failed = プレビューに失敗しました: { $reason }
tools-multi-buy-started = マルチ購入を開始しました
tools-multi-buy-stopped = マルチ購入を停止しました
tools-multi-buy-completed = マルチ購入が完了しました。{ $successful }/{ $total }件成功

## Multi-sell (scripts/pages/tools/multi_wallet_tools.js)

tools-multi-sell-mint-hint = トークンアドレスを入力して、保有しているウォレットをスキャンします
tools-multi-sell-action-scan = スキャン
tools-multi-sell-settings-title = 売却設定
tools-multi-sell-percent-label = 売却割合
tools-multi-sell-percent-hint = ウォレットごとに売却するトークンの割合（%）
tools-multi-sell-min-fee-label = 手数料用の最小 { -sol }
tools-multi-sell-min-fee-hint = トランザクション手数料に必要な最小 { -sol }
tools-multi-sell-topup-label = 必要に応じて自動補充
tools-multi-sell-topup-hint = サブウォレットの残高が不足している場合、メインウォレットから { -sol } を送金します
tools-multi-sell-post-title = 売却後のアクション
tools-multi-sell-consolidate-label = { -sol } をメインウォレットに統合
tools-multi-sell-consolidate-hint = サブウォレットのすべての { -sol } をメインウォレットに送金します
tools-multi-sell-close-atas-label = 売却後にトークン ATA をクローズ
tools-multi-sell-close-atas-hint = ATA 1 つあたり約 0.002 { -sol } を回収
tools-multi-sell-wallets-title = トークンを保有するウォレット
tools-multi-sell-empty = このトークンを保有するサブウォレットはありません
tools-multi-sell-column-tokens = トークン
tools-multi-sell-column-native = { -sol } 残高
tools-multi-sell-column-topup = 補充が必要
tools-multi-sell-none-selected = ウォレットが選択されていません
tools-multi-sell-select-required = ウォレットを1つ以上選択してください
tools-multi-sell-action-start = マルチ売却を開始
tools-multi-sell-executing = 売却を実行中...
tools-multi-sell-column-sold = 売却トークン
tools-multi-sell-column-received = { -sol } 受取額
tools-multi-sell-started = マルチ売却を開始しました
tools-multi-sell-stopped = マルチ売却を停止しました
tools-multi-sell-completed = マルチ売却が完了しました。{ $amount } を受け取りました

## Favorites dropdown (scripts/ui/tool_favorites.js)

tools-favorites-title = お気に入り
tools-favorites-saved = 保存済みのお気に入り
tools-favorites-save-current = 現在の設定を保存
tools-favorites-empty = 保存されたお気に入りはまだありません
tools-favorites-no-label = ラベルなし
tools-favorites-uses = { $count }回
tools-favorites-remove = 削除
tools-favorites-loaded = お気に入りを読み込みました: { $name }
tools-favorites-default-name = 設定
tools-favorites-mint-required = 先にトークンのミントアドレスを入力してください
tools-favorites-add-title = お気に入りを追加
tools-favorites-add-message = このお気に入りのラベルを入力してください
tools-favorites-add-placeholder = ラベル（任意）...
tools-favorites-saved-toast = お気に入りに保存しました
tools-favorites-save-failed = お気に入りを保存できませんでした
tools-favorites-remove-title = お気に入りを削除
tools-favorites-remove-message = このお気に入りを削除しますか？
tools-favorites-removed-toast = お気に入りを削除しました
tools-favorites-remove-failed = お気に入りを削除できませんでした
