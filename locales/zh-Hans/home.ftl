## Portfolio overview

home-portfolio-title = 投资组合价值
home-portfolio-today = 今日
home-stat-available = 可用 { -sol }
home-stat-holdings = 代币持仓
home-stat-open-pnl = 未平仓盈亏
home-stat-realized-today = 今日已实现

home-holdings-token-count =
    { $count ->
       *[other] { $count } 个代币
    }
home-holdings-with-unpriced = { $tokens } · { $count } 个无价格
home-holdings-unpriced-note =
    { $count ->
       *[other] { $count } 个持有的代币暂无价格，在总额中按 0 计算
    }

## Wallet address and QR code

home-wallet-copy =
    .title = 复制钱包地址
    .aria-label = 复制钱包地址
home-wallet-qr-open =
    .title = 显示钱包二维码
    .aria-label = 显示钱包二维码
home-wallet-qr-popover =
    .aria-label = 钱包二维码
home-wallet-qr-receive = 收款
home-wallet-qr-assets = { -sol } 和 SPL 代币
home-wallet-qr-close =
    .title = 关闭
    .aria-label = 关闭钱包二维码
home-wallet-qr-preparing = 正在生成二维码
home-wallet-qr-unavailable = 二维码不可用
home-wallet-qr-image =
    .alt = 主钱包地址二维码

## Performance calendar

home-calendar-title = 业绩日历
home-calendar-previous =
    .title = 上个月
    .aria-label = 上个月
home-calendar-next =
    .title = 下个月
    .aria-label = 下个月
home-calendar-month-pnl = 本月盈亏
home-calendar-trades = 交易
home-calendar-pop-net-pnl = 净盈亏
home-calendar-pop-win-rate = 胜率
home-calendar-pop-win-rate-value = { $rate } · { $wins } 胜 / { $losses } 负
home-calendar-pop-gross-profit = 总盈利
home-calendar-pop-gross-loss = 总亏损
home-calendar-pop-end-balance = 期末余额

## Position exposure and market pipeline

home-operations =
    .aria-label = 投资组合与市场状态
home-exposure-title = 仓位敞口
home-exposure-open = 持仓中
home-exposure-invested = 已投入
home-exposure-avg-size = 平均仓位规模
home-exposure-avg-hold = 平均持仓时间
home-exposure-best = 最佳
home-exposure-worst = 最差
home-pipeline-title = 市场流水线
home-pipeline-tracked = 已跟踪
home-pipeline-priced = 已定价
home-pipeline-passed = 通过过滤
home-pipeline-not-passed = 未通过（全部已跟踪）
home-pipeline-blacklisted = 已加入黑名单
home-pipeline-ohlcv = OHLCV
