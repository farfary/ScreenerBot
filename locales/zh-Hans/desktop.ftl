# Text owned by the Electron shell: tray, application menu, native dialogs, the
# splash and the boot-error screen. Server-only: read from the packaged catalogs
# by electron/src/l10n.js, never sent to the dashboard. Fatal startup errors
# arrive already rendered from startup.ftl; only their chrome lives here.
#
# Menu roles (Edit, Window, Quit and the like) are not listed: the operating
# system localizes them.

## Actions shared by dialogs.

desktop-action-ok = 确定

## Splash and loading status.

desktop-splash-starting = 正在启动 { -brand }
desktop-splash-restarting = 正在重启 { -brand }
desktop-splash-recovering = 正在恢复
desktop-splash-opening-dashboard = 正在打开仪表盘
desktop-splash-checking-dependencies = 正在检查依赖项
desktop-splash-installing-dependencies = 正在安装系统依赖项
desktop-splash-installing-dependencies-detail = { -brand } 需要 Microsoft Visual C++ Redistributable 才能运行。
desktop-splash-resetting-wallet = 正在重置钱包数据
desktop-splash-resetting-wallet-detail = 清除现有钱包数据之前会先进行备份。
desktop-splash-updating = 正在更新到 v{ $version }
desktop-splash-updating-detail = 您的设置和数据将保持原样。
desktop-splash-restoring = 正在还原到 v{ $version }
desktop-splash-restoring-detail = 更新版本 v{ $failed } 未能启动，因此将由上一版本接管。

## Boot-error screen: headings, actions and per-code subtitles.

desktop-boot-title-fallback = { -brand } 无法启动
desktop-boot-detail-fallback = 后端意外停止。
desktop-boot-remedy-label = 解决方法
desktop-boot-log-file-label = 日志文件：
desktop-boot-action-reset-wallet = 重置钱包数据并重启
desktop-boot-action-working = 处理中...
desktop-boot-action-open-logs = 打开日志文件夹
desktop-boot-action-copy = 复制详情
desktop-boot-action-copied = 已复制
desktop-boot-action-quit = 退出
desktop-boot-subtitle-wallet-mismatch = 检测到不同的钱包
desktop-boot-subtitle-port-in-use = 所需的网络端口被占用
desktop-boot-subtitle-lock-held = { -brand } 已在运行
desktop-boot-subtitle-config-invalid = 配置问题
desktop-boot-subtitle-directory-setup = 存储问题
desktop-boot-subtitle-storage-upgrade = 数据库升级问题
desktop-boot-subtitle-generic = 启动错误

## Boot errors raised by the shell itself (the backend never reported one).

desktop-boot-error-title = { -brand } 无法启动
desktop-boot-error-remedy = 请打开日志文件夹查看具体情况，然后重启应用。如果问题仍然存在，请通过 t.me/screenerbotio_support 联系支持。
desktop-boot-error-default = 后端在仪表盘就绪之前意外停止。
desktop-boot-error-restore-failed = 更新后的后端运行失败，且无法还原到上一版本（{ $error }）。
desktop-boot-error-spawn-failed = 无法启动后端程序（{ $error }）。
desktop-boot-error-spawn-missing = 无法启动后端程序。它可能已丢失，或被安全软件拦截。
desktop-boot-error-exited-running = 仪表盘运行期间后端停止（退出码 { $code }）。
desktop-boot-error-exited-early = 后端在仪表盘就绪之前停止（退出码 { $code }）。
desktop-boot-error-dashboard-load = 仪表盘加载失败（{ $description }，{ $code }）。
desktop-boot-error-renderer-gone = 仪表盘渲染进程已停止（{ $reason }）。
desktop-boot-error-unresponsive = 仪表盘无响应。
desktop-boot-error-url-failed = 无法加载仪表盘 URL（{ $error }）。
desktop-boot-error-relaunch-setup = 设置完成后无法重新启动后端。
desktop-boot-error-relaunch-recovery = 无法为恢复而重新启动后端。
desktop-boot-error-restart-offline = 重启后后端未能重新上线。
desktop-boot-error-recovery-offline = 恢复已完成，但后端未能就绪。
desktop-boot-error-start-timeout = 后端未能在规定时间内完成启动。首次运行较慢，或其他程序阻止了连接时可能出现这种情况。

## System tray.

desktop-tray-tooltip = { -brand } - Solana 交易机器人
desktop-tray-show = 显示 { -brand }
desktop-tray-open-dashboard = 打开仪表盘
desktop-tray-quit = 退出 { -brand }

## Menu items shared by the tray and the application menu.

desktop-menu-open-data-folder = 打开数据文件夹
desktop-menu-open-logs-folder = 打开日志文件夹
desktop-menu-documentation = 文档
desktop-menu-telegram-support = { -telegram } 支持
desktop-menu-check-updates = 检查更新...

## Application menu.

desktop-menu-file = 文件
desktop-menu-edit = 编辑
desktop-menu-view = 视图
desktop-menu-window = 窗口
desktop-menu-help = 帮助
desktop-menu-reset-zoom = 重置缩放
desktop-menu-zoom-in = 放大
desktop-menu-zoom-out = 缩小
desktop-menu-keyboard-shortcuts = 键盘快捷键
desktop-menu-telegram-channel = { -telegram } 频道
desktop-menu-telegram-community = { -telegram } 社区
desktop-menu-follow-x = 在 { -x }（{ -twitter }）上关注
desktop-menu-visit-website = 访问网站
desktop-menu-about = 关于 { -brand }

## About dialog.

desktop-about-title = 关于 { -brand }
desktop-about-message = { -brand }
desktop-about-detail =
    版本 { $version }

    高级 Solana 钱包管理与自动交易机器人。

    https://screenerbot.io

    © 2024-2026 { -brand }

## Keyboard shortcuts dialog. Key names stay as typed on the keyboard.

desktop-shortcuts-title = 键盘快捷键
desktop-shortcuts-message = { -brand } 键盘快捷键
desktop-shortcuts-body-mac =
    键盘快捷键：

    窗口控制：
      Cmd+M          最小化
      Cmd+W          关闭窗口
      Cmd+Q          退出
      Cmd+Ctrl+F     切换全屏

    缩放：
      Cmd++          放大
      Cmd+-          缩小
      Cmd+0          重置缩放

    导航：
      Cmd+R          重新加载仪表盘
      Cmd+Shift+D    打开数据文件夹

    其他：
      F1             打开文档
      Cmd+Alt+I      切换开发者工具
desktop-shortcuts-body-other =
    键盘快捷键：

    窗口控制：
      Alt+F4         退出
      F11            切换全屏

    缩放：
      Ctrl++         放大
      Ctrl+-         缩小
      Ctrl+0         重置缩放

    导航：
      Ctrl+R         重新加载仪表盘
      Ctrl+Shift+D   打开数据文件夹

    其他：
      F1             打开文档
      Ctrl+Shift+I   切换开发者工具

## Close confirmation (Windows and Linux).

desktop-close-title = 关闭 { -brand }
desktop-close-message = 您想要执行什么操作？
desktop-close-detail = { -brand } 可以继续在后台运行。最小化到系统托盘期间，交易机器人会继续监控和交易。
desktop-close-minimize = 最小化到托盘
desktop-close-quit = 完全退出
desktop-close-cancel = 取消

## Visual C++ Redistributable (Windows).

desktop-vcredist-missing-title = 缺少依赖项
desktop-vcredist-missing-message = 缺少 Visual C++ Redistributable
desktop-vcredist-missing-detail = { -brand } 需要 Microsoft Visual C++ Redistributable 才能运行。是否立即安装？
desktop-vcredist-install = 安装并修复
desktop-vcredist-exit = 退出
desktop-vcredist-not-found-title = 未找到安装程序
desktop-vcredist-not-found-message = 无法正确定位 { $name }。
desktop-vcredist-done-title = 安装完成
desktop-vcredist-done-message = 依赖项已成功安装。
desktop-vcredist-done-detail = { -brand } 即将启动。
desktop-vcredist-failed-title = 安装失败
desktop-vcredist-failed-message = 请手动安装 Visual C++ Redistributable。
