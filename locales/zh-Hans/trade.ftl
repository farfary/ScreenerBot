# Trade dialog messages.

## Quote panel

# Shown in the quote panel when the quote request could not reach the core.
trade-quote-error-network = 无法获取报价，请检查网络连接后重试
# Fallback title when the quote request failed without a message.
trade-quote-error-title = 无法获取报价
trade-quote-title = 兑换预览
trade-quote-refresh =
    .aria-label = 刷新报价
    .title = 刷新报价
trade-quote-idle = 选择数量以预览您的兑换
trade-quote-loading = 正在寻找最优路由…
trade-quote-retry = 重试
trade-quote-pay = 您支付
trade-quote-receive = 您收到（预估）
trade-quote-minimum = 保证最低所得
    .title = 在最大滑点下您至少能收到的数量。若成交低于该值，兑换将被回滚。
trade-quote-impact = 价格影响
trade-quote-slippage = 最大滑点
trade-quote-platform-fee = 平台费用
    .title = 0.5%，用于支持开发。已计入上方报价。
trade-quote-network-fee = 网络费用
trade-quote-route = 路由
trade-quote-disclaimer = 价格根据链上数据实时更新。若无法以高于您的保证最低所得成交，兑换将被回滚，因此您收到的不会少于所示数量。
# Price impact below the resolution of the percentage display.
trade-quote-impact-tiny = { "<0.01%" }
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-quote-impact-warning = 价格影响 { $impact } 高于您设置的 { $tolerance }% 最大滑点，此数量会推动流动性池价格。减少数量可使成交价更接近市场价。

## Units

trade-unit-native = { -sol }
trade-unit-tokens = 个代币

## Actions. Ids are the dialog actions: buy, sell, add.

trade-buy-title = 买入代币
trade-buy-subtitle = 输入 { -sol } 数量
trade-buy-confirm = 执行买入
trade-buy-hint = 留空则使用配置默认值
trade-sell-title = 卖出仓位
trade-sell-subtitle = 选择卖出比例
trade-sell-confirm = 执行卖出
trade-sell-hint = 请输入 1-100 之间的数值
trade-sell-input = 自定义比例
    .placeholder = 1-100
trade-add-title = 加仓
trade-add-subtitle = 对现有仓位 DCA
trade-add-confirm = 确认加仓
trade-add-hint = 留空则使用配置的 DCA 规模
trade-amount-input = 自定义数量
    .placeholder = 输入 { -sol } 数量

## Presets

trade-presets-quick-amount = 快捷数量
trade-presets-quick-sell = 快捷卖出
trade-presets-match-entry = 与入场一致
trade-presets-fixed-amount = 固定数量
trade-preset-partial = 少量
trade-preset-half = 一半
trade-preset-most = 大部分
trade-preset-full = 全部出场
# $label is the preset's amount.
trade-preset-select =
    .aria-label = 选择 { $label }

## Dialog chrome

trade-dialog-close =
    .aria-label = 关闭对话框
trade-input-max = 最大
    .aria-label = 使用最大值
trade-slider =
    .aria-label = 数量滑块
trade-context-available = 可用
trade-context-position-size = 仓位规模
trade-context-holdings = 持仓
trade-held-badge = 持有中
    .title = 您在此代币上有持仓中的仓位
trade-manage-title = 手动管理
trade-manage-description = 自动交易不会卖出或 DCA 此仓位。取消勾选可让其管理出场。

## Slippage

trade-slippage-label = 滑点
trade-slippage-presets =
    .aria-label = 滑点预设
trade-slippage-auto = 自动
trade-slippage-custom =
    .placeholder = 自定义
    .aria-label = 自定义滑点百分比
trade-slippage-note-auto = 自动（来自设置）
# $pct is the configured slippage as stored.
trade-slippage-note-auto-value = 自动（设置中为 { $pct }%）
# $pct is the override as typed.
trade-slippage-note-override = 覆盖：{ $pct }%
# $pct is the override as typed.
trade-slippage-warning = 滑点较高：实际收到的数量可能比报价最多少 { $pct }%。
trade-impact-warning-title = 高价格影响警告
# $impact is a formatted percentage and $tolerance the slippage tolerance as configured.
trade-impact-warning-text = 此笔交易的价格影响为 <strong>{ $impact }</strong>，超出了您 <strong>{ $tolerance }%</strong> 的滑点容忍度。您实际收到的数量可能远低于预期。
trade-impact-warning-proceed = 仍然继续

## Validation and verification

trade-error-invalid-number = 数字无效
trade-error-percentage-range = 比例必须在 1 到 100 之间
trade-error-amount-positive = 数量必须大于 0
trade-error-amount-minimum = 最小值：0.001 { -sol }
# $needed is a formatted SOL amount, $reserve the fee headroom in SOL and $balance the formatted balance.
trade-error-insufficient = 余额不足（需要 { $needed }，另加 { $reserve } 作为费用，当前有 { $balance }）
trade-error-position-closed = 此仓位已不再持仓中。
trade-error-verify-failed = 无法验证代币余额
trade-error-position-missing = 未找到仓位，它可能已被平仓
# $expected and $current are formatted token amounts.
trade-error-balance-changed = 代币余额已变化。预期 { $expected }，现为 { $current }。请刷新。
trade-error-verify-network = 验证余额时出现网络错误

## Quick trade

trade-quick-buy-title = 快速买入
trade-quick-sell-title = 快速卖出
trade-quick-subtitle = 输入代币铸造地址
trade-quick-mint-label = 输入代币铸造地址
trade-quick-mint-input =
    .placeholder = 输入铸造地址或按符号搜索...
trade-quick-paste =
    .aria-label = 从剪贴板粘贴
trade-quick-recent = 最近：
trade-quick-fetching = 正在获取代币信息...
trade-quick-continue = 继续
trade-quick-token-not-found = 未找到代币
trade-quick-token-failed = 获取代币失败
trade-quick-token-not-in-database = 数据库中未找到该代币
trade-quick-token-info-failed = 获取代币信息失败
trade-quick-no-position = 未找到此代币的仓位
trade-quick-no-holdings = 该仓位已没有剩余代币
trade-quick-position-failed = 获取仓位数据失败

## Manual trade toasts

trade-toast-no-mint = 没有可用的铸造地址
trade-toast-open-failed = 无法打开交易对话框
trade-toast-pending-buy = 买入仍在进行
trade-toast-pending-add = 加仓仍在进行
trade-toast-pending-sell = 卖出仍在进行
trade-toast-pending-message = 浏览器已停止等待，请在仓位行中查看结果
trade-toast-failed-buy = 买入失败
trade-toast-failed-add = 加仓失败
trade-toast-failed-sell = 卖出失败

# Trade and close reasons. Ids are the Debug names of TradeReason
# (src/trader/types.rs) and the reasons written by src/positions.
trade-reason-strategy-signal = 策略信号
trade-reason-manual-entry = 手动入场
trade-reason-force-buy = 强制买入
trade-reason-copy-buy = 跟单买入
trade-reason-dca-scheduled = 计划 DCA
trade-reason-take-profit = 止盈
trade-reason-stop-loss = 止损
trade-reason-trailing-stop = 追踪止损
trade-reason-time-override = 时间覆盖
trade-reason-strategy-exit = 策略出场
trade-reason-llm-analysis-exit = LLM 分析出场
trade-reason-manual-exit = 手动出场
trade-reason-risk-management = 风险管理
trade-reason-blacklisted = 已列入黑名单
trade-reason-force-sell = 强制卖出
trade-reason-copy-sell = 跟单卖出
trade-reason-closed-externally = 外部平仓
trade-reason-wallet-history = 钱包历史
trade-reason-exit-retry-pending = 出场重试待处理
trade-reason-synthetic-exit-permanent-failure = 合成出场永久失败
# $reason is the label of the base reason. Applies to a closed_reason that
# carries the pending-verification suffix.
trade-reason-pending-verification = { $reason }（待验证）
# $note is the operator text of a force close.
trade-reason-force-closed = 强制平仓：{ $note }
# $reason is a stored closed_reason that has no label; it is shown as stored.
trade-reason-stored = { $reason }

# Toast shown when a quick-trade shortcut runs without a token selected (ui/quick_trade_shortcuts.js).
trade-quick-no-token = 未选择代币
