# Transaction type labels. Ids come from TransactionType::kind() in
# src/transactions/types.rs; the dashboard maps them in ui/transaction_type.js.

transactions-type-buy = 买入
transactions-type-sell = 卖出
transactions-type-swap = 兑换
transactions-type-sol-transfer = SOL 转账
transactions-type-token-transfer = 代币转账
transactions-type-transfer = 转账
transactions-type-dust = 粉尘
transactions-type-spam = 垃圾交易
transactions-type-ata-create = 账户已开设
transactions-type-ata-close = 租金已回收
transactions-type-ata = 代币账户
transactions-type-liquidity-add = 添加流动性
transactions-type-liquidity-remove = 移除流动性
transactions-type-nft = NFT
transactions-type-program = 程序调用
transactions-type-compute = 计算
transactions-type-failed = 失败
transactions-type-unknown = 未分类

# A type with the payload that identifies it, as shown in the position activity feed.
transactions-type-with-detail = { $label }（{ $detail }）
transactions-type-token-transfer-detail = { $label } { $mint }（{ $amount }）
transactions-type-spam-detail = 垃圾空投（{ $mint }）
transactions-type-described = { $description }

# Type filter entries whose wording differs from the type label.
transactions-filter-all = 全部类型
transactions-filter-transfer = 转账
transactions-filter-ata = 租金与账户
transactions-filter-liquidity = 流动性
transactions-filter-program = 程序调用

# Wallet-relative direction. Ids come from TransactionDirection in src/transactions/types.rs
# (ui/transaction_direction.js).
transactions-direction-incoming = 转入
transactions-direction-outgoing = 转出
transactions-direction-internal = 内部
transactions-direction-unknown = 未分类

# Chain status. Ids come from TransactionStatus in src/transactions/types.rs
# (ui/transaction_status.js); Success and Unknown label a row without a status.
transactions-status-pending = 待处理
transactions-status-confirmed = 已确认
transactions-status-finalized = 已最终确认
transactions-status-failed = 失败
transactions-status-success = 成功
transactions-status-unknown = 未知

# Ids come from AtaOperationType in src/transactions/types.rs.
transactions-ata-operation-creation = 创建
transactions-ata-operation-closure = 关闭

## Transactions page (pages/transactions.js)

transactions-toolbar-title = 交易历史
transactions-search =
    .placeholder = 搜索签名…
    .aria-label = 搜索交易签名
transactions-load-failed = 无法刷新交易
transactions-summary-total = 总计
transactions-summary-estimate = 估算
transactions-summary-success = 成功
transactions-summary-failed = 失败
transactions-filter-wallet = 钱包
transactions-filter-type = 类型
transactions-filter-direction = 方向
transactions-filter-status = 状态
transactions-filter-all-directions = 全部方向
transactions-filter-all-statuses = 全部状态
transactions-wallet-main = 主钱包
transactions-col-time = 时间
transactions-col-signature = 签名
transactions-col-type = 类型
transactions-col-direction = 方向
transactions-col-status = 状态
transactions-col-native-delta = Δ { -sol }
transactions-col-fees = 费用（{ -sol }）
transactions-col-token = 代币
transactions-col-router = 路由
transactions-col-instructions = 指令

## Transaction details dialog (ui/transaction_details_dialog.js)

transactions-dialog-copy-signature =
    .title = 复制签名
transactions-dialog-close =
    .title = 关闭（ESC）
transactions-dialog-tabs-label = 交易详情分区
transactions-dialog-meta-slot = 插槽：
transactions-dialog-meta-fee = 费用：
transactions-dialog-loading = 正在加载...
transactions-dialog-loading-details = 正在加载交易详情...
transactions-dialog-load-failed = 加载交易详情失败
# $reason is the failure text reported by the server.
transactions-dialog-load-failed-reason = 加载交易详情失败：{ $reason }
transactions-dialog-not-found = 未找到交易
transactions-dialog-tab-overview = 概览
transactions-dialog-tab-balances = 余额
transactions-dialog-tab-instructions = 指令
transactions-dialog-tab-logs = 日志
transactions-dialog-tab-ata = ATA
transactions-dialog-tab-raw = 原始数据
transactions-dialog-unknown = 未知
transactions-dialog-unknown-asset = 未知资产
transactions-dialog-unavailable = 不可用

## Transaction details dialog: overview

transactions-dialog-failed-title = 交易失败
transactions-dialog-no-program-error = 未提供程序错误。
transactions-dialog-story-title = 发生了什么
# $router is the routing program name.
transactions-dialog-router-via = 通过 { $router }
transactions-dialog-flow-paid = 支付
transactions-dialog-flow-received = 收到
transactions-dialog-flow-from = 转出方
transactions-dialog-flow-to = 转入方
transactions-dialog-flow-amount = 数量
transactions-dialog-net-wallet-change = 钱包净变动：
transactions-dialog-processed = 已在 Solana 上处理
transactions-dialog-execution-title = 执行
transactions-dialog-metric-execution-price = 成交价格
transactions-dialog-metric-effective-received = 实际收到
transactions-dialog-metric-effective-spent = 实际支出
transactions-dialog-metric-network-fee = 网络费用
transactions-dialog-metric-estimated-pnl = 预估盈亏
transactions-dialog-metric-net-native-change = { -sol } 净变动
transactions-dialog-route-title = 路由与资产
transactions-dialog-route-router = 路由
transactions-dialog-route-input-asset = 输入资产
transactions-dialog-route-output-asset = 输出资产
transactions-dialog-route-pool = 流动性池
transactions-dialog-route-program = 程序
transactions-dialog-tech-title = 技术详情
transactions-dialog-tech-summary = 签名、插槽和资源
transactions-dialog-tech-signature = 签名
transactions-dialog-tech-timestamp = 时间戳
transactions-dialog-tech-slot = 插槽
transactions-dialog-tech-exact-fee = 精确费用
transactions-dialog-tech-accounts = 账户
transactions-dialog-tech-instructions = 指令
transactions-dialog-tech-compute-units = 计算单元
transactions-dialog-tech-token-decimals = 代币小数位数

## Transaction details dialog: balances, instructions, logs, ATA and raw tabs

transactions-dialog-balances-native-title = { -sol } 余额变动
transactions-dialog-balances-native-empty = 没有 { -sol } 余额变动
transactions-dialog-balances-token-title = 代币余额变动
transactions-dialog-balances-token-empty = 没有代币余额变动
transactions-dialog-balances-net-native = { -sol } 净变动
transactions-dialog-balances-fee = 交易费用
transactions-dialog-col-account = 账户
transactions-dialog-col-token = 代币
transactions-dialog-col-pre-balance = 变动前余额
transactions-dialog-col-post-balance = 变动后余额
transactions-dialog-col-change = 变动
transactions-dialog-col-type = 类型
transactions-dialog-col-rent = 租金（{ -sol }）
transactions-dialog-instructions-empty = 未找到指令
transactions-dialog-instructions-count =
    { $count ->
       *[other] { $count } 条指令
    }
transactions-dialog-instruction-program-id = 程序 ID
transactions-dialog-instruction-accounts = 账户（{ $count }）
transactions-dialog-instruction-data = 数据
transactions-dialog-logs-empty = 暂无日志
transactions-dialog-logs-filter = 筛选日志...
transactions-dialog-logs-no-match = 没有匹配的日志
transactions-dialog-logs-count =
    { $count ->
       *[other] { $count } 条日志
    }
transactions-dialog-ata-empty = 此交易中没有 ATA 操作
transactions-dialog-ata-summary-title = ATA 分析摘要
transactions-dialog-ata-creations = 创建
transactions-dialog-ata-closures = 关闭
transactions-dialog-ata-rent-spent = 已支出租金
transactions-dialog-ata-rent-recovered = 已回收租金
transactions-dialog-ata-net-rent = 租金净影响
transactions-dialog-ata-operations-title = ATA 操作（{ $count }）
transactions-dialog-raw-copy = 复制 JSON
transactions-dialog-raw-empty = 暂无原始数据

# Empty table (scripts/pages/transactions.js)
transactions-empty = 暂无交易
    .message = 交易钱包的兑换和转账在链上确认后显示在这里。
