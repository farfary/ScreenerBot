## Shared

settings-duration-minutes =
    { $count ->
       *[other] { $count } 分钟
    }
settings-duration-hours =
    { $count ->
       *[other] { $count } 小时
    }

## settings_dialog.js

settings-dialog-title = 设置
settings-dialog-close =
    .title = 关闭（ESC）
    .aria-label = 关闭设置
settings-dialog-save = 保存更改
settings-dialog-saving = 正在保存…
settings-dialog-saved = 已保存
settings-dialog-save-success = 设置已保存
settings-dialog-save-failed = 保存设置失败
settings-dialog-update-attention = 更新需要处理
settings-dialog-tab-interface = 界面
settings-dialog-tab-navigation = 导航
settings-dialog-tab-startup = 启动
settings-dialog-tab-hints = 提示
settings-dialog-tab-data = 数据
settings-dialog-tab-security = 安全
settings-dialog-tab-account = 账户
settings-dialog-tab-telegram = { -telegram }
settings-dialog-tab-agent-connections = 智能体连接
settings-dialog-tab-updates = 更新
settings-dialog-tab-licenses = 许可证
settings-dialog-tab-about = 关于
settings-dialog-link-privacy = 隐私政策
settings-dialog-link-terms = 服务条款

## settings_dialog.js: Startup tab

settings-startup-section-title = 启动行为
settings-startup-auto-start-label = 自动启动交易引擎
settings-startup-auto-start-hint = 启动应用时自动启动交易引擎
settings-startup-coming-soon = 即将推出
settings-startup-default-page-label = 默认页面
settings-startup-default-page-hint = 打开应用时显示的页面
settings-startup-notifications-label = 显示后台通知
settings-startup-notifications-hint = 为后台事件显示通知

## settings_dialog.js: About tab

settings-about-tagline = 原生 Solana 交易引擎
settings-about-link-github = { -github }
settings-about-link-docs = 文档
settings-about-link-telegram = { -telegram }
settings-about-link-website = 网站
settings-about-credits = 为 Solana 交易者打造
settings-about-copyright = © { $year } { -brand }。保留所有权利。

## interface_tab.js

settings-interface-section-appearance = 外观
settings-interface-theme-label = 主题
settings-interface-theme-hint = 选择您偏好的配色方案
settings-interface-theme-dark = 深色
settings-interface-theme-light = 浅色
settings-interface-language-label = 语言
settings-interface-language-hint = 仪表盘的显示语言
settings-interface-logo-shape-label = 代币徽标形状
settings-interface-logo-shape-hint = “圆形”会裁剪所有徽标；“原样”保留每个图标自身的轮廓
settings-interface-logo-shape-circle = 圆形
settings-interface-logo-shape-natural = 原样
settings-interface-animations-label = 启用动画
settings-interface-animations-hint = 平滑的过渡和效果
settings-interface-compact-label = 紧凑模式
settings-interface-compact-hint = 缩小间距以显示更多内容
settings-interface-section-data = 数据与显示
settings-interface-refresh-label = 刷新间隔
settings-interface-refresh-hint = 数据的刷新频率
settings-interface-refresh-seconds =
    { $count ->
       *[other] { $count } 秒
    }
settings-interface-refresh-minutes =
    { $count ->
       *[other] { $count } 分钟
    }
settings-interface-ticker-label = 显示行情栏
settings-interface-ticker-hint = 在顶部显示实时指标行情栏
settings-interface-page-size-label = 表格每页行数
settings-interface-page-size-hint = 每个表格页的默认行数
settings-interface-page-size-rows =
    { $count ->
       *[other] { $count } 行
    }
settings-interface-auto-expand-label = 自动展开分类
settings-interface-auto-expand-hint = 默认展开配置分类
settings-interface-hints-label = 显示上下文提示
settings-interface-hints-hint = 显示用于说明仪表盘功能的帮助图标
settings-interface-featured-label = 显示精选行
settings-interface-featured-hint = 在首页和代币页显示精选代币行
settings-interface-section-sound = 音效
settings-interface-sounds-label = 启用声音
settings-interface-sounds-hint = 为导航、状态变化和结果提供触感提示音

## security_tab.js

settings-security-loading = 正在加载安全设置…
settings-security-load-failed = 加载安全设置失败

settings-security-type-pin4 = 4 位 PIN
settings-security-type-pin6 = 6 位 PIN
settings-security-type-text = 文本密码
settings-security-type-unset = 未设置

settings-security-lockscreen-title = 仪表盘锁屏
settings-security-lockscreen-description = 使用 PIN 或密码保护您的仪表盘。锁屏触发后，需要验证身份才能继续使用。
settings-security-enable-label = 启用锁屏
settings-security-enable-hint = 使用密码验证保护您的仪表盘
settings-security-password-status-label = 密码状态
settings-security-password-current = 当前：{ $type }
settings-security-password-none = 未设置密码
settings-security-change = 更改
settings-security-remove = 移除
settings-security-set-password = 设置密码
settings-security-auto-lock-label = 无操作后自动锁定
settings-security-auto-lock-hint = 一段时间无操作后自动锁定
settings-security-auto-lock-never = 从不
settings-security-lock-blur-label = 窗口失去焦点时锁定
settings-security-lock-blur-hint = 切换到其他应用时自动锁定
settings-security-quick-actions-title = 快捷操作
settings-security-lock-now-label = 立即锁定仪表盘
settings-security-lock-now-hint = 立即锁定仪表盘
settings-security-needs-password = 请先设置密码再使用此功能
settings-security-needs-lockscreen = 请先启用锁屏再使用此功能
settings-security-lock-now = 立即锁定
settings-security-lock-not-ready = 无法锁定：锁屏尚未就绪
settings-security-setting-save-failed = 无法保存安全设置

## security_tab.js: two-factor authentication

settings-security-2fa-title = 双重验证
settings-security-2fa-description = 使用验证器应用（Google Authenticator、Authy 等）增加一层安全保护
settings-security-2fa-status-label = 2FA 状态
settings-security-2fa-status-enabled = 双重验证已启用
settings-security-2fa-status-none = 未配置
settings-security-2fa-disable = 停用 2FA
settings-security-2fa-enable = 启用 2FA

## security_tab.js: password dialog

settings-security-modal-close =
    .aria-label = 关闭
settings-security-password-set-title = 设置密码
settings-security-password-change-title = 更改密码
settings-security-password-current-label = 当前密码
settings-security-password-current-input =
    .placeholder = 输入当前密码
settings-security-password-type-label = 密码类型
settings-security-password-new-label = 新密码
settings-security-password-new-input =
    .placeholder = 输入新密码
settings-security-password-confirm-label = 确认密码
settings-security-password-confirm-input =
    .placeholder = 再次输入密码
settings-security-password-update = 更新密码
settings-security-placeholder-pin4 = 输入 4 位 PIN
settings-security-placeholder-pin6 = 输入 6 位 PIN
settings-security-placeholder-text = 输入密码
settings-security-password-required = 请输入密码
settings-security-password-mismatch = 两次输入的密码不一致
settings-security-pin4-invalid = PIN 必须为 4 位数字
settings-security-pin6-invalid = PIN 必须为 6 位数字
settings-security-text-too-short = 密码至少需要 4 个字符
settings-security-password-saved = 密码已保存
settings-security-password-save-failed = 保存密码失败
settings-security-password-save-failed-detail = 保存密码失败：{ $message }

## security_tab.js: remove password dialog

settings-security-remove-title = 移除密码
settings-security-remove-description = 输入当前密码以移除锁屏保护。
settings-security-remove-confirm = 移除密码
settings-security-current-required = 请输入当前密码
settings-security-password-removed = 密码已移除
settings-security-password-remove-failed = 移除密码失败
settings-security-password-remove-failed-detail = 移除密码失败：{ $message }

## security_tab.js: enable and disable two-factor authentication

settings-security-2fa-enable-title = 启用双重验证
settings-security-2fa-password-prompt = 请输入密码以继续：
settings-security-2fa-password-input =
    .placeholder = 输入密码
settings-security-2fa-continue = 继续
settings-security-2fa-manual-code = 手动输入代码：
settings-security-2fa-qr =
    .alt = TOTP 二维码
settings-security-2fa-code-prompt = 请输入验证器应用中的 6 位验证码：
settings-security-2fa-verify-enable = 验证并启用
settings-security-2fa-password-required = 请输入密码
settings-security-2fa-setup-failed = 设置 2FA 失败
settings-security-2fa-code-invalid-length = 请输入 6 位验证码
settings-security-2fa-code-invalid = 验证码无效
settings-security-2fa-enabled = 双重验证已启用
settings-security-2fa-verify-failed = 验证码校验失败
settings-security-2fa-disable-title = 停用双重验证
settings-security-2fa-disable-prompt = 请输入密码以停用 2FA：
settings-security-2fa-disable-failed = 停用 2FA 失败
settings-security-2fa-disabled = 双重验证已停用

## agent_connections_tab.js

settings-agent-category-analysis = 分析
settings-agent-category-portfolio = 投资组合
settings-agent-category-trading = 交易
settings-agent-category-config = 配置
settings-agent-category-system = 系统
settings-agent-category-analysis-description = 代币分析、市场数据和安全检查。
settings-agent-category-portfolio-description = 持仓中的仓位、余额和盈亏。
settings-agent-category-trading-description = 使用真实资金买入、卖出和平仓。
settings-agent-category-config-description = 所有机器人设置，包括 RPC 端点，但不含钱包密钥。
settings-agent-category-system-description = 状态、事件和紧急停止。
settings-agent-category-analysis-inline = 分析
settings-agent-category-portfolio-inline = 投资组合
settings-agent-category-trading-inline = 交易
settings-agent-category-config-inline = 配置
settings-agent-category-system-inline = 系统

settings-agent-level-allow = 允许
settings-agent-level-ask-user = 询问
settings-agent-level-deny = 关闭
settings-agent-level-allow-hint = 立即执行。
settings-agent-level-ask-user-hint = 等待您在应用中批准。
settings-agent-level-deny-hint = 直接拒绝，并对智能体隐藏。

settings-agent-preset-full = 完全访问
settings-agent-preset-ask = 先询问
settings-agent-preset-read = 只读
settings-agent-preset-full-description = 所有操作无需询问即可执行。钱包密钥仍无法访问。
settings-agent-preset-ask-description = 每项操作都需等待您在应用中批准。
settings-agent-preset-read-description = 可读取分析和投资组合数据，无法更改任何内容。
settings-agent-preset-custom = 自定义
settings-agent-preset-group =
    .aria-label = 权限预设
settings-agent-permission-group = { $category }权限

settings-agent-summary-asks-only = 受限：{ $asking }需询问
settings-agent-summary-off-only = 受限：{ $off }已关闭
settings-agent-summary-asks-and-off = 受限：{ $asking }需询问；{ $off }已关闭

settings-agent-client-claude = { -claude } Code / Desktop
settings-agent-client-codex = { -codex } CLI
settings-agent-client-openclaw = { -openclaw }
settings-agent-client-hermes = { -hermes }
settings-agent-client-generic = 通用 stdio MCP

settings-agent-note-placeholder = 请将 /absolute/path/to/screenerbot 替换为 { -brand } 可执行文件的绝对路径。运行中的应用无法在此系统上获取其可执行文件路径。
settings-agent-note-data-dir = 如果您使用非默认的数据目录运行 { -brand }，还需在客户端设置 SCREENERBOT_DATA_DIR（另一个 -e / --env 参数，或一个 env 条目），并指向相同路径。
settings-agent-note-codex-run = 运行该命令，或将 TOML 块添加到 ~/.codex/config.toml（$CODEX_HOME/config.toml）。之后请重启 { -codex }。
settings-agent-note-codex-get = `codex mcp get screenerbot` 的输出会隐藏密钥。
settings-agent-note-claude-code = { -claude } Code：运行该命令，然后重启 { -claude } Code。`claude mcp get screenerbot` 会输出已配置的环境变量，包括密钥。
settings-agent-note-claude-desktop = { -claude } Desktop：将该 JSON 合并到 claude_desktop_config.json 的 `mcpServers` 下，然后重启应用。
settings-agent-note-openclaw = 运行该命令，然后使用 `openclaw mcp doctor screenerbot --probe` 验证已保存的 stdio 服务器能否启动并提供工具。
settings-agent-note-hermes = 将此内容添加到 { -hermes } 配置文件的 `mcp_servers` 下，然后重启 { -hermes }。
settings-agent-note-generic = 适用于任何支持 stdio 的 MCP 客户端：在客户端保存服务器列表的位置，使用这些参数和环境变量运行此命令。
settings-agent-block-codex-command = { -codex } CLI — 终端命令
settings-agent-block-codex-toml = { -codex } CLI — ~/.codex/config.toml（备用）
settings-agent-block-claude-command = { -claude } Code — 终端命令
settings-agent-block-claude-desktop = { -claude } Desktop — claude_desktop_config.json
settings-agent-block-openclaw = { -openclaw } — 终端命令
settings-agent-block-hermes = { -hermes } — mcp_servers（YAML）
settings-agent-block-generic = 通用 stdio MCP 客户端

settings-agent-name-required = 请为此连接输入名称。
settings-agent-name-too-long = 名称不能超过 { $max } 个字符。
settings-agent-name-control-characters = 名称不能包含控制字符。

settings-agent-title = 智能体连接
settings-agent-description = 连接 { -claude }、{ -codex }、{ -hermes }、{ -openclaw } 或任何 stdio MCP 客户端。{ -brand } 必须保持运行。每个连接都有各自的权限：默认完全访问，您可随时按连接进行限制。任何连接都无法读取或更改您的钱包密钥。
settings-agent-name-label = 连接名称
settings-agent-name-hint = 显示在下方列表中，便于区分各个连接。
settings-agent-name-input =
    .placeholder = 笔记本编程智能体
settings-agent-client-label = 客户端
settings-agent-client-hint = 决定连接创建后显示的设置方式。
settings-agent-permissions-label = 权限
settings-agent-permissions-hint = 新连接可执行所有操作。您可以现在或稍后在下方列表中限制任一分类，无论如何钱包密钥都无法访问。
settings-agent-create = 创建连接
settings-agent-issued-group =
    .aria-label = 新连接凭据
settings-agent-issued-warning = 请立即复制密钥。它只显示一次，之后无法再次获取；如果丢失，请撤销并重新创建该连接。{ -brand } 仅保存单向校验值，明文由您的 MCP 客户端保存在其自身配置中。
settings-agent-issued-client-id = 客户端 ID
settings-agent-issued-secret = 一次性密钥
settings-agent-setup-for = 设置对象
settings-agent-done = 完成
settings-agent-list-title = 连接
settings-agent-loading = 正在加载连接…
settings-agent-active-count = { $count } 个活跃
settings-agent-empty = 暂无连接。请在上方创建一个以配对客户端。
settings-agent-empty-active = 暂无活跃连接。
settings-agent-revoked-title = 已撤销的连接
settings-agent-created = 创建于 { $time }
settings-agent-last-used = 最近使用 { $time }
settings-agent-never-used = 从未使用
settings-agent-permissions-edit = 权限
settings-agent-revoke = 撤销
settings-agent-permissions-save = 保存权限

settings-agent-load-failed = 加载智能体连接失败
settings-agent-list-failed = 无法加载连接
settings-agent-create-failed = 无法创建连接。
settings-agent-unreachable-create = 无法连接到 { -brand } 以创建连接。
settings-agent-permissions-update-failed = 无法更新权限
settings-agent-permissions-updated = 权限已更新
settings-agent-permissions-updated-detail = 将应用于该连接的下一次请求。
settings-agent-unreachable-save = 无法连接到 { -brand } 以保存
settings-agent-revoke-title = 撤销连接
settings-agent-revoke-message = 撤销“{ $label }”？客户端将在下一次请求时停止工作，且无法恢复。
settings-agent-revoke-fallback-name = 此连接
settings-agent-revoke-failed = 无法撤销连接
settings-agent-unreachable-revoke = 无法连接到 { -brand } 以撤销

## telegram_tab.js

settings-telegram-loading = 正在加载 { -telegram } 设置…
settings-telegram-load-failed = 加载 { -telegram } 设置失败
settings-telegram-unknown = 未知
settings-telegram-session-active = 已活跃：{ $duration }
settings-telegram-sessions-empty = 暂无活跃会话
settings-telegram-session-revoke = 撤销

settings-telegram-connection-title = 连接
settings-telegram-connection-description = 连接您的 { -telegram } 机器人，以接收通知并远程控制 { -brand }。
settings-telegram-enable-label = 启用 { -telegram }
settings-telegram-enable-hint = 启用 { -telegram } 机器人集成
settings-telegram-token-label = 机器人令牌
settings-telegram-token-saved = 令牌已保存
settings-telegram-token-help = 在 { -telegram } 上通过 @BotFather 获取
settings-telegram-token-input-saved =
    .placeholder = 令牌已保存（输入新令牌可更改）
settings-telegram-token-input =
    .placeholder = 输入机器人令牌
settings-telegram-token-toggle =
    .title = 显示/隐藏
settings-telegram-chat-label = 聊天 ID
settings-telegram-chat-connected = 已连接到聊天：
settings-telegram-chat-discover-hint = 自动发现您的聊天 ID
settings-telegram-chat-change =
    .title = 更改
settings-telegram-chat-discover = 发现聊天 ID
settings-telegram-discovery-step-add = 将您的机器人添加到 { -telegram } 群组，或与它发起私聊
settings-telegram-discovery-step-privacy = 对于群组：请检查 @BotFather → /mybots → [您的机器人] → Bot Settings → Group Privacy
settings-telegram-discovery-privacy = <strong>隐私模式关闭：</strong>机器人接收所有群消息<br/><strong>隐私模式开启：</strong>机器人仅在被 @ 提及时接收消息
settings-telegram-discovery-step-send = 发送任意消息（若隐私模式已开启，请 @ 提及您的机器人）
settings-telegram-discovery-listening = 正在监听消息…
settings-telegram-discovery-select = 选择
settings-telegram-chat-id-label = ID：
settings-telegram-language-label = 消息语言
settings-telegram-language-hint = { -telegram } 机器人消息和按钮的语言
settings-telegram-language-follow-app = 跟随应用语言
settings-telegram-test-label = 测试连接
settings-telegram-test-hint = 发送测试消息以验证配置
settings-telegram-test-send = 发送测试
settings-telegram-test-sending = 正在发送…

settings-telegram-chat-type-private = 私聊
settings-telegram-chat-type-group = 群组
settings-telegram-chat-type-supergroup = 超级群组
settings-telegram-chat-type-channel = 频道

settings-telegram-auth-title = 命令验证
settings-telegram-auth-description = { -telegram } 命令与仪表盘锁屏使用相同的 2FA。
settings-telegram-auth-protected = 已保护
settings-telegram-auth-disabled = 已停用
settings-telegram-auth-not-configured = 未配置
settings-telegram-auth-error = 错误
settings-telegram-auth-protected-note = 命令受锁屏 2FA 保护。会话过期后，用户必须通过 <code>/login</code> 命令提供验证器代码。
settings-telegram-auth-disabled-note = 锁屏 2FA 已配置，但未用于 { -telegram }。请启用上方的“命令需要 2FA”以保护 { -telegram } 命令。
settings-telegram-auth-missing-note = 尚未配置锁屏 2FA。没有 2FA 时，过期的会话会在未经验证的情况下自动重新激活。
settings-telegram-auth-managed-in = 2FA 的管理位置：
settings-telegram-auth-configure-in = 请前往
settings-telegram-auth-configure-suffix = 配置 2FA，以要求对 { -telegram } 命令进行验证。
settings-telegram-security-link = 安全设置
settings-telegram-timeout-title = 会话超时
settings-telegram-timeout-description = 已验证会话保持活跃的时长
settings-telegram-sessions-title = 活跃会话

settings-telegram-notifications-title = 通知设置
settings-telegram-notifications-description = 选择哪些事件触发 { -telegram } 通知。
settings-telegram-notify-opened-label = 仓位已开启
settings-telegram-notify-opened-hint = 新仓位开启时通知
settings-telegram-notify-closed-label = 仓位已平仓
settings-telegram-notify-closed-hint = 仓位平仓时通知
settings-telegram-notify-partial-label = 部分出场
settings-telegram-notify-partial-hint = 仓位部分出场时通知
settings-telegram-notify-dca-label = DCA 已执行
settings-telegram-notify-dca-hint = DCA 订单执行时通知
settings-telegram-notify-errors-label = 错误
settings-telegram-notify-errors-hint = 出现错误和失败时通知
settings-telegram-notify-startup-label = 启动/关闭
settings-telegram-notify-startup-hint = 机器人启动或停止时通知
settings-telegram-notify-filtering-label = 过滤提醒
settings-telegram-notify-filtering-hint = 新代币通过过滤条件时通知
settings-telegram-notify-trades-label = 交易提醒
settings-telegram-notify-trades-hint = 所关注代币出现重大交易时通知
settings-telegram-notify-daily-label = 每日摘要
settings-telegram-notify-daily-hint = 接收每日交易活动和盈亏摘要

settings-telegram-features-title = 功能
settings-telegram-features-description = 配置 { -telegram } 机器人的功能。
settings-telegram-commands-label = 启用命令
settings-telegram-commands-hint = 允许通过 { -telegram } 命令控制机器人
settings-telegram-require-2fa-label = 命令需要 2FA
settings-telegram-require-2fa-hint = 会话过期后，需要 2FA 验证码才能重新激活。使用锁屏 2FA。
settings-telegram-inline-label = 内联操作按钮
settings-telegram-inline-hint = 在通知消息中显示操作按钮

settings-telegram-setting-save-failed = 无法保存 { -telegram } 设置
settings-telegram-discovery-start-failed = 无法开始发现
settings-telegram-chat-selected = 已选择聊天
settings-telegram-chat-select-failed = 无法选择聊天
settings-telegram-test-sent = 测试消息已发送
settings-telegram-test-failed = 测试消息发送失败
settings-telegram-session-revoked = 会话已撤销
settings-telegram-session-revoke-failed = 无法撤销会话

## licenses_tab.js

settings-licenses-title = 开源许可证
settings-licenses-subtitle = { -brand } 基于以下开源软件构建
settings-licenses-footer = 完整的许可证文本可在项目仓库及各依赖项的源代码中查看。
settings-licenses-category-framework = 应用框架
settings-licenses-category-solana = Solana 区块链
settings-licenses-category-data = 数据与存储
settings-licenses-category-networking = 网络
settings-licenses-category-cryptography = 加密与编码
settings-licenses-category-assets = 界面资源
settings-licenses-desc-electron = 桌面应用框架
settings-licenses-desc-tokio = Rust 异步运行时
settings-licenses-desc-axum = Web 服务器框架
settings-licenses-desc-tower = 服务抽象层
settings-licenses-desc-hyper = HTTP 实现
settings-licenses-desc-solana-sdk = Solana SDK 核心
settings-licenses-desc-solana-client = RPC 客户端
settings-licenses-desc-solana-program = 程序库
settings-licenses-desc-spl-token = SPL Token 程序
settings-licenses-desc-spl-token-2022 = Token-2022 扩展
settings-licenses-desc-spl-associated-token-account = 关联代币账户
settings-licenses-desc-sqlite = 嵌入式数据库引擎
settings-licenses-desc-rusqlite = SQLite 的 Rust 绑定
settings-licenses-desc-r2d2 = 数据库连接池
settings-licenses-desc-serde = 序列化框架
settings-licenses-desc-toml = 配置解析
settings-licenses-desc-reqwest = HTTP 客户端
settings-licenses-desc-tokio-tungstenite = WebSocket 客户端
settings-licenses-desc-rustls = TLS 实现
settings-licenses-desc-blake3 = 哈希函数
settings-licenses-desc-sha-2 = SHA-256/512 哈希
settings-licenses-desc-bs58 = Base58 编码
settings-licenses-desc-base64 = Base64 编码
settings-licenses-desc-lucide-icons = 图标字体库
settings-licenses-desc-inter = 界面字体
settings-licenses-desc-jetbrains-mono = 等宽字体
settings-licenses-desc-orbitron = 展示字体
settings-licenses-desc-vazirmatn = 阿拉伯文和波斯文字体
settings-licenses-desc-noto-sans-devanagari = 天城文字体
settings-licenses-desc-noto-sans-sc = 简体中文字体
settings-licenses-desc-pretendard = 韩文字体
settings-licenses-desc-pretendard-jp = 日文字体

## hints_tab.js

settings-hints-title = 上下文提示
settings-hints-description = 上下文提示是用于说明仪表盘功能的帮助图标。您可以在下方查看所有提示，并恢复已通过“不再显示”隐藏的提示，可逐个恢复，也可一次全部恢复。
settings-hints-hidden-label = 已隐藏的提示
settings-hints-hidden-summary = { $total } 条提示中当前有 { $hidden } 条已隐藏。
settings-hints-restore-all = 恢复所有提示
settings-hints-toggle-shown =
    .title = 显示此提示
settings-hints-toggle-shown-title = 显示
settings-hints-toggle-hidden-title = 已隐藏，开启后显示
settings-hints-restore-title = 恢复所有提示
settings-hints-restore-message = 重新显示所有上下文提示，包括您已隐藏的每一条？
settings-hints-restore-confirm = 全部恢复
settings-hints-restored = 所有提示已恢复

## account_tab.js

settings-account-title = { -brand } 账户
settings-account-description = 免费，且为可选项。没有账户时，{ -brand } 同样可以交易、发现代币和显示图表，只是使用公共提供商的数据。下方面板列出了登录后可获得的功能。
settings-account-data-title = { -brand } 数据
settings-account-data-description = 我们在 screenerbot.io 运营共享的市场数据服务：覆盖七个时间周期的聚合 K 线、已解析的流动性池注册表、缓存的安全报告和标准化的代币身份信息。这样每个安装实例就不必各自受到公共提供商的限流，而使用该服务需要账户，以便让这项共享成本有据可依。
settings-account-data-fallback = 服务不可用时，{ -brand } 会自动回退到公共提供商。一切照常运行，只是图表填充较慢，历史数据较少。
settings-account-gateway-title = 发送交易
settings-account-gateway-description = 登录后，{ -brand } 可以通过 screenerbot.io 而不是您自己的 RPC 广播您的兑换交易。每笔交易仍由您的机器人在本机构建并签名，服务器只负责转发，且无法在不破坏签名的情况下修改已签名的交易。
settings-account-gateway-label = 使用 { -brand } RPC 发送交易
settings-account-gateway-hint = 仅用于提交交易。价格数据始终来自您自己的 RPC；流动性池轮询负载过大，不适合共享端点，因此绝不会发送到那里。
settings-account-manage-title = 管理您的账户
settings-account-manage-description = 您的密码、邮箱地址、已连接设备和推荐奖励提现均在网站上管理。在那里撤销某台设备会使其在所有位置退出登录，包括当前设备。
settings-account-open-dashboard = 打开您的仪表盘

## navigation_tab.js

settings-navigation-title = 导航标签
settings-navigation-hint = 拖动项目以重新排序，使用开关切换可见性。
settings-navigation-section-layout = 布局
settings-navigation-overflow-label = 放不下的标签页
settings-navigation-overflow-hint = 横向滚动标签行，或将放不下的标签页收进末尾的“更多”菜单。
settings-navigation-overflow-scroll = 滚动
settings-navigation-overflow-menu = “更多”菜单
settings-navigation-drag-handle =
    .title = 拖动以调整顺序
settings-navigation-defaults-failed = 无法加载默认导航
settings-navigation-reset = 导航已恢复默认

## data_tab.js

settings-data-storage-title = 数据库存储
settings-data-storage-description = 存储您的交易数据、仓位和历史信息的所有数据库概览。
settings-data-stats-loading = 正在加载数据库统计…
settings-data-stats-load-failed = 加载数据库统计失败
settings-data-total-storage = 数据库总存储
settings-data-db-tokens = 代币
settings-data-db-transactions = 交易
settings-data-db-positions = 仓位
settings-data-db-events = 事件
settings-data-db-ohlcv = OHLCV
settings-data-db-wallet = 钱包
settings-data-db-pools = 流动性池
settings-data-db-strategies = 策略
settings-data-db-actions = 操作
settings-data-directory-label = 数据目录
settings-data-directory-copied = 数据目录
settings-data-config-path-copied = 配置路径
settings-data-path-unavailable = 不可用
settings-data-path-copy-title = 点击复制路径
settings-data-path-copy-failed = 复制路径失败

settings-data-config-title = 配置管理
settings-data-config-description = 导出、导入和管理您的机器人配置。进行重大更改前请先备份。
settings-data-config-export = 导出配置
settings-data-config-import = 导入配置
settings-data-config-reset = 恢复默认
settings-data-config-location-label = 配置位置
settings-data-config-fetch-failed = 获取配置失败
settings-data-config-exported = 配置已导出
settings-data-config-export-failed = 导出配置失败：{ $message }
settings-data-config-import-title = 导入配置
settings-data-config-import-message = 导入此配置？当前设置将被覆盖，钱包凭据将保留。
settings-data-config-imported = 配置导入成功。部分更改可能需要重启后生效。
settings-data-config-import-failed = 导入配置失败：{ $message }
settings-data-config-reset-title = 重置配置
settings-data-config-reset-message = 将所有设置恢复为默认值？您的钱包凭据将保留，其他所有设置都会被重置。
settings-data-config-reset-done = 配置已恢复默认
settings-data-config-reset-failed = 重置配置失败：{ $message }
settings-data-unknown-error = 未知错误

settings-data-cleanup-title = 数据清理
settings-data-cleanup-description = 删除旧数据或未使用的数据以释放磁盘空间。这些操作无法撤销。
settings-data-ohlcv-cleanup-label = OHLCV 数据清理
settings-data-ohlcv-cleanup-hint = 删除在指定时间内未活跃的代币的 K 线数据。
settings-data-cleanup-hours-unit = 小时
settings-data-cleanup-ohlcv = 清理 OHLCV
settings-data-cleanup-running = 正在清理…
settings-data-cleanup-hours-invalid = 小时数无效
settings-data-cleanup-confirm-title = 删除 OHLCV 数据
settings-data-cleanup-confirm-message =
    删除超过 { $hours -> 
       *[other] { $hours } 小时
    }未活跃的代币的 OHLCV 数据？
settings-data-cleanup-done =
    { $count ->
       *[other] 已清理 { $count } 个未活跃代币
    }
settings-data-cleanup-failed = 清理失败
settings-data-cleanup-failed-detail = 清理失败：{ $message }

settings-data-cache-clear-label = 清除所有 OHLCV 缓存
settings-data-cache-clear-hint = 清除所有缓存的 K 线数据，并从头重新获取每个受监控的代币。当图表异常或数据逻辑更新后使用。
settings-data-cache-clear = 清除 OHLCV 缓存
settings-data-cache-clearing = 正在清除…
settings-data-cache-confirm-title = 清除所有 OHLCV 缓存
settings-data-cache-confirm-message = 清除所有代币的缓存 K 线数据？受监控的代币将从头重新获取历史数据。此操作无法撤销。
settings-data-candles-count =
    { $count ->
       *[other] { $count } 根 K 线
    }
settings-data-tokens-count =
    { $count ->
       *[other] { $count } 个代币
    }
settings-data-cache-cleared = 已清除 { $tokens } 的 { $candles }，正在重新获取
settings-data-cache-clear-failed = 清除 OHLCV 缓存失败
settings-data-cache-clear-failed-detail = 清除 OHLCV 缓存失败：{ $message }

settings-data-ui-cache-label = 界面状态缓存
settings-data-ui-cache-hint = 清除已保存的表格偏好、过滤状态和视图设置。
settings-data-ui-cache-clear = 清除界面缓存
settings-data-ui-cache-confirm-title = 清除界面状态
settings-data-ui-cache-confirm-message = 清除所有已保存的界面偏好？这会重置表格列、过滤条件和视图设置。
settings-data-ui-cache-cleared =
    { $count ->
       *[other] 已清除 { $count } 项缓存的界面设置
    }

settings-data-folder-label = 打开数据文件夹
settings-data-folder-hint = 在文件管理器中打开包含所有 { -brand } 数据的文件夹。
settings-data-folder-open = 打开文件夹
settings-data-folder-open-failed = 无法打开数据文件夹
