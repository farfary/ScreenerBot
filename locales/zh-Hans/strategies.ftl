# Strategies page: the strategy list, the condition editor and the condition catalog.
# Condition text is addressed by the keys the schemas carry (src/strategies/conditions/catalog.rs):
#   strategies-condition-<type>                        name, with `.description`
#   strategies-condition-<type>-param-<param>          parameter name, with `.description`
#   strategies-condition-<type>-param-<param>-option-<value>
#   strategies-condition-category-<slug>
#   strategies-condition-param-timeframe / -timeframe-option-<value>   shared by every condition

## Strategy list

strategies-filter-all = 全部
strategies-filter-entry = 入场
strategies-filter-exit = 出场
strategies-type-entry = 入场
strategies-type-exit = 出场
strategies-list-empty-title = 暂无策略
strategies-list-empty-hint = 创建您的第一个策略
strategies-new = 新建策略
strategies-import =
    .title = 导入策略
    .aria-label = 导入策略
strategies-item-enable =
    .title = 启用
strategies-item-disable =
    .title = 停用

# Name given to a strategy before it is saved.
strategies-new-name = 新建策略

## Editor

strategies-editor-name =
    .placeholder = 策略名称
strategies-editor-dirty =
    .title = 有未保存的更改
strategies-action-validate = 验证
strategies-editor-empty = 请选择要编辑的策略，或新建一个
strategies-conditions-empty-title = 暂无条件
strategies-conditions-empty-hint = 使用“{ strategies-add-condition }”开始构建
strategies-add-condition = 添加条件
strategies-modal-close =
    .aria-label = 关闭
strategies-card-move-up =
    .title = 上移
strategies-card-move-down =
    .title = 下移
strategies-card-duplicate =
    .title = 复制
strategies-card-delete =
    .title = 删除
# $name is the condition name.
strategies-card-delete-confirm = 移除条件
    .message = 要从此策略中移除“{ $name }”吗？

# Card summary: one "label: value" entry per parameter.
strategies-summary-param = { $label }：{ $value }
strategies-summary-none = 无参数
# An unset optional parameter: the strategy's own value it falls back to.
strategies-param-inherit = 策略设置（{ $value }）
# A duration amount with its unit, and an hour of the UTC day. $amount is the formatted number.
strategies-value-duration-seconds = { $amount } 秒
strategies-value-duration-minutes = { $amount } 分钟
strategies-value-duration-hours = { $amount } 小时
strategies-value-duration-days = { $amount } 天
strategies-value-utc-hour = { $amount }:00 UTC
# A hint under a field, display only. $time is a local clock time, $amount a percentage.
strategies-hint-local-time = 你的本地时间：{ $time }
strategies-hint-price-impact = 此规模的交易会使价格变动约 { $amount }%

# Parameter values in a card summary. $count selects the plural, $amount is the formatted number.
strategies-value-percent = { $amount }%
strategies-value-multiplier = { $amount }×
strategies-value-hours =
    { $count ->
       *[other] { $amount } 小时
    }
strategies-value-candles =
    { $count ->
       *[other] { $amount } 根 K 线
    }

# Text written beside a numeric input.
strategies-unit-percent = %
strategies-unit-native = { -sol }
strategies-unit-hours = 小时
strategies-unit-multiplier = ×
strategies-unit-utc = UTC

## Condition catalog

strategies-catalog-search =
    .placeholder = 搜索条件...
    .aria-label = 搜索条件
strategies-catalog-search-clear =
    .aria-label = 清除搜索
strategies-catalog-no-description = 暂无描述

# Editor header: the timeframe is the default for candle conditions that leave theirs unset.
strategies-editor-active = 启用
strategies-editor-timeframe = 默认时间周期
    .title = 未设置自身时间周期的条件所使用的 K 线时间周期
strategies-editor-priority = 优先级
    .title = 评估顺序，范围 1 到 100；数值越小越先评估
strategies-editor-description = 描述
strategies-editor-description-field =
    .placeholder = 此策略的作用
    .aria-label = 策略描述
# The exit strategy's binding to the entry strategies whose positions it manages.
strategies-editor-entry-binding-all = 所有入场策略
# Labels the button that names the entry strategies an exit strategy manages.
strategies-editor-entry-binding-label = 管理
strategies-editor-entry-binding-hint = 管理所勾选入场策略开立的仓位。未勾选任何策略时，管理所有仓位，包括手动和跟单仓位。
strategies-editor-entry-binding-empty = 还没有入场策略
# $id is the id of an entry strategy that no longer exists.
strategies-editor-entry-binding-deleted = { $id }（已删除）
strategies-action-export = 导出
strategies-action-test = 用代币测试
# $token is the tested token's symbol or address.
strategies-test-title = 用 { $token } 测试
# $token is the tested token's symbol or address.
strategies-test-running = 正在用 { $token } 测试…
strategies-test-verdict-pass = 满足
strategies-test-verdict-fail = 不满足
strategies-test-verdict-unknown = 未知

# Rule groups. $mode is the all/any select written into the sentence.
strategies-rules-match = 满足这些条件中的{ $mode }
strategies-group-match = 满足其中的{ $mode }
strategies-match-all = 全部
strategies-match-any = 任意一个
strategies-match-mode =
    .aria-label = 全部或任意一个条件
strategies-invert = 反转
    .title = 反转：当此项不匹配时为真
strategies-chip-off = 关闭
strategies-chip-inverted = 已反转
strategies-row-switch =
    .title = 关闭后仍保留，但不再评估
    .aria-label = 由此策略评估
strategies-row-expand =
    .aria-label = 显示或隐藏参数
strategies-add-group = 添加分组
# $count is the number of conditions in the group.
strategies-group-delete-confirm = 移除分组
    .message = 要移除此分组及其中的 { $count } 个条件吗？

# Problems that block saving. $first is the first problem, $name the condition, $field the parameter.
strategies-problems = 有 { $count } 个问题待修复：{ $first }
strategies-problem-located = { $name }：{ $problem }
strategies-problem-located-field = { $name }，{ $field }：{ $problem }
strategies-problem-rules-empty = 请至少添加一个已启用的条件
strategies-problem-group-empty = 分组至少需要一个已启用的条件
strategies-problem-not-applicable = 此条件不能用于这种类型的策略
strategies-problem-unknown-condition = 未知的条件类型：{ $type }
strategies-problem-too-deep = 规则最多嵌套 { $max } 层；反转的分组多占一层
strategies-problem-required = 请输入数字
strategies-problem-range = 必须介于 { $min } 和 { $max } 之间
strategies-problem-invalid-option = 请从列出的选项中选择

# Catalog dialog: the data a condition reads and why one is unavailable.
strategies-catalog-all = 全部
strategies-catalog-categories =
    .aria-label = 分类
strategies-catalog-empty = 没有符合搜索的条件
strategies-catalog-exit-only = 仅限出场策略
strategies-catalog-entry-only = 仅限入场策略
strategies-catalog-candles-note = { $description }只读取 K 线。
strategies-data-candles = K 线
strategies-data-pool = 池
strategies-data-position = 仓位
strategies-data-market = 市场活动
strategies-data-clock = 时段

# Trade counts: $count selects the plural, $amount is the formatted number.
strategies-unit-trades = 笔
strategies-value-trades = { $amount } 笔

# Unsaved changes, import and export. $max is the deepest level a rule tree may reach.
strategies-toast-too-deep = 层级过多
    .message = 规则最多嵌套 { $max } 层
strategies-toast-priority-invalid = 优先级无效
    .message = 请输入 1 到 100 的整数
strategies-toast-exported = 策略已导出
# $name is the strategy name.
strategies-unsaved-confirm = 要放弃未保存的更改吗？
    .message = “{ $name }”有尚未保存的更改。
strategies-unsaved-discard = 放弃
strategies-import-error-json = 文件不是有效的 JSON
strategies-import-error-shape = 文件未描述任何策略
# $type is the condition type id, $name the condition name.
strategies-import-error-unknown-condition = 文件使用了此版本没有的条件：{ $type }
strategies-import-error-not-applicable = “{ $name }”不能用于这种类型的策略

## New strategy dialog

strategies-create-title = 新建策略
strategies-create-prompt = 请选择要创建的策略类型：
strategies-create-entry-name = 入场策略
strategies-create-entry-description = 定义何时买入代币的条件
strategies-create-exit-name = 出场策略
strategies-create-exit-description = 定义何时卖出代币的条件

## Delete dialog

strategies-delete-title = 删除策略
# $name is the strategy name.
strategies-delete-message = 删除策略“{ $name }”？此操作无法撤销。

## Toasts. A message value is the title; `.message` is the body.

strategies-toast-fix-validation = 请先修复验证错误再保存
strategies-toast-load-failed = 加载失败
    .message = 从服务器加载策略失败
strategies-toast-load-strategy-failed = 加载策略失败
strategies-toast-no-strategy = 尚未创建策略
    .message = 请至少添加一个条件，或先点击“新建策略”创建策略
strategies-toast-name-required = 需要名称
    .message = 保存前请输入策略名称
strategies-toast-saved = 策略已保存
    .message = “{ $name }”已成功保存
strategies-toast-save-failed = 保存失败
    .message = 将策略保存到数据库失败
strategies-toast-no-strategy-validate = 没有可验证的策略
strategies-toast-no-strategy-test = 没有可测试的策略
strategies-toast-test-save-first = 测试前请先保存策略：测试读取已保存的规则
strategies-toast-test-failed = 测试失败
    .message = 无法测试策略
strategies-toast-valid = 策略有效
strategies-toast-invalid = 策略存在错误
strategies-toast-validation-failed = 验证失败
strategies-toast-item-enabled = 策略已启用
strategies-toast-item-disabled = 策略已停用
strategies-toast-item-toggle-failed = 切换策略失败
strategies-toast-deleted = 策略已删除
    .message = “{ $name }”已成功移除
strategies-toast-delete-failed = 删除失败
    .message = 从数据库删除策略失败
strategies-toast-imported = 策略已导入
strategies-toast-import-failed = 导入策略失败
strategies-toast-unknown-condition = 未知条件
    .message = 未找到该条件类型
strategies-toast-create-first = 请先创建策略
    .message = 添加条件之前，请先点击“新建策略”创建策略

## Conditions

strategies-condition-candle-size = K 线大小形态
    .description = 检测特定的 K 线形态：大实体、小实体（十字星）、长影线。
strategies-condition-candle-size-param-pattern = 形态类型
    .description = 要检测的 K 线形态
strategies-condition-candle-size-param-pattern-option-large-body = 大实体（强势行情）
strategies-condition-candle-size-param-pattern-option-small-body = 小实体（十字星/犹豫）
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 长上影线（受阻）
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 长下影线（支撑）
strategies-condition-candle-size-param-threshold = 大小阈值 %
    .description = 形态检测的百分比阈值

strategies-condition-consecutive-candles = 连续 K 线
    .description = 检测连续的绿色（看涨）或红色（看跌）K 线，并带有最小幅度过滤。
strategies-condition-consecutive-candles-param-count = K 线数量
    .description = 所需的连续 K 线数量
strategies-condition-consecutive-candles-param-direction = K 线方向
    .description = 连续 K 线的颜色/方向
strategies-condition-consecutive-candles-param-direction-option-green = 绿色（看涨）
strategies-condition-consecutive-candles-param-direction-option-red = 红色（看跌）
strategies-condition-consecutive-candles-param-minimum-change = 最小变化 %
    .description = 每根 K 线的最小 % 变化（过滤噪音）

strategies-condition-liquidity-level = 流动性池流动性水平
    .description = 检查以 { -sol } 计的流动性池流动性（入场：确保流动性充足；出场：检测流动性流失）
strategies-condition-liquidity-level-param-threshold = 流动性阈值（{ -sol }）
    .description = 以 { -sol } 计的流动性池流动性水平
strategies-condition-liquidity-level-param-comparison = 比较方式
    .description = 如何将流动性池流动性与阈值比较
strategies-condition-liquidity-level-param-comparison-option-greater-than = 大于（>）
strategies-condition-liquidity-level-param-comparison-option-greater-equal = 大于或等于（≥）
strategies-condition-liquidity-level-param-comparison-option-less-than = 小于（{ "<" }）
strategies-condition-liquidity-level-param-comparison-option-less-equal = 小于或等于（≤）

strategies-condition-position-holding-time = 仓位持仓时间
    .description = 检查仓位已持有的时长
strategies-condition-position-holding-time-param-hours = 时间阈值（小时）
    .description = 自仓位开启以来的小时数
strategies-condition-position-holding-time-param-comparison = 比较方式
    .description = 如何将仓位持有时长与阈值比较
strategies-condition-position-holding-time-param-comparison-option-greater-than = 长于（>）
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 至少（≥）
strategies-condition-position-holding-time-param-comparison-option-less-than = 短于（{ "<" }）
strategies-condition-position-holding-time-param-comparison-option-less-equal = 至多（≤）

strategies-condition-price-breakout = 价格突破
    .description = 检测价格向上突破阻力位（周期高点）或向下跌破支撑位（周期低点）。
strategies-condition-price-breakout-param-lookback = 回溯期
    .description = 用于确定支撑/阻力位的 K 线数量
strategies-condition-price-breakout-param-direction = 突破方向
    .description = 突破的方向
strategies-condition-price-breakout-param-direction-option-upward = 向上（突破阻力）
strategies-condition-price-breakout-param-direction-option-downward = 向下（跌破支撑）
strategies-condition-price-breakout-param-confirmation = 确认幅度 %
    .description = 超过该价位多远才确认突破（避免假信号）

strategies-condition-price-change-percent = 价格变化 %
    .description = 检查价格在一段时间内的变化是否达到百分比阈值。
strategies-condition-price-change-percent-param-percentage = 变化阈值 %
    .description = 触发条件的价格变化百分比（0.1-1000%）
strategies-condition-price-change-percent-param-direction = 方向
    .description = 价格变动方向
strategies-condition-price-change-percent-param-direction-option-above = 上涨（+%）
strategies-condition-price-change-percent-param-direction-option-below = 下跌（-%）
strategies-condition-price-change-percent-param-direction-option-within = 区间内（±%）
strategies-condition-price-change-percent-param-time-value = 回溯期
    .description = 向前回溯测量变化的时长，单位在字段内选择（1-3600 秒、1-1440 分钟、1-720 小时）
strategies-condition-price-change-percent-param-time-unit = 时间单位
    .description = 回溯期的时间单位
strategies-condition-price-change-percent-param-time-unit-option-seconds = 秒
strategies-condition-price-change-percent-param-time-unit-option-minutes = 分钟
strategies-condition-price-change-percent-param-time-unit-option-hours = 小时

strategies-condition-price-to-ma = 价格与移动平均线
    .description = 检查价格在其简单移动平均线的上方、下方还是区间内。
strategies-condition-price-to-ma-param-period = MA 周期
    .description = 计算移动平均线所用的 K 线数量
strategies-condition-price-to-ma-param-position = 位置
    .description = 价格相对于 MA 的位置
strategies-condition-price-to-ma-param-position-option-above = 高于 MA
strategies-condition-price-to-ma-param-position-option-below = 低于 MA
strategies-condition-price-to-ma-param-position-option-within = 区间内
strategies-condition-price-to-ma-param-distance = 距离 %
    .description = 与 MA 的最小距离（高于/低于时）或最大区间（区间内时）

strategies-condition-volume-spike = 成交量激增
    .description = 检测相对平均成交量的成交量激增（表明关注度上升）。
strategies-condition-volume-spike-param-lookback = 回溯期
    .description = 用于计算平均成交量的 K 线数量
strategies-condition-volume-spike-param-multiplier = 成交量倍数
    .description = 高于平均值的倍数（例如 2.0 = 平均值的 200%）

strategies-condition-rsi = RSI（相对强弱指数）
    .description = 当基于已收盘 K 线、采用威尔德平滑的 RSI 高于或低于阈值时触发。
strategies-condition-rsi-param-period = RSI 周期
    .description = 计算 RSI 所用的 K 线数量；条件需要比该值多一根 K 线
strategies-condition-rsi-param-comparison = 比较方式
    .description = RSI 必须低于还是高于阈值
strategies-condition-rsi-param-comparison-option-below = 低于（超卖）
strategies-condition-rsi-param-comparison-option-above = 高于（超买）
strategies-condition-rsi-param-threshold = RSI 阈值
    .description = 1 到 99 之间的 RSI 水平（常用 30 表示超卖、70 表示超买）

strategies-condition-ma-cross = 移动平均线交叉
    .description = 当快速移动平均线在最近几根 K 线内穿过慢速移动平均线时触发。
strategies-condition-ma-cross-param-ma-type = 均线类型
    .description = 指数均线给近期 K 线更高权重，简单均线对所有 K 线权重相同
strategies-condition-ma-cross-param-ma-type-option-ema = 指数（EMA）
strategies-condition-ma-cross-param-ma-type-option-sma = 简单（SMA）
strategies-condition-ma-cross-param-fast-period = 快线周期
    .description = 快速均线包含的 K 线数量
strategies-condition-ma-cross-param-slow-period = 慢线周期
    .description = 慢速均线包含的 K 线数量；必须大于快线周期
strategies-condition-ma-cross-param-direction = 交叉方向
    .description = 快线以哪个方向穿过慢线
strategies-condition-ma-cross-param-direction-option-bullish = 看涨（快线上穿）
strategies-condition-ma-cross-param-direction-option-bearish = 看跌（快线下穿）
strategies-condition-ma-cross-param-within-candles = 交叉发生在最近
    .description = 交叉可以出现在最近多少根 K 线内；1 表示仅最新一根

strategies-condition-position-profit = 仓位盈利
    .description = 当持仓的未实现盈利高于或低于入场价格的指定百分比时触发（按流动性池价格计算）。
strategies-condition-position-profit-param-comparison = 比较方式
    .description = 盈利必须不低于阈值，还是不高于阈值
strategies-condition-position-profit-param-comparison-option-above = 不低于（≥）
strategies-condition-position-profit-param-comparison-option-below = 不高于（≤）
strategies-condition-position-profit-param-threshold-pct = 盈利阈值 %
    .description = 未实现盈利占入场价格的百分比；负值表示亏损

strategies-condition-drawdown-from-peak = 从高点回撤
    .description = 当流动性池价格从上次买入以来的最高价下跌指定百分比，且该高点已足够高于入场价格时触发。
strategies-condition-drawdown-from-peak-param-drop-pct = 从高点回撤 %
    .description = 当前价格必须比高点低多少
strategies-condition-drawdown-from-peak-param-arm-after-profit-pct = 盈利后启用 %
    .description = 高点必须至少高于入场价格这一比例，条件才可能触发；0 表示始终启用

strategies-condition-buy-pressure = 买入压力
    .description = 当买入占某个时间窗口内交易的比例不低于指定值，且该窗口交易数量足够时触发。读取代币的买入和卖出笔数。
strategies-condition-buy-pressure-param-window = 时间窗口
    .description = 买入和卖出笔数所覆盖的时间段
strategies-condition-buy-pressure-param-window-option-5m = 5 分钟
strategies-condition-buy-pressure-param-window-option-1h = 1 小时
strategies-condition-buy-pressure-param-window-option-6h = 6 小时
strategies-condition-buy-pressure-param-window-option-24h = 24 小时
strategies-condition-buy-pressure-param-min-buy-share-pct = 最低买入占比 %
    .description = 买入占窗口内全部交易的百分比
strategies-condition-buy-pressure-param-min-transactions = 最低交易笔数
    .description = 交易笔数少于该值的窗口永远不会触发条件

strategies-condition-transaction-velocity = 交易速度
    .description = 当最近窗口的交易速率至少或至多为包含它的较长窗口速率的某个倍数时触发。读取代币的买入和卖出笔数。
strategies-condition-transaction-velocity-param-window = 窗口
    .description = 最近窗口及与之比较的较长窗口
strategies-condition-transaction-velocity-param-window-option-5m-vs-1h = 5 分钟 对 1 小时
strategies-condition-transaction-velocity-param-window-option-1h-vs-6h = 1 小时 对 6 小时
strategies-condition-transaction-velocity-param-window-option-6h-vs-24h = 6 小时 对 24 小时
strategies-condition-transaction-velocity-param-comparison = 比较
    .description = 最近速率必须至少还是至多为该比率
strategies-condition-transaction-velocity-param-comparison-option-above = 至少（≥）
strategies-condition-transaction-velocity-param-comparison-option-below = 至多（≤）
strategies-condition-transaction-velocity-param-ratio = 速率比
    .description = 最近窗口每秒交易数除以长窗口每秒交易数
strategies-condition-transaction-velocity-param-min-transactions = 最少交易数
    .description = 交易数少于此值的最近窗口永不触发条件

strategies-condition-pullback-from-high = 高点回撤
    .description = 当实时价格位于最近 K 线最高价下方最小跌幅与最大跌幅之间时触发。
strategies-condition-pullback-from-high-param-lookback = 回溯周期
    .description = 用于取最高价的 K 线数量
strategies-condition-pullback-from-high-param-min-drop-pct = 最小跌幅 %
    .description = 价格必须至少低于最高价这么多
strategies-condition-pullback-from-high-param-max-drop-pct = 最大跌幅 %
    .description = 价格最多低于最高价这么多；必须大于最小跌幅

strategies-condition-profit-giveback = 利润回吐
    .description = 当持仓自上次买入以来的最佳未实现收益达到某一水平后又回吐了一部分时触发。
strategies-condition-profit-giveback-param-arm-after-profit-pct = 盈利后启用 %
    .description = 最佳收益须先达到入场价的此百分比，条件才能触发
strategies-condition-profit-giveback-param-giveback-pct = 回吐 %
    .description = 最佳收益中回吐的比例；100% 表示价格回到入场价

strategies-condition-token-age = 代币年龄
    .description = 当代币链上创建以来的时间长于或短于某一时长时触发。创建时间未知的代币会使条件保持未定。
strategies-condition-token-age-param-comparison = 比较
    .description = 代币必须早于还是晚于该时长
strategies-condition-token-age-param-comparison-option-older-than = 早于（≥）
strategies-condition-token-age-param-comparison-option-younger-than = 晚于（{ "<" }）
strategies-condition-token-age-param-time-value = 年龄
    .description = 代币创建以来的时间，单位在字段内选择（1-10000）
strategies-condition-token-age-param-time-unit = 时间单位
    .description = 年龄的单位
strategies-condition-token-age-param-time-unit-option-minutes = 分钟
strategies-condition-token-age-param-time-unit-option-hours = 小时
strategies-condition-token-age-param-time-unit-option-days = 天

strategies-condition-vwap = VWAP
    .description = 当实时价格高于或低于最近 K 线的成交量加权平均价至少一定距离时触发。
strategies-condition-vwap-param-lookback = 回溯周期
    .description = 成交量加权平均覆盖的 K 线数量
strategies-condition-vwap-param-position = 位置
    .description = 价格相对 VWAP 的位置
strategies-condition-vwap-param-position-option-above = 高于 VWAP
strategies-condition-vwap-param-position-option-below = 低于 VWAP
strategies-condition-vwap-param-distance-pct = 距离 %
    .description = 与 VWAP 的最小距离；0 表示所选一侧的任意距离

strategies-condition-volatility-range = 波动区间
    .description = 当最近 K 线的平均真实波幅占实时价格的百分比高于或低于阈值时触发。一个组中放两个即构成区间。
strategies-condition-volatility-range-param-period = ATR 周期
    .description = 平滑平均真实波幅所用的 K 线数量；条件需要比此多一根 K 线
strategies-condition-volatility-range-param-comparison = 比较
    .description = 波幅必须高于还是低于阈值
strategies-condition-volatility-range-param-comparison-option-above = 高于
strategies-condition-volatility-range-param-comparison-option-below = 低于
strategies-condition-volatility-range-param-threshold-pct = 波幅阈值 %
    .description = 平均真实波幅占实时价格的百分比

strategies-condition-bollinger-bands = 布林带
    .description = 当实时价格低于下轨或高于上轨，或布林带收窄时触发。上下轨位于平均收盘价上下若干个标准差处。
strategies-condition-bollinger-bands-param-period = 周期
    .description = 均值和标准差覆盖的 K 线数量
strategies-condition-bollinger-bands-param-std-dev = 标准差
    .description = 每条轨道距均值的标准差倍数
strategies-condition-bollinger-bands-param-signal = 信号
    .description = 布林带须呈现的状态
strategies-condition-bollinger-bands-param-signal-option-below-lower = 低于下轨
strategies-condition-bollinger-bands-param-signal-option-above-upper = 高于上轨
strategies-condition-bollinger-bands-param-signal-option-squeeze = 收窄
strategies-condition-bollinger-bands-param-squeeze-width-pct = 收窄宽度 %
    .description = 收窄时：上下轨之间的最大距离，占均值的百分比

strategies-condition-liquidity-to-size = 流动性与规模
    .description = 当池储备至少为或低于所涉规模的某个倍数时触发：入场策略中为交易规模，出场策略中为持仓剩余价值。
strategies-condition-liquidity-to-size-param-comparison = 比较
    .description = 储备必须至少为还是低于该倍数
strategies-condition-liquidity-to-size-param-comparison-option-at-least = 至少（≥）
strategies-condition-liquidity-to-size-param-comparison-option-below = 低于（{ "<" }）
strategies-condition-liquidity-to-size-param-multiple = 流动性倍数
    .description = 池储备除以所涉规模；倍数 N 使价格变动约 1/(N+1)

strategies-condition-position-adds = 持仓加仓
    .description = 当首次入场后的加仓次数或部分卖出次数至少或至多为某个数时触发。
strategies-condition-position-adds-param-subject = 计数对象
    .description = 计数哪些操作
strategies-condition-position-adds-param-subject-option-adds = 加仓买入
strategies-condition-position-adds-param-subject-option-partial-sells = 部分卖出
strategies-condition-position-adds-param-comparison = 比较
    .description = 次数必须至少还是至多为该数
strategies-condition-position-adds-param-comparison-option-at-least = 至少（≥）
strategies-condition-position-adds-param-comparison-option-at-most = 至多（≤）
strategies-condition-position-adds-param-count = 次数
    .description = 操作次数

strategies-condition-time-of-day = 时段
    .description = 当当前小时处于 UTC 小时窗口内时触发。开始晚于结束时跨越午夜。
strategies-condition-time-of-day-param-start-hour = 开始小时（UTC）
    .description = 窗口的第一个小时，0 到 23
strategies-condition-time-of-day-param-end-hour = 结束小时（UTC）
    .description = 窗口结束的小时，1 到 24；该小时本身不在窗口内

strategies-condition-liquidity-change-since-entry = 入场以来流动性变化
    .description = 当池储备自开仓以来下降或上升至少某个百分比时触发。未记录入场流动性的持仓会使条件保持未定。
strategies-condition-liquidity-change-since-entry-param-comparison = 比较
    .description = 储备必须是下降还是上升
strategies-condition-liquidity-change-since-entry-param-comparison-option-dropped-by = 下降
strategies-condition-liquidity-change-since-entry-param-comparison-option-rose-by = 上升
strategies-condition-liquidity-change-since-entry-param-change-pct = 变化 %
    .description = 入场以来池储备的变化

## Shared by every condition

strategies-condition-param-timeframe = 时间周期
    .description = K 线大小：条件读取的每根 K 线的时长（未设置时使用策略的时间周期）
strategies-condition-timeframe-option-1m = 1 分钟
strategies-condition-timeframe-option-5m = 5 分钟
strategies-condition-timeframe-option-15m = 15 分钟
strategies-condition-timeframe-option-1h = 1 小时
strategies-condition-timeframe-option-4h = 4 小时
strategies-condition-timeframe-option-12h = 12 小时
strategies-condition-timeframe-option-1d = 1 天

## Condition categories

strategies-condition-category-price-analysis = 价格分析
strategies-condition-category-candle-patterns = K 线形态
strategies-condition-category-technical-indicators = 技术指标
strategies-condition-category-market-context = 市场环境
strategies-condition-category-position-performance = 仓位与表现
strategies-condition-category-volume-analysis = 成交量分析

## Validation errors
# Each validation error is a `UiText`; the tokens below name what the message refers to.

strategies-error-missing-parameter = 缺少 { $field } 参数
strategies-error-parameter-type = { $field } 参数必须是 { $expected }
strategies-error-invalid-value = “{ $value }”不是有效的 { $field }
strategies-error-missing-data = { $data } 不可用
strategies-error-no-candle-data = { $timeframe } 时间周期没有 K 线数据
strategies-error-insufficient-history = { $indicator } 的历史数据不足：可用 { $available } 秒，需要 { $required } 秒
strategies-error-insufficient-candles = { $indicator } 的 K 线数量不足：现有 { $available }，需要 { $required }
strategies-error-stale-candle-data = { $timeframe } K 线数据已过期：其时长 { $age } 秒超过了 { $max } 秒
strategies-error-stale-market-data = { $window } 交易笔数已过期：其时长 { $age } 秒超过了 { $max } 秒
strategies-error-unusable-data = { $field } 的值“{ $value }”无法使用
strategies-error-invalid-rule-tree = 规则树无效：{ $reason }
strategies-error-evaluation-timeout = 策略评估在 { $timeout } 毫秒后超时
strategies-error-condition-not-applicable = { $condition } 无法用于{ $strategy }策略
strategies-error-invalid-rules = 无法读取规则：{ $reason }

# Parameter names

strategies-error-field-average-volume = 平均成交量
strategies-error-field-candle-open = K 线开盘价
strategies-error-field-comparison = 比较方式
strategies-error-field-condition-type = 条件类型
strategies-error-field-confirmation = 确认幅度
strategies-error-field-count = 数量
strategies-error-field-current-price = 当前价格
strategies-error-field-direction = 方向
strategies-error-field-distance = 距离
strategies-error-field-hours = 小时数
strategies-error-field-lookback = 回溯期
strategies-error-field-minimum-change = 最小变化
strategies-error-field-multiplier = 倍数
strategies-error-field-pattern = 形态
strategies-error-field-percentage = 百分比
strategies-error-field-period = 周期
strategies-error-field-position = 位置
strategies-error-field-threshold = 阈值
strategies-error-field-time-unit = 时间单位
strategies-error-field-time-value = 时间数值
strategies-error-field-timeframe = 时间周期
strategies-error-field-arm-after-profit = 盈利后启用
strategies-error-field-drop-percentage = 回撤比例
strategies-error-field-entry-price = 入场价格
strategies-error-field-fast-period = 快线周期
strategies-error-field-minimum-buy-share = 最低买入占比
strategies-error-field-minimum-transactions = 最低交易笔数
strategies-error-field-moving-average-type = 均线类型
strategies-error-field-moving-average = 移动平均线
strategies-error-field-reference-candle-close = 参考 K 线收盘价
strategies-error-field-peak-price = 高点价格
strategies-error-field-slow-period = 慢线周期
strategies-error-field-window = 时间窗口
strategies-error-field-within-candles = 交叉 K 线数
strategies-error-field-change = 变化
strategies-error-field-end-hour = 结束小时
strategies-error-field-entry-liquidity = 入场流动性
strategies-error-field-giveback = 回吐
strategies-error-field-highest-high = 最高价
strategies-error-field-long-window-transactions = 长窗口交易数
strategies-error-field-maximum-drop = 最大跌幅
strategies-error-field-minimum-drop = 最小跌幅
strategies-error-field-multiple = 倍数
strategies-error-field-pool-liquidity = 池流动性
strategies-error-field-ratio = 比率
strategies-error-field-remaining-position-value = 持仓剩余价值
strategies-error-field-signal = 信号
strategies-error-field-squeeze-width = 收窄宽度
strategies-error-field-standard-deviations = 标准差
strategies-error-field-start-hour = 开始小时
strategies-error-field-subject = 计数对象
strategies-error-field-token-creation-time = 代币创建时间
strategies-error-field-total-volume = 总成交量
strategies-error-field-trade-size = 交易规模
strategies-error-field-vwap = VWAP

# Expected parameter types

strategies-error-expected-boolean = 布尔值
strategies-error-expected-number = 数字
strategies-error-expected-string = 字符串

# Missing context data

strategies-error-data-current-price = 当前价格
strategies-error-data-liquidity-data = 流动性数据
strategies-error-data-market-data = 市场数据
strategies-error-data-ohlcv-data = OHLCV 数据
strategies-error-data-position-data = 仓位数据
strategies-error-data-peak-price = 高点价格
strategies-error-data-transaction-counts = 交易笔数
strategies-error-data-entry-liquidity = 入场流动性
strategies-error-data-entry-pool-liquidity = 入场池的流动性
strategies-error-data-real-pool-reserves = 池的真实流动性
strategies-error-data-remaining-position-value = 持仓剩余价值
strategies-error-data-token-creation-time = 代币创建时间
strategies-error-data-trade-size = 交易规模

# Indicators

strategies-error-indicator-consecutive-candles = 连续 K 线
strategies-error-indicator-moving-average = 移动平均线
strategies-error-indicator-price-breakout = 价格突破
strategies-error-indicator-price-change-lookback = 价格变化回溯期
strategies-error-indicator-volume-spike = 成交量激增
strategies-error-indicator-moving-average-cross = 移动平均线交叉
strategies-error-indicator-rsi = RSI
strategies-error-indicator-bollinger-bands = 布林带
strategies-error-indicator-pullback-from-high = 高点回撤
strategies-error-indicator-volatility-range = 波动区间
strategies-error-indicator-vwap = VWAP

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = 分支节点缺少条件
strategies-error-rule-branch-node-missing-operator = 分支节点缺少运算符
strategies-error-rule-branch-node-must-have-at-least-one-child = 分支节点必须至少有一个子节点
strategies-error-rule-invalid-rule-tree-structure = 规则树结构无效
strategies-error-rule-leaf-node-missing-condition = 叶节点缺少条件
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 运算符必须恰好有一个子节点
strategies-error-rule-a-group-needs-at-least-one-condition-switched-on = 每个分组至少需要一个已开启的条件
strategies-error-rule-not-operator-cannot-have-a-switched-off-child = NOT 运算符的子节点不能被关闭
strategies-error-rule-rules-nest-deeper-than-four-levels = 规则嵌套不能超过四层
strategies-error-rule-the-root-group-cannot-be-switched-off = 根分组不能被关闭

strategies-readiness-priced = 所有仓位均有价格
strategies-readiness-priced-ok = 每个未平仓的模拟仓位都有流动性池价格
strategies-readiness-priced-missing =
    { $count ->
       *[other] { $count } 个未平仓的模拟仓位没有流动性池价格
    }
strategies-readiness-runtime = 可进行实盘交易
strategies-readiness-runtime-ok = 设置和安全检查均允许实盘交易
