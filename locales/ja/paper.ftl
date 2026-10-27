paper-readiness-history = ペーパー履歴
paper-readiness-history-met =
    { $count ->
       *[other] クローズ済みペーパーラウンド { $count }件（必要数: { $needed }）
    }
paper-readiness-history-short = クローズ済みペーパーラウンド { $count }/{ $needed }
paper-readiness-profit = ペーパーで黒字
paper-readiness-profit-detail =
    { $count ->
       *[other] { $count }ラウンドで { $realized } { -sol } を確定、{ $wins }勝
    }
paper-readiness-exits = 決済が約定する
paper-readiness-exits-ok = 最近のペーパー決済は約定しました
paper-readiness-exits-failing = 約定しなかった連続ペーパー決済: { $count }
paper-live-block-setup-incomplete = 先にウォレットと RPC のセットアップを完了してください
paper-live-block-force-stop = 緊急停止が有効です
