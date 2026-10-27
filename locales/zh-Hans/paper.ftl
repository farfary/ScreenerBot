paper-readiness-history = 模拟交易历史
paper-readiness-history-met =
    { $count ->
       *[other] 已有 { $count } 个已平仓的模拟轮次，需要 { $needed } 个
    }
paper-readiness-history-short = { $count } / { $needed } 个已平仓模拟轮次
paper-readiness-profit = 模拟交易盈利
paper-readiness-profit-detail =
    { $count ->
       *[other] { $count } 个轮次共实现 { $realized } { -sol }，盈利 { $wins } 次
    }
paper-readiness-exits = 平仓可以成交
paper-readiness-exits-ok = 最近的模拟平仓已成交
paper-readiness-exits-failing = 连续未成交的模拟平仓: { $count }
paper-live-block-setup-incomplete = 请先完成钱包和 RPC 设置
paper-live-block-force-stop = 紧急停止已启用
