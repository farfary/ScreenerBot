# Wallet page labels.

# Wallet types. Ids come from WalletType in src/wallets/types.rs.
wallets-type-generated = 已生成
wallets-type-imported = 已导入
wallets-type-migrated = 已迁移

# Why a watched wallet is paused. Ids come from WatchDisableReason in
# src/wallets/watch/types.rs. $limit is a signature count.
wallets-watch-disabled-user = 已由您暂停
wallets-watch-disabled-signature-budget = 已暂停：在追上进度之前已达到 { $limit } 个签名的检查上限
wallets-watch-disabled-unknown = 已暂停：无法读取已保存的监控安全原因
wallets-watch-disabled-helius-unavailable = 已暂停：高活跃度提供商不可用，游标已保留
wallets-watch-disabled-processing-failed = 已暂停：无法处理钱包活动，游标已保留

# Last runtime problem of a watch. Ids come from WatchRuntimeError in
# src/wallets/watch/types.rs.
wallets-watch-error-provider-unavailable = 高活跃度提供商不可用，监控已暂停
wallets-watch-error-provider-repeated-failure = { -helius } 检查多次失败，监控已暂停
wallets-watch-error-processing-repeated-failure = 钱包活动处理多次失败，监控已暂停
wallets-watch-error-position-unreadable = 钱包监控无法读取已保存的位置，正在重试
wallets-watch-error-provider-check-failed = 高活跃度提供商检查失败，正在重试
wallets-watch-error-decode-failed = 无法解码高活跃度交易，游标已保留
wallets-watch-error-processing-failed = 无法处理钱包活动，正在重试
wallets-watch-error-position-save-failed = 钱包监控无法保存其位置，正在重试

# Why a watched wallet is paused, as a second line under its status. Ids come from
# WatchDisableReason in src/wallets/watch/types.rs, named after the serialized kind.
# The `unknown` kind has no detail line.
wallets-watch-reason-user = 已由您暂停。
wallets-watch-reason-signature-budget = 此钱包的活动量超出了当前监控的检查能力。
wallets-watch-reason-helius-unavailable = { -helius } 检查失败。已保存的进度已保留。
wallets-watch-reason-processing-failed = 无法处理钱包活动。已保存的进度已保留。

# Vocabulary shared by the wallet tables and dialogs.
wallets-modal-close =
    .aria-label = 关闭弹窗
wallets-summary-native = { -sol }
wallets-copied-private-key = 私钥

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = 主钱包
wallets-tab-other = 其他钱包
wallets-tab-watched = 监控中
wallets-refresh-failed = 无法刷新钱包
wallets-action-failed = 失败
wallets-toast-failed = 失败：{ $reason }
wallets-export-busy = 正在解密...
wallets-export-revealed = 密钥已显示，请妥善保管

# wallets.html: Watch Wallet dialog.
wallets-watch-add-title = 监控钱包
wallets-watch-add-address = 钱包地址
wallets-watch-add-address-input =
    .placeholder = Solana 地址
wallets-watch-add-address-hint = 记录该钱包的链上活动，并通过您的 { -telegram } 设置发送交易提醒。
wallets-watch-add-label = 标签
wallets-watch-add-label-input =
    .placeholder = 可选名称
wallets-watch-add-submit = 添加监控

# wallets.html and watched.js: watch options dialog.
wallets-watch-budget-title-options = 钱包监控选项
wallets-watch-budget-title-restore = 恢复钱包监控
wallets-watch-budget-close =
    .aria-label = 关闭
wallets-watch-budget-label-signatures = 每次检查的签名数
wallets-watch-budget-label-transactions = 每次检查的成功完整交易数
wallets-watch-budget-hint-signatures = 当前上限：{ $limit }。每次检查可选 500–5,000 个签名，步长为 100。
wallets-watch-budget-hint-transactions = 当前上限：{ $limit }。每次检查可选 500–5,000 笔成功交易，步长为 100。
wallets-watch-budget-error-range = 请选择每次检查 500 至 5,000 条记录，步长为 100 条。
wallets-watch-budget-error-ack = 请确认自上次完成检查以来的签名将被跳过。
wallets-watch-budget-save-failed = 无法保存监控上限。
wallets-watch-budget-save = 保存上限
wallets-watch-budget-resume = 从当前开始恢复
wallets-watch-budget-resume-notice = 此钱包在追上进度之前已达到检查上限。从当前开始恢复会从钱包的最新活动起监控，自上次完成检查以来的活动不会被跟单。
wallets-watch-budget-resume-tasks = 跟单任务将保持暂停，直到您在跟单交易中逐个恢复。
wallets-watch-budget-resume-ack = 我了解错过的活动不会被跟单。
wallets-watch-budget-resumed = 已从钱包当前最新位置恢复监控
wallets-watch-budget-updated = 钱包监控上限已更新
wallets-watch-helius-allow = 需要时允许 { -helius } 追赶进度
wallets-watch-helius-try = 尝试使用 { -helius } 追赶进度
wallets-watch-helius-stop = 停止此钱包的 { -helius } 追赶进度
wallets-watch-helius-description-approved = 已允许此钱包使用 { -helius } 追赶进度。关闭后将恢复标准检查，活跃度高的钱包可能会落后。
wallets-watch-helius-description-available = { -helius } 可以从已保存的位置起检查成功的 Solana 交易，不会跳过未检查的区间。这可能会消耗更多提供商额度，且仍可能落后。
wallets-watch-helius-description-unavailable = { -helius } 追赶进度不可用。请配置一个已启用的 { -helius } RPC 端点后再使用。
wallets-watch-helius-description-unsupported = 此监控没有受支持的追赶进度提供商。如果监控达到上限，可以从当前开始恢复。
wallets-watch-helius-allow-title = 允许此钱包使用 { -helius } 追赶进度
wallets-watch-helius-allow-message = { -helius } 可以从已保存的位置起检查成功的 Solana 交易，不会跳过未检查的区间。目前每返回 100 笔完整交易收取 10 个额度（向上取整），每次请求至少 10 个额度。一次检查可能发出多次请求，实际用量和提供商定价可能有所不同。跟单任务将保持暂停，需另行恢复。
wallets-watch-helius-allow-confirm = 对此钱包允许
wallets-watch-helius-stop-message = 此钱包将恢复标准检查。活跃度高的钱包可能再次达到监控上限并暂停。其他钱包和您的 { -helius } RPC 配置保持不变。
wallets-watch-helius-stop-confirm = 对此钱包停止
wallets-watch-helius-stop-keep = 保持允许
wallets-watch-helius-restored = 已根据保存的进度恢复监控，跟单任务保持暂停
wallets-watch-helius-allowed = 需要时已允许此钱包使用 { -helius } 追赶进度
wallets-watch-helius-stopped = 已停止此钱包的 { -helius } 追赶进度
wallets-watch-helius-update-failed = 无法更新钱包追赶进度设置

# wallets.html: Export Private Key dialog.
wallets-export-title = 导出私钥
wallets-export-warning-title = 重要安全警告
wallets-export-warning-body = 切勿向任何人透露您的私钥。任何获得此密钥的人都可以盗走该钱包中的全部资金。
wallets-export-key-label = 私钥（Base58）
wallets-export-copy =
    .title = 复制到剪贴板
    .aria-label = 复制到剪贴板
wallets-export-reveal = 显示密钥

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = 代币
wallets-holdings-col-balance = 余额
wallets-holdings-col-value = 价值（{ -sol }）
wallets-holdings-col-type = 类型
wallets-holdings-col-decimals = 小数位数
wallets-holdings-empty-title = 暂无代币持仓
wallets-holdings-unreadable-title = 持仓未知
wallets-holdings-empty-message = 此钱包持有的代币将显示在这里。
wallets-funding-title = 为交易钱包充值以开始
wallets-funding-message = 此钱包没有 { -sol }。请从交易所或其他钱包向此地址发送 { -sol }；每次买入都会花费其金额加上网络费用。
wallets-holdings-no-main = 没有主钱包
wallets-holdings-main-title = 主钱包
wallets-holdings-tokens = 代币
wallets-holdings-last-used = 最近使用
wallets-holdings-never = 从未
wallets-holdings-search =
    .placeholder = 按代币符号或铸造地址搜索...
wallets-holdings-export = 导出密钥
wallets-holdings-export-tooltip = 导出此钱包的私钥
wallets-holdings-replace = 更换钱包
wallets-holdings-replace-tooltip = 使用另一个钱包交易；此钱包的密钥和历史将被保留
wallets-list-col-name = 名称
wallets-list-col-balance = 余额（{ -sol }）
wallets-list-col-type = 类型
wallets-list-col-created = 创建时间
wallets-list-col-replaced = 更换时间
wallets-list-replaced-on = { $date }
wallets-list-col-actions = 操作
wallets-list-action-export = 导出私钥
wallets-list-count = 钱包
wallets-list-search =
    .placeholder = 按名称或地址搜索...
wallets-list-loading-title = 正在加载钱包…
wallets-list-loading-description = 正在准备所选钱包视图。
wallets-list-kept-empty-message = 早期版本的钱包会显示在这里，方便您继续导出其密钥。
wallets-other-empty-title = 没有其他钱包

# watched.js: watched wallets table and actions.
wallets-watched-col-wallet = 钱包
wallets-watched-col-status = 状态
wallets-watched-col-progress = 已保存进度
wallets-watched-col-last-check = 上次检查
wallets-watched-unlabelled = 未命名钱包
wallets-watched-generic-name = 钱包
wallets-watched-not-synced = 尚未同步
wallets-watched-not-checked = 尚未检查
wallets-watched-action-copy = 跟单
    .title = 在跟单交易中打开此钱包
wallets-watched-action-restore = 恢复监控
wallets-watched-action-options = 监控选项
wallets-watched-action-retry = 重试监控
wallets-watched-action-pause = 暂停
wallets-watched-action-enable = 启用
wallets-watched-action-remove =
    .title = 移除
    .aria-label = 移除 { $name }
wallets-watch-state-paused = 已暂停
wallets-watch-state-catching-up = 追赶进度中
wallets-watch-state-watching = 监控中
wallets-watch-state-streaming = 流式监控中
wallets-watch-state-polling = 轮询中
wallets-watched-detail-helius = 正在通过 { -helius } 检查此钱包。
wallets-watched-empty-title = 没有监控地址
wallets-watched-empty-message = 使用“监控钱包”来记录公开钱包的链上活动。
wallets-watched-count = 监控中
wallets-watched-search =
    .placeholder = 搜索监控中的钱包...
wallets-watched-add = 监控钱包
wallets-watched-refresh = 刷新监控中的钱包
wallets-watched-loading-title = 正在加载监控中的钱包...
wallets-watched-loading-description = 正在获取监控目标。
wallets-watched-load-error-title = 无法加载监控地址
wallets-watched-load-error-description = 请刷新后重试。
wallets-watched-address-invalid = 请输入有效的 Solana 钱包地址。
wallets-watched-added = 已添加钱包监控
wallets-watched-duplicate = 该钱包已在监控中。
wallets-watched-add-failed = 无法添加钱包监控。
wallets-watched-retried = 已使用保存的游标恢复钱包监控
wallets-watched-paused = 钱包监控已暂停
wallets-watched-enabled = 钱包监控已启用
wallets-watched-removed = 钱包监控已移除
wallets-watched-update-failed = 无法更新钱包监控
wallets-setup-gate-title = 钱包需要先完成设置
