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
wallets-field-name = 钱包名称
wallets-field-notes = 备注
wallets-field-private-key = 私钥
wallets-modal-close =
    .aria-label = 关闭弹窗
wallets-this-wallet = 此钱包
wallets-summary-native = { -sol }
wallets-copied-private-key = 私钥

# wallets.js: subtabs, toasts and busy states.
wallets-tab-main = 主钱包
wallets-tab-secondaries = 副钱包
wallets-tab-archive = 归档
wallets-tab-watched = 监控中
wallets-refresh-failed = 无法刷新钱包
wallets-action-failed = 失败
wallets-toast-failed = 失败：{ $reason }
wallets-create-busy = 正在创建...
wallets-create-fallback = 创建失败
wallets-create-done = 钱包“{ $name }”已创建！
wallets-import-busy = 正在导入...
wallets-import-failed = 导入失败
wallets-import-done = 钱包“{ $name }”已导入！
wallets-archive-busy = 正在归档...
wallets-archive-confirm-text = 确定要归档 <strong>{ $name }</strong> 吗？
wallets-archive-done = 钱包已归档
wallets-restore-done = 钱包已恢复
wallets-export-busy = 正在解密...
wallets-export-revealed = 密钥已显示，请妥善保管
wallets-delete-busy = 正在删除...
wallets-delete-confirm-text = 确定要删除 <strong>{ $name }</strong> 吗？
wallets-delete-done = 钱包已永久删除

# wallets.html: Add Wallet dialog.
wallets-add-title = 添加钱包
wallets-add-tab-create = 新建
wallets-add-tab-import = 导入已有
wallets-create-name-input =
    .placeholder = 例如：交易钱包
wallets-create-name-hint = 用于识别此钱包的易记名称
wallets-create-notes-input =
    .placeholder = 可选的描述或用途...
wallets-create-submit = 创建钱包
wallets-import-warning-title = 安全警告
wallets-import-warning-body = 仅从可信来源导入私钥。您的密钥将被加密并安全存储在此设备上。
wallets-import-name-input =
    .placeholder = 例如：我的钱包
wallets-import-key-input =
    .placeholder = Base58 字符串或 JSON 数组 [1,2,3,...]
wallets-import-key-toggle =
    .aria-label = 切换私钥可见性
wallets-import-key-hint = 支持 base58 编码密钥或字节数组格式
wallets-import-notes-input =
    .placeholder = 可选的描述...
wallets-import-submit = 导入钱包

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

# wallets.html: Archive and Delete dialogs.
wallets-archive-title = 归档钱包
wallets-archive-note = 已归档的钱包不会参与任何操作，但可随时恢复。
wallets-archive-confirm = 是，归档
wallets-delete-title = 删除钱包
wallets-delete-warning-title = 此操作无法撤销！
wallets-delete-warning-body = 删除此钱包将从此设备上永久移除该钱包及其加密私钥。
wallets-delete-confirm = 是，删除

# wallets.html and bulk_operations.js: bulk import.
wallets-bulk-import-title = 导入钱包
wallets-bulk-import-submit = 导入钱包
wallets-bulk-step-upload = 上传文件
wallets-bulk-step-map = 映射列
wallets-bulk-step-results = 结果
wallets-bulk-import-file-warning-body = 仅从可信来源导入文件。私钥将被加密并安全存储在此设备上。
wallets-bulk-drop-title = 将文件拖放到此处
wallets-bulk-drop-subtitle = 或点击浏览
wallets-bulk-drop-formats = 支持 CSV 和 Excel（.xlsx、.xls）
wallets-bulk-file-remove =
    .aria-label = 移除文件
wallets-bulk-map-subtitle = 将文件中的列与钱包字段对应
wallets-bulk-preview-title = 预览（前 5 行）
wallets-bulk-summary-valid = <strong>{ $count }</strong> 个有效
wallets-bulk-summary-invalid = <strong>{ $count }</strong> 个无效
wallets-bulk-summary-duplicate =
    { $count ->
       *[other] <strong>{ $count }</strong> 个重复
    }
wallets-bulk-done = 完成
wallets-bulk-file-invalid = 文件类型无效。请使用 CSV 或 Excel 文件。
wallets-bulk-preview-busy = 正在处理...
wallets-bulk-preview-fallback = 处理文件失败
wallets-bulk-preview-failed = 处理文件失败：{ $reason }
wallets-bulk-column-select = -- 选择列 --
wallets-bulk-preview-empty = 文件中没有数据行
wallets-bulk-preview-status = 状态
wallets-bulk-status-valid = 有效
wallets-bulk-status-duplicate = 重复
wallets-bulk-status-invalid = 无效
wallets-bulk-import-busy = 正在导入...
wallets-bulk-import-toast =
    { $count ->
       *[other] 已导入 { $count } 个钱包
    }
wallets-bulk-import-error = 导入失败：{ $reason }
wallets-bulk-result-success-title = 导入成功
wallets-bulk-result-success-detail =
    { $count ->
       *[other] 全部 { $count } 个钱包均已成功导入
    }
wallets-bulk-result-partial-title = 部分成功
wallets-bulk-result-partial-detail = 已导入 { $imported } 个，失败 { $failed } 个
wallets-bulk-result-failed-title = 导入失败
wallets-bulk-result-failed-detail =
    { $count ->
       *[other] 全部 { $count } 个钱包均导入失败
    }
wallets-bulk-result-imported = 已导入
wallets-bulk-result-failed = 失败

# wallets.html and bulk_operations.js: bulk export.
wallets-bulk-export-title = 导出钱包
wallets-bulk-export-format = 格式
wallets-bulk-export-format-csv = CSV (.csv)
wallets-bulk-export-format-xlsx = Excel (.xlsx)
wallets-bulk-export-include-archived = 包含已归档的钱包
wallets-bulk-export-safe-title = 安全导出
wallets-bulk-export-safe-body = 仅导出钱包地址和元数据，不含私钥。
wallets-bulk-export-safe-submit = 导出地址
wallets-bulk-export-or = 或
wallets-bulk-export-danger-title = 危险导出
wallets-bulk-export-danger-body = 导出内容包含私钥。任何获得此文件的人都可以盗走您的资金。
wallets-bulk-export-danger-submit = 连同私钥一起导出
wallets-bulk-export-busy = 正在导出...
wallets-bulk-export-done = 已将钱包导出到 { $filename }
wallets-bulk-export-fallback = 导出失败
wallets-bulk-export-error = 导出失败：{ $reason }
wallets-bulk-confirm-title = 确认危险导出
wallets-bulk-confirm-warning =
    { $count ->
       *[other] 您即将导出 <strong>{ $count }</strong> 个私钥。这极其危险！
    }
wallets-bulk-confirm-risk-steal = 任何获得此文件的人都可以盗走全部资金
wallets-bulk-confirm-risk-share = 切勿与任何人分享此文件
wallets-bulk-confirm-risk-delete = 使用后请立即删除该文件
wallets-bulk-confirm-prompt = 请输入下方短语以确认
wallets-bulk-confirm-submit = 导出密钥

# renderers.js: main wallet holdings and wallet lists.
wallets-holdings-col-token = 代币
wallets-holdings-col-balance = 余额
wallets-holdings-col-value = 价值（{ -sol }）
wallets-holdings-col-type = 类型
wallets-holdings-col-decimals = 小数位数
wallets-holdings-empty-title = 暂无代币持仓
wallets-holdings-empty-message = 此钱包持有的代币将显示在这里。
wallets-holdings-no-main = 没有主钱包
wallets-holdings-main-tag = 主钱包
wallets-holdings-main-title = 主钱包
wallets-holdings-tokens = 代币
wallets-holdings-last-used = 最近使用
wallets-holdings-never = 从未
wallets-holdings-search =
    .placeholder = 按代币符号或铸造地址搜索...
wallets-holdings-export = 导出密钥
wallets-holdings-export-tooltip = 导出此钱包的私钥
wallets-list-col-name = 名称
wallets-list-col-balance = 余额（{ -sol }）
wallets-list-col-type = 类型
wallets-list-col-created = 创建时间
wallets-list-col-actions = 操作
wallets-list-action-export = 导出私钥
wallets-list-action-archive = 归档钱包
wallets-list-action-restore = 恢复钱包
wallets-list-action-delete = 永久删除
wallets-list-count = 钱包
wallets-list-search =
    .placeholder = 按名称或地址搜索...
wallets-list-loading-title = 正在加载钱包…
wallets-list-loading-description = 正在准备所选钱包视图。
wallets-secondaries-empty-title = 没有副钱包
wallets-secondaries-empty-message = 创建更多钱包，以便在多个账户之间分开管理您的交易活动。
wallets-secondaries-add = 添加钱包
wallets-archive-empty-title = 没有已归档的钱包
wallets-archive-empty-message = 您归档的钱包将安全保存在这里，以备日后查看。

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
