## Portfolio overview

home-portfolio-title = ポートフォリオ評価額
home-portfolio-change-label = 本日
home-wallet-unread = 残高を取得できません
home-trend-range = 00:00 UTC 以降
home-portfolio-change-incomparable =
    .title = 本日の開始時点または現在、一部のトークン価格が取得できていないため、本日の変動は表示されません。
home-stat-available = 利用可能な { -sol }
home-stat-holdings = トークン保有額
home-stat-open-pnl = オープン損益
home-stat-realized-today = 本日の確定損益

home-holdings-token-count =
    { $count ->
       *[other] { $count }トークン
    }
home-holdings-with-unpriced = { $tokens } · 価格なし { $count }件
home-holdings-unpriced-note =
    { $count ->
       *[other] 価格を取得できない保有トークンが { $count }件あり、合計では 0 として計算されます
    }

## Wallet address and QR code

home-wallet-copy =
    .title = ウォレットアドレスをコピー
    .aria-label = ウォレットアドレスをコピー
home-wallet-qr-open =
    .title = ウォレットの QR コードを表示
    .aria-label = ウォレットの QR コードを表示
home-wallet-qr-popover =
    .aria-label = ウォレットの QR コード
home-wallet-qr-receive = 受け取り
home-wallet-qr-assets = { -sol } と SPL トークン
home-wallet-qr-close =
    .title = 閉じる
    .aria-label = ウォレットの QR コードを閉じる
home-wallet-qr-preparing = QR コードを準備中
home-wallet-qr-unavailable = QR コードを利用できません
home-wallet-qr-image =
    .alt = メインウォレットアドレスの QR コード

## Performance calendar

home-calendar-title = パフォーマンスカレンダー
home-calendar-previous =
    .title = 前月
    .aria-label = 前月
home-calendar-next =
    .title = 翌月
    .aria-label = 翌月
home-calendar-month-pnl = 月間損益
home-calendar-trades = 取引
home-calendar-win-days = 利益日
home-calendar-loss-days = 損失日
home-calendar-best-day = 最高の日
home-calendar-week = 週
home-calendar-day-label = { $date }、{ $pnl }
home-calendar-pop-net-pnl = 純損益
home-calendar-pop-win-rate = 勝率
home-calendar-pop-win-rate-value = { $rate } · { $wins }勝 / { $losses }敗
home-calendar-pop-gross-profit = 総利益
home-calendar-pop-gross-loss = 総損失
home-calendar-pop-end-balance = 期末残高

## Position exposure and market pipeline

home-operations =
    .aria-label = ポートフォリオと市場の状況
home-exposure-title = ポジションエクスポージャー
home-exposure-open = オープン
home-exposure-invested = 投資額
home-exposure-value = 現在価値
home-exposure-position = ポジション
home-exposure-held = 保有時間
home-exposure-empty = オープンポジションはありません
home-exposure-avg-size = 平均サイズ
home-exposure-avg-hold = 平均保有時間
home-exposure-best = 最高
home-exposure-worst = 最低
home-pipeline-title = 市場パイプライン
home-pipeline-tracked = 追跡中
home-pipeline-monitored = 監視中
home-pipeline-passed = フィルター通過
home-pipeline-rejected = 除外
home-pipeline-blacklisted = ブラックリスト
home-pipeline-live-price = ライブ価格
home-pipeline-ohlcv = OHLCV
