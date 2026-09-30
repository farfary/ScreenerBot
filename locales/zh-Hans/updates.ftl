# Update system text. Ids come from DeferReason in src/version/types.rs, the
# update check state and the /api/updates responses.

## Why a ready update has not been applied automatically

updates-defer-automatic-install-disabled = 自动安装已禁用。更新已就绪，将在您选择时应用。
updates-defer-trading-active = 当前有仓位、交易或工具操作正在进行，因此重启已推迟。应用空闲时将自动应用更新。
updates-defer-needs-installer = 此版本还会更新桌面外壳，因此需要运行一次安装程序。

## Update check failure. `cause` is the technical error text.

updates-check-failed = { $cause }
updates-check-failed-legacy = { $cause }

## Progress and outcome of update actions

updates-download-started = 正在下载更新 v{ $version }...
updates-apply-started = 正在安装更新。{ -brand } 将自动重启并重新连接。
updates-install-opened = 已打开经过验证的更新安装程序。请完成操作系统的安装流程。

# Toast shown by ui/settings/updates_tab.js after the installer is launched.
updates-installer-toast-title = 安装程序已打开
updates-installer-toast-message = { -brand } 现在将正常退出。

## Settings > Updates (ui/settings/updates_view.js, updates_tab.js)

updates-tab-status = 状态
updates-tab-release-notes = 发行说明
updates-tab-preferences = 偏好设置
updates-tab-sections = 更新分区
updates-checking-installation = 正在检查此安装...

# Status by phase. Ids come from UpdatePhase in src/version/types.rs. The detail of
# a phase that can carry backend text is the fallback shown without it; the detail
# of an available or downloading update describes the update kind instead.
updates-phase-idle-headline = 可以检查更新
updates-phase-idle-detail = 已安装 { -brand } v{ $version }。
updates-phase-up-to-date-headline = 已是最新版本
updates-phase-up-to-date-detail = { -brand } v{ $version } 是最新版本。
updates-phase-checking-headline = 正在检查更新
updates-phase-checking-detail = 正在查找最新发布的版本。
updates-phase-available-headline = 版本 { $version } 可用
updates-phase-downloading-headline = 正在下载 v{ $version }
updates-phase-verifying-headline = 正在验证 v{ $version }
updates-phase-verifying-detail = 正在将下载内容与其发布的校验和进行比对。
updates-phase-ready-to-apply-headline = 版本 { $version } 已就绪
updates-phase-ready-to-apply-detail = 可以立即安装此更新（需短暂重启），也可以在下次启动时自动安装。
updates-phase-ready-to-install-headline = 版本 { $version } 已就绪
updates-phase-ready-to-install-detail = 桌面安装程序已准备好完成此更新。
updates-phase-applying-headline = 正在安装更新
updates-phase-applying-detail = { -brand } 正在重启并切换到新版本。
updates-phase-applied-headline = 已更新到 v{ $version }
updates-phase-applied-detail = 更新已安装，无需其他操作。
updates-phase-failed-headline = 更新未能完成
updates-phase-failed-detail = 请重新尝试更新。
updates-phase-check-failed-headline = 无法检查更新
updates-phase-check-failed-detail = 无法连接发布服务。
updates-status-unavailable-headline = 更新状态不可用
updates-phase-unrecognized-detail = 无法识别所报告的更新状态。
updates-status-load-failed-detail = 无法加载安装状态。

# What an available update replaces. Ids come from UpdateKind. $size is a formatted size.
updates-kind-core = 核心程序更新 · { $size } · 需短暂重启
updates-kind-full = 桌面更新 · { $size } · 需要安装程序
updates-size-unknown = 大小未知

updates-action-check-now = 立即检查
updates-action-check-again = 再次检查
updates-action-try-again = 重试
updates-action-download = 下载更新
updates-action-restart = 重启以更新
updates-action-open-installer = 打开安装程序

updates-busy-checking = 正在检查...
updates-busy-resuming = 正在恢复下载...
updates-busy-starting-download = 正在开始下载...
updates-busy-restarting = 正在重启...
updates-busy-opening-installer = 正在打开安装程序...

updates-progress-downloading = 正在下载更新
updates-progress-verifying = 正在验证更新
# $done and $total are formatted sizes.
updates-progress-transferred = { $done } / { $total }
# $percent is a formatted percentage.
updates-progress-summary = { $transferred } · { $percent }
updates-progress-value-text = { $label }，{ $percent }，{ $transferred }

updates-detail-list-label = 安装详情
updates-detail-installed-version = 已安装版本
updates-detail-system = 系统
updates-detail-last-checked = 上次检查
updates-detail-never = 从未
updates-detail-available-version = 可用版本
updates-detail-download-size = 下载大小

updates-version-installed = 已安装
updates-version-available = 可用

updates-notes-highlights = 亮点
updates-notes-empty-title = 暂无发行说明
updates-notes-empty-error = 无法加载发布历史。请检查网络连接后重试。
updates-notes-empty-none = 发布版本后，发行说明将显示在这里。
updates-notes-history-notice = 显示的是此安装已知的内容，无法加载发布历史。
updates-release-empty = 此版本没有列出任何更改。
updates-release-changes =
    { $count ->
       *[other] { $count } 项更改
    }

updates-preferences-unavailable-title = 更新偏好设置不可用
updates-preferences-unavailable-detail = 无法加载更新配置。
updates-preference-fallback-name = 更新偏好设置
updates-preference-save-failed = 无法保存 { $preference }

updates-request-failed = 请求失败
updates-check-request-failed = 无法检查更新
updates-resume-failed = 无法恢复更新下载
updates-download-failed = 无法开始更新下载
updates-apply-failed = 无法安装更新
updates-install-failed = 无法打开更新安装程序
updates-apply-confirm-title = 安装 v{ $version }
updates-apply-confirm-message = { -brand } 将重启并切换到新版本。交易会停止几秒钟后自动恢复，持仓中的仓位不受影响。
updates-install-confirm-title = 运行安装程序
updates-install-confirm-message = 经过验证的安装程序将打开，且 { -brand } 会正常退出。请完成安装，然后重新打开 { -brand }。

# A release version as displayed.
updates-version-number = v{ $version }
