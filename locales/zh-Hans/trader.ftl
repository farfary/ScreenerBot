# Trader page labels.

# Exit types shown in the exit breakdown. Ids are the stored closed_reason: exit
# rule ids, the Debug names of the exit TradeReason variants (src/trader/types.rs,
# shown as readable labels) and the reasons written by src/positions and src/trader/stats.rs.
trader-exit-type-stop-loss = 止损
trader-exit-type-take-profit = 止盈
trader-exit-type-roi = 止盈
trader-exit-type-roi-exit = 止盈
trader-exit-type-trailing-stop = 追踪止损
trader-exit-type-time-override = 时间出场
trader-exit-type-time-rule = 时间规则
trader-exit-type-manual = 手动
trader-exit-type-manual-close = 手动
trader-exit-type-dca = DCA
trader-exit-type-unknown = 未知

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 统计
trader-tab-strategy-control = 策略控制
trader-tab-strategies = 策略
trader-tab-stop-loss = 止损
trader-tab-trailing-stop = 追踪止损
trader-tab-take-profit = 止盈
trader-tab-time-rules = 时间出场
trader-tab-dca = DCA
trader-tab-settings = 设置

## Feature status badges and their messages

trader-feature-coming-soon = 即将推出
    .message = 此功能即将推出，暂不可用。
trader-feature-beta = 测试版
trader-feature-disabled = 已停用
    .message = 此功能当前已停用。

## Status bar and trading controls

trader-status-title = 自动交易
trader-status-loading = 正在加载...
trader-status-running = 运行中
trader-status-stopped = 已停止
trader-status-setup-required = 需要设置
trader-status-unavailable = 请先完成钱包和 RPC 设置以使用自动交易
trader-setup-action = 完成设置
trader-toggle-on = 已启用
trader-toggle-off = 已停用
trader-toggle-start-failed = 启动交易引擎失败
trader-toggle-stop-failed = 停止交易引擎失败
trader-controls-title = 交易控制
trader-halt-title = 交易已暂停
trader-halt-reason-default = 手动强制停止
trader-halt-resume = 恢复
trader-monitor-entry = 入场监控
trader-monitor-exit = 出场监控
trader-monitor-master-off = 自动交易已关闭
trader-loss-limit-title = 周期亏损限额
trader-loss-limit-resume = 恢复交易
trader-loss-limit-reset = 重置周期
trader-loss-limit-off = 关闭
trader-loss-limit-none = 未配置周期亏损限额
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = { $hours } { $minutes }后重置
trader-loss-limit-reached = 已达限额
trader-force-stop = 强制停止全部
trader-force-stop-hint = 立即停止所有监控、复制任务和交易。
trader-force-stop-idle = 当前没有可供强制停止的运行项。

## Confirmations. `.message` is the body and `.confirm` the confirming button.

trader-force-stop-confirm = 强制停止交易
    .message = 这将立即暂停所有交易操作。是否继续？
    .confirm = 停止交易
trader-loss-limit-resume-confirm = 亏损限额后恢复
    .message = 周期亏损限额已停止新的入场。恢复后，交易引擎可在周期重置前再次开仓。是否继续？
trader-loss-limit-reset-confirm = 重置亏损限额周期
    .message = 这将清除当前周期的累计亏损并开始新的周期。是否继续？

## Toasts

trader-toast-control-failed = 自动交易控制失败
trader-toast-force-stop-on = 已启用强制停止
trader-toast-force-stop-failed = 无法启用强制停止
trader-toast-force-stop-cleared = 已解除强制停止
trader-toast-resume-failed = 无法恢复交易
trader-toast-loss-limit-reset-failed = 无法重置亏损限额
trader-toast-entry-monitor-failed = 无法切换入场监控
trader-toast-exit-monitor-failed = 无法切换出场监控
trader-toast-load-failed = 加载失败
    .message = 加载交易引擎配置失败
trader-toast-saved = 配置已保存
    .message = 交易引擎设置已成功应用
trader-toast-save-failed = 保存失败
    .message = 保存交易引擎配置失败
trader-toast-feature-enabled = 功能已启用
trader-toast-feature-disabled = 功能已停用
trader-toast-feature-applied = 自动交易设置已应用
trader-toast-strategy-enabled = 策略已启用
    .message = 策略已生效
trader-toast-strategy-disabled = 策略已停用
    .message = 策略未生效
trader-toast-strategy-failed = 更新失败
    .message = 更新策略状态失败

## Stats: realized window and metrics

trader-stats-window =
    .aria-label = 统计时间窗口
trader-stats-window-day = 24H
trader-stats-window-week = 7D
trader-stats-window-month = 30D
trader-realized-title = 已实现表现
trader-metric-net-pnl = 净盈亏
trader-metric-win-rate = 胜率
trader-metric-profit-factor = 盈利因子
trader-metric-max-drawdown = 最大回撤
trader-metric-capital = 在投资金
trader-metric-avg-win-loss = 平均盈利 / 亏损
trader-metric-closed-trades = 已平仓交易
trader-metric-median-hold = 持仓时间中位数
trader-stats-empty = 此时间窗口内没有已平仓交易
# $won and $lost are formatted SOL amounts.
trader-stats-won-lost = 盈利 { $won } · 亏损 { $lost }
# $wins and $losses are the plural messages below.
trader-stats-record = { $wins } · { $losses }
trader-stats-wins =
    { $count ->
       *[other] { $amount } 次盈利
    }
trader-stats-losses =
    { $count ->
       *[other] { $amount } 次亏损
    }
# $amount is a formatted SOL amount.
trader-stats-expected = 每笔交易预期 { $amount }
trader-stats-profit-factor-basis = 总盈利 ÷ 总亏损
trader-stats-drawdown-basis = 已实现的最深峰谷跌幅
# $count is the position limit and selects the plural.
trader-stats-slots =
    { $count ->
       *[other] 已使用 { $used } / { $max } 个仓位名额
    }
trader-stats-avg-basis = 盈利交易与亏损交易的平均结果
trader-stats-closed =
    { $count ->
       *[other] 已平仓 { $amount } 个仓位
    }
# $span is a formatted duration.
trader-stats-hold-average = 平均 { $span }
trader-stats-excluded =
    { $count ->
       *[other] 已排除 { $amount } 轮已平仓交易：缺少完整成本，无法得出可信的盈亏。
    }

## Stats: daily P&L and extremes

trader-daily-title = 每日盈亏
trader-daily-subtitle = 每日已实现 { -sol }，含累计总额
trader-daily-loading = 正在加载每日盈亏...
trader-daily-chart = 以 { -sol } 计的每日已实现盈亏
trader-extreme-best = 最佳交易
trader-extreme-worst = 最差交易

## Stats: exit breakdown

trader-exit-title = 出场策略明细
trader-exit-subtitle = 仓位的平仓方式，以及每种出场的回收情况
trader-exit-loading = 正在加载出场数据...
trader-exit-empty-day = 过去 24 小时内没有已平仓交易
trader-exit-empty-days =
    { $count ->
       *[other] 过去 { $amount } 天内没有已平仓交易
    }
# $share is a formatted percentage of all exits.
trader-exit-share =
    { $count ->
       *[other] { $amount } 笔交易 · 占出场的 { $share }
    }
# $value is a formatted average percentage.
trader-exit-average = 平均 { $value }

## Shared example vocabulary

trader-example-how-it-works = 工作原理
trader-step-entry = 入场
trader-step-initial-position = 初始仓位
trader-step-auto-exit = 自动出场
trader-step-exit = 出场
trader-step-full-exit = 仓位全部出场
# $value is a percentage without its sign, as typed.
trader-value-percent = { $value }%
# $value is a percentage such as "20.0", shown after a plus sign.
trader-example-profit = 盈利 +{ $value }%

## Stop loss

trader-stop-loss-title = 止损
trader-stop-loss-subtitle = 当仓位亏损超过您设定的阈值时自动出场
# $threshold is the threshold as typed.
trader-stop-loss-impact = 较入场价下跌 { $threshold }% 时出场
trader-stop-loss-hold-immediate = 立即
# $span is a formatted duration.
trader-stop-loss-hold-delay = 延迟 { $span }
trader-stop-loss-price-falls = 价格下跌
trader-stop-loss-threshold-reached = 达到阈值
# $part is a formatted percentage of the position.
trader-stop-loss-sell-part = 卖出 { $part }
trader-stop-loss-part-of-position = 部分仓位
trader-stop-loss-sell-rest = 卖出剩余部分
trader-stop-loss-still-below = 仍低于止损位
# $loss is the loss percentage with its sign.
trader-stop-loss-summary = 亏损限制在 <strong>{ $loss }</strong>
trader-stop-loss-note = <strong>注意：</strong>止损通过提前出场来防范更大的亏损

## Trailing stop

trader-trailing-title = 追踪止损
trader-trailing-subtitle = 价格上涨时跟随价格自动保护利润
# $value is the activation percentage as typed.
trader-trailing-activation-impact = 盈利达到 +{ $value }% 时开始追踪
# $value is the trail distance percentage as typed.
trader-trailing-distance-impact = 较峰值回落 -{ $value }% 时出场
# $value is a formatted signed percentage.
trader-trailing-locked-minimum = 启动时锁定的最低收益：{ $value }
trader-trailing-locked-minimum-warning = 启动时止损位于入场价或以下；它只在盈利时卖出。打开保本止损以设定底线。
trader-trailing-activation = 启动
trader-trailing-peak = 峰值
# $value is a formatted percentage.
trader-trailing-final = 最终 +{ $value }%
# $value is a formatted percentage.
trader-trailing-summary-protected = 保住了 <strong>{ $value }</strong> 的利润
# $value is a formatted percentage.
trader-trailing-summary-avoided = 避免了较峰值 <strong>{ $value }</strong> 的亏损

## Break-even stop

trader-break-even-title = 保本止损
trader-break-even-subtitle = 持仓进入盈利后，在转为亏损前全部卖出
# $trigger and $offset are formatted signed percentages.
trader-break-even-summary = 自上次买入以来的最高价达到 { $trigger } 后，回落到 { $offset } 时全部卖出。

## Take profit

trader-take-profit-title = 止盈
trader-take-profit-subtitle = 在利润达到每个水平时分步卖出仓位
trader-take-profit-initial-buy = 初始买入
# $step is the step's position in the ladder, from 1.
trader-take-profit-step = 第 { $step } 步
# $step is the step's position in the ladder, from 1.
trader-take-profit-profit-label = 第 { $step } 步利润水平
# $step is the step's position in the ladder, from 1.
trader-take-profit-sell-label = 第 { $step } 步卖出比例
trader-take-profit-profit-caption = 利润水平
trader-take-profit-sell-caption = 卖出比例
trader-take-profit-add-step = 添加一步
# $step is the step's position in the ladder, from 1.
trader-take-profit-remove-step = 删除第 { $step } 步
# $min and $max are the configured bounds of a profit level.
trader-take-profit-error-profit-range = 每个利润水平必须在 { $min }% 到 { $max }% 之间
# $min and $max are the configured bounds of a share.
trader-take-profit-error-sell-range = 每个比例必须在 { $min }% 到 { $max }% 之间
trader-take-profit-error-order = 每一步的利润水平必须高于上一步
# $max is the largest total share, the whole position.
trader-take-profit-error-total = 所有步骤合计最多卖出 { $max }%
# $runner is the formatted percentage of the position no step sells.
trader-take-profit-runner = 仓位的 { $runner }% 作为留仓继续持有
trader-take-profit-runner-none = 最后一步卖出仓位的剩余部分
trader-take-profit-runner-warning = 追踪止损、保本止损、两种时间出场和所有出场策略均已关闭，留仓只能通过止损或手动平仓
trader-take-profit-info = 比例按初始仓位计算。价格一次越过多个水平时，每次检查只卖出一步。
# $profit is the step's formatted profit level; $share is its formatted share.
trader-take-profit-example-step = 在 +{ $profit }% 时卖出 { $share }%
trader-take-profit-example-runner = 留仓
trader-take-profit-example-runner-detail = 交由其他退出规则处理
# $sold is the formatted total share the ladder sells; $profit is its last formatted level.
trader-take-profit-summary = 到 +{ $profit }% 利润时卖出 <strong>{ $sold }%</strong>

## Time-based exit

trader-time-loss-title = 平掉亏损持仓
trader-time-subtitle = 超过最长持仓时间且亏损超过阈值时自动出场
trader-time-flat-title = 平掉横盘持仓
trader-time-flat-subtitle = 释放既不上涨也不下跌的持仓所占用的资金
# $span is a formatted duration; $band is a formatted percentage.
trader-time-flat-summary = 持有 { $span } 或更久且盈亏在 ±{ $band } 以内的持仓将被平仓。
trader-time-unit-seconds = 秒
trader-time-unit-minutes = 分钟
trader-time-unit-hours = 小时
trader-time-unit-days = 天
# Shown before the configured duration loads.
trader-time-conversion-default = 168 小时 = 7 天
# $duration and $readable are formatted durations.
trader-time-conversion = { $duration } = { $readable }
trader-duration-seconds =
    { $count ->
       *[other] { $amount } 秒
    }
trader-duration-minutes =
    { $count ->
       *[other] { $amount } 分钟
    }
trader-duration-hours =
    { $count ->
       *[other] { $amount } 小时
    }
trader-duration-days =
    { $count ->
       *[other] { $amount } 天
    }
# $value is the loss percentage as typed, without its sign.
trader-time-loss-impact = 持仓期结束后下跌 { $value }% 或以上时出场
trader-time-loss-hint = 持有时间结束后时间出场卖出的亏损幅度（40 = 在 -40% 出场）
# $day is the day number of the example.
trader-time-day = 第 { $day } 天
trader-time-position-opened = 仓位已开启
trader-time-limit = 时间上限
trader-time-hold-reached = 已达持仓期
trader-time-loss-met = 已达亏损阈值
trader-time-note = <strong>注意：</strong>盈利或亏损较小的仓位不会被出场
trader-time-positions-title = 当前仓位状态
trader-time-positions-loading = 正在加载仓位...
trader-time-positions-empty = 没有持仓中的仓位
trader-time-positions-empty-message = 持仓中的仓位会在此显示其持有时间以及相对时间限制的 ROI。
trader-time-positions-token = 代币
trader-time-positions-hold = 持仓时间
trader-time-positions-roi = ROI
trader-time-positions-rule = 时间出场
trader-time-status-off = 时间出场已关闭
trader-time-status-none = 当前没有适用的时间出场
trader-time-status-not-managed = 不由自动交易管理
# $pnl is a formatted signed percentage; $remaining is a formatted duration.
trader-time-status-losing = 亏损 { $pnl }；{ $remaining } 后平仓
trader-time-status-losing-due = 亏损 { $pnl }；下次检查时平仓
# $held and $remaining are formatted durations.
trader-time-status-flat = 横盘 { $held }；{ $remaining } 后平仓
trader-time-status-flat-due = 横盘 { $held }；下次检查时平仓

## Strategy control

trader-strategy-entry-title = 入场策略
trader-strategy-entry-subtitle = 可开启新仓位的信号。
trader-strategy-exit-title = 出场策略
trader-strategy-exit-subtitle = 可平仓或保护持仓中仓位的信号。
trader-strategy-active-unknown = -- 已启用
trader-strategy-active = 已启用 { $enabled }/{ $total }
trader-strategy-loading = 正在加载策略...
trader-strategy-load-failed = 无法加载策略
trader-strategy-empty = 尚未定义策略
trader-strategy-unnamed = 未命名策略
trader-strategy-priority-auto = 自动
trader-strategy-priority = 优先级 { $priority }

## Dollar-cost averaging

trader-dca-title = 定投加仓（DCA）
trader-dca-subtitle = 对亏损仓位自动加仓，以降低入场均价
trader-dca-threshold-impact = 仓位较入场下跌 { $value }% 时加仓
trader-dca-threshold-hint = DCA 加仓时相对入场的跌幅（10 = 在 -10% 加仓）
trader-dca-count-impact =
    { $count ->
       *[other] 每个仓位最多追加 { $count } 次
    }
trader-dca-size-impact = 每次追加买入 { $amount }
trader-dca-cooldown-impact = 两次追加之间至少间隔 { $span }
trader-dca-example =
    { $count ->
       *[other] 买入 { $initial } → 每再下跌 -{ $threshold }% 追加 { $add }，最多 { $count } 次
    }
trader-dca-info-title = DCA 策略说明
trader-dca-info-subtitle = DCA 交易的重要注意事项
trader-dca-how-title = DCA 的工作原理
trader-dca-how-trigger = <strong>触发：</strong>仓位跌破 DCA 阈值（例如 -10%）
trader-dca-how-action = <strong>操作：</strong>追加 { -sol } 以降低平均成本
trader-dca-how-repeat = <strong>重复：</strong>可根据最大次数多次 DCA
trader-dca-risk-title = 风险提示
trader-dca-risk-exposure = <strong>敞口增加：</strong>DCA 会增加每个仓位承担风险的总资金
trader-dca-risk-knife = <strong>接飞刀：</strong>如果代币持续下跌，DCA 也无济于事
trader-dca-risk-cooldown = <strong>冷却时间：</strong>使用冷却时间避免连续快速 DCA 入场

## General settings

trader-sizing-title = 仓位规模
trader-sizing-subtitle = 控制每个仓位的投入金额
trader-timing-title = 时间与冷却
trader-timing-subtitle = 控制操作之间的时间间隔
trader-timing-close-cooldown = 平仓冷却时间
trader-timing-close-cooldown-hint = 重新开仓同一代币前需等待的分钟数
trader-timing-concurrency = 入场检查并发数
trader-timing-concurrency-hint = 同时检查的代币数量（越高越快，但占用更多 CPU）
trader-timing-unit-minutes = 分钟
trader-timing-unit-tokens = 个
