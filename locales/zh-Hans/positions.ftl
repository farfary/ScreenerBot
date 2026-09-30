# Position details labels.

# State reasons. Ids come from POSITION_CREATED_REASON in src/positions/database/types.rs.
positions-state-reason-position-created = 仓位已创建


# Source: scripts/pages/positions.js

## Views, origin and toolbar

# Ids are the position status values (POSITION_STATUS_LABELS, ui/position_status.js).
positions-status-open = 持仓中
positions-status-closed = 已平仓
positions-status-archived = 已归档
positions-origin-copy = 跟单
positions-origin-manual = 手动
positions-origin-wallet = 钱包
positions-origin-copy-link =
    .title = 打开开启此仓位的跟单任务
positions-holding-frozen = 已冻结
    .title = 铸造权限已冻结此代币账户，余额无法转移或卖出
positions-toolbar-total = 总计
positions-toolbar-delete-all = 全部删除
positions-search-placeholder = 按代币符号或铸造地址搜索...
positions-filter-origin = 来源
positions-filter-origin-all = 全部来源
positions-filter-origin-auto = 自动交易
positions-filter-origin-copy = 跟单交易
positions-delete-all-tooltip = 永久删除所有已归档的仓位

## Columns

positions-column-token = 代币
positions-column-archived-at = 归档时间
positions-column-entry-time = 入场时间
positions-column-exit-time = 出场时间
positions-column-avg-entry = 入场均价（{ -sol }）
positions-column-avg-exit = 出场均价（{ -sol }）
positions-column-current-price = 当前（{ -sol }）
positions-column-total-invested = 总投入
positions-column-proceeds = 回收金额
positions-column-pnl = 盈亏
positions-column-pnl-percent = 盈亏 %
positions-column-size = 规模
positions-column-dca = DCA
positions-column-exits = 出场
positions-column-unrealized-pnl = 未实现盈亏
positions-column-unrealized-percent = 未实现 %

## Cells

# Shown instead of a figure the wallet history cannot support.
positions-unknown-basis = 此钱包的历史记录中没有成本（空投、以 USD 计价的成交，或没有 SOL 一侧的兑换）
positions-unknown-history = 本轮与链上余额不一致
positions-dca-count =
    { $count ->
       *[other] { $count } 次 DCA
    }
positions-exit-count =
    { $count ->
       *[other] { $count } 次出场
    }

## Row actions

positions-action-add =
    .title = 加仓（DCA）
    .aria-label = 加仓
positions-action-sell =
    .title = 卖出（全部或按 % 部分卖出）
    .aria-label = 卖出仓位
positions-action-sell-frozen = 已被铸造权限冻结，此持仓无法卖出
positions-action-remove =
    .title = 移除（归档或删除）
    .aria-label = 移除仓位
positions-action-restore =
    .title = 恢复为持仓中/已平仓
    .aria-label = 恢复仓位
positions-action-delete =
    .title = 永久删除
    .aria-label = 永久删除
positions-action-in-progress = 进行中…

## Live state of a row

positions-caption-buying = 买入中
# $step is the label of the current action step.
positions-caption-buying-step = 买入中 · { $step }
positions-caption-selling = 卖出中
positions-caption-selling-step = 卖出中 · { $step }
positions-caption-closing = 平仓中
positions-caption-failed = 失败
# $error is the failure text of the action.
positions-caption-failed-detail = 失败 · { $error }
positions-step-adding = 加仓中
positions-pending-buying = 买入中…
positions-pending-buy-failed = 买入失败

## Messages and confirmations

positions-load-failed = 无法刷新仓位
positions-toast-not-found = 未找到仓位数据
positions-toast-deleted = 仓位已删除
positions-toast-archived = 仓位已归档
positions-toast-restored = 仓位已恢复
positions-action-failed = 操作失败
positions-delete-title = 永久删除仓位
# $symbol is the token symbol.
positions-delete-message = 永久删除 { $symbol }？这将从数据库中移除该仓位及其历史记录，且无法撤销。您的交易和代币数据不受影响。
positions-delete-confirm = 永久删除
positions-delete-all-title = 删除所有已归档的仓位
positions-delete-all-message =
    { $count ->
       *[other] 永久删除全部 { $count } 个已归档的仓位？此操作无法撤销。交易和代币数据不受影响。
    }
positions-delete-all-message-empty = 永久删除所有已归档的仓位？此操作无法撤销。
positions-delete-all-confirm = 全部删除
positions-delete-all-done =
    { $count ->
       *[other] 已删除 { $count } 个已归档的仓位
    }
positions-delete-all-failed = 删除已归档的仓位失败

# Source: scripts/ui/position_remove_dialog.js

## Remove position dialog

positions-remove-title = 移除仓位
# Inline markup: emphasis on the opening sentence and on "not".
positions-remove-open-warning = <strong>此仓位仍在持仓中。</strong>机器人正持有该代币。移除后会释放交易名额并停止跟踪，但<strong>不会</strong>卖出。如需取回 { -sol }，请先卖出。
positions-remove-modes =
    .aria-label = 移除方式
positions-remove-archive = 归档
positions-remove-recommended = 推荐
positions-remove-archive-description = 将其收入“已归档”标签页。随时可恢复，不会卖出任何代币，所有交易记录均保留。
positions-remove-delete = 永久删除
positions-remove-delete-description = 从数据库中清除此仓位及其完整历史记录。
# Inline markup: emphasis on the irreversibility sentence.
positions-remove-danger = 这将永久移除该仓位及其历史记录。<strong>此操作无法撤销。</strong>您的交易和代币数据不受影响。
positions-remove-confirm-archive = 归档仓位

# Source: scripts/ui/position_details_dialog.js, scripts/ui/position_details/panes.js

## Position details frame

# Message shown after a management change. $mode is the label of the new mode.
positions-management-changed = 仓位管理已设为 { $mode }
positions-details-load-failed = 加载仓位详情失败
positions-details-mint-label = 铸造地址
positions-details-management-failed = 更新仓位管理失败
positions-details-favorite-add =
    .title = 添加到收藏
    .aria-label = 添加到收藏
positions-details-favorite-remove =
    .title = 从收藏中移除
    .aria-label = 从收藏中移除
positions-details-view-solscan =
    .title = 在 { -solscan } 上查看
    .aria-label = 在 { -solscan } 上查看代币
positions-details-close =
    .title = 关闭（Esc）
    .aria-label = 关闭
positions-details-chart-section =
    .aria-label = 价格图表
positions-details-loading-chart = 正在加载图表...
positions-details-activity-section =
    .aria-label = 动态
positions-details-activity-title = 动态
positions-details-split-handle =
    .aria-label = 调整图表和动态的大小
positions-details-activity-pane =
    .aria-label = 动态面板
positions-details-activity-expand =
    .title = 展开动态
    .aria-label = 展开动态
positions-details-summary-section =
    .aria-label = 仓位摘要
positions-details-loading = 正在加载仓位...

## Management modes. Ids are the PositionManagement serde ids (src/positions/types.rs).

positions-management-auto-trader = 自动交易
positions-management-user-only = 仅用户
positions-management-copy-task = 跟单任务
positions-management-hybrid = 混合
positions-pane-show-chart = 显示图表
positions-pane-show-activity = 显示动态
positions-pane-restore-activity = 恢复动态
positions-pane-expand-chart =
    .title = 展开图表
    .aria-label = 展开图表

# Source: scripts/ui/position_details/header.js

## Position details header

positions-risk-low = 低风险
positions-risk-medium = 中风险
positions-risk-high = 高风险
positions-risk-unknown = 风险未知
positions-busy-buying = 买入进行中…
positions-busy-selling = 卖出进行中…
positions-busy-closing = 平仓进行中…
positions-header-avg-entry = 入场均价
# $count is the number of buys: the entry plus each add.
positions-header-buy-count =
    { $count ->
       *[other] { $count } 次买入
    }
positions-header-exit-price = 出场价格
# $ago is the elapsed time since the close, for example "3h ago".
positions-header-closed-ago = 平仓于 { $ago }
positions-header-realized-pnl = 已实现盈亏
positions-header-usd-note = 按今日 { -sol } 价格折算的 USD
positions-header-returned = 已回收
# $amount is the formatted SOL amount invested.
positions-header-of-invested = 占已投入 { $amount }
positions-header-price = 价格
positions-header-last-price = 最新价格
positions-header-pool-ago = 流动性池 · { $ago }
positions-header-unrealized-pnl = 未实现盈亏
positions-header-pnl-last-price = 按最新价格计算的盈亏
positions-header-value = 价值
positions-header-last-value = 最新价值
positions-header-invested = 已投入 { $amount }
positions-header-origin-hint = 此仓位的开启方式
positions-header-risk-hint = { -rugcheck } 评分，越低越安全
positions-header-frozen = 已冻结
    .title = 铸造权限已冻结此持仓
positions-header-managed-by = 管理方式
positions-header-management-select =
    .aria-label = 仓位管理

## Entry origin shown in the header badge

positions-origin-unknown = 未知
# $task is the copy task id. The source wallet follows in its own element.
positions-origin-copied-task = 跟单 · 任务 { $task }
positions-origin-manual-entry = 手动入场
positions-origin-wallet-entry = 钱包入场
# $strategy is the strategy id.
positions-origin-auto-strategy = 自动 · { $strategy }
positions-origin-auto-entry = 自动入场

## Swaps that are submitted and not yet booked

positions-pending-adding = 加仓中
positions-pending-adding-amount = 加仓 { $amount }
positions-pending-selling = 卖出中
# $percent is the formatted share of the position being sold.
positions-pending-selling-percent = 卖出 { $percent }
# $label is the pending swap wording.
positions-pending-confirming = { $label } · 确认中
    .title = 已提交，等待链上确认。验证完成后数据将更新。

## Trade controls

positions-trade-add = 加仓
    .title = 对仓位加仓
positions-trade-sell = 卖出
    .title = 卖出部分仓位
positions-trade-close = 平仓
    .title = 全部卖出并平仓
positions-trade-token = 代币详情
    .title = 打开代币详情

## Favorites

positions-favorite-token-fallback = 代币
# $symbol is the token symbol.
positions-favorite-added = 已将 { $symbol } 添加到收藏
positions-favorite-removed = 已将 { $symbol } 从收藏中移除
positions-favorite-add-failed = 添加收藏失败
positions-favorite-remove-failed = 移除收藏失败
positions-favorite-update-failed = 更新收藏失败

# Source: scripts/ui/position_details/summary.js

## Summary rail

positions-summary-position = 仓位
positions-summary-price-path = 价格走势
positions-summary-network-fees = 网络费用
positions-summary-risk = 风险
positions-summary-market = 市场
positions-summary-market-now = 当前市场
positions-summary-links = 链接
positions-fact-tokens-fallback = 个代币
positions-fact-bought = 已买入
positions-fact-holding = 持仓
positions-fact-sold = 已卖出
positions-fact-realized = 已实现
positions-fact-opened = 开仓时间
positions-fact-closed = 平仓时间
positions-fact-reason = 原因
positions-fact-archived = 归档时间
positions-fact-entry = 入场
positions-fact-exit = 出场
positions-fact-total = 总计
positions-fact-verified = 已在链上验证
positions-fact-confirming = 确认中
# $percent is the formatted share, for example "12.5%".
positions-fact-share-of-bought = 占买入量 { $percent }
positions-fact-share-of-invested = 占投入量 { $percent }
# $count is the number of adds after the entry.
positions-fact-entry-count =
    { $count ->
        [0] 1 次入场
       *[other] 1 次入场 + { $count } 次加仓
    }
# $count is the number of partial exits, $returned the formatted SOL amount.
positions-fact-partial-exits-back =
    { $count ->
       *[other] { $count } 次部分出场 · 回收 { $returned }
    }
# $age is the elapsed time of the hold.
positions-fact-held = 持仓 { $age }
# $percent is the signed change against the entry price.
positions-fact-vs-entry = 较入场 { $percent }
positions-fact-exit-vs-peak = 出场价与峰值
positions-fact-now-vs-peak = 当前价与峰值
positions-fact-entry-range = 入场区间
positions-range-low = 最低
positions-range-peak = 峰值
positions-range-now = 当前
positions-range-label-exit = 入场价与出场价在最低价和峰值之间的位置
positions-range-label-now = 入场价与当前价在最低价和峰值之间的位置
positions-fact-mint-authority = 铸造权限
positions-fact-freeze-authority = 冻结权限
positions-fact-active = 有效
positions-fact-pool = 流动性池
# $amount is the formatted liquidity in SOL.
positions-fact-pool-liquidity = { $amount } { -sol } 流动性
positions-fact-market-cap = 市值
# $value is the formatted fully diluted valuation in USD.
positions-fact-fdv = FDV { $value }
positions-fact-liquidity = 流动性
positions-fact-volume-24h = 24h 成交量
positions-fact-price-change = 价格变化
positions-change-period-1h = 1h
positions-change-period-24h = 24h
positions-fact-holders = 持有者
positions-link-website = 网站
positions-link-x = X
positions-link-telegram = { -telegram }
positions-link-dexscreener = { -dexscreener }
positions-link-birdeye = { -birdeye }
positions-link-rugcheck = { -rugcheck }
positions-link-photon = { -photon }

# Source: scripts/ui/position_details/activity.js, scripts/ui/position_details/activity_event.js

## Activity

positions-activity-load-failed = 无法加载动态
positions-activity-loading = 正在加载动态...
positions-activity-empty = 此钱包中该代币尚无任何动态
positions-activity-filter-empty = 没有符合此筛选条件的动态
positions-activity-round-count =
    { $count ->
       *[other] { $count } 轮
    }
positions-activity-event-count =
    { $count ->
       *[other] { $count } 个事件
    }
positions-activity-pending-count = 待处理：{ $count }
positions-activity-failed-count = 失败：{ $count }
positions-filter-all = 全部
positions-filter-trades = 交易
positions-filter-buys = 买入
positions-filter-sells = 卖出
positions-filter-wallet = 钱包
positions-filter-issues = 异常
positions-activity-filters =
    .aria-label = 筛选动态
positions-activity-totals =
    .aria-label = 此代币的所有轮次
positions-activity-realized-all = 已实现，所有轮次
positions-activity-invested = 已投入
positions-activity-returned = 已回收
# $when is the formatted open time of a round that has not closed.
positions-activity-opened = 开仓于 { $when }
# $index is the 1-based number of the round.
positions-activity-round-title = 仓位 { $index }
positions-activity-this-position = 此仓位
positions-activity-dates-unavailable = 日期不可用
positions-activity-wallet-title = 钱包交易
# $range is the date range, $count the number of events.
positions-activity-outside =
    { $count ->
       *[other] 仓位之外 · { $range } · { $count } 个事件
    }
positions-details-signature-label = 签名

## State history milestones. Ids are the PositionState names (src/positions/database/types.rs).

positions-state-open = 仓位持仓中
positions-state-closing = 仓位平仓中
positions-state-closed = 仓位已平仓
positions-state-exit-pending = 仓位出场待处理
positions-state-exit-failed = 仓位出场失败
positions-state-phantom = 幽灵仓位
positions-state-reconciling = 仓位对账中

## Activity events

positions-event-kind-entry = 入场
positions-event-kind-dca = 加仓
positions-event-kind-partial-exit = 部分出场
positions-event-kind-exit = 出场
positions-event-kind-buy = 钱包买入
positions-event-kind-sell = 钱包卖出
positions-event-kind-transfer = 转账
positions-event-kind-ata = 代币账户
positions-event-kind-other = 交易
positions-event-state-pending = 待处理
positions-event-state-failed = 失败
positions-event-state-synthetic = 合成
# $error is the failure text reported by the chain.
positions-chain-status-failed-detail = 失败：{ $error }
positions-event-tokens-fallback = 个代币
# In the descriptions below $amount is the token amount with its symbol, $sol the SOL amount
# and $percent the share of the position sold.
positions-event-entry-submitted = 已提交买入 { $amount }
positions-event-entry-for = 以 { $sol } 买入 { $amount }
positions-event-entry = 买入 { $amount }
positions-event-dca-submitted = 已提交加仓 { $amount }
positions-event-dca-for = 以 { $sol } 加仓 { $amount }
positions-event-dca = 加仓 { $amount }
positions-event-partial-exit-submitted-percent = 已提交 { $percent } 部分出场，数量 { $amount }
positions-event-partial-exit-submitted = 已提交部分出场，数量 { $amount }
positions-event-sold-percent-for = 卖出 { $amount }（{ $percent }），得到 { $sol }
positions-event-sold-percent = 卖出 { $amount }（{ $percent }）
positions-event-sold-for = 卖出 { $amount }，得到 { $sol }
positions-event-sold = 卖出 { $amount }
positions-event-exit-submitted = 已提交全部仓位出场
positions-event-exit-for = 已平仓，卖出 { $amount }，得到 { $sol }
positions-event-exit-closed = 已平仓
positions-event-wallet-bought = 钱包在别处买入 { $amount }
positions-event-wallet-sold = 钱包在别处卖出 { $amount }
positions-event-received = 收到 { $amount }
positions-event-sent = 发送 { $amount }
positions-event-transferred = 转移 { $amount }
positions-event-ata = 代币账户活动
positions-event-wallet-transaction = 涉及 { $amount } 的钱包交易
# $price is the formatted price per token in SOL.
positions-event-price-per-token = { $price } { -sol } / 代币
# $amount is the signed SOL change of the wallet.
positions-event-wallet-change = 钱包变动 { $amount }
positions-event-after-title = 此事件后的仓位
positions-event-capital-invested = 已投入资金
positions-event-average-entry = 入场均价
positions-event-transfers-title = 代币转账
positions-event-transfer-amount = 数量
positions-event-transfer-mint = 铸造地址
positions-event-transfer-from = 转出方
positions-event-transfer-to = 转入方
positions-event-no-signature = 无链上签名
positions-event-click-to-copy = 点击复制
positions-event-solscan = { -solscan }
positions-event-token-amount = 代币数量
positions-event-trade-price = 成交价格
positions-event-sol-amount = { -sol } 数量
positions-event-cost-basis = 成本
positions-event-usd-value = USD 价值
positions-event-network-fee = 网络费用
positions-event-router = 路由
positions-event-slot = 插槽
positions-event-chain-status = 链上状态
positions-event-transaction-type = 交易类型
positions-event-direction = 方向
positions-event-wallet-sol-change = 钱包 { -sol } 变动
positions-event-instructions = 指令
positions-event-compute-units = 计算单元
positions-event-accounts = 账户
positions-event-record-id = 记录 ID
positions-event-time-unavailable = 时间不可用
positions-event-details = 详情
positions-event-hide-details = 收起详情

# Source: scripts/ui/position_details/chart.js

## Position chart

positions-chart-type-candles = K 线
positions-chart-type-line = 折线
positions-chart-type-area = 面积
positions-chart-type-group =
    .aria-label = 图表类型
positions-chart-overlays-group =
    .aria-label = 图表叠加指标
positions-chart-ema = EMA
    .title = 指数移动平均线，9 和 21
positions-chart-fit = 适配
    .title = 按此仓位的存续期显示
positions-chart-timeframes-group =
    .aria-label = 时间周期
positions-chart-pane-group =
    .aria-label = 图表面板
positions-chart-unavailable = 图表引擎不可用
positions-chart-collecting = 正在收集图表数据…
positions-chart-no-data = 此代币暂无图表数据
positions-chart-avg-entry = 入场均价
positions-chart-legend-dca = DCA
positions-chart-legend-avg-entry = 入场均价
positions-chart-legend-avg-entry-off-scale = 入场均价（超出刻度）
positions-chart-dropped-events =
    { $count ->
       *[other] { $count } 个事件在此时间周期内没有对应的 K 线
    }
positions-chart-level = 价位
# $label names the reference level, $price is its formatted price.
positions-chart-level-above = { $label } { $price } 高于当前视图
positions-chart-level-below = { $label } { $price } 低于当前视图
positions-chart-scale-hint = 拖动价格轴可缩放至该价位
positions-chart-pnl-at-bar = 该 K 线处盈亏
positions-chart-click-to-locate = 点击定位
