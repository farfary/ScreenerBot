# Trader page labels.

## Sub-tabs. Ids are the tab ids of the trader page.

trader-tab-stats = 统计
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

## Toolbar, notices and trading controls

trader-status-title = 自动交易
trader-status-running = 运行中
trader-status-stopped = 已停止
trader-status-setup-required = 需要设置
trader-setup-action = 完成设置
trader-toggle-start-failed = 启动交易引擎失败
trader-toggle-stop-failed = 停止交易引擎失败
trader-halt-title = 交易已暂停
trader-halt-reason-default = 手动强制停止
trader-halt-resume = 恢复
trader-monitor-entry = 入场监控
trader-monitor-exit = 出场监控
trader-loss-limit-title = 周期亏损限额
trader-loss-limit-resume = 恢复交易
trader-loss-limit-reset = 重置周期
trader-loss-limit-off = 关闭
# $hours and $minutes are formatted spans such as "2h" and "5m".
trader-loss-limit-resets-in = { $hours } { $minutes }后重置
trader-force-stop = 强制停止全部
trader-force-stop-hint = 立即停止所有监控、复制任务和交易。
trader-force-stop-idle = 当前没有可供强制停止的运行项。
trader-run-state-force-stopped = 强制停止
trader-run-state-waiting = 等待服务
trader-run-state-idle = 监控已关闭
trader-entries-label = 新入场
trader-entries-open = 开放
trader-entries-monitor-off = 入场监控已关闭
trader-entries-no-live-strategy = 没有实盘入场策略
trader-entries-no-paper-strategy = 没有模拟入场策略
trader-action-start = 启动自动交易
trader-action-stop = 停止自动交易
trader-notice-loss-limit = 已达亏损上限
trader-notice-loss-limit-message = 新的实盘入场已暂停，直到周期重置。出场继续进行。
trader-notice-stats-failed = 无法加载统计数据
trader-controls-group =
    .aria-label = 交易控制
trader-monitor-dormant = 随自动交易启动

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

## Stats: book, window, exposure and figures

trader-stats-window =
    .aria-label = 统计时间窗口
trader-stats-book =
    .aria-label = 交易账本
trader-exposure-cash = 现金
trader-exposure-open = 持仓
trader-exposure-unrealized = 未实现
trader-exposure-open-title = 占用名额的持仓，相对于持仓上限
trader-exposure-in-use = 已占用
trader-exposure-in-use-title = 投入持仓的 { -sol }
# $amount is the formatted count.
trader-exposure-unpriced =
    { $count ->
       *[other] { $amount } 个持仓尚无实时价格
    }
# $date is a formatted date; $amount is a formatted SOL amount.
trader-exposure-cash-title = 自 { $date } 起的模拟周期 · 初始 { $amount }
trader-paper-reset = 重置模拟周期
trader-paper-reset-title = 重置模拟周期？
trader-paper-reset-run-message = 所有未平仓的模拟仓位（包括跟单任务的模拟持仓）将按当前价格平仓，并以设定的初始余额开始新的周期。之前的周期保留在历史中。
trader-paper-reset-done = 已开始新的模拟周期
trader-paper-reset-failed = 无法重置模拟周期
trader-realized-title = 已实现表现
trader-metric-net-pnl = 净盈亏
trader-metric-win-rate = 胜率
trader-metric-profit-factor = 盈利因子
trader-metric-max-drawdown = 最大回撤
trader-metric-avg-win-loss = 平均盈利 / 亏损
trader-metric-closed-trades = 已平仓交易
trader-metric-median-hold = 持仓时间中位数
trader-metric-fees = 网络费用
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
trader-stats-excluded =
    { $count ->
       *[other] 已排除 { $amount } 笔已平仓交易：成本不完整，盈亏未知。
    }
# $ratio is a formatted ratio.
trader-stats-payoff = 盈亏比 { $ratio }
trader-stats-streak-win =
    { $count ->
       *[other] 连续 { $amount } 笔盈利
    }
trader-stats-streak-loss =
    { $count ->
       *[other] 连续 { $amount } 笔亏损
    }
# $wins and $losses are formatted durations.
trader-stats-hold-split = 盈利 { $wins } · 亏损 { $losses }
trader-stats-fees-basis = 已计入净盈亏
trader-stats-loading = 正在加载统计数据...
trader-open-strategies = 打开策略

## Stats: P&L chart, trade outcomes and extremes

trader-daily-title = 每日盈亏
trader-daily-subtitle = 每日已实现 { -sol }，含累计总额
trader-daily-chart = 以 { -sol } 计的每日已实现盈亏
trader-weekly-title = 每周盈亏
trader-weekly-subtitle = 每个 ISO 周已实现 { -sol }，含累计总额
trader-weekly-chart = 以 { -sol } 计的每周已实现盈亏
# $date is a formatted date; $pnl a signed SOL amount.
trader-daily-bar =
    { $count ->
       *[other] { $date } · { $pnl } · { $amount } 笔交易
    }
trader-weekly-bar =
    { $count ->
       *[other] { $date } 当周 · { $pnl } · { $amount } 笔交易
    }
trader-extreme-best = 最佳交易
trader-extreme-worst = 最差交易
trader-extreme-runs = 最多连续
trader-outcome-title = 交易结果
trader-outcome-subtitle = 按收益率划分的已平仓交易（百分比）
# $value, $from and $to are formatted signed percentages.
trader-outcome-below = 低于 { $value }
trader-outcome-range = { $from } 至 { $to }
trader-outcome-above = { $value } 及以上
# $range is one of the three messages above; $pnl a signed SOL amount.
trader-outcome-bucket =
    { $count ->
       *[other] { $range } · { $amount } 笔交易 · { $pnl }
    }

## Stats: empty windows, exit reasons, entry sources and recent closes

trader-stats-empty-day = 过去 24 小时内没有已平仓交易
trader-stats-empty-all = 暂无已平仓交易
trader-stats-empty-hint = 仓位卖出后，已平仓交易会显示在这里。启动自动交易，或选择更长的时间段。
trader-stats-empty-days =
    { $count ->
       *[other] 过去 { $amount } 天内没有已平仓交易
    }
trader-exits-title = 出场原因
trader-exits-subtitle = 已平仓交易如何结束，以及每种出场的收益
trader-sources-title = 入场来源
trader-sources-subtitle = 已平仓交易由什么开仓
trader-recent-title = 最近平仓
trader-recent-subtitle = 此时间窗口内最新的已平仓交易
trader-column-token = 代币
trader-column-exit = 出场
trader-column-source = 来源
trader-column-trades = 交易数
trader-column-win-rate = 胜率
trader-column-avg-return = 平均收益
trader-column-net-pnl = 净盈亏
trader-column-held = 持有
trader-column-closed = 平仓
trader-column-pnl = 盈亏
trader-column-return = 收益率

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
# $step is the band's position in the list, from 1.
trader-trailing-band = 分段 { $step }
# $step is the band's position in the list, from 1.
trader-trailing-band-profit-label = 分段 { $step } 峰值利润
# $step is the band's position in the list, from 1.
trader-trailing-band-distance-label = 分段 { $step } 追踪距离
trader-trailing-band-profit-caption = 峰值利润起
trader-trailing-band-distance-caption = 追踪距离
trader-trailing-add-band = 添加分段
# $step is the band's position in the list, from 1.
trader-trailing-remove-band = 删除分段 { $step }
# $min and $max are the configured bounds of a band's peak profit.
trader-trailing-band-error-profit-range = 每个分段的峰值利润须在 { $min }% 到 { $max }% 之间
# $min and $max are the configured bounds of a band's distance.
trader-trailing-band-error-distance-range = 每个分段的距离须在 { $min }% 到 { $max }% 之间
trader-trailing-band-error-order = 每个分段的峰值利润须高于激活值和前一个分段
trader-trailing-band-error-distance = 每个分段的追踪距离须小于其峰值利润

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

## Dollar-cost averaging

trader-dca-title = 定投加仓（DCA）
trader-dca-subtitle = 对亏损仓位自动加仓，以降低入场均价
trader-dca-measure-average = 平均入场价
trader-dca-measure-first-entry = 首次入场价
trader-dca-step = 加仓 { $step }
trader-dca-drop-label = 加仓 { $step } 的价格跌幅
trader-dca-size-label = 加仓 { $step } 的数量，首次买入的倍数
trader-dca-drop-caption = 价格跌幅
trader-dca-size-caption = 买入 × 首次买入
trader-dca-add-step = 添加档位
trader-dca-remove-step = 移除加仓 { $step }
trader-dca-step-amount = = { $amount }
trader-dca-step-below-minimum = 低于最小交易额 { $minimum }：此次加仓不会发送
trader-dca-error-drop-range = 每档跌幅必须在 { $min }% 到 { $max }% 之间
trader-dca-error-size-range = 每档数量必须在首次买入的 { $min }× 到 { $max }× 之间
trader-dca-error-order = 以首次入场价为基准时，每次加仓的跌幅都必须比上一次更深
trader-dca-cooldown-impact = 两次追加之间至少间隔 { $span }
trader-dca-min-hold-none = 首次加仓可在入场后立即进行
trader-dca-min-hold-impact = 首次加仓在入场后至少等待 { $span }
trader-dca-capacity = 此仓位最多可持有：{ $amount }（首次买入的 { $multiple }×）
trader-dca-info-title = DCA 策略说明
trader-dca-info-subtitle = DCA 交易的重要注意事项
trader-dca-how-title = DCA 的工作原理
trader-dca-how-trigger = <strong>触发：</strong>价格跌破下一档，按平均或首次入场价计算
trader-dca-how-action = <strong>操作：</strong>追加 { -sol } 以降低平均成本
trader-dca-how-repeat = <strong>阶梯：</strong>每次加仓进入下一档；所有档位用完后停止加仓
trader-dca-how-pause = <strong>暂停：</strong>上一次加仓仍在结算时，或亏损上限阻止新入场时，不会加仓
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

## Service mode notice

trader-service-notice-title = 正在通过 { -brand } 服务运行
trader-service-notice-message = 价格每隔几秒刷新一次，因此紧密追踪止损等快速离场的反应会比使用你自己的 RPC 更慢。
