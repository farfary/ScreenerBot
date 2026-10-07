# Event display text. Default ids come from src/events/recorders/; task ids
# from ScheduledTaskOutcome in src/events/display_text.rs. Arguments are data
# (subtype codes, method and API names, task names) and are not translated.
events-ohlcv-default = OHLCV 事件：{ $subtype }
events-filtering-default = 过滤事件：{ $subtype }
events-trader-default = 交易引擎事件：{ $subtype }
events-rpc-default = RPC { $method } - { $action }
events-api-default = { $api } - { $action }

# $name is the user-chosen scheduled task name.
events-task-completed = 任务“{ $name }”已完成
events-task-failed = 任务“{ $name }”失败
events-task-timed-out = 任务“{ $name }”已超时

# Event type column labels for the stable scheduled-task subtype codes.
events-subtype-task-completed = 任务已完成
events-subtype-task-failed = 任务失败
events-subtype-task-timed-out = 任务已超时

# Shown when an event carries no display text.
events-message-none = 无消息

# Producer messages. Arguments are identifiers, counts and error text; counts
# are pre-formatted so digits are never grouped.
events-ohlcv-cache-cleanup-failed = 清理 OHLCV 缓存失败
events-ohlcv-gap-cleanup-failed = 清理已填补的缺口记录失败
events-ohlcv-gap-fill-failed = { $mint } 的缺口填补出错
events-ohlcv-backfill-scheduled = 已通过 { $pool } 为 { $mint } 安排多时间周期回填
events-ohlcv-fetch-failed = 通过 { $pool } 获取 { $mint } 的 OHLCV 失败：{ $error }
events-ohlcv-gap-detection-failed = 通过 { $pool } 检测 { $mint } 的缺口失败
events-ohlcv-fetch-success = 已存储 { $mint } 的 { $count } 个 OHLCV 数据点
events-ohlcv-retention-backfill-failed = 通过 { $pool } 为 { $mint } 执行保留期回填失败
events-ohlcv-empty-fetch = 通过 { $pool } 获取 { $mint } 的 OHLCV 结果为空
events-ohlcv-pool-discovery-failed = { $mint } 的流动性池发现失败
events-ohlcv-pool-discovery-success = 已发现 { $mint } 的流动性池
events-ohlcv-process-token-error = 处理 { $mint } 时出错：{ $error }
events-ohlcv-rate-limit-hit = 处理 { $mint } 时触发速率限制
events-ohlcv-pool-unavailable = { $mint } 没有健康的流动性池可用，已推迟
events-ohlcv-token-missing = 处理期间代币 { $mint } 缺失
events-monitors-stopped = 自动交易监控已停止
events-monitors-starting = 自动交易监控正在启动
events-entry-monitor-started = 入场机会监控已启动
events-exit-monitor-started = 出场/仓位监控已启动
events-trader-service-stopped = 交易引擎服务已正常停止
events-trader-service-stopping = 已开始关闭交易引擎服务
events-trader-service-started = 交易引擎服务已完全初始化并运行
events-trader-auto-trading-error = 自动交易遇到错误
events-trader-trading-enabled = 交易已启用并处于活动状态
events-trader-trading-disabled = 配置中已停用交易
events-trader-service-initializing = 交易引擎服务开始初始化
events-connectivity-monitoring-stopped = 连接监控已停止
events-connectivity-monitoring-started = 连接监控已启动（间隔 { $seconds } 秒）
events-connectivity-service-initialized = 连接服务已初始化，共 { $count } 个监控
events-connectivity-critical-unhealthy = { $count } 个关键端点不健康，系统应暂停操作
events-connectivity-endpoint-recovered = 端点已从 { $from } 恢复为健康
events-position-entry-not-landed = { $symbol } 的买入未在链上确认，已移除其仓位
events-position-fill-after-force-close = { $symbol } 的一笔交易在其仓位被强制平仓后于链上确认；已记账并重新计算该仓位
events-position-swap-unbooked = { $symbol } 的一笔兑换已在链上确认，但尚未记入其仓位；将持续重新验证直至记账
events-position-exit-residual-unattributed = { $symbol } 已平仓，但该代币仍有余额留在钱包中。这些代币可能属于入场尚未验证的仓位 { $blocking }，因此未被卖出，也没有仓位管理它们

## Events page (pages/events.js, ui/event_labels.js)

# Category ids from EventCategory in src/events/types.rs, plus the legacy entry and learner categories.
events-category-swap = 兑换
events-category-transaction = 交易
events-category-pool = 流动性池
events-category-position = 仓位
events-category-token = 代币
events-category-wallet = 钱包
events-category-trader = 交易引擎
events-category-entry = 入场
events-category-system = 系统
events-category-ohlcv = OHLCV
events-category-rpc = RPC
events-category-api = API
events-category-security = 安全
events-category-connectivity = 连接
events-category-filtering = 过滤
events-category-scheduled-task = 计划任务
events-category-learner = 学习器
events-category-other = 其他

events-loading = 正在加载事件...
events-load-failed = 加载事件失败
events-load-failed-description = 正在等待后端响应，将自动重试。
events-load-error = 无法加载事件
events-search-placeholder = 搜索事件...
events-summary-total = 总计
events-filter-category = 类别
events-filter-all-categories = 全部类别
events-filter-all-severities = 全部严重程度
events-col-time = 时间
events-col-category = 类别
events-col-type = 类型
events-col-severity = 严重程度
events-col-message = 消息
events-col-token = 代币
events-col-details = 详情
# $count is the number of payload entries not shown in the preview.
events-payload-more = 另有 { $count } 项

## Event details dialog (ui/events_dialog.js)

events-dialog-title = 事件详情
events-dialog-close =
    .aria-label = 关闭对话框
events-dialog-payload = 载荷
events-dialog-copy = 复制详情
events-dialog-copy-title =
    .title = 复制全部事件详情
events-dialog-copy-done = 已复制！
events-dialog-copy-failed = 失败
events-dialog-not-available = 暂无
# $category is the category label; shown when an event has no message.
events-dialog-category-event = { $category }事件
events-dialog-field-id = 事件 ID
events-dialog-field-severity = 严重程度
events-dialog-field-category = 类别
events-dialog-field-subtype = 子类型
events-dialog-field-mint = 代币铸造地址
events-dialog-field-reference = 引用
events-dialog-field-time = 事件时间
events-dialog-field-age = 时长
events-dialog-field-created = 创建时间
# Copied event text: section headings and one "label: value" line per field.
events-dialog-export-heading = 事件详情
events-dialog-export-message = 消息
events-dialog-export-payload = 载荷
events-dialog-export-line = { $label }：{ $value }
