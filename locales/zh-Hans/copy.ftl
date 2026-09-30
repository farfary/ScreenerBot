copy-skip-not-buy-swap = 钱包活动不是买入
copy-skip-task-disabled = 任务已暂停
copy-skip-mode-transition-required = 执行模式需单独切换
copy-skip-live-confirmation-required = 实盘执行需要确认
copy-skip-unsupported-sizing-mode = 暂不支持该仓位规模模式
copy-skip-self-copy = 该钱包属于您自己
copy-skip-target-below-minimum = 钱包交易低于最小值
copy-skip-target-above-maximum = 钱包交易高于最大值
copy-skip-already-bought = 已买入过该代币（仅买入一次）
copy-skip-blacklisted = 代币被风控拦截
copy-skip-filter-required = 代币未通过过滤
copy-skip-budget-exhausted = 任务预算已用完
copy-skip-token-cap-reached = 已达单代币上限
copy-skip-below-minimum-size = 跟单金额过小
copy-skip-invalid-sizing = 任务仓位规模无效
copy-skip-invalid-slippage = 任务滑点无效
copy-skip-invalid-exit-policy = 任务出场规则无效
copy-skip-invalid-price = 没有可用的市场价格
copy-skip-not-sell-swap = 钱包活动不是卖出
copy-skip-exit-mode-disabled = 已忽略钱包卖出：该任务按自身规则卖出
copy-skip-force-stopped = 交易已被强制停止
copy-skip-copy-position-not-found = 此任务下没有仓位
copy-skip-position-user-only = 该仓位由您自行管理
copy-skip-position-management-mismatch = 该仓位不再跟随跟单卖出
copy-skip-latency-kill-switch = 已自动暂停：检测到交易的时间过晚
copy-skip-claim-reconciled-abandoned = 中断的实盘提交已关闭，不再重试
copy-skip-stale-observation = 停机后重放，已过久无法跟单
copy-skip-unknown-observation-time = 重放的交易没有区块时间
copy-skip-entry-blocked = 入场被拦截

copy-entry-block-force-stopped = 交易已被强制停止
copy-entry-block-loss-limit = 亏损限额已拦截新入场
copy-entry-block-connectivity = 所需服务不可用
copy-entry-block-position-limit = 已达持仓上限
copy-entry-block-already-open = 已有仓位处于持仓中
copy-entry-block-reentry-cooldown = 代币再入场冷却时间
copy-entry-block-open-cooldown = 全局入场冷却时间
copy-entry-block-entry-reserved = 另一笔入场正在处理
copy-entry-block-blacklisted = 代币被风控拦截
copy-entry-block-check-failed = 安全检查未能完成

copy-pause-user = 已由您暂停
copy-pause-latency-kill-switch = 已自动暂停：交易平均延迟 { $average } 秒到达（上限 { $threshold } 秒）
copy-pause-watch-detached = 已自动暂停：不再监控该钱包
copy-pause-watch-budget-exceeded = 已暂停：该钱包在追上进度前已达到 { $limit } 个签名的监控检查上限
copy-pause-helius-unavailable = 已暂停：{ -helius } 钱包检查失败
copy-pause-watch-processing-failed = 已暂停：无法处理钱包活动
copy-pause-unspecified = 已暂停

copy-pause-short-user = 由您暂停
copy-pause-short-latency-kill-switch = 过慢
copy-pause-short-watch-detached = 监控丢失
copy-pause-short-watch-budget-exceeded = 监控上限
copy-pause-short-helius-unavailable = 监控提供商
copy-pause-short-watch-processing-failed = 监控处理
copy-state-paused = 已暂停
copy-state-paused-reason = 已暂停 · { $reason }

copy-readiness-history = 模拟交易历史
copy-readiness-history-met =
    { $count ->
       *[other] 已有 { $count } 个已平仓的模拟轮次，需要 { $needed } 个
    }
copy-readiness-history-short = { $count } / { $needed } 个已平仓模拟轮次
copy-readiness-profit = 模拟交易盈利
copy-readiness-profit-detail =
    { $count ->
       *[other] { $count } 个轮次共实现 { $realized } { -sol }，盈利 { $wins } 次
    }
copy-readiness-latency = 交易检测及时
copy-readiness-latency-detail = p95 到达时间 { $p95 } 秒，上限 { $limit } 秒
copy-readiness-latency-none = 暂无到达时间样本
copy-readiness-priced = 所有持仓均有价格
copy-readiness-priced-ok = 每个模拟持仓都有流动性池价格
copy-readiness-priced-missing =
    { $count ->
       *[other] { $count } 个持仓没有流动性池价格
    }
copy-readiness-runtime = 可执行实盘
copy-readiness-runtime-ok = 设置和安全检查均允许实盘跟单

copy-live-block-setup-incomplete = 请先完成钱包和 RPC 设置
copy-live-block-force-stop = 紧急停止已启用
copy-live-block-copy-trading-disabled = 跟单处理已全局暂停
copy-live-block-unavailable = 无法执行实盘

## Task state, mode and exit labels.

copy-state-system-paused = 已全局暂停
copy-state-force-stopped = 已强制停止
copy-state-entries-blocked = 入场已被拦截
copy-state-running-live = 运行中
copy-state-running-paper = 运行中
copy-mode-paper = 模拟
copy-mode-live = 实盘
copy-exit-mode-buy-only = 我的出场规则
copy-exit-mode-mirror = 镜像钱包卖出
copy-exit-mode-hybrid = 钱包卖出与我的规则
copy-exit-target-sell = 钱包已卖出
copy-exit-stop-loss = 止损
copy-exit-trailing-stop = 追踪止损
copy-exit-take-profit = 止盈
copy-exit-time-override = 时间规则
copy-exit-manual = 手动平仓

## Shared wording

copy-request-failed = 请求失败
copy-keep-paused = 保持暂停
copy-paused-suffix = · 已暂停
copy-mode-paused = { $mode } · 已暂停
copy-task-ref = “{ $name }”（{ $mode }）
copy-metric-realized-pnl = 已实现盈亏
copy-metric-unrealized-pnl = 未实现盈亏
copy-metric-win-rate = 胜率
copy-metric-budget-spent = 已用预算
copy-metric-median-arrival = 到达时间中位数
copy-metric-open-holdings = 未平仓持仓
copy-record-won-lost = 盈 { $won } · 亏 { $lost }
copy-budget-of = { $spent } / { $budget } { -sol }
copy-kind-fills = 成交
copy-kind-exits = 出场
copy-kind-skips = 跳过
copy-kind-errors = 错误
copy-field-per-trade-cap = 单笔上限
copy-field-per-token-cap = 单代币上限
copy-field-total-budget = 总预算
copy-field-slippage = 滑点
copy-rules-wallet-sells-only = 仅钱包卖出
copy-filter-copy-setting-required = 跟单设置（必须通过）
copy-filter-copy-setting-not-required = 跟单设置（无需通过）
copy-count-closed-rounds =
    { $count ->
       *[other] { $count } 个已平仓轮次
    }
copy-count-open-holdings =
    { $count ->
       *[other] { $count } 个持仓
    }
copy-unrealized-partial =
    { $priced ->
       *[other] { $priced } 个持仓有价格 · { $unpriced } 个无价格
    }
copy-unrealized-unpriced =
    { $count ->
       *[other] { $count } 个持仓无价格
    }
copy-range-24h = 24h
copy-range-7d = 7d
copy-range-30d = 30d
copy-range-all = 全部
copy-range-label =
    .aria-label = 日期范围

## Page strip

copy-page-title = 跟单交易
copy-page-beta = 测试版
copy-strip-loading = 加载中
copy-strip-unavailable = 不可用
copy-strip-pause-all = 全部暂停
copy-strip-resume = 恢复处理
copy-strip-settings = 设置
copy-strip-add-wallet = 添加钱包
copy-strip-paused-globally = 已全局暂停 · 不会新增跟单，出场仍会执行
copy-strip-force-stopped = 已强制停止 · 不会跟单
copy-strip-loss-limit = 亏损限额 · 新入场已被拦截，出场仍会执行
copy-strip-idle-paused =
    { $count ->
       *[other] 空闲 · { $count } 个任务已暂停
    }
copy-strip-idle-empty = 空闲 · 暂无任务
copy-strip-processing = 处理中 · 模拟 { $paper } 个
copy-strip-processing-live = 处理中 · 实盘 { $live } 个 · 模拟 { $paper } 个
copy-figures-label =
    .aria-label = 跟单交易汇总
copy-figure-marked-at-pool = 按流动性池价格标记
copy-figure-across-tasks = 所有任务合计
copy-figure-budget-lifetime = 已启用任务的累计支出
copy-figure-budget-none = 没有已启用的任务
copy-figure-arrival-samples =
    p95 { $p95 } · { $count ->
       *[other] { $count } 笔交易
    }
copy-figure-arrival-none = 已启用的任务暂无样本

## Page frame

copy-load-failed = 无法加载跟单交易：{ $error }
copy-resume-all-title = 恢复跟单处理
copy-resume-all-message =
    { $count ->
       *[other] 当钱包再次交易时，{ $count } 个实盘任务将提交真实兑换。
    }
copy-toast-resumed-all = 跟单处理已恢复
copy-toast-paused-all = 所有跟单处理已暂停
copy-toast-global-failed = 无法更改跟单处理状态

## Onboarding

copy-onboarding-title = 跟单您信任的钱包，先用模拟交易验证
copy-onboarding-body = 每个任务都从模拟交易开始：目标交易会按流动性池价格，结合您的滑点和手续费进行模拟，您的出场规则会在模拟账本上运行。待某个钱包的模拟结果达标后，再为其单独启用实盘。
copy-onboarding-add = 添加您的第一个钱包
copy-onboarding-observe = 观察
copy-onboarding-observe-detail = 检测钱包的兑换，无需花费 { -sol }。
copy-onboarding-evaluate = 评估
copy-onboarding-evaluate-detail = 查看模拟盈亏、胜率、跳过情况、检测速度和滑点。
copy-onboarding-arm = 启用
copy-onboarding-arm-detail = 通过就绪检查，然后开启真实兑换。

## Wallet list

copy-list-label =
    .aria-label = 已跟单的钱包
copy-list-title = 钱包
copy-list-compare = 对比
copy-list-sort-label = 钱包排序
copy-list-count = { $active } 个活跃 · 共 { $total } 个
copy-sort-pnl = 盈亏
copy-sort-state = 状态
copy-sort-name = 名称
copy-compare-label =
    .aria-label = 对比钱包

## Dialog chrome

copy-dialog-close =
    .aria-label = 关闭
copy-editor-title-add = 添加钱包
copy-editor-sub-add = 新任务从模拟交易开始
copy-arm-title = 启用实盘跟单
copy-arm-sub = 使用您钱包的真实兑换
copy-arm-keep-paper = 保持模拟
copy-arm-confirm = 启用实盘
copy-profile-title = 钱包档案
copy-profile-sub = 此机器人观察到的该钱包情况

## Settings dialog

copy-settings-title = 跟单交易设置
copy-settings-subtitle = 适用于所有任务的全局策略
copy-settings-filter-warning = 在默认的过滤设置下，这几乎会拒绝所有代币，导致不会跟单。除非您的过滤器能放行钱包所交易的代币，否则请保持关闭。
copy-settings-unit-seconds = 秒
copy-settings-unit-trades = 笔交易
copy-settings-unit-tasks = 个任务
copy-settings-unit-closed-rounds = 个已平仓轮次
copy-settings-save = 保存设置
copy-settings-load-failed = 无法加载跟单设置
copy-settings-saved = 跟单交易设置已保存

## Workspace

copy-tab-overview = 概览
copy-tab-holdings = 持仓
copy-tab-activity = 活动
copy-tab-rules = 规则
copy-tab-execution = 执行
copy-tabs-label = 任务视图
copy-workspace-select = 选择一个钱包以打开其工作区。
copy-workspace-loading = 正在加载任务…
copy-workspace-load-failed = 无法加载此任务：{ $error }

copy-state-detail-paper = 模拟运行中 · 交易为模拟，不会花费资金
copy-state-detail-live = 实盘运行中 · 钱包的交易将通过真实兑换跟单
copy-state-detail-system-paused = 等待中 · 跟单处理已全局暂停，出场仍会执行
copy-state-detail-entries-blocked = 入场已被亏损限额拦截 · 出场仍会执行
copy-state-detail-force-stopped = 已强制停止 · 不会跟单

copy-paused-since = { $reason } · { $since }
copy-paused-resume-latency = 恢复后仍沿用相同的上限，因此只要交易仍然延迟到达，就会再次暂停。请检查 RPC 数据流，或在设置中提高到达时间上限。
copy-paused-resume-detached = 恢复后将重新监控该钱包。
copy-paused-holdings-rules =
    { $count ->
       *[other] 其出场规则仍会平掉其 { $count } 个持仓。
    }
copy-paused-holdings-mirror =
    { $count ->
       *[other] 该钱包的卖出仍会平掉其 { $count } 个持仓。
    }
copy-paused-holdings-hybrid =
    { $count ->
       *[other] 该钱包的卖出及其出场规则仍会平掉其 { $count } 个持仓。
    }

copy-watch-state-catching-up = 钱包监控：正在追赶进度。正通过 { -helius } 检查此钱包。
copy-watch-state-watching = 钱包监控：监控中。正通过 { -helius } 检查此钱包。
copy-watch-last-check = 上次检查：{ $ago }。
copy-watch-recovery-active = 钱包监控已启用
copy-watch-recovery-catching-up = 钱包监控正在追赶进度
copy-watch-recovery-still-paused = 跟单任务仍处于暂停状态。准备好后请恢复跟单。
copy-watch-recovery-title = 恢复钱包监控
copy-watch-recovery-processing-failed = 无法处理钱包活动。已保存的进度会保留。问题解决后请重试。
copy-watch-recovery-provider-failed = { -helius } 检查失败。已保存的进度会保留。提供商恢复可用后请重试。
copy-watch-recovery-budget-intro = 此钱包的活动量超出了当前监控可检查的范围。请选择如何继续。
copy-watch-approve = 尝试使用 { -helius } 追赶进度
copy-watch-approve-help = 从已保存的进度继续。可能消耗更多 { -helius } 额度，且仍可能落后。
copy-watch-approve-unavailable = { -helius } 追赶功能不可用。请配置一个已启用的 { -helius } RPC 端点，以便在不跳过未检查活动的情况下继续。
copy-watch-no-provider = 此监控没有受支持的追赶提供商。
copy-watch-budget-label = 每次检查的签名数
copy-watch-budget-hint = 或者跳过未检查的活动，从现在开始恢复。每次检查可选 { $min }–{ $max } 个签名；上限越高，可能消耗越多 RPC 调用。
copy-watch-ack = 我了解错过的活动将不会被跟单。
copy-watch-toast-range = 每次轮询请选择 { $min } 到 { $max } 个签名，步长为 { $step } 个签名
copy-watch-toast-ack = 请确认自上次完成检查以来的签名将被跳过
copy-watch-resumed = 钱包监控已从现在开始恢复；跟单任务仍处于暂停状态
copy-watch-resume-failed = 无法恢复钱包监控
copy-watch-retry-started = 已从保存的进度重新开始钱包监控；跟单任务仍处于暂停状态
copy-watch-retry-failed = 无法重试钱包监控
copy-watch-approve-title = 允许对此钱包使用 { -helius } 追赶进度
copy-watch-approve-message = { -helius } 可以从已保存的进度检查成功的 Solana 交易，而不跳过未检查的区间。目前每返回 100 笔完整交易收取 10 点额度（向上取整），每次请求最低 10 点额度。一次检查可能发出多个请求；实际用量和提供商定价可能有所不同。跟单仍保持暂停，直到您单独恢复。
copy-watch-approve-confirm = 允许用于此钱包
copy-watch-approved = 钱包监控已从保存的进度开始；跟单任务仍处于暂停状态
copy-watch-restore-failed = 无法恢复钱包监控

copy-action-pause = 暂停
copy-action-resume = 恢复
copy-action-resume-copy = 恢复跟单
copy-action-resume-from-now = 从现在开始恢复
copy-action-retry-watch = 重试钱包监控
copy-action-return-paper = 切回模拟
copy-action-edit-rules = 编辑规则
copy-action-clone = 克隆
copy-action-profile = 钱包档案
copy-resume-live-title = 恢复实盘跟单
copy-resume-live-message = 当此钱包再次交易时，“{ $name }”将使用您钱包提交真实兑换。
copy-resume-live-confirm = 恢复实盘
copy-task-resumed = 任务已恢复
copy-task-paused = 任务已暂停
copy-task-state-failed = 无法更改任务状态
copy-return-paper-message = “{ $name }”的新跟单将重新改为模拟，不会花费 { -sol }。
copy-return-paper-cancel = 保持实盘
copy-task-returned-paper = 任务已切回模拟
copy-mode-change-failed = 无法更改执行模式
copy-delete-title = 删除跟单任务
copy-delete-message = 删除“{ $name }”？其决策记录和模拟结果将被移除，且此任务不再监控该钱包。
copy-delete-confirm = 删除任务
copy-delete-cancel = 保留任务
copy-task-deleted = 跟单任务已删除
copy-task-delete-failed = 无法删除跟单任务

## Overview tab

copy-overview-results = 结果
copy-analytics-load-failed = 无法加载分析数据：{ $error }
copy-analytics-loading = 正在加载分析数据…
copy-exit-bucket =
    { $count ->
       *[other] { $count } 笔卖出 · { $pnl }
    }
copy-overview-average-win = 平均盈利
copy-overview-average-loss = 平均亏损 { $amount }
copy-overview-profit-factor = 盈利因子
copy-overview-profit-factor-note = 总盈利 ÷ 总亏损
copy-overview-average-hold = 平均持仓时间
copy-overview-average-hold-note = 从入场到出场
copy-overview-best-round = 最佳轮次
copy-overview-worst-round = 最差 { $amount }
copy-overview-curve-title = 累计盈亏
copy-overview-exits-title = 按出场方式统计的卖出
copy-overview-skips-title = 交易被跳过的原因
copy-book-title-live = 实盘账本
copy-book-title-paper = 模拟账本
copy-book-all-time = 全部时间
copy-book-buys =
    <strong>{ $count }</strong> { $count ->
       *[other] 笔买入
    }
copy-book-policy-exits =
    <strong>{ $count }</strong> { $count ->
       *[other] 笔按您的规则出场
    }
copy-book-wallet-sells =
    <strong>{ $count }</strong> { $count ->
       *[other] 笔钱包卖出
    }
copy-book-manual-closes = <strong>{ $count }</strong> 笔手动平仓
copy-book-skipped = <strong>{ $count }</strong> 笔已跳过
copy-book-failed = <strong>{ $count }</strong> 笔失败
copy-book-closed = { $count } 笔已平仓
copy-book-budget-note = { $mode }已用 { $total } · 剩余 { $remaining }
copy-check-passed = 已通过
copy-check-not-passed = 未通过
copy-readiness-title = 上实盘前
copy-readiness-live-note = 此任务正在实盘交易。可在上方标题处将其切回模拟。
copy-readiness-all-pass = 所有检查均已通过。
copy-readiness-needs-review = 启用前需要明确确认未就绪的项目。
copy-readiness-arm = 检查并启用实盘

## Rules tab and review

copy-rules-title = 当前生效的规则
copy-rules-size-ratio = 钱包交易的 { $pct }
copy-rules-size-fixed = 每次跟单 { $amount }
copy-rules-target-any = 任意金额
copy-rules-target-min = 至少 { $amount }
copy-rules-target-max = 至多 { $amount }
copy-rules-target-between = { $min } – { $max } { -sol }
copy-rules-source-override = 任务覆盖 · 交易引擎 { $value }
copy-rules-source-default = 交易引擎默认值
copy-rules-not-used = 未使用：由钱包的卖出决定
copy-rules-col-rule = 规则
copy-rules-col-applies = 生效
copy-rules-col-source = 来源
copy-rules-budget-note = { $mode }已花费 { $spent } · 剩余 { $remaining }
copy-rules-token-copies =
    { $count ->
       *[other] 单个代币约可完整跟单 { $count } 次
    }
copy-rules-sizing = 仓位规模
copy-rules-copy-size = 跟单金额
copy-rules-entry-filters = 入场过滤
copy-rules-target-size = 钱包交易金额
copy-rules-repeat-buys = 重复买入
copy-rules-repeat-first-only = 每个代币仅跟首次买入
copy-rules-repeat-every = 跟每次买入，直至单代币上限
copy-rules-filter-pass = 通过过滤
copy-rules-filter-required = 必须
copy-rules-filter-not-required = 无需
copy-rules-filter-task-override = 任务覆盖
copy-rules-exits = 出场
copy-rules-exits-inactive = 仅在钱包卖出时才卖出持仓；下列规则在此模式下不会运行。

## Exit rules

copy-rule-status = 状态
copy-rule-on = 已开启
copy-rule-off = 已关闭
copy-rule-unit-seconds = 秒
copy-rule-unit-minutes = 分钟
copy-rule-stop-loss-threshold = 亏损达到以下幅度时卖出
copy-rule-stop-loss-min-hold = 持仓不足以下时长不触发
copy-rule-no-minimum = 无最短时长
copy-rule-partial-exits = 部分出场
copy-rule-partial-allowed = 允许
copy-rule-partial-full-only = 仅完全出场
copy-rule-partial-size = 部分出场比例
copy-rule-trailing-activation = 盈利达到以下幅度时启动
copy-rule-trailing-distance = 较峰值回落以下幅度时卖出
copy-rule-take-profit-target = 盈利达到以下幅度时卖出
copy-rule-time-duration = 持仓达到以下时长后检查
copy-rule-time-threshold = 盈亏不高于以下值时卖出
copy-preset-inherit = 交易引擎默认值
copy-preset-conservative = 保守
copy-preset-balanced = 均衡
copy-preset-aggressive = 激进
copy-preset-custom = 自定义
copy-validate-stop-loss = 止损必须大于 0% 且不超过 100%。
copy-validate-partial-size = 部分出场比例必须在 0% 到 100% 之间。
copy-validate-min-hold = 最短持仓时间必须为整数秒。
copy-validate-trailing-activation = 追踪启动幅度必须大于 0% 且不超过 100%。
copy-validate-trailing-distance = 追踪距离必须大于 0% 且不超过 100%。
copy-validate-take-profit = 止盈必须大于 0%。
copy-validate-time-duration = 时间规则的时长必须大于零。
copy-validate-time-threshold = 时间规则的阈值为亏损：请使用 0% 或负数。
copy-warning-mirror = 仅钱包的卖出会平掉持仓：没有止损保护，钱包从不卖出的代币会一直持有。
copy-warning-no-rules = 没有开启任何出场规则，且钱包的卖出被忽略：持仓永远不会被卖出。
copy-warning-no-stop-loss = 未设置止损：下跌的代币会一直持有，直到其他规则触发或钱包卖出。
copy-warning-stop-delay = 止损会在每次买入后等待 { $hold }：下跌更快的代币平仓时会远低于 { $threshold }。
copy-warning-take-profit-cost = { $target } 的止盈无法覆盖卖出成本（{ $slippage } 滑点和 { $fee } 兑换手续费），因此平仓轮次会亏损。
copy-warning-trailing-distance = 追踪距离不小于其启动涨幅，因此已启动的追踪可能在低于入场价时卖出。

## Execution tab

copy-execution-title = 执行质量
copy-execution-bucket-upto = ≤ { $limit }
copy-execution-bucket-over = > { $limit }
copy-execution-bucket-any = 任意
copy-execution-limit-on =
    { $count ->
       *[other] 最近 { $count } 笔交易平均高于 { $limit } 时暂停
    }
copy-execution-limit-off = 紧急开关已关闭
copy-execution-arrival-samples =
    { $count ->
       *[other] 实时观察到 { $count } 笔交易
    }
copy-execution-p95 = p95 到达时间
copy-execution-median-slippage = 滑点中位数
copy-execution-slippage-samples =
    { $count ->
       *[other] { $count } 笔已测量的成交
    }
copy-execution-worst-slippage = 最大滑点
copy-execution-average-slippage = 平均 { $amount }
copy-execution-delay-title = 检测延迟
copy-execution-delay-note = 从钱包所在区块到此机器人发现交易的时间。停机后的重放不计入。
copy-execution-delay-limit = 超过 { $limit } 到达上限的柱条以琥珀色显示。
copy-execution-fastest = 最快
copy-execution-average = 平均
copy-execution-slowest = 最慢
copy-execution-fill-title = 与钱包成交的对比
copy-execution-fill-note = 正值表示比钱包更差：买入时付出更多，镜像卖出时获得更少。对于没有流动性池价格的代币，模拟成交按钱包自己的交易价计价，因此无法衡量，已被排除。
copy-execution-samples = 样本
copy-execution-median = 中位数
copy-execution-worst = 最差
copy-execution-decisions = 范围内的决策

## Compare view

copy-compare-title = 对比钱包
copy-compare-back = 返回钱包
copy-compare-load-failed = 无法加载对比：{ $error }
copy-compare-loading = 正在加载对比…
copy-compare-empty = 没有可对比的任务。
copy-compare-curve-title = 累计已实现盈亏
copy-table-wallet = 钱包
copy-table-mode = 模式
copy-table-rounds = 轮次
copy-table-realized = 已实现
copy-table-profit-factor = 盈利因子
copy-table-average-hold = 平均持仓
copy-table-median-slippage = 滑点中位数

## Charts

copy-chart-curve-label = 累计盈亏 { $amount } { -sol }
copy-chart-compare-label = 按任务的累计盈亏
copy-chart-empty-curve = 此范围内暂无已平仓轮次。
copy-chart-empty-bars = 此范围内没有记录。
copy-chart-empty-histogram = 此范围内暂无到达时间样本。
copy-chart-empty-compare = 此范围内没有可对比的已平仓轮次。
copy-chart-histogram-title = { $count } / { $total }

## Wallet profile

copy-profile-copy = 跟单此钱包
copy-profile-copy-other = 使用其他规则跟单
copy-profile-loading = 正在加载钱包档案…
copy-profile-watch-title = 监控
copy-profile-watched = 已监控
copy-profile-watch-resume-hint = 恢复任务将重新监控该钱包
copy-profile-watch-add-hint = 添加任务将开始监控该钱包
copy-profile-stream = 数据流
copy-profile-subscribed = 已订阅
copy-profile-not-subscribed = 未订阅
copy-profile-sources =
    { $count ->
       *[other] { $count } 个来源
    }
copy-profile-last-activity = 最近活动
copy-profile-last-error = 最近错误
copy-profile-own-wallet = 这是您自己的钱包之一，不允许跟单。
copy-profile-observed-title = 已观察到的交易
copy-profile-observed-none = 此机器人尚未观察到该钱包的交易。模拟任务可在不花费 { -sol } 的情况下观察它。
copy-profile-swaps-seen = 已观察到的兑换
copy-profile-swaps-seen-note = 您所有任务中不重复的钱包兑换
copy-profile-buys-sells = 买入 / 卖出
copy-profile-buys-sells-value = { $buys } / { $sells }
copy-profile-tokens-traded = 已交易代币
copy-profile-first-seen = 首次发现
copy-profile-last-seen = 最近发现
copy-profile-tasks-title = 您在此钱包上的任务
copy-table-task = 任务

## Arm live dialog

copy-arm-acks-left =
    { $count ->
       *[other] 还有 { $count } 项确认待勾选
    }
copy-arm-readiness-title = 模拟账本的就绪情况
copy-arm-exposure-title = 风险敞口
copy-arm-per-copy = 每次跟单
copy-arm-budget-left-value = { $left } / { $total } { -sol }
copy-arm-budget-left = 实盘剩余预算
copy-arm-budget-left-note = 模拟花费单独计算，不占用此预算
copy-arm-exits = 出场
copy-arm-stop-note = 持仓不足 { $hold } 时不触发：下跌更快则平仓价更低
copy-arm-shared = 此钱包同时被 { $tasks } 跟单：每个任务按各自的预算跟单其交易。
copy-arm-unavailable = 目前无法执行实盘；请查看最近一次检查。
copy-arm-ack-real-sol = 真实 { -sol }：此任务最多可从您的钱包花费 { $budget } { -sol }，每次跟单最多 { $trade } { -sol }。
copy-arm-ack-fees = 实盘跟单需支付真实的网络费用并承担滑点；模拟结果不代表实盘结果。
copy-arm-ack-unready = 部分就绪检查尚未通过。仍要启用此任务。
copy-arm-lead = “{ $name }”将使用您钱包的真实兑换来跟单该钱包的交易。
copy-arm-confirmation-missing = 无法加载实盘确认
copy-arm-armed = 实盘跟单已启用
copy-arm-failed = 无法启用实盘跟单

## Holdings tab

copy-holdings-title = 持仓
copy-holdings-view-label = 持仓视图
copy-holdings-view-open = 持仓中（{ $count }）
copy-holdings-view-closed = 已平仓轮次（{ $count }）
copy-holdings-reset = 重置模拟账本
copy-holdings-live-note = 实盘跟单是真实仓位。
copy-holdings-open-positions = 持仓中的仓位
copy-holdings-token-details = 打开代币详情
copy-holdings-opened = 开仓于 { $time }
copy-holdings-no-pool-price = 无流动性池价格
copy-holdings-close = 平仓
copy-holdings-write-off = 核销
copy-holdings-activity = 活动
copy-holdings-no-exit-rule = 无出场规则
copy-holdings-watch-stop = 止损 { $level }
copy-holdings-watch-stop-until = { $span }后止损 { $level }
copy-holdings-watch-take = 止盈 { $level }
copy-holdings-watch-trail = 追踪 { $level }
copy-holdings-watch-trail-arms = 追踪于 { $level } 启动
copy-holdings-watch-time = 时间 ≤ { $level }
copy-holdings-watch-time-until = { $span }后时间 ≤ { $level }
copy-holdings-watch-wallet-sells = 钱包卖出
copy-holdings-empty = 暂无模拟持仓。从钱包跟单的买入会显示在这里。
copy-holdings-col-token = 代币
copy-holdings-col-cost = 成本
copy-holdings-col-entry = 入场价
copy-holdings-col-mark = 标记价
copy-holdings-col-peak = 峰值
copy-holdings-col-pnl = 盈亏
copy-holdings-col-exit-rules = 出场规则
copy-holdings-col-held = 持有时长
copy-holdings-col-actions = 操作
copy-holdings-col-invested = 投入
copy-holdings-col-proceeds = 所得
copy-holdings-col-exit = 出场价
copy-holdings-col-closed = 平仓时间
copy-holdings-price-note = 价格以每个代币的 { -sol } 计。入场价已包含买入的滑点和手续费；峰值和出场水平均相对于入场价，因此持仓建立时其峰值低于入场价。将鼠标悬停在某项上可查看其流动性池价格。
copy-holdings-paused-rules = 已暂停：不会新增跟单。您的出场规则仍会平掉这些持仓。
copy-holdings-paused-mirror = 已暂停：不会新增跟单。钱包的卖出仍会平掉这些持仓。
copy-holdings-paused-hybrid = 已暂停：不会新增跟单。钱包的卖出和您的出场规则仍会平掉这些持仓。
copy-holdings-closed-load-failed = 无法加载已平仓轮次：{ $error }
copy-holdings-closed-loading = 正在加载已平仓轮次…
copy-holdings-closed-empty = 暂无已平仓轮次。
copy-holdings-closed-latest = 最近 { $shown } 个，共 { $total } 个轮次。
copy-holdings-close-title = 平掉模拟持仓
copy-holdings-close-message = 在模拟账本中按流动性池价格（{ $price }）并结合任务的滑点和手续费卖出 { $token }。
copy-holdings-close-confirm = 平仓
copy-holdings-write-off-title = 核销模拟持仓
copy-holdings-write-off-message = { $token } 没有可用于卖出的流动性池价格。核销会将其按零平仓，并把 { $cost } 的成本记为亏损。
copy-holdings-keep = 保留
copy-holdings-written-off = { $token } 已核销
copy-holdings-closed = { $token } 已平仓
copy-holdings-written-off-detail = 按零所得平仓
copy-holdings-sold-at = 卖出价 { $price }
copy-holdings-close-failed = 无法平仓
copy-holdings-reset-message = 重新开始“{ $name }”：其模拟持仓、花费、成交、出场和跳过记录将被移除，规则和钱包保留。
copy-holdings-reset-cancel = 保留历史
copy-holdings-reset-done = 模拟账本已重置
copy-holdings-reset-detail =
    { $count ->
       *[other] 已移除 { $count } 条决策
    }
copy-holdings-reset-failed = 无法重置模拟账本

## Activity tab

copy-activity-title = 活动
copy-activity-filter-label = 活动过滤
copy-filter-all = 全部
copy-outcome-paper-filled = 模拟买入
copy-outcome-live-submitted = 实盘买入已提交
copy-outcome-live-confirmed = 实盘买入已确认
copy-outcome-live-failed = 实盘买入失败
copy-outcome-paper-sell-observed = 模拟卖出 · 钱包已卖出
copy-outcome-live-sell-submitted = 实盘卖出已提交
copy-outcome-live-sell-failed = 实盘卖出失败
copy-outcome-skipped = 已跳过
copy-activity-decision = 决策
copy-activity-paper-exit = 模拟出场 · { $rule }
copy-activity-filled = { $input }，价格 { $price } · 钱包买入 { $target }
copy-activity-filled-slippage = { $input }，价格 { $price } · 钱包买入 { $target } · 滑点 { $slippage }
copy-activity-filled-unpriced = { $input }，价格 { $price } · 钱包买入 { $target } · 按钱包的交易价计价，无流动性池价格
copy-activity-live-sized = { $sized } · 钱包买入 { $target }
copy-activity-sell-nothing = 钱包卖出 { $amount } · 无持仓可卖
copy-activity-written-off = 已按零核销：无流动性池价格
copy-activity-sold = { $tokens } 个代币，以 { $price } 卖出，所得 { $proceeds }
copy-activity-full-close = 完全平仓
copy-activity-partial-exit = { $pct } 出场
copy-activity-skip-detail = { $label }（{ $detail }）
copy-activity-skip-minimum-size = 最小 { $amount }
copy-activity-skip-maximum = 最大 { $value }
copy-activity-skip-stale = 延迟 { $arrival }，上限 { $limit }
copy-activity-skip-latency = 平均 { $average }，上限 { $limit }
copy-activity-arrival-replayed = 区块产生 { $span }后重放
copy-activity-arrival-seen = 区块产生 { $span }后发现
copy-activity-link-wallet-tx = 钱包交易
copy-activity-link-own-tx = 您的交易
copy-activity-only-token = 仅此代币
copy-activity-skipped-group = 已跳过 ×{ $count }
copy-activity-group-detail =
    { $tokens ->
       *[other] { $tokens } 个代币 · 自 { $since }
    }
copy-activity-mint-filter =
    .placeholder = 代币铸造地址
    .aria-label = 按代币铸造地址过滤
copy-activity-clear = 清除
copy-activity-load-failed = 无法加载活动：{ $error }
copy-activity-loading = 正在加载活动…
copy-activity-no-match = 没有符合此过滤条件的内容。
copy-activity-empty = 暂无决策。随着钱包交易，成交、出场和跳过记录会显示在这里。
copy-activity-load-older = 加载更早的记录
copy-activity-start = 历史起点
copy-activity-older-failed = 无法加载更早的活动

## Task editor

copy-step-wallet = 钱包
copy-step-sizing = 仓位规模
copy-step-entry = 入场过滤
copy-step-exits = 出场
copy-step-review = 复核
copy-editor-title-edit = 编辑 { $name }
copy-editor-title-clone = 克隆 { $name }
copy-editor-sub-edit = { $mode }任务 · 更改将应用于其后续决策
copy-editor-sub-clone = 规则相同，模拟账本为空，从模拟开始
copy-editor-save-edit = 保存更改
copy-editor-save-clone = 创建克隆
copy-editor-save-create = 创建模拟任务
copy-editor-clone-suffix = （副本）
copy-editor-discard-edit = 放弃更改
copy-editor-discard-create = 放弃此任务
copy-editor-discard-edit-message = 您对“{ $name }”的更改尚未保存。
copy-editor-discard-create-message = 目前输入的钱包和规则尚未保存。
copy-editor-discard-confirm = 放弃
copy-editor-keep-editing = 继续编辑
copy-editor-toast-updated = 任务已更新
copy-editor-toast-clone = 克隆已创建
copy-editor-toast-created = 模拟任务已创建
copy-unit-sol = { -sol }
copy-editor-any = 任意
copy-editor-duplicate = 已被 { $tasks } 跟单。此任务会以自身的规则和预算再次跟单相同的交易。
copy-editor-wallet = 钱包
copy-editor-wallet-identity = 任务的钱包即其标识。若要用这些规则跟单其他钱包，请克隆该任务。
copy-editor-address-label = 钱包地址
copy-editor-address-placeholder = Solana 钱包地址
copy-editor-address-help-clone = 规则相同，模拟账本为空。可保留此钱包以测试其他规则，也可输入另一个钱包。
copy-editor-address-help-create = 此任务跟单其买入（如您选择，也包括卖出）的钱包。
copy-editor-name-label = 名称 <em>可选</em>
copy-editor-name-placeholder = 例如：快进快出
copy-editor-enabled-title = 处理该钱包的交易
copy-editor-enabled-help = 关闭后任务保持暂停，直到您恢复。
copy-editor-note-live = 此任务为实盘：更改将应用于其后续真实跟单。
copy-editor-note-paper = 任务在您启用实盘前一直以模拟方式运行：交易按流动性池价格模拟，不会花费资金。
copy-editor-copy-size = 跟单金额
copy-editor-sizing-fixed = 固定金额
copy-editor-sizing-ratio = 钱包交易的占比
copy-editor-amount-fixed = 每次跟单金额
copy-editor-amount-ratio = 每笔交易占比
copy-editor-amount-help-fixed = 每次跟单买入的花费，至少 { $minimum }。
copy-editor-amount-help-ratio = 按钱包自身买入金额的比例，不超过单笔上限。
copy-editor-help-trade-cap = 单次跟单花费不会超过此值。
copy-editor-help-token-cap = 单个代币的总花费。
copy-editor-help-budget = 此任务在其生命周期内可花费的总额；模拟和实盘各自计算自己的花费。
copy-editor-preview-title = 一次跟单的成本
copy-editor-preview-empty = 输入仓位规模以查看一次跟单的成本。
copy-editor-preview-example = 钱包买入 { $target } → 您跟单 <strong>{ $copy }</strong>
copy-editor-preview-once = 每个代币只买入一次，因此单个代币仅跟单一次，金额为 { $size }
copy-editor-preview-token-cap =
    { $count ->
       *[other] 单个代币最多跟单 { $count } 次，每次 { $size }
    }
copy-editor-preview-summary-exact = { $perToken }；预算约可覆盖 { $count } 次。网络费和优先费另计。
copy-editor-preview-summary-minimum = { $perToken }；预算至少可覆盖 { $count } 次。网络费和优先费另计。
copy-editor-target-min = 跟单的最小钱包交易
copy-editor-target-min-help = 忽略钱包较小的买入。留空表示无最小值。
copy-editor-target-max = 跟单的最大钱包交易
copy-editor-target-max-help = 忽略钱包较大的买入。留空表示无最大值。
copy-editor-buy-once-title = 每个代币仅买入一次
copy-editor-buy-once-help = 仅跟单钱包对某代币的首次买入；之后对它的买入将被跳过。
copy-editor-filter-require = 必须通过
copy-editor-filter-skip = 无需通过
copy-editor-filter-help = 要求代币先通过您的过滤流程，才会被跟单。
copy-editor-filter-warning = 在默认的过滤设置下，几乎所有代币都无法通过，因此要求通过的任务不会跟单任何代币。仅当您的过滤器能放行此钱包所交易的代币时才要求通过。
copy-editor-exit-both = 两者
copy-editor-exit-help-buy-only = 下方您的规则会卖出所有持仓；钱包的卖出将被忽略。
copy-editor-exit-help-hybrid = 以先发生者为准：钱包卖出，或您的某条规则触发。
copy-editor-exit-help-mirror = 仅在钱包卖出时才卖出持仓。您的出场规则不会运行。
copy-editor-who-sells = 由谁卖出
copy-editor-preset = 预设
copy-editor-preset-help = 预设会填充下方所有规则；之后可自行调整任意一项。
copy-editor-mirror-note = 当由钱包的卖出决定时，这些规则不会运行。若切换为“{ $mine }”或“{ $both }”，它们将生效。
copy-editor-rule-inherit = 交易引擎默认值
copy-editor-inherit-value = 交易引擎默认值（{ $value }）
copy-editor-rule-aria = { $rule }设置
copy-editor-rule-empty-uses = 留空则使用交易引擎默认值：{ $value }
copy-editor-rule-follows = 跟随交易引擎：{ $summary }
copy-editor-rule-follows-plain = 跟随交易引擎的设置。
copy-editor-rule-off-note = 此任务已关闭，无论交易引擎如何设置。
copy-editor-unnamed = 未命名任务
copy-editor-review-head = { $name } · { $mode } · { $status }
copy-editor-review-processes = 保存后处理交易
copy-editor-review-paused = 保存后为暂停状态
copy-editor-error-address = 请输入有效的 Solana 钱包地址。
copy-editor-error-sizing = 所有仓位规模数值必须大于零。
copy-editor-error-min-copy = 每次跟单至少为 { $minimum }：请提高每次跟单金额。
copy-editor-error-min-cap = 每次跟单至少为 { $minimum }：请提高单笔上限。
copy-editor-error-trade-cap = 单笔上限不能超过单代币上限。
copy-editor-error-token-cap = 单代币上限不能超过总预算。
copy-editor-error-slippage = 滑点必须在 { $min } 到 { $max } 之间。
copy-editor-error-target-limits = 钱包交易限额必须为零或更大。
copy-editor-error-target-order = 最小钱包交易不能超过最大钱包交易。

## Copy notices

copy-notice-task-unnamed = 任务 #{ $id }
copy-notice-heading = { $task }：{ $title }
copy-notice-event = { $task }：{ $title } — { $detail }
copy-notice-title-paper-buy = 模拟跟单买入
copy-notice-title-paper-sell = 模拟跟单卖出
copy-notice-title-paper-closed = 模拟持仓已平仓
copy-notice-title-paper-exit = 模拟出场：{ $rule }
copy-notice-title-live-buy-submitted = 实盘跟单买入已提交
copy-notice-title-live-buy-confirmed = 实盘跟单买入已确认
copy-notice-title-live-buy-failed = 实盘跟单买入失败
copy-notice-title-live-sell-submitted = 实盘跟单卖出已提交
copy-notice-title-live-sell-failed = 实盘跟单卖出失败
copy-notice-title-auto-paused = 跟单任务已自动暂停
copy-notice-detail-bought = 以 { $amount } { -sol } 买入
copy-notice-detail-sold = 以 { $amount } { -sol } 卖出
copy-notice-detail-sized = { $amount } { -sol }
copy-notice-detail-partial-close = 持仓的 { $percent }%
copy-notice-detail-full-close = 完全平仓
copy-notice-detail-error = { $error }
copy-notice-detail-swap-failed = 兑换失败
