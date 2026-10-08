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
strategies-summary-period-seconds = 周期：{ $amount } 秒
strategies-summary-period-minutes = 周期：{ $amount } 分钟
strategies-summary-period-hours = 周期：{ $amount } 小时

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

## Condition catalog

strategies-catalog-search =
    .placeholder = 搜索条件...
strategies-catalog-search-clear =
    .aria-label = 清除搜索
strategies-catalog-fold-all = 全部折叠
strategies-catalog-unfold-all = 全部展开
strategies-catalog-no-description = 暂无描述

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
strategies-toast-enabled = 策略已启用
    .message = “{ $name }”已启用
strategies-toast-disabled = 策略已停用
    .message = “{ $name }”已停用
strategies-toast-toggle-failed = 切换失败
    .message = 更新策略状态失败
strategies-toast-load-failed = 加载失败
    .message = 从服务器加载策略失败
strategies-toast-load-strategy-failed = 加载策略失败
strategies-toast-no-strategy = 尚未创建策略
    .message = 请至少添加一个条件，或先点击“新建策略”创建策略
strategies-toast-no-conditions-save = 没有条件
    .message = 保存前请至少为策略添加一个条件
strategies-toast-name-required = 需要名称
    .message = 保存前请输入策略名称
strategies-toast-saved = 策略已保存
    .message = “{ $name }”已成功保存
strategies-toast-save-failed = 保存失败
    .message = 将策略保存到数据库失败
strategies-toast-no-strategy-validate = 没有可验证的策略
strategies-toast-no-conditions-validate = 没有条件
    .message = 验证前请至少添加一个条件
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
strategies-toast-condition-added = 条件已添加
    .message = 已将 { $name } 添加到策略


## Conditions

strategies-condition-candle-size = K 线大小形态
    .description = 检测特定的 K 线形态：大实体、小实体（十字星）、长影线
strategies-condition-candle-size-param-pattern = 形态类型
    .description = 要检测的 K 线形态
strategies-condition-candle-size-param-pattern-option-large-body = 大实体（强势行情）
strategies-condition-candle-size-param-pattern-option-small-body = 小实体（十字星/犹豫）
strategies-condition-candle-size-param-pattern-option-long-upper-wick = 长上影线（受阻）
strategies-condition-candle-size-param-pattern-option-long-lower-wick = 长下影线（支撑）
strategies-condition-candle-size-param-threshold = 大小阈值 %
    .description = 形态检测的百分比阈值

strategies-condition-consecutive-candles = 连续 K 线
    .description = 检测连续的绿色（看涨）或红色（看跌）K 线，并带有最小幅度过滤
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
    .description = 检查仓位已持有的时长（用于出场策略中的按时间出场）
strategies-condition-position-holding-time-param-hours = 时间阈值（小时）
    .description = 自仓位开启以来的小时数
strategies-condition-position-holding-time-param-comparison = 比较方式
    .description = 如何将仓位持有时长与阈值比较
strategies-condition-position-holding-time-param-comparison-option-greater-than = 长于（>）
strategies-condition-position-holding-time-param-comparison-option-greater-equal = 至少（≥）
strategies-condition-position-holding-time-param-comparison-option-less-than = 短于（{ "<" }）
strategies-condition-position-holding-time-param-comparison-option-less-equal = 至多（≤）

strategies-condition-price-breakout = 价格突破
    .description = 检测价格向上突破阻力位（周期高点）或向下跌破支撑位（周期低点）
strategies-condition-price-breakout-param-lookback = 回溯期
    .description = 用于确定支撑/阻力位的 K 线数量
strategies-condition-price-breakout-param-direction = 突破方向
    .description = 突破的方向
strategies-condition-price-breakout-param-direction-option-upward = 向上（突破阻力）
strategies-condition-price-breakout-param-direction-option-downward = 向下（跌破支撑）
strategies-condition-price-breakout-param-confirmation = 确认幅度 %
    .description = 超过该价位多远才确认突破（避免假信号）

strategies-condition-price-change-percent = 价格变化 %
    .description = 检查价格在一段时间内的变化是否达到百分比阈值
strategies-condition-price-change-percent-param-percentage = 变化阈值 %
    .description = 触发条件的价格变化百分比（0.1-1000%）
strategies-condition-price-change-percent-param-direction = 方向
    .description = 价格变动方向
strategies-condition-price-change-percent-param-direction-option-above = 上涨（+%）
strategies-condition-price-change-percent-param-direction-option-below = 下跌（-%）
strategies-condition-price-change-percent-param-direction-option-within = 区间内（±%）
strategies-condition-price-change-percent-param-time-value = 时间周期
    .description = 回溯期数值（秒为 1-3600，分钟为 1-1440，小时为 1-720）
strategies-condition-price-change-percent-param-time-unit = 时间单位
    .description = 回溯期的时间单位
strategies-condition-price-change-percent-param-time-unit-option-seconds = 秒
strategies-condition-price-change-percent-param-time-unit-option-minutes = 分钟
strategies-condition-price-change-percent-param-time-unit-option-hours = 小时

strategies-condition-price-to-ma = 价格与移动平均线
    .description = 检查价格在其简单移动平均线的上方、下方还是区间内
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
    .description = 检测相对平均成交量的成交量激增（表明关注度上升）
strategies-condition-volume-spike-param-lookback = 回溯期
    .description = 用于计算平均成交量的 K 线数量
strategies-condition-volume-spike-param-multiplier = 成交量倍数
    .description = 高于平均值的倍数（例如 2.0 = 平均值的 200%）

## Shared by every condition

strategies-condition-param-timeframe = 时间周期
    .description = 要分析的 K 线时间周期（未设置时默认使用策略的时间周期）
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
strategies-error-invalid-rule-tree = 规则树无效：{ $reason }
strategies-error-evaluation-timeout = 策略评估在 { $timeout } 毫秒后超时
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

# Indicators

strategies-error-indicator-consecutive-candles = 连续 K 线
strategies-error-indicator-moving-average = 移动平均线
strategies-error-indicator-price-breakout = 价格突破
strategies-error-indicator-price-change-lookback = 价格变化回溯期
strategies-error-indicator-volume-spike = 成交量激增

# Rule tree faults

strategies-error-rule-branch-node-missing-conditions = 分支节点缺少条件
strategies-error-rule-branch-node-missing-operator = 分支节点缺少运算符
strategies-error-rule-branch-node-must-have-at-least-one-child = 分支节点必须至少有一个子节点
strategies-error-rule-invalid-rule-tree-structure = 规则树结构无效
strategies-error-rule-leaf-node-missing-condition = 叶节点缺少条件
strategies-error-rule-not-operator-must-have-exactly-one-child = NOT 运算符必须恰好有一个子节点
