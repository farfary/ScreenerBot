# Service health messages. Ids are used by the Service::health implementations in
# src/services/implementations/*.rs and by src/webserver/routes/services/handlers.rs.

# $component is a code identifier and is not translated.
services-health-component-unavailable = { $component } 组件不可用
services-health-unavailable = 健康状态不可用
services-health-pools-not-running = 流动性池服务未运行
services-health-events-db-uninitialized = 事件数据库未初始化
services-health-sol-price-not-running = SOL 价格服务未运行
services-health-sol-price-stale = SOL 价格数据已过期（已过 { $seconds } 秒）
services-health-sol-price-no-data = 暂无 SOL 价格数据
services-health-telegram-discovery = 发现模式
services-health-telegram-disconnected = 已断开连接
services-health-wallet-watch-polling-only = 检测仅依靠轮询运行
services-health-assistant-tasks-disabled = 已在配置中停用
services-health-connectivity-critical-unhealthy = 关键端点异常：{ $endpoints }
services-health-filtering-snapshot-stale = 过滤快照已过 { $seconds } 秒

## Services page (pages/services.js)

services-status-healthy = 健康
services-status-starting = 启动中
services-status-degraded = 性能下降
services-status-unhealthy = 异常
services-status-stopping = 停止中
services-status-disabled = 已停用
services-status-unknown = 未知

services-name-account = 账户
services-name-assistant-scheduled-tasks = 助手计划任务
services-name-ata-cleanup = 代币账户清理
services-name-connectivity = 连接状态
services-name-copy-trading = 跟单交易
services-name-events = 事件
services-name-filtering = 过滤
services-name-llm-analysis = LLM 分析
services-name-ohlcv = OHLCV
services-name-pool-pricing = 流动性池定价
services-name-pools = 流动性池
services-name-positions = 仓位
services-name-referral = 推荐
services-name-rpc-stats = RPC 统计
services-name-sol-price = { -sol } 价格
services-name-telegram = { -telegram }
services-name-tokens = 代币
services-name-trader = 交易引擎
services-name-transactions = 交易
services-name-update-check = 更新检查
services-name-wallet = 钱包
services-name-wallet-watch = 钱包监控
services-name-webserver = Web 服务器

services-loading = 正在加载服务…
services-load-failed = 加载服务失败
services-load-failed-description = 正在等待后端响应，将自动重试。
services-refresh-failed = 无法刷新服务
services-search-placeholder = 搜索服务…
services-summary-total = 总计
services-summary-alerts = 警报
services-summary-alerts-tooltip = { $degraded } 个性能下降 / { $unhealthy } 个异常
services-filter-status = 状态
services-filter-all-statuses = 全部状态
services-filter-all-services = 全部服务
services-filter-enabled-only = 仅已启用
services-filter-disabled-only = 仅已停用
services-col-service = 服务
services-col-health = 健康度
services-col-priority = 优先级
services-col-uptime = 运行时长
services-col-activity = 活动
services-col-last-cycle = 上次周期
services-col-avg-cycle = 平均周期
services-col-avg-poll = 平均轮询
services-col-cycle-rate = 周期速率
services-col-tasks = 任务
services-col-ops = 每秒操作数
services-col-errors = 错误
services-col-dependencies = 依赖项
services-activity-busy = 繁忙 { $percent }
services-activity-polls =
    { $count ->
       *[other] { $count } 次轮询
    }
services-tasks-tooltip =
    { $count ->
       *[other] { $count } 个任务
    }
    上次：{ $last }
    平均：{ $avg }
    轮询：{ $poll }
    空闲：{ $idle }
    轮询总数：{ $polls }
services-tasks-none = 无已监测的任务

# Empty table (scripts/pages/services.js)
services-empty = 没有正在运行的服务
    .message = 机器人启动服务后，服务会显示在这里。
